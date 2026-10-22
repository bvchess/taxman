//! A study of what a proven game tells the search of a later game (the notebook entry beside this file): the
//! obligation of game N's proof, given game M's.  It is an example of the frame solver kept out of the build: copy it
//! to solver/frame/examples/ to run it.
//!
//! ```text
//! cargo run --release --example carry -- dissolved M N [--known FILE] [--max-size K] [--out FILE]
//! cargo run --release --example carry -- problems FROM TO [--known FILE]
//! cargo run --release --example carry -- optimum N N [--known FILE]
//! ```
//!
//! `problems` prints a line a game of FROM..=TO: n, the digest of its search problem, its candidates, `max_total`, P\*
//! and the root gap against it, or `prime`.  `optimum` prints game N's known optimal promotions in hex.
//!
//! Let Prom(Π) be the promotion sets of a search problem and P\*(Π) its best promotion sum.  If game M is proven,
//! P\*_M ≤ V, then every promotion set of game N worth more than V is not a promotion set of game M (a set of both is
//! worth at most P\*_M).  A set of N fails in M only where M differs from N: in a frame of M with no twin in N (the
//! same moves, factors and freedoms, and the same candidates promoted out of it and into it), or through a candidate
//! of N that is no candidate of M.  Since a frame that a set breaks stays broken under every larger set, such a set of
//! N holds a *dissolved conflict*: a set of candidates that M cannot promote and N can, minimal in M, of the
//! candidates that touch one frame of M without a twin; or a new candidate.  So N's proof against V can search only
//! the subtrees that promote one of them.
//!
//! `dissolved` sets up both games as the solver does, finds the frames of M without a twin in N, enumerates each one's
//! dissolved conflicts by size, up to K members (default 4), and writes N's units that promote each (and each new
//! candidate) to FILE in the format of `tune units`, with each unit's gap against N's known optimum, so that `tune
//! solve N --units FILE` proves them.  It reports on stderr what it found and whether the enumeration was complete:
//! it is complete for a frame when no set of K members stays promotable in M.

#[allow(dead_code)]
#[path = "common/fixtures.rs"]
mod fixtures;
#[path = "common/known.rs"]
mod known;

use fixtures::to_hex;
use known::known_incumbent;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::game::Optimal;
use taxman::gf2::Gf2Graph;
use taxman::json::Json;
use taxman::model::Frame;
use taxman::search::{NodeBound, Prepared, SearchTactics, TaxUnit};

/// A frame with the candidates promoted out of it and into it: two frames with equal keys fail under the same sets.
type FrameKey<const W: usize> = (Frame<W>, Vec<u32>, Vec<u32>);

fn frame_keys<const W: usize>(p: &Prepared<W>) -> Vec<FrameKey<W>> {
    let frames = p.root.frames();
    let mut keys: Vec<FrameKey<W>> = frames.iter().map(|f| (*f, Vec::new(), Vec::new())).collect();
    for x in p.candidates.iter() {
        let (front, home) = p.model.frames_touched(x);
        keys[front].2.push(x);
        if let Some(h) = home {
            keys[h].1.push(x);
        }
    }
    keys
}

struct Found<const W: usize> {
    frame: usize,
    set: Bits<W>,
}

/// The dissolved conflicts among `a`: sets that `m` cannot promote and `n` can, every proper subset promotable in
/// `m`, of at most `max_size` members; and whether the enumeration ended (no set of `max_size` stayed promotable in m).
fn dissolved_in<const W: usize>(
    m: &Prepared<W>,
    n: &Prepared<W>,
    a: &[u32],
    max_size: usize,
    limit: usize,
) -> (Vec<Bits<W>>, Vec<usize>, bool) {
    let mut found = Vec::new();
    let mut sizes = Vec::new();
    let mut level: Vec<Vec<u32>> = vec![Vec::new()];
    for k in 1..=max_size {
        let alive: HashSet<Vec<u32>> = level.iter().cloned().collect();
        let mut next = Vec::new();
        for s in &level {
            let start = s.last().map_or(0, |&l| a.partition_point(|&y| y <= l));
            for &y in &a[start..] {
                let mut t = s.clone();
                t.push(y);
                // Every subset of k - 1 members must be promotable in m (else t is not minimal).
                if k > 1
                    && (0..t.len() - 1).any(|i| {
                        let mut u = t.clone();
                        u.remove(i);
                        !alive.contains(&u)
                    })
                {
                    continue;
                }
                let set: Bits<W> = t.iter().copied().collect();
                if m.can_promote(&set) {
                    next.push(t);
                } else if n.can_promote(&set) {
                    found.push(set);
                }
            }
            if next.len() > limit {
                break;
            }
        }
        sizes.push(next.len());
        if next.is_empty() {
            return (found, sizes, true);
        }
        if next.len() > limit {
            return (found, sizes, false);
        }
        level = next;
    }
    (found, sizes, false)
}

fn dissolved<const W: usize>(
    m_n: (u32, u32),
    optimal: &Optimal,
    max_size: usize,
    out: Option<PathBuf>,
) -> Result<(), String> {
    let (m, n) = m_n;
    let (bm, bn) = (Board::<W>::for_n(m), Board::<W>::for_n(n));
    let pm = bm.prepare(SearchTactics::default())?;
    let pn = bn.prepare(SearchTactics::default())?;
    let (_, opt_m) = known_incumbent(&bm, &pm, optimal)?.ok_or(format!("no known game of {m}"))?;
    let (set_n, opt_n) = known_incumbent(&bn, &pn, optimal)?.ok_or(format!("no known game of {n}"))?;
    eprintln!(
        "games {m} and {n}: P* {opt_m} and {opt_n}; candidates {} and {}; max_total {} and {}; frames {} and {}",
        pm.candidates.len(),
        pn.candidates.len(),
        pm.max_total,
        pn.max_total,
        pm.root.len(),
        pn.root.len()
    );
    if opt_n < opt_m {
        eprintln!("P* falls: the obligation of {n} is in the band ({opt_n}, {opt_m}], not only in the new sets");
    }
    let (km, kn) = (frame_keys(&pm), frame_keys(&pn));
    let twins: HashSet<&FrameKey<W>> = kn.iter().collect();
    let changed: Vec<usize> = (0..km.len()).filter(|&j| !twins.contains(&km[j])).collect();
    let new_cands = pn.candidates - pm.candidates;
    eprintln!("frames of {m} without a twin in {n}: {changed:?}; new candidates {:?}", new_cands.to_vec());
    let mut all: Vec<Found<W>> = Vec::new();
    let mut complete = true;
    for &j in &changed {
        let (frame, out_of, into) = &km[j];
        // Its nearest frame of N, and what differs.
        let near = kn
            .iter()
            .min_by_key(|(f, _, _)| {
                let (a, b) = (f.moves | f.factors, frame.moves | frame.factors);
                (a - b).len() + (b - a).len()
            })
            .ok_or("game N has no frames")?;
        let diff = |a: &[u32], b: &[u32]| {
            let (sa, sb): (HashSet<u32>, HashSet<u32>) = (a.iter().copied().collect(), b.iter().copied().collect());
            let mut minus: Vec<u32> = sa.difference(&sb).copied().collect();
            let mut plus: Vec<u32> = sb.difference(&sa).copied().collect();
            minus.sort_unstable();
            plus.sort_unstable();
            (minus, plus)
        };
        eprintln!(
            "frame {j} against its nearest of {n}: moves -/+ {:?}, factors -/+ {:?}, freedoms {} -> {}, promoted \
             out of it -/+ {:?}, into it -/+ {:?}",
            diff(&frame.moves.to_vec(), &near.0.moves.to_vec()),
            diff(&frame.factors.to_vec(), &near.0.factors.to_vec()),
            frame.freedoms,
            near.0.freedoms,
            diff(out_of, &near.1),
            diff(into, &near.2)
        );
        let mut a: Vec<u32> = out_of.iter().chain(into).copied().filter(|&x| pn.candidates.contains(x)).collect();
        a.sort_unstable();
        a.dedup();
        let (sets, sizes, done) = dissolved_in(&pm, &pn, &a, max_size, 2_000_000);
        complete &= done;
        let mut by_size: HashMap<u32, usize> = HashMap::new();
        for s in &sets {
            *by_size.entry(s.len()).or_default() += 1;
        }
        let mut by_size: Vec<_> = by_size.into_iter().collect();
        by_size.sort_unstable();
        eprintln!(
            "frame {j}: {} moves, {} factors, {} freedoms; {} candidates touch it; promotable in {m} by size {:?}; \
             dissolved by size {:?}; complete {done}",
            frame.moves.len(),
            frame.factors.len(),
            frame.freedoms,
            a.len(),
            sizes,
            by_size
        );
        all.extend(sets.into_iter().map(|set| Found { frame: j, set }));
    }
    for x in new_cands.iter() {
        let set = Bits::EMPTY.with(x);
        if pn.can_promote(&set) {
            all.push(Found { frame: usize::MAX, set });
        }
    }
    let graph = Gf2Graph::new(&pn.model, &pn.root, &pn.candidates);
    let mut bound = NodeBound::new(&pn, &graph);
    let root = TaxUnit { promoted: Bits::EMPTY, cands: pn.candidates };
    let root_bound = bound.completions(&root).ok_or("the root has no bound")?;
    eprintln!("root of {n}: bound {root_bound}, need {}, gap {}", opt_n + 1, root_bound as i64 - opt_n as i64 - 1);
    let mut lines = vec![Json::object([
        ("kind", Json::from("game")),
        ("game", Json::from(n)),
        ("width", Json::from(W)),
        ("candidates", Json::from(to_hex(&pn.candidates))),
        ("in_the_bag", Json::from(bn.in_the_bag)),
        ("max_total", Json::from(pn.max_total)),
        ("opt", Json::from(opt_n)),
        ("incumbent", Json::from(to_hex(&set_n))),
        ("incumbent_sum", Json::from(opt_n)),
        ("from", Json::from(m)),
        ("complete", Json::from(complete)),
    ])];
    let mut gaps = Vec::new();
    for (i, f) in all.iter().enumerate() {
        let unit = TaxUnit { promoted: f.set, cands: pn.candidates - f.set };
        let value = bound.completions(&unit).ok_or("a dissolved conflict that cannot be promoted")?;
        let need = (opt_n + 1).saturating_sub(f.set.sum());
        let gap = value as i64 - need as i64;
        gaps.push(gap);
        lines.push(Json::object([
            ("kind", Json::from("unit")),
            ("game", Json::from(n)),
            ("id", Json::from(format!("{n}-d{i}"))),
            ("index", Json::from(i as u64)),
            ("frame", if f.frame == usize::MAX { Json::Null } else { Json::from(f.frame as u64) }),
            ("depth", Json::from(f.set.len())),
            ("n_cands", Json::from(unit.cands.len())),
            ("sum", Json::from(f.set.sum())),
            ("bound", Json::from(value)),
            ("need", Json::from(need)),
            ("gap", Json::from(gap)),
            ("promoted", Json::from(to_hex(&unit.promoted))),
            ("cands", Json::from(to_hex(&unit.cands))),
        ]));
    }
    gaps.sort_unstable();
    let open = gaps.iter().filter(|&&g| g >= 0).count();
    eprintln!(
        "{} units ({} dissolved conflicts, {} new candidates), {open} with gap >= 0; gaps: min {:?}, median {:?}, \
         max {:?}; complete {complete}",
        all.len(),
        all.iter().filter(|f| f.frame != usize::MAX).count(),
        all.iter().filter(|f| f.frame == usize::MAX).count(),
        gaps.first(),
        gaps.get(gaps.len() / 2),
        gaps.last()
    );
    if let Some(out) = out {
        let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
        std::fs::write(&out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    }
    Ok(())
}

/// One line a game of `from..=to`: n, the digest of its search problem, its candidates, `max_total`, P\* and the root
/// gap against it, or `prime` for a prime n (the prime formula settles it).
fn problems<const W: usize>(n: u32, optimal: &Optimal) -> Result<String, String> {
    if n > 1 && (2..).take_while(|d| d * d <= n).all(|d| n % d != 0) {
        return Ok(format!("{n} prime"));
    }
    let board = Board::<W>::for_n(n);
    let p = board.prepare(SearchTactics::default())?;
    let opt = known_incumbent(&board, &p, optimal)?.map(|(_, opt)| opt);
    let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
    let mut bound = NodeBound::new(&p, &graph);
    let root = bound.completions(&TaxUnit { promoted: Bits::EMPTY, cands: p.candidates });
    let gap = match (root, opt) {
        (Some(b), Some(o)) => format!("{}", b as i64 - o as i64 - 1),
        _ => "-".into(),
    };
    Ok(format!(
        "{n} {:016x} {} {} {} {gap}",
        p.problem().digest(),
        p.candidates.len(),
        p.max_total,
        opt.map_or("-".into(), |o| o.to_string())
    ))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = "usage: carry dissolved M N [--known FILE] [--max-size K] [--out FILE] \
                 | carry problems FROM TO [--known FILE] | carry optimum N N [--known FILE] | carry gadget M N \
                 | carry states M N --units FILE";
    if args.len() < 3 || !matches!(args[0].as_str(), "dissolved" | "problems" | "optimum" | "gadget" | "states" | "verify") {
        return Err(usage.into());
    }
    let parse = |s: &str| s.parse::<u32>().map_err(|e| format!("{s}: {e}"));
    let (m, n) = (parse(&args[1])?, parse(&args[2])?);
    let (mut known, mut max_size, mut out, mut units) = (Vec::new(), 4usize, None, None);
    let mut rest = args[3..].iter();
    while let Some(flag) = rest.next() {
        let value = rest.next().ok_or(format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--known" => known.push(PathBuf::from(value)),
            "--max-size" => max_size = value.parse().map_err(|e| format!("{value}: {e}"))?,
            "--out" => out = Some(PathBuf::from(value)),
            "--units" => units = Some(value.clone()),
            other => return Err(format!("unknown option {other}; {usage}")),
        }
    }
    let optimal = Optimal::built_in().with_results_files(&known)?;
    if args[0] == "verify" {
        // carry verify M N --units FILE: a randomized check of the lemma of the near-repeat cut.  For each unit of N
        // in FILE (one in ten, at most 400), whether the cut's condition holds there (the frames its promotions leave
        // in M and N equal but for factors that can pay for nothing), then four completions made greedily in each
        // game (the unit's candidates in a random order, or by value from the largest, each added if the set stays
        // promotable in that game), each tested in the other game.  The lemma says a completion of a unit where the
        // condition holds is promotable in both games or in neither; where it does not hold, the test may tell them
        // apart, which shows that it can.
        let file = units.ok_or("verify needs --units FILE")?;
        let text = std::fs::read_to_string(&file).map_err(|e| format!("{file}: {e}"))?;
        return taxman::with_width!(m.max(n), W => {
            let pm = Board::<W>::for_n(m).prepare(SearchTactics::default())?;
            let pn = Board::<W>::for_n(n).prepare(SearchTactics::default())?;
            let mut scratch = taxman::model::Scratch::new();
            let mut rng = taxman::rng::Rng(1);
            // [condition holds, does not] x [completions tested, promotable in one game only]
            let mut tally = [[0u64; 2]; 2];
            let lines: Vec<&str> = text.lines().filter(|l| l.contains("\"unit\"")).collect();
            let step = (lines.len() / 400).max(10);
            for line in lines.iter().step_by(step) {
                let json = Json::parse(line).map_err(|e| e.to_string())?;
                let get = |k: &str| json.get(k).and_then(Json::as_str).ok_or(format!("no {k}"));
                let promoted: Bits<W> = fixtures::from_hex(get("promoted")?)?;
                let cands: Bits<W> = fixtures::from_hex(get("cands")?)?;
                let sn = pn.model.promote_all_using(&mut scratch, &pn.root, &promoted).ok_or("not promotable in N")?;
                let Some(sm) = pm.model.promote_all_using(&mut scratch, &pm.root, &promoted) else { continue };
                let live = |model: &taxman::model::FrameGraph, f: &Frame<W>| -> Bits<W> {
                    f.factors
                        .iter()
                        .filter(|&y| {
                            cands.contains(y)
                                || model.graph().composites(y).iter().any(|&z| {
                                    f.moves.contains(u32::from(z)) || cands.contains(u32::from(z))
                                })
                        })
                        .collect()
                };
                let holds = sm.frames().len() == sn.frames().len()
                    && sm.frames().iter().zip(sn.frames()).all(|(a, b)| {
                        a.moves == b.moves && a.freedoms == b.freedoms && live(&pm.model, a) == live(&pn.model, b)
                    });
                let row = usize::from(!holds);
                for k in 0..4 {
                    let mut order = cands.to_vec();
                    if k % 2 == 0 {
                        rng.shuffle(&mut order);
                    } else {
                        order.reverse();
                    }
                    let (built, other) = if k < 2 { (&pn, &pm) } else { (&pm, &pn) };
                    let mut set = promoted;
                    for &x in &order {
                        let with = set.with(x);
                        if built.model.promote_all_using(&mut scratch, &built.root, &with).is_some() {
                            set = with;
                        }
                    }
                    let in_other = other.model.promote_all_using(&mut scratch, &other.root, &set).is_some();
                    tally[row][0] += 1;
                    tally[row][1] += u64::from(!in_other);
                }
            }
            println!(
                "completions where the condition holds: {}, promotable in one game only: {}; where it does not: \
                 {}, in one game only: {}",
                tally[0][0], tally[0][1], tally[1][0], tally[1][1]
            );
            Ok(())
        });
    }
    if args[0] == "states" {
        // carry states M N --units FILE: for each unit of game N in FILE (a units file of N), whether its promotions
        // leave the same frames (moves, factors and freedoms, as a multiset) in M and in N, or cannot be promoted in M.
        let file = units.ok_or("states needs --units FILE")?;
        let text = std::fs::read_to_string(&file).map_err(|e| format!("{file}: {e}"))?;
        return taxman::with_width!(m.max(n), W => {
            let pm = Board::<W>::for_n(m).prepare(SearchTactics::default())?;
            let pn = Board::<W>::for_n(n).prepare(SearchTactics::default())?;
            let mut scratch = taxman::model::Scratch::new();
            let (mut same, mut differ, mut not_in_m) = (0u32, 0u32, 0u32);
            for line in text.lines() {
                let json = Json::parse(line).map_err(|e| e.to_string())?;
                if json.get("kind").and_then(Json::as_str) != Some("unit") {
                    continue;
                }
                let hex = json.get("promoted").and_then(Json::as_str).ok_or("no promoted")?;
                let promoted: Bits<W> = fixtures::from_hex(hex)?;
                let sn = pn.model.promote_all_using(&mut scratch, &pn.root, &promoted).ok_or("not promotable in N")?;
                let Some(sm) = pm.model.promote_all_using(&mut scratch, &pm.root, &promoted) else {
                    not_in_m += 1;
                    println!("{} not promotable", json.get("id").and_then(Json::as_str).unwrap_or("?"));
                    continue;
                };
                let key = |s: &taxman::model::FrameSet<W>| {
                    let mut frames: Vec<(Vec<u32>, Vec<u32>, i32)> =
                        s.frames().iter().map(|f| (f.moves.to_vec(), f.factors.to_vec(), f.freedoms)).collect();
                    frames.sort();
                    frames
                };
                let id = json.get("id").and_then(Json::as_str).unwrap_or("?");
                if key(&sm) == key(&sn) {
                    same += 1;
                    println!("{id} same");
                } else {
                    differ += 1;
                    println!("{id} different");
                }
            }
            println!(
                "units of {n}: frames the same in {m} and {n}: {same}; different: {differ}; not promotable in \
                 {m}: {not_in_m}"
            );
            Ok(())
        });
    }
    if args[0] == "gadget" {
        // carry gadget M N: for each move of one game's root that the other's lacks, the factors that can pay for it
        // and, for each, the numbers it can pay for, marked as moves (m) of that root, candidates (c) or neither.
        return taxman::with_width!(m.max(n), W => {
            let pm = Board::<W>::for_n(m).prepare(SearchTactics::default())?;
            let pn = Board::<W>::for_n(n).prepare(SearchTactics::default())?;
            let moves = |p: &Prepared<W>| p.root.frames().iter().fold(Bits::<W>::EMPTY, |a, f| a | f.moves);
            let (mm, mn) = (moves(&pm), moves(&pn));
            for (game, p, own, other) in [(m, &pm, mm, mn), (n, &pn, mn, mm)] {
                for x in (own - other).iter() {
                    let graph = p.model.graph();
                    println!("{game}: move {x} (home {}) paid by:", p.model.home(x));
                    for &y in graph.factors(x) {
                        let y = u32::from(y);
                        let marks: Vec<String> = graph
                            .composites(y)
                            .iter()
                            .map(|&z| {
                                let z = u32::from(z);
                                let mark = if own.contains(z) {
                                    "m"
                                } else if p.candidates.contains(z) {
                                    "c"
                                } else {
                                    ""
                                };
                                format!("{z}{mark}")
                            })
                            .collect();
                        println!("  {y} (home {}, candidate {}) pays for {}", p.model.home(y), p.candidates.contains(y),
                                 marks.join(" "));
                    }
                }
            }
            Ok(())
        });
    }
    if args[0] == "optimum" {
        // carry optimum N N: the known optimal promotions of game N in hex, the set `TAXMAN_CARRY_REF` reads.
        let hex = taxman::with_width!(n, W => {
            let board = Board::<W>::for_n(n);
            let p = board.prepare(SearchTactics::default())?;
            known_incumbent(&board, &p, &optimal)?.map(|(set, _)| to_hex(&set)).ok_or(format!("no known game of {n}"))
        })?;
        println!("{hex}");
        return Ok(());
    }
    if args[0] == "problems" {
        for k in m..=n {
            println!("{}", taxman::with_width!(k, W => problems::<W>(k, &optimal))?);
        }
        return Ok(());
    }
    taxman::with_width!(m.max(n), W => dissolved::<W>((m, n), &optimal, max_size, out))
}

fn main() {
    if let Err(e) = run() {
        eprintln!("carry: {e}");
        std::process::exit(1);
    }
}
