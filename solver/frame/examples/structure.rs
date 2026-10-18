//! The census of the structure of the frames: for each board size of a range, how the board's frames sit by the number
//! of prime factors of their numbers (their *layer*), how they are numbered from the front, what `unzip` claimed, and
//! whether the hypotheses of the exactness theorem of the frame model (docs/THEORY.md, "Theorem M") hold on the
//! board.  It looks at the frames as setup forms them and at the frames the search takes, built by the very function
//! the solver uses (`taxman::frames::numbering`), and at the model the search builds from them
//! ([`taxman::model::FrameGraph`]).  It searches nothing.
//!
//! ```text
//! cargo run --release --example structure -- FROM TO [--threads K] [--out FILE]
//! ```
//!
//! For every board `n` in `FROM..=TO` (both at least 2, `FROM <= TO`, `TO` at most the largest board the solver
//! supports) it writes one JSON object on one line, and at the end a summary.  With `--out FILE` the lines are appended
//! to FILE, and a run that finds FILE already holding lines of the range skips those boards (a last line cut short by
//! an interruption is cut off first), so an interrupted run resumes; without it the lines go to stdout.  `--threads K`
//! (default 1) runs K boards at a time; the lines are written in increasing `n` however the threads finish, each
//! flushed as it is written, so what an interrupted run leaves is a prefix of the work.  Progress goes to stderr about
//! every 100 boards.
//!
//! **Layers.**  The *layer* of a frame as setup forms it (before any merge) is the Omega of its sources (the number of
//! prime factors with multiplicity, of the sources and the impossible sources alike), or, if it has none, the Omega of
//! its smallest factor plus one.  A source's maximal factors have one prime factor fewer, so the frames in front of a
//! frame should lie one layer before it, and a frame's factors one layer before its sources.
//!
//! **The checks.**  Each yields a list of offending items, empty when it holds.
//!
//! * **A** (layers): every member of a frame's sources and impossible sources has the one Omega `k`, every factor has
//!   Omega `k - 1`, and every frame in front of it lies in layer `k - 1`.  Item: `{"frame": name, "layer": k,
//!   "sources": [..], "factors": [..], "in_front": [..]}` with what failed (the numbers off `k`, the names of the
//!   frames in front that are off `k - 1`).
//! * **C** (clean numbering): every frame with freedoms in front of a frame numbered `h` is numbered `h - 1` (any such
//!   frame in front of a frame numbered 0 is a violation), and every frame with freedoms has a number.  Item: `[name
//!   of the frame, its number, name of the frame in front, its number]`, with `null` for a number the frame lacks (all
//!   three after the name for a frame with freedoms but no number).
//! * **D** (back claims): a back claim hands the factor `f` to the one source that wants it; the search frames' claims
//!   must not hide a use of `f` as tax by a candidate, so no candidate has `f` among its maximal factors.  Item: `[i,
//!   s, f, c]`, the search frame `i` that holds the back claim of `f` by `s`, and the candidate `c`.
//! * **E** (no candidate in frame 0): the frame as setup forms it whose factors hold a candidate has a number of at
//!   least 1, so that the candidate has a frame in front to be promoted into.  Item: `[c, number]` (`null` for none).
//! * **F** (homes): the home frame of every candidate in the search's model is at least 1.  Item: `c`.
//! * **G** (the numbers above n/2): the numbers above `n/2` that the game with no promotions plays are exactly those
//!   an independent descending greedy keeps, written in this file with no frames code: the sources are the numbers `x`
//!   with `2x > n`, joined to their maximal factors by a union-find; in each component, from the largest source
//!   down, a source is kept if the kept ones plus it *dismantle completely* (repeatedly remove a member that has a
//!   maximal factor that no other remaining member lists; every member must go).  Item: `{"plain_only": [..],
//!   "greedy_only": [..]}`, the numbers in one set and not the other.
//! * **H** (the claims in Π⁺(∅)): the extended problem with nothing open and the claims of `unzip` given
//!   ([`taxman::frames::build_extended`] with `Bits::EMPTY` and the search frames' claims; docs/THEORY.md, "The
//!   claims of `unzip` in Π⁺(O)") is the frame model's problem: it refuses no claim, keeps every claim of the search's
//!   frames in the order made, has the search's frames and candidates, and poses the same search problem
//!   ([`taxman::search::problem_of_extended`] of its frames and candidates equals [`taxman::search::problem_of`] of
//!   the search's).  Item, one for each thing that differs: `{"what": "refused", "count": k, "claims": [[s, f], ..]}`
//!   (the first 20), `{"what": "claims", "at": i, "kept": k, "given": g}` (the first place where the claims kept
//!   differ from those given, and how many each), `{"what": "frames", "count": k, "expected": e}` or `{"what":
//!   "frames", "frame": h}` (a different number of frames, or frame `h` differs), `{"what": "candidates",
//!   "extended_only": [..], "frame_model_only": [..]}`, `{"what": "problem"}`.  Every board to the largest the solver
//!   supports can be checked: Π⁺(∅) never has more frames than the frame model.
//!
//! **A line.**  `n`; `w`, the set width; `secs`, the wall seconds for the board.  The counts: `frames` (as setup forms
//! them, before any merge), `numbers` (the search's frames, one per number), `candidates`, `max_layer`.  By layer, as
//! arrays indexed by layer `0..=max_layer`: `frames_by_layer`, `closed_by_layer`, `freedoms_by_layer` (the sum of the
//! frames' freedoms), `kept_by_layer` (the sum of their sources, mandatory ones included), `impossible_by_layer`.
//! The numbering: `frame0_layers`, the sorted set of the layers of the frames numbered 0 (this is no check);
//! `number_layers`, for each number the sorted set of the layers of its frames; `layer_numbering`, true if every
//! frame with freedoms has the number `layer - l0`, `l0` being the lowest layer of a frame with freedoms (true when
//! there is none); `layer2_closed`, true if every frame of layer 2 is closed (true when there is none).  What `unzip`
//! claimed: `claims_pre`, `[front, back]` counts over all the frames as setup forms them, and `claims_numbered`, the
//! same over the search's merged frames; `back_with_composite_count`, the number of back claims `(s, f)` of the
//! search's frames whose factor `f` has a maximal composite `c = f*p` that is a factor of some numbered frame as setup
//! forms it (the boards where check D has something to check), and `back_with_composite`, the first 20 as `[s, f,
//! c]` with the smallest such `c`.  `merged_away`, `[reached, true]` (`BoardFrames::merged_away_reach`): how often
//! `unzip`, asking whether a factor has a use outside its frame, met a number homed in a frame that has since been
//! merged away, and how often that number counted as such a use.  `violations`: an object with a key for each failed
//! check (`"A"`, `"C"`, `"D"`, `"E"`, `"F"`, `"G"`, `"H"`) whose value is the first 20 of its items, and `"A_count"`,
//! ... with how many there are; `{}` when everything holds.  `extended_frames`, `[count, lowest, highest]`: the
//! frames of the extended problem Π⁺(O) with every source of three or more prime factors open, the most frames any O
//! gives (`taxman::frames::extended_frame_count`; docs/THEORY.md, "The extended problem"), and the layers they
//! span (`[0, null, null]` when no frame has freedoms there); the search holds at most `MAX_FRAMES` = 9.
//!
//! **Lemma M5⁺'s check.**  Π⁺(O) keeps a back claim `(s, f)` of the search's frames only if `f` passes the check
//! ([`taxman::frames::ClaimCheck`]): it is no two-sided candidate of Π⁺(O), and no two-sided candidate has it among its
//! maximal factors (docs/THEORY.md, "The claims of `unzip` in Π⁺(O)"); a component that fails it for one back
//! claim keeps none of its claims, front claims included; so does one that holds a source in O.  The candidates of
//! Π⁺(O) only grow with O, so a back claim fails the check for some O exactly when it fails for the largest O that
//! holds none of its component's sources: U\*_{≥3} (the kept sources of three or more prime factors) less the
//! component's own sources, or all of U\*_{≥3} for a component of layer 1 or 2, which no O touches.
//! Two pairs of fields count them, each a count and the first 20 as `[s, f]` (the source and the factor of the back
//! claim, in the order the claims were made), all four `null` where Π⁺(U\*_{≥3}) needs more than `MAX_FRAMES` frames
//! and so cannot be built (`extended_frames` above 9: every board from 4096):
//!
//! * `m5_at_risk_count` and `m5_at_risk`: the back claims of the search's frames *at risk*, whose factor fails the
//!   check against the two-sided candidates of Π⁺(U\*_{≥3}).  Failing there is necessary for failing for any O (the
//!   candidates of every O are among them), so no other back claim is ever refused.
//! * `m5_refused_count` and `m5_refused`: the back claims *some O refuses*, exactly.  Only the components that hold an
//!   at-risk back claim are built (`taxman::frames::build_extended` with the largest O free of the component and the
//!   search frames' claims), and of the claims the build refuses, those of the component's back claims that fail the
//!   check against that problem's candidates are listed: the claims the component loses along with them (its other
//!   back claims that pass, its front claims) are not.  Every pair listed is at risk, which the census checks.
//!
//! **The summary** is computed from the lines of `FROM..=TO` (with `--out`, read again from FILE at the end, so a
//! resumed run summarises everything; without it, from memory) and written as plain text to stdout (to stderr when the
//! lines went to stdout): the boards checked; for each of A, C, D, E, F, G and H the boards that fail it, with the
//! first items of the first 5; the sets of layers that frame 0 had, with the number of boards of each, and the boards
//! where frame 0 includes a frame of layer 2, as ranges; the boards whose `layer_numbering` is false, those where a
//! number's frames span several layers, and those whose `layer2_closed` is false; the totals of `claims_pre` and
//! `claims_numbered`; the boards with back claims that have a composite among the factors; the boards where
//! `merged_away` was reached, and the totals; the largest `frames`, `numbers`, `max_layer` and `candidates`; the
//! boards by the frame count of `extended_frames`, and those where it is above `MAX_FRAMES`; for Lemma M5⁺'s check,
//! the boards with back claims at risk (how many, the total of their claims, the first as ranges), the boards beyond
//! the reach of the check (the fields are `null`), the boards where some O refuses a back claim (as ranges, and the
//! items of the first 5), and the boards whose lines lack the fields; the seconds, summed over the boards.  The last
//! line is `PASS` or `FAIL`.
//!
//! **Lines from an earlier version.**  A line of a file written before the census had check H lacks the four fields
//! (`violations` lists only what failed, so the fields are the sign that H ran).  Resuming into such a file does not
//! crash: those lines are left out of the three rows about Lemma M5⁺'s check, and the last row counts their boards.
//! But check H did not run on them, so the run is a `FAIL` while any exists: run the range again into a new file.
//!
//! **Exit status**: 0 if every board of the range has a line with the Lemma M5⁺ fields and passes A, C, D, E, F, G and
//! H; 1 if one does not, or a board could not be analysed (the message goes to stderr); 2 for a usage error.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Instant;
use taxman::accelerate::solution_for_promotions;
use taxman::bits::Bits;
use taxman::frames::{
    build_extended, extended_frame_count, numbering, Claim, ClaimCheck, ClaimKind, NumberedFrame, Numbering,
    SearchFrames,
};
use taxman::json::Json;
use taxman::model::{FrameGraph, MAX_FRAMES};
use taxman::numtheory::Tables;
use taxman::search::{problem_of, problem_of_extended};
use taxman::solver::MAX_N;

const USAGE: &str = "usage: structure FROM TO [--threads K] [--out FILE]   (2 <= FROM <= TO)";

/// The checks, in the order of the summary.
const CHECKS: [&str; 7] = ["A", "C", "D", "E", "F", "G", "H"];

/// How many items of a failed check, and how many back claims with a composite or at risk, a line keeps.
const ITEM_LIMIT: usize = 20;

/// What the command line asks for.
struct Args {
    from: u32,
    to: u32,
    threads: usize,
    out: Option<PathBuf>,
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let (range, flags) = args.split_at(2.min(args.len()));
    let [from, to] = range else { return Err(USAGE.to_owned()) };
    let board = |text: &String| text.parse::<u32>().map_err(|_| USAGE.to_owned());
    let (from, to) = (board(from)?, board(to)?);
    let (mut threads, mut out) = (1, None);
    let mut flags = flags.iter();
    while let Some(flag) = flags.next() {
        match (flag.as_str(), flags.next()) {
            ("--threads", Some(k)) => threads = k.parse().ok().filter(|&k| k >= 1).ok_or(USAGE)?,
            ("--out", Some(path)) => out = Some(PathBuf::from(path)),
            _ => return Err(USAGE.to_owned()),
        }
    }
    if from < 2 || from > to || to > MAX_N {
        return Err(format!("{USAGE}\nthe boards must be from 2 to {MAX_N}"));
    }
    Ok(Args { from, to, threads, out })
}

// ---------------------------------------------------------------------------------------------------------
// The census of one board

/// Omega of every number `0..=n`: the number of its prime factors with multiplicity (0 for 0 and 1).
fn omega_table(t: &Tables) -> Vec<u32> {
    let mut omega = vec![0; t.n as usize + 1];
    for x in 2..=t.n as usize {
        omega[x] = omega[x / t.spf[x] as usize] + 1;
    }
    omega
}

/// The layer of a frame as setup forms it: Omega of its first source (sources and impossible sources alike; check A
/// asks whether the others agree), or Omega of its smallest factor plus one if it has no source.
fn layer_of<const W: usize>(frame: &NumberedFrame<W>, omega: &[u32]) -> usize {
    match (frame.sources | frame.impossible).min() {
        Some(x) => omega[x as usize] as usize,
        None => frame.factors.min().map_or(0, |f| omega[f as usize] as usize + 1),
    }
}

/// `Json` for an optional number: `null` for none.
fn optional(number: Option<usize>) -> Json {
    number.map_or(Json::Null, Json::from)
}

/// Check A: the layers of the frames and of what they hold and have in front of them.
fn check_layers<const W: usize>(numbered: &Numbering<W>, layers: &[usize], omegas: &[u32]) -> Vec<Json> {
    let omega = |x: u32| omegas[x as usize] as usize;
    let mut items = Vec::new();
    for (frame, &k) in numbered.frames.iter().zip(layers) {
        let off_sources: Vec<u32> = (frame.sources | frame.impossible).iter().filter(|&x| omega(x) != k).collect();
        let off_factors: Vec<u32> = frame.factors.iter().filter(|&f| omega(f) + 1 != k).collect();
        let off_in_front: Vec<&str> = (frame.in_front.iter())
            .filter(|&&j| layers[j] + 1 != k)
            .map(|&j| numbered.frames[j].name.as_str())
            .collect();
        if off_sources.is_empty() && off_factors.is_empty() && off_in_front.is_empty() {
            continue;
        }
        let mut item = vec![("frame", Json::from(frame.name.as_str())), ("layer", Json::from(k))];
        for (key, list) in [("sources", Json::array(off_sources)), ("factors", Json::array(off_factors))] {
            if list.as_arr().is_some_and(|list| !list.is_empty()) {
                item.push((key, list));
            }
        }
        if !off_in_front.is_empty() {
            item.push(("in_front", Json::array(off_in_front)));
        }
        items.push(Json::object(item));
    }
    items
}

/// Check C: the numbering is clean, and every frame with freedoms has a number.
fn check_clean_numbering<const W: usize>(numbered: &Numbering<W>) -> Vec<Json> {
    let mut items = Vec::new();
    for frame in &numbered.frames {
        let name = Json::from(frame.name.as_str());
        let Some(h) = frame.number else {
            if !frame.closed {
                items.push(Json::Arr(vec![name, Json::Null, Json::Null, Json::Null]));
            }
            continue;
        };
        for &x in &frame.in_front {
            let in_front = &numbered.frames[x];
            if !in_front.closed && (h == 0 || in_front.number != Some(h - 1)) {
                let item = [name.clone(), Json::from(h), Json::from(in_front.name.as_str()), optional(in_front.number)];
                items.push(Json::Arr(item.to_vec()));
            }
        }
    }
    items
}

/// Check D: no back claim of the search's frames takes a factor that a candidate lists among its maximal factors.
fn check_back_claims<const W: usize>(t: &Tables, frames: &SearchFrames<W>) -> Vec<Json> {
    let mut items = Vec::new();
    for (i, claims) in frames.claims.iter().enumerate() {
        for &(s, f, _) in claims.iter().filter(|claim| claim.2 == ClaimKind::Back) {
            // `c` has `f` among its maximal factors exactly when `c` is a maximal composite of `f`.
            for &c in t.max_composites[f as usize].iter().filter(|&&c| frames.candidates.contains(c)) {
                items.push(Json::Arr(vec![Json::from(i), Json::from(s), Json::from(f), Json::from(c)]));
            }
        }
    }
    items
}

/// Check E: the frame as setup forms it whose factors hold a candidate has a number of at least 1.
fn check_candidates_not_in_front<const W: usize>(numbered: &Numbering<W>, frames: &SearchFrames<W>) -> Vec<Json> {
    let mut items = Vec::new();
    for c in frames.candidates.iter() {
        let number = numbered.frames.iter().find(|frame| frame.factors.contains(c)).and_then(|frame| frame.number);
        if number.is_none_or(|h| h == 0) {
            items.push(Json::Arr(vec![Json::from(c), optional(number)]));
        }
    }
    items
}

/// Check F: the search's model gives every candidate a home frame of at least 1.
fn check_homes<const W: usize>(graph: &FrameGraph, frames: &SearchFrames<W>) -> Vec<Json> {
    frames.candidates.iter().filter(|&c| graph.home(c) < 1).map(Json::from).collect()
}

/// The root of `x` in a union-find forest, halving the path on the way.
fn find(parent: &mut [u32], mut x: u32) -> u32 {
    while parent[x as usize] != x {
        let grandparent = parent[parent[x as usize] as usize];
        parent[x as usize] = grandparent;
        x = grandparent;
    }
    x
}

/// Whether `members` dismantle completely: repeatedly a member that has a maximal factor that no other remaining
/// member lists is removed, and they do if every one is.  `count` and `owner` are scratch arrays by number, all zero
/// on entry and on exit: for each factor, how many remaining members list it and the exclusive or of their numbers
/// (which is the member itself when there is just one).
fn dismantles(t: &Tables, members: &[u32], count: &mut [u32], owner: &mut [u32]) -> bool {
    let factors_of = |m: u32| t.max_factors[m as usize].iter().copied();
    for &m in members {
        for f in factors_of(m) {
            count[f as usize] += 1;
            owner[f as usize] ^= m;
        }
    }
    let mut worklist: Vec<u32> =
        members.iter().flat_map(|&m| factors_of(m)).filter(|&f| count[f as usize] == 1).collect();
    let mut removed = 0;
    while let Some(f) = worklist.pop() {
        if count[f as usize] != 1 {
            continue; // its only member has gone, by another factor
        }
        let m = owner[f as usize];
        removed += 1;
        for g in factors_of(m) {
            count[g as usize] -= 1;
            owner[g as usize] ^= m;
            if count[g as usize] == 1 {
                worklist.push(g);
            }
        }
    }
    for &m in members {
        for f in factors_of(m) {
            (count[f as usize], owner[f as usize]) = (0, 0);
        }
    }
    removed == members.len()
}

/// The numbers above `n/2` that a descending greedy keeps, with no frames code (see check G in the module comment),
/// ascending.
fn greedy_upper(t: &Tables) -> Vec<u32> {
    let sources: Vec<u32> = (1..=t.n).filter(|&x| 2 * x > t.n).collect();
    let mut parent: Vec<u32> = (0..=t.n).collect();
    for &x in &sources {
        for &f in &t.max_factors[x as usize] {
            let (a, b) = (find(&mut parent, x), find(&mut parent, f));
            parent[a as usize] = b;
        }
    }
    let mut components: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &x in &sources {
        components.entry(find(&mut parent, x)).or_default().push(x);
    }

    let (mut count, mut owner) = (vec![0; t.n as usize + 1], vec![0; t.n as usize + 1]);
    let mut all_kept = Vec::new();
    for members in components.values() {
        let mut kept: Vec<u32> = Vec::new();
        for &s in members.iter().rev() {
            kept.push(s);
            if !dismantles(t, &kept, &mut count, &mut owner) {
                kept.pop();
            }
        }
        all_kept.extend(kept);
    }
    all_kept.sort_unstable();
    all_kept
}

/// Check G: the numbers above `n/2` in the game with no promotions are those the greedy keeps.
fn check_upper(t: &Tables, plain: &[u32]) -> Vec<Json> {
    let in_plain: BTreeSet<u32> = plain.iter().copied().filter(|&x| 2 * x > t.n).collect();
    let in_greedy: BTreeSet<u32> = greedy_upper(t).into_iter().collect();
    if in_plain == in_greedy {
        return Vec::new();
    }
    let plain_only: Vec<u32> = in_plain.difference(&in_greedy).copied().collect();
    let greedy_only: Vec<u32> = in_greedy.difference(&in_plain).copied().collect();
    vec![Json::object([("plain_only", Json::array(plain_only)), ("greedy_only", Json::array(greedy_only))])]
}

/// The count of `pairs` and its first `ITEM_LIMIT` members as `[s, f]`, the two fields of a line that list pairs
/// (both `null` for none).
fn count_and_pairs(pairs: Option<&[(u32, u32)]>) -> (Json, Json) {
    pairs.map_or((Json::Null, Json::Null), |pairs| {
        let first = pairs.iter().take(ITEM_LIMIT).map(|&(s, f)| Json::array([s, f]));
        (Json::from(pairs.len()), Json::Arr(first.collect()))
    })
}

/// An item of check H: what differs, and the details.
fn differs(what: &str, details: impl IntoIterator<Item = (&'static str, Json)>) -> Json {
    Json::object(std::iter::once(("what", Json::from(what))).chain(details))
}

/// Check H: the extended problem Π⁺(∅) with the claims of `unzip` is the frame model's problem.
fn check_extended_is_frame_model<const W: usize>(t: &Tables, frames: &SearchFrames<W>) -> Result<Vec<Json>, String> {
    let extended = build_extended::<W>(t, &Bits::EMPTY, &frames.claims)?;
    let mut items = Vec::new();
    if !extended.refused.is_empty() {
        let refused: Vec<(u32, u32)> = extended.refused.iter().map(|&(s, f, _)| (s, f)).collect();
        let (count, claims) = count_and_pairs(Some(&refused));
        items.push(differs("refused", [("count", count), ("claims", claims)]));
    }
    let given: Vec<Claim> = frames.claims.iter().flatten().copied().collect();
    if extended.claims != given {
        let at = extended.claims.iter().zip(&given).take_while(|(kept, given)| kept == given).count();
        let sizes = [("at", at), ("kept", extended.claims.len()), ("given", given.len())];
        items.push(differs("claims", sizes.map(|(key, size)| (key, Json::from(size)))));
    }
    if extended.frames.len() != frames.frames.len() {
        let sizes = [("count", extended.frames.len()), ("expected", frames.frames.len())];
        items.push(differs("frames", sizes.map(|(key, size)| (key, Json::from(size)))));
    } else {
        let off = (extended.frames.iter().zip(&frames.frames)).enumerate().filter(|(_, (ours, theirs))| ours != theirs);
        items.extend(off.map(|(h, _)| differs("frames", [("frame", Json::from(h))])));
    }
    if extended.candidates != frames.candidates {
        let (ours, theirs) = (extended.candidates, frames.candidates);
        items.push(differs(
            "candidates",
            [
                ("extended_only", Json::array((ours - theirs).to_vec())),
                ("frame_model_only", Json::array((theirs - ours).to_vec())),
            ],
        ));
    }
    let ours = problem_of_extended::<W>(t, &extended.columns(), &extended.one_sided, &extended.candidates)?;
    if ours != problem_of::<W>(t, &frames.columns(), &frames.candidates)? {
        items.push(differs("problem", []));
    }
    Ok(items)
}

/// What the check of Lemma M5⁺ could do to the back claims of the search's frames, as `(s, f)` pairs in the order the
/// claims were made (see the module comment).
struct ClaimRisk {
    /// The back claims whose factor fails the check against the two-sided candidates of Π⁺(U\*_{≥3}).
    at_risk: Vec<(u32, u32)>,
    /// The back claims that fail the check for some O: a subset of those at risk.
    refused: Vec<(u32, u32)>,
}

/// The `ClaimRisk` of the board `1..=t.n`, given its `numbered` frames as setup forms them with their `layers`, the
/// search's `frames`, and `upper`, U\*_{≥3}, the sources of the frames of layer 3 or more.  The caller has seen that
/// Π⁺(`upper`) has no more than `MAX_FRAMES` frames, so that it can be built.
fn claim_risk<const W: usize>(
    t: &Tables,
    numbered: &Numbering<W>,
    layers: &[usize],
    frames: &SearchFrames<W>,
    upper: &Bits<W>,
) -> Result<ClaimRisk, String> {
    let check = ClaimCheck::new(t, &build_extended::<W>(t, upper, &[])?.candidates);
    let back_claims = frames.claims.iter().flatten().filter(|claim| claim.2 == ClaimKind::Back);
    let at_risk: Vec<(u32, u32)> =
        back_claims.filter(|&&(_, f, _)| !check.passes(f)).map(|&(s, f, _)| (s, f)).collect();

    // The components that hold an at-risk back claim, each with the largest O free of it: a component of layer 1 or 2
    // is free of every O, so those share the largest, all of U*_{>=3}.
    let component_of = |s: u32| {
        let found = numbered.frames.iter().position(|component| component.sources.contains(s));
        found.ok_or_else(|| format!("{s} claims a factor and is the source of no frame"))
    };
    let components = at_risk.iter().map(|&(s, _)| component_of(s)).collect::<Result<BTreeSet<usize>, String>>()?;
    let (low, high): (Vec<usize>, Vec<usize>) = components.into_iter().partition(|&i| layers[i] < 3);
    let mut builds: Vec<(Bits<W>, Bits<W>)> = Vec::new();
    if !low.is_empty() {
        builds.push((*upper, low.iter().fold(Bits::EMPTY, |all, &i| all | numbered.frames[i].sources)));
    }
    builds.extend(high.iter().map(|&i| (*upper - numbered.frames[i].sources, numbered.frames[i].sources)));

    // Of the claims a build refuses, the back claims of its components that fail the check against its candidates.
    let mut refused = BTreeSet::new();
    for (largest, sources) in builds {
        let extended = build_extended::<W>(t, &largest, &frames.claims)?;
        let check = ClaimCheck::new(t, &extended.candidates);
        let failing = (extended.refused.iter())
            .filter(|&&(s, f, kind)| kind == ClaimKind::Back && sources.contains(s) && !check.passes(f));
        refused.extend(failing.map(|&(s, f, _)| (s, f)));
    }
    let refused_in_order: Vec<(u32, u32)> = at_risk.iter().copied().filter(|pair| refused.contains(pair)).collect();
    if refused_in_order.len() != refused.len() {
        return Err(format!("back claims refused for an O that are not at risk: {refused:?}"));
    }
    Ok(ClaimRisk { at_risk, refused: refused_in_order })
}

/// For each layer `0..=max_layer`, the sum of `value` over the frames of that layer.
fn by_layer<const W: usize>(
    numbered: &Numbering<W>,
    layers: &[usize],
    max_layer: usize,
    value: impl Fn(&NumberedFrame<W>) -> i64,
) -> Json {
    let mut sums = vec![0i64; max_layer + 1];
    for (frame, &layer) in numbered.frames.iter().zip(layers) {
        sums[layer] += value(frame);
    }
    Json::array(sums)
}

/// The layers of the frames for which `wanted` holds.
fn layer_set<const W: usize>(
    numbered: &Numbering<W>,
    layers: &[usize],
    wanted: impl Fn(&NumberedFrame<W>) -> bool,
) -> BTreeSet<usize> {
    (numbered.frames.iter().zip(layers)).filter(|(frame, _)| wanted(frame)).map(|(_, &layer)| layer).collect()
}

/// Whether every frame with freedoms has the number `layer - l0`, `l0` being the lowest layer of a frame with freedoms
/// (true when there is none).
fn numbers_follow_layers<const W: usize>(numbered: &Numbering<W>, layers: &[usize]) -> bool {
    let with_freedoms = || (numbered.frames.iter().zip(layers)).filter(|(frame, _)| !frame.closed);
    let lowest = with_freedoms().map(|(_, &layer)| layer).min();
    with_freedoms().all(|(frame, &layer)| lowest.is_some_and(|l0| frame.number == Some(layer - l0)))
}

/// Whether every frame of layer 2 is closed (true when there is none).
fn layer_2_is_closed<const W: usize>(numbered: &Numbering<W>, layers: &[usize]) -> bool {
    (numbered.frames.iter().zip(layers)).all(|(frame, &layer)| layer != 2 || frame.closed)
}

/// The back claims `(s, f)` of the search's frames whose factor `f` has a maximal composite that is a factor of a
/// numbered frame as setup forms it, as `[s, f, c]` with the smallest such composite `c`.
fn back_claims_with_composite<const W: usize>(
    t: &Tables,
    numbered: &Numbering<W>,
    frames: &SearchFrames<W>,
) -> Vec<Json> {
    let numbered_factors = (numbered.frames.iter())
        .filter(|frame| frame.number.is_some())
        .fold(Bits::<W>::EMPTY, |all, frame| all | frame.factors);
    let back_claims = frames.claims.iter().flatten().filter(|claim| claim.2 == ClaimKind::Back);
    back_claims
        .filter_map(|&(s, f, _)| {
            let c = t.max_composites[f as usize].iter().find(|&&c| numbered_factors.contains(c))?;
            Some(Json::Arr(vec![Json::from(s), Json::from(f), Json::from(*c)]))
        })
        .collect()
}

/// The `violations` of a line: for each check that failed (`CHECKS` in order) its first `ITEM_LIMIT` items and, as
/// `"<check>_count"`, how many there are.
fn failed_checks(violations: &[Vec<Json>; CHECKS.len()]) -> Json {
    let mut failed = Vec::new();
    for (check, items) in CHECKS.iter().zip(violations).filter(|(_, items)| !items.is_empty()) {
        failed.push((check.to_string(), Json::Arr(items.iter().take(ITEM_LIMIT).cloned().collect())));
        failed.push((format!("{check}_count"), Json::from(items.len())));
    }
    Json::object(failed)
}

/// `[front, back]`: how many of `claims` are of each kind.
fn count_claims<'a>(claims: impl Iterator<Item = &'a (u32, u32, ClaimKind)>) -> Json {
    let (mut front, mut back) = (0u32, 0u32);
    for claim in claims {
        match claim.2 {
            ClaimKind::Front => front += 1,
            ClaimKind::Back => back += 1,
        }
    }
    Json::array([front, back])
}

/// The census of the board `1..=n` at the set width `W`.
fn census<const W: usize>(n: u32) -> Result<Json, String> {
    let start = Instant::now();
    let t = Tables::new(n);
    let omega = omega_table(&t);
    let (numbered, frames) = numbering::<W>(&t);
    let plain = solution_for_promotions::<W>(&t, Bits::EMPTY);
    let (graph, _root) = FrameGraph::build::<W>(&t, &frames.columns())?;

    let layers: Vec<usize> = numbered.frames.iter().map(|frame| layer_of(frame, &omega)).collect();
    let max_layer = layers.iter().copied().max().unwrap_or(0);
    let violations = [
        check_layers(&numbered, &layers, &omega),
        check_clean_numbering(&numbered),
        check_back_claims(&t, &frames),
        check_candidates_not_in_front(&numbered, &frames),
        check_homes(&graph, &frames),
        check_upper(&t, &plain),
        check_extended_is_frame_model(&t, &frames)?,
    ];

    // The layers of frame 0 and of each number.
    let frame0_layers = layer_set(&numbered, &layers, |frame| frame.number == Some(0));
    let numbers = frames.frames.len();
    let number_layers: Vec<BTreeSet<usize>> =
        (0..numbers).map(|h| layer_set(&numbered, &layers, |frame| frame.number == Some(h))).collect();
    let back_with_composite = back_claims_with_composite(&t, &numbered, &frames);

    let frame_count = |value: fn(&NumberedFrame<W>) -> i64| by_layer(&numbered, &layers, max_layer, value);
    let (reached, answered) = numbered.merged_away_reach;

    // The extended problem with every source of three or more prime factors open, and what Lemma M5⁺'s check does to
    // the back claims there, where it can be built.
    let from_layer_3 = numbered.frames.iter().zip(&layers).filter(|&(_, &layer)| layer >= 3);
    let upper = from_layer_3.fold(Bits::<W>::EMPTY, |acc, (frame, _)| acc | frame.sources);
    let (extended, span) = extended_frame_count::<W>(&t, &upper)?;
    let extended_frames = match span {
        Some((lowest, highest)) => Json::array([extended as u64, u64::from(lowest), u64::from(highest)]),
        None => Json::Arr(vec![Json::from(extended), Json::Null, Json::Null]),
    };
    let risk = if extended <= MAX_FRAMES { Some(claim_risk(&t, &numbered, &layers, &frames, &upper)?) } else { None };
    let (at_risk_count, at_risk) = count_and_pairs(risk.as_ref().map(|risk| risk.at_risk.as_slice()));
    let (refused_count, refused) = count_and_pairs(risk.as_ref().map(|risk| risk.refused.as_slice()));
    Ok(Json::object([
        ("n", Json::from(n)),
        ("w", Json::from(W)),
        ("secs", Json::from((start.elapsed().as_secs_f64() * 1000.0).round() / 1000.0)),
        ("frames", Json::from(numbered.frames.len())),
        ("numbers", Json::from(frames.frames.len())),
        ("candidates", Json::from(frames.candidates.len())),
        ("max_layer", Json::from(max_layer)),
        ("frames_by_layer", frame_count(|_| 1)),
        ("closed_by_layer", frame_count(|frame| i64::from(frame.closed))),
        ("freedoms_by_layer", frame_count(|frame| i64::from(frame.freedoms))),
        ("kept_by_layer", frame_count(|frame| i64::from(frame.sources.len()))),
        ("impossible_by_layer", frame_count(|frame| i64::from(frame.impossible.len()))),
        ("frame0_layers", Json::array(frame0_layers)),
        ("number_layers", Json::Arr(number_layers.into_iter().map(Json::array).collect())),
        ("layer_numbering", Json::from(numbers_follow_layers(&numbered, &layers))),
        ("layer2_closed", Json::from(layer_2_is_closed(&numbered, &layers))),
        ("claims_pre", count_claims(numbered.frames.iter().flat_map(|frame| &frame.claims))),
        ("claims_numbered", count_claims(frames.claims.iter().flatten())),
        ("back_with_composite_count", Json::from(back_with_composite.len())),
        ("back_with_composite", Json::Arr(back_with_composite.into_iter().take(ITEM_LIMIT).collect())),
        ("merged_away", Json::array([reached, answered])),
        ("violations", failed_checks(&violations)),
        ("extended_frames", extended_frames),
        ("m5_at_risk_count", at_risk_count),
        ("m5_at_risk", at_risk),
        ("m5_refused_count", refused_count),
        ("m5_refused", refused),
    ]))
}

/// The census of the board `1..=n` at the set width the solver gives it.
fn census_of(n: u32) -> Result<Json, String> {
    taxman::with_width!(n, W => census::<W>(n))
}

// ---------------------------------------------------------------------------------------------------------
// The summary

/// The field `key` of a line as a number (0 if it is missing).
fn number_of(line: &Json, key: &str) -> u64 {
    line.get(key).and_then(Json::as_u64).unwrap_or(0)
}

/// The elements of the array `key` of a line (none if it is missing).
fn elements<'a>(line: &'a Json, key: &str) -> &'a [Json] {
    line.get(key).and_then(Json::as_arr).unwrap_or_default()
}

/// The pair of numbers `key` of a line.
fn pair_of(line: &Json, key: &str) -> (u64, u64) {
    let at = |i| elements(line, key).get(i).and_then(Json::as_u64).unwrap_or(0);
    (at(0), at(1))
}

/// What a line says of a count of back claims for Lemma M5⁺'s check (`key`: `m5_at_risk_count` or `m5_refused_count`):
/// `None` if the line lacks the field (an earlier version of the census wrote it), `Some(None)` if it is `null`
/// (Π⁺ of every source of three or more prime factors cannot be built), else `Some(Some(count))`.
fn claims_reported(line: &Json, key: &str) -> Option<Option<u64>> {
    line.get(key).map(Json::as_u64)
}

/// The items of the check `check` that a line lists as failed.
fn violations_of<'a>(line: &'a Json, check: &str) -> &'a [Json] {
    line.get("violations").map_or(&[][..], |violations| elements(violations, check))
}

/// `boards` (ascending) as ranges: `8, 12-14, 18-24`.
fn ranges(boards: &[u32]) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut at = 0;
    while at < boards.len() {
        let mut end = at;
        while end + 1 < boards.len() && boards[end + 1] == boards[end] + 1 {
            end += 1;
        }
        parts.push(if end > at { format!("{}-{}", boards[at], boards[end]) } else { boards[at].to_string() });
        at = end + 1;
    }
    if parts.is_empty() {
        "none".to_owned()
    } else {
        parts.join(", ")
    }
}

/// The first `k` of `boards`, with `...` when there are more.
fn first(boards: &[u32], k: usize) -> String {
    let shown: Vec<String> = boards.iter().take(k).map(u32::to_string).collect();
    let more = if boards.len() > k { ", ..." } else { "" };
    if shown.is_empty() {
        "none".to_owned()
    } else {
        format!("{}{more}", shown.join(", "))
    }
}

/// The first `k` of `boards` (ascending) as ranges, with `...` when there are more.
fn first_ranges(boards: &[u32], k: usize) -> String {
    let more = if boards.len() > k { ", ..." } else { "" };
    format!("{}{more}", ranges(&boards[..boards.len().min(k)]))
}

/// The boards of `lines` for which `wanted` holds.
fn boards_where(lines: &[Json], wanted: impl Fn(&Json) -> bool) -> Vec<u32> {
    lines.iter().filter(|line| wanted(line)).map(|line| number_of(line, "n") as u32).collect()
}

/// The summary of the lines of the boards `from..=to` (see the module comment) and whether the range passes.
fn summary(lines: &[Json], from: u32, to: u32) -> (String, bool) {
    let mut rows: Vec<String> = Vec::new();
    let expected = (to - from + 1) as usize;
    rows.push(format!("boards checked: {} of {from}..={to}", lines.len()));
    let present: BTreeSet<u32> = lines.iter().map(|line| number_of(line, "n") as u32).collect();
    let missing: Vec<u32> = (from..=to).filter(|n| !present.contains(n)).collect();
    if !missing.is_empty() {
        rows.push(format!("missing boards ({} of {expected}): {}", missing.len(), ranges(&missing)));
    }

    // The checks.
    let mut passes = missing.is_empty();
    for check in CHECKS {
        let failing: Vec<&Json> = lines.iter().filter(|line| !violations_of(line, check).is_empty()).collect();
        passes &= failing.is_empty();
        let shown: Vec<String> = (failing.iter().take(5))
            .map(|line| {
                let items: Vec<String> = violations_of(line, check).iter().take(3).map(Json::to_string).collect();
                format!("n={}: {}", number_of(line, "n"), items.join(" "))
            })
            .collect();
        let tail = if failing.is_empty() { String::new() } else { format!("; first: {}", shown.join("; ")) };
        rows.push(format!("check {check}: {} boards fail{tail}", failing.len()));
    }

    // Where frame 0 sits.
    let mut sets: BTreeMap<String, usize> = BTreeMap::new();
    for line in lines {
        let layers: Vec<String> = elements(line, "frame0_layers").iter().map(Json::to_string).collect();
        *sets.entry(format!("{{{}}}", layers.join(","))).or_default() += 1;
    }
    let sets: Vec<String> = sets.iter().map(|(set, boards)| format!("{set}: {boards} boards")).collect();
    rows.push(format!("layers of frame 0: {}", sets.join("; ")));
    let on_layer_2 = boards_where(lines, |line| elements(line, "frame0_layers").iter().any(|l| l.as_u64() == Some(2)));
    rows.push(format!("boards where frame 0 includes layer 2 ({}): {}", on_layer_2.len(), ranges(&on_layer_2)));

    // How the numbers follow the layers.
    let off = boards_where(lines, |line| line.get("layer_numbering").and_then(Json::as_bool) == Some(false));
    rows.push(format!("boards where the numbers do not follow the layers ({}): first {}", off.len(), first(&off, 10)));
    let spans = boards_where(lines, |line| {
        elements(line, "number_layers").iter().any(|set| set.as_arr().is_some_and(|set| set.len() > 1))
    });
    rows.push(format!(
        "boards where a number's frames span several layers ({}): first {}",
        spans.len(),
        first(&spans, 10)
    ));
    let open = boards_where(lines, |line| line.get("layer2_closed").and_then(Json::as_bool) == Some(false));
    rows.push(format!("boards where a frame of layer 2 has freedoms ({}): {}", open.len(), ranges(&open)));

    // What unzip claimed.
    let total = |key: &str| lines.iter().map(|line| pair_of(line, key)).fold((0, 0), |(a, b), (x, y)| (a + x, b + y));
    let (pre, numbered) = (total("claims_pre"), total("claims_numbered"));
    rows.push(format!("claims as setup forms the frames: {} front, {} back", pre.0, pre.1));
    rows.push(format!("claims in the search's frames: {} front, {} back", numbered.0, numbered.1));
    let composite: Vec<(u32, u64)> = (lines.iter())
        .map(|line| (number_of(line, "n") as u32, number_of(line, "back_with_composite_count")))
        .filter(|&(_, count)| count > 0)
        .collect();
    let listed: Vec<String> = composite.iter().take(20).map(|(n, count)| format!("{n} ({count})")).collect();
    let more = if composite.len() > 20 { ", ..." } else { "" };
    rows.push(format!(
        "boards with back claims that have a composite among the factors ({}): {}{more}",
        composite.len(),
        listed.join(", ")
    ));

    // The quirk of the merged-away frames.
    let reached = boards_where(lines, |line| pair_of(line, "merged_away").0 > 0);
    let reach = total("merged_away");
    rows.push(format!(
        "merged_away: reached on {} boards ({}), {} times, answered yes {} times",
        reached.len(),
        ranges(&reached),
        reach.0,
        reach.1
    ));

    // The largest.
    let largest = |key: &str| {
        let most = lines.iter().map(|line| number_of(line, key)).max().unwrap_or(0);
        let at = lines.iter().find(|line| number_of(line, key) == most).map_or(0, |line| number_of(line, "n"));
        format!("{key} {most} (n={at})")
    };
    let sizes: Vec<String> = ["frames", "numbers", "max_layer", "candidates"].iter().map(|key| largest(key)).collect();
    rows.push(format!("largest: {}", sizes.join(", ")));

    // The frames of the extended problem with every source of three or more prime factors open.
    let extended_frames = |line: &Json| elements(line, "extended_frames").first().and_then(Json::as_u64).unwrap_or(0);
    let mut by_count: BTreeMap<u64, Vec<u32>> = BTreeMap::new();
    for line in lines {
        by_count.entry(extended_frames(line)).or_default().push(number_of(line, "n") as u32);
    }
    let counts: Vec<String> = by_count
        .iter()
        .map(|(count, boards)| format!("{count}: {} boards (n={})", boards.len(), ranges(boards)))
        .collect();
    rows.push(format!(
        "frames of the extended problem with every source of layer 3 or more open: {}",
        counts.join("; ")
    ));
    let beyond = boards_where(lines, |line| extended_frames(line) > MAX_FRAMES as u64);
    rows.push(format!(
        "boards where it needs more than {MAX_FRAMES} frames ({}): first {}",
        beyond.len(),
        first(&beyond, 10)
    ));

    // Lemma M5⁺'s check on the back claims; a line without the fields is left out.
    let reported = |line: &Json, key: &str| claims_reported(line, key).flatten().filter(|&count| count > 0);
    let number = |line: &Json| number_of(line, "n") as u32;
    let at_risk: Vec<(u32, u64)> =
        lines.iter().filter_map(|line| Some((number(line), reported(line, "m5_at_risk_count")?))).collect();
    let at_risk_boards: Vec<u32> = at_risk.iter().map(|&(n, _)| n).collect();
    rows.push(format!(
        "boards with back claims at risk of Lemma M5⁺ ({}), {} claims: {}",
        at_risk.len(),
        at_risk.iter().map(|&(_, count)| count).sum::<u64>(),
        first_ranges(&at_risk_boards, 30)
    ));
    let out_of_reach = boards_where(lines, |line| claims_reported(line, "m5_at_risk_count") == Some(None));
    rows.push(format!(
        "boards beyond the reach of Lemma M5⁺'s check, Π⁺ of every source of layer 3 or more needing more than \
         {MAX_FRAMES} frames ({}): {}",
        out_of_reach.len(),
        first_ranges(&out_of_reach, 30)
    ));
    let refusing: Vec<&Json> = lines.iter().filter(|line| reported(line, "m5_refused_count").is_some()).collect();
    let refusing_boards: Vec<u32> = refusing.iter().map(|line| number(line)).collect();
    let shown: Vec<String> = (refusing.iter().take(5))
        .map(|line| {
            let items: Vec<String> = elements(line, "m5_refused").iter().take(3).map(Json::to_string).collect();
            format!("n={}: {}", number(line), items.join(" "))
        })
        .collect();
    let tail = if refusing.is_empty() { String::new() } else { format!("; first: {}", shown.join("; ")) };
    rows.push(format!(
        "boards where some O refuses a back claim ({}): {}{tail}",
        refusing.len(),
        first_ranges(&refusing_boards, 30)
    ));
    let lacking = boards_where(lines, |line| claims_reported(line, "m5_at_risk_count").is_none());
    passes &= lacking.is_empty();
    rows.push(format!(
        "boards whose lines lack the Lemma M5⁺ fields (check H did not run on them), left out of the rows above \
         ({}): {}",
        lacking.len(),
        first_ranges(&lacking, 30)
    ));
    let seconds: f64 = lines.iter().map(|line| line.get("secs").and_then(Json::as_f64).unwrap_or(0.0)).sum();
    rows.push(format!("seconds, summed over the boards: {seconds:.1}"));
    rows.push(if passes { "PASS" } else { "FAIL" }.to_owned());
    (rows.join("\n"), passes)
}

// ---------------------------------------------------------------------------------------------------------
// Running

/// Where the lines go.
enum Sink {
    Stdout,
    File(File),
}

impl Sink {
    /// Writes `line` and a newline, and flushes.
    fn write_line(&mut self, line: &str) -> io::Result<()> {
        match self {
            Sink::Stdout => {
                let mut out = io::stdout().lock();
                writeln!(out, "{line}")?;
                out.flush()
            }
            Sink::File(file) => {
                file.write_all(format!("{line}\n").as_bytes())?;
                file.flush()
            }
        }
    }
}

/// The lines of the file `path` that hold a board of `from..=to`, by board size (a missing file has none).  A last
/// line that lacks its newline was cut short by an interruption; with `repair` it is cut off the file, and it is
/// ignored in any case.
fn read_lines(path: &Path, from: u32, to: u32, repair: bool) -> Result<BTreeMap<u32, Json>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let complete = text.rfind('\n').map_or(0, |at| at + 1);
    if repair && complete < text.len() {
        let file = OpenOptions::new().write(true).open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        file.set_len(complete as u64).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let mut lines = BTreeMap::new();
    for (i, line) in text[..complete].lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
        let line = Json::parse(line).map_err(|e| format!("{} line {}: {e}", path.display(), i + 1))?;
        let n =
            line.get("n").and_then(Json::as_u32).ok_or_else(|| format!("{} line {}: no n", path.display(), i + 1))?;
        if (from..=to).contains(&n) {
            lines.insert(n, line);
        }
    }
    Ok(lines)
}

/// Analyses the boards `todo` (ascending) on `threads` threads and writes their lines to `sink` in that order, each as
/// soon as it and all before it are done; returns the lines, in that order, if `keep`.
fn analyse(todo: &[u32], threads: usize, sink: &mut Sink, keep: bool) -> Result<Vec<Json>, String> {
    let (next, stop) = (AtomicUsize::new(0), AtomicBool::new(false));
    let (sender, receiver) = mpsc::channel::<(usize, Result<Json, String>)>();
    let start = Instant::now();
    let mut kept = Vec::new();
    std::thread::scope(|scope| -> Result<(), String> {
        for _ in 0..threads.min(todo.len()) {
            let sender = sender.clone();
            let (next, stop) = (&next, &stop);
            scope.spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let at = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&n) = todo.get(at) else { break };
                    if sender.send((at, census_of(n).map_err(|e| format!("n={n}: {e}")))).is_err() {
                        break;
                    }
                }
            });
        }
        drop(sender);

        // The lines come back in any order; write the ones that make a prefix of the work.
        let mut waiting: BTreeMap<usize, Json> = BTreeMap::new();
        let mut written = 0;
        for (at, line) in receiver {
            waiting.insert(at, line.inspect_err(|_| stop.store(true, Ordering::Relaxed))?);
            while let Some(line) = waiting.remove(&written) {
                sink.write_line(&line.to_string()).map_err(|e| format!("writing a line: {e}"))?;
                written += 1;
                if written % 100 == 0 || written == todo.len() {
                    eprintln!(
                        "{written} of {} boards, last n={}, {:.0} s",
                        todo.len(),
                        todo[written - 1],
                        start.elapsed().as_secs_f64()
                    );
                }
                if keep {
                    kept.push(line);
                }
            }
        }
        Ok(())
    })?;
    Ok(kept)
}

/// Runs the census of `args` and returns whether every board passes.
fn run(args: &Args) -> Result<bool, String> {
    let to_stdout = args.out.is_none();
    let done: BTreeSet<u32> = match &args.out {
        Some(path) => read_lines(path, args.from, args.to, true)?.into_keys().collect(),
        None => BTreeSet::new(),
    };
    let todo: Vec<u32> = (args.from..=args.to).filter(|n| !done.contains(n)).collect();
    if !done.is_empty() {
        eprintln!("{} boards of {}..={} are in the file already; {} to do", done.len(), args.from, args.to, todo.len());
    }
    let mut sink = match &args.out {
        Some(path) => {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            Sink::File(file)
        }
        None => Sink::Stdout,
    };
    let in_memory = analyse(&todo, args.threads, &mut sink, to_stdout)?;

    let lines: Vec<Json> = match &args.out {
        Some(path) => read_lines(path, args.from, args.to, false)?.into_values().collect(),
        None => in_memory,
    };
    let (text, passes) = summary(&lines, args.from, args.to);
    if to_stdout {
        eprintln!("{text}");
    } else {
        println!("{text}");
    }
    Ok(passes)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args(&args) {
        Ok(args) => args,
        Err(usage) => {
            eprintln!("{usage}");
            return ExitCode::from(2);
        }
    };
    match run(&args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
