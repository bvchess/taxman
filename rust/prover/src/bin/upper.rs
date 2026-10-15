//! The upper part of the optimal sets, game by game: for every game in a range, which members of U\*_{≥3} (the
//! sources of three or more prime factors that descending greedy keeps) the parity bound certifies to be in every
//! set beating the recorded score, and which it leaves open.  The recorded games are optimal.json's (to 1300) and, for a board
//! size it lacks, those of the `--results` files (default the solver's `results/strict.jsonl`), read in the order
//! given after optimal.json, an earlier file winning.  One line per game, resumable:
//!
//!     upper --from A --to B [--list ...] [--threads K] [--seconds S] [--optimal FILE] [--results FILE]...
//!           --out FILE
//!
//! Every line has the game (`n`, its recorded `score`), its `relation` to the game before (`chain`, `prime` or
//! `none`; `src/game.rs`) and `needs`, 1 if the relation is `none`: a game at its ceiling or meeting the prime
//! formula rests on n − 1, not on the frame model, and has nothing more.  For a game that needs it the line gives
//! the parity bound at the roles of Theorem B (`root`) and its `gap` over the score, whether the root's heaviest set
//! holds all of U\*_{≥3} (`root_has_upper`), the number of members of U\*_{≥3} (`members`), how many the bound
//! certifies (`certified`), and the `open` ones as triples [u, Ω(u), margin], ascending in u, the margin being the
//! bound with u barred minus the score (null if the bound could not be certified).  Per game the recorded game is
//! replayed by the rules, its moves above n/2 are checked to be U\* and the whole game to obey Theorem B's roles
//! (checks of the data).
//!
//! The roles are those of every optimal set (THEORY.md, Theorem B; `upper.rs`).  A member u is certified when the
//! bound with u barred as well is at most the score V (or the roles cannot be met): then every set that beats V and
//! obeys Theorem B holds u, so if opt(n) > V every optimal set holds every certified member.

use std::path::{Path, PathBuf};
use std::time::Instant;
use taxman_prover::game::{relation, Board, Relation};
use taxman_prover::jsonl::{campaign, parse_items, recorded_games, Obj};
use taxman_prover::primes::smallest_prime_factors;
use taxman_prover::upper::{barred_bounds, low_roles, obeys, recorded_ustar, Open};

/// An open member as a JSON triple [u, Ω(u), margin], the margin null when there is none.
fn triple(open: &Open) -> String {
    let margin = open.margin.map_or_else(|| String::from("null"), |m| m.to_string());
    format!("[{},{},{margin}]", open.u, open.omega)
}

fn main() {
    let (items, flags) = parse_items(std::env::args().skip(1));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let (mut threads, mut seconds) = (1usize, f64::INFINITY);
    let mut optimal = root.join("../../src/main/resources/optimal.json");
    let mut results: Vec<PathBuf> = Vec::new();
    let mut out = PathBuf::new();
    for (flag, value) in flags {
        match flag.as_str() {
            "--threads" => threads = value.parse().unwrap(),
            "--seconds" => seconds = value.parse().unwrap(),
            "--optimal" => optimal = value.into(),
            "--results" => results.push(value.into()),
            "--out" => out = value.into(),
            _ => panic!("unknown flag {flag}"),
        }
    }
    assert!(!out.as_os_str().is_empty(), "--out is required");
    if results.is_empty() {
        results.push(root.join("../results/strict.jsonl"));
    }
    let files: Vec<&Path> = std::iter::once(optimal.as_path()).chain(results.iter().map(PathBuf::as_path)).collect();
    let known = recorded_games(&files);
    let items: Vec<u64> = items.into_iter().filter(|n| *n >= 2 && known.contains_key(n)).collect();
    let spf = smallest_prime_factors(items.iter().copied().max().unwrap_or(2) as usize);
    campaign("upper", &items, threads, seconds, &out, |n| {
        let started = Instant::now();
        let (score, moves) = &known[&n];
        let board = Board::new(n as u32, &spf);
        assert_eq!(board.replay(moves), Ok(*score), "n={n}: the recorded game does not replay to its score");
        let ustar = recorded_ustar(n, moves, &spf);
        let roles = low_roles(n as u32, &spf, &ustar);
        obeys(&roles, moves).unwrap_or_else(|e| panic!("n={n}: the recorded game breaks the roles of Theorem B: {e}"));
        let prev = known.get(&(n - 1)).map_or(0, |(s, _)| *s);
        let relation = relation(n, *score, prev, &spf);
        let needs = relation == Relation::None;
        let mut line =
            Obj::new().num("n", n).num("score", *score).str("relation", relation.name()).num("needs", u8::from(needs));
        let mut note = None;
        if needs {
            let barred = barred_bounds(&board, &spf, *score, &ustar);
            let gap = barred.root - *score as i64;
            assert!(gap >= 0, "n={n}: the parity bound {} is below a legal game's {score}", barred.root);
            let open: Vec<String> = barred.open.iter().map(triple).collect();
            line = line
                .num("root", barred.root)
                .num("gap", gap)
                .num("root_has_upper", u8::from(barred.root_has_upper))
                .num("members", barred.members.len())
                .num("certified", barred.members.len() - barred.open.len())
                .raw("open", &format!("[{}]", open.join(",")));
            note = Some(format!("{} of {} open, gap {gap}", barred.open.len(), barred.members.len()));
        }
        (Some(line.float("secs", started.elapsed().as_secs_f64()).finish()), note)
    });
}
