package org.taxman.h6.search;

import org.taxman.h6.bombus.Apiary;
import org.taxman.h6.bombus.BombusSolution;
import org.taxman.h6.bombus.GoldenHiveAccess;
import org.taxman.h6.bombus.Hive;
import org.taxman.h6.bombus.Namer;
import org.taxman.h6.frame.BaseFrame;
import org.taxman.h6.frame.FrameBuilder;
import org.taxman.h6.frame.FrameSolver;
import org.taxman.h6.frame.GoldenFrameAccess;
import org.taxman.h6.game.Board;
import org.taxman.h6.util.TxCollection;
import org.taxman.h6.util.TxSet;
import org.taxman.h6.util.TxUnmodifiableSet;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Exports intermediate results of the Java Taxman solver as JSON, so that a port can be checked stage by stage.
 * Lives in package org.taxman.h6.search to reach package-private members of SearchManager, FrameSet and
 * FactorProvider; the helpers GoldenHiveAccess and GoldenFrameAccess reach into org.taxman.h6.bombus and
 * org.taxman.h6.frame.
 *
 * <p>Usage: {@code GoldenExport [--structure-only] [--skip-existing] OUTDIR N-or-range...}, where a range is
 * {@code A-B} or {@code A-B/STEP}.  Writes OUTDIR/nNNNN.json for each n (atomically, so an interrupted run never
 * leaves a partial file).  {@code --structure-only} leaves out the searchManager, solve and timings sections;
 * {@code --skip-existing} skips n whose file already exists, which makes a run resumable.
 *
 * <p>Output schema (all integer sets are ascending arrays; lists keep their natural Java order; the file has one
 * line per top-level field):
 * <pre>
 * n
 * apiary: { hives: [ {name, sources, factors, remainingSources, remainingFactors, impossible, freeFactorCount,
 *                     promotionCandidates, solution} ... ],         // in Apiary.hives() order
 *           solution (list order), impossible, promotionCandidates }
 * frames: [ {name, remainingFactors, remainingSources, freeFactorCount, candidates} ... ]  // downstream first
 * estimateMaxPromotions: top.estimateMaxPromotions(0)
 * allCandidates: top.allCandidateNumbersIncludingDownstream()
 * frameSet: [ {moves, factors, freedoms} ... ]    // fs.frames of the FrameSet SearchManager.create builds
 * maxPromotionsUnrevised: fs.computeMaxNumberOfPromotions(allCandidates)
 * greedy: LowerBoundEstimator.greedy(fs.memoize(), allCandidates)
 * searchManager: { maxTotalPromotions, revisedFrame0Freedoms (null if there are no frames), initialBest }
 * solve: { accelerated, moves (list order), score, promotions }
 * timings: { searchManagerMs, solveMs }           // wall-clock, so these differ from run to run
 * error: only present if a stage threw; text like "stage: exception"; the failed stage's field is null
 * </pre>
 * "frames" holds, per frame: the name of the (possibly merged) hive, remainingFactors/remainingSources (these
 * are the columns FrameSet is built from), freeFactorCount, and the promotion candidates of that frame's hives.
 * The "solve" stage uses a fresh FrameSolver per n, so the solution to n-1 comes from optimal.json.
 * "accelerated" says whether FrameSolver's solve-from-previous shortcut succeeds for n.
 */
public class GoldenExport {
    private final int n;
    private final boolean structureOnly;
    private final Board board;
    private final Map<String, Object> out = new LinkedHashMap<>();
    private final Map<String, Object> timings = new LinkedHashMap<>();
    private final List<String> errors = new ArrayList<>();

    // results of earlier stages that later stages build on
    private BaseFrame top;
    private TxUnmodifiableSet candidates;
    private FrameSet frameSet;

    private interface Stage {
        Object run() throws Exception;
    }

    private GoldenExport(int n, boolean structureOnly) {
        this.n = n;
        this.structureOnly = structureOnly;
        this.board = Board.of(n);
    }

    public static void main(String[] args) throws IOException {
        boolean structureOnly = false;
        boolean skipExisting = false;
        Path outDir = null;
        List<Integer> ns = new ArrayList<>();
        for (String arg : args) {
            if (arg.equals("--structure-only")) {
                structureOnly = true;
            } else if (arg.equals("--skip-existing")) {
                skipExisting = true;
            } else if (outDir == null) {
                outDir = Path.of(arg);
            } else {
                ns.addAll(parseRange(arg));
            }
        }
        if (outDir == null || ns.isEmpty()) {
            System.err.println("usage: GoldenExport [--structure-only] [--skip-existing] OUTDIR N-or-A-B[/STEP]...");
            System.exit(2);
        }
        Files.createDirectories(outDir);

        int failed = 0;
        long start = System.nanoTime();
        for (int n : ns) {
            Path file = outDir.resolve(String.format("n%04d.json", n));
            if (skipExisting && Files.exists(file)) continue;
            long t0 = System.nanoTime();
            GoldenExport exporter = new GoldenExport(n, structureOnly);
            String json = exporter.export();
            Path tmp = outDir.resolve(file.getFileName() + ".tmp");
            Files.writeString(tmp, json, StandardCharsets.UTF_8);
            Files.move(tmp, file, StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
            String status = exporter.errors.isEmpty() ? "ok" : "ERROR " + exporter.errors;
            if (!exporter.errors.isEmpty()) failed++;
            System.out.printf("n=%d %s %d ms%n", n, status, (System.nanoTime() - t0) / 1_000_000);
        }
        System.out.printf("done: %d requested, %d with errors, %.1f s%n",
                ns.size(), failed, (System.nanoTime() - start) / 1e9);
    }

    static List<Integer> parseRange(String spec) {
        String[] byStep = spec.split("/");
        int step = byStep.length > 1 ? Integer.parseInt(byStep[1]) : 1;
        String[] bounds = byStep[0].split("-");
        int lo = Integer.parseInt(bounds[0]);
        int hi = bounds.length > 1 ? Integer.parseInt(bounds[1]) : lo;
        List<Integer> result = new ArrayList<>();
        for (int i = lo; i <= hi; i += step) result.add(i);
        return result;
    }

    private String export() {
        out.put("n", n);
        out.put("apiary", stage("apiary", null, this::apiary));
        out.put("frames", stage("frames", null, this::frames));
        out.put("estimateMaxPromotions", top == null ? null : stage("estimateMaxPromotions", null,
                () -> top.estimateMaxPromotions(0)));
        out.put("allCandidates", top == null ? null : stage("allCandidates", null,
                () -> top.allCandidateNumbersIncludingDownstream()));
        out.put("frameSet", top == null ? null : stage("frameSet", null, this::frameSet));
        out.put("maxPromotionsUnrevised", frameSet == null ? null : stage("maxPromotionsUnrevised", null,
                () -> frameSet.computeMaxNumberOfPromotions(candidates)));
        out.put("greedy", frameSet == null ? null : stage("greedy", null,
                () -> LowerBoundEstimator.greedy(frameSet.memoize(), candidates, "")));
        if (!structureOnly) {
            out.put("searchManager", stage("searchManager", "searchManagerMs", this::searchManager));
            out.put("solve", stage("solve", null, this::solve));
            out.put("timings", timings);
        }
        if (!errors.isEmpty()) out.put("error", String.join("; ", errors));
        return toJson();
    }

    /** Runs one stage; on failure records the error and yields null instead of aborting the whole export. */
    private Object stage(String name, String timingKey, Stage stage) {
        long t0 = System.nanoTime();
        try {
            return stage.run();
        } catch (Throwable t) {
            errors.add(name + ": " + describe(t));
            return null;
        } finally {
            if (timingKey != null) timings.put(timingKey, (System.nanoTime() - t0) / 1_000_000);
        }
    }

    private static String describe(Throwable t) {
        StackTraceElement[] trace = t.getStackTrace();
        String where = trace.length > 0 ? " at " + trace[0] : "";
        return t + where;
    }

    private Object apiary() {
        Apiary a1 = new Apiary(board, new Namer());
        List<Object> hives = new ArrayList<>();
        for (Hive h : a1.hives()) {
            Map<String, Object> m = new LinkedHashMap<>();
            m.put("name", h.getName());
            m.put("sources", h.sources());
            m.put("factors", h.factors());
            m.put("remainingSources", h.remainingSources());
            m.put("remainingFactors", h.remainingFactors());
            m.put("impossible", h.getImpossible());
            m.put("freeFactorCount", h.freeFactorCount());
            m.put("promotionCandidates", h.getPromotionCandidateNumbers());
            m.put("solution", GoldenHiveAccess.solution(h));
            hives.add(m);
        }
        Map<String, Object> result = new LinkedHashMap<>();
        result.put("hives", hives);
        result.put("solution", a1.getSolution());
        result.put("impossible", a1.getImpossible());
        result.put("promotionCandidates", a1.getPromotionCandidateNumbers());
        return result;
    }

    private Object frames() {
        top = FrameBuilder.build(board);
        List<BaseFrame> frames = top.getAllFrames();
        List<TxSet> candidatesByFrame = top.allCandidateNumbersByFrame();
        assert frames.size() == candidatesByFrame.size();
        List<Object> result = new ArrayList<>();
        for (int i = 0; i < frames.size(); i++) {
            BaseFrame f = frames.get(i);
            Map<String, Object> m = new LinkedHashMap<>();
            m.put("name", f.getName());
            m.put("remainingFactors", f.remainingFactors());
            m.put("remainingSources", f.remainingSources());
            m.put("freeFactorCount", f.freeFactorCount());
            m.put("candidates", candidatesByFrame.get(i));
            result.add(m);
        }
        return result;
    }

    /** Replicates SearchManager.create(Board, BaseFrame) up to the point where the FrameSet exists. */
    private Object frameSet() {
        candidates = top.allCandidateNumbersIncludingDownstream().unmodifiable();
        List<TxUnmodifiableSet> columns = new ArrayList<>();
        for (BaseFrame f : top.getAllFrames()) {
            columns.add(f.remainingFactors().unmodifiable());
            columns.add(f.remainingSources().unmodifiable());
        }
        TxSet allNumbers = TxSet.of(columns.stream().flatMapToInt(TxSet::stream));
        FactorProvider math = FactorProvider.create(board, allNumbers);
        frameSet = FrameSet.create(math, columns);

        List<Object> result = new ArrayList<>();
        for (Frame f : frameSet.frames) {
            Map<String, Object> m = new LinkedHashMap<>();
            m.put("moves", f.getMoves());
            m.put("factors", f.getFactors());
            m.put("freedoms", f.freedoms());
            result.add(m);
        }
        return result;
    }

    private Object searchManager() {
        // a fresh build, so nothing from the earlier stages can influence the manager
        SearchManager mgr = SearchManager.create(board, FrameBuilder.build(board));
        Map<String, Object> result = new LinkedHashMap<>();
        result.put("maxTotalPromotions", mgr.maxTotalPromotions);
        result.put("revisedFrame0Freedoms",
                mgr.startingPoint.frames.length > 0 ? mgr.startingPoint.frames[0].freedoms() : null);
        result.put("initialBest", mgr.bestSolution);
        return result;
    }

    private Object solve() {
        boolean accelerated = GoldenFrameAccess.solveBasedOnPrevOnly(new FrameSolver(), n) != null;
        long t0 = System.nanoTime();
        BombusSolution s;
        try {
            s = new FrameSolver().solve(n);
        } finally {
            timings.put("solveMs", (System.nanoTime() - t0) / 1_000_000);
        }
        Map<String, Object> result = new LinkedHashMap<>();
        result.put("accelerated", accelerated);
        result.put("moves", s.moves);
        result.put("score", s.score());
        result.put("promotions", s.promotions);
        return result;
    }

    // ---- JSON output: one line per top-level field ----

    private String toJson() {
        List<String> lines = new ArrayList<>();
        for (Map.Entry<String, Object> e : out.entrySet()) {
            StringBuilder sb = new StringBuilder();
            quote(sb, e.getKey());
            sb.append(':');
            write(sb, e.getValue());
            lines.add(sb.toString());
        }
        return "{\n" + String.join(",\n", lines) + "\n}\n";
    }

    private static void write(StringBuilder sb, Object v) {
        if (v == null) {
            sb.append("null");
        } else if (v instanceof Number || v instanceof Boolean) {
            sb.append(v);
        } else if (v instanceof String) {
            quote(sb, (String) v);
        } else if (v instanceof TxCollection) {
            write(sb, ((TxCollection) v).toArray());
        } else if (v instanceof int[]) {
            sb.append('[');
            int[] arr = (int[]) v;
            for (int i = 0; i < arr.length; i++) {
                if (i > 0) sb.append(',');
                sb.append(arr[i]);
            }
            sb.append(']');
        } else if (v instanceof List) {
            sb.append('[');
            boolean first = true;
            for (Object o : (List<?>) v) {
                if (!first) sb.append(',');
                first = false;
                write(sb, o);
            }
            sb.append(']');
        } else if (v instanceof Map) {
            sb.append('{');
            boolean first = true;
            for (Map.Entry<?, ?> e : ((Map<?, ?>) v).entrySet()) {
                if (!first) sb.append(',');
                first = false;
                quote(sb, (String) e.getKey());
                sb.append(':');
                write(sb, e.getValue());
            }
            sb.append('}');
        } else {
            throw new IllegalArgumentException("cannot serialize " + v.getClass());
        }
    }

    private static void quote(StringBuilder sb, String s) {
        sb.append('"');
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            switch (c) {
                case '"': sb.append("\\\""); break;
                case '\\': sb.append("\\\\"); break;
                case '\n': sb.append("\\n"); break;
                case '\r': sb.append("\\r"); break;
                case '\t': sb.append("\\t"); break;
                default:
                    if (c < 0x20) {
                        sb.append(String.format("\\u%04x", (int) c));
                    } else {
                        sb.append(c);
                    }
            }
        }
        sb.append('"');
    }
}
