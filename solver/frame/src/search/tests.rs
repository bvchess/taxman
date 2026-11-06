use super::header::Header;
use super::*;
use crate::board::Board;
use crate::engine::Unit;
use crate::frames::build_frames;
use crate::json::Json;
use crate::model::test_support::*;
use crate::model::{FrameGraph, Scratch};
use crate::numtheory::Tables;
use crate::test_support::TempDir;
use std::thread;

fn config(threads: usize) -> EngineConfig {
    EngineConfig { threads, progress: None, ..EngineConfig::default() }
}

/// The golden games the Java had to search (not accelerated), up to `max_n`.
fn searched_games(max_n: u32) -> Vec<Golden> {
    golden_ns()
        .into_iter()
        .filter(|&n| n <= max_n)
        .map(load)
        .filter(|g| g.json.get("solve").and_then(|s| s.get("accelerated")).and_then(Json::as_bool) == Some(false))
        .collect()
}

/// The sum of the Java's optimal promotions.
fn golden_sum(g: &Golden) -> u64 {
    u32s(g.json.get("solve").expect("solve"), "promotions").iter().map(|&x| u64::from(x)).sum()
}

fn prepare_golden<const W: usize>(g: &Golden, tactics: SearchTactics) -> Prepared<W> {
    prepare::<W>(&Tables::new(g.n), &g.columns, &Bits::from_slice(&g.candidates), tactics).unwrap()
}

/// Searches and checks that the result is complete, made of candidates, and promotable.
fn search_checked<const W: usize>(p: &Prepared<W>, threads: usize, what: &str) -> SearchResult<W> {
    let r = search(p, &config(threads), None, None, false).unwrap();
    assert!(r.completed, "{what}: incomplete");
    assert!(r.promotions.is_subset(&p.candidates), "{what}: {:?} are not all candidates", r.promotions);
    let promotable = p.model.promote_all_using(&mut Scratch::new(), &p.root, &r.promotions);
    assert!(promotable.is_some(), "{what}: {:?} cannot be promoted", r.promotions);
    r
}

fn check_against_java<const W: usize>(g: &Golden) {
    let p = prepare_golden::<W>(g, SearchTactics::default());
    // The Java's frame set and `max_total` before its revision of frame 0, which this build does not have.
    let frames = g.json.get("frameSet").and_then(Json::as_arr).expect("frameSet");
    let frame0 = frames.first().and_then(|frame| frame.get("freedoms")).and_then(Json::as_i64);
    assert_eq!(p.root.frames().first().map(|f| i64::from(f.freedoms)), frame0, "n={} frame 0 freedoms", g.n);
    let max_total = g.json.get("maxPromotionsUnrevised").and_then(Json::as_i64).expect("maxPromotionsUnrevised");
    assert_eq!(i64::from(p.max_total), max_total, "n={} max_total", g.n);
    for threads in [1, 4] {
        let r = search_checked(&p, threads, &format!("n={} with {threads} threads", g.n));
        assert_eq!(r.sum, golden_sum(g), "n={} with {threads} threads: {:?}", g.n, r.promotions);
    }
}

#[test]
fn golden_games_up_to_400_match_the_java() {
    let games = searched_games(400);
    assert!(games.len() > 80, "only {} games", games.len());
    for g in &games {
        check_against_java::<8>(g);
    }
}

#[test]
fn golden_games_above_400_match_the_java() {
    let games: Vec<Golden> = golden_ns().into_iter().filter(|&n| n > 400).map(load).collect();
    assert_eq!(games.len(), 8);
    for g in &games {
        check_against_java::<16>(g);
    }
}

/// Every candidate of the frame builder, for the games `ns` (all with candidates, or none), can be promoted
/// from the root by itself.
fn candidates_can_be_promoted_from_the_root<const W: usize>(ns: impl Iterator<Item = u32>) {
    let mut scratch = Scratch::new();
    for n in ns {
        let tables = Tables::new(n);
        let frames = build_frames::<W>(&tables);
        let (model, root) = FrameGraph::build::<W>(&tables, &frames.columns()).unwrap();
        for x in frames.candidates.iter() {
            assert!(model.can_promote_using(&mut scratch, &root, x), "n={n}: candidate {x} cannot be promoted");
        }
    }
}

/// So a test of each candidate at the root would find nothing to drop (see solver/docs/design/search.md, "Forced
/// members and nogoods", the item "Single-member conflicts belong to the node"), and the single-member conflicts of the
/// search are relative to its nodes.  The cost is all in building the frames (10 to 20 ms a board above 500), so every
/// board to 511 is checked (0.2 s) and every sixth above (1.2 s); all of 2..=1230 takes 7 s.
#[test]
fn the_frame_builders_candidates_can_all_be_promoted_from_the_root() {
    candidates_can_be_promoted_from_the_root::<8>(2..=511);
    candidates_can_be_promoted_from_the_root::<16>((512..=1023).step_by(6));
    candidates_can_be_promoted_from_the_root::<32>((1024..=1230).step_by(6));
}

fn sums_by_tactics<const W: usize>(g: &Golden) -> Vec<(SearchTactics, u64)> {
    let default = SearchTactics::default();
    let variants = [
        default,
        SearchTactics { fix_forced: false, ..default },
        SearchTactics { nogoods: false, ..default },
        SearchTactics { forget_nogoods: false, ..default },
        SearchTactics { symmetry: false, ..default },
        SearchTactics { fix_forced: false, nogoods: false, symmetry: false, ..default },
    ];
    variants.iter().map(|&t| (t, search_checked(&prepare_golden::<W>(g, t), 1, "tactics").sum)).collect()
}

#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is tactics_do_not_change_answers_on_a_sample")]
fn tactics_do_not_change_answers() {
    tactics_do_not_change_answers_on_every(1);
}

/// [`tactics_do_not_change_answers`] on every fifth of its golden games.
#[test]
fn tactics_do_not_change_answers_on_a_sample() {
    tactics_do_not_change_answers_on_every(5);
}

/// No tactic changes the answers on every `every`th of the golden games.
fn tactics_do_not_change_answers_on_every(every: usize) {
    let mut differences = Vec::new();
    for g in searched_games(400).iter().step_by(every) {
        let sums = sums_by_tactics::<8>(g);
        for &(tactics, sum) in &sums[1..] {
            if sum != sums[0].1 {
                differences.push(format!("n={}: {tactics:?} gives {sum}, default {}", g.n, sums[0].1));
            }
        }
    }
    assert!(differences.is_empty(), "A TACTIC CHANGES ANSWERS:\n{}", differences.join("\n"));
}

/// The searches of these games are shorter than [`NOGOOD_PERIOD`] steps, so they never forget with it: forget
/// after every step, and after a few.
#[test]
fn forgetting_nogoods_often_does_not_change_answers() {
    for g in &searched_games(400) {
        let p = prepare_golden::<8>(g, SearchTactics::default());
        for (period, threads) in [(1, 1), (3, 2), (40, 1)] {
            let cfg = config(threads);
            let r = search_forgetting(&p, &cfg, None, None, false, Some(period), None, None, None, None, None, None)
                .unwrap();
            let what = format!("n={}, forgetting every {period} steps on {threads} threads", g.n);
            assert!(r.completed && r.promotions.is_subset(&p.candidates), "{what}");
            assert_eq!(r.sum, golden_sum(g), "{what}");
        }
    }
}

/// The default search (with the rule of the symmetric pairs, tactic `symmetry`, whose units hold the candidates it has
/// dropped from their nodes), and without the rule (whose tree is larger, so its limit is), stopped every few hundred
/// nodes and resumed from its checkpoint, on one thread and on two, until it finishes with game 992's optimum: its
/// searches of 760 are too short to interrupt (the engine checks the node limit every 1024 nodes of a searcher), and so
/// are those of 1000 on two threads since the window of the forced members became `all` (2,026 nodes).
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is checkpoint_and_resume_with_the_gf2_bound_of_a_small_game"
)]
fn checkpoint_and_resume_with_the_gf2_bound() {
    let default = SearchTactics::default();
    let cases = [(default, 1000), (SearchTactics { symmetry: false, ..default }, 1500)];
    resume_the_gf2_search(992, &cases, &[1, 2]);
}

/// [`checkpoint_and_resume_with_the_gf2_bound`] for game 1000 (2,026 nodes by the default) on one thread, stopped
/// every 1,000 nodes.
#[test]
fn checkpoint_and_resume_with_the_gf2_bound_of_a_small_game() {
    resume_the_gf2_search(1000, &[(SearchTactics::default(), 1000)], &[1]);
}

/// The search of game `n` under each of `cases` (tactics and the node limit that stops it), on each of `threads`,
/// stopped and resumed from its checkpoint until it finishes with P*.
fn resume_the_gf2_search(n: u32, cases: &[(SearchTactics, u64)], threads: &[usize]) {
    let board = crate::board::Board::<16>::for_n(n);
    let opt = crate::game::Optimal::built_in().score(n).unwrap() - board.in_the_bag;
    for &(tactics, node_limit) in cases {
        let p = board.prepare(tactics).unwrap();
        for &threads in threads {
            let what = format!("{threads} threads, symmetry {}", tactics.symmetry);
            let dir = TempDir::new(&format!("resume-gf2-{threads}-{}", tactics.bits()));
            let ckpt = dir.0.join("ckpt.bin");
            let cfg = EngineConfig {
                checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
                node_limit: Some(node_limit),
                queue_mem_cap: 4,
                spill_dir: Some(dir.0.join("spill")),
                ..config(threads)
            };
            let mut result = search(&p, &cfg, None, None, false).unwrap();
            let mut pieces = 1;
            while !result.completed {
                let resumed = board.prepare_for_resume(&ckpt).unwrap();
                assert_eq!(resumed.tactics, tactics, "{what}: the checkpoint records the tactics");
                result = search(&resumed, &cfg, None, None, true).unwrap();
                pieces += 1;
                assert!(pieces < 200, "no progress");
            }
            assert!(pieces >= 2, "{what}: only {pieces} pieces");
            assert_eq!(result.sum, opt, "{what}");
        }
    }
}

#[test]
fn a_checkpoint_of_another_game_is_refused() {
    let (a, b) = (load(760), load(720));
    let dir = TempDir::new("refuse");
    let ckpt = dir.0.join("ckpt.bin");
    let pa = prepare_golden::<16>(&a, SearchTactics::default());
    // A time limit of nothing stops the search before it starts, with its checkpoint.
    let checkpoint = Some((ckpt.clone(), Duration::from_secs(3600)));
    let cfg = EngineConfig { checkpoint, time_limit: Some(Duration::ZERO), ..config(1) };
    assert!(!search(&pa, &cfg, None, None, false).unwrap().completed);
    let cands = Bits::<16>::from_slice(&b.candidates);
    assert!(prepare_for_resume::<16>(&Tables::new(b.n), &b.columns, &cands, &ckpt).is_err());
    let cands = Bits::<32>::from_slice(&a.candidates);
    assert!(prepare_for_resume::<32>(&Tables::new(a.n), &a.columns, &cands, &ckpt).is_err());
}

#[test]
fn seeds_and_caps() {
    let g = load(300);
    let p = prepare_golden::<8>(&g, SearchTactics::default());
    let optimum = search_checked(&p, 1, "plain");
    // Seeded with an optimal set (plus a non-candidate), the search only has to prove it.
    let seed = optimum.promotions.with(299);
    let seeded = search(&p, &config(1), Some(&seed), None, false).unwrap();
    assert_eq!((seeded.sum, seeded.promotions), (optimum.sum, optimum.promotions));
    assert!(seeded.stats.nodes <= optimum.stats.nodes);
    // A ceiling the seed already reaches ends the search before it starts; one the search reaches ends it
    // there.
    let capped = search(&p, &config(1), Some(&seed), Some(optimum.sum), false).unwrap();
    assert!(capped.completed && capped.reached_cap && capped.stats.nodes == 0);
    let capped = search(&p, &config(1), None, Some(optimum.sum), false).unwrap();
    assert!(capped.completed && capped.reached_cap && capped.sum == optimum.sum);
    assert!(capped.stats.nodes <= optimum.stats.nodes);
    // A worse seed changes nothing.
    let worse = search(&p, &config(1), Some(&Bits::EMPTY), None, false).unwrap();
    assert_eq!(worse.sum, optimum.sum);
}

/// A start that reaches the ceiling is the incumbent as it stands: the search skips the dive and the local search,
/// which could not have changed it, and ends before it starts, with that start.
#[test]
fn a_start_at_the_ceiling_is_neither_dived_nor_improved() {
    let g = load(300);
    let p = prepare_golden::<8>(&g, SearchTactics::default());
    let optimum = search_checked(&p, 1, "plain");
    let graph = MatchGraph::new(&p.model, &p.root);
    let hood = Neighborhood { model: &p.model, root: &p.root, candidates: &p.candidates };

    // The seed (an optimal set, plus a non-candidate) reaches the ceiling, which P* is: the start is the better of
    // the greedy set and the seed's survivors, and the dive and the local search leave it as it is.
    let seed = optimum.promotions.with(299);
    let kept = surviving(&p, &seed);
    let begun = if kept.sum() > p.greedy.sum() { kept } else { p.greedy };
    assert_eq!(begun.sum(), optimum.sum);
    assert!(dive(&p.model, &p.root, &graph, &p.candidates).sum() <= begun.sum());
    assert_eq!(hood.improve(&mut Scratch::new(), &begun, true), begun);
    let r = search(&p, &config(1), Some(&seed), Some(optimum.sum), false).unwrap();
    assert!(r.completed && r.reached_cap && r.stats.nodes == 0 && r.found.is_none());
    assert_eq!((r.start, r.promotions, r.sum), (begun, begun, optimum.sum));

    // A ceiling below P* (which a game's proof never offers) that the greedy set reaches shows that the local
    // search is skipped: it would have raised the start, as it does without the ceiling.
    let improved = hood.improve(&mut Scratch::new(), &p.greedy, true);
    assert!(improved.sum() > p.greedy.sum(), "the local search leaves the greedy set of 300 as it is");
    let r = search(&p, &config(1), None, Some(p.greedy.sum()), false).unwrap();
    assert!(r.completed && r.reached_cap && r.stats.nodes == 0);
    assert_eq!((r.start, r.promotions), (p.greedy, p.greedy));
}

/// The start is offered to the control as it is made, before the engine runs: a search that ends at its start, as a
/// ceiling that the start reaches ends it, leaves the control with that start, whether it is the greedy set, the
/// seed's survivors, the dive or the local search that made it.
#[test]
fn the_start_is_offered_to_the_control_as_it_is_made() {
    let n = 930;
    let board = crate::board::Board::<16>::for_n(n);
    let p = board.prepare(SearchTactics::default()).unwrap();
    let graph = MatchGraph::new(&p.model, &p.root);
    let hood = Neighborhood { model: &p.model, root: &p.root, candidates: &p.candidates };
    let dived = dive(&p.model, &p.root, &graph, &p.candidates);
    let improved = hood.improve(&mut Scratch::new(), &dived, true);
    let seed = board.seed(crate::game::Optimal::built_in().moves(n - 1).unwrap());
    let kept = surviving(&p, &seed);
    assert!(p.greedy.sum() < dived.sum() && dived.sum() < improved.sum(), "each step of the start improves it");
    assert!(p.greedy.sum() < kept.sum(), "the seed's survivors beat the greedy set");

    // Each ceiling is the sum of one step: the search ends there, and the control has that step.
    let steps = [("the greedy set", None, p.greedy), ("the dive", None, dived), ("the local search", None, improved)];
    let seeded = [("the seed", Some(&seed), kept)];
    for (what, seed, step) in steps.into_iter().chain(seeded) {
        let control = Control::new();
        let cfg = EngineConfig { control: Some(Arc::clone(&control)), ..config(1) };
        assert_eq!(best_so_far::<16>(&control), None);
        let r = search(&p, &cfg, seed, Some(step.sum()), false).unwrap();
        assert!(r.completed && r.reached_cap && r.stats.nodes == 0, "{what}");
        assert_eq!((r.start, r.promotions), (step, step), "{what}: the start");
        assert_eq!(best_so_far::<16>(&control), Some(step), "{what}: the control's best");
    }
}

/// The start of a search does not depend on its threads: the local search on several of them finds what one does,
/// with and without a seed, in the search of a game as a whole.
#[test]
fn the_start_is_the_same_on_every_number_of_threads() {
    for n in [300, 600] {
        let board = crate::board::Board::<16>::for_n(n);
        let p = board.prepare(SearchTactics::default()).unwrap();
        let optimal = crate::game::Optimal::built_in();
        let seed = board.seed(optimal.moves(n - 1).unwrap());
        let cap = board.promotion_cap(optimal.score(n - 1).unwrap());
        for (seed, cap) in [(None, None), (Some(&seed), None), (Some(&seed), cap)] {
            let one = search(&p, &config(1), seed, cap, false).unwrap();
            for threads in [2, 4] {
                let more = search(&p, &config(threads), seed, cap, false).unwrap();
                let what = format!("n={n}, seeded {}, ceiling {cap:?}, {threads} threads", seed.is_some());
                assert_eq!((more.start, more.sum, more.completed), (one.start, one.sum, one.completed), "{what}");
            }
        }
    }
}

/// The start from a seed, and from none, is the steps of the start spelled out: the greedy set, or the seed's survivors
/// if they are heavier, then the dive if that is heavier still, then the local search.  Moves for the board itself
/// survive whole; in game 187 the start from no seed weighs 751 and what survives of the moves for 186 only 748, but the
/// start from the moves for 186 reaches 762, so the seed with fewer survivors starts higher.  A search with the seed
/// starts there: a ceiling at that start ends it at once, and the control has it.
#[test]
fn the_start_is_the_steps_of_the_start_spelled_out() {
    let n = 187;
    let board = Board::<8>::for_n(n);
    let p = board.prepare(SearchTactics::default()).unwrap();
    let graph = MatchGraph::new(&p.model, &p.root);
    let hood = Neighborhood { model: &p.model, root: &p.root, candidates: &p.candidates };
    let prev = board.seed(crate::game::Optimal::built_in().moves(n - 1).unwrap());
    let start = |seed: Option<&Bits<8>>| start_incumbent(&p, &graph, &Control::new(), 1, seed);
    let (without, from_prev) = (start(None), start(Some(&prev)));
    assert_eq!((without.sum(), from_prev.sum()), (751, 762));
    assert!(seed_survivors_sum(&p, &without) > seed_survivors_sum(&p, &prev), "the start survives heavier");
    assert_eq!(start(Some(&without)), without, "the start from the start stays where it is");

    let steps = |seed: Option<&Bits<8>>| {
        let mut begun = p.greedy;
        if let Some(kept) = seed.map(|seed| surviving(&p, seed)).filter(|kept| kept.sum() > begun.sum()) {
            begun = kept;
        }
        let dived = dive(&p.model, &p.root, &graph, &p.candidates);
        if dived.sum() > begun.sum() {
            begun = dived;
        }
        hood.improve(&mut Scratch::new(), &begun, true)
    };
    assert_eq!((without, from_prev), (steps(None), steps(Some(&prev))));

    let control = Control::new();
    let cfg = EngineConfig { control: Some(Arc::clone(&control)), ..config(1) };
    let r = search(&p, &cfg, Some(&prev), Some(from_prev.sum()), false).unwrap();
    assert!(r.completed && r.reached_cap && r.stats.nodes == 0);
    assert_eq!((r.start, r.promotions), (from_prev, from_prev));
    assert_eq!(best_so_far::<8>(&control), Some(from_prev));
}

/// A ceiling given to the control of a search ends it when the incumbent reaches it, whether it is given before the
/// search starts, as `cap` is, or from another thread while the search runs.  Game 988 starts from an incumbent
/// 155 below its optimum, which the search finds at its step 4,614 of 6,287, a third of a second after it starts
/// (the other thread has that long to give the ceiling).
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is a_ceiling_given_before_the_search_to_the_control_ends_it"
)]
fn a_ceiling_given_to_the_control_ends_the_search() {
    a_ceiling_given_to_the_control_ends_the_search_of(988, true);
}

/// [`a_ceiling_given_to_the_control_ends_the_search`] for game 780, whose search (498 steps; 114 with the ceiling) is
/// too short to give the ceiling to while it runs: given before the search, in the control or as `cap`.
#[test]
fn a_ceiling_given_before_the_search_to_the_control_ends_it() {
    a_ceiling_given_to_the_control_ends_the_search_of(780, false);
}

/// A ceiling given to the control of the search of game `n` ends it, before it starts and, if `live`, from another
/// thread while it runs.
fn a_ceiling_given_to_the_control_ends_the_search_of(n: u32, live: bool) {
    let board = crate::board::Board::<16>::for_n(n);
    let p = board.prepare(SearchTactics::default()).unwrap();
    let optimum = crate::game::Optimal::built_in().score(n).unwrap() - board.in_the_bag;
    let whole = search(&p, &config(1), None, None, false).unwrap();
    assert!(whole.completed && !whole.reached_cap && whole.sum == optimum);
    assert!(whole.start.sum() < optimum, "the search has nothing to do from its start");

    // Given before the search, in the control or as `cap`: the same search, which ends where it finds P*.
    let control = Control::new();
    control.set_ceiling(optimum);
    let cfg = EngineConfig { control: Some(Arc::clone(&control)), ..config(1) };
    let given = search(&p, &cfg, None, None, false).unwrap();
    let capped = search(&p, &config(1), None, Some(optimum), false).unwrap();
    assert!(given.completed && given.reached_cap && given.sum == optimum);
    assert_eq!((given.sum, given.stats.nodes), (capped.sum, capped.stats.nodes));
    assert!(given.stats.nodes < whole.stats.nodes, "{} steps, as many as without the ceiling", given.stats.nodes);

    if !live {
        return;
    }
    // Given from another thread once the run goes.
    let control = Control::new();
    let cfg = EngineConfig { control: Some(Arc::clone(&control)), ..config(1) };
    let live = thread::scope(|scope| {
        let run = scope.spawn(|| search(&p, &cfg, None, None, false).unwrap());
        while control.status().is_none() && !run.is_finished() {
            thread::yield_now();
        }
        control.set_ceiling(optimum);
        run.join().unwrap()
    });
    assert!(live.completed && live.reached_cap && live.sum == optimum, "{live:?}");
    assert!(live.stats.nodes < whole.stats.nodes, "{} steps, as many as without the ceiling", live.stats.nodes);
    assert_eq!(best_so_far::<16>(&control), Some(live.promotions));
    assert_eq!(control.ceiling(), Some(optimum));
}

/// The promotions of the previous moves offered to the control before the run raise its start as `seed` does: game 930
/// starts from 20,469 on its own and from its optimum, 20,480, with the seed, and the search is the same either way.
#[test]
fn a_seed_offered_to_the_control_is_the_seed_of_the_search() {
    let n = 930;
    let board = crate::board::Board::<16>::for_n(n);
    let p = board.prepare(SearchTactics::default()).unwrap();
    let seed = board.seed(crate::game::Optimal::built_in().moves(n - 1).unwrap());
    let seeded = search(&p, &config(1), Some(&seed), None, false).unwrap();
    let unseeded = search(&p, &config(1), None, None, false).unwrap();
    assert!(seeded.start.sum() > unseeded.start.sum() && seeded.found.is_none() && unseeded.found.is_some());

    let control = Control::new();
    let cfg = EngineConfig { control: Some(Arc::clone(&control)), ..config(1) };
    assert_eq!(best_so_far::<16>(&control), None);
    let kept = surviving(&p, &seed);
    assert!(offer_seed(&p, &control, &seed));
    assert_eq!(best_so_far::<16>(&control), Some(kept));
    // What does not beat the incumbent is not taken.
    assert!(!offer_seed(&p, &control, &Bits::EMPTY));
    assert_eq!(best_so_far::<16>(&control), Some(kept));

    let offered = search(&p, &cfg, None, None, false).unwrap();
    assert!(offered.completed && offered.found.is_none());
    assert_eq!((offered.sum, offered.promotions), (seeded.sum, seeded.promotions));
    assert_eq!(offered.stats.nodes, seeded.stats.nodes);
    assert_eq!(best_so_far::<16>(&control), Some(offered.promotions));
}

#[test]
fn units_and_headers_round_trip() {
    let unit = TaxUnit::<3> { promoted: Bits::from_slice(&[1, 64, 190]), cands: Bits::from_slice(&[0, 63, 127]) };
    let mut bytes = Vec::new();
    unit.encode(&mut bytes);
    assert_eq!(bytes.len(), 48);
    assert_eq!(TaxUnit::<3>::decode(&bytes), unit);

    let p = prepare_golden::<8>(&load(300), SearchTactics { nogoods: false, ..SearchTactics::default() });
    let header = Header::of(&p);
    let bytes = header.encode();
    assert_eq!(bytes.len(), Header::<8>::LEN);
    assert_eq!((Header::<8>::LEN, Header::<8>::VERSION), (38, 8));
    let back = Header::<8>::decode(&bytes).unwrap();
    assert_eq!(back, header);
    assert_eq!((back.n, back.max_total, back.tactics), (300, p.max_total, p.tactics));
    assert_eq!(back.mismatch(&header), None);
    assert!(Header::<16>::decode(&bytes).unwrap_err().contains("Bits<8>"));
    assert!(Header::<8>::decode(b"nonsense").unwrap_err().contains("not a checkpoint"));
    assert!(Header::<8>::decode(&bytes[..20]).is_err());
    // The layout: the magic, the version, the width and n, the digests of the candidates and of the frames,
    // `max_total` and the tactics' bits, which the defaults make 0x2FF.
    assert_eq!(&bytes[..4], b"TXSH");
    assert_eq!(bytes[4..8], 8u32.to_le_bytes());
    assert_eq!(bytes[8..12], 8u32.to_le_bytes());
    assert_eq!(bytes[12..16], 300u32.to_le_bytes());
    assert_eq!(bytes[32..36], p.max_total.to_le_bytes());
    assert_eq!(Header::of(&prepare_golden::<8>(&load(300), SearchTactics::default())).encode()[36..], [0xFF, 0x02]);
    // Every other format version, the older ones included (`TXS2` was version 2), is refused with one message, naming
    // its number; a header of the magic of another format is no checkpoint of a search at all.
    let versioned = |version: u32| [&bytes[..4], &version.to_le_bytes(), &bytes[8..]].concat();
    assert_eq!(Header::<8>::decode(&versioned(8)), Ok(header));
    for (version, bytes) in [3, 4, 5, 6, 7, 9]
        .map(|v| (v.to_string(), versioned(v)))
        .into_iter()
        .chain([("unreadable".to_owned(), b"TXSH".to_vec())])
    {
        let err = Header::<8>::decode(&bytes).unwrap_err();
        let message = format!("format version {version}, and this build reads version 8 only");
        assert!(err.contains(&message), "{err}");
    }
    assert!(Header::<8>::decode(&[b"TXS2".as_slice(), &bytes[4..]].concat()).unwrap_err().contains("not a checkpoint"));
    // A bit that no tactic uses is refused.
    let mut unknown = bytes.clone();
    unknown[37] = 0x04;
    assert!(Header::<8>::decode(&unknown).unwrap_err().contains("unknown tactics"));
    // Every field that differs is named.
    let other = |change: fn(&mut Header<8>)| {
        let mut h = header;
        change(&mut h);
        h.mismatch(&header).unwrap()
    };
    assert!(other(|h| h.n = 301).contains("n=301, not n=300"));
    assert!(other(|h| h.candidates ^= 1).contains("candidates"));
    assert!(other(|h| h.frames ^= 1).contains("frames"));
    assert!(other(|h| h.max_total += 1).contains("max_total"));
    assert!(other(|h| h.tactics.nogoods = !h.tactics.nogoods).contains("tactics"));
    // The branching switch is recorded, as the search began with it, but binds nothing: a header with it the other way
    // is this search's too.
    let mut branching = header;
    branching.tactics.conflict_branching = !header.tactics.conflict_branching;
    assert_eq!(branching.mismatch(&header), None);
    assert_ne!(branching.encode(), header.encode());
    // So is the look, which is on by default: a header with it off is this search's.
    let mut unlooking = header;
    unlooking.tactics.conflict_look = false;
    assert_eq!(unlooking.mismatch(&header), None);
    assert_ne!(unlooking.encode(), header.encode());
    assert_eq!(Header::<8>::decode(&unlooking.encode()), Ok(unlooking));
    assert!(header.tactics.conflict_look);

    // So is the near-repeat cut, which is on by default.
    let mut cutting = header;
    cutting.tactics.near_repeat = !header.tactics.near_repeat;
    assert_eq!(cutting.mismatch(&header), None);
    assert_ne!(cutting.encode(), header.encode());
    assert_eq!(Header::<8>::decode(&cutting.encode()), Ok(cutting));
    assert!(header.tactics.near_repeat);

    // So is the reuse of prices, which is on by default: a header with it off is this search's.
    let mut reusing = header;
    reusing.tactics.price_reuse = false;
    assert_eq!(reusing.mismatch(&header), None);
    assert_eq!(header.mismatch(&reusing), None);
    assert_ne!(reusing.encode(), header.encode());
    assert_eq!(Header::<8>::decode(&reusing.encode()), Ok(reusing));
    assert!(header.tactics.price_reuse);

    // So is the carrying of nogoods, which is off by default.
    let mut carrying = header;
    carrying.tactics.carried_nogoods = true;
    assert_eq!(carrying.mismatch(&header), None);
    assert_eq!(header.mismatch(&carrying), None);
    assert_ne!(carrying.encode(), header.encode());
    assert_eq!(Header::<8>::decode(&carrying.encode()), Ok(carrying));
    assert!(!header.tactics.carried_nogoods);

    // So is the warm start of the GF(2) bound, which is on by default.
    let mut warming = header;
    warming.tactics.warm_start = false;
    assert_eq!(warming.mismatch(&header), None);
    assert_eq!(header.mismatch(&warming), None);
    assert_ne!(warming.encode(), header.encode());
    assert_eq!(Header::<8>::decode(&warming.encode()), Ok(warming));
    assert!(header.tactics.warm_start);

    // Every setting of the search tactics reads back as it was written: a bit for each switch that is on, in the order
    // of `SearchTactics::bits` (`fix_forced` bit 0, `nogoods` 1, `forget_nogoods` 2, `symmetry` 3, `conflict_branching`
    // 4, `conflict_look` 5, `near_repeat` 6, `price_reuse` 7, `carried_nogoods` 8 and `warm_start` 9: the defaults make
    // 0x2FF), and no higher bit is used.  The switches of bits 4 to 9 bind nothing
    // ([`SearchTactics::same_recorded`]): a checkpoint records each as the search began with it, and resumes with the
    // tactic on or off, and a run that resumes a search says it.  The others bind.
    let flips: [fn(SearchTactics) -> SearchTactics; 10] = [
        |t| SearchTactics { fix_forced: !t.fix_forced, ..t },
        |t| SearchTactics { nogoods: !t.nogoods, ..t },
        |t| SearchTactics { forget_nogoods: !t.forget_nogoods, ..t },
        |t| SearchTactics { symmetry: !t.symmetry, ..t },
        |t| SearchTactics { conflict_branching: !t.conflict_branching, ..t },
        |t| SearchTactics { conflict_look: !t.conflict_look, ..t },
        |t| SearchTactics { near_repeat: !t.near_repeat, ..t },
        |t| SearchTactics { price_reuse: !t.price_reuse, ..t },
        |t| SearchTactics { carried_nogoods: !t.carried_nogoods, ..t },
        |t| SearchTactics { warm_start: !t.warm_start, ..t },
    ];
    let default = SearchTactics::default();
    assert_eq!(default.bits(), 0x2FF);
    for (bit, flip) in flips.iter().enumerate() {
        let flipped = flip(default);
        assert_eq!(flipped.bits() ^ default.bits(), 1 << bit, "the bit of switch {bit}");
        assert_eq!(SearchTactics::from_bits(flipped.bits()), Ok(flipped));
        let binds = bit < 4;
        assert_eq!(flipped.same_recorded(&default), !binds, "{flipped:?}");
        assert_eq!(default.same_recorded(&flipped), !binds, "{flipped:?}");
        assert_eq!(default.with_unbound_of(&flipped), if binds { default } else { flipped });
        assert_eq!(flipped.with_unbound_of(&default), if binds { flipped } else { default });
        // A switch that binds still binds beside one that does not.
        assert!(!SearchTactics { symmetry: false, ..flipped }.same_recorded(&default));
    }
    let all_flipped = flips.iter().fold(default, |t, flip| flip(t));
    assert_eq!(all_flipped.bits(), !default.bits() & 0x3FF);
    assert_eq!(SearchTactics::from_bits(all_flipped.bits()), Ok(all_flipped));
    assert!(SearchTactics::from_bits(1 << 10).unwrap_err().contains("unknown tactics"));
    assert_eq!(SearchTactics::BRANCHING_BITS, 1 << 4 | 1 << 5);
    assert_eq!(SearchTactics::NEAR_REPEAT_BIT, 1 << 6);
    assert_eq!(SearchTactics::PRICE_REUSE_BIT, 1 << 7);
    assert_eq!(SearchTactics::CARRIED_NOGOODS_BIT, 1 << 8);
    assert_eq!(SearchTactics::WARM_START_BIT, 1 << 9);
}

/// The form of `problem` ([`Problem`]'s list of numbers).
fn form_of(problem: &Problem) -> Vec<u32> {
    problem.form().to_vec()
}

/// Where `form` ends: after the frames (their freedoms, moves and factors), the candidates, `max_total`, and for each
/// number of the frames, ascending, its record.  Checked as it is walked: the frames of `prepared` are what it lists.
fn end_of_form<const W: usize>(form: &[u32], prepared: &Prepared<W>) -> usize {
    let list = |at: &mut usize| -> Vec<u32> {
        let len = form[*at] as usize;
        *at += 1 + len;
        form[*at - len..*at].to_vec()
    };
    let frames = prepared.root.frames();
    assert_eq!(form[0] as usize, frames.len());
    let (mut at, mut numbers) = (1, Bits::<W>::EMPTY);
    for frame in frames {
        assert_eq!(form[at], frame.freedoms as u32);
        at += 1;
        assert_eq!(list(&mut at), frame.moves.to_vec());
        assert_eq!(list(&mut at), frame.factors.to_vec());
        numbers |= frame.moves | frame.factors;
    }
    assert_eq!(list(&mut at), prepared.candidates.to_vec());
    assert_eq!(form[at], prepared.max_total);
    at += 1;
    let graph = prepared.model.graph();
    for x in numbers.iter() {
        assert_eq!(form[at..at + 2], [x, prepared.model.home(x) as u32]);
        at += 2;
        let factors = graph.factors(x).iter().map(|&y| u32::from(y)).collect::<Vec<_>>();
        let composites = graph.composites(x).iter().map(|&y| u32::from(y)).collect::<Vec<_>>();
        assert_eq!((list(&mut at), list(&mut at)), (factors, composites), "the record of {x}");
    }
    at
}

/// The form of the problem that [`prepare`] poses is what [`Problem`] lists, with nothing after it; its digest is the
/// checksum of the whole form; and [`problem_of`] rebuilds the same problem: on every board to 300 (at width 8) and at
/// larger boards at their own widths.
#[test]
fn the_form_of_a_problem_is_what_the_problem_lists() {
    fn check<const W: usize>(n: u32) {
        let board = Board::<W>::for_n(n);
        let prepared = board.prepare(SearchTactics::default()).unwrap();
        let (problem, form) = (prepared.problem(), form_of(&prepared.problem()));
        assert_eq!(problem, board.problem().unwrap(), "n={n}");
        assert_eq!(end_of_form(&form, &prepared), form.len(), "n={n}: a form with nothing after the numbers' lists");
        let bytes: Vec<u8> = form.iter().flat_map(|word| word.to_le_bytes()).collect();
        assert_eq!(problem.digest(), engine::checksum(&bytes), "n={n}");
        assert_eq!(problem.size().candidates, prepared.candidates.len(), "n={n}");
    }
    (2..=300).for_each(check::<8>);
    for n in [480, 600, 760, 1000, 1133, 1300] {
        crate::with_width!(n, W => check::<W>(n));
    }
}

// ---- benchmark: cargo test --release bench_search -- --ignored --nocapture --test-threads=1 ----

/// Prepares and searches one game with each thread count, printing one line each.
fn bench_game<const W: usize>(n: u32, columns: &[Vec<u32>], cands: &Bits<W>, java: (f64, f64)) {
    let tables = Tables::new(n);
    for (threads, java_secs) in [(1, java.0), (10, java.1)] {
        let t0 = Instant::now();
        let p = prepare::<W>(&tables, columns, cands, SearchTactics::default()).unwrap();
        let r = search(&p, &config(threads), None, None, false).unwrap();
        let total = t0.elapsed().as_secs_f64();
        let t = &p.timings;
        eprintln!(
            "n={n:4} threads={threads:2}: total {total:7.3} s (Java {java_secs:5.1} s) | greedy {:.3} search {:.3} \
             | nodes {} | sum {} | extra {:?}",
            t.greedy.as_secs_f64(),
            r.time.as_secs_f64(),
            r.stats.nodes,
            r.sum,
            r.stats.extra,
        );
    }
}

/// One game (`TAXMAN_BENCH_N`, default 700; `TAXMAN_BENCH_THREADS`, default 1), with progress lines, for
/// profiling.
#[test]
#[ignore]
fn bench_search_one() {
    let n = std::env::var("TAXMAN_BENCH_N").ok().and_then(|s| s.parse().ok()).unwrap_or(700);
    let threads = std::env::var("TAXMAN_BENCH_THREADS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let tactics = SearchTactics::default();
    let seconds = std::env::var("TAXMAN_BENCH_SECONDS").ok().and_then(|s| s.parse().ok());
    let tables = Tables::new(n);
    // The golden columns if there are any, else the frame builder's.
    let (columns, cands) = if golden_ns().contains(&n) {
        let g = load(n);
        (g.columns, Bits::<16>::from_slice(&g.candidates))
    } else {
        let frames = build_frames::<16>(&tables);
        (frames.frames.iter().flat_map(|f| [f.factors.to_vec(), f.moves.to_vec()]).collect(), frames.candidates)
    };
    let p = prepare::<16>(&tables, &columns, &cands, tactics).unwrap();
    let cfg = EngineConfig {
        progress: Some(Duration::from_secs(10)),
        time_limit: seconds.map(Duration::from_secs),
        ..config(threads)
    };
    // What the table says the best promotion sum is, and the `prev_cap` ceiling.
    {
        let optimal = crate::game::Optimal::built_in();
        let in_the_bag: u64 =
            crate::frames::BoardFrames::<16>::new(&tables, Bits::EMPTY).solution().iter().map(|&x| u64::from(x)).sum();
        let best = optimal.score(n).map(|s| s - in_the_bag);
        let cap = optimal.score(n - 1).map(|s| u64::from(n) + s - in_the_bag);
        eprintln!("n={n}: optimal promotion sum {best:?}, prev_cap {cap:?}, greedy {}", p.greedy.sum());
    }
    let r = search(&p, &cfg, None, None, false).unwrap();
    eprintln!(
        "n={n}: {:?} search {:.3} s (completed {}), nodes {}, best {}, max_total {}, extra {:?}",
        p.timings,
        r.time.as_secs_f64(),
        r.completed,
        r.stats.nodes,
        r.sum,
        p.max_total,
        r.stats.extra
    );
}

#[test]
#[ignore]
fn bench_search() {
    // Java whole-game wall times, one thread and ten (including ~0.5 s of JVM start and setup).
    let java = [
        (480, 2.3, 1.5),
        (500, 2.1, 1.4),
        (600, 2.7, 1.7),
        (640, 2.0, 1.6),
        (660, 4.1, 2.1),
        (700, 18.0, 4.8),
        (720, 23.2, 5.6),
        (760, 65.6, 12.4),
    ];
    for (n, one, ten) in java {
        let g = load(n);
        if n < 512 {
            bench_game::<8>(n, &g.columns, &Bits::from_slice(&g.candidates), (one, ten));
        } else {
            bench_game::<16>(n, &g.columns, &Bits::from_slice(&g.candidates), (one, ten));
        }
    }
    if std::env::var_os("TAXMAN_BENCH_1000").is_some() {
        let frames = build_frames::<16>(&Tables::new(1000));
        let columns: Vec<Vec<u32>> =
            frames.frames.iter().flat_map(|f| [f.factors.to_vec(), f.moves.to_vec()]).collect();
        bench_game::<16>(1000, &columns, &frames.candidates, (f64::NAN, f64::NAN));
    }
}
