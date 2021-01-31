package org.taxman.h6.frame;

import org.taxman.h6.bombus.Apiary;
import org.taxman.h6.bombus.BombusSolution;
import org.taxman.h6.bombus.Namer;
import org.taxman.h6.game.Board;
import org.taxman.h6.game.OptimalResult;
import org.taxman.h6.game.Solver;
import org.taxman.h6.util.TxPredicate;
import org.taxman.h6.util.TxSet;

import java.io.PrintStream;
import java.util.ArrayList;
import java.util.List;
import java.util.function.Predicate;
import java.util.stream.IntStream;
import java.util.stream.Stream;

import static org.taxman.h6.util.TxUnmodifiableSet.EmptySet;


public class GreedySolver implements Solver {
    public boolean warnOnImperfectScore = true;
    private final int maxComboSize;

    public GreedySolver(int maxComboSize) {
        this.maxComboSize = maxComboSize;
    }

    public BombusSolution solve(int n) {
        var board = Board.of(n);
        var a = new Apiary(board, new Namer());
        var frame = FrameBuilder.build(board);
        var p = new TxPredicate<TxSet>(c -> frame.fits(EmptySet, c));
        var candidates = a.getPromotionCandidateNumbers();
        int maxPromotions = frame.estimateMaxPromotions(0);
        var greedyPromotions = new GreedyPromotionMaximizer(candidates, maxPromotions, p).find();
        var b = new Apiary(board, greedyPromotions, new Namer());
        var sln = new BombusSolution(board, b.getSolution(), greedyPromotions);
        var opt = OptimalResult.get(n);
        if (opt != null) {
            if (warnOnImperfectScore && sln.score() != opt.score) {
                System.out.printf("WARNING: greedy score is %,d but optimal score is %,d (a difference of %,d)\n",
                        sln.score(), opt.score, opt.score-sln.score());
            }
        }
        return sln;
    }

    public void printInternalsReport(PrintStream ps) {
        // nothing to say just yet
    }

    public class GreedyPromotionMaximizer {
        private final TxSet allCandidates;
        private final Predicate<TxSet> predicate;
        private final int maxPromotions;

        public GreedyPromotionMaximizer(TxSet allCandidates, int maxPromotions, Predicate<TxSet> predicate) {
            this.allCandidates = allCandidates;
            this.predicate = predicate;
            this.maxPromotions = maxPromotions;
        }

        private TxSet baseCase(TxSet numbers) {
            var result = TxSet.empty();
            for (int n : numbers.descendingArray()) {
                TxSet newTry = TxSet.or(result, n);
                if (predicate.test(newTry)) result = newTry;
            }
            return result;
        }

        private List<Integer> promotionsToSkipVector(TxSet promotions) {
            TxSet promotionsThusFar = TxSet.empty();
            int skips = 0;
            List<Integer> skipVector = new ArrayList<>();
            for (int candidate : allCandidates.descendingArray()) {
                if (!predicate.test(TxSet.or(promotionsThusFar, candidate))) continue;
                if (promotions.contains(candidate)) {
                    promotionsThusFar.append(candidate);
                    skipVector.add(skips);
                    skips = 0;
                } else {
                    ++skips;
                }
            }
            for (int i = skipVector.size(); i < maxPromotions; i++) skipVector.add(0);
            return skipVector;
        }

        private TxSet skipVectorToPromotions(List<Integer> skipVector) {
            var skiterator = skipVector.iterator();
            int skips = (skiterator.hasNext()) ? skiterator.next() : -1;
            TxSet result = TxSet.empty();
            for (int candidate : allCandidates.descendingArray()) {
                if (!predicate.test(TxSet.or(result, candidate))) continue;
                if (--skips < 0) {
                    result.append(candidate);
                    skips = (skiterator.hasNext()) ? skiterator.next() : -1;
                }
            }
            return result;
        }

        private TxSet find() {
            var bestPromotions = baseCase(allCandidates);
            //System.out.printf("  greedy search initial: %d=%s from %s\n", bestPromotions.sum(), bestPromotions, allCandidates);

            while (true) {
                var result = improveSkipVector(bestPromotions);
                if (result.sum() <= bestPromotions.sum()) break;
                //var dropped = TxSet.subtract(bestPromotions, result);
                //var added = TxSet.subtract(result, bestPromotions);
                //System.out.printf("         greedy upgrade: %d = %s: added %s, dropped %s\n", result.sum(), result, added, dropped);
                bestPromotions = result;
            }

            return bestPromotions;
        }

        private TxSet improveSkipVector(TxSet base) {
            var bestVector = this.promotionsToSkipVector(base);
            return GreedySolver.combinationsUpToSize(TxSet.of(IntStream.rangeClosed(1, maxPromotions)), maxComboSize)
                    .parallel()
                    .map(combo -> addToVector(bestVector, combo))
                    .map(this::skipVectorToPromotions)
                    .reduce(base, (best, other) -> best.sum() >= other.sum() ? best : other);
        }
    }

    private static List<Integer> addToVector(List<Integer> vector, TxSet toAdd) {
        var result = new ArrayList<>(vector);
        toAdd.forEach(x -> result.set(x - 1, result.get(x - 1) + 1));
        return result;
    }

    static Stream<TxSet> combinationsUpToSize(TxSet numbers, int size) {
        return IntStream.rangeClosed(1, size)
                .mapToObj(x -> combinations(numbers, x))
                .flatMap(x -> x);
    }

    static Stream<TxSet> combinations(TxSet numbers, int size) {
        if (numbers.size() < size) {
            return Stream.of();
        } else if (numbers.size() == size) {
            return Stream.of(numbers);
        } else if (size == 1) {
            return numbers.stream()
                    .mapToObj(TxSet::of);
        } else {
            return numbers.stream()
                    .parallel()
                    .mapToObj(n -> combinations(TxSet.of(numbers.filter(x -> x < n)), size-1)
                            .map(set -> TxSet.or(set, n))
                    )
                    .flatMap(x -> x);
        }
    }
}
