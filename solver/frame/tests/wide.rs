//! Large boards without large searches.
//!
//! The Java stopped at n = 1000, and the sets that hold the numbers of a board are 8 words wide up
//! to n = 511, then 16, 20, 24, 28, 32, 36, 40, 48 and 56 words up to 1023, 1279, 1535, 1791, 2047, 2303, 2559, 3071
//! and 3583, 64 up to 4095 and 128 up to 8191 (`bits::WIDTHS`), so the boards above n = 1023 meet few tests besides
//! these: `tests/board_layer.rs` and `tests/golden_frames.rs` (to 4900 and 2049) and the committed results
//! (`results/strict.jsonl`, n = 1..4900; `optimal.json` has its games to 1300, but that is the solver's own answer and
//! no check of it).  What is cheap to run here: the same small game at every
//! width that holds it (a width-dependent bug changes the tree), games of 1100 to 1250 that need a few thousand
//! nodes against the committed results, boards of 2048 and more through their setup, and the boards past 4095
//! (128 words, and the GF(2) bound's engines of four and five block words) through theirs.

mod common;

use common::{
    config, matching_options, options, prev_of, profile_at_width, random_subset, stored_games, testdata, variants,
    Config, Game, Profile, Rng, TempDir,
};
use std::time::Duration;
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::EngineConfig;
use taxman::frames::build_frames;
use taxman::gf2::{Gf2Bound, Gf2Graph, MAX_WORDS};
use taxman::matchbound::{match_bound, MatchBound, MatchGraph};
use taxman::model::{FrameGraph, Scratch, MAX_FRAMES};
use taxman::numtheory::Tables;
use taxman::sacrifice::{sacrifice_bound, PuncturedOptions};
use taxman::search::{SearchTactics, TaxUnit};
use taxman::solver::{self, Method, Prev, PrevSource, SolveOptions};
use taxman::{accelerate, game, with_width};

// ---------------------------------------------------------------------------------------------------------
// The width does not matter

/// The same game, built at every width that holds it, has the same frames, candidates, greedy set, `max_total`,
/// search and tree (nodes counted).  A bug that depends on the width (a loop over fewer words than the set has, a
/// mask that is wrong at the top, a width that is not a power of two) shows as a different tree: the widths above
/// the ones the boards below need are otherwise reached by no test.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is the_search_of_a_small_board_does_not_depend_on_the_width"
)]
fn the_search_does_not_depend_on_the_width() {
    let default = SearchTactics::default();
    // Boards that W = 8 holds, at every width.
    small_boards_at_every_width(&[300, 400]);
    // Boards that W = 16 holds, with trees of more than 1,000 nodes: on the matching path and with the GF(2) bound.
    let matching = SearchTactics::matching();
    for (n, tactics) in [(760, matching), (1000, default)] {
        let at16 = profile_at_width::<16>(n, tactics, None);
        assert!(at16.completed && at16.nodes > 1000, "n={n}: {} nodes", at16.nodes);
        assert_eq!(profile_at_width::<32>(n, tactics, None), at16, "n={n} at W=32");
        assert_eq!(profile_at_width::<64>(n, tactics, None), at16, "n={n} at W=64");
        assert_eq!(profile_at_width::<128>(n, tactics, None), at16, "n={n} at W=128");
    }

    // Boards that need W = 20: 1133 on the matching path, and 1050 with the GF(2) bound (which searches 1133 in a
    // hundred nodes).
    for (n, name, tactics) in [(1133, "matching", matching), (1050, "default", default)] {
        let at20 = profile_at_width::<20>(n, tactics, None);
        assert!(at20.completed && at20.nodes > 500, "{name}: n={n}: {} nodes", at20.nodes);
        assert_eq!(profile_at_width::<24>(n, tactics, None), at20, "{name}: n={n} at W=24");
        assert_eq!(profile_at_width::<32>(n, tactics, None), at20, "{name}: n={n} at W=32");
        assert_eq!(profile_at_width::<64>(n, tactics, None), at20, "{name}: n={n} at W=64");
        assert_eq!(profile_at_width::<128>(n, tactics, None), at20, "{name}: n={n} at W=128");
    }
}

/// [`the_search_does_not_depend_on_the_width`] for the boards `boards` that W = 8 holds, at every width: each
/// completes in the same tree at all twelve.
fn small_boards_at_every_width(boards: &[u32]) {
    let default = SearchTactics::default();
    for &n in boards {
        let at8 = profile_at_width::<8>(n, default, None);
        assert!(at8.completed && at8.sum > 0, "n={n}");
        let others = [
            (16, profile_at_width::<16>(n, default, None)),
            (20, profile_at_width::<20>(n, default, None)),
            (24, profile_at_width::<24>(n, default, None)),
            (28, profile_at_width::<28>(n, default, None)),
            (32, profile_at_width::<32>(n, default, None)),
            (36, profile_at_width::<36>(n, default, None)),
            (40, profile_at_width::<40>(n, default, None)),
            (48, profile_at_width::<48>(n, default, None)),
            (56, profile_at_width::<56>(n, default, None)),
            (64, profile_at_width::<64>(n, default, None)),
            (128, profile_at_width::<128>(n, default, None)),
        ];
        for (w, profile) in others {
            assert_eq!(profile, at8, "n={n} at W={w}");
        }
    }
}

/// [`the_search_does_not_depend_on_the_width`] for the board 300 alone.
#[test]
fn the_search_of_a_small_board_does_not_depend_on_the_width() {
    small_boards_at_every_width(&[300]);
}

/// What a search starts from at width `W`, before its first node: the frames, candidates, greedy set and `max_total`
/// of the board `n` (a [`Profile`] without the search, which costs seconds on the boards past 2047: the root's
/// exact solves).
#[derive(Debug, PartialEq, Eq)]
struct Setup {
    columns: Vec<Vec<u32>>,
    candidates: Vec<u32>,
    greedy: Vec<u32>,
    max_total: u32,
}

fn setup_at_width<const W: usize>(n: u32) -> Setup {
    let board = Board::<W>::for_n(n);
    let prepared = board.prepare(SearchTactics::default()).unwrap();
    Setup {
        columns: board.frames.columns(),
        candidates: board.candidates().to_vec(),
        greedy: prepared.greedy.to_vec(),
        max_total: prepared.max_total,
    }
}

/// The last board a width holds (511, 1023, 1279, 1535, 1791, 2047, 2303, 2559, 3071, 3583, 4095) fills the top bit
/// of its set: the start of every search (frames, candidates, greedy set, `max_total`; up to 2047 also the first
/// nodes) is the same at the width the board dispatches to and at the next one.  (The boards just above 511, 1023 and
/// 2047 are in `testdata/golden_wide` at their own widths.)
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is the_setups_at_the_edges_of_the_widths_hold")]
fn the_edges_of_the_widths_hold() {
    let default = SearchTactics::default();
    let limit = Some(1);
    let check = |profiles: [Profile; 2], n: u32| assert_eq!(profiles[0], profiles[1], "n={n}");
    check([profile_at_width::<8>(511, default, limit), profile_at_width::<16>(511, default, limit)], 511);
    check([profile_at_width::<16>(1023, default, limit), profile_at_width::<20>(1023, default, limit)], 1023);
    check([profile_at_width::<20>(1279, default, limit), profile_at_width::<24>(1279, default, limit)], 1279);
    check([profile_at_width::<24>(1535, default, limit), profile_at_width::<28>(1535, default, limit)], 1535);
    check([profile_at_width::<28>(1791, default, limit), profile_at_width::<32>(1791, default, limit)], 1791);
    check([profile_at_width::<32>(2047, default, limit), profile_at_width::<36>(2047, default, limit)], 2047);
    the_setups_at_the_edges_of_the_widths_hold();
}

/// [`the_edges_of_the_widths_hold`] for the setup of every edge (the frames, candidates, greedy set and `max_total`,
/// without the first nodes of a search, which take most of the time of the boards past 1023).
#[test]
fn the_setups_at_the_edges_of_the_widths_hold() {
    let check_setup = |setups: [Setup; 2], n: u32| assert_eq!(setups[0], setups[1], "n={n}");
    check_setup([setup_at_width::<8>(511), setup_at_width::<16>(511)], 511);
    check_setup([setup_at_width::<16>(1023), setup_at_width::<20>(1023)], 1023);
    check_setup([setup_at_width::<20>(1279), setup_at_width::<24>(1279)], 1279);
    check_setup([setup_at_width::<24>(1535), setup_at_width::<28>(1535)], 1535);
    check_setup([setup_at_width::<28>(1791), setup_at_width::<32>(1791)], 1791);
    check_setup([setup_at_width::<32>(2047), setup_at_width::<36>(2047)], 2047);
    check_setup([setup_at_width::<36>(2303), setup_at_width::<40>(2303)], 2303);
    check_setup([setup_at_width::<40>(2559), setup_at_width::<48>(2559)], 2559);
    check_setup([setup_at_width::<48>(3071), setup_at_width::<56>(3071)], 3071);
    check_setup([setup_at_width::<56>(3583), setup_at_width::<64>(3583)], 3583);
    check_setup([setup_at_width::<64>(4095), setup_at_width::<128>(4095)], 4095);
}

/// A board at each of the widths of 36, 40, 48 and 56 words (2050, 2400, 2800 and 3100 here; the boards 2048 to 3583
/// dispatch to them) gives the same frames, candidates, greedy set, `max_total` and the same node-limited search on
/// one thread (its nodes, best and promotions) at its own width and at 64, the width all of them had before the four
/// were added; 2050 also at 128, the widest.  The trees of these boards are far larger than the limit.  (The root's
/// exact solves make the first 1,023 nodes of a search cost 0.6 s at 2050, 3 s at 3100 and 7 s at 3300, so the boards
/// stay low in their widths, and only the cheapest is searched at 128 too.)
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is the_setups_of_boards_past_2047_are_the_same_at_their_width_at_64_and_at_128"
)]
fn boards_past_2047_give_the_same_search_at_their_width_at_64_and_at_128() {
    let default = SearchTactics::default();
    let limit = Some(2000);
    for (n, width) in [(2050, 36), (2400, 40), (2800, 48), (3100, 56)] {
        assert_eq!(with_width!(n, W => W), width, "n={n}: the width the board dispatches to");
        let at_its_width = with_width!(n, W => profile_at_width::<W>(n, default, limit));
        assert!(!at_its_width.completed && at_its_width.nodes >= 2000, "n={n}: {} nodes", at_its_width.nodes);
        assert!(at_its_width.candidates.len() > 500 && at_its_width.sum > 0, "n={n}: {at_its_width:?}");
        assert_eq!(profile_at_width::<64>(n, default, limit), at_its_width, "n={n} at W=64 and W={width}");
        if n == 2050 {
            assert_eq!(profile_at_width::<128>(n, default, limit), at_its_width, "n={n} at W=128 and W={width}");
        }
    }
}

/// [`boards_past_2047_give_the_same_search_at_their_width_at_64_and_at_128`] without the search: the setup (frames,
/// candidates, greedy set and `max_total`) of each board, which costs a fiftieth of the first nodes.
#[test]
fn the_setups_of_boards_past_2047_are_the_same_at_their_width_at_64_and_at_128() {
    for (n, width) in [(2050, 36), (2400, 40), (2800, 48), (3100, 56)] {
        assert_eq!(with_width!(n, W => W), width, "n={n}: the width the board dispatches to");
        let at_its_width = with_width!(n, W => setup_at_width::<W>(n));
        assert!(at_its_width.candidates.len() > 500 && at_its_width.max_total > 0, "n={n}: {at_its_width:?}");
        assert_eq!(setup_at_width::<64>(n), at_its_width, "n={n} at W=64 and W={width}");
        if n == 2050 {
            assert_eq!(setup_at_width::<128>(n), at_its_width, "n={n} at W=128 and W={width}");
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// Games of 1100 and more, against the committed results

/// Game 1133 (W = 20, the width of the hardest games) searches in a few thousand nodes: every sound tactic must
/// give the score of `results/strict.jsonl`, on one thread, and the default also on two.  Nothing else searches at
/// a width of 20 or more and checks the answer.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is some_tactics_give_the_recorded_score_at_w20")]
fn every_tactic_gives_the_recorded_score_at_w20() {
    tactics_give_the_recorded_score_at_w20(1);
}

/// [`every_tactic_gives_the_recorded_score_at_w20`] for every fourth of the variants (the default among them).
#[test]
fn some_tactics_give_the_recorded_score_at_w20() {
    tactics_give_the_recorded_score_at_w20(4);
}

/// Every `every`th of the variants of the tactics gives the recorded score of game 1133.
fn tactics_give_the_recorded_score_at_w20(every: usize) {
    let games = stored_games("strict.jsonl");
    let prev = prev_of(&games, 1133);
    for (name, tactics) in variants().into_iter().step_by(every) {
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
        prev = Prev {
            moves: r.moves,
            score: r.score,
            source: PrevSource::Run,
            proof: r.proof,
            assumes: r.assumes,
            proven: true,
        };
    }
    assert!(searched >= 5, "{searched} of the 8 games were searched");
}

// ---------------------------------------------------------------------------------------------------------
// Boards of 2048 and more

/// A frame set holds `MAX_FRAMES` = 9 frames, which the largest boards have (every board up to 8191, the largest
/// supported, was checked: 6 frames from n = 512, 7 from 1024, 8 from 2048, 9 from 4096).  The boards at the steps,
/// and the largest.
#[test]
fn the_largest_boards_fit_in_a_frame_set() {
    assert_eq!(MAX_FRAMES, 9);
    for (n, frames) in [(511, 5), (512, 6), (1023, 6), (1024, 7), (2047, 7), (2048, 8), (4095, 8), (4096, 9), (8191, 9)]
    {
        let built = with_width!(n, W => build_frames::<W>(&Tables::new(n)).frames.len());
        assert_eq!(built, frames, "n={n}");
    }
}

/// Board `n`'s frames, model, matching bound, conflicts, greedy set and a short search, built at width `W`, for a
/// board with numbers above `above` in its frames (so the numbers that the width before `W` could not hold are in
/// the model, the matching graph and the search).
fn builds_bounds_and_searches<const W: usize>(n: u32, above: u32) {
    let tables = Tables::new(n);
    let frames = build_frames::<W>(&tables);
    let (model, root) = FrameGraph::build::<W>(&tables, &frames.columns()).unwrap();
    assert!(frames.candidates.len() > 500);
    assert!(frames.frames.iter().any(|f| (f.moves | f.factors).max().is_some_and(|m| m > above)));

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
    // a search at this width.  (`tests/solver.rs` runs the whole pipeline's setup and first nodes for boards of
    // 3600 and 4100.)
    let game = Game::<W>::new(model, root, frames.candidates);
    let engine = EngineConfig { node_limit: Some(1), ..config(1) };
    let out = game.search(Config::standard(), vec![game.root_unit()], greedy.sum(), Some(engine), None);
    assert!(!out.completed && out.nodes > 0 && out.best >= greedy.sum());
    assert!(out.best == greedy.sum() || out.real, "{out:?}");
}

/// The first board with a number above 2047 is n = 2048 (dispatched to more than 32 words whether or not a frame
/// holds one); n = 2050 has such numbers in its frames.  Its frames, model, matching bound, conflicts, greedy set,
/// setup and a short search, built at W = 64 although it dispatches to 36: the widest set up to board 4095, which
/// only the boards from 3584 to 4095 dispatch to, runs here on a board the tests can afford (`tests/solver.rs` runs
/// the pipeline's own dispatch on a board of 3600).
#[test]
fn a_board_past_2047_builds_bounds_and_searches_at_w64() {
    builds_bounds_and_searches::<64>(2050, 2047);
}

/// The same for n = 4100 at W = 128, the set width of every board from 4096 up: a board with numbers above 4095 in
/// its frames, which no narrower set holds.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is a_board_past_2047_builds_bounds_and_searches_at_w64")]
fn a_board_past_4095_builds_bounds_and_searches_at_w128() {
    builds_bounds_and_searches::<128>(4100, 4095);
}

/// At W = 20 the sacrifice bound (a search of punctured boards, which is the theorem that opt(n) is at most the
/// best of them plus a known term) is the only upper bound that does not come from the search being tested (the
/// Java stops at 1023): it must be at least the recorded score, and where it is tight it proves it, as it does for
/// game 1133 (not for game 1131, whose bound is larger).
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
// Boards past 4095

/// The search of `game` from its root, with the default tactics on one thread and the GF(2) graph padded to `words`
/// block words (if any), started from the greedy sum: how it ends (best, the set that reaches it, whether it
/// completed) and the nodes of its tree.
fn root_search<const W: usize>(game: &Game<W>, words: Option<usize>) -> (u64, Vec<u32>, bool, u64) {
    let greedy = taxman::bounds::greedy(&game.model, &game.root, &game.candidates).sum();
    let config = Config { gf2_words: words, ..Config::standard() };
    let out = game.search(config, vec![game.root_unit()], greedy, None, None);
    assert!(out.best == greedy || out.real, "{out:?}");
    (out.best, out.set.to_vec(), out.completed, out.nodes)
}

/// The GF(2) bound's engines of four and five words search the same tree as the engine a board needs.  The engines of
/// four and five words are only for the boards of 4740 and more, which no test can search; game 1000 (frames of one
/// block word, a tree of thousands of nodes) is searched from its root with its graph padded to four and to five
/// words at W = 128 (the only width that has them), and must end with the same best, set and nodes as at W = 16 and
/// as at W = 128 with its own one word.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is the_first_board_past_4095_is_set_up_at_128_words")]
fn the_gf2_engines_of_four_and_five_words_search_the_same_tree() {
    assert!(Config::standard().gf2);
    let expected = root_search(&Game::<16>::of_board(1000).unwrap(), None);
    assert!(expected.2 && expected.3 > 1000, "{} nodes", expected.3);
    let wide = Game::<128>::of_board(1000).unwrap();
    for words in [None, Some(4), Some(5)] {
        assert_eq!(root_search(&wide, words), expected, "at W=128 with {words:?} words");
    }
}

/// Boards past 4095 are set up at 128 words, the widest, and agree with the board layer's own invariants, at the
/// first boards, where the frames need three block words of GF(2) vectors, at 4740 and 6000 (four) and from 6392
/// (five), and the largest.  The board has nine frames, whose numbers are all on it; the game with no promotions is
/// legal and finished and scores what the board says; the setup succeeds, every root frame has as many freedoms as
/// factors beyond its moves, and the greedy set is made of candidates and can be promoted.  The greedy set and random
/// subsets of it (which can be promoted too) turn into games that are legal, finished, open with the largest prime and
/// score the plain game's score plus the promotions' sum.  The root's GF(2) bound is at least the greedy sum and at
/// most the matching bound, and an engine of five words gives the same as the one the board needs.  (The setup's
/// local search is left out: this is the board layer's invariants, not the solver's.)
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is the_first_board_past_4095_is_set_up_at_128_words")]
fn boards_past_4095_are_set_up_at_128_words() {
    boards_are_set_up_at_128_words(&[(4096, 3), (4100, 3), (4740, 4), (6000, 4), (6392, 5), (8191, 5)]);
}

/// [`boards_past_4095_are_set_up_at_128_words`] for the first board past 4095.
#[test]
fn the_first_board_past_4095_is_set_up_at_128_words() {
    boards_are_set_up_at_128_words(&[(4096, 3)]);
}

/// The boards `boards` (each with the block words its frames' GF(2) vectors need) are set up at 128 words and agree
/// with the board layer's invariants.
fn boards_are_set_up_at_128_words(boards: &[(u32, usize)]) {
    let mut rng = Rng(0x4096_8191_0000_0001);
    for &(n, words) in boards {
        assert_eq!(with_width!(n, W => W), 128, "n={n}");
        let board = Board::<128>::for_n(n);
        let columns = board.frames.columns();
        assert_eq!(columns.len() / 2, 9, "n={n}: frames");
        assert!(!board.candidates().is_empty(), "n={n}: no candidates");
        assert!(columns.iter().flatten().all(|x| (1..=n).contains(x)), "n={n}: a number off the board");
        assert_eq!(game::replay_finished(n, &board.plain), Ok(board.in_the_bag), "n={n}: the plain game");

        let p = board.prepare(SearchTactics::default()).unwrap();
        assert_eq!(p.root.len(), 9, "n={n}");
        for frame in p.root.frames() {
            assert_eq!(frame.freedoms, frame.factors.len() as i32 - frame.moves.len() as i32, "n={n}: {frame:?}");
            assert!(frame.freedoms >= 0, "n={n}: {frame:?}");
        }
        assert!(p.greedy.is_subset(&p.candidates), "n={n}: the greedy set is not made of candidates");
        let greedy = p.greedy.sum();
        assert!(greedy > 0, "n={n}");
        let mut scratch = Scratch::<128>::new();
        let subsets = [random_subset(&mut rng, &p.greedy, 50), random_subset(&mut rng, &p.greedy, 10)];
        for set in std::iter::once(p.greedy).chain(subsets) {
            assert!(p.model.can_promote_all_using(&mut scratch, &p.root, &set), "n={n}: {set:?} cannot be promoted");
            let moves = accelerate::solution_for_promotions::<128>(&board.tables, set);
            assert_eq!(game::replay_finished(n, &moves), Ok(board.in_the_bag + set.sum()), "n={n}: {set:?}");
            assert_eq!(moves.first().copied(), game::largest_prime_at_most(n), "n={n}: {set:?}");
        }

        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        assert_eq!(graph.words(), words, "n={n}: the largest frame rank is {:?}", graph.ranks().iter().max());
        let root_bound = |mut bound: Gf2Bound<128>| {
            bound.reset(&Bits::EMPTY, &p.candidates);
            bound.evaluate(p.max_total, None).value
        };
        let value = root_bound(Gf2Bound::new(&graph));
        let matching = match_bound(&p.model, &p.root, &p.candidates);
        assert!(greedy <= value && value <= matching, "n={n}: {greedy} <= {value} <= {matching}");
        assert_eq!(root_bound(Gf2Bound::with_words(&graph, MAX_WORDS)), value, "n={n}: with {MAX_WORDS} words");
    }
}

// ---------------------------------------------------------------------------------------------------------
// Resuming

/// `opts` with the window of `force_window` that the headers of versions 4 and 5 imply (80; the default is `all`
/// now), as a run resumes a checkpoint of the builds before the default changed.
fn old_window(mut opts: SolveOptions) -> SolveOptions {
    opts.tactics.set("force_window=80").unwrap();
    opts
}

/// A checkpoint that an earlier build wrote (`testdata/n1133_after_1000_nodes.ckpt`: game 1133 at W = 20 with
/// the default tactics of that build, now those of the matching path (`SearchTactics::matching`), stopped after
/// 1,023 nodes on one thread; search header format TXSH v5) still resumes and
/// finishes with the committed score.  Every write/read test of the format uses the code's own constants, so a
/// change of the layout that round-trips (the version, the byte order of a set, the order of the header's fields,
/// the packing of the tactics) passes them and refuses this file.  Its header implies the window of 80 for the forced
/// members, which the default (`all`) is not, so the run says `--tactic force_window=80`.
///
/// A deliberate change of the format (a new header version, say, which refuses older checkpoints) fails this test.
/// To make the file again with the new build: `taxman solve 1133 --use-prev --threads 1 --node-limit 1000 --out R
/// --checkpoint-dir D --tactic gf2=off --tactic lagrange=on --tactic symmetry=off` with a new file `R`, then copy
/// `D/n1133.ckpt` over the fixture.
#[test]
fn a_committed_checkpoint_resumes() {
    let games = stored_games("strict.jsonl");
    let dir = TempDir::new("committed-checkpoint");
    let opts = old_window(SolveOptions { checkpoint_dir: Some(dir.join("ckpt")), ..matching_options(1) });
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

/// A checkpoint of the format before the extended problem's search (`testdata/n1133_after_1000_nodes_v4.ckpt`: the
/// same search as the fixture above, written by the build of commit 2e8b851, whose headers are version 4, with no digest
/// of the one-sided candidates) resumes with this build and finishes as the build that wrote it finishes it: the same
/// score, two invocations and 3,058 nodes in all (the count that build's binary gave, finishing the same file).  That
/// build had no near-repeat cut, which the game of 1132 makes end the search at once, so the run turns it off.  Its
/// header reads as a version-5 header with no one-sided candidates; asked to resume a search of the extended problem
/// with open sources instead, it is refused, saying why.
#[test]
fn a_committed_checkpoint_of_format_4_resumes_as_the_build_that_wrote_it_finishes_it() {
    let games = stored_games("strict.jsonl");
    let dir = TempDir::new("committed-checkpoint-v4");
    let mut opts = old_window(SolveOptions { checkpoint_dir: Some(dir.join("ckpt")), ..matching_options(1) });
    opts.tactics.set("near_repeat=off").unwrap();
    let checkpoint = opts.checkpoint_file(1133).unwrap();
    std::fs::create_dir_all(checkpoint.parent().unwrap()).unwrap();
    std::fs::copy(testdata("n1133_after_1000_nodes_v4.ckpt"), &checkpoint).unwrap();
    let version = std::fs::read(&checkpoint).unwrap()[24..28].to_vec();
    assert_eq!(version, 4u32.to_le_bytes(), "the fixture's header is of version 4");

    // A search of the extended problem with open sources cannot be this checkpoint's, and is refused, saying why.
    let board = Board::<20>::for_n(1133);
    let open: Bits<20> = board.plain.iter().copied().filter(|&x| 2 * x > 1133 && board.tables.omega(x) >= 3).collect();
    let refused = board.prepare_extended_for_resume(&open, false, &checkpoint).err().expect("refused");
    assert!(refused.contains("format version 4") && refused.contains("one-sided candidates"), "{refused}");

    let r = solver::solve_game(1133, Some(&prev_of(&games, 1133)), &opts).unwrap();
    assert!(r.completed && r.method == Method::Search);
    assert_eq!(r.score, games[&1133].0);
    assert_eq!((r.invocations, r.nodes), (2, 3058), "the build that wrote the checkpoint finishes it so");
    assert!(r.assumes.is_empty(), "{:?}", r.assumes);
    assert!(!checkpoint.exists(), "a finished game removes its checkpoint");
}

/// A W = 20 search interrupted every few hundred nodes and resumed from its checkpoint, under two sets of tactics
/// of the matching path (the tactics are part of the checkpoint's header and must read back as written; the GF(2)
/// bound's search of 1133 is too short to stop, and `tests/soundness.rs` resumes game 992 under it at W = 20, 24 and
/// 64).  The unit tests resume only at W = 16.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is a_w20_search_interrupted_and_resumed_once_gives_the_same_answer"
)]
fn a_w20_search_interrupted_and_resumed_gives_the_same_answer() {
    a_w20_search_interrupted_and_resumed_under(2);
}

/// [`a_w20_search_interrupted_and_resumed_gives_the_same_answer`] under the first set of tactics alone.
#[test]
fn a_w20_search_interrupted_and_resumed_once_gives_the_same_answer() {
    a_w20_search_interrupted_and_resumed_under(1);
}

/// The W = 20 search of game 1133 interrupted and resumed gives the answer, under the first `sets` sets of tactics of
/// the matching path, without the near-repeat cut, by which the game of 1132 ends the search at its first step.
fn a_w20_search_interrupted_and_resumed_under(sets: usize) {
    let games = stored_games("strict.jsonl");
    let prev = prev_of(&games, 1133);
    for (name, tactics) in variants().into_iter().filter(|(name, _)| name.starts_with("matching")).take(sets) {
        let dir = TempDir::new(&format!("resume-{}", name.replace(['=', ','], "-")));
        let mut opts = options(1);
        opts.tactics.search = SearchTactics { near_repeat: false, ..tactics };
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

/// Promoting one number at a time (`FrameGraph::promote`, which skips the peel when the move has a factor no move of
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
/// the unit tests' size reaches.  The last comparison is made among the variants without the rule of the tactic
/// `symmetry` and among those with it: a search of a unit below the root with the rule finds the best completion
/// that keeps the symmetric pairs' inequalities, which can be lighter than the unit's best.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is incremental_solver_matches_the_reference_on_a_sample")]
fn deep_levels_of_a_large_game() {
    deep_levels_of_a_large_game_below(8);
}

/// [`deep_levels_of_a_large_game`] with the subtrees below 2 units searched under every tactic.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is incremental_solver_matches_the_reference_on_a_sample")]
fn deep_levels_of_a_large_game_below_a_few_units() {
    deep_levels_of_a_large_game_below(2);
}

/// The deep levels of game 1215, and the subtrees below `subtrees` of its deepest units searched under every tactic.
fn deep_levels_of_a_large_game_below(subtrees: usize) {
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
    // little below it, must end with the same incumbent under every tactic, with the rule of `symmetry` off and on
    // in turn (with it a search of a unit below the root finds the best completion that keeps the symmetric pairs'
    // inequalities, which can be lighter than the unit's best).
    let search_unit = |tactics: &SearchTactics, unit: TaxUnit<32>, start: u64| {
        let engine = EngineConfig { time_limit: Some(Duration::from_secs(5)), ..config(1) };
        let out = game.search(Config::of(tactics), vec![unit], start, Some(engine), None);
        assert!(out.completed && (out.best == start || out.real), "{tactics:?}: {out:?}");
        out.best
    };
    let mut compared = 0;
    for rule in [false, true] {
        let mut all: Vec<(&str, SearchTactics)> = Vec::new();
        for (name, tactics) in variants() {
            let tactics = SearchTactics { symmetry: rule, ..tactics };
            if all.iter().all(|(_, seen)| *seen != tactics) {
                all.push((name, tactics));
            }
        }
        for unit in units.iter().filter(|u| u.promoted.len() >= 13).take(subtrees) {
            for start in [best, best - 25] {
                let expected = search_unit(&all[0].1, *unit, start);
                for (name, tactics) in &all[1..] {
                    let what = format!("{name}, symmetry {rule}: unit of {}", unit.promoted.len());
                    assert_eq!(search_unit(tactics, *unit, start), expected, "{what}");
                }
                compared += 1;
            }
        }
    }
    assert!(compared >= 4 * subtrees, "{compared}");
}
