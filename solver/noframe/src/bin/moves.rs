//! The longest games against the optimal ones: for every game of a range, M(n), the most moves a game of n can
//! have, decided by the prover ([`taxman_prover::moves`]), against the moves of the recorded optimal game
//! (optimal.json and, for a game it lacks, the game of `--results`).  One line per
//! game, resumable:
//!
//!     moves --from A --to B [--list ...] [--budget NODES] [--heaviest 0] [--seconds S] [--optimal FILE]
//!           [--results FILE] --out FILE
//!
//! The games are worked in increasing order, each from its predecessor's line (in the output file or just
//! written): a longest game of n − 1 is a game of n, M(n) <= M(n − 1) + 1, and M(n) = M(n − 1) for a prime
//! n >= 3.  Per game: the recorded game is replayed by the rules (its moves `recorded_moves`, of which
//! `recorded_upper` above n/2); M(n) lies between the longer of it and the predecessor's longest game, and the
//! smaller of ν(n) (`nu`), the matching number of the board's maximal-factor graph, and what M(n − 1) allows.
//! Where the two meet, M(n) is settled (`"how":"matching"` if ν(n) is met, `"predecessor"` otherwise);
//! elsewhere the unit-weight branch and bound decides it (`"search"`, with the segment cuts from the smaller
//! games' bounds; `"budget"` if `--budget` nodes did not suffice, and `most` is then null).  A range should
//! start at 2, so that each game has its predecessor; one that does not starts from the recorded game alone.
//!
//! Where a game is longer than the recorded one (`gap` > 0), the line has a longest game (`longest`, ascending;
//! replayed by the rules in the order `play_order` gives) and, unless `--heaviest 0`, the heaviest longest game:
//! its sum and how it differs from the recorded game (`heaviest_out`, the recorded game's moves it does not
//! play, and `heaviest_in`, its moves the recorded game does not play), from a search for the heaviest set of
//! M(n) picks that starts with those whose upper part is the recorded game's.  The comparison with the optimal
//! game rests on the recorded score being opt(n) (proven on the whole board to 989 and 999-1019, by `exact`).
//! Starts no new game after `--seconds`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use taxman_prover::game::Board;
use taxman_prover::jsonl::{field_nums, field_u64, parse_items, recorded_games, Appender, Obj};
use taxman_prover::moves::{heaviest_longest, longest, matching_numbers, play_order};
use taxman_prover::primes::{is_prime, smallest_prime_factors};

/// What a game's line says its successor needs: M(n) if decided, and a longest game found (its moves).
struct Known {
    most: Option<u64>,
    longest: Vec<u32>,
}

fn sum(set: &[u32]) -> u64 {
    set.iter().map(|&c| u64::from(c)).sum()
}

fn main() {
    let (items, flags) = parse_items(std::env::args().skip(1));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let (mut budget, mut heaviest, mut seconds) = (u64::MAX, true, f64::INFINITY);
    let mut optimal = root.join("../../results/optimal.json");
    let mut results = root.join("../../results/strict.jsonl");
    let mut out = PathBuf::new();
    for (flag, value) in flags {
        match flag.as_str() {
            "--budget" => budget = value.parse().unwrap(),
            "--heaviest" => heaviest = value == "1" || value == "true",
            "--seconds" => seconds = value.parse().unwrap(),
            "--optimal" => optimal = value.into(),
            "--results" => results = value.into(),
            "--out" => out = value.into(),
            _ => panic!("unknown flag {flag}"),
        }
    }
    assert!(!out.as_os_str().is_empty(), "--out is required");
    let recorded = recorded_games(&[&optimal, &results]);
    let mut items: Vec<u64> = items.into_iter().filter(|n| *n >= 2 && recorded.contains_key(n)).collect();
    items.sort_unstable();
    items.dedup();
    let Some(&last) = items.last() else { return };
    let spf = smallest_prime_factors(last as usize);
    // ν(m) for every m <= the last game: the board 1..=m's graph is the one of 1..=last restricted to it.
    let nu = matching_numbers(&Board::new(last as u32, &spf));
    let (mut file, lines) = Appender::open(&out).expect("open the output file");
    let mut known: BTreeMap<u64, Known> = BTreeMap::new();
    for line in &lines {
        let Some(n) = field_u64(line, "n") else { continue };
        let longest = match field_nums(line, "longest") {
            Some(set) => set.into_iter().map(|c| c as u32).collect(),
            None => recorded[&n].1.clone(),
        };
        known.insert(n, Known { most: field_u64(line, "most"), longest });
    }
    let todo: Vec<u64> = items.iter().copied().filter(|n| !known.contains_key(n)).collect();
    eprintln!("moves: {} to do ({} already in {})", todo.len(), known.len(), out.display());
    let deadline = Instant::now() + Duration::from_secs_f64(seconds.min(1e9));
    for n in todo {
        if Instant::now() >= deadline {
            break;
        }
        let started = Instant::now();
        let board = Board::new(n as u32, &spf);
        let (score, moves) = &recorded[&n];
        assert_eq!(board.replay(moves), Ok(*score), "n={n}: the recorded game does not replay to its score");
        let k = moves.len() as u64;
        let upper_part: Vec<u32> = moves.iter().copied().filter(|&c| 2 * u64::from(c) > n).collect();
        let prev = known.get(&(n - 1));
        // M(n) is at least the recorded game's moves and the predecessor's longest game's (a game of n − 1 is a
        // game of n), and at most ν(n) and M(n − 1) + 1, or M(n − 1) for a prime n >= 3.
        let mut witness = match prev {
            Some(p) if p.longest.len() as u64 > k => p.longest.clone(),
            _ => moves.clone(),
        };
        witness.sort_unstable();
        let below = witness.len() as u64;
        let step = u64::from(!(n >= 3 && is_prime(n, &spf)));
        let above = prev.and_then(|p| p.most).map_or(nu[n as usize], |m| nu[n as usize].min(m + step));
        assert!(below <= above, "n={n}: a game of {below} moves above the bound {above}");
        let (most, how, nodes) = if below == above {
            (Some(below), if nu[n as usize] == below { "matching" } else { "predecessor" }, 0)
        } else {
            // Upper bounds on M(m), m < n, for the segment cuts.
            let smaller: Vec<u64> =
                (0..n).map(|m| known.get(&m).and_then(|g| g.most).unwrap_or(u64::MAX).min(nu[m as usize])).collect();
            let found = longest(&board, below, Some(&smaller), budget);
            if let Some(set) = found.longer {
                witness = set;
            }
            (found.most, if found.most.is_some() { "search" } else { "budget" }, found.nodes)
        };
        let mut line = Obj::new()
            .num("n", n)
            .num("score", *score)
            .num("recorded_moves", k)
            .num("recorded_upper", upper_part.len())
            .num("nu", nu[n as usize])
            .raw("most", &most.map_or("null".into(), |m| m.to_string()))
            .str("how", how)
            .num("nodes", nodes);
        let longer = witness.len() as u64 > k;
        if longer {
            let order = play_order(&board, &witness).expect("a longest game is playable");
            assert_eq!(board.replay(&order), Ok(sum(&witness)), "n={n}: the longest game does not replay");
            line = line.num("gap", witness.len() as u64 - k);
            line = line.nums("longest", &witness).num("longest_sum", sum(&witness));
        } else if most.is_some() {
            line = line.num("gap", 0);
        }
        if longer && heaviest && most.is_some() {
            let found = heaviest_longest(&board, &witness, Some(&upper_part), budget);
            line = line.num("heaviest_nodes", found.nodes);
            if let Some(set) = found.set {
                let order = play_order(&board, &set).expect("the heaviest longest game is playable");
                assert_eq!(board.replay(&order), Ok(sum(&set)), "n={n}: the heaviest longest game does not replay");
                let mut gone: Vec<u32> = moves.iter().copied().filter(|c| !set.contains(c)).collect();
                gone.sort_unstable();
                let new: Vec<u32> = set.iter().copied().filter(|c| !moves.contains(c)).collect();
                line = line.num("heaviest_sum", sum(&set)).nums("heaviest_out", &gone).nums("heaviest_in", &new);
            }
        }
        let line = line.float("secs", started.elapsed().as_secs_f64()).finish();
        file.append(&line).expect("append a result");
        if longer || how == "search" || how == "budget" || n % 100 == 0 {
            eprintln!("moves: n={n} moves={k} nu={} most={most:?} {how}", nu[n as usize]);
        }
        known.insert(n, Known { most, longest: witness });
    }
}
