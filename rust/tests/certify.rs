//! Certificates of U (`taxman certify`, `taxman::certify`): on the committed chain every game that rests on the frame
//! model comes out certified, a score planted wrong comes out beaten with a game that the referee accepts, a search
//! that is stopped keeps its checkpoint and ends as an uninterrupted one does (with a little work redone), and the
//! file of certificates and the reader of the prover's barred bounds are strict about what they are given.
//!
//! The scores of the chain are proven optimal to board 2175, so a game whose line in the barred bounds says it `needs`
//! a certificate must be certified.  The data are the real ones: `results/strict.jsonl` over the table built into the
//! binary for the games, and `prover/data/upper_*.jsonl` for the open members.  Every search runs on one thread, so
//! that its tree and its node count are the same on every run; the tests print the summary line of each game they
//! certify (`cargo test --test certify -- --nocapture` shows them).

mod common;

use common::{options, path, results_path, taxman, Run, TempDir};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;
use taxman::bits::{width_for, words_for};
use taxman::certify::{certify_game, read_upper, Certificate, CertificateFile, Upper, Verdict};
use taxman::game::{replay, replay_finished, Optimal};
use taxman::json::Json;
use taxman::solver::SolveOptions;

// ---------------------------------------------------------------------------------------------------------
// The data and the helpers

/// What the tests read: the recorded games of the chain and the lines of the barred bounds.
struct Fixtures {
    known: Optimal,
    upper: BTreeMap<u32, Upper>,
}

/// The prover's barred bounds that cover the chain, `prover/data/upper_*.jsonl`: boards 2 to 1300 and 1301 to 2175.
fn upper_files() -> Vec<PathBuf> {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("prover/data");
    ["upper_2_1300.jsonl", "upper_1301_2175.jsonl"].iter().map(|name| data.join(name)).collect()
}

/// The recorded games (the built-in table, then `results/strict.jsonl`) and the barred bounds, read once.
fn fixtures() -> &'static Fixtures {
    static FIXTURES: OnceLock<Fixtures> = OnceLock::new();
    FIXTURES.get_or_init(|| {
        let known = Optimal::built_in().with_results_files(&[results_path("strict.jsonl")]).expect("the chain");
        let upper = read_upper(&upper_files()).expect("the barred bounds");
        Fixtures { known, upper }
    })
}

/// The boards of `range` that need a certificate according to the barred bounds, ascending.
fn needing(range: RangeInclusive<u32>) -> Vec<u32> {
    range.filter(|n| fixtures().upper[n].needs).collect()
}

/// Certifies the recorded game of `n` with the committed data and `opts`.
fn certify(n: u32, opts: &SolveOptions) -> Certificate {
    let Fixtures { known, upper } = fixtures();
    certify_game(n, known, &upper[&n], opts).unwrap_or_else(|e| panic!("{e}"))
}

/// Checks a certificate that says nothing beats the recorded game of `n` against the data it was made from: the
/// verdict, the score, the open members, the sizes of the problem (the one-sided candidates are the open members and
/// the sources greedy rejected), a search that did something where there was something to search, and the run.
fn assert_certified(c: &Certificate, n: u32) {
    let Fixtures { known, upper } = fixtures();
    let line = &upper[&n];
    assert_eq!(c.verdict, Verdict::Certified, "{}", c.summary_line());
    assert!(c.beaten_by.is_none(), "n={n}: a certified game is beaten by nothing");
    assert_eq!(c.n, n);
    assert_eq!(Some(c.score), known.score(n), "n={n}: the score is the recorded one");
    assert_eq!(c.score, line.score, "n={n}: the bounds were found against the recorded score");
    assert_eq!(c.open, line.open, "n={n}: the open members are the line's");
    assert_eq!(c.one_sided, c.open.len() as u32 + c.rejected, "n={n}: the one-sided candidates are O and R");
    assert!(c.frames > 0, "n={n}");
    if c.two_sided + c.one_sided > 0 {
        assert!(c.nodes > 0, "n={n}: a problem with candidates takes a search: {}", c.summary_line());
    }
    assert_eq!(c.threads, 1);
    let own = width_for(words_for(n)).unwrap();
    assert!(c.width == own || c.width == 128, "n={n}: searched at width {}, not {own} or 128", c.width);
}

/// Certifies each of `boards` on one thread, checks and prints each certificate, and returns the nodes and the seconds
/// of the lot.
fn certify_all(boards: &[u32]) -> (u64, f64) {
    let (start, mut nodes) = (Instant::now(), 0);
    for &n in boards {
        let c = certify(n, &options(1));
        assert_certified(&c, n);
        println!("{}", c.summary_line());
        nodes += c.nodes;
    }
    (nodes, start.elapsed().as_secs_f64())
}

/// A game of a results file: the board, the score and the moves.
type Game<'a> = (u32, u64, &'a [u32]);

/// Writes a results file with `games` (a completed search a line; the reader of the chain takes only `n`, `score`,
/// `moves` and `completed` from a line) and returns the table of optimal games over which it is read.
fn known_with(file: &Path, games: &[Game]) -> Optimal {
    let line = |&(n, score, moves): &Game| {
        let line = Json::object([
            ("n", Json::from(n)),
            ("score", Json::from(score)),
            ("moves", Json::array(moves.iter().copied())),
            ("completed", Json::Bool(true)),
        ]);
        format!("{line}\n")
    };
    fs::write(file, games.iter().map(line).collect::<String>()).unwrap();
    Optimal::built_in().with_results_files(&[file.to_owned()]).unwrap()
}

/// A wrong score for board `n`, planted in the results file `dir/results.jsonl` that holds the chain's games of
/// `n − 1` and `n`, except that the game of `n` has its smallest move up to n/2 left out and its score lowered by that
/// move.  A game stays legal when a move is left out (what the move would have taxed stays in the pot, and none of its
/// divisors is played after it), and it plays the same numbers above n/2, so it is still a promotion set of the
/// extended problem; but its score is below the optimum, which the chain has proven, so a search must beat it.
/// Returns the file, the planted score and the move.
fn plant_wrong_score(dir: &Path, n: u32) -> (PathBuf, u64, u32) {
    let known = &fixtures().known;
    let (score, moves) = (known.score(n).unwrap(), known.moves(n).unwrap());
    let left_out = moves.iter().copied().filter(|&m| 2 * m <= n).min().expect("a move up to n/2");
    let kept: Vec<u32> = moves.iter().copied().filter(|&m| m != left_out).collect();
    let planted = score - u64::from(left_out);
    assert_eq!(replay(n, &kept), Ok(planted), "n={n}: a game without a move is a legal game");
    let file = dir.join("results.jsonl");
    known_with(&file, &[(n - 1, known.score(n - 1).unwrap(), known.moves(n - 1).unwrap()), (n, planted, &kept)]);
    (file, planted, left_out)
}

/// The lines of a file of JSON lines.
fn json_lines(file: &Path) -> Vec<Json> {
    let text = fs::read_to_string(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    text.lines().map(|line| Json::parse(line).unwrap_or_else(|e| panic!("{line}: {e}"))).collect()
}

/// The verdicts of the lines of a file `taxman certify` wrote, in order.
fn verdicts(file: &Path) -> Vec<String> {
    let verdict = |line: Json| line.get("verdict").and_then(Json::as_str).unwrap().to_owned();
    json_lines(file).into_iter().map(verdict).collect()
}

/// Runs `taxman certify RANGE` on the games of `results` and the committed barred bounds, appending to `out`, with one
/// thread and no progress lines, and the flags `extra` besides.
fn run_certify(range: &str, results: &Path, out: &Path, extra: &[&str]) -> Run {
    let uppers = upper_files();
    let mut args = vec!["certify", range, "--results", path(results), "--out", path(out)];
    args.extend(["--threads", "1", "--progress", "0"]);
    args.extend(uppers.iter().flat_map(|file| ["--upper", path(file)]));
    args.extend_from_slice(extra);
    taxman(&args)
}

// ---------------------------------------------------------------------------------------------------------
// The chain

/// Every fourth game of 600..=1300 that needs a certificate, and 1287, the one with the biggest search of them: all
/// certified, with the data that each certificate says it was made from.  (All of them are in
/// `every_game_that_needs_a_certificate_to_1300_is_certified`.)
#[test]
fn a_sample_of_the_games_to_1300_that_need_a_certificate_is_certified() {
    let mut boards: Vec<u32> = needing(600..=1300).into_iter().step_by(4).collect();
    boards.push(1287);
    boards.sort_unstable();
    boards.dedup();
    assert!(boards.len() >= 90, "{boards:?}");
    let (nodes, seconds) = certify_all(&boards);
    println!("{} games certified: {nodes} nodes, {seconds:.1} s", boards.len());
}

/// The whole of 600..=1300, one game after the other on one thread: every game that needs a certificate is certified.
#[test]
#[ignore = "certifies the 377 games of 600..=1300 that need a certificate, one thread: about 40 s of search"]
fn every_game_that_needs_a_certificate_to_1300_is_certified() {
    let boards = needing(600..=1300);
    assert_eq!(boards.len(), 377, "the barred bounds say these boards need a certificate: {boards:?}");
    let (nodes, seconds) = certify_all(&boards);
    println!("{} games certified: {nodes} nodes, {seconds:.1} s", boards.len());
}

/// Games past the table built into the binary (it ends at 1300), where the games of n − 1 and n come from the committed
/// chain alone and the open members from the second file of bounds: a few of the cheap ones are certified too.
#[test]
fn games_past_1300_that_need_a_certificate_are_certified() {
    let boards = [1302, 1304, 1305, 1309];
    for n in boards {
        assert!(fixtures().upper[&n].needs, "n={n} is a game that needs a certificate");
    }
    let (nodes, seconds) = certify_all(&boards);
    println!("{} games certified: {nodes} nodes, {seconds:.1} s", boards.len());
}

/// The certificate as the command writes it: the fields, the sizes, the digest of the problem and no counterexample;
/// and the summary line has the numbers of the game and its verdict.
#[test]
fn a_certificate_writes_the_fields_of_its_search() {
    let n = 1302;
    let c = certify(n, &options(1));
    assert_certified(&c, n);
    let json = Json::parse(&c.to_json().to_string()).unwrap();
    let number = |name: &str| json.get(name).and_then(Json::as_u64).unwrap_or_else(|| panic!("no number {name:?}"));
    assert_eq!(json.get("verdict").and_then(Json::as_str), Some("certified"));
    assert_eq!(json.get("open").and_then(Json::as_u32_vec), Some(vec![682, 715, 754]));
    assert_eq!(
        [number("n"), number("score"), number("rejected"), number("nodes"), number("threads"), number("width")],
        [1302, c.score, u64::from(c.rejected), c.nodes, 1, c.width as u64]
    );
    assert_eq!(number("frames"), c.frames as u64);
    assert_eq!(number("in_the_bag"), c.in_the_bag);
    assert_eq!(number("two_sided"), u64::from(c.two_sided));
    assert_eq!(number("one_sided"), u64::from(c.one_sided));
    assert_eq!(json.get("digest").and_then(Json::as_str).map(str::len), Some(16));
    assert!(json.get("beaten_by").is_none() && json.get("build").is_some(), "{json}");
    let summary = c.summary_line();
    for text in ["n=1302", "V=533559", "|O|=3", "certified", &c.nodes.to_string()] {
        assert!(summary.contains(text), "no {text:?} in {summary:?}");
    }
    println!("{summary}");
}

// ---------------------------------------------------------------------------------------------------------
// A planted wrong score

/// A wrong score planted on a board (see [`plant_wrong_score`]) is beaten, with the real barred bounds: the verdict
/// is `Beaten` and the game found is one that the referee accepts, scores more than the planted score, and, since the
/// chain's score is the optimum, scores that.
#[test]
fn a_planted_wrong_score_is_beaten_by_a_game_the_referee_accepts() {
    let Fixtures { known, upper } = fixtures();
    for n in [1302, 1305, 1309] {
        let dir = TempDir::new("certify-planted");
        let (file, planted, left_out) = plant_wrong_score(&dir, n);
        let table = Optimal::built_in().with_results_files(&[file]).unwrap();
        assert_eq!(table.score(n), Some(planted));

        let c = certify_game(n, &table, &upper[&n], &options(1)).unwrap_or_else(|e| panic!("{e}"));
        println!("{} (the smallest move up to n/2, {left_out}, left out)", c.summary_line());
        assert_eq!(c.verdict, Verdict::Beaten, "n={n}: {}", c.summary_line());
        assert_eq!(c.score, planted);
        let (found, moves) = c.beaten_by.as_ref().expect("a beaten game has the game that beats it");
        assert_eq!(replay_finished(n, moves), Ok(*found), "n={n}: the referee accepts the game found");
        assert!(*found > planted, "n={n}: the game found scores {found}, not more than {planted}");
        assert_eq!(Some(*found), known.score(n), "n={n}: the best game is the proven optimum");
        assert!(c.summary_line().contains(&format!("by a game of {found}")), "{}", c.summary_line());
        let json = Json::parse(&c.to_json().to_string()).unwrap();
        assert_eq!(json.get("verdict").and_then(Json::as_str), Some("beaten"));
        let beating = json.get("beaten_by").expect("beaten_by");
        assert_eq!(beating.get("score").and_then(Json::as_u64), Some(*found));
        assert_eq!(beating.get("moves").and_then(Json::as_u32_vec).as_ref(), Some(moves));
    }
}

/// The same through the binary: status 2, the `beaten` line in `--out` and the same line in `OUT.counterexample.jsonl`.
#[test]
fn the_command_exits_with_status_2_on_a_planted_wrong_score_and_writes_the_counterexample() {
    let n = 1302;
    let dir = TempDir::new("certify-planted-command");
    let (results, planted, _) = plant_wrong_score(&dir, n);
    let real = fixtures().known.score(n).unwrap();
    let out = dir.join("out.jsonl");

    let run = run_certify(&format!("{n}-{n}"), &results, &out, &[]);
    assert_eq!(run.code, Some(2), "{}{}", run.stdout, run.stderr);
    let message = format!("counterexample: n={n}: a game of {real} beats the recorded score {planted}");
    assert!(run.stderr.contains(&message), "{}", run.stderr);
    assert!(run.stdout.contains(&format!("beaten by a game of {real}")), "{}", run.stdout);
    assert!(run.stdout.contains(&format!("  the game of {real}: [")), "{}", run.stdout);

    assert_eq!(verdicts(&out), ["beaten"]);
    let line = &json_lines(&out)[0];
    assert_eq!(line.get("score").and_then(Json::as_u64), Some(planted));
    let beating = line.get("beaten_by").unwrap();
    assert_eq!(beating.get("score").and_then(Json::as_u64), Some(real));
    let moves = beating.get("moves").and_then(Json::as_u32_vec).unwrap();
    assert_eq!(replay_finished(n, &moves), Ok(real));
    let counterexamples = dir.join("out.counterexample.jsonl");
    assert_eq!(fs::read_to_string(&counterexamples).unwrap(), fs::read_to_string(&out).unwrap());

    // A counterexample stays one: the same command again says so, with status 2, and searches nothing.
    let again = run_certify(&format!("{n}-{n}"), &results, &out, &[]);
    assert_eq!(again.code, Some(2), "{}{}", again.stdout, again.stderr);
    let recorded = format!("counterexample: n={n}: --out already records a game that beats the recorded score");
    assert!(again.stderr.contains(&recorded), "{}", again.stderr);
    assert_eq!(verdicts(&out), ["beaten"], "the rerun added no line");
}

/// A game is refused, with an error that says why, instead of certified or beaten: when the barred bounds are those of
/// another board, when they were found against a score above the recorded one (nothing beats the recorded score, but
/// they certify the open members against a higher one, so nothing is proven), when the results have no game of the
/// board, when the recorded game does not replay to its score, and when it does not play the sources above n/2 that
/// setup keeps.
#[test]
fn a_game_that_a_certificate_cannot_be_made_for_is_refused() {
    let Fixtures { known, upper } = fixtures();
    let (n, line) = (1302, &upper[&1302]);
    let (score, moves) = (known.score(n).unwrap(), known.moves(n).unwrap());
    let dir = TempDir::new("certify-refused");
    let refused = |table: &Optimal, line: &Upper, why: &str| {
        let e = certify_game(n, table, line, &options(1)).err().unwrap_or_else(|| panic!("n={n}: not refused: {why}"));
        assert!(e.contains(&format!("n={n}: ")) && e.contains(why), "expected {why:?} in {e:?}");
    };

    refused(known, &upper[&1304], "the barred bounds given are those of n=1304");
    let higher = Upper { score: score + 1, ..line.clone() };
    refused(known, &higher, &format!("found against {}, more: not a certificate", score + 1));
    refused(Optimal::built_in(), line, "the results have no game of n");

    let wrong = known_with(
        &dir.join("score.jsonl"),
        &[(n - 1, known.score(n - 1).unwrap(), known.moves(n - 1).unwrap()), (n, score + 1, moves)],
    );
    refused(&wrong, line, &format!("the recorded game scores {score}, not the {} recorded", score + 1));

    // The last of the moves above n/2 left out: a legal game that scores less and plays one source fewer than setup
    // keeps.
    let dropped = moves.iter().copied().rev().find(|&m| 2 * m > n).unwrap();
    let rest: Vec<u32> = moves.iter().copied().filter(|&m| m != dropped).collect();
    let game =
        [(n - 1, known.score(n - 1).unwrap(), known.moves(n - 1).unwrap()), (n, score - u64::from(dropped), &rest[..])];
    let short = known_with(&dir.join("short.jsonl"), &game);
    refused(
        &short,
        line,
        &format!("does not play U*, the sources setup keeps, above n/2: it plays [] besides, and not [{dropped}]"),
    );
}

// ---------------------------------------------------------------------------------------------------------
// Stopped and resumed

/// The game the stop-and-resume tests use: its search takes 4,891 nodes on one thread, which a limit of 1,000 nodes
/// (the engine looks at a limit every 1,024 steps of a searcher) cuts into several pieces.
const RESUMED: u32 = 1302;

/// Checks the nodes of a search that was stopped and resumed against those of the same search uninterrupted.  The
/// tree is the same, but a stop loses the unit of work in flight, which the resumed search does again (for 1302,
/// 4,891 nodes in one run and 5,851 in six, and a plain `solve` of a board does the same): so the nodes over all the
/// runs are never fewer than the uninterrupted search's, and nowhere near double.
fn assert_resumed_work(resumed: u64, uninterrupted: u64) {
    assert!(
        (uninterrupted..2 * uninterrupted).contains(&resumed),
        "{resumed} nodes over all the runs of a resumed search, {uninterrupted} in one run"
    );
}

/// A search stopped by a node limit returns `Incomplete` and keeps its checkpoint; the same call again and again ends
/// `Certified`, after at least two pieces that stopped, with the certificate of an uninterrupted search: the same
/// problem, the same verdict, and about the same work.
#[test]
fn a_stopped_search_resumes_to_the_certificate_of_an_uninterrupted_one() {
    let n = RESUMED;
    let whole = certify(n, &options(1));
    assert_certified(&whole, n);
    assert!(whole.nodes > 3_000, "n={n}: {} nodes are too few for pieces of 1,000", whole.nodes);

    let dir = TempDir::new("certify-resume");
    let ckpt = dir.join("ckpt");
    fs::create_dir_all(&ckpt).unwrap();
    let opts = SolveOptions { checkpoint_dir: Some(ckpt.clone()), node_limit: Some(1000), ..options(1) };
    let checkpoint = ckpt.join(format!("n{n:04}.ckpt"));
    assert!(!checkpoint.exists());

    let mut pieces: Vec<u64> = Vec::new();
    let last = loop {
        let piece = certify(n, &opts);
        if piece.verdict != Verdict::Incomplete {
            break piece;
        }
        println!("{}", piece.summary_line());
        assert!(piece.beaten_by.is_none());
        assert!(checkpoint.exists(), "a stopped search keeps its checkpoint");
        assert!(pieces.last().is_none_or(|&before| piece.nodes > before), "a piece made no progress: {pieces:?}");
        pieces.push(piece.nodes);
        assert!(pieces.len() < 100, "{pieces:?}: the search does not end");
    };
    println!("{}  (after {} pieces that stopped)", last.summary_line(), pieces.len());
    assert!(pieces.len() >= 2, "n={n}: the search ended after {} pieces that stopped", pieces.len());

    assert_certified(&last, n);
    assert_resumed_work(last.nodes, whole.nodes);
    assert_eq!(last.digest, whole.digest, "the same problem");
    let sizes = |c: &Certificate| (c.frames, c.two_sided, c.one_sided, c.rejected, c.in_the_bag, c.width);
    assert_eq!(sizes(&last), sizes(&whole));
    assert!(last.seconds_all_runs >= last.seconds, "the seconds of all runs include this run's");

    // The library leaves a finished search's checkpoint for the caller to delete (the command does, once the line is
    // safe), and a call that finds it reports the finished search again.
    assert!(checkpoint.exists());
    let again = certify(n, &opts);
    assert_eq!((again.verdict, again.nodes, again.digest), (Verdict::Certified, last.nodes, last.digest));
}

/// The same through the binary: a run with a node limit stops with exit status 0 and an `incomplete` line; the same
/// command again and again ends with `certified`, and the checkpoint goes with the last; and a further run skips the
/// game, which the summary says.
#[test]
fn the_command_stops_resumes_and_then_skips_a_game() {
    let n = RESUMED;
    let dir = TempDir::new("certify-resume-command");
    let (results, out, ckpt) = (results_path("strict.jsonl"), dir.join("out.jsonl"), dir.join("ckpt"));
    let flags = ["--node-limit", "1000", "--checkpoint-dir", path(&ckpt)];
    let range = format!("{n}-{n}");

    let run = run_certify(&range, &results, &out, &flags);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("incomplete"), "{}", run.stdout);
    let stopped =
        format!("stopped: the search for n={n} hit a limit before it finished; run the same command to resume it");
    assert!(run.stdout.contains(&stopped), "{}", run.stdout);
    assert_eq!(verdicts(&out), ["incomplete"]);
    assert!(ckpt.join(format!("n{n:04}.ckpt")).exists(), "a stopped search keeps its checkpoint");

    let mut runs = 1;
    let last = loop {
        let run = run_certify(&range, &results, &out, &flags);
        assert_eq!(run.code, Some(0), "{}", run.stderr);
        runs += 1;
        if !run.stdout.contains("stopped:") {
            break run;
        }
        assert!(verdicts(&out).iter().all(|verdict| verdict == "incomplete"));
        assert!(runs < 100, "the search does not end");
    };
    println!("{}  (in {runs} runs)", last.stdout.lines().next().unwrap());
    assert!(runs >= 3, "the game was certified in {runs} runs");
    assert!(
        last.stdout.contains("certified") && last.stdout.contains(&format!("{n}..={n}: certified 1")),
        "{}",
        last.stdout
    );
    let all = verdicts(&out);
    assert_eq!(all.len(), runs);
    assert_eq!(all.last().map(String::as_str), Some("certified"));
    assert!(all[..runs - 1].iter().all(|verdict| verdict == "incomplete"), "{all:?}");
    assert!(!ckpt.join(format!("n{n:04}.ckpt")).exists(), "a finished game removes its checkpoint");
    assert!(!dir.join("out.counterexample.jsonl").exists());

    // The certificate of the pieces is the one of an uninterrupted search.
    let whole = certify(n, &options(1));
    let line = &json_lines(&out)[runs - 1];
    assert_resumed_work(line.get("nodes").and_then(Json::as_u64).unwrap(), whole.nodes);
    assert_eq!(line.get("score").and_then(Json::as_u64), Some(whole.score));
    assert_eq!(line.get("digest").and_then(Json::as_str), Some(taxman::solver::digest_text(whole.digest).as_str()));

    let before = fs::read(&out).unwrap();
    let run = run_certify(&range, &results, &out, &flags);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains(&format!("{n}..={n}: already in --out 1")), "{}", run.stdout);
    assert!(!run.stdout.contains("nodes="), "a game with a verdict is not searched again: {}", run.stdout);
    assert_eq!(fs::read(&out).unwrap(), before, "a game with a verdict adds no line");
}

// ---------------------------------------------------------------------------------------------------------
// The command

/// The command on a range: the games that do not need a certificate are passed over (they are counted), each of the
/// others gets a `certified` line in `--out` and one on stdout, and a rerun on a longer range skips what is done.
#[test]
fn the_command_certifies_a_range_and_skips_what_is_done() {
    let dir = TempDir::new("certify-range");
    let (results, out) = (results_path("strict.jsonl"), dir.join("out.jsonl"));

    let run = run_certify("1301-1305", &results, &out, &[]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    for n in [1302, 1304, 1305] {
        assert!(run.stdout.contains(&format!("n={n} ")), "{}", run.stdout);
    }
    assert_eq!(run.stdout.lines().count(), 4, "{}", run.stdout);
    assert!(run.stdout.contains("1301..=1305: certified 3, not needing one 2"), "{}", run.stdout);
    assert_eq!(run.stderr, "");
    assert_eq!(verdicts(&out), ["certified"; 3]);
    let boards: Vec<_> = json_lines(&out).iter().map(|line| line.get("n").and_then(Json::as_u32).unwrap()).collect();
    assert_eq!(boards, [1302, 1304, 1305]);

    // Longer, 1308 and 1309 are the new ones that need a certificate.
    let run = run_certify("1301-1309", &results, &out, &[]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("1301..=1309: already in --out 3, certified 2, not needing one 4"), "{}", run.stdout);
    assert_eq!(run.stdout.lines().count(), 3, "{}", run.stdout);
    assert_eq!(verdicts(&out), ["certified"; 5]);
}

/// An `--out` file that another process holds open (here this one, appending to it as a run does) is refused with
/// status 1 and nothing is written; once it is closed, the command goes on with it.
#[test]
fn the_command_refuses_an_out_file_another_process_has_open() {
    let dir = TempDir::new("certify-lock");
    let (results, out) = (results_path("strict.jsonl"), dir.join("out.jsonl"));
    let held = CertificateFile::open(&out).unwrap();
    let run = run_certify("1302", &results, &out, &[]);
    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert!(run.stderr.contains("out.jsonl is in use by another process"), "{}", run.stderr);
    assert_eq!(run.stdout, "");
    assert_eq!(fs::read_to_string(&out).unwrap(), "");
    drop(held);
    let run = run_certify("1302", &results, &out, &[]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(verdicts(&out), ["certified"]);
}

// ---------------------------------------------------------------------------------------------------------
// The file of certificates

/// A certificate with the verdict `verdict`, made by hand, for the tests of the file.
fn certificate(n: u32, verdict: Verdict) -> Certificate {
    let beaten_by = (verdict == Verdict::Beaten).then(|| (100, vec![7, 4, 6]));
    Certificate {
        n,
        score: 99,
        open: vec![30, 42],
        rejected: 1,
        frames: 3,
        two_sided: 5,
        one_sided: 3,
        in_the_bag: 20,
        nodes: 17,
        seconds: 0.25,
        seconds_all_runs: 0.5,
        threads: 1,
        width: 8,
        verdict,
        digest: 0xfeed,
        beaten_by,
    }
}

/// Appends a torn line to `file`: the start of a line, cut off by a crash.
fn append_raw(file: &Path, text: &str) {
    OpenOptions::new().append(true).open(file).unwrap().write_all(text.as_bytes()).unwrap();
}

/// A verdict is read back from the name it is written with, and from nothing else.
#[test]
fn verdicts_name_themselves() {
    for verdict in [Verdict::Certified, Verdict::Beaten, Verdict::Incomplete] {
        assert_eq!(Verdict::parse(verdict.as_str()), Some(verdict));
    }
    assert_eq!(Verdict::parse("Certified"), None);
    assert_eq!(Verdict::parse(""), None);
}

/// A last line that a run was killed while writing is cut off when the file is opened: whatever of it was written,
/// also a whole line of JSON that lacks its newline (a line is written with its newline, in one write) and a file of
/// one torn line.  The lines before it are kept, with their verdicts, and the file takes new lines after them.
#[test]
fn a_torn_last_line_is_cut_off_on_opening() {
    let dir = TempDir::new("certify-torn");
    let file = dir.join("out.jsonl");
    let mut certificates = CertificateFile::open(&file).unwrap();
    certificates.append(&certificate(30, Verdict::Certified)).unwrap();
    certificates.append(&certificate(32, Verdict::Beaten)).unwrap();
    drop(certificates);
    let whole = fs::read_to_string(&file).unwrap();
    assert_eq!(whole.lines().count(), 2);

    let whole_line = certificate(34, Verdict::Certified).to_json().to_string();
    for torn in ["{\"n\":34,\"verdict\":\"cert", "{\"n\"", "{", whole_line.as_str()] {
        append_raw(&file, torn);
        assert_ne!(fs::read_to_string(&file).unwrap(), whole);
        let certificates = CertificateFile::open(&file).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), whole, "{torn:?} is cut off");
        assert_eq!(
            (certificates.verdict(30), certificates.verdict(32)),
            (Some(Verdict::Certified), Some(Verdict::Beaten))
        );
        assert_eq!(certificates.verdict(34), None, "a torn line gives no verdict: {torn:?}");
    }

    // After the cut the file takes a new line where the torn one began, and reads back whole.
    let mut certificates = CertificateFile::open(&file).unwrap();
    certificates.append(&certificate(34, Verdict::Certified)).unwrap();
    drop(certificates);
    assert_eq!(verdicts(&file), ["certified", "beaten", "certified"]);
    assert_eq!(CertificateFile::open(&file).unwrap().verdict(34), Some(Verdict::Certified));

    // A file that is only a torn line is empty once it is opened.
    let only = dir.join("only.jsonl");
    fs::write(&only, "{\"n\":3,\"verd").unwrap();
    assert_eq!(CertificateFile::open(&only).unwrap().verdict(3), None);
    assert_eq!(fs::read_to_string(&only).unwrap(), "");
}

/// An `incomplete` line is no verdict, in the file that holds it and in one that is opened later, so a run goes on
/// with that game; `certified` and `beaten` are verdicts, and the last one a game has is the one that counts.
#[test]
fn incomplete_lines_do_not_count_as_a_verdict() {
    let dir = TempDir::new("certify-incomplete");
    let file = dir.join("out.jsonl");
    let mut certificates = CertificateFile::open(&file).unwrap();
    assert_eq!(certificates.verdict(30), None);
    certificates.append(&certificate(30, Verdict::Incomplete)).unwrap();
    certificates.append(&certificate(30, Verdict::Incomplete)).unwrap();
    certificates.append(&certificate(32, Verdict::Incomplete)).unwrap();
    assert_eq!((certificates.verdict(30), certificates.verdict(32)), (None, None));
    drop(certificates);

    let mut certificates = CertificateFile::open(&file).unwrap();
    assert_eq!((certificates.verdict(30), certificates.verdict(32)), (None, None));
    certificates.append(&certificate(30, Verdict::Certified)).unwrap();
    certificates.append(&certificate(32, Verdict::Beaten)).unwrap();
    assert_eq!((certificates.verdict(30), certificates.verdict(32)), (Some(Verdict::Certified), Some(Verdict::Beaten)));
    // A search that is stopped again after a verdict (it cannot be, but the file takes what it is given) does not
    // take the verdict away.
    certificates.append(&certificate(30, Verdict::Incomplete)).unwrap();
    assert_eq!(certificates.verdict(30), Some(Verdict::Certified));
    drop(certificates);

    let certificates = CertificateFile::open(&file).unwrap();
    assert_eq!((certificates.verdict(30), certificates.verdict(32)), (Some(Verdict::Certified), Some(Verdict::Beaten)));
    assert_eq!(certificates.verdict(34), None);
    assert_eq!(verdicts(&file), ["incomplete", "incomplete", "incomplete", "certified", "beaten", "incomplete"]);
}

/// A second `open` of the file while the first is held is refused, whoever holds it, and works once the first is
/// dropped; a line that is complete and is no certificate is refused too, naming the file and the line.
#[test]
fn a_certificate_file_is_locked_while_it_is_open() {
    let dir = TempDir::new("certify-file-lock");
    let file = dir.join("out.jsonl");
    let held = CertificateFile::open(&file).unwrap();
    let Err(message) = CertificateFile::open(&file) else { panic!("a file that is held was opened a second time") };
    assert!(message.contains("out.jsonl is in use by another process"), "{message}");
    drop(held);
    let mut reopened = CertificateFile::open(&file).unwrap();
    reopened.append(&certificate(30, Verdict::Certified)).unwrap();
    drop(reopened);

    for (text, why) in [
        ("not json\n", "line 1"),
        ("{\"n\":30,\"verdict\":\"certified\"}\n{\"n\":32}\n", "line 2: no verdict"),
        ("{\"verdict\":\"beaten\"}\n", "line 1: no board size"),
        ("{\"n\":30,\"verdict\":\"maybe\"}\n", "line 1: no verdict"),
    ] {
        let bad = dir.join("bad.jsonl");
        fs::write(&bad, text).unwrap();
        let Err(message) = CertificateFile::open(&bad) else { panic!("{text:?} was read as certificates") };
        assert!(message.contains("bad.jsonl") && message.contains(why), "{text:?}: {message}");
    }
}

/// The file of counterexamples is beside the certificates', named by dropping a `.jsonl` ending and adding
/// `.counterexample.jsonl`, and takes the lines it is given.
#[test]
fn counterexamples_go_to_a_file_beside_the_certificates() {
    let dir = TempDir::new("certify-counterexamples");
    for (name, counterexamples) in [("out.jsonl", "out.counterexample.jsonl"), ("out", "out.counterexample.jsonl")] {
        let certificates = CertificateFile::open(&dir.join(name)).unwrap();
        assert_eq!(certificates.counterexample_path(), dir.join(counterexamples));
        let first = certificates.append_counterexample(&certificate(30, Verdict::Beaten)).unwrap();
        certificates.append_counterexample(&certificate(32, Verdict::Beaten)).unwrap();
        assert_eq!(first, dir.join(counterexamples));
        assert_eq!(verdicts(&first), ["beaten", "beaten"]);
        let _ = fs::remove_file(&first);
    }
}

// ---------------------------------------------------------------------------------------------------------
// The barred bounds

/// A line of the barred bounds, as the prover writes it: `n`, `score` and `needs`, and where it needs a certificate the
/// `open` members, each as `[u, omega, margin]`.
fn upper_line(n: u32, score: u64, open: Option<&[[u32; 3]]>) -> String {
    match open {
        Some(open) => {
            let members: Vec<String> =
                open.iter().map(|[u, omega, margin]| format!("[{u},{omega},{margin}]")).collect();
            format!(
                "{{\"n\":{n},\"score\":{score},\"relation\":\"none\",\"needs\":1,\"open\":[{}],\"secs\":0.4}}\n",
                members.join(",")
            )
        }
        None => format!("{{\"n\":{n},\"score\":{score},\"relation\":\"prime\",\"needs\":0,\"secs\":0.002}}\n"),
    }
}

/// Writes `text` to `name` in `dir` and returns the path.
fn write_file(dir: &Path, name: &str, text: &str) -> PathBuf {
    let file = dir.join(name);
    fs::write(&file, text).unwrap();
    file
}

/// Two files with the same line for a board are fine, and so is a board twice in one file and a blank line; the lines
/// are those of the boards (the open members ascending, none on a line that needs no certificate, whatever it lists).
#[test]
fn read_upper_takes_the_same_line_in_two_files() {
    let dir = TempDir::new("upper-same");
    let line_1302 = upper_line(1302, 533559, Some(&[[754, 3, 15], [682, 3, 87], [715, 3, 10]]));
    let line_1305 = upper_line(1305, 535468, Some(&[[682, 3, 50]]));
    let prime = upper_line(1303, 533561, None);
    let first = write_file(&dir, "a.jsonl", &format!("{line_1302}\n{prime}{line_1302}{line_1305}"));
    let second = write_file(&dir, "b.jsonl", &format!("{prime}{line_1305}{}", upper_line(1304, 534237, Some(&[]))));

    let lines = read_upper(&[first.clone(), second.clone()]).unwrap();
    assert_eq!(lines.keys().copied().collect::<Vec<_>>(), [1302, 1303, 1304, 1305]);
    assert_eq!(lines[&1302], Upper { n: 1302, score: 533559, needs: true, open: vec![682, 715, 754] });
    assert_eq!(lines[&1303], Upper { n: 1303, score: 533561, needs: false, open: vec![] });
    assert_eq!(lines[&1304], Upper { n: 1304, score: 534237, needs: true, open: vec![] });
    assert_eq!(lines[&1305], Upper { n: 1305, score: 535468, needs: true, open: vec![682] });
    // The order of the files does not matter, and no files are no lines.
    assert_eq!(read_upper(&[second, first]).unwrap(), lines);
    assert!(read_upper(&[]).unwrap().is_empty());

    // A line that needs no certificate has no open members, whatever it lists.
    let listed = write_file(&dir, "c.jsonl", "{\"n\":1301,\"score\":5,\"needs\":0,\"open\":[[682,3,87]]}\n");
    assert_eq!(read_upper(&[listed]).unwrap()[&1301], Upper { n: 1301, score: 5, needs: false, open: vec![] });
}

/// A board with two different lines is refused, in two files and in one, whatever differs (the score, whether it needs
/// a certificate, the open members), and the message names the second file, its line and the board.
#[test]
fn read_upper_refuses_two_different_lines_for_a_board() {
    let dir = TempDir::new("upper-differ");
    let line = upper_line(1302, 533559, Some(&[[682, 3, 87], [715, 3, 10]]));
    let first = write_file(&dir, "a.jsonl", &line);
    let others = [
        upper_line(1302, 533560, Some(&[[682, 3, 87], [715, 3, 10]])),
        upper_line(1302, 533559, Some(&[[682, 3, 87]])),
        upper_line(1302, 533559, Some(&[[682, 3, 87], [715, 3, 10], [754, 3, 15]])),
        upper_line(1302, 533559, None),
    ];
    for (i, other) in others.iter().enumerate() {
        let second = write_file(&dir, &format!("b{i}.jsonl"), &format!("{}{other}", upper_line(1301, 532272, None)));
        let Err(message) = read_upper(&[first.clone(), second]) else { panic!("{other:?} was taken") };
        let named = format!("b{i}.jsonl: line 2: n=1302 differs from an earlier line of the files");
        assert!(message.contains(&named), "{message}");

        let one = write_file(&dir, &format!("c{i}.jsonl"), &format!("{line}{other}"));
        let Err(message) = read_upper(&[one]) else { panic!("{other:?} was taken") };
        assert!(message.contains(&format!("c{i}.jsonl: line 2: n=1302 differs")), "{message}");
    }
}

/// A line that is no line of the bounds is refused with the file, the line and what is wrong with it: a `needs` that is
/// neither 0 nor 1, fields that are missing or bad, an `open` that is no list of members, text that is no JSON, and a
/// file that is not there.
#[test]
fn read_upper_refuses_a_bad_line() {
    let dir = TempDir::new("upper-bad");
    let ok = upper_line(1301, 532272, None);
    let cases = [
        ("{\"n\":1302,\"score\":5,\"needs\":2}", "line 2: n=1302: a bad \"needs\""),
        ("{\"n\":1302,\"score\":5,\"needs\":-1}", "line 2: n=1302: a bad \"needs\""),
        ("{\"n\":1302,\"score\":5,\"needs\":\"yes\"}", "line 2: n=1302: a bad \"needs\""),
        ("{\"n\":1302,\"score\":5,\"needs\":true}", "line 2: n=1302: a bad \"needs\""),
        ("{\"n\":1302,\"score\":5}", "line 2: n=1302: no \"needs\""),
        ("{\"n\":1302,\"needs\":0}", "line 2: n=1302: no \"score\""),
        ("{\"n\":1302,\"score\":\"high\",\"needs\":0}", "line 2: n=1302: a bad \"score\""),
        ("{\"score\":5,\"needs\":0}", "line 2: no \"n\""),
        ("{\"n\":-4,\"score\":5,\"needs\":0}", "line 2: a bad \"n\""),
        ("{\"n\":1302,\"score\":5,\"needs\":1}", "line 2: n=1302: no \"open\""),
        ("{\"n\":1302,\"score\":5,\"needs\":1,\"open\":682}", "line 2: n=1302: a bad \"open\""),
        ("{\"n\":1302,\"score\":5,\"needs\":1,\"open\":[682]}", "line 2: n=1302: a bad member of \"open\""),
        ("{\"n\":1302,\"score\":5,\"needs\":1,\"open\":[[\"x\",3,1]]}", "line 2: n=1302: a bad member of \"open\""),
        ("{\"n\":1302,\"score\":5,\"needs\":1,\"open\":[[]]}", "line 2: n=1302: a bad member of \"open\""),
        ("this is not json", "line 2"),
        ("[1302, 5, 0]", "line 2"),
    ];
    for (i, (bad, why)) in cases.iter().enumerate() {
        let file = write_file(&dir, &format!("bad{i}.jsonl"), &format!("{ok}{bad}\n"));
        let Err(message) = read_upper(&[file]) else { panic!("{bad} was taken") };
        assert!(
            message.contains(&format!("bad{i}.jsonl: ")) && message.contains(why),
            "{bad}: expected {why:?} in {message}"
        );
    }

    // A file that is not there is named.
    let good = write_file(&dir, "good.jsonl", &ok);
    let Err(message) = read_upper(&[good, dir.join("missing.jsonl")]) else { panic!("a missing file was read") };
    assert!(message.contains("missing.jsonl"), "{message}");
}

/// The committed bounds are a line for each board from 2 to 2175, and agree with the chain where it matters: a board
/// that needs a certificate has the recorded score, a board that does not need one has no open members, and the lines
/// of the games used here are the ones the tests expect.
#[test]
fn the_committed_bounds_cover_the_chain() {
    let Fixtures { known, upper } = fixtures();
    assert_eq!(upper.keys().copied().collect::<Vec<_>>(), (2..=2175).collect::<Vec<_>>());
    for (&n, line) in upper {
        assert_eq!(line.n, n);
        if line.needs {
            assert_eq!(Some(line.score), known.score(n), "n={n}: the bounds were found against the recorded score");
        } else {
            assert!(line.open.is_empty(), "n={n}");
        }
    }
    assert_eq!(needing(2..=1300).len(), 665);
    assert_eq!(needing(1301..=2175).len(), 490);
    assert_eq!(upper[&1302], Upper { n: 1302, score: 533559, needs: true, open: vec![682, 715, 754] });
    assert_eq!(upper[&1301], Upper { n: 1301, score: 532272, needs: false, open: vec![] });
}
