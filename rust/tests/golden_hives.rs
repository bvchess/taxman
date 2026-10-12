//! Checks the hive, frame and acceleration code against the Java solver's results (`testdata/golden`).
//!
//! The golden files hold, for each board size, what the Java produced for the stages these modules
//! reimplement.  Every comparison is exact, including list orders.  Mismatches are collected over all the
//! files and reported together, so one bad board does not hide the rest.

use std::fmt::Debug;
use std::path::PathBuf;
use std::time::Instant;
use taxman::accelerate::{prime_shortcut, reuse_prev, solution_for_promotions};
use taxman::bits::Bits;
use taxman::frames::build_frames;
use taxman::game::{replay, Optimal};
use taxman::hives::Apiary;
use taxman::json::Json;
use taxman::numtheory::Tables;

/// Wide enough for every golden board (n <= 1023).
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

fn load_golden() -> Vec<Golden> {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/golden"));
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    let golden: Vec<Golden> = paths
        .iter()
        .map(|path| {
            let json = Json::from_file(path).unwrap();
            let n = json.get("n").and_then(Json::as_u32).expect("n");
            assert!(json.get("error").is_none(), "n={n}: the Java export reported {:?}", json.get("error"));
            Golden { n, json }
        })
        .collect();
    assert!(golden.len() >= 300, "only {} golden files in {}", golden.len(), dir.display());
    golden
}

fn ints(j: &Json) -> Vec<u32> {
    j.as_u32_vec().unwrap_or_else(|| panic!("not an int array: {j:?}"))
}

fn set(j: &Json) -> Bits<W> {
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

#[test]
fn apiary_matches_java() {
    let mut bad = Mismatches::default();
    let golden = load_golden();
    for g in &golden {
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
        bad.check(
            n,
            "promotionCandidates",
            apiary.promotion_candidates(),
            set(want.get("promotionCandidates").unwrap()),
        );
    }
    bad.assert_none(golden.len());
}

#[test]
fn frames_match_java() {
    let mut bad = Mismatches::default();
    let golden = load_golden();
    let mut total_frames = 0;
    for g in &golden {
        let n = g.n;
        let cols = build_frames::<W>(&Tables::new(n));
        let want_frames = g.field("frames").as_arr().unwrap();
        total_frames += want_frames.len();
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
    }
    assert!(total_frames > 500, "suspiciously few frames: {total_frames}");
    bad.assert_none(golden.len());
}

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

/// The setup for one game of n = 1000 must be cheap next to the search it precedes.
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
    assert!(best < 0.5, "setup took {best} s");
}
