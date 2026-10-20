//! `wall images`: whether a node's conflict stays a conflict under the exchange its bound makes when the node goes on
//! without a member, and what learning those images saves.  Units of the tuning suite are solved as `wall anatomy`
//! solves them (one thread, fresh tables, from the known optimum), with a hook on every step (`search::StepHook`); with
//! `--root` the unit is the whole game, as `tune solve` solves `GAME-root`.  The notebook entry
//! solver/notebook/conflicts-closed-under-cheap-exchanges-2026-10/README.md reads the lines and the files; this comment
//! says what the code does.
//!
//! ```text
//! cargo run --profile test --features studies --example wall -- images --known FILE (--suite FILE --ids ID[,ID...]
//!     | --root) [--learn [--closure] | --bare | --count] [--trace DIR] [--nodes K] [--tactic NAME=VALUE]... N
//! ```
//!
//! **The test** (the default).  A node branches on a member `x` of its conflict `K` and then goes on without `x`; at
//! its next step its bound's heaviest set has *lost* some numbers (`x` among them) and *gained* others.  The
//! *substitution* is ([`Substitution`]):
//!
//! * a *twin swap* when it lost `x` alone and gained `y` alone, `x = m·p` and `y = m·q` with `p` and `q` prime;
//! * a *paired exchange* when it lost `x` and `b` and gained `c` and `d`, `x = m₁·p`, `c = m₁·q`, `b = m₂·q` and
//!   `d = m₂·p`;
//! * *other* else.
//!
//! For a twin swap or a paired exchange, the map `σ` takes each number lost to its twin gained (`x` to `y`; `x` to `c`
//! and `b` to `d`), and the *image* of `K` is `σ(K)`: `K` with each member that `σ` moves replaced by its twin.  The
//! image lies in the new heaviest set.  The test asks whether the image is a conflict of the node: whether it cannot be
//! promoted from the node's state (`FrameGraph::conflict_using`).
//!
//! **The learning** (`--learn`).  Whenever a bound check computes and learns a conflict `K`, the hook proposes `K`'s
//! images under the cheap exchanges available at the node ([`cheap_images`]), and the searcher learns a conflict within
//! each that a peel finds, as it learned `K` (`search::StepHook::propose`).  The searcher learns only what the peel
//! shows cannot be promoted, so the search stays exact.  With `--closure` the hook proposes the images of the images
//! that are conflicts too, and so on, up to [`CLOSURE`] sets a conflict.  `--bare` solves the units with no hook at
//! all, for the time of the search alone.  `--count` solves them with the learning's hook that proposes nothing: the
//! search of `--bare`, with `steps`, `tree_nodes` and `leaves` counted, which `--bare` cannot.
//!
//! **The start.**  A unit is solved from the known optimum of its game (`--known` has it), which makes the solve a
//! proof.  For a game with no known optimum, the units of a `--suite` are solved from the incumbent of the checkpoint
//! they were cut from, which the suite's `game` line holds, as `tune solve` solves the units of an unfinished game: a
//! subtree may then hold a better set, and `opt` is null.  `--root` needs the known optimum.
//!
//! **The line**, one a unit: `id`, `mode` (`check`, `learn`, `closure`, `count` or `bare`), `tactics`, `nodes` (the
//! engine's steps), `completed`, `best` (the sum of the set the search ended with), `opt` (the known optimum, or null),
//! `incumbent` (the sum of the set the search started from; `opt` when that is known), `cpu_s`; but for `bare`,
//! `steps` (the hook's: the same as `nodes`), `tree_nodes` and `leaves` (nodes with no child explored).  The test adds
//! `exclusions` (steps that went on without a member), `priced` (those whose node checked its bound again at its next
//! step), `twin`, `paired` and `other` (the priced ones by their substitution), and `twin_images` and `paired_images`
//! (of those, the ones whose image is a conflict).  The learning adds `conflicts` (the conflicts computed, each asked
//! for its images), `proposed` (the images proposed) and `learned` (the conflicts learned within them).
//!
//! **The rows** of the test (`--trace DIR`): `DIR/ID.images.tsv` has a line an exclusion, under a header, in the order
//! in which the nodes' loops ended: `start` and `end`, the steps from the node's next step to the end of its loop,
//! which are the steps of the search *under the exclusion* (the hook counts the steps from 1); `d`, the node's depth
//! below the unit's node; `x`, the member, and `home`, its home frame; `price`, the bound before less the bound at the
//! next step (`-` if the next step ended the loop before a bound check); `kind`, `T`, `P`, `O` or `-` (unpriced); for
//! `T` and `P`, `p` and `q`, the primes of `x` and of its twin, `m`, the cofactor of `x`, and `b` and `m2`, the other
//! number lost and its cofactor (0 for `T`); `nk`, the size of `K`, and `moved`, the members of `K` that `σ` moves;
//! `img`, 1 if the image is a conflict, 0 if not, `-` for no image; `imin`, the size of a minimal conflict within the
//! image (`-1` if none); `next`, how the conflict of the node's next step stands to the image (`same`, `within` it,
//! `other`, or `none` if the step found none), and `nc`, 1 if that conflict was computed rather than known; and `lost`
//! and `gained`, comma separated.
//!
//! **The classes** of the learning (`--trace DIR`): `DIR/ID.classes.tsv` has a line a *class*, a conflict the search
//! computed (keyed by its nogood) with the conflicts learned within its images: `key` (the nogood, comma separated),
//! `size` (the conflict's size where it was first computed), `times` (how often the search computed it), `proposed` and
//! `learned` (over all those times), `distinct` (the distinct nogoods learned), `branched` (the steps that branched on
//! a conflict that one of those nogoods gives the node) and `ruled` (the candidates one of them left alone outside a
//! child's promotions, that a child made after promoting one of its members: an estimate of the candidates they ruled
//! out, which counts those that the node's other conflicts or narrowing rule out too).  The hook forgets the nogoods
//! when the searcher's table does (tactic `forget_nogoods`).

use super::anatomy::{suite_start, suite_units};
use super::cycles::gcd;
use super::{beyond, Line, Options};
use crate::counters::cpu_seconds;
use crate::known::known_incumbent;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::{self, Control, EngineConfig, Start};
use taxman::game::Optimal;
use taxman::matchbound::MatchGraph;
use taxman::model::{FrameGraph, FrameSet, Scratch};
use taxman::numtheory;
use taxman::search::{
    payload_of, set_of_payload, MakeHook, Prepared, SearchSpace, StepEnd, StepHook, StepRecord, TaxSearcher, TaxUnit,
};
use taxman::tactics::Tactics;

/// The options of `images`.
#[derive(Clone, Copy, Default)]
pub struct Params {
    /// Learn the images of the conflicts (`--learn`), solve with no hook (`--bare`), or only count the tree
    /// (`--count`); else the test.
    pub learn: bool,
    pub bare: bool,
    pub count: bool,
    /// With `--learn`, propose the images of the images too, and so on (`--closure`).
    pub closure: bool,
}

/// An exchange priced under this many points is *cheap* (the anatomy's line).
const CHEAP: i64 = 50;

/// With `--closure`, the most sets proposed for one conflict.
const CLOSURE: usize = 64;

// ---------------------------------------------------------------------------------------------------------
// The substitutions

/// How a node's heaviest set changed when the node went on without a member.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Twin,
    Paired,
    Other,
    /// The node's next step ended its loop before a bound check.
    Unpriced,
}

impl Kind {
    fn letter(self) -> char {
        match self {
            Kind::Twin => 'T',
            Kind::Paired => 'P',
            Kind::Other => 'O',
            Kind::Unpriced => '-',
        }
    }
}

/// A substitution (see the module docs): its kind and, for a twin swap or a paired exchange, the map `σ` from the
/// numbers lost to their twins gained (the member's first), with the primes of the member and of its twin.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Substitution {
    kind: Kind,
    map: Vec<(u32, u32)>,
    primes: (u32, u32),
}

/// Whether `v` is prime, by the table of smallest prime factors `spf`.
fn is_prime(v: u32, spf: &[u32]) -> bool {
    v >= 2 && spf.get(v as usize) == Some(&v)
}

/// The two primes when `a = m·p` and `b = m·q` with `p ≠ q` prime: `(p, q)`.
fn twins(a: u32, b: u32, spf: &[u32]) -> Option<(u32, u32)> {
    let g = gcd(a, b);
    let (p, q) = (a / g, b / g);
    (p != q && is_prime(p, spf) && is_prime(q, spf)).then_some((p, q))
}

impl Substitution {
    /// The substitution of the member `x` that took `lost` (with `x`) out of the heaviest set and put `gained` in.
    fn of(x: u32, lost: &[u32], gained: &[u32], spf: &[u32]) -> Substitution {
        let other = Substitution { kind: Kind::Other, map: Vec::new(), primes: (0, 0) };
        match (lost, gained) {
            (&[a], &[y]) if a == x => match twins(x, y, spf) {
                Some(primes) => Substitution { kind: Kind::Twin, map: vec![(x, y)], primes },
                None => other,
            },
            (&[a0, a1], &[g0, g1]) if a0 == x || a1 == x => {
                let b = if a0 == x { a1 } else { a0 };
                for (c, d) in [(g0, g1), (g1, g0)] {
                    if let Some((p, q)) = twins(x, c, spf) {
                        if twins(b, d, spf) == Some((q, p)) {
                            return Substitution { kind: Kind::Paired, map: vec![(x, c), (b, d)], primes: (p, q) };
                        }
                    }
                }
                other
            }
            _ => other,
        }
    }

    /// `σ(k)`: `k` with each member that the map moves replaced by its twin.
    fn image<const W: usize>(&self, k: &Bits<W>) -> Bits<W> {
        let mut out = *k;
        for &(a, b) in self.map.iter().filter(|(a, _)| k.contains(*a)) {
            out.remove(a);
            out.insert(b);
        }
        out
    }
}

/// The images of the conflict `k` under the cheap exchanges available at a node with the candidates `cands` and the
/// heaviest set `chosen`, added to `out` (each once).  An image replaces a member `a = m·p` of `k` by a candidate
/// `a' = m·q` outside `k` (`q` prime) when the twin swap is cheap, `0 < a − a' < CHEAP`, or when a paired exchange
/// makes it cheap: a member `b = m₂·q` of `chosen` whose twin `d = m₂·p` is a candidate outside `chosen`, with the
/// exchange's price `(a − a') + (b − d)` at least 0 and under `CHEAP`.  The image of such a paired exchange replaces
/// `b` by `d` too when `b` is in `k`.
fn cheap_images<const W: usize>(spf: &[u32], k: &Bits<W>, cands: &Bits<W>, chosen: &Bits<W>, out: &mut Vec<Bits<W>>) {
    let top = cands.max().unwrap_or(0);
    let mut push = |image: Bits<W>| {
        if !out.contains(&image) {
            out.push(image);
        }
    };
    for a in k.iter() {
        let mut rest = a;
        while rest > 1 {
            let p = spf[rest as usize];
            while rest.is_multiple_of(p) {
                rest /= p;
            }
            let m = a / p;
            // A paired exchange's price is (m − m₂)·(p − q), so a cheap one has |p − q| < CHEAP.
            let low = p.saturating_sub(CHEAP as u32 - 1).max(2);
            for q in (low..p + CHEAP as u32).filter(|&q| q != p && is_prime(q, spf)) {
                let twin = m * q;
                if twin > top || !cands.contains(twin) || k.contains(twin) {
                    continue;
                }
                let swap = i64::from(a) - i64::from(twin);
                if 0 < swap && swap < CHEAP {
                    push(k.without(a).with(twin));
                }
                for b in chosen.iter().filter(|&b| b != a && b != twin && b.is_multiple_of(q)) {
                    let d = b / q * p;
                    if d > top || d == a || !cands.contains(d) || chosen.contains(d) {
                        continue;
                    }
                    let price = swap + i64::from(b) - i64::from(d);
                    if (0..CHEAP).contains(&price) {
                        let image = k.without(a).with(twin);
                        push(if k.contains(b) { image.without(b).with(d) } else { image });
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// What the hooks share

/// What every searcher's hook of a game shares: the model, the root, the smallest prime factor of each number, and the
/// period after which a searcher forgets its nogoods.
pub(super) struct Game<const W: usize> {
    model: Arc<FrameGraph>,
    root: FrameSet<W>,
    spf: Vec<u32>,
    forget_period: Option<u64>,
    /// Also check each image's verdict against promoting it (the test does).
    verify: bool,
    /// Gather the classes of the learning (`--trace`).
    classes: bool,
    /// Propose the images of the images too (`--closure`).
    pub(super) closure: bool,
    /// The learning's hook proposes images; if not, it only counts the tree (`--count`).
    propose: bool,
}

impl<const W: usize> Game<W> {
    pub(super) fn new(p: &Prepared<W>, verify: bool, classes: bool) -> Game<W> {
        let top = p.root.frames().iter().filter_map(|f| (f.moves | f.factors).max()).max().unwrap_or(1);
        let (spf, _) = numtheory::sieve(top);
        let forget_period = p.tactics.forgetting().period;
        let model = Arc::clone(&p.model);
        Game { model, root: p.root, spf, forget_period, verify, classes, closure: false, propose: true }
    }
}

/// The shape of a search's tree, told step by step: the depth of the step's node below the unit's, the nodes, and the
/// leaves (nodes that made no child that was explored).
struct Shape {
    depth: usize,
    parent: Vec<bool>,
    nodes: u64,
    leaves: u64,
}

impl Default for Shape {
    fn default() -> Self {
        Shape { depth: 0, parent: vec![false], nodes: 1, leaves: 0 }
    }
}

impl Shape {
    fn step(&mut self, end: StepEnd) {
        let d = self.depth;
        if self.parent.len() < d + 2 {
            self.parent.resize(d + 2, false);
        }
        match end {
            StepEnd::Child => {
                self.nodes += 1;
                self.parent[d] = true;
                self.parent[d + 1] = false;
                self.depth = d + 1;
            }
            StepEnd::LoopEnd(_) => {
                self.leaves += u64::from(!self.parent[d]);
                self.parent[d] = false;
                self.depth = d.saturating_sub(1);
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// The test

/// How the conflict of a node's next step stands to the image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Next {
    Same,
    Within,
    Other,
    None,
}

/// One exclusion, priced or not (see the module docs, "The rows").
struct Row {
    start: u64,
    end: u64,
    depth: usize,
    member: u32,
    home: usize,
    price: Option<i64>,
    sub: Substitution,
    conflict_len: u32,
    moved: u32,
    image: Option<bool>,
    minimal: i64,
    next: Next,
    computed: bool,
    lost: Vec<u32>,
    gained: Vec<u32>,
}

impl Row {
    fn tsv(&self) -> String {
        let join = |xs: &[u32]| xs.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
        let (p, q) = self.sub.primes;
        let cofactor = |i: usize| self.sub.map.get(i).map_or((0, 0), |&(a, _)| (a, if i == 0 { a / p } else { a / q }));
        let ((_, m), (b, m2)) = (cofactor(0), cofactor(1));
        let price = self.price.map_or("-".to_owned(), |v| v.to_string());
        let image = self.image.map_or("-", |c| if c { "1" } else { "0" });
        let next = match self.next {
            Next::Same => "same",
            Next::Within => "within",
            Next::Other => "other",
            Next::None => "none",
        };
        let kind = self.sub.kind.letter();
        let (start, end, d, x, home, nk, moved, imin) =
            (self.start, self.end, self.depth, self.member, self.home, self.conflict_len, self.moved, self.minimal);
        let (lost, gained, nc) = (join(&self.lost), join(&self.gained), u8::from(self.computed));
        format!(
            "{start}\t{end}\t{d}\t{x}\t{home}\t{price}\t{kind}\t{p}\t{q}\t{m}\t{b}\t{m2}\t{nk}\t{moved}\t{image}\t\
             {imin}\t{next}\t{nc}\t{lost}\t{gained}"
        )
    }
}

const HEADER: &str =
    "start\tend\td\tx\thome\tprice\tkind\tp\tq\tm\tb\tm2\tnk\tmoved\timg\timin\tnext\tnc\tlost\tgained";

/// What the test's hooks of a unit's search gathered.
#[derive(Default)]
struct Tally {
    steps: u64,
    nodes: u64,
    leaves: u64,
    exclusions: u64,
    rows: Vec<Row>,
}

/// The exclusion a node made at its last step, for its next step to price.
struct Excluded<const W: usize> {
    member: u32,
    conflict: Bits<W>,
}

/// The node at one depth of the stack.
struct Level<const W: usize> {
    /// Its last bound check: the value and the heaviest set.
    check: Option<(u64, Bits<W>)>,
    pending: Option<Excluded<W>>,
    /// Its exclusions priced, waiting for its loop to end.
    rows: Vec<Row>,
}

impl<const W: usize> Default for Level<W> {
    fn default() -> Self {
        Level { check: None, pending: None, rows: Vec::new() }
    }
}

/// The test's hook of one searcher.  What it gathers goes to `shared` when the searcher is dropped.
struct Tester<const W: usize> {
    game: Arc<Game<W>>,
    shared: Arc<Mutex<Tally>>,
    tally: Tally,
    shape: Shape,
    levels: Vec<Level<W>>,
    /// The exclusion of the step in progress, for the level once the step is told.
    fresh: Option<Excluded<W>>,
    scratch: Scratch<W>,
}

impl<const W: usize> Tester<W> {
    fn new(game: Arc<Game<W>>, shared: Arc<Mutex<Tally>>) -> Self {
        let (tally, shape, levels, scratch) =
            (Tally::default(), Shape::default(), vec![Level::default()], Scratch::new());
        Tester { game, shared, tally, shape, levels, fresh: None, scratch }
    }

    /// The row of the exclusion `ex`, at the node's next step `r` in `state`: `before` the node's last check, if any.
    fn price(
        &mut self,
        ex: Excluded<W>,
        before: Option<(u64, Bits<W>)>,
        r: &StepRecord<W>,
        state: &FrameSet<W>,
    ) -> Row {
        let game = Arc::clone(&self.game);
        let model = &game.model;
        let (into, out) = model.frames_touched(ex.member);
        let mut row = Row {
            start: self.tally.steps,
            end: self.tally.steps,
            depth: self.shape.depth,
            member: ex.member,
            home: out.unwrap_or(into),
            price: None,
            sub: Substitution { kind: Kind::Unpriced, map: Vec::new(), primes: (0, 0) },
            conflict_len: ex.conflict.len(),
            moved: 0,
            image: None,
            minimal: -1,
            next: Next::None,
            computed: false,
            lost: Vec::new(),
            gained: Vec::new(),
        };
        let (Some(before), Some(now)) = (before, r.node) else { return row };
        row.price = Some(before.0 as i64 - now.value as i64);
        row.lost = (before.1 - now.chosen).to_vec();
        row.gained = (now.chosen - before.1).to_vec();
        row.sub = Substitution::of(ex.member, &row.lost, &row.gained, &game.spf);
        if matches!(row.sub.kind, Kind::Other) {
            return row;
        }
        let image = row.sub.image(&ex.conflict);
        row.moved = row.sub.map.iter().filter(|(a, _)| ex.conflict.contains(*a)).count() as u32;
        let conflict = model.conflict_using(&mut self.scratch, state, &image, false).is_some();
        if game.verify {
            let after = model.promote_all_using(&mut self.scratch, state, &image);
            let promotable = after.is_some_and(|s| s.frames().iter().all(|f| f.freedoms >= 0));
            assert_eq!(conflict, !promotable, "the image {image:?} of {:?} at {}", ex.conflict, ex.member);
        }
        row.image = Some(conflict);
        if conflict {
            let minimal = model.conflict_using(&mut self.scratch, state, &image, true).expect("a conflict");
            row.minimal = i64::from(minimal.len());
        }
        (row.next, row.computed) = match now.conflict {
            Some(k) if k == image => (Next::Same, now.computed),
            Some(k) if k.is_subset(&image) => (Next::Within, now.computed),
            Some(_) => (Next::Other, now.computed),
            None => (Next::None, false),
        };
        row
    }
}

impl<const W: usize> Drop for Tester<W> {
    fn drop(&mut self) {
        // A search stopped by its step limit leaves loops open: their rows end at its last step.
        let steps = self.tally.steps;
        for level in &mut self.levels {
            for mut row in level.rows.drain(..) {
                row.end = steps;
                self.tally.rows.push(row);
            }
        }
        let tally = std::mem::take(&mut self.tally);
        let mut shared = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
        shared.steps += tally.steps;
        shared.nodes += self.shape.nodes;
        shared.leaves += self.shape.leaves;
        shared.exclusions += tally.exclusions;
        shared.rows.extend(tally.rows);
    }
}

impl<const W: usize> StepHook<W> for Tester<W> {
    fn step(&mut self, r: &StepRecord<W>, state: &FrameSet<W>, _child: Option<&FrameSet<W>>) {
        self.tally.steps += 1;
        let d = self.shape.depth;
        if self.levels.len() < d + 2 {
            self.levels.resize_with(d + 2, Level::default);
        }
        if let Some(ex) = self.levels[d].pending.take() {
            let row = self.price(ex, self.levels[d].check, r, state);
            self.levels[d].rows.push(row);
        }
        if let Some(c) = r.node {
            self.levels[d].check = Some((c.value, c.chosen));
        }
        self.levels[d].pending = self.fresh.take();
        match r.end {
            StepEnd::Child => {
                self.levels[d + 1] = Level { check: r.child.map(|c| (c.value, c.chosen)), ..Level::default() };
            }
            StepEnd::LoopEnd(_) => {
                let steps = self.tally.steps;
                for mut row in self.levels[d].rows.drain(..) {
                    row.end = steps;
                    self.tally.rows.push(row);
                }
                self.levels[d] = Level::default();
            }
            _ => {}
        }
        self.shape.step(r.end);
    }

    fn excluding(&mut self, _: &FrameSet<W>, _: &Bits<W>, _: &Bits<W>, conflict: Option<&Bits<W>>, member: u32) {
        self.tally.exclusions += 1;
        let conflict = conflict.copied().unwrap_or(Bits::EMPTY.with(member));
        self.fresh = Some(Excluded { member, conflict });
    }
}

// ---------------------------------------------------------------------------------------------------------
// The learning

/// A class (see the module docs, "The classes").
#[derive(Default)]
struct Class {
    key: Vec<u32>,
    size: u32,
    times: u64,
    proposed: u64,
    learned: u64,
    distinct: HashSet<Vec<u32>>,
    branched: u64,
    ruled: u64,
}

/// What the learning's hooks of a unit's search gathered.
#[derive(Default)]
pub(super) struct Learned {
    steps: u64,
    nodes: u64,
    leaves: u64,
    pub(super) conflicts: u64,
    pub(super) proposed: u64,
    pub(super) learned: u64,
    classes: Vec<Class>,
}

/// With `--trace`, what a learning hook tracks of the classes: the class of each key, the class of the conflict last
/// proposed for, the nogoods learned within images that the searcher's table holds (with their classes) and, for each
/// number, those that it is a member of; and the conflict of each depth's last check, for a first step that keeps its
/// node's verdict.
#[derive(Default)]
struct Track<const W: usize> {
    keys: HashMap<Vec<u32>, usize>,
    current: Option<usize>,
    alive: Vec<(Bits<W>, usize)>,
    by_member: HashMap<u32, Vec<usize>>,
    conflicts: Vec<Option<Bits<W>>>,
}

impl<const W: usize> Track<W> {
    /// The class of the first nogood held that gives a node with the promotions `promoted` the conflict `k`.
    fn giving(&self, promoted: &Bits<W>, k: &Bits<W>) -> Option<usize> {
        let ids = self.by_member.get(&k.max()?)?;
        ids.iter()
            .map(|&id| self.alive[id])
            .find(|(n, _)| k.is_subset(n) && (*n - *k).is_subset(promoted))
            .map(|(_, c)| c)
    }
}

/// The learning's hook of one searcher.  What it gathers goes to `shared` when the searcher is dropped.
pub(super) struct Learner<const W: usize> {
    game: Arc<Game<W>>,
    shared: Arc<Mutex<Learned>>,
    tally: Learned,
    shape: Shape,
    scratch: Scratch<W>,
    track: Option<Track<W>>,
}

impl<const W: usize> Learner<W> {
    pub(super) fn new(game: Arc<Game<W>>, shared: Arc<Mutex<Learned>>) -> Self {
        let track = game.classes.then(Track::default);
        Learner { game, shared, tally: Learned::default(), shape: Shape::default(), scratch: Scratch::new(), track }
    }

    /// With `--trace`: what the step `r` did with the nogoods learned within images (see the module docs, "The
    /// classes").
    fn track_step(&mut self, r: &StepRecord<W>) {
        let Some(track) = &mut self.track else { return };
        let classes = &mut self.tally.classes;
        let d = self.shape.depth;
        if track.conflicts.len() < d + 2 {
            track.conflicts.resize(d + 2, None);
        }
        if let Some(c) = r.node {
            track.conflicts[d] = c.conflict;
        }
        if let (Some(_), Some(k)) = (r.branch, track.conflicts[d]) {
            if let Some(class) = track.giving(&r.promoted, &k) {
                classes[class].branched += 1;
            }
        }
        if matches!(r.end, StepEnd::Child | StepEnd::Leaf | StepEnd::ChildCut(_)) {
            let promoted = r.promoted | r.added;
            let mut counted: Vec<(u32, usize)> = Vec::new();
            for a in r.added.iter() {
                for &id in track.by_member.get(&a).into_iter().flatten() {
                    let (n, class) = track.alive[id];
                    let rest = n - promoted;
                    let Some(z) = rest.max().filter(|_| rest.len() == 1) else { continue };
                    if r.cands.contains(z) && !counted.contains(&(z, class)) {
                        counted.push((z, class));
                        classes[class].ruled += 1;
                    }
                }
            }
        }
        if r.end == StepEnd::Child {
            track.conflicts[d + 1] = r.child.and_then(|c| c.conflict);
        }
        // The searcher's table forgets its nogoods after each period of steps (`forget_nogoods`).
        if self.game.forget_period.is_some_and(|period| self.tally.steps.is_multiple_of(period)) {
            track.alive.clear();
            track.by_member.clear();
        }
    }
}

impl<const W: usize> Drop for Learner<W> {
    fn drop(&mut self) {
        let mut tally = std::mem::take(&mut self.tally);
        (tally.nodes, tally.leaves) = (self.shape.nodes, self.shape.leaves);
        let mut shared = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
        shared.steps += tally.steps;
        shared.nodes += tally.nodes;
        shared.leaves += tally.leaves;
        shared.conflicts += tally.conflicts;
        shared.proposed += tally.proposed;
        shared.learned += tally.learned;
        shared.classes.extend(tally.classes);
    }
}

impl<const W: usize> StepHook<W> for Learner<W> {
    fn step(&mut self, r: &StepRecord<W>, _: &FrameSet<W>, _: Option<&FrameSet<W>>) {
        self.tally.steps += 1;
        self.track_step(r);
        self.shape.step(r.end);
    }

    fn propose(
        &mut self,
        state: &FrameSet<W>,
        promoted: &Bits<W>,
        cands: &Bits<W>,
        chosen: &Bits<W>,
        conflict: &Bits<W>,
        more: &mut Vec<Bits<W>>,
    ) {
        let game = &self.game;
        if !game.propose {
            return;
        }
        cheap_images(&game.spf, conflict, cands, chosen, more);
        if game.closure {
            // The images of the images that are conflicts, and so on: the conflict's class closed under the cheap
            // exchanges, up to CLOSURE sets.
            let mut i = 0;
            while i < more.len() && more.len() < CLOSURE {
                let image = more[i];
                i += 1;
                if game.model.conflict_using(&mut self.scratch, state, &image, false).is_some() {
                    cheap_images(&game.spf, &image, cands, chosen, more);
                }
            }
            more.retain(|image| image != conflict);
        }
        self.tally.conflicts += 1;
        self.tally.proposed += more.len() as u64;
        let Some(track) = &mut self.track else { return };
        let key = game.model.nogood(&mut self.scratch, &game.root, promoted, conflict).unwrap_or(*conflict).to_vec();
        let classes = &mut self.tally.classes;
        let class = *track.keys.entry(key.clone()).or_insert_with(|| {
            classes.push(Class { key, size: conflict.len(), ..Class::default() });
            classes.len() - 1
        });
        classes[class].times += 1;
        classes[class].proposed += more.len() as u64;
        track.current = Some(class);
    }

    fn learned(&mut self, _: &Bits<W>, _: &Bits<W>, nogood: Option<&Bits<W>>) {
        self.tally.learned += 1;
        let (Some(track), Some(&n)) = (&mut self.track, nogood) else { return };
        let class = track.current.expect("a class proposed for");
        let c = &mut self.tally.classes[class];
        c.learned += 1;
        c.distinct.insert(n.to_vec());
        let id = track.alive.len();
        track.alive.push((n, class));
        for x in n.iter() {
            track.by_member.entry(x).or_default().push(id);
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// The command

/// What solving a unit gave.
struct Solved {
    nodes: u64,
    completed: bool,
    best: u64,
    cpu_s: f64,
}

/// Solves `unit` on one thread from the incumbent `start` (its value, and its set or none: a floor), with fresh tables
/// and the hooks `hooks` makes, if any.
fn solve<const W: usize>(
    p: &Prepared<W>,
    graph: &MatchGraph,
    hooks: Option<&MakeHook<W>>,
    unit: TaxUnit<W>,
    (best, start): (u64, &Bits<W>),
    node_limit: Option<u64>,
) -> Solved {
    let space = SearchSpace { hooks, ..SearchSpace::new(p, graph) };
    let cfg = EngineConfig { threads: 1, node_limit, control: Some(Control::new()), ..EngineConfig::default() };
    let begin = Start::Fresh { units: vec![unit], best, payload: payload_of(start) };
    let before = cpu_seconds();
    let out = engine::run(&cfg, begin, b"images", |_| TaxSearcher::new(&space)).expect("the engine");
    let cpu_s = before.zip(cpu_seconds()).map_or(0.0, |(a, b)| b - a);
    let best = set_of_payload::<W>(&out.payload).sum();
    Solved { nodes: out.stats.nodes, completed: out.completed, best, cpu_s }
}

/// The test on `unit`: its search, and what the hooks gathered.
fn check<const W: usize>(
    p: &Prepared<W>,
    graph: &MatchGraph,
    game: &Arc<Game<W>>,
    unit: TaxUnit<W>,
    start: (u64, &Bits<W>),
    node_limit: Option<u64>,
) -> (Solved, Tally) {
    let shared = Arc::new(Mutex::new(Tally::default()));
    let make = {
        let (game, shared) = (Arc::clone(game), Arc::clone(&shared));
        move || Box::new(Tester::<W>::new(Arc::clone(&game), Arc::clone(&shared))) as Box<dyn StepHook<W>>
    };
    let solved = solve(p, graph, Some(&make), unit, start, node_limit);
    let tally = std::mem::take(&mut *shared.lock().unwrap_or_else(PoisonError::into_inner));
    (solved, tally)
}

/// The learning on `unit`: its search, and what the hooks gathered.
fn learn<const W: usize>(
    p: &Prepared<W>,
    graph: &MatchGraph,
    game: &Arc<Game<W>>,
    unit: TaxUnit<W>,
    start: (u64, &Bits<W>),
    node_limit: Option<u64>,
) -> (Solved, Learned) {
    let shared = Arc::new(Mutex::new(Learned::default()));
    let make = {
        let (game, shared) = (Arc::clone(game), Arc::clone(&shared));
        move || Box::new(Learner::<W>::new(Arc::clone(&game), Arc::clone(&shared))) as Box<dyn StepHook<W>>
    };
    let solved = solve(p, graph, Some(&make), unit, start, node_limit);
    let learned = std::mem::take(&mut *shared.lock().unwrap_or_else(PoisonError::into_inner));
    (solved, learned)
}

/// Writes `header` and then `lines` to `path`.
fn write_lines(path: &Path, header: &str, lines: impl IntoIterator<Item = String>) -> Result<(), String> {
    let fail = |e: std::io::Error| format!("{}: {e}", path.display());
    let mut out = BufWriter::new(File::create(path).map_err(fail)?);
    writeln!(out, "{header}").map_err(fail)?;
    for line in lines {
        writeln!(out, "{line}").map_err(fail)?;
    }
    out.flush().map_err(fail)
}

/// What a game's units are solved from: the sum of the incumbent, and the known optimum if there is one (then the two
/// are equal and the search proves; else the search may find better).
#[derive(Clone, Copy)]
struct Origin {
    incumbent: u64,
    opt: Option<u64>,
}

/// The fields every line has.
fn line(n: u32, id: &str, mode: &str, o: &Options, start: Origin, s: &Solved) -> Line {
    let mut out = Line::of(n);
    out.text("id", id)
        .text("mode", mode)
        .text("tactics", &Tactics { search: o.tactics, ..Tactics::default() }.description())
        .field("nodes", s.nodes)
        .field("completed", s.completed)
        .field("best", s.best)
        .maybe("opt", start.opt)
        .field("incumbent", start.incumbent)
        .seconds("cpu_s", s.cpu_s);
    out
}

/// The units of game `n` to solve: the whole game with `--root`, as the suite names it (`GAME-root`); else the suite's
/// units `--ids` names.
pub(super) fn units_of<const W: usize>(
    n: u32,
    o: &Options,
    p: &Prepared<W>,
) -> Result<Vec<(String, TaxUnit<W>)>, String> {
    if o.exchanges.root {
        return Ok(vec![(format!("{n}-root"), TaxUnit { promoted: Bits::EMPTY, cands: p.candidates })]);
    }
    let suite = o.params.suite.as_ref().ok_or("images needs --suite or --root")?;
    suite_units::<W>(suite, n, &o.anatomy.ids)
}

/// The set to solve the units of game `n` from, and what is known of it: the game's known optimum (the units are
/// proved), or else, for the units of a suite, the incumbent of the checkpoint they were cut from, as `tune solve`
/// solves the units of an unfinished game (a subtree may then hold a better set than the incumbent).
fn start_of<const W: usize>(
    n: u32,
    o: &Options,
    board: &Board<W>,
    p: &Prepared<W>,
    optimal: &Optimal,
) -> Result<(Bits<W>, Origin), String> {
    if let Some((set, opt)) = known_incumbent(board, p, optimal)? {
        return Ok((set, Origin { incumbent: opt, opt: Some(opt) }));
    }
    let from_units = match (&o.params.suite, o.exchanges.root) {
        (Some(suite), false) => suite_start(suite, n, p)?,
        _ => None,
    };
    let set = from_units.ok_or_else(|| {
        format!("n={n}: needs its known optimum (--known), or the units of a --suite whose game line has an incumbent")
    })?;
    Ok((set, Origin { incumbent: set.sum(), opt: None }))
}

/// `images` on game `n` at the set width `W`.
fn play<const W: usize>(n: u32, o: &Options, optimal: &Optimal) -> Result<(), String> {
    let board = Board::<W>::for_n(n);
    if board.is_trivial() {
        return Ok(());
    }
    let p = board.prepare(o.tactics)?;
    let (set, known) = start_of(n, o, &board, &p, optimal)?;
    let units = units_of(n, o, &p)?;
    let graph = MatchGraph::new(&p.model, &p.root);
    let start = (known.incumbent, &set);
    let trace = o.anatomy.trace.as_ref();
    let game =
        Arc::new(Game { closure: o.images.closure, propose: !o.images.count, ..Game::new(&p, false, trace.is_some()) });
    for (id, unit) in units {
        if o.images.bare {
            let solved = solve(&p, &graph, None, unit, start, o.nodes);
            line(n, &id, "bare", o, known, &solved).print();
        } else if o.images.count {
            let (solved, t) = learn(&p, &graph, &game, unit, start, o.nodes);
            line(n, &id, "count", o, known, &solved)
                .field("steps", t.steps)
                .field("tree_nodes", t.nodes)
                .field("leaves", t.leaves)
                .print();
        } else if o.images.learn {
            let (solved, t) = learn(&p, &graph, &game, unit, start, o.nodes);
            if let Some(dir) = trace {
                let header = "key\tsize\ttimes\tproposed\tlearned\tdistinct\tbranched\truled";
                let lines = t.classes.iter().map(|c| {
                    let key = c.key.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
                    let (size, times, proposed, learned) = (c.size, c.times, c.proposed, c.learned);
                    let (distinct, branched, ruled) = (c.distinct.len(), c.branched, c.ruled);
                    format!("{key}\t{size}\t{times}\t{proposed}\t{learned}\t{distinct}\t{branched}\t{ruled}")
                });
                write_lines(&dir.join(format!("{id}.classes.tsv")), header, lines)?;
            }
            let mut out = line(n, &id, if o.images.closure { "closure" } else { "learn" }, o, known, &solved);
            out.field("steps", t.steps)
                .field("tree_nodes", t.nodes)
                .field("leaves", t.leaves)
                .field("conflicts", t.conflicts)
                .field("proposed", t.proposed)
                .field("learned", t.learned)
                .print();
        } else {
            let (solved, t) = check(&p, &graph, &game, unit, start, o.nodes);
            if let Some(dir) = trace {
                write_lines(&dir.join(format!("{id}.images.tsv")), HEADER, t.rows.iter().map(Row::tsv))?;
            }
            let count = |kind: Kind, image: Option<bool>| {
                t.rows.iter().filter(|r| r.sub.kind == kind && (image.is_none() || r.image == image)).count()
            };
            let mut out = line(n, &id, "check", o, known, &solved);
            out.field("steps", t.steps)
                .field("tree_nodes", t.nodes)
                .field("leaves", t.leaves)
                .field("exclusions", t.exclusions)
                .field("priced", t.rows.iter().filter(|r| r.price.is_some()).count())
                .field("twin", count(Kind::Twin, None))
                .field("paired", count(Kind::Paired, None))
                .field("other", count(Kind::Other, None))
                .field("twin_images", count(Kind::Twin, Some(true)))
                .field("paired_images", count(Kind::Paired, Some(true)))
                .print();
        }
    }
    Ok(())
}

/// `images` on each game of `o`.
pub fn run(o: &Options, optimal: &Optimal) -> Result<(), String> {
    if [o.images.learn, o.images.bare, o.images.count].into_iter().filter(|&mode| mode).count() > 1 {
        return Err("images: --learn, --bare and --count do not go together".to_owned());
    }
    if o.images.closure && !o.images.learn {
        return Err("images: --closure needs --learn".to_owned());
    }
    for &n in &o.games {
        taxman::with_width!(n, W => play::<W>(n, o, optimal), else Err(beyond(n)))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use taxman::search::SearchTactics;

    #[test]
    fn the_substitutions_of_the_notebook_are_classified() {
        let (spf, _) = numtheory::sieve(2000);
        // 1292 = 68·19 out for 1156 = 68·17: a twin swap.
        let twin = Substitution::of(1292, &[1292], &[1156], &spf);
        assert_eq!((twin.kind, twin.primes), (Kind::Twin, (19, 17)));
        // 1608 = 24·67 and 1220 = 20·61 out for 1464 = 24·61 and 1340 = 20·67: a paired exchange.
        let paired = Substitution::of(1608, &[1220, 1608], &[1340, 1464], &spf);
        assert_eq!(paired.kind, Kind::Paired);
        assert_eq!((paired.map.clone(), paired.primes), (vec![(1608, 1464), (1220, 1340)], (67, 61)));
        let k: Bits<32> = [1220, 1608, 1700].into_iter().collect();
        assert_eq!(paired.image(&k).to_vec(), vec![1340, 1464, 1700]);
        // 1210 = 10·121 out for 1089 = 9·121: kin, not twins.
        assert_eq!(Substitution::of(1210, &[1210], &[1089], &spf).kind, Kind::Other);
        // Two twin swaps of different primes: no paired exchange.
        assert_eq!(Substitution::of(1608, &[1220, 1608], &[1380, 1464], &spf).kind, Kind::Other);
    }

    #[test]
    fn the_cheap_images_are_the_cheap_twin_swaps_and_paired_exchanges() {
        let (spf, _) = numtheory::sieve(2000);
        let set = |xs: &[u32]| -> Bits<32> { xs.iter().copied().collect() };
        let mut out = Vec::new();
        // 1292 = 68·19 for 1156 = 68·17 costs 136: not cheap alone.  With 1020 = 60·17 in the heaviest set and
        // 1140 = 60·19 a candidate outside it, the paired exchange costs 136 − 120 = 16.
        let (k, cands) = (set(&[1292, 1700]), set(&[1020, 1140, 1156, 1292, 1700]));
        cheap_images(&spf, &k, &cands, &set(&[1020, 1292, 1700]), &mut out);
        assert_eq!(out, vec![set(&[1156, 1700])]);
        out.clear();
        cheap_images(&spf, &k, &cands, &set(&[1292, 1700]), &mut out);
        assert!(out.is_empty(), "{out:?}");
        // 1592 = 8·199 for 1576 = 8·197 costs 16: cheap alone.
        cheap_images(&spf, &set(&[1592]), &set(&[1576, 1592]), &set(&[1592]), &mut out);
        assert_eq!(out, vec![set(&[1576])]);
    }

    /// Game 1215 searched from its known best set, worth P*: with the test's hook the tree is the search's, with a row
    /// for every exclusion and each image's verdict the same as promoting it (`Game::verify`); with the learning the
    /// tree may change, the best may not.
    fn check_1215<const W: usize>() {
        let n = 1215;
        let board = Board::<W>::for_n(n);
        let p = board.prepare(SearchTactics::default()).expect("prepare");
        let (set, opt) = known_incumbent(&board, &p, Optimal::built_in()).expect("known").expect("a known game");
        let graph = MatchGraph::new(&p.model, &p.root);
        let root = TaxUnit { promoted: Bits::EMPTY, cands: p.candidates };
        let plain = solve(&p, &graph, None, root, (opt, &set), None);
        let (solved, t) = check(&p, &graph, &Arc::new(Game::new(&p, true, false)), root, (opt, &set), None);
        assert!(plain.completed && solved.completed);
        assert_eq!((solved.nodes, t.steps, solved.best), (plain.nodes, plain.nodes, plain.best));
        assert_eq!(t.rows.len() as u64, t.exclusions);
        assert!(t.rows.iter().all(|r| r.start <= r.end && r.end <= t.steps));
        assert!(t.rows.iter().any(|r| r.image == Some(true)), "no image is a conflict at {n}");
        let (learned, l) = learn(&p, &graph, &Arc::new(Game::new(&p, false, true)), root, (opt, &set), None);
        assert!(learned.completed && l.learned > 0, "nothing learned at {n}");
        assert_eq!(learned.best, plain.best);
        assert_eq!(l.classes.iter().map(|c| c.learned).sum::<u64>(), l.learned);
    }

    #[test]
    fn the_test_changes_no_tree_and_the_learning_no_best() {
        taxman::with_width!(1215, W => check_1215::<W>(), else panic!("{}", beyond(1215)));
    }

    /// The count of game 1215 from its known best set finds the tree of the search with no hook, the test's tree
    /// (steps, nodes, leaves), and proposes and learns nothing; `--root` names the whole game as the suite does.
    fn count_1215<const W: usize>() {
        let n = 1215;
        let board = Board::<W>::for_n(n);
        let p = board.prepare(SearchTactics::default()).expect("prepare");
        let (set, opt) = known_incumbent(&board, &p, Optimal::built_in()).expect("known").expect("a known game");
        let graph = MatchGraph::new(&p.model, &p.root);
        let root = TaxUnit { promoted: Bits::EMPTY, cands: p.candidates };
        let plain = solve(&p, &graph, None, root, (opt, &set), None);
        let (_, t) = check(&p, &graph, &Arc::new(Game::new(&p, false, false)), root, (opt, &set), None);
        let counting = Arc::new(Game { propose: false, ..Game::new(&p, false, false) });
        let (counted, c) = learn(&p, &graph, &counting, root, (opt, &set), None);
        assert_eq!((counted.nodes, counted.best, counted.completed), (plain.nodes, plain.best, true));
        assert_eq!((c.steps, c.nodes, c.leaves), (t.steps, t.nodes, t.leaves));
        assert!(c.leaves > 0 && c.proposed == 0 && c.learned == 0 && c.conflicts == 0);
        let mut options = Options::new("images".to_owned());
        options.exchanges.root = true;
        let units = units_of(n, &options, &p).expect("the root");
        let [(id, unit)] = units.as_slice() else { panic!("one unit, not {}", units.len()) };
        assert_eq!((id.as_str(), unit.promoted, unit.cands), ("1215-root", root.promoted, root.cands));
    }

    #[test]
    fn the_count_finds_the_tree_of_the_search_and_root_names_the_whole_game() {
        taxman::with_width!(1215, W => count_1215::<W>(), else panic!("{}", beyond(1215)));
    }

    /// A game with no known optimum is solved from the incumbent of its units' `game` line, as `tune solve` does: here
    /// game 1215 with the table of known games emptied, and an incumbent worth less than P*.  The default search and
    /// the learning of its root unit find P* from it, and the line says `opt` is unknown.
    fn unfinished_1215<const W: usize>() {
        use crate::fixtures::to_hex;
        use taxman::json::Json;
        let n = 1215;
        let board = Board::<W>::for_n(n);
        let p = board.prepare(SearchTactics::default()).expect("prepare");
        let (best_set, opt) = known_incumbent(&board, &p, Optimal::built_in()).expect("known").expect("a known game");
        let low = crate::known::survivors(&p, &best_set.without(best_set.max().expect("a number"))).expect("a set");
        assert!(low.sum() < opt, "the incumbent must be worth less than P*");

        let dir = std::env::temp_dir().join(format!("images-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let units = dir.join("units.jsonl");
        let game_line = |game: u32, sum: u64| {
            format!(
                "{{\"kind\":\"game\",\"game\":{game},\"incumbent\":\"{}\",\"incumbent_sum\":{sum}}}\n",
                to_hex(&low)
            )
        };
        let unit_line = format!(
            "{{\"kind\":\"unit\",\"game\":{n},\"id\":\"{n}-0\",\"promoted\":\"0\",\"cands\":\"{}\"}}\n",
            to_hex(&p.candidates)
        );
        std::fs::write(&units, format!("{}{}", game_line(n, low.sum()), unit_line)).expect("the units file");
        assert_eq!(suite_start(&units, n, &p).expect("a start"), Some(low));
        assert_eq!(suite_start(&units, n + 1, &p).expect("no start"), None, "another game's");
        let wrong = dir.join("wrong.jsonl");
        std::fs::write(&wrong, game_line(n, low.sum() + 1)).expect("the file");
        assert!(suite_start(&wrong, n, &p).is_err(), "an incumbent worth other than its recorded sum");

        let nothing = Optimal::from_json(&Json::parse("[]").expect("JSON")).expect("an empty table");
        let mut options = Options::new("images".to_owned());
        options.params.suite = Some(units.clone());
        let (set, start) = start_of(n, &options, &board, &p, &nothing).expect("the units' incumbent");
        assert_eq!((set, start.incumbent, start.opt), (low, low.sum(), None));
        let (set, start) = start_of(n, &options, &board, &p, Optimal::built_in()).expect("the known optimum");
        assert_eq!((set, start.incumbent, start.opt), (best_set, opt, Some(opt)), "a known game is proved");
        options.exchanges.root = true;
        assert!(start_of(n, &options, &board, &p, &nothing).is_err(), "the whole game needs its optimum");
        options.exchanges.root = false;
        options.params.suite = None;
        assert!(start_of(n, &options, &board, &p, &nothing).is_err(), "no units and no optimum");
        std::fs::remove_dir_all(&dir).expect("removed");

        let graph = MatchGraph::new(&p.model, &p.root);
        let root = TaxUnit { promoted: Bits::EMPTY, cands: p.candidates };
        let plain = solve(&p, &graph, None, root, (low.sum(), &low), None);
        let (learned, l) = learn(
            &p,
            &graph,
            &Arc::new(Game { closure: true, ..Game::new(&p, false, false) }),
            root,
            (low.sum(), &low),
            None,
        );
        assert!(plain.completed && learned.completed);
        assert_eq!((plain.best, learned.best), (opt, opt), "from below, both find P*");
        assert!(l.learned > 0, "nothing learned");
    }

    #[test]
    fn a_game_with_no_known_optimum_starts_from_its_units_incumbent() {
        taxman::with_width!(1215, W => unfinished_1215::<W>(), else panic!("{}", beyond(1215)));
    }

    /// The root search of game `n` from a floor 60 under P*, with no set, with the learning (of the closures, with
    /// `closure`): it finds P* itself.  Returns the conflicts learned within images.
    fn from_floor<const W: usize>(n: u32, closure: bool) -> u64 {
        let board = Board::<W>::for_n(n);
        let p = board.prepare(SearchTactics::default()).expect("prepare");
        let (_, opt) = known_incumbent(&board, &p, Optimal::built_in()).expect("known").expect("a known game");
        let graph = MatchGraph::new(&p.model, &p.root);
        let root = TaxUnit { promoted: Bits::EMPTY, cands: p.candidates };
        let floor = (opt - 60, &Bits::EMPTY);
        let game = Arc::new(Game { closure, ..Game::new(&p, false, false) });
        let (solved, l) = learn(&p, &graph, &game, root, floor, None);
        assert!(solved.completed);
        assert_eq!(solved.best, opt, "n={n}, closure {closure}");
        l.learned
    }

    #[test]
    fn the_learning_finds_the_optimum_from_a_floor() {
        let mut learned = 0;
        for n in [800, 1000, 1215] {
            for closure in [false, true] {
                learned += taxman::with_width!(n, W => from_floor::<W>(n, closure), else panic!("{}", beyond(n)));
            }
        }
        assert!(learned > 0, "nothing learned");
    }
}
