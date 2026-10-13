//! Checks the hive, frame and acceleration code against the Java solver's results (`testdata/golden`,
//! `testdata/golden_wide`, `testdata/accel_java.txt`, `testdata/games_java.txt`).
//!
//! The golden files hold, for each board size, what the Java produced for the stages these modules
//! reimplement.  Every comparison is exact, including list orders.  Mismatches are collected over all the
//! files and reported together, so one bad board does not hide the rest.
//!
//! `testdata/golden` (n <= 760, and so `W <= 16`) is what the Java solved with its own assertions on.  Beyond its
//! limit of 1023 the Java was run without them (`java` without `-ea`: its limit is an `assert`, and nothing else
//! depends on it); `testdata/golden_wide` holds the structure of the boards at the edges of the set widths and
//! beyond, checked at the width `with_width!` picks for them, which no file of `testdata/golden` reaches.

mod common;

use common::{fnv1a64, stored_games, testdata};
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::time::Instant;
use taxman::accelerate::{prime_shortcut, reuse_prev, solution_for_promotions};
use taxman::bits::Bits;
use taxman::bounds::greedy;
use taxman::frames::build_frames;
use taxman::game::{replay, Optimal};
use taxman::hives::Apiary;
use taxman::json::Json;
use taxman::model::Model;
use taxman::numtheory::Tables;
use taxman::with_width;

/// Wide enough for every file of `testdata/golden` (n <= 1023).
const W: usize = 16;

struct Golden {
    n: u32,
    json: Json,
}

impl Golden {
    fn field(&self, name: &str) -> &Json {
        self.json.get(name).unwrap_or_else(|| panic!("n={}: no field {name}", self.n))
    }
}

/// Every `n*.json` under `testdata/<dir>`, by n.
fn load_dir(dir: &str) -> Vec<Golden> {
    let dir = testdata(dir);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|path| {
            let json = Json::from_file(path).unwrap();
            let n = json.get("n").and_then(Json::as_u32).expect("n");
            assert!(json.get("error").is_none(), "n={n}: the Java export reported {:?}", json.get("error"));
            Golden { n, json }
        })
        .collect()
}

fn load_golden() -> Vec<Golden> {
    let golden = load_dir("golden");
    assert!(golden.len() >= 300, "only {} golden files", golden.len());
    golden
}

/// The boards at the edges of the set widths and beyond: the last and first n of `W = 8` / 16 (511, 512), 16 / 20
/// (1023, 1024) and 32 / 64 (2047, 2048), the Java's former limit and the games past it.
const WIDE: [u32; 11] = [511, 512, 1000, 1001, 1023, 1024, 1025, 1100, 1231, 2047, 2048];

fn load_wide() -> Vec<Golden> {
    let golden = load_dir("golden_wide");
    assert_eq!(golden.iter().map(|g| g.n).collect::<Vec<_>>(), WIDE, "the files of testdata/golden_wide");
    golden
}

fn ints(j: &Json) -> Vec<u32> {
    j.as_u32_vec().unwrap_or_else(|| panic!("not an int array: {j:?}"))
}

fn set<const W: usize>(j: &Json) -> Bits<W> {
    ints(j).into_iter().collect()
}

/// Collects mismatches; each is one line naming the board and the quantity.
#[derive(Default)]
struct Mismatches(Vec<String>);

impl Mismatches {
    fn check<T: PartialEq + Debug>(&mut self, n: u32, what: &str, got: T, want: T) {
        if got != want {
            self.0.push(format!("n={n} {what}: got {got:?}, want {want:?}"));
        }
    }

    fn assert_none(self, checked: usize) {
        if !self.0.is_empty() {
            let shown: Vec<&str> = self.0.iter().take(15).map(String::as_str).collect();
            panic!("{} mismatches in {checked} checks; first ones:\n{}", self.0.len(), shown.join("\n"));
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// The structure of a board, at any width

fn check_apiary<const W: usize>(g: &Golden, bad: &mut Mismatches) {
    let n = g.n;
    let t = Tables::new(n);
    let apiary = Apiary::<W>::new(&t, Bits::EMPTY);
    let want = g.field("apiary");
    let want_hives = want.get("hives").and_then(Json::as_arr).unwrap();
    bad.check(n, "hive count", apiary.hives().len(), want_hives.len());
    for (i, (got, want)) in apiary.hives().iter().zip(want_hives).enumerate() {
        let field = |name: &str| want.get(name).unwrap_or_else(|| panic!("n={n} hive {i}: no {name}"));
        let at = |what: &str| format!("hive {i} {what}");
        bad.check(n, &at("name"), got.name(), field("name").as_str().unwrap());
        bad.check(n, &at("sources"), got.sources(), set(field("sources")));
        bad.check(n, &at("factors"), got.factors(), set(field("factors")));
        bad.check(n, &at("remainingSources"), got.remaining_sources(), set(field("remainingSources")));
        bad.check(n, &at("remainingFactors"), got.remaining_factors(), set(field("remainingFactors")));
        bad.check(n, &at("impossible"), got.impossible(), set(field("impossible")));
        bad.check(
            n,
            &at("freeFactorCount"),
            i64::from(got.free_factor_count()),
            field("freeFactorCount").as_i64().unwrap(),
        );
        bad.check(
            n,
            &at("promotionCandidates"),
            apiary.hive_promotion_candidates(i).to_vec(),
            ints(field("promotionCandidates")),
        );
        bad.check(n, &at("solution"), apiary.hive_solution(i), ints(field("solution")));
    }
    bad.check(n, "solution", apiary.solution(), ints(want.get("solution").unwrap()));
    bad.check(n, "impossible", apiary.impossible(), set(want.get("impossible").unwrap()));
    bad.check(n, "promotionCandidates", apiary.promotion_candidates(), set(want.get("promotionCandidates").unwrap()));
}

/// The frames of the frame builder; returns how many there were.
fn check_frames<const W: usize>(g: &Golden, bad: &mut Mismatches) -> usize {
    let n = g.n;
    let cols = build_frames::<W>(&Tables::new(n));
    let want_frames = g.field("frames").as_arr().unwrap();
    bad.check(n, "frame count", cols.frames.len(), want_frames.len());
    for (i, (got, want)) in cols.frames.iter().zip(want_frames).enumerate() {
        let field = |name: &str| want.get(name).unwrap_or_else(|| panic!("n={n} frame {i}: no {name}"));
        let at = |what: &str| format!("frame {i} {what}");
        bad.check(n, &at("name"), got.name.as_str(), field("name").as_str().unwrap());
        bad.check(n, &at("factors"), got.factors, set(field("remainingFactors")));
        bad.check(n, &at("moves"), got.moves, set(field("remainingSources")));
        bad.check(
            n,
            &at("freeFactorCount"),
            i64::from(got.free_factor_count),
            field("freeFactorCount").as_i64().unwrap(),
        );
    }
    bad.check(n, "candidates", cols.candidates, set(g.field("allCandidates")));
    bad.check(
        n,
        "estimateMaxPromotions",
        u64::from(cols.estimate_max_promotions),
        g.field("estimateMaxPromotions").as_u64().unwrap(),
    );
    want_frames.len()
}

/// The reduced frame set the search starts from, its unrevised promotion cap and the greedy incumbent, from the
/// Java's own columns (the `frames` of the file) and from the frame builder's.
fn check_reduced<const W: usize>(g: &Golden, bad: &mut Mismatches) {
    let n = g.n;
    let tables = Tables::new(n);
    let java_columns: Vec<Vec<u32>> = g
        .field("frames")
        .as_arr()
        .unwrap()
        .iter()
        .flat_map(|f| [ints(f.get("remainingFactors").unwrap()), ints(f.get("remainingSources").unwrap())])
        .collect();
    let built = build_frames::<W>(&tables);
    bad.check(n, "the frame builder's columns", built.columns(), java_columns.clone());
    let candidates: Bits<W> = set(g.field("allCandidates"));
    let (model, root) = Model::build::<W>(&tables, &java_columns).unwrap();
    let want = g.field("frameSet").as_arr().unwrap();
    bad.check(n, "reduced frame count", root.len(), want.len());
    for (i, (frame, want)) in root.frames().iter().zip(want).enumerate() {
        bad.check(n, &format!("reduced frame {i} moves"), frame.moves.to_vec(), ints(want.get("moves").unwrap()));
        bad.check(n, &format!("reduced frame {i} factors"), frame.factors.to_vec(), ints(want.get("factors").unwrap()));
        bad.check(
            n,
            &format!("reduced frame {i} freedoms"),
            Some(i64::from(frame.freedoms)),
            want.get("freedoms").and_then(Json::as_i64),
        );
    }
    bad.check(
        n,
        "maxPromotionsUnrevised",
        Some(i64::from(root.max_promotions_among(&candidates))),
        g.field("maxPromotionsUnrevised").as_i64(),
    );
    bad.check(n, "greedy incumbent", greedy(&model, &root, &candidates).to_vec(), ints(g.field("greedy")));
    // The root of the search never narrows its candidates, so each must be a factor of a frame above the first:
    // `FrameMapper.homeFrame` is never the default 0.
    for x in candidates.iter() {
        if !model.contains_factor(&root, x) || model.home(x) == 0 {
            bad.0.push(format!("n={n}: candidate {x} is not a factor of a frame above the first"));
        }
    }
}

#[test]
fn apiary_matches_java() {
    let mut bad = Mismatches::default();
    let golden = load_golden();
    for g in &golden {
        check_apiary::<W>(g, &mut bad);
    }
    bad.assert_none(golden.len());
}

#[test]
fn frames_match_java() {
    let mut bad = Mismatches::default();
    let golden = load_golden();
    let total_frames: usize = golden.iter().map(|g| check_frames::<W>(g, &mut bad)).sum();
    assert!(total_frames > 500, "suspiciously few frames: {total_frames}");
    bad.assert_none(golden.len());
}

/// The Java's structure for the boards at the edges of the set widths and beyond its former limit of 1023, at the
/// width the board dispatches to: no other test checks `W` of 20 or more against anything the Java made.
#[test]
fn wide_boards_match_the_java() {
    let mut bad = Mismatches::default();
    let golden = load_wide();
    let widths: Vec<usize> = golden.iter().map(|g| with_width!(g.n, W => W)).collect();
    assert_eq!(widths, [8, 16, 16, 16, 16, 20, 20, 20, 20, 32, 64], "the set width each board dispatches to");
    for g in &golden {
        with_width!(g.n, W => {
            check_apiary::<W>(g, &mut bad);
            check_frames::<W>(g, &mut bad);
            check_reduced::<W>(g, &mut bad);
        });
    }
    bad.assert_none(golden.len());
}

// ---------------------------------------------------------------------------------------------------------
// Where the previous game gives the answer

/// `FrameSolver.solveBasedOnPrev`: a prime takes the shortcut, anything else tries `reuse_prev`.
#[test]
fn acceleration_matches_java() {
    let optimal = Optimal::load_default().unwrap();
    let mut bad = Mismatches::default();
    let golden = load_golden();
    let (mut primes, mut reused, mut searched) = (0, 0, 0);
    for g in &golden {
        let n = g.n;
        let t = Tables::new(n);
        let prev: &[u32] = if n == 1 { &[] } else { optimal.moves(n - 1).expect("optimal.json has n-1") };
        let solve = g.field("solve");
        let java_accelerated = solve.get("accelerated").and_then(Json::as_bool).unwrap();
        let java_moves = ints(solve.get("moves").unwrap());

        let (moves, promotions) = if t.is_prime(n) {
            primes += 1;
            (Some(prime_shortcut(n, prev)), None)
        } else {
            let found = reuse_prev(&t, &build_frames::<W>(&t), prev);
            (found.as_ref().map(|a| a.moves.clone()), found.map(|a| a.promotions))
        };
        bad.check(n, "accelerated", moves.is_some(), java_accelerated);
        if let Some(moves) = moves {
            bad.check(n, "accelerated moves", moves, java_moves);
            if let Some(promotions) = promotions {
                reused += 1;
                bad.check(n, "accelerated promotions", promotions, ints(solve.get("promotions").unwrap()));
            }
        } else {
            searched += 1;
        }
    }
    println!("{primes} primes, {reused} reused, {searched} need a search");
    assert!(primes > 50 && reused > 50 && searched > 50);
    bad.assert_none(golden.len());
}

/// Every how-many-th line of the two Java fixtures below the tests check.  All of them take 3.4 and 3.2 s (building
/// the frames or the hives of 94 and 171 boards of 760 to 2049); half of them have the same power, as the lines are
/// samples of the same few kinds.  Set to 1 to check them all.
const FIXTURE_STRIDE: usize = 2;

/// The data lines of a fixture of `testdata`: the lines that are not comments, split at `separator`.
fn fixture_lines(name: &str, separator: &str) -> Vec<Vec<String>> {
    let text = std::fs::read_to_string(testdata(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    let rows = text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty());
    rows.map(|l| l.split(separator).map(|field| field.trim().to_owned()).collect()).collect()
}

/// The same decision for the boards past the golden data (761..1228, every fifth): `testdata/accel_java.txt` holds
/// what the Java's acceleration did with the previous game (optimal.json's to n = 1000, `results/default.jsonl`'s
/// above, as fingerprinted in the file).  A game it accepts must be the same game, to the move.
#[test]
fn acceleration_matches_the_java_beyond_the_golden_data() {
    let optimal = Optimal::load_default().unwrap();
    let chain = stored_games("default.jsonl");
    let rows = fixture_lines("accel_java.txt", " ");
    assert!(rows.len() >= 90, "{} lines", rows.len());
    let (mut accepted, mut rejected, mut primes) = (0, 0, 0);
    for row in rows.iter().step_by(FIXTURE_STRIDE) {
        let n: u32 = row[0].parse().unwrap();
        let hex = |text: &str| u64::from_str_radix(text, 16).unwrap();
        let (accel, sum, fingerprint, prev_fingerprint) =
            (row[1] == "accel", row[2].parse::<u64>().unwrap(), hex(&row[3]), hex(&row[4]));
        let prev: Vec<u32> =
            if n - 1 <= 1000 { optimal.moves(n - 1).unwrap().to_vec() } else { chain[&(n - 1)].1.clone() };
        assert_eq!(
            fnv1a64(&prev),
            prev_fingerprint,
            "n={n}: the game {} is not the one the Java's decision was made with; regenerate testdata/accel_java.txt",
            n - 1
        );
        let t = Tables::new(n);
        let moves = if t.is_prime(n) {
            primes += 1;
            Some(prime_shortcut(n, &prev))
        } else {
            with_width!(n, W => reuse_prev(&t, &build_frames::<W>(&t), &prev)).map(|a| a.moves)
        };
        assert_eq!(moves.is_some(), accel, "n={n}: the Java {}, the Rust {}", row[1], moves.is_some());
        match moves {
            Some(moves) => {
                accepted += 1;
                assert_eq!(
                    (moves.iter().map(|&m| u64::from(m)).sum::<u64>(), fnv1a64(&moves)),
                    (sum, fingerprint),
                    "n={n}"
                );
            }
            None => rejected += 1,
        }
    }
    println!("{accepted} accepted ({primes} primes), {rejected} rejected");
    assert!(accepted * FIXTURE_STRIDE > 50 && rejected * FIXTURE_STRIDE > 10 && primes * FIXTURE_STRIDE > 5);
}

// ---------------------------------------------------------------------------------------------------------
// The game made of a set of promotions

/// The final step of every solve: the promotions found become a game.
#[test]
fn promotions_become_the_java_game() {
    let mut bad = Mismatches::default();
    let golden = load_golden();
    let mut checked = 0;
    for g in &golden {
        let n = g.n;
        let t = Tables::new(n);
        let solve = g.field("solve");
        let java_accelerated = solve.get("accelerated").and_then(Json::as_bool).unwrap();
        // A prime's game is its predecessor's with one move changed; its promotions are the predecessor's.
        if java_accelerated && t.is_prime(n) {
            continue;
        }
        checked += 1;
        let promotions = set(solve.get("promotions").unwrap());
        let moves = solution_for_promotions::<W>(&t, promotions);
        bad.check(n, "moves", moves.clone(), ints(solve.get("moves").unwrap()));
        match replay(n, &moves) {
            Ok(score) => bad.check(n, "replayed score", score, solve.get("score").and_then(Json::as_u64).unwrap()),
            Err(e) => bad.0.push(format!("n={n}: the game is illegal: {e}")),
        }
    }
    assert!(checked > 200);
    bad.assert_none(checked);
}

/// The Java's apiary game for sets of promotions that are not just the optimal ones (halves and random subsets of
/// them, and sets at the edges of the set widths), for boards up to 2049: the same moves in the same order, which
/// then score what the game with no promotions scores plus the promotions (the identity the search's incumbent
/// value stands on).
#[test]
fn promotion_sets_become_the_java_game_beyond_the_golden_data() {
    let rows = fixture_lines("games_java.txt", "|");
    assert!(rows.len() >= 170, "{} lines", rows.len());
    let mut tables: BTreeMap<u32, Tables> = BTreeMap::new();
    let mut in_the_bag: BTreeMap<u32, u64> = BTreeMap::new();
    for row in rows.iter().step_by(FIXTURE_STRIDE) {
        let n: u32 = row[0].parse().unwrap();
        let promotions: Vec<u32> = row[1].split_whitespace().map(|x| x.parse().unwrap()).collect();
        let (sum, fingerprint) = (row[2].parse::<u64>().unwrap(), u64::from_str_radix(&row[3], 16).unwrap());
        let t = tables.entry(n).or_insert_with(|| Tables::new(n));
        let moves = with_width!(n, W => solution_for_promotions::<W>(t, Bits::from_slice(&promotions)));
        let what = format!("n={n}, {} promotions", promotions.len());
        assert_eq!((moves.iter().map(|&m| u64::from(m)).sum::<u64>(), fnv1a64(&moves)), (sum, fingerprint), "{what}");
        let bag = *in_the_bag.entry(n).or_insert_with(|| {
            let moves = with_width!(n, W => solution_for_promotions::<W>(t, Bits::EMPTY));
            replay(n, &moves).expect("the game with no promotions")
        });
        let promoted: u64 = promotions.iter().map(|&p| u64::from(p)).sum();
        assert_eq!(replay(n, &moves), Ok(bag + promoted), "{what}: score = inTheBag + promotions");
    }
}

/// Boards beyond the Java's 1023 limit need wider sets; check that nothing breaks and the shape is right.
#[test]
fn wide_boards_build() {
    for n in [1500u32, 2000] {
        let t = Tables::new(n);
        let cols = build_frames::<32>(&t);
        assert_eq!(cols.frames.len(), 7, "n={n}");
        for (i, frame) in cols.frames.iter().enumerate() {
            assert!(frame.free_factor_count > 0, "n={n} frame {i}");
            assert!(frame.factors.len() as i32 >= frame.free_factor_count, "n={n} frame {i}");
            assert!(!frame.moves.is_empty(), "n={n} frame {i}");
        }
        assert!(cols.estimate_max_promotions > 0 && cols.candidates.len() > cols.estimate_max_promotions);
        // The frames are disjoint columns of numbers.
        let all: Bits<32> = cols.frames.iter().fold(Bits::EMPTY, |acc, f| acc | f.factors | f.moves);
        let counted: u32 = cols.frames.iter().map(|f| f.factors.len() + f.moves.len()).sum();
        assert_eq!(all.len(), counted, "n={n}");
        let moves = Apiary::<32>::new(&t, Bits::EMPTY).solution();
        assert!(replay(n, &moves).is_ok(), "n={n}");
    }
}

/// The setup for one game of n = 1000 must be cheap next to the search it precedes.  Wall time depends on the
/// machine and on the optimizer, so the 25-fold margin over the 20 ms of an optimized build is asserted only where
/// the build is optimized (without debug assertions, as `cargo test --release`); the test profile still runs it.
#[test]
fn setup_for_1000_is_fast() {
    let optimal = Optimal::load_default().unwrap();
    let t = Tables::new(1000);
    let prev = optimal.moves(999).unwrap();
    let mut best = f64::MAX;
    for _ in 0..5 {
        let start = Instant::now();
        let frames = build_frames::<W>(&t);
        let built = start.elapsed();
        let reused = reuse_prev(&t, &frames, prev);
        let total = start.elapsed();
        println!(
            "n=1000: build_frames {:.2} ms, reuse_prev {:.2} ms ({})",
            built.as_secs_f64() * 1e3,
            (total - built).as_secs_f64() * 1e3,
            if reused.is_some() { "accepted" } else { "rejected" }
        );
        best = best.min(total.as_secs_f64());
    }
    if !cfg!(debug_assertions) {
        assert!(best < 0.5, "setup took {best} s");
    }
}
