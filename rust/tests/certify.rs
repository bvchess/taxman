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
//!
//! Consecutive games often pose the same extended problem, and then the earlier game's certificate settles the later
//! one with no search (`same_problem`): the tests check the games it settles against a search of the same game, that a
//! wrong score on such a game is still beaten, and that lines that are not what they say are not used.
//!
//! The extended problem has two formulations, with the claims of `unzip` that Lemma M5⁺ keeps (the default, which the
//! tests use unless they say otherwise) and without (`--no-claims`, the committed certificates' of 600..=1300).  A line
//! records which; the rule compares the problems in the run's formulation whichever the earlier game's certificate was
//! made in, which the tests check within each formulation and across the two; the claims make the rule settle more
//! games (101 of 600..=1300 against 81); and a file that mixes the formulations verifies.
//!
//! `taxman certify` takes the boards of a list file as well as a range, and `taxman verify-certificates` checks the
//! files it wrote: a file made over a stretch with repeats passes, each doctored line is refused naming the file, the
//! line and the board, and the proven end of the chain is found on synthetic data and on the committed chain.
//!
//! The work list of the campaign (`tools/certify_plan.py`) is tested on its own and against the committed certificates
//! of 600..=1300 (`bench/certify/certificates_600_1300.jsonl`): it names the games that were certified, their open
//! members and the repeats that the rule settled with no search; and those certificates verify.
//!
//! The campaign's tools (`tools/certify_campaign.py`, `tools/certify_pilot.py`) pass their own tests, its lists cover
//! every game that needs a certificate once and `taxman certify --list` reads them, and the real lines of the pilot
//! tool's tests verify.

mod common;

use common::{long, options, path, python, results_path, taxman, Run, TempDir};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;
use taxman::bits::{width_for, words_for};
use taxman::certify::verify::{frontier, read_exact, ExactLine, Frontier, Gap};
use taxman::certify::{
    certificate_width, certify_game, certify_game_with, extended_problem, read_upper, Certificate, CertificateFile,
    EarlierCertificates, Posed, SameProblem, Upper, Verdict,
};
use taxman::game::{replay, replay_finished, Optimal};
use taxman::json::Json;
use taxman::search::Problem;
use taxman::solver::{digest_text, SolveOptions, SAME_SEARCH_WINDOW};

// ---------------------------------------------------------------------------------------------------------
// The data and the helpers

/// What the tests read: the recorded games of the chain and the lines of the barred bounds.
struct Fixtures {
    known: Optimal,
    upper: BTreeMap<u32, Upper>,
}

/// A file of the crate, by its path from the crate's root.
fn crate_file(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// The prover's barred bounds that cover the chain, `prover/data/upper_*.jsonl`: boards 2 to 1300 and 1301 to 2175.
fn upper_files() -> Vec<PathBuf> {
    ["upper_2_1300.jsonl", "upper_1301_2175.jsonl"].iter().map(|name| crate_file("prover/data").join(name)).collect()
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

/// Certifies the recorded game of `n` with the committed data and `opts`, in the default formulation (with the claims
/// of `unzip` that Lemma M5⁺ keeps).
fn certify(n: u32, opts: &SolveOptions) -> Certificate {
    certify_in(n, true, opts)
}

/// [`certify`] in the formulation `claims`.
fn certify_in(n: u32, claims: bool, opts: &SolveOptions) -> Certificate {
    let Fixtures { known, upper } = fixtures();
    certify_game(n, known, &upper[&n], claims, opts).unwrap_or_else(|e| panic!("{e}"))
}

/// Checks a certificate that says nothing beats the recorded game of `n` against the data it was made from: the
/// verdict, the score, the open members, the sizes of the problem (the one-sided candidates are the open members and
/// the sources greedy rejected), a search that did something where there was something to search (a game the rule of
/// the same problem settled has none), and the run.
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
    if c.two_sided + c.one_sided > 0 && c.same_problem.is_none() {
        assert!(c.nodes > 0, "n={n}: a problem with candidates takes a search: {}", c.summary_line());
    }
    assert_eq!(c.threads, 1);
    let own = width_for(words_for(n)).unwrap();
    assert!(c.width == own || c.width == 128, "n={n}: searched at width {}, not {own} or 128", c.width);
    assert_eq!(
        certificate_width(n, &c.open, c.claims).unwrap().1,
        Some(c.width),
        "n={n}: the width is the one it takes"
    );
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

/// A wrong game for board `n`: the chain's game with its smallest move up to n/2 left out and its score lowered by that
/// move.  A game stays legal when a move is left out (what the move would have taxed stays in the pot, and none of its
/// divisors is played after it), and it plays the same numbers above n/2, so it is still a promotion set of the
/// extended problem; but its score is below the optimum, which the chain has proven, so a search must beat it.
/// Returns the planted score, the moves and the move left out.
fn planted_game(n: u32) -> (u64, Vec<u32>, u32) {
    let known = &fixtures().known;
    let (score, moves) = (known.score(n).unwrap(), known.moves(n).unwrap());
    let left_out = moves.iter().copied().filter(|&m| 2 * m <= n).min().expect("a move up to n/2");
    let kept: Vec<u32> = moves.iter().copied().filter(|&m| m != left_out).collect();
    let planted = score - u64::from(left_out);
    assert_eq!(replay(n, &kept), Ok(planted), "n={n}: a game without a move is a legal game");
    (planted, kept, left_out)
}

/// A wrong score for board `n`, planted in the results file `dir/results.jsonl` that holds the chain's games of
/// `n − 1` and `n`, except that the game of `n` is the one of [`planted_game`].  Returns the file, the planted score
/// and the move left out.
fn plant_wrong_score(dir: &Path, n: u32) -> (PathBuf, u64, u32) {
    let known = &fixtures().known;
    let (planted, kept, left_out) = planted_game(n);
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
    run_certify_boards(&[range], results, out, extra)
}

/// [`run_certify`] with the arguments `boards` in place of the range (`--list FILE`, say).
fn run_certify_boards(boards: &[&str], results: &Path, out: &Path, extra: &[&str]) -> Run {
    let uppers = upper_files();
    let mut args = vec!["certify"];
    args.extend_from_slice(boards);
    args.extend(["--results", path(results), "--out", path(out)]);
    args.extend(["--threads", "1", "--progress", "0"]);
    args.extend(uppers.iter().flat_map(|file| ["--upper", path(file)]));
    args.extend_from_slice(extra);
    taxman(&args)
}

// ---------------------------------------------------------------------------------------------------------
// The chain

/// Every `step`th game of 600..=1300 that needs a certificate, and the games `besides`: all certified, with the data
/// that each certificate says it was made from.
fn certify_a_sample(step: usize, besides: &[u32], fewest: usize) {
    let mut boards: Vec<u32> = needing(600..=1300).into_iter().step_by(step).collect();
    boards.extend_from_slice(besides);
    boards.sort_unstable();
    boards.dedup();
    assert!(boards.len() >= fewest, "{boards:?}");
    let (nodes, seconds) = certify_all(&boards);
    println!("{} games certified: {nodes} nodes, {seconds:.1} s", boards.len());
}

/// Every sixtieth game of 600..=1300 that needs a certificate, all certified.
#[test]
fn a_thin_sample_of_the_games_to_1300_that_need_a_certificate_is_certified() {
    certify_a_sample(60, &[], 6);
}

/// Every fourth game of 600..=1300 that needs a certificate, and 1287, the one with the biggest search of them: all
/// certified, with the data that each certificate says it was made from.  (All of them are in
/// `every_game_that_needs_a_certificate_to_1300_is_certified`.)
#[test]
#[cfg_attr(debug_assertions, ignore = long!(a_thin_sample_of_the_games_to_1300_that_need_a_certificate_is_certified))]
fn a_sample_of_the_games_to_1300_that_need_a_certificate_is_certified() {
    certify_a_sample(4, &[1287], 90);
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
#[cfg_attr(debug_assertions, ignore = long!(a_game_past_1300_that_needs_a_certificate_is_certified))]
fn games_past_1300_that_need_a_certificate_are_certified() {
    games_past_1300_are_certified(&[1302, 1304, 1305, 1309]);
}

/// [`games_past_1300_that_need_a_certificate_are_certified`] for the cheapest of them, 1305.
#[test]
fn a_game_past_1300_that_needs_a_certificate_is_certified() {
    games_past_1300_are_certified(&[1305]);
}

/// The games `boards` past 1300 that need a certificate are certified.
fn games_past_1300_are_certified(boards: &[u32]) {
    for &n in boards {
        assert!(fixtures().upper[&n].needs, "n={n} is a game that needs a certificate");
    }
    let (nodes, seconds) = certify_all(boards);
    println!("{} games certified: {nodes} nodes, {seconds:.1} s", boards.len());
}

/// The certificate as the command writes it: the fields, the sizes, the digest of the problem and no counterexample;
/// and the summary line has the numbers of the game and its verdict.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(a_certificate_of_a_small_game_writes_the_fields_of_its_search))]
fn a_certificate_writes_the_fields_of_its_search() {
    a_certificate_writes_the_fields_of_its_search_for(1302, [682, 715, 754], 533559);
}

/// [`a_certificate_writes_the_fields_of_its_search`] for the game 600, which has four open members.
#[test]
fn a_certificate_of_a_small_game_writes_the_fields_of_its_search() {
    let open = fixtures().upper[&600].open.clone();
    assert_eq!(open.len(), 4);
    a_certificate_writes_the_fields_of_its_search_for(600, open, 112623);
}

/// The certificate of the game `n`, whose open members are `open` and whose score is `score`, as the command writes it.
fn a_certificate_writes_the_fields_of_its_search_for(n: u32, open: impl Into<Vec<u32>>, score: u64) {
    let (open, c) = (open.into(), certify(n, &options(1)));
    assert_certified(&c, n);
    let json = Json::parse(&c.to_json().to_string()).unwrap();
    let number = |name: &str| json.get(name).and_then(Json::as_u64).unwrap_or_else(|| panic!("no number {name:?}"));
    assert_eq!(json.get("verdict").and_then(Json::as_str), Some("certified"));
    assert_eq!(json.get("open").and_then(Json::as_u32_vec).as_ref(), Some(&open));
    assert_eq!(
        [number("n"), number("score"), number("rejected"), number("nodes"), number("threads"), number("width")],
        [u64::from(n), c.score, u64::from(c.rejected), c.nodes, 1, c.width as u64]
    );
    assert_eq!(number("frames"), c.frames as u64);
    assert_eq!(number("in_the_bag"), c.in_the_bag);
    assert_eq!(number("two_sided"), u64::from(c.two_sided));
    assert_eq!(number("one_sided"), u64::from(c.one_sided));
    assert_eq!(json.get("digest").and_then(Json::as_str).map(str::len), Some(16));
    assert!(json.get("beaten_by").is_none() && json.get("build").is_some(), "{json}");
    let summary = c.summary_line();
    for text in
        [&format!("n={n}"), &format!("V={score}"), &format!("|O|={}", open.len()), "certified", &c.nodes.to_string()]
    {
        assert!(summary.contains(text), "no {text:?} in {summary:?}");
    }
    println!("{summary}");

    // The formulation is written right after the width, and said in the summary where it is the older one.
    let keys: Vec<&str> = json.as_obj().unwrap().iter().map(|(key, _)| key.as_str()).collect();
    let at = keys.iter().position(|&key| key == "width").unwrap();
    assert_eq!(keys[at..at + 3], ["width", "claims", "digest"], "{keys:?}");
    assert!(c.claims && json.get("claims") == Some(&Json::Bool(true)) && !summary.contains("no claims"));
    let older = certify_in(n, false, &options(1));
    assert_certified(&older, n);
    let json = Json::parse(&older.to_json().to_string()).unwrap();
    assert!(!older.claims && json.get("claims") == Some(&Json::Bool(false)), "{json}");
    assert!(older.summary_line().ends_with(&format!("problem {} (no claims)", digest_text(older.digest))));
    assert_eq!((older.score, older.open.clone()), (c.score, c.open.clone()));
}

// ---------------------------------------------------------------------------------------------------------
// A planted wrong score

/// Checks that `c` says the planted score of board `n` is beaten: the verdict is `Beaten` and the game found is one
/// that the referee accepts, scores more than the planted score, and, since the chain's score is the optimum, scores
/// that.
fn assert_beaten_by_the_optimum(c: &Certificate, n: u32, planted: u64) {
    assert_eq!(c.verdict, Verdict::Beaten, "n={n}: {}", c.summary_line());
    assert_eq!(c.score, planted);
    let (found, moves) = c.beaten_by.as_ref().expect("a beaten game has the game that beats it");
    assert_eq!(replay_finished(n, moves), Ok(*found), "n={n}: the referee accepts the game found");
    assert!(*found > planted, "n={n}: the game found scores {found}, not more than {planted}");
    assert_eq!(Some(*found), fixtures().known.score(n), "n={n}: the best game is the proven optimum");
    assert!(c.summary_line().contains(&format!("by a game of {found}")), "{}", c.summary_line());
    let json = Json::parse(&c.to_json().to_string()).unwrap();
    assert_eq!(json.get("verdict").and_then(Json::as_str), Some("beaten"));
    let beating = json.get("beaten_by").expect("beaten_by");
    assert_eq!(beating.get("score").and_then(Json::as_u64), Some(*found));
    assert_eq!(beating.get("moves").and_then(Json::as_u32_vec).as_ref(), Some(moves));
}

/// A wrong score planted on a board (see [`plant_wrong_score`]) is beaten, with the real barred bounds: the verdict
/// is `Beaten` and the game found is one that the referee accepts, scores more than the planted score, and, since the
/// chain's score is the optimum, scores that.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(a_planted_wrong_score_of_1305_is_beaten_by_a_game_the_referee_accepts))]
fn a_planted_wrong_score_is_beaten_by_a_game_the_referee_accepts() {
    planted_wrong_scores_are_beaten(&[1302, 1305, 1309]);
}

/// [`a_planted_wrong_score_is_beaten_by_a_game_the_referee_accepts`] for 1305, the cheapest.
#[test]
fn a_planted_wrong_score_of_1305_is_beaten_by_a_game_the_referee_accepts() {
    planted_wrong_scores_are_beaten(&[1305]);
}

/// A wrong score planted on each of the boards `boards` is beaten.
fn planted_wrong_scores_are_beaten(boards: &[u32]) {
    let upper = &fixtures().upper;
    for &n in boards {
        let dir = TempDir::new("certify-planted");
        let (file, planted, left_out) = plant_wrong_score(&dir, n);
        let table = Optimal::built_in().with_results_files(&[file]).unwrap();
        assert_eq!(table.score(n), Some(planted));

        let c = certify_game(n, &table, &upper[&n], true, &options(1)).unwrap_or_else(|e| panic!("{e}"));
        println!("{} (the smallest move up to n/2, {left_out}, left out)", c.summary_line());
        assert_beaten_by_the_optimum(&c, n, planted);
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
        let e = certify_game(n, table, line, true, &options(1))
            .err()
            .unwrap_or_else(|| panic!("n={n}: not refused: {why}"));
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

/// The game the quick stop-and-resume tests use: its search takes 1,593 nodes, which a limit of 1,000 nodes cuts in
/// two.
const RESUMED_QUICK: u32 = 1041;

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
#[cfg_attr(debug_assertions, ignore = long!(a_stopped_search_of_a_small_game_resumes_to_the_certificate_of_an_uninterrupted_one))]
fn a_stopped_search_resumes_to_the_certificate_of_an_uninterrupted_one() {
    a_stopped_search_resumes(RESUMED, 3_000, 2);
}

/// [`a_stopped_search_resumes_to_the_certificate_of_an_uninterrupted_one`] for a game of 1,593 nodes, stopped once.
#[test]
fn a_stopped_search_of_a_small_game_resumes_to_the_certificate_of_an_uninterrupted_one() {
    a_stopped_search_resumes(RESUMED_QUICK, 1_200, 1);
}

/// The search for the game `n`, which takes more than `most_nodes` nodes, stopped by a limit of 1,000 nodes in at
/// least `min_pieces` pieces and resumed to the certificate of an uninterrupted search.
fn a_stopped_search_resumes(n: u32, most_nodes: u64, min_pieces: usize) {
    let whole = certify(n, &options(1));
    assert_certified(&whole, n);
    assert!(whole.nodes > most_nodes, "n={n}: {} nodes are too few for pieces of 1,000", whole.nodes);

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
    assert!(pieces.len() >= min_pieces, "n={n}: the search ended after {} pieces that stopped", pieces.len());

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
#[cfg_attr(debug_assertions, ignore = long!(the_command_stops_resumes_and_then_skips_a_small_game))]
fn the_command_stops_resumes_and_then_skips_a_game() {
    the_command_stops_resumes_and_then_skips(RESUMED, 3);
}

/// [`the_command_stops_resumes_and_then_skips_a_game`] for a game of 1,593 nodes, stopped once.
#[test]
fn the_command_stops_resumes_and_then_skips_a_small_game() {
    the_command_stops_resumes_and_then_skips(RESUMED_QUICK, 2);
}

/// The command on the game `n`, stopped by a limit of 1,000 nodes, resumed by the same command in at least `min_runs`
/// runs in all, and then skipping the game.
fn the_command_stops_resumes_and_then_skips(n: u32, min_runs: usize) {
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
    assert!(runs >= min_runs, "the game was certified in {runs} runs");
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
#[cfg_attr(debug_assertions, ignore = long!(the_command_certifies_a_range_of_small_games_and_skips_what_is_done))]
fn the_command_certifies_a_range_and_skips_what_is_done() {
    // Longer, 1308 and 1309 are the new ones that need a certificate.
    the_command_certifies_a_range_and_skips_what_is_done_over(1301..=1305, 1301..=1309);
}

/// [`the_command_certifies_a_range_and_skips_what_is_done`] over 600..=608 and then 600..=610.
#[test]
fn the_command_certifies_a_range_of_small_games_and_skips_what_is_done() {
    the_command_certifies_a_range_and_skips_what_is_done_over(600..=608, 600..=610);
}

/// The command on the range `first` and then on the longer range `longer` (which starts where `first` does): each game
/// that needs a certificate gets a line, the others are counted, and the rerun skips what is done.
fn the_command_certifies_a_range_and_skips_what_is_done_over(first: RangeInclusive<u32>, longer: RangeInclusive<u32>) {
    let dir = TempDir::new("certify-range");
    let (results, out) = (results_path("strict.jsonl"), dir.join("out.jsonl"));
    let (done, all) = (needing(first.clone()), needing(longer.clone()));
    let new: Vec<u32> = all.iter().copied().filter(|n| !done.contains(n)).collect();
    assert!(!done.is_empty() && !new.is_empty(), "{done:?} {new:?}");
    let (lo, hi) = (*first.start(), *first.end());
    let not_needing = (hi - lo + 1) as usize - done.len();

    let run = run_certify(&format!("{lo}-{hi}"), &results, &out, &[]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    for n in &done {
        assert!(run.stdout.contains(&format!("n={n} ")), "{}", run.stdout);
    }
    assert_eq!(run.stdout.lines().count(), done.len() + 1, "{}", run.stdout);
    let tally = format!("{lo}..={hi}: certified {}, not needing one {not_needing}", done.len());
    assert!(run.stdout.contains(&tally), "{}", run.stdout);
    assert_eq!(run.stderr, "");
    assert_eq!(verdicts(&out), vec!["certified"; done.len()]);
    assert_eq!(boards_in_order(&out), done);

    let (lo, hi) = (*longer.start(), *longer.end());
    let not_needing = (hi - lo + 1) as usize - all.len();
    let run = run_certify(&format!("{lo}-{hi}"), &results, &out, &[]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let tally =
        format!("{lo}..={hi}: already in --out {}, certified {}, not needing one {not_needing}", done.len(), new.len());
    assert!(run.stdout.contains(&tally), "{}", run.stdout);
    assert_eq!(run.stdout.lines().count(), new.len() + 1, "{}", run.stdout);
    assert_eq!(verdicts(&out), vec!["certified"; all.len()]);
}

/// The boards of the lines of a file `taxman certify` wrote, in order.
fn boards_in_order(file: &Path) -> Vec<u32> {
    json_lines(file).iter().map(|line| line.get("n").and_then(Json::as_u32).unwrap()).collect()
}

/// `--list` takes the boards of a file, in the file's order, with comments and blank lines skipped: three games that
/// need a certificate, out of order, and one that needs none, which is counted and passed over.  The lines of `--out`
/// come in the order of the file, the tally names the list, and a rerun skips what is done.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(the_command_certifies_the_small_boards_of_a_list_in_the_order_of_the_file))]
fn the_command_certifies_the_boards_of_a_list_in_the_order_of_the_file() {
    the_command_certifies_the_boards_of_a_list_in_the_order_of_the_file_for([1309, 1302, 1304], 1301);
}

/// [`the_command_certifies_the_boards_of_a_list_in_the_order_of_the_file`] for 620, 600 and 616 (that pose three
/// different problems), and the first of 600..=640 that needs none.
#[test]
fn the_command_certifies_the_small_boards_of_a_list_in_the_order_of_the_file() {
    let needless = (600..=640).find(|n| !fixtures().upper[n].needs).unwrap();
    the_command_certifies_the_boards_of_a_list_in_the_order_of_the_file_for([620, 600, 616], needless);
}

/// The command on the list of the games `needing` (out of order, with a comment and a blank line between them) and
/// the game `needless`, which needs no certificate.
fn the_command_certifies_the_boards_of_a_list_in_the_order_of_the_file_for(needing: [u32; 3], needless: u32) {
    let dir = TempDir::new("certify-list");
    let (results, out) = (results_path("strict.jsonl"), dir.join("out.jsonl"));
    let [first, second, third] = needing;
    let text = format!(
        "# three that need a certificate, out of order, and a prime that needs none\n{first}\n\n{second}\n  \
         {needless}  \n# 1303 is for later\n{third}\n"
    );
    let list = write_file(&dir, "boards.txt", &text);
    assert!(!fixtures().upper[&needless].needs && needing.iter().all(|n| fixtures().upper[n].needs));

    let run = run_certify_boards(&["--list", path(&list)], &results, &out, &[]);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    assert_eq!(boards_in_order(&out), needing);
    assert_eq!(verdicts(&out), ["certified"; 3]);
    let stdout_boards: Vec<_> = run.stdout.lines().filter_map(|line| line.strip_prefix("n=")).collect();
    assert!(stdout_boards[0].starts_with(&format!("{first} ")), "{}", run.stdout);
    assert!(stdout_boards[1].starts_with(&format!("{second} ")), "{}", run.stdout);
    let tally = format!("list {} (4 boards): certified 3, not needing one 1", path(&list));
    assert_eq!(run.stdout.lines().last(), Some(tally.as_str()), "{}", run.stdout);
    assert_eq!(run.stderr, "");

    // A rerun finds the games done.
    let before = fs::read(&out).unwrap();
    let run = run_certify_boards(&["--list", path(&list)], &results, &out, &[]);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    let tally = format!("list {} (4 boards): already in --out 3, not needing one 1", path(&list));
    assert_eq!(run.stdout.trim_end(), tally);
    assert_eq!(fs::read(&out).unwrap(), before);
}

/// An `--out` file that another process holds open (here this one, appending to it as a run does) is refused with
/// status 1 and nothing is written; once it is closed, the command goes on with it.
#[test]
fn the_command_refuses_an_out_file_another_process_has_open() {
    let n = needing(600..=640)[0].to_string();
    let dir = TempDir::new("certify-lock");
    let (results, out) = (results_path("strict.jsonl"), dir.join("out.jsonl"));
    let held = CertificateFile::open(&out).unwrap();
    let run = run_certify(&n, &results, &out, &[]);
    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert!(run.stderr.contains("out.jsonl is in use by another process"), "{}", run.stderr);
    assert_eq!(run.stdout, "");
    assert_eq!(fs::read_to_string(&out).unwrap(), "");
    drop(held);
    let run = run_certify(&n, &results, &out, &[]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(verdicts(&out), ["certified"]);
}

// ---------------------------------------------------------------------------------------------------------
// The rule of the same problem

/// The sizes of the problem a certificate records: its frames, two-sided and one-sided candidates, rejected sources and
/// `in_the_bag`.
fn problem_sizes(c: &Certificate) -> (usize, u32, u32, u32, u64) {
    (c.frames, c.two_sided, c.one_sided, c.rejected, c.in_the_bag)
}

/// What the rule did over a run of games: the pairs (m, n) it settled, and the nodes that searching those games takes.
struct RuleRun {
    settled: Vec<(u32, u32)>,
    nodes_saved: u64,
}

/// Π⁺(O) of each of `boards` (games that need a certificate) in the formulation `claims`, set up without a search.
fn posed_of(boards: &[u32], claims: bool) -> BTreeMap<u32, Posed> {
    let upper = &fixtures().upper;
    boards.iter().map(|&n| (n, extended_problem(n, &upper[&n].open, claims).unwrap())).collect()
}

/// The problems of [`posed_of`].
fn problems_of(boards: &[u32], claims: bool) -> BTreeMap<u32, Problem> {
    posed_of(boards, claims).into_iter().map(|(n, posed)| (n, posed.problem)).collect()
}

/// The game the rule rests `n` on, given the problems of the games before it: the latest of the last
/// `SAME_SEARCH_WINDOW` board sizes whose problem is `problem`, found by comparing the problems whole.
fn latest_with_problem(n: u32, problem: &Problem, problems: &BTreeMap<u32, Problem>) -> Option<u32> {
    (n.saturating_sub(SAME_SEARCH_WINDOW)..n).rev().find(|m| problems.get(m) == Some(problem))
}

/// The pairs (m, n) the rule settles among the games whose problems are `problems`, ascending: the games that have a
/// game in the window before them with the same problem, and that game.
fn same_problem_pairs(problems: &BTreeMap<u32, Problem>) -> Vec<(u32, u32)> {
    problems.iter().filter_map(|(&n, problem)| latest_with_problem(n, problem, problems).map(|m| (m, n))).collect()
}

/// Certifies `boards` (games that need a certificate, ascending) in order, as `taxman certify` does, in the formulation
/// `claims`: each rests on the games certified before it in the run where the rule allows.  The game it rests on must
/// be the latest of the window whose problem is the same, which is found here by comparing the problems of the games
/// themselves; and every game the rule settles is checked against a search of the same game (the same problem, sizes
/// and score, and the record of the game it rests on).
fn certify_by_the_rule(boards: &[u32], claims: bool) -> RuleRun {
    let Fixtures { known, upper } = fixtures();
    let mut earlier = EarlierCertificates::default();
    let (mut certificates, problems) = (BTreeMap::new(), problems_of(boards, claims));
    let mut run = RuleRun { settled: Vec::new(), nodes_saved: 0 };
    for &n in boards {
        let c = certify_game_with(n, known, &upper[&n], claims, &options(1), &mut earlier)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_certified(&c, n);
        assert_eq!(c.claims, claims, "n={n}: the formulation is the run's");
        println!("{}", c.summary_line());

        let latest_equal = latest_with_problem(n, &problems[&n], &problems);
        assert_eq!(c.same_problem.as_ref().map(|same| same.n), latest_equal, "n={n}: the game the rule rests on");

        if let Some(same) = &c.same_problem {
            let m: &Certificate = certificates.get(&same.n).expect("the game it rests on was certified in the run");
            let (best, worth) = (m.score - m.in_the_bag, c.score - c.in_the_bag);
            assert_eq!(*same, SameProblem { n: same.n, digest: c.digest, best, worth }, "n={n}");
            assert_eq!((best, same.digest), (worth, m.digest), "n={n}: the same problem, the same heaviest set");
            assert!(c.nodes == 0 && c.seconds <= 0.0 && c.seconds_all_runs <= 0.0, "n={n}: no search");

            let direct = certify_in(n, claims, &options(1));
            assert_certified(&direct, n);
            assert!(direct.same_problem.is_none() && direct.nodes > 0, "n={n}: a search: {}", direct.summary_line());
            let (rule, search) =
                ((c.digest, c.score, problem_sizes(&c)), (direct.digest, direct.score, problem_sizes(&direct)));
            assert_eq!(rule, search, "n={n}: the rule's certificate is the search's");
            run.settled.push((same.n, n));
            run.nodes_saved += direct.nodes;
        }
        certificates.insert(n, c);
    }
    run
}

/// Over stretches of the chain where consecutive games pose the same extended problem, the rule settles those games
/// with no search, and each comes out as a search of it does: in 630..=640 two pairs, 632 → 633 and (633 being settled)
/// 633 → 635; in 1312..=1317 three, 1312 → 1313, 1314 → 1315 and 1316 → 1317.  The same pairs in both formulations (the
/// claims make more games pose the same problem elsewhere, as the test of all of 600..=1300 below counts).
#[test]
#[cfg_attr(debug_assertions, ignore = long!(the_rule_settles_repeats_in_630_to_640_as_a_search_does))]
fn the_rule_settles_repeats_as_a_search_does() {
    for claims in [true, false] {
        the_rule_settles_630_to_640(claims);
        let run = certify_by_the_rule(&needing(1312..=1317), claims);
        assert_eq!(run.settled, [(1312, 1313), (1314, 1315), (1316, 1317)], "claims {claims}");
        println!("claims {claims}: {} nodes of search saved", run.nodes_saved);
    }
}

/// [`the_rule_settles_repeats_as_a_search_does`] over 630..=640, in the formulation with the claims alone.
#[test]
fn the_rule_settles_repeats_in_630_to_640_as_a_search_does() {
    the_rule_settles_630_to_640(true);
}

/// The two pairs the rule settles in 630..=640 in the formulation `claims`, each as a search does.
fn the_rule_settles_630_to_640(claims: bool) {
    let run = certify_by_the_rule(&needing(630..=640), claims);
    assert_eq!(run.settled, [(632, 633), (633, 635)], "claims {claims}");
}

/// The same over every game of 600..=1300 that needs a certificate, in both formulations: the rule rests each repeat
/// on the latest game of the window with the same problem, and each comes out as a search of it does.
#[test]
#[ignore = "certifies the 377 games of 600..=1300 that need one twice, and searches the repeats again: a few minutes"]
fn the_rule_over_600_to_1300() {
    let boards = needing(600..=1300);
    assert_eq!(boards.len(), 377);
    for claims in [false, true] {
        let start = Instant::now();
        let run = certify_by_the_rule(&boards, claims);
        println!(
            "claims {claims}: {} games, {} settled by the rule, which saved {} nodes of search: {:.1} s with the \
             searches that check them",
            boards.len(),
            run.settled.len(),
            run.nodes_saved,
            start.elapsed().as_secs_f64()
        );
        assert!(run.settled.len() > 50, "{} games settled", run.settled.len());
    }
}

/// The claims change the problems and not their sizes, and make the rule settle more games.  Over the 377 games of
/// 600..=1300 that need a certificate: the frames, the candidates, the rejected sources and `in_the_bag⁺` are the same
/// in both formulations (the rule goes by the first two to pass over a line of the other formulation); 231 of the
/// problems differ; and the rule settles 81 games without the claims, the ones the committed certificates were settled
/// by, and 101 with them: the same 81 and 20 more, whose problems differ without the claims and are equal with them.
/// No two games that pose the same problem without the claims pose different ones with them.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(the_claims_change_the_problems_and_not_their_sizes_of_the_first_games_to_700))]
fn the_claims_change_the_problems_and_not_their_sizes_and_the_rule_settles_more_games_with_them() {
    the_claims_change_the_problems_and_not_their_sizes(&needing(600..=1300), Some([231, 81, 101, 20]));
}

/// [`the_claims_change_the_problems_and_not_their_sizes_and_the_rule_settles_more_games_with_them`] over the games of
/// 600..=700 that need a certificate: the sizes are the same, the committed certificates settled the pairs that the
/// rule finds without the claims, the rule settles those and more with them, and the games gained pose different
/// problems without the claims and the same one with them.
#[test]
fn the_claims_change_the_problems_and_not_their_sizes_of_the_first_games_to_700() {
    the_claims_change_the_problems_and_not_their_sizes(&needing(600..=700), None);
}

/// Over `boards`, games that need a certificate (ascending): the claims change neither the sizes of the problems nor
/// `in_the_bag⁺`, the pairs the rule settles without them are the committed certificates' (those among `boards`), and
/// the rule settles all of those pairs with them, and the other pairs it settles with them pose different problems
/// without them.  With `counts` (the problems that differ, the pairs without the claims, with them, and gained), those
/// are the counts as well.
fn the_claims_change_the_problems_and_not_their_sizes(boards: &[u32], counts: Option<[usize; 4]>) {
    let (without, with) = (posed_of(boards, false), posed_of(boards, true));
    let sizes = |posed: &Posed| (posed.frames, posed.two_sided, posed.one_sided, posed.rejected, posed.in_the_bag);
    for &n in boards {
        assert_eq!(sizes(&without[&n]), sizes(&with[&n]), "n={n}: the claims change neither the sizes nor the bag");
    }
    let differ = boards.iter().filter(|n| without[n].problem != with[n].problem).count();

    let (problems_without, problems_with) = (problems_of(boards, false), problems_of(boards, true));
    let (pairs_without, pairs_with) = (same_problem_pairs(&problems_without), same_problem_pairs(&problems_with));
    let committed: Vec<(u32, u32)> = json_lines(&crate_file("bench/certify/certificates_600_1300.jsonl"))
        .iter()
        .filter_map(|line| Some((field(line.get("same_problem")?, "n").as_u32()?, board_of(line)?)))
        .filter(|(m, n)| boards.contains(m) && boards.contains(n))
        .collect();
    assert_eq!(pairs_without, committed, "the pairs the rule settled in the committed certificates");
    let gained: Vec<(u32, u32)> = pairs_with.iter().copied().filter(|pair| !pairs_without.contains(pair)).collect();
    assert!(pairs_without.iter().all(|pair| pairs_with.contains(pair)), "nothing the rule settled is lost");
    for &(m, n) in &gained {
        assert!(problems_without[&m] != problems_without[&n] && problems_with[&m] == problems_with[&n], "{m} {n}");
    }
    if let Some(counts) = counts {
        assert_eq!(
            [differ, pairs_without.len(), pairs_with.len(), gained.len()],
            counts,
            "problems that the claims change"
        );
    }
    println!("settled with the claims and not without: {gained:?}");
}

/// A pair the claims make the rule settle: 609 and 611 pose different problems without the claims and the same one
/// with them.  With them, 611 is certified from the certificate of 609 with no search, and comes out as a search of it
/// does (the heaviest set is the recorded game's in both formulations); without them, 611 is searched.
#[test]
fn a_pair_of_games_is_settled_by_the_rule_only_with_the_claims() {
    let (m, n) = (609, 611);
    let (without, with) = (problems_of(&[m, n], false), problems_of(&[m, n], true));
    assert!(without[&m] != without[&n] && with[&m] == with[&n], "the problems of {m} and {n}");

    let run = certify_by_the_rule(&[m, n], true);
    assert_eq!(run.settled, [(m, n)]);
    // Without them both are searched (a game the rule settles has no nodes, and `certify_by_the_rule` checks that the
    // game it rests on is the latest of the window with the same problem, which there is none of).
    let run = certify_by_the_rule(&[m, n], false);
    assert!(run.settled.is_empty(), "{:?}", run.settled);
    assert_eq!(run.nodes_saved, 0);
}

// ---------------------------------------------------------------------------------------------------------
// The two formulations

/// Certifies `boards` (games that need a certificate, ascending) in order in the formulation `claims`, as `taxman
/// certify` does, and returns the certificates.
fn certify_run(boards: &[u32], claims: bool) -> Vec<Certificate> {
    let Fixtures { known, upper } = fixtures();
    let mut earlier = EarlierCertificates::default();
    let mut certify = |&n: &u32| {
        let opts = options(1);
        let c = certify_game_with(n, known, &upper[&n], claims, &opts, &mut earlier).unwrap_or_else(|e| panic!("{e}"));
        assert_certified(&c, n);
        assert_eq!(c.claims, claims, "n={n}: the formulation is the run's");
        c
    };
    boards.iter().map(&mut certify).collect()
}

/// Certifies `boards` in the formulation opposite to `first` with only the certificates of `first` to rest on: the
/// games of `boards` are certified in `first` (as a run does, the rule at work among them), and then each game that
/// the rule settles in the other formulation, `second`, is certified there on its own, with the certificates of the
/// games before it, made in `first`, as the only earlier games.  Each must come out settled by the latest game of the
/// window whose problem in `second` is its own, and as a search of it in `second` does.  Returns the pairs (m, n).
fn certify_across(boards: &[u32], first: bool) -> Vec<(u32, u32)> {
    let Fixtures { known, upper } = fixtures();
    let second = !first;
    let (lines, problems) = (certify_run(boards, first), problems_of(boards, second));
    let mut settled = Vec::new();
    for &n in boards {
        let Some(m) = latest_with_problem(n, &problems[&n], &problems) else { continue };
        let mut earlier = EarlierCertificates::default();
        lines.iter().filter(|line| line.n < n).for_each(|line| earlier.insert(line.clone()));
        let c = certify_game_with(n, known, &upper[&n], second, &options(1), &mut earlier)
            .unwrap_or_else(|e| panic!("{e}"));
        assert_certified(&c, n);
        println!("{}", c.summary_line());

        let line = lines.iter().find(|line| line.n == m).expect("the game it rests on was certified");
        let same = c.same_problem.as_ref().unwrap_or_else(|| panic!("n={n} rests on {m}: {}", c.summary_line()));
        assert_eq!((line.claims, c.claims), (first, second), "n={n}: a certificate of the other formulation");
        let (best, worth) = (line.worth().unwrap(), c.worth().unwrap());
        let digest = problems[&n].digest();
        assert_eq!(*same, SameProblem { n: m, digest, best, worth }, "n={n}: the digest is the problem's in `second`");
        assert_eq!((c.digest, best), (digest, worth), "n={n}: the same heaviest set, whichever formulation m was in");
        assert!(c.nodes == 0 && c.seconds <= 0.0, "n={n}: no search");

        let direct = certify_in(n, second, &options(1));
        assert_certified(&direct, n);
        assert!(direct.same_problem.is_none() && direct.nodes > 0, "n={n}: a search: {}", direct.summary_line());
        let (rule, search) =
            ((c.digest, c.score, problem_sizes(&c)), (direct.digest, direct.score, problem_sizes(&direct)));
        assert_eq!(rule, search, "n={n}: the rule's certificate is the search's");
        settled.push((m, n));
    }
    settled
}

/// A certificate made in one formulation settles a later game in the other, when the later game's problem in its own
/// formulation is the earlier game's problem in that one: the rule settles across the two exactly the pairs it settles
/// within one, each with the digest of the problem in the formulation of the run, and each comes out as a search of the
/// game in that formulation does.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(the_rule_settles_across_formulations_in_630_to_640_as_it_does_within_one))]
fn the_rule_settles_across_formulations_as_it_does_within_one() {
    for boards in [needing(630..=640), needing(1312..=1317)] {
        the_rule_settles_across_formulations_over(&boards);
    }
}

/// [`the_rule_settles_across_formulations_as_it_does_within_one`] over 630..=640 alone.
#[test]
fn the_rule_settles_across_formulations_in_630_to_640_as_it_does_within_one() {
    the_rule_settles_across_formulations_over(&needing(630..=640));
}

/// From each formulation to the other, the rule settles over `boards` exactly the pairs it settles within one.
fn the_rule_settles_across_formulations_over(boards: &[u32]) {
    for first in [false, true] {
        let pairs = same_problem_pairs(&problems_of(boards, !first));
        assert!(!pairs.is_empty(), "{boards:?}: games that pose the same problem");
        assert_eq!(certify_across(boards, first), pairs, "{boards:?}: from the claims {first} to {}", !first);
    }
}

/// A wrong score planted on a game that repeats an earlier game's problem is still beaten: its recorded game is worth
/// less than the earlier certificate says the problem allows, so the rule is not used and the search finds the better
/// set.
#[test]
fn a_planted_wrong_score_on_a_repeat_is_beaten() {
    let Fixtures { known, upper } = fixtures();
    let (m, n) = (1314, 1315);
    let mut earlier = EarlierCertificates::default();
    let before =
        certify_game_with(m, known, &upper[&m], true, &options(1), &mut earlier).unwrap_or_else(|e| panic!("{e}"));
    assert_certified(&before, m);

    // The real game of 1315 is settled by the rule from the certificate of 1314 (set up again from its line).
    let mut from_the_line = EarlierCertificates::default();
    from_the_line.insert(before);
    let real = certify_game_with(n, known, &upper[&n], true, &options(1), &mut from_the_line).unwrap();
    assert_eq!(real.same_problem.as_ref().map(|same| same.n), Some(m));

    let dir = TempDir::new("certify-planted-repeat");
    let (file, planted, left_out) = plant_wrong_score(&dir, n);
    let table = Optimal::built_in().with_results_files(&[file]).unwrap();
    let c = certify_game_with(n, &table, &upper[&n], true, &options(1), &mut earlier).unwrap_or_else(|e| panic!("{e}"));
    println!("{} (the smallest move up to n/2, {left_out}, left out)", c.summary_line());
    assert_beaten_by_the_optimum(&c, n, planted);
    assert!(c.same_problem.is_none() && c.nodes > 0, "searched, not settled by the rule");
}

/// A certificate of an earlier game is used only if it is what it says: a verdict that is not `certified` settles
/// nothing; a line with the digest of the game but another problem (it does not reproduce the line when set up again,
/// or it is not the game's problem) is not used; a score lower than the game's contradicts it (an error), and a higher
/// one only makes the game be searched.  The same through the binary, for lines in `--out`.
#[test]
fn doctored_lines_are_not_used() {
    let Fixtures { known, upper } = fixtures();
    let (m, n) = (633, 635);
    let certify_n =
        |earlier: &mut EarlierCertificates| certify_game_with(n, known, &upper[&n], true, &options(1), earlier);
    let whole = certify(n, &options(1));
    assert_certified(&whole, n);
    let searched = |c: Result<Certificate, String>| {
        let c = c.unwrap_or_else(|e| panic!("{e}"));
        assert_certified(&c, n);
        assert!(c.same_problem.is_none() && c.nodes > 0, "n={n} is searched: {}", c.summary_line());
        assert_eq!((c.digest, c.score, problem_sizes(&c)), (whole.digest, whole.score, problem_sizes(&whole)));
    };
    let with = |lines: &[&Certificate]| {
        let mut earlier = EarlierCertificates::default();
        lines.iter().for_each(|&line| earlier.insert(line.clone()));
        earlier
    };

    // The real line of m settles n, set up again from the line.
    let real = certify(m, &options(1));
    assert_certified(&real, m);
    let settled = certify_n(&mut with(&[&real])).unwrap();
    assert_eq!(settled.same_problem.as_ref().map(|same| same.n), Some(m));

    // The refusal of bounds that were found against a higher score applies to a game the rule settles as to a searched.
    let higher = Upper { score: whole.score + 1, ..upper[&n].clone() };
    let e = certify_game_with(n, known, &higher, true, &options(1), &mut with(&[&real]))
        .expect_err("bounds above the score");
    assert!(e.contains(&format!("n={n}: nothing beats {}, but the open members", whole.score)), "{e}");
    assert!(e.contains(&format!("found against {}, more: not a certificate", whole.score + 1)), "{e}");

    // (a) A line whose verdict is not `certified` is no certificate to rest on.
    for verdict in [Verdict::Beaten, Verdict::Incomplete] {
        searched(certify_n(&mut with(&[&Certificate { verdict, ..real.clone() }])));
    }

    // (b) Another game of the window whose problem is not n's, with n's digest on its line: it does not reproduce its
    // line when set up again.
    let ours = extended_problem(n, &upper[&n].open, true).unwrap().problem;
    let other = (n - SAME_SEARCH_WINDOW..n)
        .rev()
        .find(|&k| k != m && upper[&k].needs && extended_problem(k, &upper[&k].open, true).unwrap().problem != ours)
        .expect("another game of the window that needs a certificate and poses another problem");
    let other = Certificate { digest: ours.digest(), ..certify(other, &options(1)) };
    searched(certify_n(&mut with(&[&other])));

    // (c) The line of m with a member of its open sources dropped: the problem set up again from it is another.
    let mut open = real.open.clone();
    assert!(open.pop().is_some(), "the open members of n={m}");
    let dropped = Certificate { open, ..real.clone() };
    searched(certify_n(&mut with(&[&dropped])));

    // (c') A line of m whose digest is right, and whose problem set up again is n's, but with a size that is not the
    // problem's: it does not reproduce the line either.  (A line that was used would settle n, or, with another
    // `in_the_bag`, contradict it.)
    let wrong_sizes = [
        Certificate { frames: real.frames + 1, ..real.clone() },
        Certificate { two_sided: real.two_sided + 1, ..real.clone() },
        Certificate { one_sided: real.one_sided + 1, ..real.clone() },
        Certificate { rejected: real.rejected + 1, ..real.clone() },
        Certificate { in_the_bag: real.in_the_bag + 1, ..real.clone() },
    ];
    for line in &wrong_sizes {
        searched(certify_n(&mut with(&[line])));
    }

    // (d) A score lower than the game's contradicts it; a higher one is no reason to rest on the line.
    let lowered = Certificate { score: real.score - 1, ..real.clone() };
    let e = certify_n(&mut with(&[&lowered])).expect_err("a line that contradicts the game is an error");
    assert!(e.contains(&format!("n={n}: ")) && e.contains(&format!("contradicts the certificate of n={m}")), "{e}");
    searched(certify_n(&mut with(&[&Certificate { score: real.score + 1, ..real.clone() }])));

    // The same lines in `--out`: the game is searched, its new line has no `same_problem`.
    let dir = TempDir::new("certify-doctored");
    for (name, line) in [("other", &other), ("dropped", &dropped)] {
        let out = dir.join(format!("{name}.jsonl"));
        let mut file = CertificateFile::open(&out).unwrap();
        file.append(line).unwrap();
        drop(file);
        let run = run_certify(&format!("{n}-{n}"), &results_path("strict.jsonl"), &out, &[]);
        assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
        assert!(run.stdout.contains(&format!("{n}..={n}: certified 1")), "{}", run.stdout);
        let lines = json_lines(&out);
        let last = lines.last().unwrap();
        assert_eq!(last.get("n").and_then(Json::as_u32), Some(n));
        assert!(last.get("same_problem").is_none(), "{last}");
        assert!(last.get("nodes").and_then(Json::as_u64).unwrap() > 0, "{last}");
    }
}

/// The command writes `same_problem` lines: the repeat's line has the four keys and no nodes, stdout says what game
/// it rests on, and the tally counts it apart.  A second run that starts after the game it rests on, within the window,
/// settles a later repeat from that game's line in `--out` (a game this run did not certify, set up again from its line).
#[test]
fn the_command_writes_same_problem_lines() {
    let dir = TempDir::new("certify-same-problem");
    let (results, out) = (results_path("strict.jsonl"), dir.join("out.jsonl"));

    // 632 and 633 pose the same problem, 633 and 635 too; 631 is a prime, which needs no certificate.
    let run = run_certify("630-633", &results, &out, &[]);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    assert!(run.stdout.contains("certified (same problem as 632)"), "{}", run.stdout);
    assert!(
        run.stdout.contains("630..=633: certified 2, certified (same problem) 1, not needing one 1"),
        "{}",
        run.stdout
    );
    let lines = json_lines(&out);
    assert_eq!(lines.len(), 3);
    let repeat = &lines[2];
    assert_eq!(repeat.get("n").and_then(Json::as_u32), Some(633));
    assert_eq!(repeat.get("verdict").and_then(Json::as_str), Some("certified"));
    assert_eq!(repeat.get("nodes").and_then(Json::as_u64), Some(0));
    let same = repeat.get("same_problem").expect("same_problem");
    let keys: Vec<&str> = same.as_obj().unwrap().iter().map(|(key, _)| key.as_str()).collect();
    assert_eq!(keys, ["n", "digest", "best", "worth"]);
    assert_eq!(same.get("n").and_then(Json::as_u32), Some(632));
    let (best, worth) = (same.get("best").and_then(Json::as_u64), same.get("worth").and_then(Json::as_u64));
    assert!(best.is_some() && best == worth, "the heaviest set m's certificate proved is n's recorded game: {same}");
    let digest = repeat.get("digest").and_then(Json::as_str);
    assert_eq!(same.get("digest").and_then(Json::as_str), digest);
    assert_eq!(lines[1].get("digest").and_then(Json::as_str), digest, "632 posed the same problem");
    assert!(lines[0].get("same_problem").is_none() && lines[1].get("same_problem").is_none());

    // A later run: 635 rests on 633, whose line it finds in --out; 636 is searched.
    let run = run_certify("634-636", &results, &out, &[]);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    assert!(run.stdout.contains("n=635 "), "{}", run.stdout);
    assert!(run.stdout.contains("certified (same problem as 633)"), "{}", run.stdout);
    assert!(
        run.stdout.contains("634..=636: certified 1, certified (same problem) 1, not needing one 1"),
        "{}",
        run.stdout
    );
    let lines = json_lines(&out);
    let at = |n: u32| lines.iter().find(|line| line.get("n").and_then(Json::as_u32) == Some(n)).unwrap();
    assert_eq!(at(635).get("same_problem").and_then(|same| same.get("n")).and_then(Json::as_u32), Some(633));
    assert_eq!(at(635).get("nodes").and_then(Json::as_u64), Some(0));
    assert!(at(636).get("same_problem").is_none());
    assert_eq!(at(635).get("digest"), at(633).get("digest"));
    let posed = extended_problem(633, &fixtures().upper[&633].open, true).unwrap();
    assert_eq!(at(633).get("digest").and_then(Json::as_str), Some(digest_text(posed.problem.digest()).as_str()));
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
        claims: true,
        verdict,
        digest: 0xfeed,
        same_problem: None,
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

    let good = certificate(30, Verdict::Certified).to_json();
    let without_digest = Json::object(good.as_obj().unwrap().iter().filter(|(key, _)| key != "digest").cloned());
    for (text, why) in [
        ("not json\n".to_owned(), "line 1"),
        (format!("{good}\n{{\"n\":32}}\n"), "line 2: no verdict"),
        ("{\"verdict\":\"beaten\"}\n".to_owned(), "line 1: no board size"),
        ("{\"n\":30,\"verdict\":\"maybe\"}\n".to_owned(), "line 1: no verdict"),
        (format!("{good}\n{without_digest}\n"), "line 2: missing \"digest\""),
    ] {
        let bad = dir.join("bad.jsonl");
        fs::write(&bad, &text).unwrap();
        let Err(message) = CertificateFile::open(&bad) else { panic!("{text:?} was read as certificates") };
        assert!(message.contains("bad.jsonl") && message.contains(why), "{text:?}: {message}");
    }
}

/// The file keeps the last certificate with a final verdict of each game, which is what a later game may rest on if
/// it is `certified`: `incomplete` lines are not kept, and the certified ones come out in the order of the boards.
#[test]
fn the_file_keeps_the_last_final_certificate_of_each_game() {
    let dir = TempDir::new("certify-finals");
    let file = dir.join("out.jsonl");
    let mut certificates = CertificateFile::open(&file).unwrap();
    let later = Certificate { score: 120, ..certificate(34, Verdict::Certified) };
    for line in [
        certificate(34, Verdict::Certified),
        certificate(30, Verdict::Incomplete),
        certificate(32, Verdict::Beaten),
        certificate(30, Verdict::Certified),
        later.clone(),
        certificate(34, Verdict::Incomplete),
    ] {
        certificates.append(&line).unwrap();
    }
    let boards = |file: &CertificateFile| file.certified().map(|c| c.n).collect::<Vec<_>>();
    assert_eq!(boards(&certificates), [30, 34]);
    assert_eq!(certificates.certificate(34), Some(&later));
    assert_eq!(certificates.certificate(32).map(|c| c.verdict), Some(Verdict::Beaten));
    assert_eq!(certificates.certificate(36), None);
    drop(certificates);

    let reopened = CertificateFile::open(&file).unwrap();
    assert_eq!(boards(&reopened), [30, 34]);
    assert_eq!(reopened.certificate(34), Some(&later));
    assert_eq!(reopened.certificate(30), Some(&certificate(30, Verdict::Certified)));
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

// ---------------------------------------------------------------------------------------------------------
// Verifying the certificates

/// The barred bounds to board 3314, which the first game the proofs do not reach is looked up in.
fn upper_files_to_3314() -> Vec<PathBuf> {
    let mut files = upper_files();
    files.push(crate_file("prover/data/upper_2176_3314.jsonl"));
    files
}

/// Runs `taxman verify-certificates` on the files `certs`, over the games of `results` and the barred bounds `upper`,
/// on one thread, with the flags `extra` besides.
fn run_verify(certs: &[&Path], results: &Path, upper: &[PathBuf], extra: &[&str]) -> Run {
    let mut args = vec!["verify-certificates"];
    args.extend(certs.iter().map(|file| path(file)));
    args.extend(["--results", path(results), "--threads", "1"]);
    args.extend(upper.iter().flat_map(|file| ["--upper", path(file)]));
    args.extend_from_slice(extra);
    taxman(&args)
}

/// The board of a line of certificates.
fn board_of(line: &Json) -> Option<u32> {
    line.get("n").and_then(Json::as_u32)
}

/// The stretch 1312..=1317 certified into `dir/certs.jsonl`: six games that need a certificate, of which 1313, 1315 and
/// 1317 rest on the game before them (`same_problem`).  Returns the file.
fn certify_stretch(dir: &Path) -> PathBuf {
    let certs = dir.join("certs.jsonl");
    let run = run_certify("1312-1317", &results_path("strict.jsonl"), &certs, &[]);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    let lines = json_lines(&certs);
    let rule: Vec<u32> = lines.iter().filter(|l| l.get("same_problem").is_some()).filter_map(board_of).collect();
    assert_eq!((lines.len(), rule), (6, vec![1313, 1315, 1317]));
    certs
}

/// `json` with the value at `keys` (the names of nested objects, ending with the key itself) replaced by `value`.
fn with_value(json: &Json, keys: &[&str], value: Json) -> Json {
    let pairs = json.as_obj().expect("an object");
    Json::object(pairs.iter().map(|(key, old)| {
        let new = match keys {
            [only] if key == only => value.clone(),
            [first, rest @ ..] if key == first => with_value(old, rest, value.clone()),
            _ => old.clone(),
        };
        (key.clone(), new)
    }))
}

/// Writes `lines`, one JSON object a line, to `name` in `dir`.
fn write_lines(dir: &Path, name: &str, lines: &[Json]) -> PathBuf {
    write_file(dir, name, &lines.iter().map(|line| format!("{line}\n")).collect::<String>())
}

/// The stretch 630..=640 certified into `dir/certs.jsonl`: seven games that need a certificate, of which 633 and 635
/// rest on the game before them (`same_problem`), the cheapest stretch with repeats.  Returns the file and the boards
/// of its lines.
fn certify_short_stretch(dir: &Path) -> (PathBuf, Vec<u32>) {
    let certs = dir.join("certs.jsonl");
    let run = run_certify("630-640", &results_path("strict.jsonl"), &certs, &[]);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    let lines = json_lines(&certs);
    let boards: Vec<u32> = lines.iter().filter_map(board_of).collect();
    let rule: Vec<u32> = lines.iter().filter(|l| l.get("same_problem").is_some()).filter_map(board_of).collect();
    assert_eq!((boards.len(), rule), (7, vec![633, 635]));
    (certs, boards)
}

/// The lines of a certified stretch with line `k` changed at `keys`.
fn with_line_changed(lines: &[Json], k: usize, keys: &[&str], value: Json) -> Vec<Json> {
    let mut doctored = lines.to_vec();
    doctored[k] = with_value(&lines[k], keys, value);
    doctored
}

/// The checks of [`a_file_certify_wrote_passes_and_the_summary_counts_it`] and of the first of
/// [`doctored_lines_are_refused_naming_the_line`] on the stretch 630..=640 instead of 1312..=1317, which takes a tenth
/// of the time to certify: the file passes with the right counts, and a line with another digest or another score than
/// the barred bounds' is refused naming the line and the board.
#[test]
fn a_short_stretch_certified_passes_and_a_doctored_digest_or_score_is_refused() {
    let dir = TempDir::new("verify-short");
    let (certs, boards) = certify_short_stretch(&dir);
    let (results, upper) = (results_path("strict.jsonl"), upper_files());
    let lines = json_lines(&certs);
    let run = run_verify(&[&certs], &results, &upper, &[]);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    assert_eq!(run.stderr, "");
    let counts = "7 lines, 7 final, 0 incomplete; certified by search 5, certified by the same problem 2, beaten 0";
    assert!(run.stdout.contains(&format!("{}: {counts}", certs.display())), "{}", run.stdout);
    assert!(run.stdout.contains("boards certified: 7 ("), "{}", run.stdout);
    assert!(run.stdout.contains("counterexamples: none"), "{}", run.stdout);

    let wrong = with_line_changed(&lines, 2, &["digest"], Json::from("0123456789abcdef"));
    assert_refused(&dir, "digest.jsonl", &wrong, 3, boards[2], "Π⁺(O) set up again is not the line's");
    let score = number_at(&lines[0], &["score"]);
    let wrong = with_line_changed(&lines, 0, &["score"], Json::from(score + 1));
    let why = format!("its score is {}, that of the --upper files {score}", score + 1);
    assert_refused(&dir, "score.jsonl", &wrong, 1, boards[0], &why);
}

/// The checks of [`doctored_lines_are_refused_naming_the_line`] on a game that rests on another, on the stretch
/// 630..=640: a `same_problem` with a wrong `best` is refused, and so is one whose game has no line in the file.
#[test]
fn a_doctored_same_problem_of_a_short_stretch_is_refused() {
    let dir = TempDir::new("verify-short-rule");
    let (certs, boards) = certify_short_stretch(&dir);
    let lines = json_lines(&certs);
    let k = lines.iter().position(|line| line.get("same_problem").is_some()).unwrap();
    let rests_on = number_at(&lines[k], &["same_problem", "n"]) as u32;
    let best = number_at(&lines[k], &["same_problem", "best"]);
    let wrong = with_line_changed(&lines, k, &["same_problem", "best"], Json::from(best + 1));
    let why = format!("its same_problem is wrong: best {} (the certificate of n={rests_on} proved {best})", best + 1);
    assert_refused(&dir, "best.jsonl", &wrong, k + 1, boards[k], &why);
    let at = boards.iter().position(|&n| n == rests_on).unwrap();
    let mut removed = lines.clone();
    removed.remove(at);
    let why = format!("it rests on n={rests_on}, but the file has no final line of");
    assert_refused(&dir, "removed.jsonl", &removed, at + 1, boards[k], &why);
}

/// The checks of [`incomplete_lines_are_counted_a_torn_last_line_is_ignored_and_a_bad_line_is_refused`] on the stretch
/// 630..=640: an `incomplete` line is counted and skipped, a torn last line is ignored, and a line that is no
/// certificate is refused naming the file and the line.
#[test]
fn an_incomplete_and_a_torn_and_a_bad_line_of_a_short_stretch_are_handled() {
    let dir = TempDir::new("verify-short-torn");
    let (certs, _) = certify_short_stretch(&dir);
    let (results, upper) = (results_path("strict.jsonl"), upper_files());
    let lines = json_lines(&certs);
    let incomplete = with_value(&lines[0], &["verdict"], Json::from("incomplete"));
    let mut text: String = [vec![incomplete], lines.clone()].concat().iter().map(|line| format!("{line}\n")).collect();
    text.push_str("{\"n\":641,\"verd");
    let file = write_file(&dir, "torn.jsonl", &text);
    let run = run_verify(&[&file], &results, &upper, &[]);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    let counts = "8 lines, 7 final, 1 incomplete; certified by search 5, certified by the same problem 2, beaten 0";
    let torn = format!("{}: {counts}; a torn last line (ignored)", file.display());
    assert!(run.stdout.contains(&torn), "{}", run.stdout);
    assert!(run.stderr.contains(&format!("note: {}: torn last line ignored", file.display())), "{}", run.stderr);

    let text: String = lines.iter().map(|line| format!("{line}\n")).collect::<String>() + "this is not json\n";
    let file = write_file(&dir, "bad.jsonl", &text);
    let run = run_verify(&[&file], &results, &upper, &[]);
    assert_eq!(run.code, Some(1), "{}{}", run.stdout, run.stderr);
    assert!(run.stderr.contains(&format!("{}: line 8: JSON syntax error", file.display())), "{}", run.stderr);
    assert_eq!(run.stdout, "");
}

/// The check of [`a_line_whose_score_is_not_the_results_is_refused`] on the game 1305, the cheapest past the table
/// that the results are not known by (the table ends at 1300): a score changed in the results' game, with the line
/// right, is refused.
#[test]
fn a_line_whose_score_is_not_the_results_game_is_refused() {
    let (n, dir) = (1305, TempDir::new("verify-short-score"));
    let certs = dir.join("certs.jsonl");
    let run = run_certify(&format!("{n}-{n}"), &results_path("strict.jsonl"), &certs, &[]);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    let (results, planted, _) = plant_wrong_score(&dir, n);
    let run = run_verify(&[&certs], &results, &upper_files(), &[]);
    assert_eq!(run.code, Some(1), "{}{}", run.stdout, run.stderr);
    let score = fixtures().known.score(n).unwrap();
    let message = format!("line 1: n={n}: its score is {score}, that of the results' game {planted}");
    assert!(run.stderr.contains(&message), "{}", run.stderr);
}

/// A file of certificates `taxman certify` wrote over a stretch with repeats passes, through the binary: the summary
/// counts its lines right (six, of which three rest on the game before them), names the boards, the build and what is
/// taken on trust, and gives the proven end of the chain with the committed prover files: every game to 2175, the first
/// past it needing a certificate that the files do not have.  The result is the same on two threads.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(a_short_stretch_certified_passes_and_a_doctored_digest_or_score_is_refused))]
fn a_file_certify_wrote_passes_and_the_summary_counts_it() {
    let dir = TempDir::new("verify-passes");
    let (certs, results) = (certify_stretch(&dir), results_path("strict.jsonl"));
    let upper = upper_files_to_3314();
    let run = run_verify(&[&certs], &results, &upper, &[]);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    assert_eq!(run.stderr, "");
    let out = &run.stdout;
    let counts = "6 lines, 6 final, 0 incomplete; certified by search 3, certified by the same problem 3, beaten 0";
    assert!(out.contains(&format!("{}: {counts}", certs.display())), "{out}");
    assert!(out.contains("boards certified: 6 (1312-1317)"), "{out}");
    assert!(out.contains("counterexamples: none"), "{out}");
    let build = json_lines(&certs)[0].get("build").cloned().unwrap();
    let commit = build.get("commit").and_then(Json::as_str).unwrap();
    let dirty = if build.get("dirty") == Some(&Json::Bool(true)) { " (dirty)" } else { "" };
    assert!(out.contains(&format!("  {commit}{dirty}: 6 lines")), "{out}");
    assert!(out.contains("not checked: that a search explored its whole tree"), "{out}");
    let files = taxman::paths::exact_files_in_repository().unwrap();
    assert_eq!(files.len(), 4, "{files:?}");
    assert!(out.contains(&format!("the prover's proofs read, from 4 files: {}", files[0].display())), "{out}");
    let end = "proven frontier: every game to 2175 is proven; the first that is not is 2176 (needs a certificate and \
               has none in the files given)";
    assert!(out.contains(end), "{out}");
    assert!(out.contains("proven past it all the same: 0 games"), "{out}");

    // The same on two threads, but for the time it took.
    let steady = |out: &str| out.lines().filter(|line| !line.starts_with("checked in")).collect::<Vec<_>>().join("\n");
    let again = run_verify(&[&certs], &results, &upper, &["--threads", "2"]);
    assert_eq!(again.code, Some(0), "{}{}", again.stdout, again.stderr);
    assert_eq!(steady(&again.stdout), steady(out));
}

/// A file that mixes the formulations passes: the first game of a stretch certified with `--no-claims` and then the
/// stretch with the claims, into the same `--out`, which skips the first game and settles the next from its line (the
/// problem of that game set up again with the claims is the next game's), and each after that from the game before it,
/// which was made with them.  The stretches are ones where the games pose different problems without the claims and one
/// problem with them (609 and 611, with five open sources each; 667, 669 and 671, with none), so the rule settles
/// across the formulations what it could not within the older one.  The counts are those of a file of one formulation.
/// A line whose formulation is changed to the other is refused, naming the line and the board: the problem set up again
/// in that formulation is not the line's.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(a_file_that_mixes_the_formulations_over_609_to_611_passes_and_a_changed_formulation_is_refused))]
fn a_file_that_mixes_the_formulations_passes_and_a_changed_formulation_is_refused() {
    a_file_that_mixes_the_formulations_over(&[(609, 611), (667, 671)]);
}

/// [`a_file_that_mixes_the_formulations_passes_and_a_changed_formulation_is_refused`] for the stretch 609..=611 alone.
#[test]
fn a_file_that_mixes_the_formulations_over_609_to_611_passes_and_a_changed_formulation_is_refused() {
    a_file_that_mixes_the_formulations_over(&[(609, 611)]);
}

/// The file that mixes the formulations over each of the stretches `stretches` (the first and last game of each) passes,
/// and each line with its formulation flipped is refused.
fn a_file_that_mixes_the_formulations_over(stretches: &[(u32, u32)]) {
    let dir = TempDir::new("verify-mixed");
    let results = results_path("strict.jsonl");
    for &(first, last) in stretches {
        let boards = needing(first..=last);
        let total = boards.len();
        let (without, with) = (problems_of(&boards, false), problems_of(&boards, true));
        assert!(boards.iter().all(|n| with[n] == with[&boards[0]]), "{boards:?} pose one problem with the claims");
        assert!(boards[1..].iter().all(|n| without[n] != without[&boards[0]]), "{boards:?}: not without them");

        let certs = dir.join(format!("certs-{first}.jsonl"));
        let run = run_certify(&format!("{first}-{first}"), &results, &certs, &["--no-claims"]);
        assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
        assert!(run.stdout.contains("(no claims)"), "{}", run.stdout);
        let run = run_certify(&format!("{first}-{last}"), &results, &certs, &[]);
        assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
        assert!(run.stdout.contains("already in --out 1"), "{}", run.stdout);
        assert!(!run.stdout.contains("(no claims)"), "{}", run.stdout);

        let lines = json_lines(&certs);
        assert_eq!(lines.iter().filter_map(board_of).collect::<Vec<_>>(), boards);
        let claims: Vec<bool> = lines.iter().map(|line| line.get("claims") == Some(&Json::Bool(true))).collect();
        assert_eq!(claims, (0..total).map(|k| k > 0).collect::<Vec<_>>(), "the first game is the one made without");
        for (k, line) in lines.iter().enumerate().skip(1) {
            let rests_on = line.get("same_problem").and_then(|same| same.get("n")).and_then(Json::as_u32);
            assert_eq!(rests_on, Some(boards[k - 1]), "n={}: settled by the game before it", boards[k]);
        }
        assert_ne!(lines[0].get("digest"), lines[1].get("digest"), "the line without the claims is of another problem");

        let run = run_verify(&[&certs], &results, &upper_files(), &[]);
        assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
        let counts = format!(
            "{total} lines, {total} final, 0 incomplete; certified by search 1, certified by the same problem {}, \
             beaten 0",
            total - 1
        );
        assert!(run.stdout.contains(&format!("{}: {counts}", certs.display())), "{}", run.stdout);
        assert!(run.stdout.contains(&format!("boards certified: {total} (")), "{}", run.stdout);

        // The formulation of a line changed: the problem set up again is not the line's (a line that rests on another
        // is checked before the rule is, so the first to fail is the line whose formulation was changed).
        for (k, line) in lines.iter().enumerate() {
            let flipped = with_value(line, &["claims"], Json::Bool(!claims[k]));
            let mut doctored = lines.clone();
            doctored[k] = flipped;
            let (name, why) = (format!("flipped-{}.jsonl", boards[k]), "Π⁺(O) set up again is not the line's");
            assert_refused(&dir, &name, &doctored, k + 1, boards[k], why);
        }
    }
}

/// Verifying the file of `lines` (written to `name` in `dir`, over the real games and bounds) fails with status 1 and
/// prints nothing on stdout; the message names the file, the line `at` (from 1), the board `n` and `why`.
fn assert_refused(dir: &Path, name: &str, lines: &[Json], at: usize, n: u32, why: &str) {
    let file = write_lines(dir, name, lines);
    let run = run_verify(&[&file], &results_path("strict.jsonl"), &upper_files(), &[]);
    assert_eq!(run.code, Some(1), "{name}: {}{}", run.stdout, run.stderr);
    let named = format!("error: {}: line {at}: ", file.display());
    let board = format!("n={n}");
    for text in [named.as_str(), board.as_str(), why] {
        assert!(run.stderr.contains(text), "{name}: no {text:?} in {:?}", run.stderr);
    }
    assert_eq!(run.stdout, "", "{name}");
}

/// The number at `keys` (nested object names) in `json`.
fn number_at(json: &Json, keys: &[&str]) -> u64 {
    let found = keys.iter().try_fold(json, |json, key| json.get(key));
    found.and_then(Json::as_u64).unwrap_or_else(|| panic!("no number at {keys:?} in {json}"))
}

/// Each doctored line of a file `taxman certify` wrote is refused, with a message that names the file, the line and
/// the board: another digest, a wrong score in the line, open members that are not the barred bounds', a
/// `same_problem` that names a game with no certified line before it (its line removed, its verdict changed, a game
/// outside the window, a game with another problem) or has its `best`, `worth` or `digest` changed, a certified line
/// with an entry in the counterexample file, a `build` without a commit, and a width that is not the one the problem
/// takes.  (A wrong score in the results' game is in the next test.)
#[test]
#[cfg_attr(debug_assertions, ignore = long!(a_doctored_same_problem_of_a_short_stretch_is_refused))]
fn doctored_lines_are_refused_naming_the_line() {
    let dir = TempDir::new("verify-doctored");
    let lines = json_lines(&certify_stretch(&dir));
    let boards: Vec<u32> = lines.iter().filter_map(board_of).collect();
    assert_eq!(boards, [1312, 1313, 1314, 1315, 1316, 1317]);
    // The lines with line `k` changed at `keys`.
    let doctored = |k: usize, keys: &[&str], value: Json| {
        let mut doctored = lines.clone();
        doctored[k] = with_value(&lines[k], keys, value);
        doctored
    };

    // Another digest of the problem.
    let wrong = doctored(2, &["digest"], Json::from("0123456789abcdef"));
    assert_refused(&dir, "digest.jsonl", &wrong, 3, 1314, "Π⁺(O) set up again is not the line's");
    // A score that is not the barred bounds'.
    let (score, v) = (number_at(&lines[0], &["score"]), "its score is");
    let wrong = doctored(0, &["score"], Json::from(score + 1));
    assert_refused(
        &dir,
        "score.jsonl",
        &wrong,
        1,
        1312,
        &format!("{v} {}, that of the --upper files {score}", score + 1),
    );
    // Open members that are not the barred bounds'.
    let open = lines[2].get("open").and_then(Json::as_u32_vec).unwrap();
    let wrong = doctored(2, &["open"], Json::array(open[1..].iter().copied()));
    assert_refused(
        &dir,
        "open.jsonl",
        &wrong,
        3,
        1314,
        &format!("its open members are {:?}, those of the", &open[1..]),
    );
    // A width that is not the one the problem takes.
    let wrong = doctored(4, &["width"], Json::from(16u32));
    assert_refused(&dir, "width.jsonl", &wrong, 5, 1316, "it records the width 16, but its");
    // A build without a commit.
    let wrong = doctored(3, &["build", "commit"], Json::Null);
    assert_refused(&dir, "build.jsonl", &wrong, 4, 1315, "the \"build\" has no \"commit\" of 40 hexadecimal digits");

    // A game that rests on another with no certified line before it: the line removed, its verdict changed, a game that
    // is none of the eight before it, and a game with another problem.
    assert_refused(
        &dir,
        "removed.jsonl",
        &lines[1..],
        1,
        1313,
        "it rests on n=1312, but the file has no final line of",
    );
    let wrong = doctored(0, &["verdict"], Json::from("beaten"));
    assert_refused(&dir, "beaten.jsonl", &wrong, 2, 1313, "whose last final line before it (line 1) is beaten");
    let wrong = doctored(1, &["same_problem", "n"], Json::from(1300u32));
    assert_refused(&dir, "window.jsonl", &wrong, 2, 1313, "it rests on n=1300, which is none of the 8 board sizes");
    let wrong = doctored(3, &["same_problem", "n"], Json::from(1313u32));
    assert_refused(&dir, "other.jsonl", &wrong, 4, 1315, "its problem is not that of n=1313, which it rests on");

    // A `same_problem` with its best, worth or digest changed.
    let best = number_at(&lines[1], &["same_problem", "best"]);
    let wrong = doctored(1, &["same_problem", "best"], Json::from(best + 1));
    let why = format!("its same_problem is wrong: best {} (the certificate of n=1312 proved {best})", best + 1);
    assert_refused(&dir, "best.jsonl", &wrong, 2, 1313, &why);
    let worth = number_at(&lines[3], &["same_problem", "worth"]);
    let wrong = doctored(3, &["same_problem", "worth"], Json::from(worth + 1));
    assert_refused(&dir, "worth.jsonl", &wrong, 4, 1315, &format!("worth {} (the line's score less", worth + 1));
    let wrong = doctored(5, &["same_problem", "digest"], Json::from("0123456789abcdef"));
    assert_refused(&dir, "same-digest.jsonl", &wrong, 6, 1317, "its same_problem is wrong: digest 0123456789abcdef");

    // A certified line with a line of its board in the counterexample file beside the certificates.
    let file = write_lines(&dir, "ce.jsonl", &lines);
    write_lines(&dir, "ce.counterexample.jsonl", &lines[2..3]);
    let run = run_verify(&[&file], &results_path("strict.jsonl"), &upper_files(), &[]);
    assert_eq!(run.code, Some(1), "{}{}", run.stdout, run.stderr);
    let message = format!("error: {}: line 3: n=1314: it is certified, but ", file.display());
    assert!(run.stderr.contains(&message), "{}", run.stderr);
    assert!(run.stderr.contains("ce.counterexample.jsonl has a counterexample for it"), "{}", run.stderr);
}

/// A score changed in the results' game, with the line right, is refused too: the line's score is the barred bounds',
/// but not the results'.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(a_line_whose_score_is_not_the_results_game_is_refused))]
fn a_line_whose_score_is_not_the_results_is_refused() {
    let dir = TempDir::new("verify-results-score");
    let lines = json_lines(&certify_stretch(&dir));
    let known = &fixtures().known;
    let (planted, kept, _) = planted_game(1314);
    let games: Vec<(u32, u64, &[u32])> = (1312..=1317)
        .map(|n| match n {
            1314 => (n, planted, &kept[..]),
            _ => (n, known.score(n).unwrap(), known.moves(n).unwrap()),
        })
        .collect();
    let results = dir.join("doctored-results.jsonl");
    known_with(&results, &games);
    let certs = write_lines(&dir, "certs-again.jsonl", &lines);

    let run = run_verify(&[&certs], &results, &upper_files(), &[]);
    assert_eq!(run.code, Some(1), "{}{}", run.stdout, run.stderr);
    let score = known.score(1314).unwrap();
    let message = format!("line 3: n=1314: its score is {score}, that of the results' game {planted}");
    assert!(run.stderr.contains(&message), "{}", run.stderr);

    // The real results pass, with one of the prover's files; and a proof of the prover's with another score than the
    // results' is an error that names the board and both scores.
    let exact = taxman::paths::exact_files_in_repository().unwrap();
    let real = results_path("strict.jsonl");
    let run = run_verify(&[&certs], &real, &upper_files(), &["--exact", path(&exact[0])]);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    let proof = "{\"n\":1314,\"score\":1,\"fm_bound\":1,\"relation\":\"none\",\"search\":\"skipped\"}\n";
    let wrong = write_file(&dir, "exact_wrong.jsonl", proof);
    let run = run_verify(&[&certs], &real, &upper_files(), &["--exact", path(&wrong)]);
    assert_eq!(run.code, Some(1), "{}{}", run.stdout, run.stderr);
    let message = format!("error: n=1314: the prover's proof is of the score 1, the results' game scores {score}");
    assert!(run.stderr.contains(&message), "{}", run.stderr);
}

/// A line that is no certificate, a last line that was torn, and lines that are `incomplete`: a complete line that is
/// no certificate is refused naming the file and the line (the way the file refuses it when `certify` opens it), a
/// torn last line is noted on stderr and ignored, and an `incomplete` line is counted and skipped.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(an_incomplete_and_a_torn_and_a_bad_line_of_a_short_stretch_are_handled))]
fn incomplete_lines_are_counted_a_torn_last_line_is_ignored_and_a_bad_line_is_refused() {
    let dir = TempDir::new("verify-torn");
    let lines = json_lines(&certify_stretch(&dir));
    let (results, upper) = (results_path("strict.jsonl"), upper_files());
    let incomplete = with_value(&lines[0], &["verdict"], Json::from("incomplete"));
    let mut text: String = [vec![incomplete], lines.clone()].concat().iter().map(|line| format!("{line}\n")).collect();
    text.push_str("{\"n\":1318,\"verd");
    let file = write_file(&dir, "torn.jsonl", &text);

    let run = run_verify(&[&file], &results, &upper, &[]);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    let counts = "7 lines, 6 final, 1 incomplete; certified by search 3, certified by the same problem 3, beaten 0";
    assert!(
        run.stdout.contains(&format!("{}: {counts}; a torn last line (ignored)", file.display())),
        "{}",
        run.stdout
    );
    assert!(run.stderr.contains(&format!("note: {}: torn last line ignored", file.display())), "{}", run.stderr);
    assert!(run.stdout.contains("boards certified: 6 (1312-1317)"), "{}", run.stdout);

    // A complete line that is no certificate: not JSON, a certificate without a field.
    for (bad, why) in [
        ("this is not json\n", "line 7: JSON syntax error"),
        ("{\"n\":1318,\"verdict\":\"certified\"}\n", "line 7: missing \"score\""),
    ] {
        let text: String = lines.iter().map(|line| format!("{line}\n")).collect::<String>() + bad;
        let file = write_file(&dir, "bad.jsonl", &text);
        let run = run_verify(&[&file], &results, &upper, &[]);
        assert_eq!(run.code, Some(1), "{}{}", run.stdout, run.stderr);
        assert!(run.stderr.contains(&format!("{}: {why}", file.display())), "{}", run.stderr);
        assert_eq!(run.stdout, "");
    }
}

/// A beaten game is verified too, with its counterexample.  A wrong score is planted in the results and, as the prover
/// would have found them against it, in the barred bounds; `taxman certify` writes a `beaten` line and its
/// counterexample, and `verify-certificates` passes both and lists the counterexample.  Without the counterexample's
/// line, or with a game that does not beat the score, it refuses; and a board with a certified line in one place and a
/// beaten line in another has contradictory verdicts, in two files and in one.
#[test]
fn a_beaten_line_is_verified_with_its_counterexample_and_cannot_stand_beside_a_certified_one() {
    let n = 1302;
    let dir = TempDir::new("verify-beaten");
    let (results, planted, _) = plant_wrong_score(&dir, n);
    let real = fixtures().known.score(n).unwrap();
    let members: Vec<[u32; 3]> = fixtures().upper[&n].open.iter().map(|&u| [u, 3, 0]).collect();
    let bounds = write_file(&dir, "planted-upper.jsonl", &upper_line(n, planted, Some(&members)));
    let beaten = dir.join("beaten.jsonl");
    let args = ["--results", path(&results), "--upper", path(&bounds), "--threads", "1"];
    let run = taxman(&[&["certify", "1302", "--out", path(&beaten), "--progress", "0"][..], &args].concat());
    assert_eq!(run.code, Some(2), "{}{}", run.stdout, run.stderr);

    let exact = taxman::paths::exact_files_in_repository().unwrap();
    let verify = |files: &[&Path]| {
        let certs: Vec<&str> = files.iter().map(|file| path(file)).collect();
        taxman(&[&["verify-certificates"][..], &certs, &args, &["--exact", path(&exact[0])]].concat())
    };
    let run = verify(&[&beaten]);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    let counts = "1 lines, 1 final, 0 incomplete; certified by search 0, certified by the same problem 0, beaten 1";
    assert!(run.stdout.contains(&format!("{}: {counts}", beaten.display())), "{}", run.stdout);
    assert!(run.stdout.contains("boards certified: 0 (none)"), "{}", run.stdout);
    let found =
        format!("n={n}: the recorded score {planted} is beaten by a game of {real} ({} line 1)", beaten.display());
    assert!(run.stdout.contains(&format!("counterexamples: 1\n  {found}")), "{}", run.stdout);

    // Without its line in the counterexample file, and with a game that does not beat the score.
    let kept = fs::read_to_string(dir.join("beaten.counterexample.jsonl")).unwrap();
    fs::remove_file(dir.join("beaten.counterexample.jsonl")).unwrap();
    let run = verify(&[&beaten]);
    assert_eq!(run.code, Some(1), "{}{}", run.stdout, run.stderr);
    let message = format!("error: {}: line 1: n={n}: ", beaten.display());
    assert!(
        run.stderr.contains(&message) && run.stderr.contains("beaten.counterexample.jsonl has no line identical to it"),
        "{}",
        run.stderr
    );
    fs::write(dir.join("beaten.counterexample.jsonl"), &kept).unwrap();
    let line = json_lines(&beaten).remove(0);
    let shortfall =
        write_lines(&dir, "short.jsonl", &[with_value(&line, &["beaten_by", "score"], Json::from(planted))]);
    fs::write(dir.join("short.counterexample.jsonl"), &kept).unwrap();
    let run = verify(&[&shortfall]);
    assert_eq!(run.code, Some(1), "{}{}", run.stdout, run.stderr);
    let message = format!("the game that beats it scores {real}, not the {planted} recorded");
    assert!(run.stderr.contains(&message), "{}", run.stderr);

    // A certified line of the same board beside it: in another file, and in the same file.
    let certified = dir.join("certified.jsonl");
    let run = run_certify("1302", &results_path("strict.jsonl"), &certified, &[]);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    let strict = results_path("strict.jsonl");
    let real_args = ["--results", path(&strict), "--threads", "1"];
    let uppers = upper_files();
    let contradictory = |files: &[&Path], here: &Path, there: &Path| {
        let certs: Vec<&str> = files.iter().map(|file| path(file)).collect();
        let upper: Vec<&str> = uppers.iter().flat_map(|file| ["--upper", path(file)]).collect();
        let run =
            taxman(&[&["verify-certificates"][..], &certs, &real_args, &upper, &["--exact", path(&exact[0])]].concat());
        assert_eq!(run.code, Some(1), "{}{}", run.stdout, run.stderr);
        let message = format!("error: {}: line ", here.display());
        let why = format!("n={n}: contradictory verdicts: beaten here, and certified at {} line 1", there.display());
        assert!(run.stderr.contains(&message) && run.stderr.contains(&why), "{}", run.stderr);
    };
    contradictory(&[&certified, &beaten], &beaten, &certified);
    let both = write_lines(&dir, "both.jsonl", &[json_lines(&certified).remove(0), line]);
    contradictory(&[&both], &both, &both);
}

// ---------------------------------------------------------------------------------------------------------
// How far the proofs reach

/// The prover's proof of a score, for a synthetic chain: the bounds, the search it made and its relation to the game
/// before.
fn proof(score: u64, fm_bound: u64, search: &str, segments: bool, relation: &str) -> ExactLine {
    ExactLine {
        score,
        fm_bound,
        parity_bound: None,
        search: search.to_owned(),
        segments,
        relation: relation.to_owned(),
    }
}

/// The proven end of the chain on synthetic data, a game at a time.  Game 1 is proven; 2 is at the ceiling 2 + 1
/// after a proven game; 3 has a proof of the prover's (its Franklín–Moniot bound is its score); 4 is at its ceiling
/// again; 5 is certified.  Game 6 has a proof whose bounds are above its score and whose search did not finish, and
/// is the first not proven.  Game 7 is at its ceiling, but the game before it is not proven, and 8 has a finished
/// search that used the segment cuts, which counts only when every smaller game is proven: neither is.  Past the gap,
/// 9 is proven by its bound, 10 is certified and 11 is at its ceiling after the proven 10.
#[test]
fn the_proven_end_of_the_chain_on_synthetic_data() {
    let scores: BTreeMap<u32, u64> =
        [(1, 1), (2, 3), (3, 5), (4, 9), (5, 12), (6, 17), (7, 24), (8, 30), (9, 40), (10, 52), (11, 63)].into();
    let mut exact: BTreeMap<u32, ExactLine> = BTreeMap::new();
    exact.insert(3, proof(5, 5, "skipped", false, "none"));
    exact.insert(6, ExactLine { parity_bound: Some(19), ..proof(17, 20, "budget", false, "none") });
    exact.insert(8, proof(30, 40, "proven", true, "none"));
    exact.insert(9, proof(40, 40, "skipped", false, "none"));
    let certified: BTreeSet<u32> = [5, 10].into();

    let reached = frontier(&scores, &exact, &certified).unwrap();
    assert_eq!(reached, Frontier { through: 5, first_gap: Some((6, Gap::Unproven)), beyond: vec![9, 10, 11] });

    // The same search without the segment cuts proves game 8 past the gap; with them, once every smaller game is proven
    // (game 6 certified), it proves it too.
    exact.insert(8, proof(30, 40, "proven", false, "none"));
    let reached = frontier(&scores, &exact, &certified).unwrap();
    assert_eq!(reached.beyond, [8, 9, 10, 11]);
    exact.insert(8, proof(30, 40, "proven", true, "none"));
    let mended: BTreeSet<u32> = [5, 6, 10].into();
    let reached = frontier(&scores, &exact, &mended).unwrap();
    assert_eq!(reached, Frontier { through: 11, first_gap: None, beyond: vec![] });

    // A game whose line says `chain` or `prime` is proven when the game before it is, and only then.
    exact.insert(8, proof(30, 99, "skipped", false, "chain"));
    let reached = frontier(&scores, &exact, &certified).unwrap();
    assert_eq!(reached.beyond, [9, 10, 11], "game 7 is not proven, so game 8 is not");
    let reached = frontier(&scores, &exact, &mended).unwrap();
    assert_eq!((reached.through, reached.first_gap), (11, None));

    // A game the results lack is the first gap; the games past it are proven all the same where they can be.
    let mut missing = scores.clone();
    missing.remove(&4);
    let reached = frontier(&missing, &exact, &certified).unwrap();
    assert_eq!(reached, Frontier { through: 3, first_gap: Some((4, Gap::NoGame)), beyond: vec![5, 9, 10, 11] });

    // Nothing in the results, and nothing proven but the ceiling of game 2.
    let nothing = Frontier { through: 0, first_gap: None, beyond: vec![] };
    assert_eq!(frontier(&BTreeMap::new(), &exact, &certified).unwrap(), nothing);
    let reached = frontier(&scores, &BTreeMap::new(), &BTreeSet::new()).unwrap();
    assert_eq!(reached, Frontier { through: 2, first_gap: Some((3, Gap::Unproven)), beyond: vec![] });
}

/// A proof of the prover's with another score than the results' for the board is an error that names the board and both
/// scores, whether or not the proof is needed.
#[test]
fn a_proof_with_another_score_than_the_results_is_an_error() {
    let scores: BTreeMap<u32, u64> = [(1, 1), (2, 3), (3, 6)].into();
    let exact: BTreeMap<u32, ExactLine> = [(3, proof(5, 5, "skipped", false, "none"))].into();
    let e = frontier(&scores, &exact, &BTreeSet::new()).unwrap_err();
    assert_eq!(e, "n=3: the prover's proof is of the score 5, the results' game scores 6");
    // A proof of a board past the results' is of nothing the results have: ignored.
    let exact: BTreeMap<u32, ExactLine> = [(4, proof(6, 6, "skipped", false, "none"))].into();
    assert_eq!(frontier(&scores, &exact, &BTreeSet::new()).unwrap().through, 3);
}

/// The committed prover files prove every game of the chain to 2175 (the prover's own proofs, without a certificate),
/// and the committed chain past it, to 3440, is not proven at 2176, the first game that needs a certificate: nothing
/// past it is proven, since each game rests on the one before it.
#[test]
fn the_prover_proves_the_committed_chain_to_2175() {
    let exact = read_exact(&taxman::paths::exact_files_in_repository().unwrap()).unwrap();
    let scores: BTreeMap<u32, u64> = fixtures().known.games().map(|(n, score, _)| (n, score)).collect();
    assert_eq!(scores.keys().next_back(), Some(&3440));
    let to_2175: BTreeMap<u32, u64> = scores.range(..=2175).map(|(&n, &score)| (n, score)).collect();
    let reached = frontier(&to_2175, &exact, &BTreeSet::new()).unwrap();
    assert_eq!(reached, Frontier { through: 2175, first_gap: None, beyond: vec![] });

    let reached = frontier(&scores, &exact, &BTreeSet::new()).unwrap();
    assert_eq!(reached, Frontier { through: 2175, first_gap: Some((2176, Gap::Unproven)), beyond: vec![] });
    // With every game of 2176..=3440 that needs a certificate certified, the whole chain is proven.
    let bounds = read_upper(&upper_files_to_3314()).unwrap();
    let certified: BTreeSet<u32> =
        bounds.values().filter(|line| line.needs && line.n > 2175).map(|line| line.n).collect();
    let reached = frontier(&scores, &exact, &certified).unwrap();
    assert!(reached.through >= 3314, "{reached:?}: the games to 3314 that need a certificate are certified");
}

/// The set width of a certificate: the board's own where the GF(2) bound's block words fit its engines (1302 with its
/// three open members), an error naming the board for an open member that cannot be open and for a board beyond the
/// widest set; and a game whose open members need more block words than any engine has (here all of U\*_{≥3} of the
/// board 3000, which the barred bounds never say is open) is refused by `certify`, naming the words, before any search.
#[test]
fn the_width_a_certificate_takes_is_the_boards_own_where_the_block_words_fit() {
    let open = &fixtures().upper[&1302].open;
    let (words, width) = certificate_width(1302, open, true).unwrap();
    assert_eq!((words <= 3, width), (true, width_for(words_for(1302))));
    assert!(certificate_width(1302, &[7], true).unwrap_err().starts_with("n=1302: "));
    assert!(certificate_width(8192, &[], true).unwrap_err().contains("beyond the largest supported board"));

    let (n, known) = (3000, &fixtures().known);
    let open: Vec<u32> = taxman::with_width!(n, W => {
        let board = taxman::board::Board::<W>::for_n(n);
        board.plain.iter().copied().filter(|&x| 2 * x > n && board.tables.omega(x) >= 3).collect()
    });
    assert_eq!(certificate_width(n, &open, true).unwrap().1, None);
    let upper = Upper { n, score: known.score(n).unwrap(), needs: true, open };
    let e = certify_game(n, known, &upper, true, &options(1)).unwrap_err();
    assert!(e.starts_with("n=3000: ") && e.contains("block words of the GF(2) bound"), "{e}");
}

// ---------------------------------------------------------------------------------------------------------
// The work list of the campaign, and the committed certificates

/// Every committed file of the prover's barred bounds, `prover/data/upper_*.jsonl`, ordered by name.
fn committed_upper_files() -> Vec<PathBuf> {
    let dir = crate_file("prover/data");
    let entries = fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    let mut files: Vec<PathBuf> = entries.map(|entry| entry.unwrap().path()).collect();
    files.retain(|file| {
        let name = file.file_name().and_then(|name| name.to_str()).unwrap_or_default();
        name.starts_with("upper_") && name.ends_with(".jsonl")
    });
    files.sort();
    files
}

/// The field `key` of a line, which it must have.
fn field<'a>(line: &'a Json, key: &str) -> &'a Json {
    line.get(key).unwrap_or_else(|| panic!("no field {key:?} in {line}"))
}

/// The work list's own tests (`tools/test_certify_plan.py`, standard library only) pass.
#[test]
fn the_work_list_tool_passes_its_own_tests() {
    let run = python(&crate_file("tools/test_certify_plan.py"), &[]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
}

/// The work list of 600..=1300, made from the games, the barred bounds and the problems file alone, agrees with the
/// certificates that `taxman certify` made over the same boards (`bench/certify/certificates_600_1300.jsonl`): it
/// names the games that were certified and no other, with the open members and the rejected sources of each, and it
/// predicts the games that the rule of the same problem settled with no search, and no other.  Every other game that
/// needs a certificate has a cost to plan with (or says that it has none to know), and a game that needs none costs
/// nothing.
#[test]
fn the_work_list_on_600_to_1300_agrees_with_the_certificates() {
    let dir = TempDir::new("work-list");
    let results = results_path("strict.jsonl");
    let upper = crate_file("prover/data/upper_2_1300.jsonl");
    let problems = crate_file("bench/certify/problems_600_3440.jsonl");
    let args = ["--results", path(&results), "--upper", path(&upper), "--problems", path(&problems)];
    let range = ["--from", "600", "--to", "1300", "--era", "600-1300:1", "--out-dir", path(&dir)];
    let run = python(&crate_file("tools/certify_plan.py"), &[&args[..], &range[..]].concat());
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);

    let plan = json_lines(&dir.join("plan.jsonl"));
    let boards: Vec<u32> = plan.iter().map(|game| board_of(game).expect("n")).collect();
    assert_eq!(boards, (600..=1300).collect::<Vec<u32>>(), "the plan has a line for each board, in order");
    let lines = json_lines(&crate_file("bench/certify/certificates_600_1300.jsonl"));
    let certified: BTreeMap<u32, &Json> = lines.iter().map(|line| (board_of(line).expect("n"), line)).collect();
    assert_eq!(certified.len(), lines.len(), "a board certified twice");

    let needing: Vec<&Json> = plan.iter().filter(|game| field(game, "needs").as_u32() == Some(1)).collect();
    let planned: Vec<u32> = needing.iter().map(|game| board_of(game).expect("n")).collect();
    assert_eq!(planned, certified.keys().copied().collect::<Vec<u32>>(), "the games that need a certificate");
    assert_eq!(planned.len(), 377);

    let mut bases: BTreeMap<&str, usize> = BTreeMap::new();
    for game in needing {
        let n = board_of(game).expect("n");
        let line = certified[&n];
        let members = field(line, "open").as_arr().expect("the open members").len() as u64;
        assert_eq!(field(game, "open").as_u64(), Some(members), "n={n}: |O|");
        assert_eq!(field(game, "rejected").as_u64(), field(line, "rejected").as_u64(), "n={n}: |R|");
        let rule = line.get("same_problem").map(|settled| field(settled, "n").as_u32().expect("the game it rests on"));
        assert_eq!(field(game, "share").as_u32(), rule, "n={n}: the game whose certificate settles it");
        let basis = field(game, "basis").as_str().expect("the basis");
        *bases.entry(basis).or_default() += 1;
        assert_eq!(basis == "shared", rule.is_some(), "n={n}: basis {basis}, settled by {rule:?}");
        let hours = field(game, "expected_thread_hours").as_f64();
        assert!(basis == "shared" || basis == "unknown" || hours.is_some_and(|h| h > 0.0), "n={n}: {basis}, {hours:?}");
    }
    let settled = lines.iter().filter(|line| line.get("same_problem").is_some()).count();
    assert_eq!(bases.get("shared").copied().unwrap_or(0), settled);
    for game in plan.iter().filter(|game| field(game, "needs").as_u32() != Some(1)) {
        let hours = field(game, "expected_thread_hours").as_f64().unwrap_or(0.0);
        assert!(hours <= 0.0, "n={:?}: needs no certificate, but costs {hours}", board_of(game));
    }
    println!(
        "work list 600..=1300: {} games, {settled} shared, {} searched; by basis {bases:?}",
        lines.len(),
        lines.len() - settled
    );
}

/// The committed certificates of 600..=1300 verify against the chain and every committed file of barred bounds: all
/// 377 lines are final, the searched and the same-problem ones as the file counts them, none beaten, no
/// counterexample; and the proven end of the chain is still 2175 (they are of games the prover proved already), the
/// first game that is not proven being 2176.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(the_first_committed_certificates_verify))]
fn the_committed_certificates_verify() {
    the_committed_certificates_verify_first(377);
}

/// [`the_committed_certificates_verify`] for the first 40 lines of the file.
#[test]
fn the_first_committed_certificates_verify() {
    the_committed_certificates_verify_first(40);
}

/// The first `count` lines of the committed certificates (a file that `taxman certify` could have written itself)
/// verify, with the counts of those lines.
fn the_committed_certificates_verify_first(count: usize) {
    let committed = crate_file("bench/certify/certificates_600_1300.jsonl");
    let mut lines = json_lines(&committed);
    assert_eq!(lines.len(), 377);
    lines.truncate(count);
    let dir = TempDir::new("verify-committed");
    let certs = if count == 377 { committed } else { write_lines(&dir, "first.jsonl", &lines) };
    let settled = lines.iter().filter(|line| line.get("same_problem").is_some()).count();
    let (total, searched) = (lines.len(), lines.len() - settled);

    let run = run_verify(&[&certs], &results_path("strict.jsonl"), &committed_upper_files(), &["--threads", "2"]);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    assert_eq!(run.stderr, "");
    let out = &run.stdout;
    let counts = format!(
        "{total} lines, {total} final, 0 incomplete; certified by search {searched}, certified by the same problem \
         {settled}, beaten 0"
    );
    assert!(out.contains(&format!("{}: {counts}", certs.display())), "{out}");
    assert!(out.contains(&format!("boards certified: {total} (")), "{out}");
    assert!(out.contains("counterexamples: none"), "{out}");
    let end = "proven frontier: every game to 2175 is proven; the first that is not is 2176 (needs a certificate and \
               has none in the files given)";
    assert!(out.contains(end), "{out}");
}

// ---------------------------------------------------------------------------------------------------------
// The campaign's lists and tools

/// The campaign's own tools pass their tests (`tools/test_certify_campaign.py`, `tools/test_certify_pilot.py`, standard
/// library only): the lists of the phases, and the pilot's stop rule on synthetic lines, on real ones and on the
/// committed plan.
#[test]
fn the_campaign_tools_pass_their_own_tests() {
    for script in ["tools/test_certify_campaign.py", "tools/test_certify_pilot.py"] {
        let run = python(&crate_file(script), &[]);
        assert_eq!(run.code, Some(0), "{script}: {}", run.stderr);
    }
}

/// The lists of the campaign (`bench/certify/lists/stage4`) hold every game that needs a certificate once, a game that
/// rests on another's certificate after it in the same list, and the repeats the rule cannot settle in their own list
/// (`tools/certify_campaign.py check`); and `taxman certify --list` reads each of them (it reads the list before it
/// asks for its other flags, so the usage error about `--results` is the sign that the list was accepted).
#[test]
fn the_campaign_lists_cover_every_game_once_and_certify_reads_them() {
    let lists = crate_file("bench/certify/lists/stage4");
    let plan = crate_file("bench/certify/plan.jsonl");
    let args = ["check", "--plan", path(&plan), "--lists-dir", path(&lists)];
    let run = python(&crate_file("tools/certify_campaign.py"), &args);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    assert!(run.stdout.starts_with("7 lists, 708 games: "), "{}", run.stdout);

    let mut files: Vec<PathBuf> = fs::read_dir(&lists).unwrap().map(|entry| entry.unwrap().path()).collect();
    files.sort();
    assert_eq!(files.len(), 7);
    for file in files {
        let run = taxman(&["certify", "--list", path(&file)]);
        let accepted = run.stderr.contains("certify needs --results FILE and --upper FILE");
        assert!(run.code == Some(2) && accepted, "{}: {}", file.display(), run.stderr);
    }
}

/// The real certificates of 1301..=2175 that the pilot tool's tests read (`bench/certify/certificates_sample_1301_2175
/// .jsonl`, made by `taxman certify` on the laptop) verify: eight searches, three games settled by the rule, and one
/// search that was stopped by a limit and resumed.
#[test]
fn the_sample_certificates_of_the_pilot_tests_verify() {
    let certs = crate_file("bench/certify/certificates_sample_1301_2175.jsonl");
    let run = run_verify(&[&certs], &results_path("strict.jsonl"), &committed_upper_files(), &["--threads", "2"]);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
    let counts = "12 lines, 11 final, 1 incomplete; certified by search 8, certified by the same problem 3, beaten 0";
    assert!(run.stdout.contains(&format!("{}: {counts}", certs.display())), "{}", run.stdout);
    assert!(run.stdout.contains("counterexamples: none"), "{}", run.stdout);
}
