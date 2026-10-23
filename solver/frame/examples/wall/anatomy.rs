//! `wall anatomy`: a hard subtree taken apart.  Units of the tuning suite are solved as `tune solve` solves them (one
//! thread, fresh tables, from the known optimum, so that the subtree is a proof), with a hook on every step of the
//! searcher (`search::StepHook`), which writes what each step saw and did; a script of the notebook entry reads it.
//! With `--root` the unit is the whole game (`GAME-root`), as `tune solve` solves it (`images::units_of`).
//!
//! ```text
//! cargo run --profile test --features studies --example wall -- anatomy --known FILE (--suite FILE --ids ID[,ID...]
//!     | --root) [--learn [--closure]] [--trace DIR] [--prefer FILE] [--best-cpu S] [--nodes K]
//!     [--tactic NAME=VALUE]... N
//! ```
//!
//! `N` is the game of the units, `--ids` the suite's ids of them (`N-INDEX`).  Each unit has a line on stdout: `id`,
//! `depth` (its promotions), `cands`, `need` (what a completion must add to beat P*), `nodes` (the engine's
//! steps), `steps` (those the hook was told of: the same), `cpu_s` (the thread's), `completed`, `best` (the best
//! promotion sum; P* unless the subtree beat it), `root_value` and `root_chosen` (the GF(2) bound of the
//! unit's first step and the size of its heaviest set), `conflicts` (the distinct conflicts the steps' checks found),
//! `residues` (the distinct residues of each frame and of the whole node, then the distinct states of each frame and
//! of all of them, below), `asked` and `overridden` (how often
//! the hook was asked which member to branch on, and how often its answer was not strong branching's),
//! `known_own`, `known_image`, `known_both` and `known_unexplained` (the known conflicts of the steps' checks by the
//! source of the nogoods that give them, below), `nogoods_own` and `nogoods_image` (the nogoods of each source that
//! the hook's mirror of the searcher's table took), `tactics` (the changes from the default tactics), `prefer` (the
//! order's file, or null), with `--learn` the learning's `learning` (`"learn"` or `"closure"`), `computed` (the
//! conflicts the checks computed, each asked for its images), `proposed` (the images proposed) and `learned` (the
//! conflicts learned within them), and with `--best-cpu` the best completion found (below).
//!
//! **The learning** (`--learn [--closure]`).  Each searcher's hook holds a `images::Learner` and tells it what the
//! searcher tells the hook, so that the search learns the images of its conflicts as `wall images --learn` makes it
//! learn them: it proposes, wherever a check computes a conflict, the images of the conflict under the cheap exchanges
//! (with `--closure`, the images of the images that are conflicts too), and the searcher learns a conflict within each
//! as a nogood.  The tree changes, the best does not.  Without `--learn` the search is the default one and its tree
//! is the same as with no hook.
//!
//! **The sources of the known conflicts.**  A check finds its conflict *known* when the searcher's table of nogoods
//! (tactic `nogoods`) has one, and *computes* it otherwise.  A nogood `n` gives the conflict `k` of a node with the
//! promotions `P` when `k ⊆ n` and `n − k ⊆ P`.  The hook keeps a *mirror* of the table: each nogood the searcher
//! learns, with its source, *own* (the nogood of a conflict a check computed, which the hook makes of the conflict
//! as the searcher does) or *image* (a nogood learned within a set that the learning proposed).  The mirror forgets
//! everything when the searcher's table does, after each `nogood_period` steps and when the table is full, so
//! `nogood_keep` and `nogood_limit` must be at their defaults.  The *code* of a known conflict is 1 if only own nogoods
//! of the mirror give it, 2 if only image ones, 3 if both, and 0 if none does, which is a fault of the instrument
//! (`known_unexplained`, to be 0); -1 where the check has no conflict or computed it.  The node's check sees the
//! nogoods learned before the step, and the child's check those of the node's check too.  The line counts the
//! known conflicts of the node's and of the child's checks together by code: `known_own` (code 1), `known_image` (2),
//! `known_both` (3) and `known_unexplained` (0).
//!
//! **The trace** (`--trace DIR`): for unit `ID`, `DIR/ID.steps.tsv` has a line a step, in the order of the steps,
//! under a header: `d` (the node's depth on the stack below the unit's node), `np` (its promotions), `nc` (the
//! candidates its loop had), `need`; the node's own bound check, if the step made one (`nv` the bound, `nk` the size of
//! its heaviest set, `kid` the id of its conflict, `kc` 1 if that conflict was computed and learned rather than known,
//! `ksrc` the code of its source if it was known), or `inh` 1 if the verdict was the one the node's check as a child
//! left; the branch (`br` the member, 0 if none,
//! `lf` its live factors at the node: its maximal factors that are factors of the frame it would move into, `nadd` and
//! `sadd` the size and sum of what the child promotes, `nf` the forced members); the child's check (`cv`, `ckid`,
//! `ckc`, `cksrc`; -1 where there is none); `end` (`P` the child pushed, `E` a child with no candidates, `F` the
//! promotion failed, `C<r>` the child cut and `L<r>` the loop ended, `r` the `search::cut` reason); `gain` and `loss`,
//! what the node's heaviest set gained and lost since the node's last check (members, comma separated, or `-`; only at
//! a step after the node's first: after it went on without a member, the substitutes for it); and the residues `r0`,
//! `r1`, ... and `rall`, and the states `s0`, `s1`, ... and `sall` (ids).  A frame's *residue* at a step is its state
//! (moves, factors, freedoms) and the node's candidates whose promotion touches it (those of home `f` and `f + 1`);
//! `rall` is the state of all frames and all the candidates, which is the node's whole residual problem but for the sum
//! it has; `s0`, ... are the frames' states alone and `sall` all of them.  Ids number the distinct values in order of
//! appearance.  `DIR/ID.conflicts.tsv` has the conflicts (`id`, `size`, `members` ascending, comma
//! separated), and `DIR/N.candidates.tsv` the game's candidates (`x`, `home`, `factors`: its maximal factors).
//!
//! **The nogood log** (`--trace DIR`): `DIR/ID.nogoods.tsv` has a line for each nogood the mirror takes, in the order
//! they come, under a header: `step` (the step in progress), `src` (`C` own, `I` image), `d` (the depth of the step's
//! node), `np` (the promotions of the check; of the `C` line before it, for an `I`), and the sets `nogood` (`-` if
//! `FrameGraph::nogood` made none, which only a `C` line can show), `conflict` (the one the check computed, or the one
//! learned within the set), `set` (the set proposed; `-` for a `C`), `promoted`, `chosen` (the check's heaviest set;
//! the last check's, for an `I`) and `cands` (the check's; `-` for an `I`).  A set is the integer Σ 2^i over its
//! members, `i` the member's index among the game's candidates in ascending order (the order of `N.candidates.tsv`), in
//! lower case hex with no leading zeros (`0` for the empty set).
//!
//! **The branching order** (`--prefer FILE`): a list of numbers (whitespace separated), the first the most preferred.
//! Where strong branching found no member of a node's conflict forced, the node branches on the member of the conflict
//! that comes first in the list, and on strong branching's choice if none is in it.  Any member keeps the search exact
//! (`search::StepHook::branch`), so the best is the same and only the tree changes.  `--nodes K` stops a unit after
//! about K steps (`completed` false).
//!
//! **The best completion** (`--best-cpu S`): large neighborhood search below the unit's node for S CPU seconds (`lns`,
//! its units restricted to the node's candidates, from the greedy completion of the node), not sound; the line has
//! `best_completion` (the sum of what it found beyond the node's promotions), its members (`completion`), the heaviest
//! set of the unit's first step (`chosen`) with each member's home and live factors at the node (`chosen_lf`, as
//! `[x, home, live]`), and that set's minimal conflicts at the node, found one after the other with each one's
//! members dropped before the next (`cores`).

use super::images::{units_of, Game, Learned, Learner};
use super::{beyond, Line, Options};
use crate::counters::cpu_seconds;
use crate::fixtures::from_hex;
use crate::known::{known_incumbent, survivors};
use crate::lns::run_unit_with;
use crate::rng::Rng;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::Control;
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::matchbound::MatchGraph;
use taxman::model::{FrameGraph, FrameSet, Keep, Scratch};
use taxman::search::{MakeHook, Prepared, SearchSpace, StepEnd, StepHook, StepRecord, TaxSearcher, TaxUnit};
use taxman::tactics::Tactics;

/// The options of `anatomy`.
#[derive(Clone, Default)]
pub struct Params {
    /// The units' ids (`--ids`), the directory of the traces (`--trace`), the branching order (`--prefer`) and the
    /// CPU seconds of the search for the best completion (`--best-cpu`; 0: none).
    pub ids: Vec<String>,
    pub trace: Option<PathBuf>,
    pub prefer: Option<PathBuf>,
    pub best_cpu: f64,
}

// ---------------------------------------------------------------------------------------------------------
// The hook

/// What every searcher's hook knows of the game: its frames, the candidates that touch each, the order to branch in,
/// and what it needs of the searcher's nogoods.
struct Known<const W: usize> {
    /// For each frame, the candidates whose promotion touches it (of home `f`, out of it, and `f + 1`, into it).
    touching: Vec<Bits<W>>,
    /// For each number up to the largest candidate: its home frame, the frame it moves into and its maximal factors
    /// (empty for a number that is no candidate).
    home: Vec<usize>,
    into: Vec<usize>,
    factors: Vec<Vec<u32>>,
    /// For each number, its place in the preferred order (`u32::MAX`: not in it); empty without one.
    rank: Vec<u32>,
    /// The model and the root, for the nogood of a conflict (`FrameGraph::nogood`, as the searcher makes it).
    model: Arc<FrameGraph>,
    root: FrameSet<W>,
    /// Whether the searcher keeps a table of nogoods (tactic `nogoods`), and after how many steps it forgets them
    /// (`None`: never).
    nogoods: bool,
    forget_period: Option<u64>,
    /// For each number up to the largest candidate, its index among the candidates in ascending order (`u32::MAX`:
    /// no candidate); and the number of candidates.
    index: Vec<u32>,
    candidates: usize,
}

impl<const W: usize> Known<W> {
    fn new(p: &Prepared<W>, prefer: &[u32]) -> Known<W> {
        let model = &p.model;
        let frames = p.root.frames().len();
        let top = p.candidates.max().unwrap_or(0) as usize;
        let (mut home, mut into_frame, mut factors) = (vec![0; top + 1], vec![0; top + 1], vec![Vec::new(); top + 1]);
        let mut touching = vec![Bits::EMPTY; frames];
        for x in p.candidates.iter() {
            let (into, out) = model.frames_touched(x);
            home[x as usize] = out.unwrap_or(into);
            into_frame[x as usize] = into;
            factors[x as usize] = model.graph().factors(x).iter().map(|&y| u32::from(y)).collect();
            for f in [Some(into), out].into_iter().flatten().filter(|&f| f < frames) {
                touching[f].insert(x);
            }
        }
        let mut rank = Vec::new();
        if !prefer.is_empty() {
            rank = vec![u32::MAX; top + 1];
            for (i, &x) in prefer.iter().enumerate() {
                if let Some(slot) = rank.get_mut(x as usize).filter(|slot| **slot == u32::MAX) {
                    *slot = i as u32;
                }
            }
        }
        let mut index = vec![u32::MAX; top + 1];
        for (i, x) in p.candidates.iter().enumerate() {
            index[x as usize] = i as u32;
        }
        let forget_period = p.tactics.forgetting().period;
        let candidates = p.candidates.len() as usize;
        let (model, root, nogoods) = (Arc::clone(&p.model), p.root, p.tactics.nogoods);
        Known {
            touching,
            home,
            into: into_frame,
            factors,
            rank,
            model,
            root,
            nogoods,
            forget_period,
            index,
            candidates,
        }
    }

    /// `set` as the integer Σ 2^i over its members, `i` the member's index among the candidates, in lower case hex
    /// with no leading zeros; an error if a member is no candidate.
    fn hex(&self, set: &Bits<W>) -> Result<String, String> {
        let mut words = vec![0u64; self.candidates.div_ceil(64)];
        for x in set.iter() {
            let index = self.index.get(x as usize).copied().filter(|&i| i != u32::MAX);
            let i = index.ok_or_else(|| format!("the nogood log: {x} is no candidate"))? as usize;
            words[i / 64] |= 1 << (i % 64);
        }
        let Some(top) = words.iter().rposition(|&word| word != 0) else { return Ok("0".to_owned()) };
        let mut text = format!("{:x}", words[top]);
        for word in words[..top].iter().rev() {
            write!(text, "{word:016x}").expect("writing to a string");
        }
        Ok(text)
    }

    /// The line of the nogood log for `row`, learned in the step `step` at the node's depth `depth`.
    fn nogood_line(&self, step: u64, depth: usize, row: &NogoodRow<W>) -> Result<String, String> {
        let hex = |set: Option<&Bits<W>>| set.map_or_else(|| Ok("-".to_owned()), |set| self.hex(set));
        let source = match row.source {
            Source::Own => 'C',
            Source::Image => 'I',
        };
        let (nogood, set, cands) = (hex(row.nogood.as_ref())?, hex(row.set.as_ref())?, hex(row.cands.as_ref())?);
        let (conflict, promoted, chosen) = (self.hex(&row.conflict)?, self.hex(&row.promoted)?, self.hex(&row.chosen)?);
        Ok(format!(
            "{step}\t{source}\t{depth}\t{}\t{nogood}\t{conflict}\t{set}\t{promoted}\t{chosen}\t{cands}\n",
            row.promoted.len()
        ))
    }

    /// The maximal factors of the candidate `x` that are factors of the frame it moves into, in `state`.
    fn live_factors(&self, state: &FrameSet<W>, x: u32) -> usize {
        let factors = &state.frames()[self.into[x as usize]].factors;
        self.factors[x as usize].iter().filter(|&&y| factors.contains(y)).count()
    }
}

/// The most nogoods the searcher's table holds: it forgets them all when it is full (`NOGOOD_CAPACITY` in
/// `search/searcher.rs`, which is private).
const NOGOOD_CAPACITY: usize = 1 << 16;

/// Where a nogood of the searcher's table came from; the value is its bit in a code (see [`Mirror::code`]).
#[derive(Clone, Copy)]
enum Source {
    /// The nogood of a conflict that a bound check computed.
    Own = 1,
    /// A nogood learned within a set that the learning proposed.
    Image = 2,
}

/// A mirror of the searcher's table of nogoods (tactic `nogoods`): the nogoods it holds, each with its source and its
/// place in the order they came in, and for each number the nogoods it is a member of (indices of `held`).
#[derive(Default)]
struct Mirror<const W: usize> {
    held: Vec<(Bits<W>, Source, u64)>,
    by_member: HashMap<u32, Vec<usize>>,
    /// The place of the next nogood, and of the first that came in the step in progress.
    next: u64,
    began: u64,
    /// How many nogoods of each source the mirror took.
    own: u64,
    image: u64,
}

impl<const W: usize> Mirror<W> {
    fn add(&mut self, nogood: Bits<W>, source: Source) {
        if self.held.len() >= NOGOOD_CAPACITY {
            self.clear();
        }
        for x in nogood.iter() {
            self.by_member.entry(x).or_default().push(self.held.len());
        }
        self.held.push((nogood, source, self.next));
        self.next += 1;
        match source {
            Source::Own => self.own += 1,
            Source::Image => self.image += 1,
        }
    }

    fn clear(&mut self) {
        self.held.clear();
        self.by_member.clear();
    }

    /// The step in progress is over: the next one begins with the nogoods held now.
    fn end_step(&mut self) {
        self.began = self.next;
    }

    /// The code of the nogoods that give a node with the promotions `promoted` the conflict `k` (as `Track::giving`
    /// of `images`: `k ⊆ n` and `n − k ⊆ promoted`), among those that came before the place `before`: 1 if only own
    /// ones do, 2 if only image ones, 3 if both, 0 if none does.
    fn code(&self, promoted: &Bits<W>, k: &Bits<W>, before: u64) -> usize {
        let Some(ids) = k.max().and_then(|x| self.by_member.get(&x)) else { return 0 };
        let mut code = 0;
        for &id in ids {
            let (n, source, place) = self.held[id];
            if place >= before {
                break;
            }
            if k.is_subset(&n) && (n - *k).is_subset(promoted) {
                code |= source as usize;
            }
        }
        code
    }
}

/// A line of the nogood log (see the module docs): the nogood a conflict made (`None`: it made none), and the sets of
/// the check.
struct NogoodRow<const W: usize> {
    source: Source,
    nogood: Option<Bits<W>>,
    conflict: Bits<W>,
    set: Option<Bits<W>>,
    promoted: Bits<W>,
    chosen: Bits<W>,
    cands: Option<Bits<W>>,
}

/// The files of the trace of a unit.
struct Files {
    steps: BufWriter<File>,
    nogoods: BufWriter<File>,
}

/// What the hooks of a unit's solve gathered, and where they write the steps and the nogoods.
struct Tally<const W: usize> {
    files: Option<Files>,
    steps: u64,
    /// The node's depth on the stack for the next step.
    depth: usize,
    conflicts: HashMap<Bits<W>, u32>,
    listed: Vec<Bits<W>>,
    /// The distinct residues of each frame and of the whole node, then the distinct states of each frame and of all,
    /// by their hash.
    residues: Vec<HashMap<u64, u32>>,
    root: Option<(u64, Bits<W>)>,
    /// The heaviest set of the node at each depth of the stack, as its last check found it.
    heaviest: Vec<Option<Bits<W>>>,
    asked: u64,
    overridden: u64,
    error: Option<String>,
    /// The mirror of the searcher's nogood table, and the promotions and heaviest set of the check that last computed
    /// a conflict (the images learned after it are of its node).
    mirror: Mirror<W>,
    last: (Bits<W>, Bits<W>),
    /// The known conflicts of the node's and the child's checks by the code of their source (index 0 to 3).
    known: [u64; 4],
    /// What the learning of the searcher's hook gathered, which the learner adds to when its searcher is dropped.
    learned: Arc<Mutex<Learned>>,
}

impl<const W: usize> Tally<W> {
    fn new(frames: usize, files: Option<Files>) -> Tally<W> {
        Tally {
            files,
            steps: 0,
            depth: 0,
            conflicts: HashMap::new(),
            listed: Vec::new(),
            residues: vec![HashMap::new(); 2 * (frames + 1)],
            root: None,
            heaviest: Vec::new(),
            asked: 0,
            overridden: 0,
            error: None,
            mirror: Mirror::default(),
            last: (Bits::EMPTY, Bits::EMPTY),
            known: [0; 4],
            learned: Arc::default(),
        }
    }

    /// The code of the source of the conflict `k` of a check at the promotions `promoted` (-1: the check found none,
    /// or computed it), counted.  The node's check (`began`) sees the nogoods that came before the step; the child's
    /// check, those of the node's check too.
    fn source(&mut self, k: Option<Bits<W>>, promoted: &Bits<W>, began: bool) -> i64 {
        let Some(k) = k else { return -1 };
        let before = if began { self.mirror.began } else { u64::MAX };
        let code = self.mirror.code(promoted, &k, before);
        self.known[code] += 1;
        code as i64
    }

    /// Takes the nogood of `row` into the mirror, and writes its line to the nogood log.
    fn learn(&mut self, known: &Known<W>, row: &NogoodRow<W>) {
        if let Some(n) = row.nogood {
            self.mirror.add(n, row.source);
        }
        let Some(files) = &mut self.files else { return };
        let written = known
            .nogood_line(self.steps + 1, self.depth, row)
            .and_then(|line| files.nogoods.write_all(line.as_bytes()).map_err(|e| format!("the nogood log: {e}")));
        if let Err(e) = written {
            self.error.get_or_insert(e);
        }
    }

    fn conflict(&mut self, k: Option<Bits<W>>) -> i64 {
        let Some(k) = k else { return -1 };
        let next = self.listed.len() as u32;
        let id = *self.conflicts.entry(k).or_insert(next);
        if id == next {
            self.listed.push(k);
        }
        i64::from(id)
    }

    fn residue(&mut self, slot: usize, hash: u64) -> u32 {
        let map = &mut self.residues[slot];
        let next = map.len() as u32;
        *map.entry(hash).or_insert(next)
    }
}

/// A 64-bit hash of `value` (SipHash with fixed keys: the same in every run).
fn hash_of(value: &impl Hash) -> u64 {
    let mut h = DefaultHasher::new();
    value.hash(&mut h);
    h.finish()
}

/// The hook of one searcher: everything goes to the tally of the unit being solved (one thread searches a unit, so
/// the lock is never contended).  A searcher made while no unit is (the search for the best completion) is told
/// nothing and branches as strong branching does.  With `--learn` the hook also tells a learner of `images` what the
/// searcher tells it, and the learner has the searcher learn the images of its conflicts.
struct Recorder<const W: usize> {
    known: Arc<Known<W>>,
    tally: Option<Arc<Mutex<Tally<W>>>>,
    scratch: Scratch<W>,
    learner: Option<Learner<W>>,
}

/// The hooks of a game's searchers ([`Recorder`]), and the tally of the unit being solved.  With `--learn` there is a
/// game for their learners to share.
struct Hooks<const W: usize> {
    known: Arc<Known<W>>,
    game: Option<Arc<Game<W>>>,
    current: Arc<Mutex<Option<Arc<Mutex<Tally<W>>>>>>,
}

impl<const W: usize> Hooks<W> {
    fn new(known: Known<W>, game: Option<Arc<Game<W>>>) -> Hooks<W> {
        Hooks { known: Arc::new(known), game, current: Arc::default() }
    }

    /// What makes each searcher's hook ([`SearchSpace::hooks`]).
    fn maker(&self) -> impl Fn() -> Box<dyn StepHook<W>> + Sync + 'static {
        let (known, game, current) = (Arc::clone(&self.known), self.game.clone(), Arc::clone(&self.current));
        move || {
            let tally = current.lock().unwrap_or_else(PoisonError::into_inner).clone();
            let learner = tally.as_ref().zip(game.as_ref()).map(|(tally, game)| {
                let shared = Arc::clone(&tally.lock().unwrap_or_else(PoisonError::into_inner).learned);
                Learner::new(Arc::clone(game), shared)
            });
            Box::new(Recorder { known: Arc::clone(&known), tally, scratch: Scratch::new(), learner })
        }
    }
}

const HEADER: &str =
    "d\tnp\tnc\tneed\tnv\tnk\tkid\tkc\tksrc\tinh\tbr\tlf\tnadd\tsadd\tnf\tcv\tckid\tckc\tcksrc\tend\tgain\tloss";

/// The header of the nogood log.
const NOGOODS_HEADER: &str = "step\tsrc\td\tnp\tnogood\tconflict\tset\tpromoted\tchosen\tcands";

fn end_code(end: StepEnd) -> String {
    match end {
        StepEnd::Child => "P".into(),
        StepEnd::Leaf => "E".into(),
        StepEnd::ChildFailed => "F".into(),
        StepEnd::ChildCut(r) => format!("C{r}"),
        StepEnd::LoopEnd(r) => format!("L{r}"),
    }
}

impl<const W: usize> Tally<W> {
    /// Takes in the step `r` of a node in `state`: the trace's line, if it is written, and the mirror's forgetting.
    fn record(&mut self, known: &Known<W>, r: &StepRecord<W>, state: &FrameSet<W>) {
        let t = self;
        t.steps += 1;
        let d = t.depth;
        t.depth = match r.end {
            StepEnd::Child => d + 1,
            StepEnd::LoopEnd(_) => d.saturating_sub(1),
            _ => d,
        };
        if t.root.is_none() {
            t.root = r.node.map(|c| (c.value, c.chosen));
        }
        let node = r.node;
        let kid = t.conflict(node.and_then(|c| c.conflict));
        let ckid = t.conflict(r.child.and_then(|c| c.conflict));
        // Where the known conflicts came from: the node's check promotes `r.promoted`, the child's also `r.added`.
        let ksrc = t.source(node.and_then(|c| c.conflict.filter(|_| !c.computed)), &r.promoted, true);
        let cksrc = t.source(r.child.and_then(|c| c.conflict.filter(|_| !c.computed)), &(r.promoted | r.added), false);
        let frames = state.frames();
        let mut residues = Vec::with_capacity(2 * (frames.len() + 1));
        for (f, frame) in frames.iter().enumerate() {
            let touching = known.touching.get(f).copied().unwrap_or_default();
            residues.push(t.residue(f, hash_of(&(frame, r.cands & touching))));
        }
        residues.push(t.residue(frames.len(), hash_of(&(frames, r.cands))));
        let states = frames.len() + 1;
        for (f, frame) in frames.iter().enumerate() {
            residues.push(t.residue(states + f, hash_of(frame)));
        }
        residues.push(t.residue(states + frames.len(), hash_of(&frames)));
        // What the node's heaviest set gained and lost since its last check (at a step after the first: the members
        // that replaced the one its last step branched on, if it went on without it).
        let first = r.inherited || t.steps == 1;
        if t.heaviest.len() < d + 2 {
            t.heaviest.resize(d + 2, None);
        }
        let now = node.map(|c| c.chosen);
        let swap = match (first, t.heaviest[d], now) {
            (false, Some(before), Some(now)) => Some((now - before, before - now)),
            _ => None,
        };
        if now.is_some() {
            t.heaviest[d] = now;
        }
        if r.end == StepEnd::Child {
            t.heaviest[d + 1] = r.child.map(|c| c.chosen);
        }
        if let Some(files) = &mut t.files {
            let lf = r.branch.map_or(-1, |x| known.live_factors(state, x) as i64);
            let members = |set: Option<Bits<W>>| match set.filter(|set| !set.is_empty()) {
                Some(set) => set.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","),
                None => "-".to_owned(),
            };
            let value = |c: Option<u64>| c.map_or(-1, |v| v as i64);
            let line = format!(
                "{d}\t{}\t{}\t{}\t{}\t{}\t{kid}\t{}\t{ksrc}\t{}\t{}\t{lf}\t{}\t{}\t{}\t{}\t{ckid}\t{}\t{cksrc}\t\
                 {}\t{}\t{}\t{}\n",
                r.promoted.len(),
                r.cands.len(),
                r.need,
                value(node.map(|c| c.value)),
                node.map_or(-1, |c| i64::from(c.chosen.len())),
                u8::from(node.is_some_and(|c| c.computed)),
                u8::from(r.inherited),
                r.branch.unwrap_or(0),
                r.added.len(),
                r.added.sum(),
                r.forced.len(),
                value(r.child.map(|c| c.value)),
                u8::from(r.child.is_some_and(|c| c.computed)),
                end_code(r.end),
                members(swap.map(|s| s.0)),
                members(swap.map(|s| s.1)),
                residues.iter().map(u32::to_string).collect::<Vec<_>>().join("\t"),
            );
            if let Err(e) = files.steps.write_all(line.as_bytes()) {
                t.error.get_or_insert(format!("the trace: {e}"));
            }
        }
        // The searcher's table forgets its nogoods after each period of steps (`forget_nogoods`), as the step ends.
        if known.forget_period.is_some_and(|period| t.steps.is_multiple_of(period)) {
            t.mirror.clear();
        }
        t.mirror.end_step();
    }
}

impl<const W: usize> StepHook<W> for Recorder<W> {
    fn step(&mut self, r: &StepRecord<W>, state: &FrameSet<W>, child: Option<&FrameSet<W>>) {
        let Some(tally) = &self.tally else { return };
        tally.lock().unwrap_or_else(PoisonError::into_inner).record(&self.known, r, state);
        if let Some(learner) = &mut self.learner {
            learner.step(r, state, child);
        }
    }

    fn branch(&mut self, conflict: &Bits<W>, strong: u32) -> u32 {
        let Some(tally) = &self.tally else { return strong };
        let rank = &self.known.rank;
        let mut t = tally.lock().unwrap_or_else(PoisonError::into_inner);
        t.asked += 1;
        if rank.is_empty() {
            return strong;
        }
        let place = |x: u32| rank.get(x as usize).copied().unwrap_or(u32::MAX);
        let first = conflict.iter().min_by_key(|&x| place(x)).filter(|&x| place(x) != u32::MAX);
        let chosen = first.unwrap_or(strong);
        t.overridden += u64::from(chosen != strong);
        chosen
    }

    /// The searcher has computed `conflict`, learned its nogood (if it keeps a table), and asks for sets to learn
    /// within: the hook takes in the nogood as the searcher made it, and then the learner proposes.
    fn propose(
        &mut self,
        state: &FrameSet<W>,
        promoted: &Bits<W>,
        cands: &Bits<W>,
        chosen: &Bits<W>,
        conflict: &Bits<W>,
        more: &mut Vec<Bits<W>>,
    ) {
        let Some(tally) = &self.tally else { return };
        let known = &*self.known;
        let mut t = tally.lock().unwrap_or_else(PoisonError::into_inner);
        t.last = (*promoted, *chosen);
        if known.nogoods {
            let nogood = known.model.nogood(&mut self.scratch, &known.root, promoted, conflict);
            let row = NogoodRow {
                source: Source::Own,
                nogood,
                conflict: *conflict,
                set: None,
                promoted: *promoted,
                chosen: *chosen,
                cands: Some(*cands),
            };
            t.learn(known, &row);
        }
        drop(t);
        if let Some(learner) = &mut self.learner {
            learner.propose(state, promoted, cands, chosen, conflict, more);
        }
    }

    /// The searcher has learned `conflict` within the set `set` that was proposed, with the nogood it made.
    fn learned(&mut self, set: &Bits<W>, conflict: &Bits<W>, nogood: Option<&Bits<W>>) {
        let Some(tally) = &self.tally else { return };
        let mut t = tally.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(&n) = nogood {
            let (promoted, chosen) = t.last;
            let row = NogoodRow {
                source: Source::Image,
                nogood: Some(n),
                conflict: *conflict,
                set: Some(*set),
                promoted,
                chosen,
                cands: None,
            };
            t.learn(&self.known, &row);
        }
        drop(t);
        if let Some(learner) = &mut self.learner {
            learner.learned(set, conflict, nogood);
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// The units

/// The units `ids` of game `n` in the suite at `path`, each with its id and depth.
pub(super) fn suite_units<const W: usize>(
    path: &Path,
    n: u32,
    ids: &[String],
) -> Result<Vec<(String, TaxUnit<W>)>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut found = HashMap::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let json = Json::parse(line).map_err(|e| format!("{}: {e:?}", path.display()))?;
        let field = |key: &str| json.get(key).and_then(Json::as_str).unwrap_or_default();
        if field("kind") == "unit" && json.get("game").and_then(Json::as_u32) == Some(n) {
            let unit = TaxUnit { promoted: from_hex(field("promoted"))?, cands: from_hex(field("cands"))? };
            found.insert(field("id").to_owned(), unit);
        }
    }
    ids.iter()
        .map(|id| {
            found.get(id).map(|&unit| (id.clone(), unit)).ok_or_else(|| format!("no unit {id} of {n} in the suite"))
        })
        .collect()
}

/// The incumbent the units of game `n` in the suite at `path` start from when the game has no known optimum: the
/// incumbent of the checkpoint they were cut from, which the `game` line of `tune units` holds (`incumbent` in hex, and
/// its sum `incumbent_sum`), reduced to what can be promoted in `p` as `tune solve` reduces it.  `None` if the file has
/// no such line for the game.
pub(super) fn suite_start<const W: usize>(path: &Path, n: u32, p: &Prepared<W>) -> Result<Option<Bits<W>>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let json = Json::parse(line).map_err(|e| format!("{}: {e:?}", path.display()))?;
        let is_game = json.get("kind").and_then(Json::as_str) == Some("game");
        if !is_game || json.get("game").and_then(Json::as_u32) != Some(n) {
            continue;
        }
        let (Some(hex), Some(sum)) =
            (json.get("incumbent").and_then(Json::as_str), json.get("incumbent_sum").and_then(Json::as_u64))
        else {
            return Ok(None);
        };
        let set = survivors(p, &from_hex(hex)?)?;
        if set.sum() != sum {
            return Err(format!("the checkpoint's incumbent is worth {sum}, but {} of it can be promoted", set.sum()));
        }
        return Ok(Some(set));
    }
    Ok(None)
}

/// The numbers of the file at `path`, in order.
fn read_order(path: &Path) -> Result<Vec<u32>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    text.split_whitespace().map(|word| word.parse().map_err(|e| format!("{}: {word}: {e}", path.display()))).collect()
}

/// What solving a unit with the hook gave.
struct Solved<const W: usize> {
    nodes: u64,
    cpu_s: f64,
    completed: bool,
    best: u64,
    tally: Tally<W>,
    /// What the learners of the hooks gathered (nothing without `--learn`).
    learned: Learned,
}

/// Solves `unit` on one thread from the incumbent `start`, with fresh tables, as `tune solve` does, each step told to
/// the [`Recorder`] of `space`'s searcher (made by `hooks`), which writes to `files` (if any).
fn solve<const W: usize>(
    space: &SearchSpace<'_, W>,
    hooks: &Hooks<W>,
    unit: TaxUnit<W>,
    start: &Bits<W>,
    node_limit: Option<u64>,
    files: Option<Files>,
) -> Result<Solved<W>, String> {
    let tally = Arc::new(Mutex::new(Tally::new(space.root.frames().len(), files)));
    *hooks.current.lock().unwrap_or_else(PoisonError::into_inner) = Some(Arc::clone(&tally));
    let before = cpu_seconds();
    let ran = run_unit_with(&Control::new(), unit, start, node_limit, |_| TaxSearcher::new(space));
    let cpu_s = before.zip(cpu_seconds()).map_or(0.0, |(a, b)| b - a);
    *hooks.current.lock().unwrap_or_else(PoisonError::into_inner) = None;
    let mut tally = Arc::try_unwrap(tally)
        .map_err(|_| "the hooks outlived the search".to_owned())?
        .into_inner()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(files) = &mut tally.files {
        files.steps.flush().map_err(|e| format!("the trace: {e}"))?;
        files.nogoods.flush().map_err(|e| format!("the nogood log: {e}"))?;
    }
    if let Some(e) = tally.error.take() {
        return Err(e);
    }
    // Every learner has added its counts: its searcher was dropped.
    let learned = std::mem::take(&mut *tally.learned.lock().unwrap_or_else(PoisonError::into_inner));
    Ok(Solved { nodes: ran.nodes, cpu_s, completed: ran.completed, best: ran.set.sum(), tally, learned })
}

// ---------------------------------------------------------------------------------------------------------
// The best completion and the cores

/// The best completion of the node of `unit` that large neighborhood search finds in `cpu` CPU seconds of this thread:
/// from the greedy completion (the node's candidates promoted from the largest down where they can be), units that fix
/// the node's promotions and a random 70 to 90% of the completion's, searching the rest of the node's candidates for at
/// most 1,024 steps.  Returns the completion (without the node's promotions).
fn best_completion<const W: usize>(space: &SearchSpace<'_, W>, unit: &TaxUnit<W>, cpu: f64, n: u32) -> Bits<W> {
    let model = space.model;
    let mut scratch = Scratch::new();
    let mut state = model.promote_all_using(&mut scratch, space.root, &unit.promoted).expect("the unit's promotions");
    let mut best = unit.promoted;
    let mut cands = unit.cands.to_vec();
    cands.reverse();
    for x in cands {
        let before = state;
        if model.promote_in_place_using(&mut scratch, &mut state, x) && state.frames().iter().all(|f| f.freedoms >= 0) {
            best.insert(x);
        } else {
            state = before;
        }
    }
    let mut rng = Rng(0xA4A7_0000 ^ u64::from(n));
    let began = cpu_seconds().unwrap_or(0.0);
    let control = Control::new();
    while cpu_seconds().unwrap_or(f64::INFINITY) - began < cpu {
        let mut members = (best - unit.promoted).to_vec();
        rng.shuffle(&mut members);
        let share = 0.7 + 0.2 * rng.unit();
        let fixed: Bits<W> = members[..(members.len() as f64 * share).round() as usize].iter().copied().collect();
        let unit = TaxUnit { promoted: unit.promoted | fixed, cands: unit.cands - fixed };
        let ran = run_unit_with(&control, unit, &best, Some(1024), |_| TaxSearcher::new(space));
        if ran.set.sum() > best.sum() {
            best = ran.set;
        }
    }
    best - unit.promoted
}

/// The minimal conflicts of `set` at `state`, one after the other, each one's members dropped before the next.
fn cores<const W: usize>(model: &FrameGraph, state: &FrameSet<W>, set: &Bits<W>) -> Vec<Bits<W>> {
    let mut scratch = Scratch::new();
    let (mut left, mut found) = (*set, Vec::new());
    while let Some(k) = model.conflict_using(&mut scratch, state, &left, true) {
        if k.is_empty() {
            break;
        }
        left -= k;
        found.push(k);
    }
    found
}

// ---------------------------------------------------------------------------------------------------------
// The command

fn create(path: &Path) -> Result<BufWriter<File>, String> {
    File::create(path).map(BufWriter::new).map_err(|e| format!("{}: {e}", path.display()))
}

/// Writes the game's candidates: each with its home frame and its maximal factors.
fn write_candidates<const W: usize>(path: &Path, p: &Prepared<W>, known: &Known<W>) -> Result<(), String> {
    let mut out = create(path)?;
    let mut text = String::from("x\thome\tfactors\n");
    for x in p.candidates.iter() {
        let factors: Vec<String> = known.factors[x as usize].iter().map(u32::to_string).collect();
        text.push_str(&format!("{x}\t{}\t{}\n", known.home[x as usize], factors.join(",")));
    }
    out.write_all(text.as_bytes()).and_then(|()| out.flush()).map_err(|e| format!("{}: {e}", path.display()))
}

/// Writes the game's candidates as the trace has them, for another study (`near`).
pub(super) fn write_game_candidates<const W: usize>(path: &Path, p: &Prepared<W>) -> Result<(), String> {
    write_candidates(path, p, &Known::new(p, &[]))
}

fn write_conflicts<const W: usize>(path: &Path, listed: &[Bits<W>]) -> Result<(), String> {
    let mut out = create(path)?;
    let mut text = String::from("id\tsize\tmembers\n");
    for (id, k) in listed.iter().enumerate() {
        let members: Vec<String> = k.iter().map(|x| x.to_string()).collect();
        text.push_str(&format!("{id}\t{}\t{}\n", k.len(), members.join(",")));
    }
    out.write_all(text.as_bytes()).and_then(|()| out.flush()).map_err(|e| format!("{}: {e}", path.display()))
}

/// Creates the files of the trace of the unit `id` in `dir`, with their headers (`frames` is the number of frames).
fn open_files(dir: &Path, id: &str, frames: usize) -> Result<Files, String> {
    let mut steps = create(&dir.join(format!("{id}.steps.tsv")))?;
    let numbered = |prefix: &str| (0..frames).map(|f| format!("{prefix}{f}")).collect::<Vec<_>>().join("\t");
    writeln!(steps, "{HEADER}\t{}\trall\t{}\tsall", numbered("r"), numbered("s"))
        .map_err(|e| format!("the trace: {e}"))?;
    let mut nogoods = create(&dir.join(format!("{id}.nogoods.tsv")))?;
    writeln!(nogoods, "{NOGOODS_HEADER}").map_err(|e| format!("the nogood log: {e}"))?;
    Ok(Files { steps, nogoods })
}

/// `anatomy` on game `n` at the set width `W`.
fn play<const W: usize>(n: u32, o: &Options, optimal: &Optimal) -> Result<(), String> {
    let params = &o.anatomy;
    let forgetting = o.tactics.forgetting();
    if forgetting.limit.is_some() || (forgetting.period.is_some() && forgetting.keep != Keep::Nothing) {
        return Err("anatomy: its mirror of the nogood table forgets everything, with no limit: the defaults of \
                    nogood_keep and nogood_limit"
            .to_owned());
    }
    let board = Board::<W>::for_n(n);
    let p = board.prepare(o.tactics)?;
    let units = units_of(n, o, &p)?;
    let (start, opt) =
        known_incumbent(&board, &p, optimal)?.ok_or(format!("n={n}: needs its known optimum (--known)"))?;
    let prefer = params.prefer.as_deref().map(read_order).transpose()?.unwrap_or_default();
    let game = o.images.learn.then(|| {
        let mut game = Game::new(&p, false, false);
        game.closure = o.images.closure;
        Arc::new(game)
    });
    let hooks = Hooks::new(Known::new(&p, &prefer), game);
    let make = hooks.maker();
    let graph = MatchGraph::new(&p.model, &p.root);
    let space = SearchSpace { hooks: Some(&make as &MakeHook<W>), ..SearchSpace::new(&p, &graph) };
    if let Some(dir) = &params.trace {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        write_candidates(&dir.join(format!("{n}.candidates.tsv")), &p, &hooks.known)?;
    }
    for (id, unit) in units {
        let files = params.trace.as_deref().map(|dir| open_files(dir, &id, p.root.frames().len())).transpose()?;
        let solved = solve(&space, &hooks, unit, &start, o.nodes, files)?;
        let t = &solved.tally;
        if let Some(dir) = &params.trace {
            write_conflicts(&dir.join(format!("{id}.conflicts.tsv")), &t.listed)?;
        }
        let mut line = Line::of(n);
        line.text("id", &id)
            .field("depth", unit.promoted.len())
            .field("cands", unit.cands.len())
            .field("need", (opt + 1).saturating_sub(unit.promoted.sum()))
            .field("nodes", solved.nodes)
            .field("steps", t.steps)
            .seconds("cpu_s", solved.cpu_s)
            .field("completed", solved.completed)
            .field("best", solved.best)
            .field("opt", opt)
            .maybe("root_value", t.root.map(|r| r.0))
            .maybe("root_chosen", t.root.map(|r| r.1.len()))
            .field("conflicts", t.listed.len())
            .list("residues", t.residues.iter().map(HashMap::len))
            .field("asked", t.asked)
            .field("overridden", t.overridden)
            .field("known_own", t.known[1])
            .field("known_image", t.known[2])
            .field("known_both", t.known[3])
            .field("known_unexplained", t.known[0])
            .field("nogoods_own", t.mirror.own)
            .field("nogoods_image", t.mirror.image)
            .text("tactics", &Tactics { search: o.tactics, ..Tactics::default() }.description())
            .maybe("prefer", params.prefer.as_ref().map(|path| format!("\"{}\"", path.display())));
        if o.images.learn {
            let learned = &solved.learned;
            line.text("learning", if o.images.closure { "closure" } else { "learn" })
                .field("computed", learned.conflicts)
                .field("proposed", learned.proposed)
                .field("learned", learned.learned);
        }
        if params.best_cpu > 0.0 {
            let completion = best_completion(&space, &unit, params.best_cpu, n);
            let mut scratch = Scratch::new();
            let state = p.model.promote_all_using(&mut scratch, &p.root, &unit.promoted).expect("the unit");
            let chosen = t.root.map(|r| r.1).unwrap_or_default();
            let list = |k: &Bits<W>| format!("[{}]", k.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","));
            let known = &hooks.known;
            let lf = |x: u32| format!("[{x},{},{}]", known.home[x as usize], known.live_factors(&state, x));
            line.field("best_completion", completion.sum())
                .field("completion", list(&completion))
                .field("chosen", list(&chosen))
                .list("chosen_lf", chosen.iter().map(lf))
                .list("cores", cores(&p.model, &state, &chosen).iter().map(list));
        }
        line.print();
    }
    Ok(())
}

/// `anatomy` on each game of `o`.
pub fn run(o: &Options, optimal: &Optimal) -> Result<(), String> {
    if o.images.closure && !o.images.learn {
        return Err("anatomy: --closure needs --learn".to_owned());
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

    /// A game whose root search takes a few hundred steps on one thread: big enough to branch, small enough to test.
    const N: u32 = 800;

    /// Solves `root` from `start` with the hooks `hooks` makes, in the game `p` (and its matching graph), told to none
    /// of the files.
    fn hooked<const W: usize>(
        p: &Prepared<W>,
        graph: &MatchGraph,
        hooks: &Hooks<W>,
        root: TaxUnit<W>,
        start: &Bits<W>,
    ) -> Solved<W> {
        let make = hooks.maker();
        let space = SearchSpace { hooks: Some(&make as &MakeHook<W>), ..SearchSpace::new(p, graph) };
        solve(&space, hooks, root, start, None, None).expect("solve")
    }

    fn check<const W: usize>() {
        let optimal = Optimal::built_in();
        let board = Board::<W>::for_n(N);
        let p = board.prepare(SearchTactics::default()).expect("prepare");
        let (start, _) = known_incumbent(&board, &p, optimal).expect("known").expect("a known game");
        let graph = MatchGraph::new(&p.model, &p.root);
        let root = TaxUnit { promoted: Bits::EMPTY, cands: p.candidates };
        let space = SearchSpace::new(&p, &graph);
        let plain = run_unit_with(&Control::new(), root, &start, None, |_| TaxSearcher::new(&space));
        // Told of every step, and the same tree as without the hook.
        let told = hooked(&p, &graph, &Hooks::new(Known::new(&p, &[]), None), root, &start);
        assert!(plain.completed && told.completed);
        assert_eq!((told.nodes, told.tally.steps, told.best), (plain.nodes, plain.nodes, plain.set.sum()));
        assert!(
            told.tally.asked > 0 && told.tally.overridden == 0,
            "asked {} of {} steps",
            told.tally.asked,
            told.nodes
        );
        // Another order changes the tree and not the best.
        let order: Vec<u32> = p.candidates.iter().collect(); // the smallest first: strong branching never is that
        let steered = hooked(&p, &graph, &Hooks::new(Known::new(&p, &order), None), root, &start);
        assert!(steered.completed && steered.tally.overridden > 0);
        assert_eq!(steered.best, plain.set.sum());
        assert_ne!(steered.nodes, plain.nodes);
    }

    #[test]
    fn the_hook_sees_every_step_changes_nothing_and_an_order_keeps_the_best() {
        taxman::with_width!(N, W => check::<W>(), else panic!("{}", beyond(N)));
    }

    /// A set is the integer of the indices of its members among the candidates, in hex, whatever the number of words.
    fn check_hex<const W: usize>() {
        let p = Board::<W>::for_n(N).prepare(SearchTactics::default()).expect("prepare");
        let known = Known::new(&p, &[]);
        let cands: Vec<u32> = p.candidates.iter().collect();
        assert!(cands.len() > 64, "{} candidates: not enough for a second word", cands.len());
        let set = |xs: &[u32]| xs.iter().copied().collect::<Bits<W>>();
        assert_eq!(known.hex(&Bits::EMPTY).as_deref(), Ok("0"));
        assert_eq!(known.hex(&set(&cands[..1])).as_deref(), Ok("1"));
        assert_eq!(known.hex(&set(&[cands[0], cands[3], cands[5]])).as_deref(), Ok("29"));
        assert_eq!(known.hex(&set(&[cands[63]])).as_deref(), Ok("8000000000000000"));
        assert_eq!(known.hex(&set(&[cands[64], cands[1]])).as_deref(), Ok("10000000000000002"));
        let other = (1..).find(|x| !p.candidates.contains(*x)).expect("a number that is no candidate");
        assert!(known.hex(&set(&[other])).is_err());
    }

    #[test]
    fn a_set_is_written_as_the_integer_of_the_indices_of_its_members() {
        taxman::with_width!(N, W => check_hex::<W>(), else panic!("{}", beyond(N)));
    }

    /// The nogood table of the searcher forgets every this many steps.
    const PERIOD: u64 = 500;

    /// Game 1215 searched from its known best set, its searcher forgetting its nogoods every `PERIOD` steps: the mirror
    /// explains every known conflict, and the searcher's nogoods are the mirror's, so that without the learning only
    /// own nogoods give any; with the learning of the closures, image nogoods give some, and the mirror took as many
    /// as the searcher learned.
    fn check_mirror<const W: usize>() {
        let n = 1215;
        let board = Board::<W>::for_n(n);
        let p = board.prepare(SearchTactics { nogood_period: PERIOD, ..SearchTactics::default() }).expect("prepare");
        let (start, _) = known_incumbent(&board, &p, Optimal::built_in()).expect("known").expect("a known game");
        let graph = MatchGraph::new(&p.model, &p.root);
        let root = TaxUnit { promoted: Bits::EMPTY, cands: p.candidates };
        let space = SearchSpace::new(&p, &graph);
        let plain = run_unit_with(&Control::new(), root, &start, None, |_| TaxSearcher::new(&space));
        let told = hooked(&p, &graph, &Hooks::new(Known::new(&p, &[]), None), root, &start);
        assert!(plain.completed && told.completed);
        assert_eq!((told.nodes, told.tally.steps, told.best), (plain.nodes, plain.nodes, plain.set.sum()));
        assert!(told.tally.steps > 2 * PERIOD, "{} steps", told.tally.steps);
        let [unexplained, own, image, both] = told.tally.known;
        assert_eq!((unexplained, image, both, told.tally.mirror.image), (0, 0, 0, 0));
        assert!(own > 0 && told.tally.mirror.own > 0);
        let mut game = Game::new(&p, false, false);
        game.closure = true;
        let hooks = Hooks::new(Known::new(&p, &[]), Some(Arc::new(game)));
        let learning = hooked(&p, &graph, &hooks, root, &start);
        assert!(learning.completed);
        assert_eq!(learning.best, plain.set.sum());
        let [unexplained, _, image, both] = learning.tally.known;
        assert_eq!(unexplained, 0);
        assert!(image + both > 0);
        assert_eq!(learning.tally.mirror.image, learning.learned.learned);
    }

    #[test]
    fn the_mirror_explains_every_known_conflict_with_and_without_the_learning() {
        taxman::with_width!(1215, W => check_mirror::<W>(), else panic!("{}", beyond(1215)));
    }
}
