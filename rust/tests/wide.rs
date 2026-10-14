//! Large boards without large searches.
//!
//! The Java stopped at n = 1000, and the sets that hold the numbers of a board are 8 words wide up
//! to n = 511, then 16, 20, 24, 28 and 32 words up to 1023, 1279, 1535, 1791 and 2047, and 64 up to 4095
//! (`bits::WIDTHS`), so nothing above n = 1023 meets anything but these tests and the committed results
//! (`results/strict.jsonl`, n = 1..2175; `optimal.json` has its games to 1300, but that is the solver's own answer
//! and no check of it).  What is cheap to run here: the same
//! small game at every width that holds it (a width-dependent bug changes the tree), games of 1100 to 1250 that need
//! a few thousand nodes against the committed results, and boards of 2048 and more through their setup.

mod common;

use common::{
    config, matching_options, options, prev_of, profile_at_width, stored_games, testdata, variants, Config, Game,
    Profile, Rng, TempDir,
};
use std::time::Duration;
use taxman::bits::Bits;
use taxman::engine::EngineConfig;
use taxman::frames::build_frames;
use taxman::matchbound::{match_bound, MatchBound, MatchGraph};
use taxman::model::{Model, Scratch, MAX_FRAMES};
use taxman::numtheory::Tables;
use taxman::sacrifice::{sacrifice_bound, PuncturedOptions};
use taxman::search::{SearchTactics, TaxUnit};
use taxman::solver::{self, Method, Prev, PrevSource, SolveOptions};
use taxman::with_width;

// ---------------------------------------------------------------------------------------------------------
// The width does not matter

/// The same game, built at every width that holds it, has the same frames, candidates, greedy set, `max_total`,
/// search and tree (nodes counted).  A bug that depends on the width (a loop over fewer words than the set has, a
/// mask that is wrong at the top, a width that is not a power of two) shows as a different tree: the widths above
/// the ones the boards below need are otherwise reached by no test.
#[test]
fn the_search_does_not_depend_on_the_width() {
    let default = SearchTactics::default();
    // Boards that W = 8 holds, at every width.
    for n in [300, 400] {
        let at8 = profile_at_width::<8>(n, default, None);
        assert!(at8.completed && at8.sum > 0, "n={n}");
        let others = [
            (16, profile_at_width::<16>(n, default, None)),
            (20, profile_at_width::<20>(n, default, None)),
            (24, profile_at_width::<24>(n, default, None)),
            (28, profile_at_width::<28>(n, default, None)),
            (32, profile_at_width::<32>(n, default, None)),
            (64, profile_at_width::<64>(n, default, None)),
        ];
        for (w, profile) in others {
            assert_eq!(profile, at8, "n={n} at W={w}");
        }
    }
    // Boards that W = 16 holds, with trees of more than 1,000 nodes: on the matching path and with the GF(2) bound.
    let matching = SearchTactics::matching();
    for (n, tactics) in [(760, matching), (1000, default)] {
        let at16 = profile_at_width::<16>(n, tactics, None);
        assert!(at16.completed && at16.nodes > 1000, "n={n}: {} nodes", at16.nodes);
        assert_eq!(profile_at_width::<32>(n, tactics, None), at16, "n={n} at W=32");
        assert_eq!(profile_at_width::<64>(n, tactics, None), at16, "n={n} at W=64");
    }

    // Boards that need W = 20: 1133 on the matching path, and 1050 with the GF(2) bound (which searches 1133 in a
    // hundred nodes).
    for (n, name, tactics) in [(1133, "matching", matching), (1050, "default", default)] {
        let at20 = profile_at_width::<20>(n, tactics, None);
        assert!(at20.completed && at20.nodes > 500, "{name}: n={n}: {} nodes", at20.nodes);
        assert_eq!(profile_at_width::<24>(n, tactics, None), at20, "{name}: n={n} at W=24");
        assert_eq!(profile_at_width::<32>(n, tactics, None), at20, "{name}: n={n} at W=32");
        assert_eq!(profile_at_width::<64>(n, tactics, None), at20, "{name}: n={n} at W=64");
    }
}

/// The last board a width holds (511, 1023, 1279, 1535, 1791, 2047) fills the top bit of its set: the start of every
/// search (frames, candidates, greedy set, `max_total`, the first nodes) is the same at the width the board
/// dispatches to and at the next one.  (The boards just above 511, 1023 and 2047 are in `testdata/golden_wide` at
/// their own widths.)
#[test]
fn the_edges_of_the_widths_hold() {
    let default = SearchTactics::default();
    let limit = Some(1);
    let check = |profiles: [Profile; 2], n: u32| assert_eq!(profiles[0], profiles[1], "n={n}");
    check([profile_at_width::<8>(511, default, limit), profile_at_width::<16>(511, default, limit)], 511);
    check([profile_at_width::<16>(1023, default, limit), profile_at_width::<20>(1023, default, limit)], 1023);
    check([profile_at_width::<20>(1279, default, limit), profile_at_width::<24>(1279, default, limit)], 1279);
    check([profile_at_width::<24>(1535, default, limit), profile_at_width::<28>(1535, default, limit)], 1535);
    check([profile_at_width::<28>(1791, default, limit), profile_at_width::<32>(1791, default, limit)], 1791);
    check([profile_at_width::<32>(2047, default, limit), profile_at_width::<64>(2047, default, limit)], 2047);
}

// ---------------------------------------------------------------------------------------------------------
// Games of 1100 and more, against the committed results

/// Game 1133 (W = 20, the width of the hardest games) searches in a few thousand nodes: every sound tactic must
/// give the score of `results/strict.jsonl`, on one thread, and the default also on two.  Nothing else searches at
/// a width of 20 or more and checks the answer.
#[test]
fn every_tactic_gives_the_recorded_score_at_w20() {
    let games = stored_games("strict.jsonl");
    let prev = prev_of(&games, 1133);
    for (name, tactics) in variants() {
        for threads in if name == "default" { vec![1, 2] } else { vec![1] } {
            let mut opts = options(threads);
            opts.tactics.search = tactics;
            let r = solver::solve_game(1133, Some(&prev), &opts).unwrap();
            assert!(r.completed && r.method == Method::Search, "{name}");
            assert_eq!(r.score, games[&1133].0, "{name} on {threads} threads");
            assert_eq!(r.tactics.search, tactics, "{name}");
        }
    }
}

/// A slice of the chain (every game feeding the next, from game 1126 as the committed results have it) reproduces
/// the committed results' scores with the defaults they were made with (on two threads, not four).  The games are
/// handed on in memory; `tests/integrity.rs` covers the results file.
#[test]
fn a_slice_of_the_chain_reproduces_the_results() {
    let games = stored_games("strict.jsonl");
    let opts = options(2);
    let mut prev = prev_of(&games, 1127);
    let mut searched = 0;
    for n in 1127..=1134 {
        let r = solver::solve_game(n, Some(&prev), &opts).unwrap();
        assert!(r.completed, "n={n}");
        assert_eq!(r.score, games[&n].0, "n={n}");
        searched += usize::from(r.method == Method::Search);
        prev = Prev { moves: r.moves, score: r.score, source: PrevSource::Run, proof: r.proof, assumes: r.assumes };
    }
    assert!(searched >= 5, "{searched} of the 8 games were searched");
}

// ---------------------------------------------------------------------------------------------------------
// Boards that need W = 64

/// A frame set holds `MAX_FRAMES` = 8 frames, which the largest boards have (every board up to 4095, the largest
/// supported, was checked when it came down from 12: 6 frames from n = 512, 7 from 1024, 8 from 2048).  The boards
/// at the steps, and the largest.
#[test]
fn the_largest_boards_fit_in_a_frame_set() {
    assert_eq!(MAX_FRAMES, 8);
    for (n, frames) in [(511, 5), (512, 6), (1023, 6), (1024, 7), (2047, 7), (2048, 8), (4095, 8)] {
        let built = with_width!(n, W => build_frames::<W>(&Tables::new(n)).frames.len());
        assert_eq!(built, frames, "n={n}");
    }
}

/// The first board with a number above 2047 is n = 2048 (W = 64 by dispatch only until a frame holds one); n = 2050
/// has such numbers in its frames.  Its frames, model, matching bound, conflicts, greedy set, setup and a short
/// search, which nothing else runs at W = 64.
#[test]
fn a_board_that_needs_w64_builds_bounds_and_searches() {
    let n = 2050;
    let tables = Tables::new(n);
    let frames = build_frames::<64>(&tables);
    let (model, root) = Model::build::<64>(&tables, &frames.columns()).unwrap();
    assert!(frames.candidates.len() > 500);
    assert!(frames.frames.iter().any(|f| (f.moves | f.factors).max().is_some_and(|m| m > 2047)));

    let graph = MatchGraph::new(&model, &root);
    let mut m = MatchBound::new(&graph);
    m.reset(&Bits::EMPTY, &frames.candidates);
    assert_eq!(m.value(), match_bound(&model, &root, &frames.candidates));
    // The matching's members cannot all be promoted (the root of every large game), and a conflict says why.
    let chosen = m.chosen();
    let mut scratch = Scratch::new();
    let k = model.conflict_using(&mut scratch, &root, &chosen, true).expect("a conflict at the root");
    assert!(k.is_subset(&chosen) && !model.can_promote_all_using(&mut scratch, &root, &k));
    let greedy = taxman::bounds::greedy(&model, &root, &frames.candidates);
    assert!(model.can_promote_all_using(&mut scratch, &root, &greedy) && greedy.sum() <= m.value());

    // A short search from the root, which the engine's node limit stops: the unit, the incumbent and the payload of
    // a W = 64 search.  (`tests/solver.rs` runs the whole pipeline's setup and first nodes for this board.)
    let game = Game::<64>::new(model, root, frames.candidates);
    let engine = EngineConfig { node_limit: Some(1), ..config(1) };
    let out = game.search(Config::standard(), vec![game.root_unit()], greedy.sum(), Some(engine), None);
    assert!(!out.completed && out.nodes > 0 && out.best >= greedy.sum());
    assert!(out.best == greedy.sum() || out.real, "{out:?}");
}

/// At W = 20 the sacrifice bound (a search of punctured boards, which is the theorem that the optimum of `n` is at
/// most the best of them plus a known term) is the only upper bound that does not come from the search being tested
/// (the Java stops at 1023): it must be at least the recorded score, and where it is tight it proves it, as it does
/// for game 1133 (not for game 1131, whose bound is larger).
#[test]
fn the_sacrifice_bound_proves_a_w20_result() {
    let games = stored_games("strict.jsonl");
    let n = 1133;
    let (prev_score, prev) = &games[&(n - 1)];
    let bound = sacrifice_bound(n, prev, *prev_score, None, &PuncturedOptions::default()).unwrap();
    let score = games[&n].0;
    assert!(bound.exact(), "n={n}: some punctured game is not exact");
    assert_eq!(bound.value, score, "n={n}: the bound {} should be the recorded optimum", bound.value);
}

// ---------------------------------------------------------------------------------------------------------
// Resuming

/// A checkpoint that an earlier build wrote (`testdata/n1133_after_1000_nodes.ckpt`: game 1133 at W = 20 with
/// the default tactics of that build, now those of the matching path (`SearchTactics::matching`), stopped after
/// 1,023 nodes on one thread; search header format TXSH v4) still resumes and
/// finishes with the committed score.  Every write/read test of the format uses the code's own constants, so a
/// change of the layout that round-trips (the version, the byte order of a set, the order of the header's fields,
/// the packing of the tactics) passes them and refuses this file.
///
/// A deliberate change of the format (a new header version, say, which refuses older checkpoints) fails this test.
/// To make the file again with the new build: `taxman solve 1133 --use-prev --threads 1 --node-limit 1000 --out R
/// --checkpoint-dir D --tactic gf2=off --tactic lagrange=on` with a new file `R`, then copy `D/n1133.ckpt` over the
/// fixture.
#[test]
fn a_committed_checkpoint_resumes() {
    let games = stored_games("strict.jsonl");
    let dir = TempDir::new("committed-checkpoint");
    let opts = SolveOptions { checkpoint_dir: Some(dir.join("ckpt")), ..matching_options(1) };
    let checkpoint = opts.checkpoint_file(1133).unwrap();
    std::fs::create_dir_all(checkpoint.parent().unwrap()).unwrap();
    std::fs::copy(testdata("n1133_after_1000_nodes.ckpt"), &checkpoint).unwrap();

    let r = solver::solve_game(1133, Some(&prev_of(&games, 1133)), &opts).unwrap();
    assert!(r.completed && r.method == Method::Search);
    assert_eq!(r.score, games[&1133].0);
    assert_eq!(r.invocations, 2, "the game was resumed, not started over");
    assert!(r.assumes.is_empty(), "{:?}", r.assumes);
    assert!(r.nodes > 1000, "{} nodes in all, 1,000 of them before the checkpoint", r.nodes);
    assert!(!checkpoint.exists(), "a finished game removes its checkpoint");
}

/// A W = 20 search interrupted every few hundred nodes and resumed from its checkpoint, under two sets of tactics
/// of the matching path (the tactics are part of the checkpoint's header and must read back as written; the GF(2)
/// bound's search of 1133 is too short to stop, and `tests/soundness.rs` resumes it on game 1000 at W = 20).  The
/// unit tests resume only at W = 16.
#[test]
fn a_w20_search_interrupted_and_resumed_gives_the_same_answer() {
    let games = stored_games("strict.jsonl");
    let prev = prev_of(&games, 1133);
    for (name, tactics) in variants().into_iter().filter(|(name, _)| name.starts_with("matching")).take(2) {
        let dir = TempDir::new(&format!("resume-{}", name.replace(['=', ','], "-")));
        let mut opts = options(1);
        opts.tactics.search = tactics;
        opts.checkpoint_dir = Some(dir.join("ckpt"));
        opts.node_limit = Some(700);
        opts.queue_mem_cap = 2;
        let mut pieces = 0;
        let result = loop {
            let r = solver::solve_game(1133, Some(&prev), &opts).unwrap();
            pieces += 1;
            if r.completed {
                break r;
            }
            assert!(opts.checkpoint_file(1133).unwrap().exists(), "{name}: an unfinished game keeps its checkpoint");
            assert!(pieces < 50, "{name}: no progress");
        };
        assert_eq!(result.score, games[&1133].0, "{name}");
        assert!(!opts.checkpoint_file(1133).unwrap().exists(), "{name}");
        assert!(pieces >= 2, "{name}: {pieces} pieces");
    }
}

// ---------------------------------------------------------------------------------------------------------
// Promoting on wide boards

/// Promoting one number at a time (`Model::promote`, which skips the peel when the move has a factor no move of
/// the frame in front of it uses) leaves the same state, and fails in the same places, as promoting it with the peel of
/// every frame (`promote_all_using` of the one number), in states reached by promoting random candidates; and those
/// states are what promoting them all at once leaves.  The shortcut is exact only in winnable frames, which every
/// reached state has because every root frame is (the frame builder's roots; `search::prepare` checks it in debug
/// builds, and `Game::of_board` here): the unit tests check it on perturbed boards up to n = 250 and W = 4.  Returns
/// the promotions tried and how many could be made.
fn single_promotions_agree_with_the_peel<const W: usize>(n: u32, rng: &mut Rng) -> (u64, u64) {
    let game = Game::<W>::of_board(n).expect("a game whose root frames are winnable");
    let mut scratch = Scratch::<W>::new();
    let (mut tried, mut made) = (0, 0);
    for _ in 0..6 {
        let (mut state, mut promoted) = (game.root, Bits::EMPTY);
        let percent = rng.pick(&[2u64, 5, 10, 25]);
        for x in game.candidates.iter_desc() {
            if rng.chance(percent) && game.model.promote_in_place(&mut state, x) {
                promoted.insert(x);
            }
        }
        let at_once = game.model.promote_all_using(&mut scratch, &game.root, &promoted);
        assert!(at_once == Some(state), "n={n}: {promoted:?} one by one and at once differ");
        for x in (game.candidates - promoted).iter() {
            let peeled = game.model.promote_all_using(&mut scratch, &state, &Bits::EMPTY.with(x));
            assert!(game.model.promote(&state, x) == peeled, "n={n}: promoting {x} after {promoted:?}");
            tried += 1;
            made += u64::from(peeled.is_some());
        }
    }
    (tried, made)
}

#[test]
fn single_promotions_agree_with_the_peel_at_wide_widths() {
    let mut rng = Rng(0x5107_C0DE_0000_0001);
    let counts = [
        single_promotions_agree_with_the_peel::<20>(1133, &mut rng),
        single_promotions_agree_with_the_peel::<28>(1600, &mut rng),
        single_promotions_agree_with_the_peel::<64>(2050, &mut rng),
        single_promotions_agree_with_the_peel::<64>(3000, &mut rng),
    ];
    let (tried, made) = counts.iter().fold((0, 0), |(t, m), (a, b)| (t + a, m + b));
    assert!(tried > 3000 && made > 500, "{tried} promotions tried, {made} made");
}

// ---------------------------------------------------------------------------------------------------------
// Deep in the tree of a large game

/// The units a node-limited search of game 1215 leaves in its checkpoint (`testdata/n1215_deep_units.txt`; W = 32,
/// default tactics, 30,000 nodes, one thread, from game 1214 of `strict.jsonl`): the levels of its depth-first
/// stack, from the root to 34 promotions deep.  Returns the checkpoint's incumbent and the units.
///
/// To make such a file again, run `taxman solve 1215 --use-prev --threads 1 --node-limit 30000 --out R
/// --checkpoint-dir D` (`R` a new file) and list the
/// members of each unit of `D/n1215.ckpt` (a unit is its promoted set then its candidates, `W` little-endian words
/// each; the engine's layout is described in `engine.rs`).
fn deep_units() -> (u64, Vec<TaxUnit<32>>) {
    let text = std::fs::read_to_string(testdata("n1215_deep_units.txt")).unwrap();
    let mut lines = text.lines().filter(|l| !l.starts_with('#'));
    let best = lines.next().and_then(|l| l.strip_prefix("best ")).unwrap().parse().unwrap();
    let set = |text: &str| text.split_whitespace().map(|x| x.parse::<u32>().unwrap()).collect::<Bits<32>>();
    let units = lines.map(|l| l.split_once('|').unwrap()).map(|(a, b)| TaxUnit { promoted: set(a), cands: set(b) });
    (best, units.collect())
}

/// Along the path of a real search to 34 promotions deep, the incremental matching bound and the quick bounds
/// without a member equal the min-cost-flow reference (the unit tests walk only ~10 deep, at n <= 760), and the
/// subtrees of the deep levels give the same answer under every tactic: states and conflicts that no search of
/// the unit tests' size reaches.
#[test]
fn deep_levels_of_a_large_game() {
    let (best, units) = deep_units();
    assert!(units.len() >= 20, "{} units", units.len());
    let game = Game::<32>::of_board(1215).unwrap();
    let graph = MatchGraph::new(&game.model, &game.root);
    let (mut fresh, mut incremental) = (MatchBound::new(&graph), MatchBound::new(&graph));
    let mut scratch = Scratch::new();
    let mut previous: Option<TaxUnit<32>> = None;
    let deepest = units.iter().map(|u| u.promoted.len()).max().unwrap();
    assert!(deepest >= 25, "deepest level has {deepest} promotions");
    for (k, unit) in units.iter().enumerate() {
        let state = game.model.promote_all_using(&mut scratch, &game.root, &unit.promoted).expect("promotable");
        let reference = match_bound(&game.model, &state, &unit.cands);
        fresh.reset(&unit.promoted, &unit.cands);
        assert_eq!(fresh.value(), reference, "level {k} from scratch");
        for x in fresh.chosen().iter().take(4) {
            let (quick, exact) = (fresh.bound_excluding(x), fresh.value_excluding(x));
            fresh.pop();
            assert_eq!(exact, match_bound(&game.model, &state, &unit.cands.without(x)), "level {k} without {x}");
            assert!(exact <= quick && quick <= reference, "level {k}: {exact} <= {quick} <= {reference}");
        }
        match previous {
            Some(before) if before.promoted.is_subset(&unit.promoted) => {
                for x in (incremental.candidates() - before.cands).iter() {
                    incremental.remove_candidate(x);
                }
                incremental.push_all(&(unit.promoted - before.promoted), &unit.cands);
            }
            _ => incremental.reset(&unit.promoted, &unit.cands),
        }
        assert_eq!(incremental.value(), reference, "level {k} incrementally");
        previous = Some(*unit);
    }

    // The subtrees below the deepest levels (milliseconds each), searched from the checkpoint's incumbent and a
    // little below it, must end with the same incumbent under every tactic.
    let search_unit = |tactics: &SearchTactics, unit: TaxUnit<32>, start: u64| {
        let engine = EngineConfig { time_limit: Some(Duration::from_secs(5)), ..config(1) };
        let out = game.search(Config::of(tactics), vec![unit], start, Some(engine), None);
        assert!(out.completed && (out.best == start || out.real), "{tactics:?}: {out:?}");
        out.best
    };
    let mut compared = 0;
    for unit in units.iter().filter(|u| u.promoted.len() >= 13).take(8) {
        for start in [best, best - 25] {
            let all = variants();
            let expected = search_unit(&all[0].1, *unit, start);
            for (name, tactics) in &all[1..] {
                assert_eq!(search_unit(tactics, *unit, start), expected, "{name}: unit of {}", unit.promoted.len());
            }
            compared += 1;
        }
    }
    assert!(compared >= 10, "{compared}");
}
