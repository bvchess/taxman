//! The profile of a search (`taxman solve --profile DIR`): where its steps go, recorded while it runs, for research.
//! It observes the search and never changes it: the same game gives the same tree and the same answer with and
//! without it.  solver/docs/design/engine.md, "The profile of a search", explains the file and why a unit's place in
//! the tree can be found from its promotions alone; this comment says where each piece is.
//!
//! * The tree.  Every node of the search tree at depth `D` or less (depth: branchings from the root) has an id, given
//!   when the node is made ([`SearchProfile::tag`]), up to [`MOST_IDS`] a game; past them, or below depth `D`, a node's
//!   steps count to its nearest tagged ancestor.  The profile keeps of each tagged node only what places a unit: its
//!   depth, the promotions it added to its parent's, and its children in the order they were made ([`TreeNode`]).
//! * The searchers.  Each searcher keeps the ids of the nodes on its stack and counts its steps by id and by number of
//!   promotions ([`Tags`]), and hands its counts over when its run ends ([`SearchProfile::add`]).  A unit's node finds
//!   its id from its promotions ([`SearchProfile::locate`]), so the units, and the checkpoints, carry nothing new.
//! * The file.  At each checkpoint, before the engine writes it, the profile appends what it learned since the last
//!   one, as JSON lines, and syncs them ([`SearchProfile::flush`]): the nodes tagged, and a `steps` line, the steps by
//!   id and by number of promotions, which is the mark that a resume finds ([`Resumed`]).  After the checkpoint is
//!   written, a thread beside the search reads it and appends a summary of the open units ([`open_work`]).  The end of
//!   each run appends a last `steps` line and an `end` line ([`SearchProfile::finish`]).
//! * The phases.  Each searcher times one step in [`PHASE_EVERY`] phase by phase ([`Phases`], read by the searcher as
//!   each phase ends), and a `steps` line has the time of each phase in its interval.

use super::dual::NodeBound;
use super::setup::Prepared;
use super::TaxUnit;
use crate::bits::Bits;
use crate::engine::{self, CheckpointEvent, Unit};
use crate::gf2::Gf2Graph;
use crate::json::Json;
use crate::rng::Rng;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, PoisonError, RwLock};
use std::time::{Duration, Instant};

/// The depth of the tagged tree when the caller does not say (`--profile-depth`): the nodes at most this many
/// branchings below the root have ids of their own.  The top of a tree is a spine, each node making one or two children
/// of which one holds nearly all of the steps, down to depth 15 or 20 in the whole games 3500, 4050 and 4176 (0.6 to
/// 1.2 million steps on 8 threads, 2026-10-23); at depth 30 their steps are spread over hundreds of subtrees, the
/// largest ten holding 36 to 73% of them, with 4,468 to 13,220 ids in all, and the first ten minutes of game 4899 made
/// 2,493.
pub const DEPTH: u32 = 30;

/// The most ids a game's profile gives; past them, new nodes count to their nearest tagged ancestor, and the profile
/// says that it is full.  A node's line takes about 220 bytes, so a game's file holds at most 22 MB of them.
pub const MOST_IDS: u32 = 100_000;

/// The most open units of a checkpoint whose bound the summary of the open work computes ([`open_work`]): a sample, if
/// the checkpoint holds more.  A unit's bound takes about 20 ms at n = 4899.
pub const OPEN_BOUNDED: usize = 1_000;

/// With a profile, each searcher times one step in this many phase by phase ([`Phases`]).
pub const PHASE_EVERY: u32 = 64;

/// The format of the file, in its first line.
const VERSION: u64 = 1;

/// The phases of a step that the profile times, in the order of [`PHASE_NAMES`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    /// The bound of the node or of the child: the GF(2) bound's solve (and the child's level pushed), or the matching
    /// bound and its refinement on the matching path.
    Bound,
    /// The conflict of the heaviest set: the nogood table's lookup, and a peel where the table has none.
    Conflict,
    /// Conflict branching's choice of the conflict: the prices, the ordered conflicts and the look.
    Pricing,
    /// The Lagrangian bound: its check, and its refits after the step.
    Lagrange,
    /// The member to branch on: strong branching's solves.
    Branching,
    /// The child's promotion.
    Promotion,
    /// The child's candidates: narrowed, and ruled out by the known conflicts, the nogoods and the rule of symmetry.
    Children,
    /// The rest: the cheap checks, the rule of symmetry and the near-repeat cut, the offers, the bookkeeping.
    Other,
}

/// The names of the phases in a `steps` line, in the order of [`Phase`].
const PHASE_NAMES: [&str; 8] =
    ["bound", "conflict", "pricing", "lagrange", "branching", "promotion", "children", "other"];

/// A searcher's clock of the phases of its steps: it times one step in [`PHASE_EVERY`], reading the clock as each
/// phase ends ([`mark`](Phases::mark)), and the others cost it a test.  The times are those of the thread's clock on the
/// wall, the CPU's while the thread runs.
pub(super) struct Phases {
    countdown: u32,
    timing: bool,
    last: Instant,
    /// Nanoseconds by phase, and the steps timed, since the counts were last handed over.
    spent: [u64; 8],
    steps: u64,
}

impl Phases {
    pub(super) fn new() -> Self {
        Phases { countdown: PHASE_EVERY, timing: false, last: Instant::now(), spent: [0; 8], steps: 0 }
    }

    /// A step begins, timed if it is due.
    #[inline]
    pub(super) fn begin(&mut self) {
        self.countdown -= 1;
        if self.countdown == 0 {
            self.countdown = PHASE_EVERY;
            self.timing = true;
            self.last = Instant::now();
        }
    }

    /// The phase `phase` of the step ends here.
    #[inline]
    pub(super) fn mark(&mut self, phase: Phase) {
        if self.timing {
            self.lap(phase);
        }
    }

    /// The step ends: what is left of it is [`Phase::Other`].
    #[inline]
    pub(super) fn end(&mut self) {
        if self.timing {
            self.lap(Phase::Other);
            self.timing = false;
            self.steps += 1;
        }
    }

    #[cold]
    fn lap(&mut self, phase: Phase) {
        let now = Instant::now();
        self.spent[phase as usize] += u64::try_from(now.saturating_duration_since(self.last).as_nanos()).unwrap_or(0);
        self.last = now;
    }
}

/// The id of the steps of a unit that the profile cannot place in its tree: the units of a search that it joined after
/// the search began, without a profile of its start.
const UNPLACED: u32 = u32::MAX;

/// What a search's profile records, and where (`taxman solve --profile DIR`, `--profile-depth D`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfilePlan {
    /// The file, appended to (`nNNNN.profile.jsonl` in the directory of `solve`).
    pub file: PathBuf,
    /// The depth of the tagged tree, [`DEPTH`] by default.  A resumed profile keeps the depth it began with.
    pub depth: u32,
    /// The most ids, [`MOST_IDS`] by default.  A resumed profile keeps its own.
    pub most_ids: u32,
}

impl ProfilePlan {
    /// The plan of game `n`'s profile in the directory `dir`, at `depth`, with the default most ids.
    pub fn in_dir(dir: &Path, n: u32, depth: u32) -> Self {
        ProfilePlan { file: dir.join(format!("n{n:04}.profile.jsonl")), depth, most_ids: MOST_IDS }
    }
}

/// How the parent of a node chose the branch that made it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Branched {
    /// On the search's own conflict (conflict branching off, or the parent's gap below its gate), or with no conflict
    /// on the largest candidate.
    #[default]
    Default,
    /// On the conflict that conflict branching chose, the parent's gap at or above the gate (it may be the search's
    /// own conflict).
    Conflict,
    /// Promoting the parent's forced members together (tactic `fix_forced`): the parent's loop ends with this child.
    Forced,
}

impl Branched {
    fn name(self) -> &'static str {
        match self {
            Branched::Default => "default",
            Branched::Conflict => "conflict",
            Branched::Forced => "forced",
        }
    }
}

/// What a step that made a child knew of it, for its line: how its parent branched, and the child's own check when
/// it was made (the bound, the GF(2) bound of its completions with its check's limit on promotions, or the matching bound on
/// the matching path, and its need at the incumbent the step read).
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Made {
    pub(super) branched: Branched,
    pub(super) bound: u64,
    pub(super) need: u64,
}

/// A tagged node, as the profile keeps it to place units.
struct TreeNode {
    depth: u32,
    /// The promotions the node added to its parent's (its own, at the root: none).
    added: Box<[u32]>,
    /// Its tagged children, in the order they were made.
    children: Vec<u32>,
}

/// The tagged nodes, by id.
#[derive(Default)]
struct Tree {
    nodes: Vec<TreeNode>,
    /// The profile has its most ids: no node is tagged any more.
    full: bool,
    /// No node is tagged any more, because the tree lacks some that were (a resume whose profile is behind its
    /// checkpoint): a unit below such a node is placed at its nearest ancestor in the tree, which a new child of that
    /// ancestor could otherwise take from it.
    frozen: bool,
}

/// A tagged node's line, written at the next flush.
struct NodeLine {
    id: u32,
    parent: Option<u32>,
    depth: u32,
    child: Option<u32>,
    branched: Option<Branched>,
    added: Vec<u32>,
    promotions: u32,
    sum: u64,
    cands: u32,
    bound: Option<u64>,
    need: u64,
    /// The search's time when it was made, over every run.
    time: Duration,
}

impl NodeLine {
    fn json(&self) -> Json {
        let opt = |x: Option<u32>| x.map_or(Json::Null, Json::from);
        Json::object([
            ("kind", Json::from("node")),
            ("id", Json::from(self.id)),
            ("parent", opt(self.parent)),
            ("depth", Json::from(self.depth)),
            ("child", opt(self.child)),
            ("branch", self.branched.map_or(Json::Null, |b| Json::from(b.name()))),
            ("added", Json::array(self.added.iter().copied())),
            ("promotions", Json::from(self.promotions)),
            ("sum", Json::from(self.sum)),
            ("cands", Json::from(self.cands)),
            ("bound", self.bound.map_or(Json::Null, Json::from)),
            ("need", Json::from(self.need)),
            ("gap", self.bound.map_or(Json::Null, |bound| Json::from(bound as i64 - self.need as i64))),
            ("t", Json::Num(round3(self.time.as_secs_f64()))),
        ])
    }
}

/// What the searchers have counted and tagged since the last flush.
#[derive(Default)]
struct Pending {
    nodes: Vec<NodeLine>,
    steps: HashMap<u32, u64>,
    unplaced: u64,
    /// Steps by the number of promotions of the node that took them.
    promotions: Vec<u64>,
    /// The phases of the steps timed: nanoseconds by phase, and the steps.
    phases: [u64; 8],
    timed: u64,
}

/// The file, and what has been written to it.
struct Out {
    file: BufWriter<File>,
    /// The `steps` lines written, over every run.
    seq: u64,
    /// The steps written, over every run (ids, the unplaced ones, and the steps before the profile began).
    counted: u64,
    /// A write failed: the profile writes nothing more (the search goes on).
    failed: bool,
}

/// A checkpoint just written, open, for the summary of its open units.
pub(super) struct OpenJob {
    file: File,
    seq: u64,
}

/// The profile of one search: the tagged tree, what the searchers counted since the last flush, and the file.  The
/// searchers share it through their [`SearchSpace`](super::SearchSpace), and the engine's checkpoint hook writes it.
pub struct SearchProfile {
    depth: u32,
    most_ids: u32,
    /// Whether the tree begins at the root: false for a profile that joined a search after it began, whose units it
    /// cannot place.
    rooted: bool,
    tree: RwLock<Tree>,
    pending: Mutex<Pending>,
    out: Mutex<Out>,
    /// When this run began, and the time of the runs before it, for the time of a node.
    began: Instant,
    prior: Duration,
    /// The checkpoints for [`open_work`], until the search ends; and whether it has (the summary under way is dropped).
    jobs: Mutex<Option<Sender<OpenJob>>>,
    receiver: Mutex<Option<Receiver<OpenJob>>>,
    ending: AtomicBool,
    path: PathBuf,
}

/// What a resumed search's profile file holds, up to the `steps` line that matches its checkpoint.
struct Resumed {
    /// The bytes to keep: through that line, and the `open` lines right after it.
    keep: u64,
    depth: u32,
    most_ids: u32,
    tree: Tree,
    seq: u64,
    counted: u64,
    /// The steps of the checkpoint that the file does not have (it is behind it), counted as unplaced.
    lost: u64,
}

/// A `steps` line of a profile file being read, and what the file holds up to it.
#[derive(Clone, Copy)]
struct Mark {
    /// The line's index.
    line: usize,
    nodes: u64,
    made: u64,
    seq: u64,
    counted: u64,
    full: bool,
    frozen: bool,
}

/// How the profile of a search starts: a fresh search, or one resumed from a checkpoint of `nodes` steps.
pub(super) enum Begin {
    Fresh {
        /// The root's bound ([`NodeBound`], with the tactic `gf2`), and its need at the starting incumbent.
        bound: Option<u64>,
        need: u64,
        candidates: u32,
    },
    Resumed {
        nodes: u64,
        elapsed: Duration,
    },
}

impl SearchProfile {
    /// The profile of the search of `prepared` that `plan` asks for, as `begin` says, with its file opened and its
    /// first lines written (`threads` and `incumbent`, the search's, go in its `start` line).
    ///
    /// # Errors
    ///
    /// The file cannot be read or written.
    pub(super) fn open<const W: usize>(
        plan: &ProfilePlan,
        prepared: &Prepared<W>,
        begin: &Begin,
        threads: usize,
        incumbent: u64,
    ) -> Result<SearchProfile, String> {
        let at = |e: io::Error| format!("{}: {e}", plan.file.display());
        if let Some(dir) = plan.file.parent().filter(|dir| !dir.as_os_str().is_empty()) {
            fs::create_dir_all(dir).map_err(at)?;
        }
        let (resumed, prior) = match begin {
            Begin::Fresh { .. } => (None, Duration::ZERO),
            Begin::Resumed { nodes, elapsed } => (read_resumed(&plan.file, prepared.n, W, *nodes), *elapsed),
        };
        // What this run starts with that the steps it counts will not show: the steps of a search that the profile
        // joins after it began, or those of the checkpoint that a profile behind it does not have.
        let unseen = match (&resumed, begin) {
            (Some(resumed), _) => resumed.lost,
            (None, Begin::Resumed { nodes, .. }) => *nodes,
            (None, Begin::Fresh { .. }) => 0,
        };
        // A file that this search does not go on with is set aside, not lost: an earlier search of the game, which a
        // restart or a rerun replaces.
        if resumed.is_none() {
            set_aside(&plan.file).map_err(at)?;
        }
        let mut file =
            OpenOptions::new().create(true).read(true).write(true).truncate(false).open(&plan.file).map_err(at)?;
        file.set_len(resumed.as_ref().map_or(0, |r| r.keep)).map_err(at)?;
        io::Seek::seek(&mut file, io::SeekFrom::End(0)).map_err(at)?;
        let mut header = Vec::new();
        let (depth, most_ids, tree, seq, counted, lost) = match resumed {
            Some(r) => (r.depth, r.most_ids, r.tree, r.seq, r.counted, r.lost),
            None => {
                header.push(Json::object([
                    ("kind", Json::from("profile")),
                    ("version", Json::from(VERSION)),
                    ("n", Json::from(prepared.n)),
                    ("width", Json::from(W)),
                    ("depth", Json::from(plan.depth)),
                    ("most_ids", Json::from(plan.most_ids)),
                ]));
                let tree = match begin {
                    Begin::Fresh { .. } => Tree {
                        nodes: vec![TreeNode { depth: 0, added: Box::new([]), children: Vec::new() }],
                        ..Tree::default()
                    },
                    Begin::Resumed { .. } => Tree::default(),
                };
                (plan.depth, plan.most_ids, tree, 0, 0, 0)
            }
        };
        let rooted = !tree.nodes.is_empty();
        let tree = Tree { frozen: tree.frozen || lost > 0, ..tree };
        let (resumed, nodes_before) = match begin {
            Begin::Fresh { .. } => (false, 0),
            Begin::Resumed { nodes, .. } => (true, *nodes),
        };
        header.push(Json::object([
            ("kind", Json::from("start")),
            ("resumed", Json::from(resumed)),
            ("rooted", Json::from(rooted)),
            ("nodes", Json::from(nodes_before)),
            ("elapsed", Json::Num(round3(prior.as_secs_f64()))),
            ("threads", Json::from(threads)),
            ("incumbent", Json::from(incumbent)),
            ("unplaced", Json::from(unseen)),
            ("frozen", Json::from(tree.frozen)),
        ]));
        let mut pending = Pending::default();
        if let Begin::Fresh { bound, need, candidates } = begin {
            pending.nodes.push(NodeLine {
                id: 0,
                parent: None,
                depth: 0,
                child: None,
                branched: None,
                added: Vec::new(),
                promotions: 0,
                sum: 0,
                cands: *candidates,
                bound: *bound,
                need: *need,
                time: Duration::ZERO,
            });
        }
        let mut out = BufWriter::new(file);
        for line in &header {
            writeln!(out, "{line}").map_err(at)?;
        }
        out.flush().map_err(at)?;
        let (sender, receiver) = mpsc::channel();
        Ok(SearchProfile {
            depth,
            most_ids,
            rooted,
            tree: RwLock::new(tree),
            pending: Mutex::new(pending),
            out: Mutex::new(Out { file: out, seq, counted: counted + unseen, failed: false }),
            began: Instant::now(),
            prior,
            jobs: Mutex::new(Some(sender)),
            receiver: Mutex::new(Some(receiver)),
            ending: AtomicBool::new(false),
            path: plan.file.clone(),
        })
    }

    /// The place of the node whose promotions are `promoted` in the tagged tree: the id its steps count to, its depth
    /// if it is a tagged node, and whether it tags its children.
    ///
    /// From the root, it goes to the first child (in the order they were made) whose promotions are among `promoted`,
    /// as long as there is one.  A node of the subtree of a child c of a node v promotes c's own member, which v left
    /// out of its candidates before it made any later child, so no node of a later child's subtree holds it; nor does
    /// v itself.  So the first child whose promotions the node holds is the one whose subtree holds it.  The node is
    /// the tagged node reached when it has no more promotions than that node.
    pub(super) fn locate<const W: usize>(&self, promoted: &Bits<W>) -> Level {
        let promotions = promoted.len();
        if !self.rooted {
            return Level { id: UNPLACED, depth: None, tags: false, promotions };
        }
        let tree = self.tree.read().unwrap_or_else(PoisonError::into_inner);
        let (mut id, mut held) = (0u32, 0u32);
        'walk: loop {
            for &c in &tree.nodes[id as usize].children {
                let added = &tree.nodes[c as usize].added;
                if added.iter().all(|&x| promoted.contains(x)) {
                    id = c;
                    held += added.len() as u32;
                    continue 'walk;
                }
            }
            break;
        }
        let node = &tree.nodes[id as usize];
        let own = held == promotions;
        let tags = own && node.depth < self.depth && !tree.full && !tree.frozen;
        Level { id, depth: own.then_some(node.depth), tags, promotions }
    }

    /// Tags a child of the tagged node `parent`, at `depth`, that added `added` to its parent's promotions, unless the
    /// profile is full: its id, and its line for the next flush.
    #[allow(clippy::too_many_arguments)]
    fn tag(
        &self,
        parent: u32,
        depth: u32,
        added: Vec<u32>,
        promotions: u32,
        sum: u64,
        cands: u32,
        made: &Made,
    ) -> Option<u32> {
        let mut tree = self.tree.write().unwrap_or_else(PoisonError::into_inner);
        if tree.frozen || tree.full {
            return None;
        }
        if tree.nodes.len() >= self.most_ids as usize {
            tree.full = true;
            return None;
        }
        let id = tree.nodes.len() as u32;
        let child = tree.nodes[parent as usize].children.len() as u32;
        tree.nodes[parent as usize].children.push(id);
        tree.nodes.push(TreeNode { depth, added: added.clone().into_boxed_slice(), children: Vec::new() });
        drop(tree);
        let line = NodeLine {
            id,
            parent: Some(parent),
            depth,
            child: Some(child),
            branched: Some(made.branched),
            added,
            promotions,
            sum,
            cands,
            bound: Some(made.bound),
            need: made.need,
            time: self.prior + self.began.elapsed(),
        };
        lock(&self.pending).nodes.push(line);
        Some(id)
    }

    /// Adds a searcher's counts, and empties them: its steps by id and by number of promotions, and the phases of the
    /// steps it timed.
    fn add(&self, steps: &mut HashMap<u32, u64>, promotions: &mut [u64], phases: Option<&mut Phases>) {
        let mut pending = lock(&self.pending);
        if let Some(phases) = phases {
            for (total, spent) in pending.phases.iter_mut().zip(&mut phases.spent) {
                *total += std::mem::take(spent);
            }
            pending.timed += std::mem::take(&mut phases.steps);
        }
        for (id, count) in steps.drain() {
            if id == UNPLACED {
                pending.unplaced += count;
            } else {
                *pending.steps.entry(id).or_default() += count;
            }
        }
        if pending.promotions.len() < promotions.len() {
            pending.promotions.resize(promotions.len(), 0);
        }
        for (total, count) in pending.promotions.iter_mut().zip(promotions.iter_mut()) {
            *total += std::mem::take(count);
        }
    }

    /// What the engine's hook tells the profile ([`CheckpointEvent`]): before a checkpoint is written, the profile
    /// writes what it has and syncs it; after, it hands the checkpoint to the summary of the open work.
    pub(super) fn checkpoint(&self, event: CheckpointEvent<'_>) {
        match event {
            CheckpointEvent::Writing { nodes, elapsed } => self.flush(nodes, elapsed),
            CheckpointEvent::Written { path, .. } => {
                let seq = lock(&self.out).seq;
                if let (Some(jobs), Ok(file)) = (lock(&self.jobs).as_ref(), File::open(path)) {
                    let _ = jobs.send(OpenJob { file, seq });
                }
            }
        }
    }

    /// Writes the nodes tagged since the last flush and a `steps` line, at the search's `nodes` steps and `elapsed`
    /// time, and syncs them.  Every searcher has handed its counts over (the workers are parked, or the run is over),
    /// so what the file counts is `nodes`.
    fn flush(&self, nodes: u64, elapsed: Duration) {
        let mut pending = std::mem::take(&mut *lock(&self.pending));
        let (made, full) = {
            let tree = self.tree.read().unwrap_or_else(PoisonError::into_inner);
            (tree.nodes.len(), tree.full)
        };
        let mut out = lock(&self.out);
        if out.failed {
            return;
        }
        pending.nodes.sort_unstable_by_key(|line| line.id);
        let mut steps: Vec<(u32, u64)> = pending.steps.into_iter().collect();
        steps.sort_unstable();
        out.counted += steps.iter().map(|&(_, count)| count).sum::<u64>() + pending.unplaced;
        debug_assert_eq!(out.counted, nodes, "the profile counts the steps of the checkpoint");
        out.seq += 1;
        let line = Json::object([
            ("kind", Json::from("steps")),
            ("seq", Json::from(out.seq)),
            ("nodes", Json::from(nodes)),
            ("counted", Json::from(out.counted)),
            ("elapsed", Json::Num(round3(elapsed.as_secs_f64()))),
            ("ids", Json::array(steps.iter().map(|&(id, count)| Json::array([u64::from(id), count])))),
            ("unplaced", Json::from(pending.unplaced)),
            (
                "promotions",
                Json::array(
                    pending
                        .promotions
                        .iter()
                        .enumerate()
                        .filter(|&(_, &count)| count > 0)
                        .map(|(k, &count)| Json::array([k as u64, count])),
                ),
            ),
            ("made", Json::from(made)),
            ("full", Json::from(full)),
            (
                "phases",
                Json::object(
                    [("timed", pending.timed)]
                        .into_iter()
                        .chain(PHASE_NAMES.into_iter().zip(pending.phases))
                        .map(|(name, x)| (name, Json::from(x))),
                ),
            ),
        ]);
        let written = (|| -> io::Result<()> {
            for node in &pending.nodes {
                writeln!(out.file, "{}", node.json())?;
            }
            writeln!(out.file, "{line}")?;
            out.file.flush()?;
            out.file.get_ref().sync_data()
        })();
        if let Err(e) = written {
            self.fail(&mut out, &e);
        }
    }

    /// Appends `line` (an `open` or `end` line) and flushes it.
    fn append(&self, line: &Json) {
        let mut out = lock(&self.out);
        if out.failed {
            return;
        }
        if let Err(e) = writeln!(out.file, "{line}").and_then(|()| out.file.flush()) {
            self.fail(&mut out, &e);
        }
    }

    /// A write failed: the profile says so once, and writes nothing more; the search goes on.
    fn fail(&self, out: &mut Out, e: &io::Error) {
        out.failed = true;
        eprintln!("warning: the profile {} stops: {e}", self.path.display());
    }

    /// The end of the search's run: the summary of the open work under way is dropped, the last counts are written with
    /// the run's `nodes`, and an `end` line says how the run ended (`completed`, the best promotion sum `best`, the
    /// search's time `elapsed`; `error` if the engine failed, when the nodes are what the profile counted).
    pub(super) fn finish(
        &self,
        nodes: Option<u64>,
        completed: bool,
        best: u64,
        elapsed: Duration,
        error: Option<&str>,
    ) {
        self.end();
        let nodes = nodes.unwrap_or_else(|| {
            let pending = lock(&self.pending);
            lock(&self.out).counted + pending.steps.values().sum::<u64>() + pending.unplaced
        });
        self.flush(nodes, elapsed);
        let (made, full, frozen) = {
            let tree = self.tree.read().unwrap_or_else(PoisonError::into_inner);
            (tree.nodes.len(), tree.full, tree.frozen)
        };
        self.append(&Json::object([
            ("kind", Json::from("end")),
            ("nodes", Json::from(nodes)),
            ("completed", Json::from(completed)),
            ("best", Json::from(best)),
            ("elapsed", Json::Num(round3(elapsed.as_secs_f64()))),
            ("made", Json::from(made)),
            ("full", Json::from(full)),
            ("frozen", Json::from(frozen)),
            ("error", error.map_or(Json::Null, Json::from)),
        ]));
    }

    /// The checkpoints for [`open_work`], taken once by the thread that summarizes them.
    pub(super) fn jobs(&self) -> Option<Receiver<OpenJob>> {
        lock(&self.receiver).take()
    }

    /// Ends [`open_work`]: the summary under way is dropped, and no checkpoint follows.
    fn end(&self) {
        self.ending.store(true, Ordering::Relaxed);
        lock(&self.jobs).take();
    }
}

/// Ends [`open_work`] when dropped ([`SearchProfile::end`]), so that a thread that waits for it to end returns when
/// the search ends, a panic unwinding included.
pub(super) struct Ending<'a>(pub(super) &'a SearchProfile);

impl Drop for Ending<'_> {
    fn drop(&mut self) {
        self.0.end();
    }
}

/// Where a node on a searcher's stack stands in the profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Level {
    /// The id its steps count to: its own if it is tagged, else its nearest tagged ancestor's, or [`UNPLACED`].
    id: u32,
    /// Its depth, if known (always for a tagged node).
    depth: Option<u32>,
    /// Whether it is tagged and above the depth limit, so that its children are tagged.
    tags: bool,
    promotions: u32,
}

/// A searcher's side of the profile: where the nodes on its stack stand, level for level, and what it has counted since
/// it last handed its counts over.
pub(super) struct Tags<'a> {
    profile: &'a SearchProfile,
    levels: Vec<Level>,
    /// The id the last steps counted to, and how many of them are not in `steps` yet.
    current: u32,
    run: u64,
    steps: HashMap<u32, u64>,
    promotions: Vec<u64>,
}

impl<'a> Tags<'a> {
    pub(super) fn new(profile: &'a SearchProfile) -> Self {
        Tags { profile, levels: Vec::new(), current: UNPLACED, run: 0, steps: HashMap::new(), promotions: Vec::new() }
    }

    /// The unit whose node has the promotions `promoted` starts: its place, at level 0.
    pub(super) fn start<const W: usize>(&mut self, promoted: &Bits<W>) {
        self.levels.clear();
        self.levels.push(self.profile.locate(promoted));
    }

    /// A step of the node at `depth` on the stack.
    #[inline]
    pub(super) fn step(&mut self, depth: usize) {
        let level = self.levels[depth];
        if level.id == self.current {
            self.run += 1;
        } else {
            if self.run > 0 {
                *self.steps.entry(self.current).or_default() += self.run;
            }
            (self.current, self.run) = (level.id, 1);
        }
        let k = level.promotions as usize;
        if k >= self.promotions.len() {
            self.promotions.resize(k + 1, 0);
        }
        self.promotions[k] += 1;
    }

    /// The node at `depth`, with the promotions `parent`, made a child with the promotions `child` (worth `sum`, with
    /// `cands` candidates), which goes on the stack at `depth + 1`: tagged if its parent tags its children.
    pub(super) fn child<const W: usize>(
        &mut self,
        depth: usize,
        parent: &Bits<W>,
        child: &Bits<W>,
        sum: u64,
        cands: u32,
        made: &Made,
    ) {
        let up = self.levels[depth];
        let promotions = child.len();
        let below = up.depth.map(|d| d + 1);
        let mut level = Level { id: up.id, depth: below, tags: false, promotions };
        if let (true, Some(d)) = (up.tags, below) {
            let added = (*child - *parent).iter().collect();
            if let Some(id) = self.profile.tag(up.id, d, added, promotions, sum, cands, made) {
                (level.id, level.tags) = (id, d < self.profile.depth);
            }
        }
        self.levels.truncate(depth + 1);
        self.levels.push(level);
    }

    /// Hands the counts over to the profile (the end of a run), with the phases of the steps timed.
    pub(super) fn flush(&mut self, phases: Option<&mut Phases>) {
        if self.run > 0 {
            *self.steps.entry(self.current).or_default() += self.run;
            self.run = 0;
        }
        self.profile.add(&mut self.steps, &mut self.promotions, phases);
    }

    /// Whether the node at `depth`, with the promotions `promoted`, is where the profile places it: what a unit made
    /// of it would be counted to (checked in debug builds as a searcher yields).
    pub(super) fn placed<const W: usize>(&self, depth: usize, promoted: &Bits<W>) -> bool {
        self.profile.locate(promoted).id == self.levels[depth].id
    }
}

/// Summarizes the open units of each checkpoint that `profile` hands over (the search of `prepared`, whose root's GF(2)
/// graph is `graph` with the tactic `gf2`), until the search ends: an `open` line with their number by id, and the
/// spread of the gaps of at most [`OPEN_BOUNDED`] of them, a uniform sample of them if there are more.  A unit's gap is
/// its bound ([`NodeBound`]) less its need at the checkpoint's incumbent, as in the units file of `examples/tune.rs`.
/// With `background` the thread takes the idle scheduling policy, as the workers do.  The end of the search drops the
/// summary under way.
pub(super) fn open_work<const W: usize>(
    profile: &SearchProfile,
    prepared: &Prepared<W>,
    graph: Option<&Gf2Graph>,
    jobs: Receiver<OpenJob>,
    background: bool,
) {
    if background {
        engine::background_priority();
    }
    let mut bound = graph.map(|graph| NodeBound::new(prepared, graph));
    for job in jobs {
        if profile.ending.load(Ordering::Relaxed) {
            return;
        }
        let began = Instant::now();
        let mut by_id: HashMap<u32, (u64, u64, Option<i64>)> = HashMap::new();
        let mut sample: Vec<(u32, TaxUnit<W>)> = Vec::new();
        let (mut units, mut rng) = (0u64, Rng(job.seq));
        let scanned = engine::scan_checkpoint_file(job.file, |bytes| {
            let unit = TaxUnit::<W>::decode(bytes);
            let id = profile.locate(&unit.promoted).id;
            by_id.entry(id).or_default().0 += 1;
            units += 1;
            // Algorithm R: each unit is in the sample with the same chance.
            if sample.len() < OPEN_BOUNDED {
                sample.push((id, unit));
            } else if let Some(slot) = sample.get_mut(rng.below(units as usize)) {
                *slot = (id, unit);
            }
            Ok(())
        });
        let Ok(head) = scanned else { continue };
        let mut gaps = Vec::new();
        if let Some(bound) = &mut bound {
            for (id, unit) in &sample {
                if profile.ending.load(Ordering::Relaxed) {
                    return;
                }
                let Some(add) = bound.completions(unit) else { continue };
                let gap = add as i64 - (head.best + 1) as i64 + unit.promoted.sum() as i64;
                let entry = by_id.entry(*id).or_default();
                entry.1 += 1;
                entry.2 = Some(entry.2.map_or(gap, |max| max.max(gap)));
                gaps.push(gap);
            }
        }
        if profile.ending.load(Ordering::Relaxed) {
            return;
        }
        gaps.sort_unstable();
        let quantile = |q: f64| gaps[((gaps.len() - 1) as f64 * q).round() as usize];
        let spread =
            if gaps.is_empty() { Json::Null } else { Json::array([0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0].map(quantile)) };
        let mut ids: Vec<_> = by_id.into_iter().collect();
        ids.sort_unstable_by_key(|&(id, _)| id);
        let row = |id: u32, (count, bounded, max): (u64, u64, Option<i64>)| {
            let id = if id == UNPLACED { Json::Null } else { Json::from(id) };
            Json::array([id, Json::from(count), Json::from(bounded), max.map_or(Json::Null, Json::from)])
        };
        profile.append(&Json::object([
            ("kind", Json::from("open")),
            ("seq", Json::from(job.seq)),
            ("nodes", Json::from(head.stats.nodes)),
            ("incumbent", Json::from(head.best)),
            ("units", Json::from(units)),
            ("bounded", Json::from(gaps.len())),
            ("gaps", spread),
            ("ids", Json::array(ids.into_iter().map(|(id, counts)| row(id, counts)))),
            ("seconds", Json::Num(round3(began.elapsed().as_secs_f64()))),
        ]));
    }
}

/// Reads the profile `path` of game `n` at width `width` for a search resumed from a checkpoint of `nodes` steps: up to
/// its last `steps` line with those nodes, or failing that the last with fewer (the profile is then behind, by the
/// steps it lacks).  `None` if there is no such file, or it is another game's or another format's, or it is ahead of
/// the checkpoint all the way: a new profile then starts, as one that joined the search after it began.
fn read_resumed(path: &Path, n: u32, width: usize, nodes: u64) -> Option<Resumed> {
    let text = fs::read(path).ok()?;
    let mut lines = Vec::new();
    let mut at = 0usize;
    // Complete lines only: a line cut short by a crash is the end of what can be read.
    while let Some(len) = text[at..].iter().position(|&b| b == b'\n') {
        let Ok(json) =
            std::str::from_utf8(&text[at..at + len]).map_err(|_| ()).and_then(|s| Json::parse(s).map_err(drop))
        else {
            break;
        };
        at += len + 1;
        lines.push((json, at as u64));
    }
    let (first, _) = lines.first()?;
    let field = |json: &Json, key: &str| json.get(key).and_then(Json::as_u64);
    let kind = |json: &Json| json.get("kind").and_then(Json::as_str).map(str::to_owned);
    if kind(first).as_deref() != Some("profile")
        || field(first, "version") != Some(VERSION)
        || field(first, "n") != Some(u64::from(n))
        || field(first, "width") != Some(width as u64)
    {
        return None;
    }
    let depth = u32::try_from(field(first, "depth")?).ok()?;
    let most_ids = u32::try_from(field(first, "most_ids")?).ok()?;
    let flag = |json: &Json, key: &str| json.get(key).and_then(Json::as_bool).unwrap_or(false);
    let mut marks: Vec<Mark> = Vec::new();
    let (mut counted, mut frozen) = (0u64, false);
    for (line, (json, _)) in lines.iter().enumerate() {
        match kind(json).as_deref() {
            Some("start") => {
                counted += field(json, "unplaced").unwrap_or(0);
                frozen |= flag(json, "frozen");
            }
            Some("steps") => {
                let ids = json.get("ids").and_then(Json::as_arr).unwrap_or_default();
                counted += ids.iter().filter_map(|pair| pair.at(1).and_then(Json::as_u64)).sum::<u64>();
                counted += field(json, "unplaced").unwrap_or(0);
                marks.push(Mark {
                    line,
                    nodes: field(json, "nodes")?,
                    made: field(json, "made")?,
                    seq: field(json, "seq")?,
                    counted,
                    full: flag(json, "full"),
                    frozen,
                });
            }
            _ => {}
        }
    }
    let exact = marks.iter().rev().find(|mark| mark.nodes == nodes);
    let mark = *exact.or_else(|| marks.iter().rev().find(|mark| mark.nodes < nodes))?;
    let Mark { line, made, .. } = mark;
    // The `open` lines right after the mark belong with it.
    let mut last = line;
    while lines.get(last + 1).is_some_and(|(json, _)| kind(json).as_deref() == Some("open")) {
        last += 1;
    }
    let mut tree = Tree { full: mark.full, frozen: mark.frozen, nodes: Vec::new() };
    for (json, _) in &lines[..line] {
        if kind(json).as_deref() != Some("node") {
            continue;
        }
        let id = field(json, "id")?;
        if id >= made {
            continue;
        }
        if id != tree.nodes.len() as u64 {
            return None;
        }
        let added = json.get("added").and_then(Json::as_u32_vec)?;
        let depth = u32::try_from(field(json, "depth")?).ok()?;
        if let Some(parent) = json.get("parent").and_then(Json::as_u64) {
            tree.nodes.get_mut(usize::try_from(parent).ok()?)?.children.push(id as u32);
        }
        tree.nodes.push(TreeNode { depth, added: added.into_boxed_slice(), children: Vec::new() });
    }
    if tree.nodes.len() as u64 != made {
        return None;
    }
    Some(Resumed {
        keep: lines[last].1,
        depth,
        most_ids,
        tree,
        seq: mark.seq,
        counted: mark.counted,
        lost: nodes - mark.nodes,
    })
}

/// Renames the file `path`, if it has anything in it, to `path.K`, K the first number not taken.
fn set_aside(path: &Path) -> io::Result<()> {
    if fs::metadata(path).map_or(true, |meta| meta.len() == 0) {
        return Ok(());
    }
    let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let free = (1u32..).map(|k| path.with_file_name(format!("{name}.{k}"))).find(|to| !to.exists());
    fs::rename(path, free.expect("a free name"))
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `x` to three decimals, for times in seconds.
fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::engine::EngineConfig;
    use crate::search::{search_steered, BranchSettings, Branching, SearchResult, SearchTactics};
    use crate::test_support::TempDir;
    use std::sync::{Arc, OnceLock};

    const W: usize = 16;

    /// Game `n`: 800, whose search takes about 1,400 steps, or 1020, about 8,900, which leaves the engine's node limit
    /// time to stop it after its first check (at 1,024 steps).
    fn game(n: u32) -> Prepared<W> {
        Board::<W>::for_n(n).prepare(SearchTactics::default()).expect("the setup")
    }

    /// The search of `p` on `threads` threads from its start, or resumed (`resume`) from the checkpoint `checkpoint`,
    /// written every `every`; stopped after `limit` steps if given; with the profile `plan` if given.
    fn searched(
        p: &Prepared<W>,
        threads: usize,
        checkpoint: Option<(&Path, Duration)>,
        limit: Option<u64>,
        resume: bool,
        plan: Option<&ProfilePlan>,
    ) -> SearchResult<W> {
        let cfg = EngineConfig {
            threads,
            progress: None,
            node_limit: limit,
            checkpoint: checkpoint.map(|(path, every)| (path.to_path_buf(), every)),
            ..EngineConfig::default()
        };
        let branching = Arc::new(Branching::new(BranchSettings::of(&p.tactics)));
        search_steered(p, &cfg, &[], None, resume, None, &branching, &OnceLock::new(), plan).expect("the search")
    }

    /// The lines of the profile `path`.
    fn lines(path: &Path) -> Vec<Json> {
        let text = fs::read_to_string(path).expect("the profile");
        text.lines().map(|line| Json::parse(line).expect("a line of JSON")).collect()
    }

    fn of_kind<'a>(lines: &'a [Json], kind: &'a str) -> impl Iterator<Item = &'a Json> + 'a {
        lines.iter().filter(move |line| line.get("kind").and_then(Json::as_str) == Some(kind))
    }

    fn number(line: &Json, key: &str) -> u64 {
        line.get(key).and_then(Json::as_u64).unwrap_or_else(|| panic!("{key} in {line}"))
    }

    /// The steps a profile counted to ids and as unplaced, over all its lines.
    fn counted(lines: &[Json]) -> (u64, u64) {
        let pairs = of_kind(lines, "steps").flat_map(|line| line.get("ids").and_then(Json::as_arr).unwrap_or_default());
        let placed = pairs.map(|pair| pair.at(1).and_then(Json::as_u64).expect("a count")).sum();
        let unplaced =
            of_kind(lines, "steps").chain(of_kind(lines, "start")).map(|line| number(line, "unplaced")).sum();
        (placed, unplaced)
    }

    /// Every steps line counts the steps of the search so far, and they add up to the search's `nodes`: every step is
    /// counted once, by id or as unplaced, and by number of promotions.
    fn counts_every_step_once(lines: &[Json], nodes: u64) {
        for line in of_kind(lines, "steps") {
            assert_eq!(number(line, "counted"), number(line, "nodes"), "{line}");
        }
        let (placed, unplaced) = counted(lines);
        assert_eq!(placed + unplaced, nodes, "the steps counted, placed and not");
        let by_promotions: u64 = of_kind(lines, "steps")
            .flat_map(|line| line.get("promotions").and_then(Json::as_arr).unwrap_or_default())
            .map(|pair| pair.at(1).and_then(Json::as_u64).expect("a count"))
            .sum();
        let unseen: u64 = of_kind(lines, "start").map(|line| number(line, "unplaced")).sum();
        assert_eq!(by_promotions, nodes - unseen, "the steps by number of promotions");
        let end = of_kind(lines, "end").last().expect("an end line");
        assert_eq!(number(end, "nodes"), nodes, "{end}");
    }

    /// The node lines name each node once, in order, each one branching below its parent, at most `depth` deep, the
    /// root first.  Returns their depths.
    fn the_tree_is_well_formed(lines: &[Json], depth: u64) -> Vec<u64> {
        let mut depths: Vec<u64> = Vec::new();
        for (id, line) in of_kind(lines, "node").enumerate() {
            assert_eq!(number(line, "id"), id as u64, "{line}");
            let d = number(line, "depth");
            assert!(d <= depth, "{line}");
            match line.get("parent").and_then(Json::as_u64) {
                Some(parent) => assert_eq!(depths[parent as usize] + 1, d, "{line}"),
                None => assert_eq!((id, d), (0, 0), "{line}"),
            }
            depths.push(d);
        }
        depths
    }

    /// One thread: the profile changes nothing of the tree (the same steps and counters, the same answer), counts every
    /// step once, and tags the nodes down to its depth, each with what its check found.
    #[test]
    fn a_profile_changes_nothing_and_counts_every_step_once() {
        let p = game(800);
        let dir = TempDir::new("profile-same");
        let without = searched(&p, 1, None, None, false, None);
        let plan = ProfilePlan { file: dir.join("p.jsonl"), depth: 3, most_ids: MOST_IDS };
        let with = searched(&p, 1, None, None, false, Some(&plan));
        assert!(without.completed && with.completed);
        assert_eq!((with.sum, with.promotions, with.stats), (without.sum, without.promotions, without.stats));
        let lines = lines(&plan.file);
        counts_every_step_once(&lines, with.stats.nodes);
        assert_eq!(counted(&lines).1, 0, "a profile from the root places every step");
        let depths = the_tree_is_well_formed(&lines, 3);
        assert!(depths.contains(&3), "nodes are tagged down to the depth: {depths:?}");
        for line in of_kind(&lines, "node") {
            let (bound, need) = (number(line, "bound"), number(line, "need"));
            assert!(bound >= need, "a node is pushed only if its bound reaches its need: {line}");
        }
        // The one searcher timed one step in `PHASE_EVERY`, each in some phases.
        let phases = |key: &str| -> u64 {
            of_kind(&lines, "steps").filter_map(|line| line.get("phases")?.get(key)?.as_u64()).sum()
        };
        assert_eq!(phases("timed"), with.stats.nodes / u64::from(PHASE_EVERY));
        assert!(phases("bound") > 0 && PHASE_NAMES.iter().map(|name| phases(name)).sum::<u64>() > 0);
    }

    /// Several threads, a checkpoint every millisecond: every steps line counts the steps of its checkpoint, every step
    /// is counted once, and every unit a searcher hands over is placed where its node stands (checked as it yields, in
    /// debug builds).  A small limit on the ids stops the tagging, and the profile says it is full.
    #[test]
    fn a_profile_counts_every_step_once_on_many_threads() {
        let p = game(800);
        let dir = TempDir::new("profile-threads");
        let checkpoint = dir.join("ckpt");
        for most_ids in [MOST_IDS, 12] {
            let plan = ProfilePlan { file: dir.join(format!("p{most_ids}.jsonl")), depth: DEPTH, most_ids };
            let r = searched(&p, 4, Some((&checkpoint, Duration::from_millis(1))), None, false, Some(&plan));
            assert!(r.completed);
            let lines = lines(&plan.file);
            counts_every_step_once(&lines, r.stats.nodes);
            assert_eq!(counted(&lines).1, 0);
            let made = the_tree_is_well_formed(&lines, u64::from(DEPTH)).len() as u64;
            let end = of_kind(&lines, "end").last().expect("an end line");
            assert_eq!(number(end, "made"), made);
            assert_eq!(end.get("full").and_then(Json::as_bool), Some(made == u64::from(most_ids)), "{end}");
        }
    }

    /// A checkpoint interval longer than any test search.
    const HOUR: Duration = Duration::from_secs(3600);

    /// The search of `p` on one thread with the profile `plan`, from its start or resumed (`resume`), checkpointed in
    /// `checkpoint` every `every` and stopped by `limits`, a node limit and a time limit, of which a time limit of zero
    /// stops it before its first step: its steps.
    fn limited(
        p: &Prepared<W>,
        checkpoint: &Path,
        every: Duration,
        limits: (Option<u64>, Option<Duration>),
        resume: bool,
        plan: &ProfilePlan,
    ) -> u64 {
        let (node_limit, time_limit) = limits;
        let checkpoint = Some((checkpoint.to_path_buf(), every));
        let cfg =
            EngineConfig { threads: 1, progress: None, node_limit, time_limit, checkpoint, ..EngineConfig::default() };
        let branching = Arc::new(Branching::new(BranchSettings::of(&p.tactics)));
        let r = search_steered(p, &cfg, &[], None, resume, None, &branching, &OnceLock::new(), Some(plan));
        let r = r.expect("the search");
        assert!(!r.completed, "the limit stops the search");
        r.stats.nodes
    }

    /// The profile of a search resumed from a checkpoint of `stopped` steps and ended at `nodes`: every step counted
    /// once, the tree well formed, and its last `start` line that of a resumed search.  Returns its lines, its `start`
    /// lines, and its steps placed and not.
    fn resumed(plan: &ProfilePlan, stopped: u64, nodes: u64) -> (Vec<Json>, usize, u64, u64) {
        let lines = lines(&plan.file);
        counts_every_step_once(&lines, nodes);
        the_tree_is_well_formed(&lines, u64::from(plan.depth));
        let last = of_kind(&lines, "start").last().expect("a start line").clone();
        assert_eq!(last.get("resumed").and_then(Json::as_bool), Some(true));
        assert_eq!(number(&last, "nodes"), stopped);
        let (placed, unplaced) = counted(&lines);
        let starts = of_kind(&lines, "start").count();
        (lines, starts, placed, unplaced)
    }

    /// The value of the switch `key` of the last `start` line of `lines`.
    fn last_start(lines: &[Json], key: &str) -> Option<bool> {
        of_kind(lines, "start").last().and_then(|line| line.get(key)).and_then(Json::as_bool)
    }

    /// A search stopped and resumed goes on with its profile: the file is cut back to the steps line of the checkpoint
    /// (here past a block that a crash left after it), the tree goes on growing from the root the file holds, and every
    /// step is counted once.  A search resumed with no profile gets one that places none of its steps.  The search is
    /// stopped before its first step, the one stop that does not depend on the threads' timing.
    #[test]
    fn a_profile_resumes_with_its_search() {
        let p = game(800);
        let dir = TempDir::new("profile-resume");
        let (checkpoint, kept_checkpoint, kept_profile) = (dir.join("ckpt"), dir.join("ckpt.kept"), dir.join("kept"));
        let plan = ProfilePlan { file: dir.join("p.jsonl"), depth: DEPTH, most_ids: MOST_IDS };
        let stopped = limited(&p, &checkpoint, HOUR, (None, Some(Duration::ZERO)), false, &plan);
        fs::copy(&checkpoint, &kept_checkpoint).expect("a copy of the checkpoint");
        fs::copy(&plan.file, &kept_profile).expect("a copy of the profile");
        for case in ["ahead", "joined"] {
            fs::copy(&kept_checkpoint, &checkpoint).expect("the checkpoint");
            fs::copy(&kept_profile, &plan.file).expect("the profile");
            if case == "ahead" {
                // A block written for a checkpoint that a crash kept from being written.
                let mut file = OpenOptions::new().append(true).open(&plan.file).expect("the profile");
                writeln!(file, r#"{{"kind":"steps","seq":999,"nodes":{},"made":1,"ids":[]}}"#, stopped + 7)
                    .expect("a line");
            } else {
                fs::remove_file(&plan.file).expect("no profile");
            }
            let r = searched(&p, 1, Some((&checkpoint, HOUR)), None, true, Some(&plan));
            assert!(r.completed, "{case}");
            let (lines, starts, placed, unplaced) = resumed(&plan, stopped, r.stats.nodes);
            if case == "ahead" {
                assert!(starts == 2 && unplaced == 0 && placed == r.stats.nodes);
                assert!(of_kind(&lines, "node").count() > 1, "the tree goes on from the root");
                assert!(lines.iter().all(|line| line.get("seq").and_then(Json::as_u64) != Some(999)));
                assert_eq!(last_start(&lines, "frozen"), Some(false));
            } else {
                assert_eq!((starts, placed, unplaced), (1, 0, r.stats.nodes));
                assert_eq!(last_start(&lines, "rooted"), Some(false));
            }
        }
    }

    /// A profile behind its checkpoint, its last block lost, counts the steps it lacks as unplaced and tags no more:
    /// every unit is still placed at a node of its file (checked as the searcher yields, in debug builds).  Game 1020
    /// does not finish before the node limit stops it; the resumed search is stopped after a moment.
    #[test]
    fn a_profile_behind_its_checkpoint_counts_what_it_lacks_as_unplaced() {
        let p = game(1020);
        let dir = TempDir::new("profile-behind");
        let checkpoint = dir.join("ckpt");
        let plan = ProfilePlan { file: dir.join("p.jsonl"), depth: DEPTH, most_ids: MOST_IDS };
        // Checkpointed every 20 ms, so that the profile has steps lines before the stop's (a checkpoint costs the
        // worker a rebuild of each node of its stack).  The file is cut back before the stop's steps lines.
        let every = Duration::from_millis(20);
        let stopped = limited(&p, &checkpoint, every, (Some(1024), None), false, &plan);
        let text = fs::read_to_string(&plan.file).expect("the profile");
        let mark = format!(r#""nodes":{stopped},"#);
        let at = text.lines().position(|line| line.starts_with(r#"{"kind":"steps""#) && line.contains(&mark));
        let kept: Vec<&str> = text.lines().take(at.expect("the stop's steps line")).collect();
        fs::write(&plan.file, kept.join("\n") + "\n").expect("the profile cut back");
        let nodes = limited(&p, &checkpoint, HOUR, (None, Some(Duration::from_millis(100))), true, &plan);
        let (lines, starts, placed, unplaced) = resumed(&plan, stopped, nodes);
        assert!(starts == 2 && unplaced > 0 && placed > 0, "{unplaced} unplaced");
        assert_eq!(last_start(&lines, "frozen"), Some(true));
    }

    /// A fresh search sets an earlier profile aside, unless it is empty.
    #[test]
    fn an_earlier_profile_is_set_aside() {
        let dir = TempDir::new("profile-aside");
        let file = dir.join("p.jsonl");
        fs::write(&file, "").expect("an empty file");
        set_aside(&file).expect("nothing to set aside");
        assert!(file.exists() && !dir.join("p.jsonl.1").exists());
        for k in 1..=2 {
            fs::write(&file, "a line\n").expect("a profile");
            set_aside(&file).expect("set aside");
            assert!(!file.exists() && dir.join(format!("p.jsonl.{k}")).exists(), "{k}");
        }
    }
}
