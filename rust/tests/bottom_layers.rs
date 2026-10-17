//! Theorem B, the bottom layers of an optimal set, against every recorded and enumerated optimal set.
//!
//! The theorem (heuristics/THEORY.md, "Theorem B"):
//!
//! > The *layer* of a number x is Ω(x), its number of prime factors counted with multiplicity.  Let q be the largest
//! > prime ≤ n, and U\*_2 the numbers of layer 2 above n/2 that setup keeps (descending greedy; on layer 2 a set is
//! > playable exactly when the graph on the primes up to n/2 plus a ground vertex, with the edge p–q for pq and
//! > p–ground for p², is a forest, so the greedy is Kruskal's).  Every optimal set S of n ≥ 2 holds exactly one number
//! > of layer 1, q; its numbers of layer 2 above n/2 are exactly U\*_2; and its numbers of layer 2 up to n/2, as edges,
//! > keep U\*_2 a forest.  So where the frames of layer 2 are closed (U\*_2 spans every prime up to n/2: |U\*_2| =
//! > π(n/2)) S plays no number of layer 2 up to n/2.
//!
//! A theorem is not a measurement: one optimal set that breaks it would mean an error in its proof or in the data, and
//! the test says which statement fails and where.  So the theorem is checked against every optimal set there is
//! evidence for.  U\*_2 is rebuilt here by Kruskal's algorithm, from the definition, with no code of the frame solver
//! (the first test compares it with the numbers that setup keeps), and playability is decided by `common::playable`,
//! the peel of THEORY.md, which shares no code with `frames` or `model` either.
//!
//! * [`the_check_says_which_statement_fails`]: the check of the theorem accepts what it allows and fails each of its
//!   statements in turn on small sets made to break it.
//! * [`layer_2_greedy_is_what_setup_keeps`]: the numbers of layer 2 above n/2 in the game with no promotions
//!   (`Board::plain`) are Kruskal's, on every board to 400 and on a sample of the boards to 8191 (an ignored test
//!   takes every 25th).
//! * [`theorem_b_holds_for_every_recorded_game`]: the games of the table `optimal.json`, of the committed chain
//!   `results/strict.jsonl` (2..3440) and of the chain 2176 to 2276 of the Graviton calibration, another run of games
//!   the committed chain has.  The frame solver's games play numbers of layer 2 up to n/2 only on small boards, where
//!   the frames of layer 2 have freedoms.
//! * [`theorem_b_holds_for_every_enumerated_optimal_set`]: the prover's lists of all the optimal sets of a board
//!   (`prover/data/optsets_*.jsonl`): the other sets of a board as well as the recorded game.  Each set is checked to
//!   be playable and worth the optimum before the theorem is asked of it.
//! * [`theorem_b_holds_for_every_optimal_set_of_the_small_boards`]: on the small boards the optimal sets are found
//!   again, by an exhaustive search over the playable sets that uses no model of the game, only the oracle and the
//!   optimum of the table.  The brute force of the rules has the optimum itself on the boards it reaches, and the
//!   number of optimal sets is the prover's.

#[macro_use]
mod common;

use common::{playable, results_path};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::time::Instant;
use taxman::board::Board;
use taxman::brute::{self, Rule};
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::numtheory::Tables;
use taxman::with_width;

// ---------------------------------------------------------------------------------------------------------
// The theorem

/// The layer of `x`: Ω(x), its number of prime factors counted with multiplicity.
fn layer(t: &Tables, mut x: u32) -> u32 {
    let mut count = 0;
    while x > 1 {
        x /= t.spf[x as usize];
        count += 1;
    }
    count
}

/// Whether `x` is above n/2.
fn above_half(t: &Tables, x: u32) -> bool {
    2 * x > t.n
}

/// The ends of the edge of `x`, a number of layer 2: p–q for x = pq, and p–ground (vertex 0) for x = p².
fn edge(t: &Tables, x: u32) -> (usize, usize) {
    let p = t.spf[x as usize];
    let q = x / p;
    (p as usize, if p == q { 0 } else { q as usize })
}

/// The graph of layer 2 as a union-find over the ground (vertex 0) and the numbers up to n/2, among them the primes,
/// which are its vertices.  An edge that joins two components keeps the graph a forest; one that does not closes a
/// cycle.
struct Forest {
    parent: Vec<usize>,
}

impl Forest {
    fn new(t: &Tables) -> Self {
        Forest { parent: (0..=t.n as usize / 2).collect() }
    }

    fn root(&mut self, mut v: usize) -> usize {
        while self.parent[v] != v {
            self.parent[v] = self.parent[self.parent[v]];
            v = self.parent[v];
        }
        v
    }

    /// Adds the edge of `x` if that keeps the graph a forest, and says whether it did.
    fn add(&mut self, t: &Tables, x: u32) -> bool {
        let (a, b) = edge(t, x);
        let (a, b) = (self.root(a), self.root(b));
        if a != b {
            self.parent[a] = b;
        }
        a != b
    }
}

/// U\*_2 by Kruskal's algorithm, in decreasing order: the numbers `x` above n/2 with Ω(x) = 2, taken from the largest,
/// each kept if its edge joins two components of the graph of the ones kept so far.
fn layer_2_greedy(t: &Tables) -> Vec<u32> {
    let mut forest = Forest::new(t);
    let above = (t.n / 2 + 1..=t.n).rev();
    above.filter(|&x| layer(t, x) == 2).filter(|&x| forest.add(t, x)).collect()
}

/// The numbers of layer 2 up to n/2 in `set`, ascending.
fn lower_layer_2(t: &Tables, set: &[u32]) -> Vec<u32> {
    let mut lower: Vec<u32> = set.iter().copied().filter(|&x| !above_half(t, x) && layer(t, x) == 2).collect();
    lower.sort_unstable();
    lower
}

/// What is in `from` and not in `other` (both ascending).
fn missing_from(from: &[u32], other: &[u32]) -> Vec<u32> {
    from.iter().copied().filter(|x| other.binary_search(x).is_err()).collect()
}

/// Theorem B on `set`, a set of numbers of the board of `t` (n ≥ 2): the three statements and the closed case.  Returns
/// how many numbers of layer 2 up to n/2 the set holds (none where the frames of layer 2 are closed), or says which
/// statement fails and with what numbers.
fn check_theorem_b(t: &Tables, set: &[u32]) -> Result<usize, String> {
    let n = t.n;
    assert!(n >= 2, "Theorem B is about the boards from 2");
    let mut seen = BTreeSet::new();
    if let Some(x) = set.iter().find(|&&x| !(2..=n).contains(&x) || !seen.insert(x)) {
        return Err(format!("the set is no set of numbers of 2..={n}: {x} is off the board or in it twice"));
    }

    // Statement 1: the one number of layer 1 is q, the largest prime.
    let q = *t.primes().last().expect("a prime up to n >= 2");
    let layer_1: Vec<u32> = set.iter().copied().filter(|&x| layer(t, x) == 1).collect();
    if layer_1 != [q] {
        return Err(format!("statement 1: the numbers of layer 1 are {layer_1:?}, not just q = {q}"));
    }

    // Statement 2: the numbers of layer 2 above n/2 are U*_2.
    let mut upper = layer_2_greedy(t);
    upper.sort_unstable();
    let mut held: Vec<u32> = set.iter().copied().filter(|&x| above_half(t, x) && layer(t, x) == 2).collect();
    held.sort_unstable();
    if held != upper {
        return Err(format!(
            "statement 2: the numbers of layer 2 above n/2 are not U*_2: it lacks {:?} and has {:?} besides",
            missing_from(&upper, &held),
            missing_from(&held, &upper)
        ));
    }

    // Statement 3: the numbers of layer 2 up to n/2, as edges, keep U*_2 a forest.
    let mut forest = Forest::new(t);
    for &x in &upper {
        assert!(forest.add(t, x), "n={n}: U*_2 is no forest: {x}");
    }
    let lower = lower_layer_2(t, set);
    for (i, &x) in lower.iter().enumerate() {
        if !forest.add(t, x) {
            let (a, b) = edge(t, x);
            return Err(format!(
                "statement 3: {x}, the edge {a}-{b}, closes a cycle with U*_2 and {:?}, the smaller numbers it plays",
                &lower[..i]
            ));
        }
    }

    // The closed case: U*_2 spans every prime up to n/2, and the set plays no number of layer 2 up to n/2.
    let primes_up_to_half = t.primes().iter().filter(|&&p| !above_half(t, p)).count();
    if upper.len() == primes_up_to_half && !lower.is_empty() {
        return Err(format!(
            "closed case: U*_2 spans the {primes_up_to_half} primes up to n/2, yet the set plays {lower:?} up to n/2"
        ));
    }
    Ok(lower.len())
}

/// Theorem B on `set`; the numbers of layer 2 up to n/2 that it plays, if any, are noted in `plays_lower` as the board
/// size and the numbers.
fn check_and_note(t: &Tables, set: &[u32], plays_lower: &mut Vec<(u32, Vec<u32>)>) -> Result<(), String> {
    if check_theorem_b(t, set)? > 0 {
        plays_lower.push((t.n, lower_layer_2(t, set)));
    }
    Ok(())
}

/// `plays_lower` (ascending in the board size) as text, with a run of boards in a row that play the same numbers
/// written once: `n=42..=53 [21]`.
fn runs(plays_lower: &[(u32, Vec<u32>)]) -> String {
    let mut runs: Vec<(u32, u32, &[u32])> = Vec::new();
    for (n, numbers) in plays_lower {
        match runs.last_mut() {
            Some((_, last, same)) if *same == numbers.as_slice() && *n <= *last + 1 => *last = *n,
            _ => runs.push((*n, *n, numbers)),
        }
    }
    let text = |&(first, last, numbers): &(u32, u32, &[u32])| match first == last {
        true => format!("n={first} {numbers:?}"),
        false => format!("n={first}..={last} {numbers:?}"),
    };
    runs.iter().map(text).collect::<Vec<_>>().join("; ")
}

/// The check accepts what the theorem allows and says which statement fails for each way to break it, on two boards
/// where the answers are worked out by hand.
#[test]
fn the_check_says_which_statement_fails() {
    let fails = |t: &Tables, set: &[u32], what: &str| {
        let error = check_theorem_b(t, set).expect_err(&format!("{set:?} breaks the theorem"));
        assert!(error.contains(what), "{set:?}: {error:?} does not say {what:?}");
    };

    // n = 12: q = 11, and U*_2 = {10, 9}, the edges 2-5 and 3-ground.  Of the numbers of layer 2 up to 6, 4 (the edge
    // 2-ground) keeps the graph a forest, and 6 (the edge 2-3) then closes the cycle 2-3-ground.
    let t = Tables::new(12);
    assert_eq!(layer_2_greedy(&t), [10, 9]);
    assert_eq!(check_theorem_b(&t, &[11, 10, 9]), Ok(0));
    assert_eq!(check_theorem_b(&t, &[11, 10, 9, 4]), Ok(1));
    assert_eq!(check_theorem_b(&t, &[11, 10, 9, 8, 12]), Ok(0), "the numbers of layer 3 are not asked about");
    fails(&t, &[7, 10, 9], "statement 1");
    fails(&t, &[11, 7, 10, 9], "statement 1");
    fails(&t, &[10, 9], "statement 1");
    fails(&t, &[11, 10], "statement 2");
    fails(&t, &[11, 10, 9, 4, 6], "statement 3");
    fails(&t, &[11, 13], "off the board");

    // n = 39: Kruskal's algorithm refuses 22 above n/2, which closes the cycle 2-13-3-11 with 26, 39 and 33.
    let t = Tables::new(39);
    let mut set = layer_2_greedy(&t);
    assert!(!set.contains(&22) && set.contains(&26) && set.contains(&39) && set.contains(&33));
    set.push(37);
    assert_eq!(check_theorem_b(&t, &set), Ok(0));
    set.push(22);
    fails(&t, &set, "statement 2");
}

// ---------------------------------------------------------------------------------------------------------
// U*_2 is what setup keeps

/// The numbers of layer 2 above n/2 in the game with no promotions of the board `1..=n`, at the set width the solver
/// picks, and the board's tables.
fn kept_by_setup(n: u32) -> (Vec<u32>, Tables) {
    let (plain, tables) = with_width!(n, W => {
        let Board { plain, tables, .. } = Board::<W>::for_n(n);
        (plain, tables)
    });
    let mut kept: Vec<u32> = plain.into_iter().filter(|&x| 2 * x > n && layer(&tables, x) == 2).collect();
    kept.sort_unstable();
    (kept, tables)
}

/// For each of `boards`, the numbers of layer 2 above n/2 that setup keeps are Kruskal's, from the definition.
fn check_setup_keeps(boards: &[u32]) {
    let start = Instant::now();
    let mut kept_in_all = 0;
    for &n in boards {
        let (kept, t) = kept_by_setup(n);
        let mut greedy = layer_2_greedy(&t);
        greedy.sort_unstable();
        assert_eq!(kept, greedy, "n={n}: the numbers of layer 2 above n/2 that setup keeps and Kruskal's");
        kept_in_all += kept.len();
    }
    println!("{} boards, {kept_in_all} numbers kept in all: {:.1?}", boards.len(), start.elapsed());
    assert!(kept_in_all > boards.len(), "the boards keep numbers of layer 2");
}

/// The numbers of layer 2 above n/2 that setup keeps are Kruskal's, on every board to 400, every 25th to 2200, every
/// 100th to 4000, every 500th to 8000, and the boards 4095, 4096 (the last at a width of 64 words and the first at 128)
/// and 8191.  A board costs about as much as n² (0.2 s at 8000), so the boards past 2200 are thinned.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(layer_2_greedy_is_what_setup_keeps_on_the_boards_to_400))]
fn layer_2_greedy_is_what_setup_keeps() {
    let sampled = |n: u32| {
        n <= 400
            || (n <= 2200 && n.is_multiple_of(25))
            || (n <= 4000 && n.is_multiple_of(100))
            || n.is_multiple_of(500)
            || [4095, 4096, 8191].contains(&n)
    };
    let boards: Vec<u32> = (2..=8191).filter(|&n| sampled(n)).collect();
    check_setup_keeps(&boards);
}

/// [`layer_2_greedy_is_what_setup_keeps`] on every board to 400 and on 4096, the first board at the width of 128 words.
#[test]
fn layer_2_greedy_is_what_setup_keeps_on_the_boards_to_400() {
    let boards: Vec<u32> = (2..=400).chain([4096]).collect();
    check_setup_keeps(&boards);
}

/// The same on every board to 400 and every 25th board to 8191.
#[test]
#[ignore = "takes twenty seconds; a thinner choice of boards is in the default test"]
fn layer_2_greedy_is_what_setup_keeps_on_every_25th_board() {
    let boards: Vec<u32> = (2..=8191u32).filter(|&n| n <= 400 || n.is_multiple_of(25) || n == 8191).collect();
    check_setup_keeps(&boards);
}

// ---------------------------------------------------------------------------------------------------------
// The recorded games

/// A file under the crate's directory (`rust/`).
fn crate_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// The completed games of the results file at `path` with `n` ≥ 2, as `(n, moves)` ascending in `n` (the lines of
/// searches that did not complete are skipped).
fn completed_games(path: &Path) -> Vec<(u32, Vec<u32>)> {
    let none = Optimal::from_json(&Json::Arr(Vec::new())).expect("an empty table");
    let table = none.with_results(path).unwrap_or_else(|e| panic!("{e}"));
    table.games().filter(|&(n, _, _)| n >= 2).map(|(n, _, moves)| (n, moves.to_vec())).collect()
}

/// Theorem B on each of `games` (a board size and a game's moves, taken as a set).  Prints what it found under the name
/// `source`, and returns the failures.
fn check_games(source: &str, games: &[(u32, Vec<u32>)]) -> Vec<String> {
    let start = Instant::now();
    let (mut failures, mut plays_lower) = (Vec::new(), Vec::new());
    for (n, moves) in games {
        if let Err(e) = check_and_note(&Tables::new(*n), moves, &mut plays_lower) {
            failures.push(format!("{source}: n={n}: {e}"));
        }
    }
    let (first, last) = (games[0].0, games[games.len() - 1].0);
    println!(
        "{source}: {} games checked, n from {first} to {last}, {} failures, {:.1?}",
        games.len(),
        failures.len(),
        start.elapsed()
    );
    println!("  games that play numbers of layer 2 up to n/2: {}: {}", plays_lower.len(), runs(&plays_lower));
    failures
}

/// Theorem B on the games of the table `optimal.json`, of the committed chain `results/strict.jsonl` and of the chain
/// 2176 to 2276 of the Graviton calibration.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(theorem_b_holds_for_every_20th_recorded_game))]
fn theorem_b_holds_for_every_recorded_game() {
    theorem_b_holds_for_the_recorded_games(1);
}

/// [`theorem_b_holds_for_every_recorded_game`] for every 20th game of each source.
#[test]
fn theorem_b_holds_for_every_20th_recorded_game() {
    theorem_b_holds_for_the_recorded_games(20);
}

/// Theorem B on every `every`th game of the table, the chain and the calibration's chain.
fn theorem_b_holds_for_the_recorded_games(every: usize) {
    let table: Vec<(u32, Vec<u32>)> =
        Optimal::built_in().games().filter(|&(n, _, _)| n >= 2).map(|(n, _, moves)| (n, moves.to_vec())).collect();
    let chain = completed_games(&results_path("strict.jsonl"));
    let ec2 = completed_games(&crate_path("bench/cloud/c8g.chain-2176.jsonl"));
    assert!(table.len() >= 1299, "the table has the games of 2 to 1300, not {}", table.len());
    assert!(chain.len() >= 3439, "the chain has the games of 2 to 3440, not {}", chain.len());
    assert!(ec2.len() >= 101, "the calibration's chain has the games of 2176 to 2276, not {}", ec2.len());
    let thin = |games: Vec<(u32, Vec<u32>)>| games.into_iter().step_by(every).collect::<Vec<_>>();

    let mut failures = check_games("optimal.json", &thin(table));
    failures.extend(check_games("results/strict.jsonl", &thin(chain)));
    failures.extend(check_games("bench/cloud/c8g.chain-2176.jsonl", &thin(ec2)));
    assert!(failures.is_empty(), "Theorem B fails on {} recorded games:\n{}", failures.len(), failures.join("\n"));
}

// ---------------------------------------------------------------------------------------------------------
// The enumerated optimal sets

/// The lines of the file `prover/data/<name>`.
fn data_lines(name: &str) -> Vec<Json> {
    let path = crate_path("prover/data").join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.lines().filter(|line| !line.trim().is_empty()).map(|line| Json::parse(line).expect("a line of JSON")).collect()
}

/// The sets a line of the prover's lists holds, each ascending: the canonical one (written, or else the recorded game
/// `recorded`) first, then each other as the canonical one less `out` plus `in`.
fn sets_of_line(line: &Json, recorded: &[u32]) -> Result<Vec<Vec<u32>>, String> {
    let mut canonical = match line.get("canonical") {
        Some(written) => written.as_u32_vec().ok_or("a canonical set that is no list of numbers")?,
        None => recorded.to_vec(),
    };
    canonical.sort_unstable();
    let mut sets = vec![canonical.clone()];
    for other in line.get("others").and_then(Json::as_arr).ok_or("no list of other sets")? {
        let numbers =
            |key: &str| other.get(key).and_then(Json::as_u32_vec).ok_or(format!("an other set with no \"{key}\""));
        let (out, added) = (numbers("out")?, numbers("in")?);
        if let Some(x) = out.iter().find(|x| !canonical.contains(x)) {
            return Err(format!("an other set takes out {x}, which the canonical set lacks"));
        }
        if let Some(x) = added.iter().find(|x| canonical.contains(x)) {
            return Err(format!("an other set puts in {x}, which the canonical set has"));
        }
        let mut set: Vec<u32> = canonical.iter().copied().filter(|x| !out.contains(x)).chain(added).collect();
        set.sort_unstable();
        sets.push(set);
    }
    Ok(sets)
}

/// Theorem B on every set of every line of `prover/data/<name>` that holds sets.  Each is checked to be what the line
/// says (the sets are distinct, as many as the line counts, worth its score, playable, the recorded game among them
/// where the line lists all the optimal sets), and then against the theorem.  Prints what it found and returns the
/// failures.
fn check_enumerated(name: &str) -> Vec<String> {
    let start = Instant::now();
    let lines = data_lines(name);
    let table = Optimal::built_in();
    let (mut failures, mut checked, mut sets_checked) = (Vec::new(), 0, 0);
    let (mut several, mut plays_lower) = (Vec::new(), Vec::new());
    for line in &lines {
        let field = |key: &str| line.get(key).and_then(Json::as_u64).unwrap_or_else(|| panic!("a line with no {key}"));
        let (n, score, counted) = (field("n") as u32, field("score"), field("sets") as usize);
        if counted == 0 {
            continue;
        }
        let t = Tables::new(n);
        let mut fail = |why: String| failures.push(format!("{name}: n={n}: {why}"));
        let recorded = table.moves(n).unwrap_or_else(|| panic!("n={n}: no recorded game"));
        let sets = match sets_of_line(line, recorded) {
            Ok(sets) => sets,
            Err(e) => {
                fail(format!("the line cannot be read: {e}"));
                continue;
            }
        };
        let distinct: BTreeSet<&Vec<u32>> = sets.iter().collect();
        if distinct.len() != sets.len() || sets.len() != counted {
            fail(format!("the line counts {counted} sets, and it gives {} ({} distinct)", sets.len(), distinct.len()));
        }
        if line.get("ended").and_then(Json::as_str) != Some("complete") {
            fail("the line holds sets and did not end complete".to_owned());
        }
        let mut recorded_set = recorded.to_vec();
        recorded_set.sort_unstable();
        if line.get("upper").and_then(Json::as_str) == Some("all") && !sets.contains(&recorded_set) {
            fail("the recorded game is not among the optimal sets of the line".to_owned());
        }
        checked += 1;
        if sets.len() > 1 {
            several.push(format!("n={n} ({})", sets.len()));
        }
        for set in &sets {
            sets_checked += 1;
            let sum: u64 = set.iter().map(|&x| u64::from(x)).sum();
            if sum != score {
                fail(format!("a set is worth {sum}, not the score {score}: {set:?}"));
            }
            let on_board = set.iter().all(|x| (2..=n).contains(x)) && set.windows(2).all(|pair| pair[0] != pair[1]);
            if !on_board || !playable(&t, set) {
                fail(format!("a set of the line is not playable: {set:?}"));
            }
            if let Err(e) = check_and_note(&t, set, &mut plays_lower) {
                fail(format!("{e}: the set {set:?}"));
            }
        }
    }
    println!(
        "{name}: {} lines read, {checked} with sets, {sets_checked} sets checked, {} failures, {:.1?}",
        lines.len(),
        failures.len(),
        start.elapsed()
    );
    println!("  boards with more than one set: {}: {}", several.len(), several.join(", "));
    println!("  sets that play numbers of layer 2 up to n/2: {}: {}", plays_lower.len(), runs(&plays_lower));
    assert!(checked > 0, "{name} has no line with sets");
    failures
}

/// Theorem B on every optimal set the prover enumerated: all those of the boards 2 to 989, and those of the boards 798
/// to 1019 that have U\* above n/2.
#[test]
fn theorem_b_holds_for_every_enumerated_optimal_set() {
    let mut failures = check_enumerated("optsets_2_989.jsonl");
    failures.extend(check_enumerated("optsets_fixed_798_1019.jsonl"));
    assert!(failures.is_empty(), "Theorem B fails on {} enumerated sets:\n{}", failures.len(), failures.join("\n"));
}

// ---------------------------------------------------------------------------------------------------------
// The small boards, by exhaustive search

/// A depth first search for every playable set worth exactly `target` on the board of `t`, with no model of the game.
///
/// The numbers `2..=n` are decided from the largest, each in or out.  A number can go in only while the set stays
/// playable (the oracle; a subset of a playable set is playable, so no extension of an unplayable set is), so each node
/// has the numbers *open*: those not yet decided that the set so far can still take.  A branch ends when the set and
/// all its open numbers cannot reach `target`.  Each playable set is reached once.
struct Search<'a> {
    t: &'a Tables,
    target: u64,
    found: Vec<Vec<u32>>,
    /// The playable sets worth more than `target`, which the optimum rules out.
    beaten: Vec<Vec<u32>>,
    nodes: u64,
}

impl<'a> Search<'a> {
    /// Every playable set worth `target` on the board of `t`, ascending, and the number of nodes visited.
    fn run(t: &'a Tables, target: u64) -> (Vec<Vec<u32>>, u64) {
        let mut search = Search { t, target, found: Vec::new(), beaten: Vec::new(), nodes: 0 };
        search.extend(&mut Vec::new(), 0, &(2..=t.n).rev().collect::<Vec<u32>>());
        assert!(search.beaten.is_empty(), "n={}: playable sets worth more than {target}: {:?}", t.n, search.beaten);
        let mut found = search.found;
        found.iter_mut().for_each(|set| set.sort_unstable());
        found.sort_unstable();
        (found, search.nodes)
    }

    /// Decides the `open` numbers (in decreasing order) for the playable `set`, worth `sum`.
    fn extend(&mut self, set: &mut Vec<u32>, sum: u64, open: &[u32]) {
        self.nodes += 1;
        if sum + open.iter().map(|&x| u64::from(x)).sum::<u64>() < self.target {
            return;
        }
        let Some((&x, later)) = open.split_first() else {
            if sum == self.target {
                self.found.push(set.clone());
            } else {
                self.beaten.push(set.clone());
            }
            return;
        };
        // With `x`: of the later numbers, those that the set with `x` can still take stay open.
        set.push(x);
        let t = self.t;
        let still_open: Vec<u32> = later
            .iter()
            .copied()
            .filter(|&y| {
                set.push(y);
                let takes = playable(t, set);
                set.pop();
                takes
            })
            .collect();
        self.extend(set, sum + u64::from(x), &still_open);
        set.pop();
        // Without `x`: the later numbers stay as they are.
        self.extend(set, sum, later);
    }
}

/// How many optimal sets the prover found for each board it finished (`optsets_2_989.jsonl`).
fn enumerated_counts() -> BTreeMap<u32, usize> {
    let complete = |line: &Json| line.get("ended").and_then(Json::as_str) == Some("complete");
    let count = |line: &Json| {
        let field = |key: &str| line.get(key).and_then(Json::as_u32).expect("a number in the line");
        (field("n"), field("sets") as usize)
    };
    data_lines("optsets_2_989.jsonl").iter().filter(|line| complete(line)).map(count).collect()
}

/// The largest board on which the brute force of the rules is run (past this it takes too long).
const BRUTE_FORCE_TO: u32 = 40;

/// Theorem B on every optimal set of each board of `boards`, found by [`Search`]: there is one, the brute force has the
/// optimum (on the boards it reaches), the number of optimal sets is the prover's, the recorded game is one of them,
/// and the theorem holds of each.  Prints what it found and returns the failures.
fn check_small_boards(boards: RangeInclusive<u32>) -> Vec<String> {
    let start = Instant::now();
    let (table, enumerated) = (Optimal::built_in(), enumerated_counts());
    let (mut failures, mut sets_in_all, mut nodes_in_all) = (Vec::new(), 0, 0);
    let (mut several, mut plays_lower) = (Vec::new(), Vec::new());
    for n in boards.clone() {
        let t = Tables::new(n);
        let target = table.score(n).unwrap_or_else(|| panic!("n={n}: no score in the table"));
        let (sets, nodes) = Search::run(&t, target);
        let mut fail = |why: String| failures.push(format!("n={n}: {why}"));
        if sets.is_empty() {
            fail(format!("no playable set is worth the optimum {target}"));
        }
        if n <= BRUTE_FORCE_TO {
            let brute_force = brute::solve(n, &[], Rule::Divisor).score;
            if brute_force != target {
                fail(format!("the brute force scores {brute_force}, the table {target}"));
            }
        }
        if let Some(&counted) = enumerated.get(&n) {
            if counted != sets.len() {
                fail(format!("{} optimal sets found, the prover has {counted}", sets.len()));
            }
        }
        let mut recorded = table.moves(n).expect("a recorded game").to_vec();
        recorded.sort_unstable();
        if !sets.contains(&recorded) {
            fail(format!("the recorded game {recorded:?} is not among the optimal sets"));
        }
        for set in &sets {
            if let Err(e) = check_and_note(&t, set, &mut plays_lower) {
                fail(format!("{e}: the set {set:?}"));
            }
        }
        if sets.len() > 1 {
            several.push(format!("n={n} ({})", sets.len()));
        }
        sets_in_all += sets.len();
        nodes_in_all += nodes;
    }
    println!(
        "the boards {boards:?}: {sets_in_all} optimal sets, {nodes_in_all} nodes, {} failures, {:.1?}",
        failures.len(),
        start.elapsed()
    );
    println!("  boards with more than one set: {}: {}", several.len(), several.join(", "));
    println!("  sets that play numbers of layer 2 up to n/2: {}: {}", plays_lower.len(), runs(&plays_lower));
    failures
}

/// The largest board of the quick test of the small boards: the search takes a few tenths of a second to here.
const QUICK_SMALL_BOARDS_TO: u32 = 36;

/// The largest board of the default test of the small boards: the search takes a few seconds to here.
const SMALL_BOARDS_TO: u32 = 48;

/// The largest board of the ignored test of the small boards: 54 ends the run of boards from 42 whose optimal sets play
/// a number of layer 2 up to n/2 (the next such board is 70).
const MORE_SMALL_BOARDS_TO: u32 = 54;

/// Theorem B on every optimal set of every board from 2 to [`QUICK_SMALL_BOARDS_TO`], found by exhaustive search.
#[test]
fn theorem_b_holds_for_every_optimal_set_of_the_smallest_boards() {
    let failures = check_small_boards(2..=QUICK_SMALL_BOARDS_TO);
    assert!(failures.is_empty(), "Theorem B fails on {} small boards:\n{}", failures.len(), failures.join("\n"));
}

/// Theorem B on every optimal set of every board from 2 to [`SMALL_BOARDS_TO`], found by exhaustive search.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(theorem_b_holds_for_every_optimal_set_of_the_smallest_boards))]
fn theorem_b_holds_for_every_optimal_set_of_the_small_boards() {
    let failures = check_small_boards(2..=SMALL_BOARDS_TO);
    assert!(failures.is_empty(), "Theorem B fails on {} small boards:\n{}", failures.len(), failures.join("\n"));
}

/// The same on the boards from [`SMALL_BOARDS_TO`] + 1 to [`MORE_SMALL_BOARDS_TO`], where each board takes the search
/// seconds (and about a third more than the one before).
#[test]
#[ignore = "takes twenty seconds; the boards to 48 are in the default test"]
fn theorem_b_holds_for_every_optimal_set_of_more_small_boards() {
    let failures = check_small_boards(SMALL_BOARDS_TO + 1..=MORE_SMALL_BOARDS_TO);
    assert!(failures.is_empty(), "Theorem B fails on {} small boards:\n{}", failures.len(), failures.join("\n"));
}
