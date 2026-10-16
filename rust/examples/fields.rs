//! The fields study: a bound on the promotions that adds GF(3) signed frame matroids to the two GF(2) frame matroids
//! and couples them by Lagrangian decomposition (avenue A1 of `docs/AGENDA.md`, experiments E1 and E2; the record is
//! `docs/experiments/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10.md`).  `common/dw.rs` has the bound and
//! why it is valid (`heuristics/THEORY.md`, Theorems 14 and 15), `common/gf3.rs` the signed
//! matroids, `common/cores.rs` the cores and their signings, `common/lp.rs` the master's simplex, `common/fit.rs`
//! the fits saved to disk.
//!
//! ```text
//! cargo run --release --example fields -- roots --known results/strict.jsonl [--known FILE...]
//!     [--arms gf2,f1,f1r1,f1r3,cores,f1cores,f1cores16] [--rounds 40] [--per-frame 8] [--cores-per-frame 4]
//!     [--pricings 3000] [--seconds 120] [--fixed-seconds 30] [--fit-mode certified|search] [--save-fit DIR]
//!     [--frames 1,2,3] [--defaults random|ones] [--out FILE] [--census FILE] N|A-B|A-B/STEP...
//! cargo run --release --example fields -- nodes --known results/strict.jsonl [--modes root,node] [--seconds 10]
//!     [--root-seconds 120] [--fit-mode certified|search] [--exact bench/matroid/nodes_exact.jsonl] [--out FILE]
//!     FIXTURE...
//! cargo run --release --example fields -- check [--known FILE] [--samples 100] N|A-B...
//! cargo run --release --example fields -- bnb --known results/strict.jsonl [--variants v0,v1,r1,r2,r2c|r3]
//!     [--refit 20] [--policy all|excess:E|tier1:F|r3-all|r3-top:A] [--r3-seconds 1] [--r3-pricings 300]
//!     [--r3-exact on|off] [--r3-cores on|off] [--r3-order exact-first|cores-first] [--label NAME] [--fit PATH]
//!     [--fit-seconds 300]
//!     [--fit-mode certified|search] [--seconds 7200] [--nodes 10000000] [--refit-seconds 30] [--audit EVERY]
//!     [--audit-seconds 10] [--audit-out FILE] [--knuth DIVES] [--max-depth D] [--seed 1] [--ci 0.90] [--log DIR]
//!     [--checkpoint DIR] [--checkpoint-minutes 10] [--save-states DIR] [--every 1] [--harvest FILE] [--out FILE]
//!     N|A-B|A-B/STEP...
//! cargo run --release --example fields -- replay --known results/strict.jsonl [--methods r2,r3:1,ref:10]
//!     [--refit 20] [--r3-pricings 300] [--refit-seconds 30] [--out FILE] STATES...
//! cargo run --release --example fields -- fit --known results/strict.jsonl [--start FIT] [--prune K] [--offer CORES]
//!     [--seconds 60] [--per-frame 8] [--label own] [--save-fit PATH] [--out FILE] N...
//! ```
//!
//! Games are set up as `taxman solve` sets them up; `P*`, the optimum promotion sum, is the known score less
//! `in_the_bag` (`--known` adds a results file to the built-in table, which ends at 1300, and may be given several
//! times: the later files' games replace the earlier ones' and must score the same).  `A-B` stands for its games
//! that are not prime, `A-B/STEP` for A, A + STEP, ... up to B.  Every command writes one JSON object per line, to
//! `--out` (appended, and a game or node already there is skipped, so a run resumes) or to standard output.
//!
//! * **roots**: per game and *arm* (the family of extra matroids), the bound and its bracket (see `common/dw.rs`):
//!   `gf2` none (the GF(2) bound); `f1` the plain signing (all values 1) of each frame of `--frames`; `f1r1`, `f1r3`
//!   that and one or three random signings per frame; `cores` the core-derived signings that the separation adds
//!   alone; `f1cores` (the main arm) the plain signings and the core-derived ones, at most `--per-frame` core-derived
//!   signings a frame, and `f1cores16` (any `f1cores` followed by a number) the same with that many.  `--seconds`
//!   budgets the core arms, `--fixed-seconds` the others.  With `--census`, every core the main arm finds, with its
//!   census data.  With `--save-fit DIR`, the fit of each `f1cores` arm goes to `DIR/n{N}.json` (`DIR/n{N}.{arm}.json`
//!   for the others; the format is `common/fit.rs`'s).
//! * **nodes**: per node of a `.nodes` fixture of `testdata/perf` (target `P* − sum(promoted)`, cut when the bound is
//!   at most it), the bound in mode `root` (the main arm's final signings and multipliers at the root, one evaluation
//!   at the node's matroids) and `node` (the node's own master and separation from those signings, `--seconds` a
//!   node), each the lower of it and the node's GF(2) bound.  `--exact` gives best completions to check against.
//! * **check**: Theorems 14 and 15 and the column-sum signings at scale (the tests 3, 4 and 6 of the study on any
//!   boards).
//! * **bnb** (design S5.2, experiment E3): the plain branch and bound of `common/bnb.rs` (the GF(2) bound and its
//!   conflict branching), which proves that nothing beats `P*`, with the bound of `common/dw.rs` at the nodes the
//!   GF(2) bound leaves open, per game and *variant*: `v0` none (the GF(2) bound alone); `v1` Tier 1 (one evaluation of
//!   `L`) at the root fit's multipliers at every node; `r1`, `r2`, `r2c` Tier 1 at the state the parent hands down,
//!   then, at the Tier-1-open nodes the policy selects, a refit of at most `--refit` pricings (R1, deflected Polyak;
//!   R2, the warm master; R2c, R2 with node-local signings, `dw.rs`'s "Warm states and the node methods") whose final
//!   state the children take over.  `--policy`: refit every such node (`all`), those whose GF(2) excess is at least
//!   `E`, or those whose Tier-1 excess is at most `F` times their GF(2) excess.  The variants of a game start from the
//!   same root fit, the main arm's (`--fit-seconds`, `--fit-mode`), read from `--fit` if it is there and written there
//!   if not, in the format of `common/fit.rs` (`roots --save-fit` writes the same files; `--fit` a directory, holding
//!   `n{N}.json`, unless it names a `.json` file, which serves its own game): the root's state is the fit's signings,
//!   frozen, its multipliers, and its master's columns and rank inequalities (`dw::Warm::from_saved`).
//!   `--seconds` and `--nodes` are a run's budgets (the audit's time not counted), `--refit-seconds` a refit's.
//!   One line a run, keyed by `n`, `variant`, `policy`, `k` (the `--refit`) and `mode` (`tree` or `knuth`): the root's
//!   GF(2) gap `g`, `opt`, `gf2`, `fit` (Tier 1 at the root from the fit, as the search evaluates it) and `fit_bound`
//!   (the fit's own bound), `completed`, the tally (`nodes`, the cuts by count, GF(2) bound, Tier 1 and refit,
//!   `better`, `branches`), the extra bound's counters (`tier1_evals`, `refits`, `pricings_refit`, `signings_local`,
//!   `refit_timeouts`, `audited`), the times in ms (`ms` the run's and `ms_extra` the extra bound's, both with the
//!   audit's; `ms_gf2` the GF(2) bound's with `ms_gf2_build` its matroids'; `ms_builds`, `ms_tier1`, `ms_refit`,
//!   `ms_audit` the extra bound's parts; `ms_mi`, `ms_greedy`, `ms_simplex` its evaluations' and masters') and
//!   `checks` (intersections cross-checked), `breakdowns` (master solves that ended optimal at a basis that is not
//!   primal feasible, whose duals are not to be trusted) and `saturated` (evaluations of `L` whose value saturated at
//!   `i64::MAX`, multipliers having gone far beyond every weight).  The default is the exact tree; with `--knuth
//!   DIVES`, paired dives of Knuth's estimator instead (dive `i`'s generator seeded by `--seed`, `n` and `i`, so that
//!   every variant follows the same dives), and the line has `dives`, `seed`, `ci` and the means, medians and bootstrap
//!   intervals (level `--ci`, 1,000 resamples) of the GF(2) tree's estimate (`gf2_*`), of the A1 tree's (`a1_*`) and of
//!   the ratio of their means (`ratio*`).  `--log DIR`: per node of a tree, a line of
//!   `n{N}-{variant}-{policy}-k{K}.jsonl` (`id`, `parent`, `depth`, `excess_gf2`, `excess_tier1`, `pricings`, `cut`,
//!   `subtree`, null for none; `cut` is `count`, `gf2`, `tier1`, `refit`, `leaf` or `none`); per dive, a line of
//!   `n{N}-{variant}-{policy}-k{K}-knuth.jsonl`, which a resumed run skips.  `--checkpoint DIR` (with `--fit`, so that
//!   a resumed run has the same root fit; for trees): every `--checkpoint-minutes`, the tree's stack, warm states and
//!   counters in `n{N}-{variant}-{policy}-k{K}.ckpt.json`, from which an interrupted run resumes to the same tree.
//!   `--audit EVERY` (to `--audit-out`): at each Tier-1-open node below the root whose hash is 0 modulo `EVERY`, from
//!   its inherited state, the reference (`nodes`' node mode, `--audit-seconds`) and the traces of R1, R2 and R2c over
//!   20 pricings, a line keyed by `n`, `variant` and `node`.
//!
//!   Variant `r3` (design U4) refits by R3 (`dw::refit_r3`: R2's pricing, the exact checks and the node's own
//!   separation, within `--r3-seconds` and `--r3-pricings`, `--r3-exact` and `--r3-cores` switching the last two off,
//!   `--r3-order cores-first` putting the separation before the exact checks, as the reference does) at the
//!   Tier-1-open nodes its policy selects and by R2 at the others: `r3-all` every one, `r3-top:A` those whose GF(2)
//!   excess is at least `A` times the root's GF(2) gap.  It runs alone, and the r3 policies are its alone.  Its line
//!   and files carry R3's settings (the key field `r3`, `t{T}-p{P}` with `-noexact`, `-nocores`, `-coresfirst`), and a
//!   run with `--label` the label (a key field too).  The run line also counts the Tier-1-open nodes below the root
//!   (`tier1_open`), R3's refits, those that cut, their pricings, rounds and ms (`r3_*`, `ms_r3`), the refits ended by
//!   a second breakdown of their master (`refit_breakdowns`), and the time in the exact checks, the separation and
//!   the masters' builds (`ms_exact`, `ms_separate`, `ms_master_build`); the per-node log the refit's `method`.
//!   `--max-depth D` stops each Knuth dive after its node of depth `D`.  `--save-states DIR` (with `--every K`): the
//!   state of every K-th Tier-1-open node below the root, as the run reaches it, a line of
//!   `DIR/{stem}.states.jsonl` (`state_line`: the node, its bounds, the state it inherited, and what the run's refit
//!   made of it); `--harvest FILE`: every distinct core that the nodes' separation finds, a line each
//!   (`cores::Core::to_json`).
//! * **replay** (design U4): each state of the files given (or of the `*.states.jsonl` of the directories given)
//!   made again, its GF(2) bound and Tier 1 checked against the state's line, then each method of `--methods` from
//!   it, on its own statistics: `r1`, `r2`, `r2c` (with `--refit` pricings), `r3:T`, `r3x:T` (R3 without its
//!   separation: R2 and the exact checks), `r3c:T` (without its exact checks: R2 and the separation), `r3s:T` (in the
//!   reference's order, the separation first), each with T₃ = T seconds and `--r3-pricings`, and `ref:T` (the
//!   reference, the audit's, within T seconds).  A line per state and
//!   method, keyed by `n`, `host`, `index` and `method`: the node's depth and excesses, the bound, `excess`, `cut`,
//!   `pricings`, `rounds`, `new_signings`, `cores` (found), `stop`, `host_same` (for the host's own method: whether
//!   the bound and the final state are the host's), the exact checks' cuts and checks, `breakdowns`, and the ms of
//!   the whole (`ms`), of making the node (`ms_node`), of the pricings, the simplex, the exact checks, the separation
//!   and the masters' builds.
//! * **fit** (design U4 and T7): the main arm's fit at a root, certified with separation within `--seconds` (60), from
//!   the family of the fit `--start` (its signings frozen, those of frames the root lacks left out, with `--prune K`
//!   those without a positive multiplier too and all but the `K` of the largest sums of multipliers a frame; its
//!   multipliers by candidate number) or from the plain signings, with the cores of `--offer` (a `--harvest` file)
//!   placed first in new core-derived signings (`dw::bound_offered`, at most `--per-frame` a frame); saved to
//!   `--save-fit` (a `.json` file, or a directory for `n{N}.json`); a line keyed by `n` and `label` (`--label`,
//!   default `own`) with the bound, its share of the GF(2) gap, the carried family's bound and share before the
//!   refit, the cores offered and placed, the signings and the time.  It asserts the bound at least `P*` and the
//!   optimal promotions independent in every matroid of the family.
//!
//! `--fit-mode` (`dw::FitMode`; of `roots` and `nodes`, every computation of the run; of `bnb`, the root fit):
//! `certified` (the default) brackets each LP with a certified lower bound (the exact checks); `search` fits the
//! multipliers as a search would, without the exact checks (a family's generation ends when nothing prices out and no
//! cheap inequality is violated, a separation round that places no core ends the computation, stop `Fitted`), and
//! reports `lb` 0, so `converged` false.
//!
//! Every run asserts that the bound is at least `P*` (at nodes, at least the best completion where known), that the
//! optimal game's promotions are independent in every final matroid, and checks the crate's intersection against
//! the reference algorithm every 50th evaluation; `bnb` asserts that no completion beats `P*`, and that a node the
//! extra bound cuts has no promotable GF(2) heaviest set.

#[allow(dead_code)]
#[path = "common/bnb.rs"]
mod bnb;
#[allow(dead_code)]
#[path = "common/cores.rs"]
mod cores;
#[allow(dead_code)]
#[path = "common/dw.rs"]
mod dw;
#[allow(dead_code)]
#[path = "common/fit.rs"]
mod fit;
#[allow(dead_code)]
#[path = "common/fixtures.rs"]
mod fixtures;
#[allow(dead_code)]
#[path = "common/frames.rs"]
mod frames;
#[allow(dead_code)]
#[path = "common/gf3.rs"]
mod gf3;
#[allow(dead_code)]
#[path = "common/lp.rs"]
mod lp;
#[allow(dead_code)]
#[path = "common/meet.rs"]
mod meet;

use bnb::{BoundKind, ExtraBound, Fate, NodeLog, NodeRecord, NodeView, Pending, Search, Tally};
use cores::{core_of, cores_of, frame_with, merge, primes, residual, sees, zero_one, Core};
use dw::{
    bound, bound_offered, evaluate, refit_r1, refit_r2, refit_r3, tier1, Ctx, Eval, Family, FitMode, Outcome, Settings,
    Stats, Stop, Trace, Warm, LOCAL_PER_NODE, R3,
};
use fit::SavedFit;
use frames::{frame_matroids, is_prime, setup, Known, Rng};
use gf3::{seed, Default3, Matroid3, Signing};
use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};
use taxman::bits::Bits;
use taxman::json::Json;
use taxman::matchbound::MatchGraph;
use taxman::model::{FrameGraph, FrameSet, Scratch};
use taxman::search::Prepared;

/// The salt of the random arms' seeds.
const SALT_RANDOM: u64 = 1;

// ---------------------------------------------------------------------------------------------------------
// Options

struct Options {
    command: String,
    known: Vec<PathBuf>,
    arms: Vec<String>,
    modes: Vec<String>,
    settings: Settings,
    fixed_seconds: f64,
    root_seconds: f64,
    frames: Vec<usize>,
    out: Option<PathBuf>,
    census: Option<PathBuf>,
    save_fit: Option<PathBuf>,
    exact: Option<PathBuf>,
    samples: usize,
    games: Vec<u32>,
    files: Vec<PathBuf>,
    /// The methods of `replay`.
    methods: Vec<String>,
    /// The fit `fit` starts from, the cores it offers, and how many carried signings a frame it keeps (`--prune`).
    start: Option<PathBuf>,
    offer: Option<PathBuf>,
    prune: Option<usize>,
    bnb: BnbOptions,
}

/// The options of `bnb` (see the module docs).
struct BnbOptions {
    variants: Vec<Variant>,
    /// The most pricings of a refit (`--refit`).
    k: usize,
    policy: Policy,
    /// R3's budgets and switches (`--r3-seconds`, `--r3-pricings`, `--r3-exact`, `--r3-cores`).
    r3: R3,
    /// A name for the run beside its variant and policy (`--label`; the root fit's arm, say).
    label: Option<String>,
    /// Where the states of every `every`-th Tier-1-open node go (`--save-states`, `--every`).
    save_states: Option<PathBuf>,
    every: u64,
    /// Where the distinct cores that the nodes' separation finds go (`--harvest`).
    harvest: Option<PathBuf>,
    /// The depth at which a Knuth dive stops (`--max-depth`).
    max_depth: Option<usize>,
    /// Where the root fits are (`--fit`).
    fit: Option<PathBuf>,
    fit_seconds: f64,
    /// A run's budgets: seconds (the audit's not counted) and nodes.
    seconds: f64,
    nodes: u64,
    refit_seconds: f64,
    /// Audit the nodes whose hash is 0 modulo this.
    audit: Option<u64>,
    audit_seconds: f64,
    audit_out: Option<PathBuf>,
    /// Dives of Knuth's estimator instead of the exact tree.
    knuth: Option<usize>,
    seed: u64,
    ci: f64,
    log: Option<PathBuf>,
    checkpoint: Option<PathBuf>,
    checkpoint_every: Every,
    /// For the tests: the run stops, as if it were killed, once it has treated this many nodes.
    halt_at: Option<u64>,
}

impl Default for BnbOptions {
    fn default() -> Self {
        BnbOptions {
            variants: Variant::PILOT.to_vec(),
            k: 20,
            policy: Policy::All,
            r3: R3::default(),
            label: None,
            save_states: None,
            every: 1,
            harvest: None,
            max_depth: None,
            fit: None,
            fit_seconds: 300.0,
            seconds: 7200.0,
            nodes: 10_000_000,
            refit_seconds: 30.0,
            audit: None,
            audit_seconds: 10.0,
            audit_out: None,
            knuth: None,
            seed: 1,
            ci: 0.90,
            log: None,
            checkpoint: None,
            checkpoint_every: Every::Minutes(10.0),
            halt_at: None,
        }
    }
}

impl BnbOptions {
    /// Whether a run that has treated `nodes` nodes in `ms` ms, with the extra bound's `counters`, is within its
    /// budgets (the audit's time not counted).
    fn within(&self, nodes: u64, ms: f64, counters: &Counters) -> bool {
        nodes < self.nodes && ms - counters.ms_audit < self.seconds * 1e3
    }
}

fn parse() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or("expected a command: roots, nodes, check, bnb, replay or fit")?;
    let mut o = Options {
        command,
        known: Vec::new(),
        arms: ["gf2", "f1", "f1r1", "f1r3", "cores", "f1cores"].map(String::from).to_vec(),
        modes: vec!["root".into(), "node".into()],
        settings: Settings::default(),
        fixed_seconds: 30.0,
        root_seconds: 120.0,
        frames: vec![1, 2, 3],
        out: None,
        census: None,
        save_fit: None,
        exact: None,
        samples: 100,
        games: Vec::new(),
        files: Vec::new(),
        methods: ["r2", "r3:1", "ref:10"].map(String::from).to_vec(),
        start: None,
        offer: None,
        prune: None,
        bnb: BnbOptions::default(),
    };
    let mut seconds = None;
    let list = |s: String| s.split(',').map(str::to_string).collect::<Vec<_>>();
    let on_off = |name: &str, v: String| match v.as_str() {
        "on" => Ok(true),
        "off" => Ok(false),
        other => Err(format!("{name} {other:?}: not on or off")),
    };
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        let num = |v: String| v.parse::<f64>().map_err(|e| format!("{v}: {e}"));
        match arg.as_str() {
            "--r3-seconds" => o.bnb.r3.seconds = num(value("--r3-seconds")?)?,
            "--r3-pricings" => o.bnb.r3.pricings = num(value("--r3-pricings")?)? as usize,
            "--r3-exact" => o.bnb.r3.exact = on_off("--r3-exact", value("--r3-exact")?)?,
            "--r3-cores" => o.bnb.r3.cores = on_off("--r3-cores", value("--r3-cores")?)?,
            "--r3-order" => {
                o.bnb.r3.cores_first = match value("--r3-order")?.as_str() {
                    "exact-first" => false,
                    "cores-first" => true,
                    other => return Err(format!("--r3-order {other:?}: not exact-first or cores-first")),
                }
            }
            "--label" => o.bnb.label = Some(value("--label")?),
            "--save-states" => o.bnb.save_states = Some(value("--save-states")?.into()),
            "--every" => match num(value("--every")?)? as u64 {
                0 => return Err("--every needs a positive K".into()),
                every => o.bnb.every = every,
            },
            "--harvest" => o.bnb.harvest = Some(value("--harvest")?.into()),
            "--max-depth" => o.bnb.max_depth = Some(num(value("--max-depth")?)? as usize),
            "--methods" => o.methods = list(value("--methods")?),
            "--start" => o.start = Some(value("--start")?.into()),
            "--offer" => o.offer = Some(value("--offer")?.into()),
            "--prune" => o.prune = Some(num(value("--prune")?)? as usize),
            "--known" => o.known.push(value("--known")?.into()),
            "--arms" => o.arms = list(value("--arms")?),
            "--modes" => o.modes = list(value("--modes")?),
            "--rounds" => o.settings.rounds = num(value("--rounds")?)? as usize,
            "--per-frame" => o.settings.per_frame = num(value("--per-frame")?)? as usize,
            "--cores-per-frame" => o.settings.cores_per_frame = num(value("--cores-per-frame")?)? as usize,
            "--pricings" => o.settings.pricings = num(value("--pricings")?)? as usize,
            "--seconds" => seconds = Some(num(value("--seconds")?)?),
            "--fixed-seconds" => o.fixed_seconds = num(value("--fixed-seconds")?)?,
            "--root-seconds" => o.root_seconds = num(value("--root-seconds")?)?,
            "--samples" => o.samples = num(value("--samples")?)? as usize,
            "--close" => o.settings.close = num(value("--close")?)?,
            "--frames" => {
                o.frames = list(value("--frames")?)
                    .iter()
                    .map(|f| f.parse().map_err(|e| format!("{f}: {e}")))
                    .collect::<Result<_, _>>()?
            }
            "--defaults" => {
                o.settings.random_defaults = match value("--defaults")?.as_str() {
                    "random" => true,
                    "ones" => false,
                    other => return Err(format!("unknown defaults {other:?}")),
                }
            }
            "--fit-mode" => {
                o.settings.mode = match value("--fit-mode")?.as_str() {
                    "certified" => FitMode::Certified,
                    "search" => FitMode::Search,
                    other => return Err(format!("unknown fit mode {other:?}")),
                }
            }
            "--out" => o.out = Some(value("--out")?.into()),
            "--census" => o.census = Some(value("--census")?.into()),
            "--save-fit" => o.save_fit = Some(value("--save-fit")?.into()),
            "--exact" => o.exact = Some(value("--exact")?.into()),
            "--variants" => {
                o.bnb.variants =
                    list(value("--variants")?).iter().map(|v| Variant::parse(v)).collect::<Result<_, _>>()?
            }
            "--refit" => o.bnb.k = num(value("--refit")?)? as usize,
            "--policy" => o.bnb.policy = Policy::parse(&value("--policy")?)?,
            "--fit" => o.bnb.fit = Some(value("--fit")?.into()),
            "--fit-seconds" => o.bnb.fit_seconds = num(value("--fit-seconds")?)?,
            "--nodes" => o.bnb.nodes = num(value("--nodes")?)? as u64,
            "--refit-seconds" => o.bnb.refit_seconds = num(value("--refit-seconds")?)?,
            "--audit" => match num(value("--audit")?)? as u64 {
                0 => return Err("--audit needs a positive EVERY".into()),
                every => o.bnb.audit = Some(every),
            },
            "--audit-seconds" => o.bnb.audit_seconds = num(value("--audit-seconds")?)?,
            "--audit-out" => o.bnb.audit_out = Some(value("--audit-out")?.into()),
            "--knuth" => o.bnb.knuth = Some(num(value("--knuth")?)? as usize),
            "--seed" => {
                let v = value("--seed")?;
                o.bnb.seed = v.parse().map_err(|e| format!("{v}: {e}"))?
            }
            "--ci" => o.bnb.ci = num(value("--ci")?)?,
            "--log" => o.bnb.log = Some(value("--log")?.into()),
            "--checkpoint" => o.bnb.checkpoint = Some(value("--checkpoint")?.into()),
            "--checkpoint-minutes" => o.bnb.checkpoint_every = Every::Minutes(num(value("--checkpoint-minutes")?)?),
            file if o.command == "nodes" || o.command == "replay" => o.files.push(file.into()),
            range => o.games.extend(games(range)?),
        }
    }
    if o.command == "bnb" {
        o.bnb.seconds = seconds.unwrap_or(o.bnb.seconds);
        let r3 = o.bnb.variants.contains(&Variant::R3);
        if r3 != o.bnb.policy.is_r3() || (r3 && o.bnb.variants.len() > 1) {
            return Err(
                "variant r3 runs alone, with the policy r3-all or r3-top:A, which no other variant takes".into()
            );
        }
        if o.bnb.checkpoint.is_some() && o.bnb.fit.is_none() {
            return Err("--checkpoint needs --fit: a resumed run must start from the same root fit".into());
        }
        if o.bnb.checkpoint.is_some() && o.bnb.knuth.is_some() {
            return Err("--checkpoint is for exact trees: a Knuth run resumes from its dives under --log".into());
        }
        if !(o.bnb.ci > 0.0 && o.bnb.ci < 1.0) {
            return Err(format!("--ci {} is not a level between 0 and 1", o.bnb.ci));
        }
    } else {
        let default_seconds = match o.command.as_str() {
            "nodes" => 10.0,
            "fit" => 60.0,
            _ => 120.0,
        };
        o.settings.seconds = seconds.unwrap_or(default_seconds);
    }
    Ok(o)
}

/// The games of `N`, `A-B` (those that are not prime) or `A-B/STEP`.
fn games(range: &str) -> Result<Vec<u32>, String> {
    let parse_n = |t: &str| t.parse::<u32>().map_err(|e| format!("{range}: {e}"));
    let (span, step) = match range.split_once('/') {
        Some((span, step)) => (span, Some(parse_n(step)?)),
        None => (range, None),
    };
    Ok(match (span.split_once('-'), step) {
        (Some((a, b)), Some(step)) => (parse_n(a)?..=parse_n(b)?).step_by(step.max(1) as usize).collect(),
        (Some((a, b)), None) => (parse_n(a)?..=parse_n(b)?).filter(|&n| !is_prime(n)).collect(),
        (None, _) => vec![parse_n(span)?],
    })
}

/// Where lines go: a file, appended (with the keys of the lines already there), or standard output.
struct Output {
    file: Option<std::fs::File>,
    done: HashSet<String>,
}

impl Output {
    /// `key` names the fields that identify a line (a line is skipped when its key is done).
    fn open(path: Option<&Path>, key: &[&str]) -> Output {
        let Some(path) = path else { return Output { file: None, done: HashSet::new() } };
        let mut done = HashSet::new();
        if let Ok(text) = std::fs::read_to_string(path) {
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                let json = Json::parse(line).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                done.insert(Self::key_of(&json, key));
            }
        }
        let file = std::fs::OpenOptions::new().create(true).append(true).open(path);
        Output { file: Some(file.unwrap_or_else(|e| panic!("{}: {e}", path.display()))), done }
    }

    fn key_of(json: &Json, key: &[&str]) -> String {
        key.iter()
            .map(|k| match json.get(k) {
                Some(Json::Str(s)) => s.clone(),
                Some(v) => v.as_u64().map_or_else(|| format!("{v:?}"), |x| x.to_string()),
                None => String::new(),
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    fn done(&self, key: &str) -> bool {
        self.done.contains(key)
    }

    /// Writes `json` as a line unless a line with its key (the fields `key`) is there or was written; whether it was.
    fn write_once(&mut self, json: &Json, key: &[&str]) -> bool {
        let fresh = self.done.insert(Self::key_of(json, key));
        if fresh {
            self.write(&json.to_string());
        }
        fresh
    }

    fn write(&mut self, line: &str) {
        match &mut self.file {
            Some(f) => {
                writeln!(f, "{line}").expect("write");
                f.flush().expect("flush");
            }
            None => println!("{line}"),
        }
    }
}

fn main() {
    let o = parse().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(2);
    });
    let known = Known::load(&o.known, None).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(2);
    });
    match o.command.as_str() {
        "roots" => {
            let mut out = Output::open(o.out.as_deref(), &["n", "arm", "mode"]);
            let mut census = o.census.as_deref().map(|p| Output::open(Some(p), &["n", "index"]));
            for &n in &o.games {
                taxman::with_width!(n, W => roots::<W>(n, &known, &o, &mut out, census.as_mut()));
            }
        }
        "nodes" => {
            let mut out = Output::open(o.out.as_deref(), &["n", "node", "mode"]);
            let exact = o.exact.as_deref().map(load_exact).unwrap_or_default();
            for path in &o.files {
                let n = fixture_n(path);
                taxman::with_width!(n, W => nodes::<W>(path, &known, &o, &exact, &mut out));
            }
        }
        "check" => {
            for &n in &o.games {
                taxman::with_width!(n, W => check::<W>(n, &known, o.samples));
            }
        }
        "bnb" => {
            let mut out = Output::open(o.out.as_deref(), &BNB_KEY);
            let mut audit = o.bnb.audit.map(|_| Output::open(o.bnb.audit_out.as_deref(), &AUDIT_KEY));
            for &n in &o.games {
                taxman::with_width!(n, W => bnb::<W>(n, &known, &o, &mut out, audit.as_mut()));
            }
        }
        "replay" => {
            let probes: Result<Vec<Probe>, String> = o.methods.iter().map(|m| Probe::parse(m, &o.bnb)).collect();
            let probes = probes.unwrap_or_else(|e| {
                eprintln!("error: {e}");
                std::process::exit(2);
            });
            let mut out = Output::open(o.out.as_deref(), &REPLAY_KEY);
            for path in state_files(&o.files) {
                replay(&path, &known, &probes, &o.bnb, &mut out);
            }
        }
        "fit" => {
            let mut out = Output::open(o.out.as_deref(), &["n", "label"]);
            for &n in &o.games {
                taxman::with_width!(n, W => fit_game::<W>(n, &known, &o, &mut out));
            }
        }
        other => {
            eprintln!("error: unknown command {other:?}");
            std::process::exit(2);
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// Arms

/// The plain and random signings of an arm: the plain signing of each frame of `frames` (that the state has),
/// then `random` random signings of each.
fn fixed_signings(n: u32, len: usize, frames: &[usize], plain: bool, random: usize) -> Vec<Signing> {
    let frames: Vec<usize> = frames.iter().copied().filter(|&h| h < len).collect();
    let mut out: Vec<Signing> = Vec::new();
    if plain {
        out.extend(frames.iter().map(|&h| Signing::ones(h)));
    }
    for &h in &frames {
        out.extend((0..random).map(|i| Signing::random(h, seed(n, h, i, SALT_RANDOM))));
    }
    out
}

/// The most core-derived signings a frame, if the arm's name gives it (`f1cores16`: 16).
fn arm_cap(name: &str) -> Option<usize> {
    name.strip_prefix("f1cores").and_then(|cap| cap.parse().ok())
}

/// The initial signings of `arm` and whether it separates.
fn arm(name: &str, n: u32, len: usize, frames: &[usize]) -> (Vec<Signing>, bool) {
    match if arm_cap(name).is_some() { "f1cores" } else { name } {
        "gf2" => (Vec::new(), false),
        "f1" => (fixed_signings(n, len, frames, true, 0), false),
        "f1r1" => (fixed_signings(n, len, frames, true, 1), false),
        "f1r3" => (fixed_signings(n, len, frames, true, 3), false),
        "cores" => (Vec::new(), true),
        "f1cores" => (fixed_signings(n, len, frames, true, 0), true),
        other => panic!("unknown arm {other:?}"),
    }
}

// ---------------------------------------------------------------------------------------------------------
// roots

fn list<T: std::fmt::Display>(v: impl IntoIterator<Item = T>) -> String {
    v.into_iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

/// The optimal game's promotions, if the known game reaches `P*`.
fn optimal_promotions<const W: usize>(
    n: u32,
    known: &Known,
    board: &taxman::board::Board<W>,
    opt: u64,
) -> Option<Bits<W>> {
    let seed = board.seed(known.optimal.moves(n)?);
    (seed.sum() == opt).then_some(seed)
}

/// Asserts what every run asserts of an outcome: the optimal promotions independent in every final matroid.
fn assert_independent<const W: usize>(n: u32, fam: &Family, set: &Bits<W>, what: &str) {
    let members: Vec<u32> = set.iter().collect();
    for ex in &fam.extras {
        assert!(
            ex.m.independent(&members),
            "n={n}: {what} is dependent in the signed matroid of frame {} ({:?})",
            ex.m.frame,
            ex.signing.default
        );
    }
}

/// Runs `f` at the root of `n` set up as every root computation is: the GF(2) matroids of the candidates, the most
/// promotions `max_total`, the target `P*` (not stopping there) and the GF(2) gap for the closing tolerance; `f` is
/// given the context and the GF(2) bound.
fn at_root<const W: usize, R>(n: u32, p: &Prepared<W>, opt: u64, f: impl FnOnce(&Ctx<'_, W>, i64) -> R) -> R {
    let fm = frame_matroids(&p.model, &p.root, &p.candidates);
    let cap = p.max_total as usize;
    let gf2 = fm.bound(cap).value;
    let mut ctx = Ctx::new(n, &p.model, &p.root, &fm, cap, opt as i64, false);
    ctx.gap = Some(gf2 - opt as i64);
    f(&ctx, gf2)
}

/// The main arm's bound at a root (the context of [`at_root`]), with `settings`; asserted at least `P*`.
fn fit_outcome<const W: usize>(ctx: &Ctx<'_, W>, frames: &[usize], settings: &Settings) -> Outcome {
    let (signings, separate) = arm("f1cores", ctx.n, ctx.state.len(), frames);
    let r = bound(ctx, signings, None, separate, settings);
    assert!(r.bound >= ctx.target, "n={}: the root's bound {} is below P* = {}", ctx.n, r.bound, ctx.target);
    r
}

fn roots<const W: usize>(n: u32, known: &Known, o: &Options, out: &mut Output, census: Option<&mut Output>) {
    let Some((board, p)) = setup::<W>(n) else { return };
    let Some(opt) = known.opt(n, board.in_the_bag) else {
        eprintln!("n={n}: no known optimum, skipped");
        return;
    };
    let model = &p.model;
    let optimal = optimal_promotions(n, known, &board, opt);
    let mut census = census;
    let mode = o.settings.mode.name();
    at_root(n, &p, opt, |ctx, gf2| {
        if let Some(set) = &optimal {
            ctx.fm.independent(set).unwrap_or_else(|e| panic!("n={n}: the optimal promotions: {e}"));
        }
        for name in &o.arms {
            if out.done(&format!("{n}/{name}/{mode}")) {
                continue;
            }
            let (signings, separate) = arm(name, n, p.root.len(), &o.frames);
            let seconds = if separate { o.settings.seconds } else { o.fixed_seconds };
            let per_frame = arm_cap(name).unwrap_or(o.settings.per_frame);
            let settings = Settings { seconds, per_frame, ..o.settings.clone() };
            let r = bound(ctx, signings, None, separate, &settings);
            assert!(r.bound >= opt as i64, "n={n}, arm {name}: the bound {} is below P* = {opt}", r.bound);
            if name == "gf2" {
                assert_eq!(r.bound, gf2, "n={n}: the gf2 arm differs from the GF(2) bound");
            }
            // The fit that gave the bound gives it again, and the optimal promotions are independent in its matroids.
            let saved = SavedFit::of(n, opt, gf2, mode, &r);
            assert_eq!(saved.bound(ctx), r.bound, "n={n}, arm {name}: the fit does not give the bound");
            if let Some(set) = &optimal {
                assert_independent(n, &r.family, set, "the optimal promotions");
                assert_independent(n, &Family::new(ctx, saved.fit.signings.clone()), set, "the optimal promotions");
            }
            if let Some(dir) = o.save_fit.as_ref().filter(|_| name.starts_with("f1cores")) {
                let file = if name == "f1cores" { format!("n{n}.json") } else { format!("n{n}.{name}.json") };
                saved.save(&dir.join(file)).unwrap_or_else(|e| panic!("{e}"));
            }
            let max_rank = ctx.fm.ranks.iter().copied().max();
            let info = RootInfo { n, arm: name, opt, gf2, mode, per_frame, max_rank };
            out.write(&root_line(&info, &r, model, &p.root));
            if name == "f1cores" {
                if let Some(c) = census.as_deref_mut() {
                    for (i, f) in r.cores.iter().enumerate() {
                        if !c.done(&format!("{n}/{i}")) {
                            c.write(&census_line(n, i, f.core.clone(), f.assigned));
                        }
                    }
                }
            }
            eprintln!(
                "n={n} {name}: bound {} (gf2 {gf2}, P* {opt}), bracket [{:.2}, {:.2}], {:?}, {:.1} s",
                r.bound, r.lb, r.l_best, r.stop, r.seconds
            );
        }
    });
}

/// What a root's line says about the run besides its outcome.
struct RootInfo<'a> {
    n: u32,
    arm: &'a str,
    opt: u64,
    gf2: i64,
    /// `certified` or `search` (`dw::FitMode`).
    mode: &'a str,
    /// The most core-derived signings a frame the arm was allowed.
    per_frame: usize,
    /// The largest rank of a frame's matroid (a block of more than 64 or 128 needs two or three words).
    max_rank: Option<usize>,
}

fn root_line<const W: usize>(info: &RootInfo, r: &Outcome, model: &FrameGraph, root: &FrameSet<W>) -> String {
    let RootInfo { n, arm, opt, gf2, mode, per_frame: cap, max_rank } = *info;
    let frames = root.len();
    let gap = gf2 - opt as i64;
    let bracket = r.l_best - r.lb;
    let converged = gap > 0 && bracket <= 0.01 * gap as f64;
    let share = if gap > 0 { format!("{:.5}", (gf2 - r.bound) as f64 / gap as f64) } else { "null".into() };
    let mut per_frame = vec![0usize; frames];
    let mut core_signings = Vec::new();
    for ex in &r.family.extras {
        per_frame[ex.m.frame] += 1;
        if ex.signing.mergeable {
            core_signings.push(format!("[{},{}]", ex.m.frame, ex.signing.cores.len()));
        }
    }
    let mut scratch = Scratch::<W>::new();
    let (mut promotable, mut unpromotable_mass) = (0, 0.0);
    for (a, set) in &r.support {
        let bits: Bits<W> = Bits::from_slice(set);
        if model.can_promote_all_using(&mut scratch, root, &bits) {
            promotable += 1;
        } else {
            unpromotable_mass += a;
        }
    }
    let s: &Stats = &r.stats;
    format!(
        "{{\"n\":{n},\"arm\":\"{arm}\",\"mode\":\"{mode}\",\"cap\":{cap},\"max_rank\":{},\"opt\":{opt},\"gf2\":{gf2},\
         \"bound\":{},\"lb\":{:.4},\"l_best\":{:.4},\
         \"bracket\":{bracket:.4},\"converged\":{converged},\"share\":{share},\"stop\":\"{:?}\",\"matroids\":[{}],\
         \"core_signings\":[{}],\"cores_found\":{},\"cores_assigned\":{},\"cores_skipped\":{},\
         \"cores_repeated\":{},\"gf3_loops\":{},\"rounds\":{},\"pricings_mi\":{},\"pricings_greedy\":{},\
         \"pivots\":{},\"degenerate\":{},\"repairs\":{},\"solves\":{},\"masters\":{},\"columns\":{},\"cuts\":{},\
         \"master\":{:.4},\"cuts_cheap\":{},\"cuts_exact\":{},\"checks_exact\":{},\"pivots_exact\":{},\"max_s\":{},\
         \"purged\":{},\"restarts\":{},\"checks_capped\":{},\"support\":{},\
         \"support_promotable\":{promotable},\"mass_unpromotable\":{unpromotable_mass:.5},\"checks\":{},\
         \"ms\":{:.0},\"ms_mi\":{:.0},\"ms_simplex\":{:.0}}}",
        max_rank.map_or("null".into(), |m| m.to_string()),
        r.bound,
        r.lb,
        r.l_best,
        r.stop,
        list(&per_frame),
        core_signings.join(","),
        r.cores.len(),
        r.cores_assigned,
        r.cores_unplaced,
        r.cores_repeated,
        r.family.loops(),
        r.rounds,
        s.pricings_mi,
        s.pricings_greedy,
        s.pivots,
        s.degenerate,
        s.repairs,
        s.solves,
        s.masters,
        r.columns,
        r.cuts,
        r.master,
        s.cuts_cheap,
        s.cuts_exact,
        s.checks_exact,
        s.pivots_exact,
        s.max_s,
        s.purged,
        s.restarts,
        s.checks_capped,
        r.support.len(),
        s.checks,
        r.seconds * 1e3,
        s.ms_mi,
        s.ms_simplex,
    )
}

/// The census of a core the main arm found (for avenue A4 and conjecture C2): its size, its 0/1 matrix's
/// determinant mod 3 when square (else its rank deficiency over GF(3)), its primes, whether the plain signing and
/// the three random signings of its frame (those of arm `f1r3`) see it, and the signing it went to.
fn census_line(n: u32, index: usize, core: Core, assigned: Option<usize>) -> String {
    let (det0, deficiency) = zero_one(&core);
    let r3 = (0..3).any(|i| sees(&Signing::random(core.frame, seed(n, core.frame, i, SALT_RANDOM)), &core));
    format!(
        "{{\"n\":{n},\"index\":{index},\"frame\":{},\"size\":{},\"members\":[{}],\"factors\":{},\"square\":{},\
         \"primes\":{},\"det0\":{},\"deficiency3\":{deficiency},\"promoted_in\":{},\"promoted_out\":{},\
         \"seen_f1\":{},\"seen_r3\":{r3},\"assigned_to\":{}}}",
        core.frame,
        core.members.len(),
        list(&core.members),
        core.columns.len(),
        core.columns.len() == core.members.len(),
        primes(&core),
        det0.map_or("null".into(), |d| d.to_string()),
        core.promoted_in.len(),
        core.promoted_out.len(),
        sees(&Signing::ones(core.frame), &core),
        assigned.map_or("null".into(), |a| a.to_string()),
    )
}

// ---------------------------------------------------------------------------------------------------------
// nodes

fn fixture_n(path: &Path) -> u32 {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let line = text.lines().find(|l| l.starts_with("n ")).expect("an n line");
    line[2..].trim().parse().expect("a board size")
}

/// The best completions of `bench/matroid/nodes_exact.jsonl`, by (n, depth, promoted sum, candidates).
type Exact = HashMap<(u64, u64, u64, u64), u64>;

fn load_exact(path: &Path) -> Exact {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut out = HashMap::new();
    let mut clash = HashSet::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let json = Json::parse(line).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let field = |k| json.get(k).and_then(Json::as_u64);
        let (Some(n), Some(d), Some(s), Some(c)) =
            (field("n"), field("depth"), field("promoted_sum"), field("candidates"))
        else {
            continue;
        };
        if let Some(e) = field("exact") {
            if out.insert((n, d, s, c), e).is_some_and(|old| old != e) {
                clash.insert((n, d, s, c));
            }
        }
    }
    for k in clash {
        out.remove(&k);
    }
    out
}

fn nodes<const W: usize>(path: &Path, known: &Known, o: &Options, exact: &Exact, out: &mut Output) {
    let (n, incumbent, units) = fixtures::read_nodes::<W>(path).expect("a fixture");
    let (board, p) = setup::<W>(n).expect("a game with candidates");
    let opt = known.opt(n, board.in_the_bag).expect("a known optimum");
    let model = &p.model;
    let root_settings = Settings { seconds: o.root_seconds, ..o.settings.clone() };
    let root = at_root(n, &p, opt, |ctx, _| fit_outcome(ctx, &o.frames, &root_settings));
    let final_signings: Vec<Signing> = root.family.extras.iter().map(|ex| ex.signing.clone()).collect();
    eprintln!(
        "n={n}: P* {opt} (fixture incumbent {}), root bound {} in {:.1} s, {} signings",
        incumbent.sum(),
        root.bound,
        root.seconds,
        final_signings.len()
    );
    let mut scratch = Scratch::<W>::new();
    for (i, u) in units.iter().filter(|u| !u.cands.is_empty()).enumerate() {
        let state = model.promote_all_using(&mut scratch, &p.root, &u.promoted).expect("a node of the game");
        let max_add = (p.max_total as i32 - u.promoted.len() as i32).min(state.max_promotions_among(&u.cands)).max(0);
        let fmn = frame_matroids(model, &state, &u.cands);
        let gf2 = fmn.bound(max_add as usize).value;
        let target = opt as i64 - u.promoted.sum() as i64;
        let key = (u64::from(n), u64::from(u.promoted.len()), u.promoted.sum(), u64::from(u.cands.len()));
        let best = exact.get(&key).copied();
        for mode in &o.modes {
            if out.done(&format!("{n}/{i}/{mode}")) {
                continue;
            }
            let t = Instant::now();
            let ctxn = Ctx::new(n, model, &state, &fmn, max_add as usize, target, true);
            let (l_bound, pricings) = match mode.as_str() {
                "root" => {
                    let fam = Family::new(&ctxn, final_signings.clone());
                    let lam = fam.multipliers_of(&root.lambda);
                    let mut stats = Stats::default();
                    let ev = evaluate(&ctxn, &fam, &lam, &mut stats, 1);
                    (ev.value.div_euclid(dw::SCALE), 1)
                }
                "node" => {
                    // The root's signings stay as they are (their cores are the root's); the node's separation adds
                    // signings of its own.
                    let frozen = final_signings.iter().map(dw::frozen).collect();
                    let r = bound(&ctxn, frozen, Some(&root.lambda), true, &o.settings);
                    (r.bound, r.stats.pricings_mi)
                }
                other => panic!("unknown mode {other:?}"),
            };
            let value = l_bound.min(gf2);
            if let Some(best) = best {
                assert!(value >= best as i64, "n={n}, node {i}: the bound {value} is below the best completion {best}");
            }
            out.write(&format!(
                "{{\"n\":{n},\"node\":{i},\"mode\":\"{mode}\",\"depth\":{},\"target\":{target},\"gf2\":{gf2},\
                 \"l\":{l_bound},\"bound\":{value},\"cut\":{},\"excess\":{},\"exact\":{},\"pricings\":{pricings},\
                 \"ms\":{:.1}}}",
                u.promoted.len(),
                value <= target,
                value - target,
                best.map_or("null".into(), |b| b.to_string()),
                t.elapsed().as_secs_f64() * 1e3,
            ));
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// check

/// A random maximal promotable set: the candidates promoted in a random order, those that can be.
fn random_promotable<const W: usize>(model: &FrameGraph, root: &FrameSet<W>, cands: &[u32], rng: &mut Rng) -> Bits<W> {
    let mut order = cands.to_vec();
    for i in (1..order.len()).rev() {
        order.swap(i, rng.below(i as u64 + 1) as usize);
    }
    let mut state = *root;
    let mut set = Bits::<W>::EMPTY;
    for &x in &order {
        if model.promote_in_place(&mut state, x) {
            set.insert(x);
        }
    }
    set
}

/// The column-sum signing of a core: merged into a fresh signing of its frame.
fn column_sum_signing(core: &Core, seed: u64) -> Signing {
    let mut s = Signing::for_cores(core.frame, Default3::Random(seed));
    assert!(merge(&mut s, core, &[]), "a core merges into a fresh signing");
    s
}

/// The checks of Theorem 14, the column-sum signings and Theorem 15 at the root of `n` (see the module docs), with
/// `samples` random sets of each kind; prints what was checked.
fn check<const W: usize>(n: u32, known: &Known, samples: usize) {
    let Some((board, p)) = setup::<W>(n) else { return };
    let model = &p.model;
    let root = &p.root;
    let cands: Vec<u32> = p.candidates.iter().collect();
    let mut rng = Rng(0x2545_F491_4F6C_DD1D ^ u64::from(n));
    let mut scratch = Scratch::<W>::new();
    // Theorem 14: the optimal promotions and random maximal promotable sets under the plain and random signings.
    let mut promotable: Vec<Bits<W>> = (0..samples).map(|_| random_promotable(model, root, &cands, &mut rng)).collect();
    let opt = known.opt(n, board.in_the_bag);
    if let Some(set) = opt.and_then(|o| optimal_promotions(n, known, &board, o)) {
        promotable.push(set);
    }
    let mut signings: Vec<Signing> = Vec::new();
    for h in 0..root.len() {
        signings.push(Signing::ones(h));
        signings.extend((0..5).map(|i| Signing::random(h, seed(n, h, i, SALT_RANDOM))));
    }
    // The converse: random unpromotable sets, made dependent by the column-sum signings of their residuals and cores.
    let (mut unpromotable, mut cores_checked) = (0, 0);
    let mut generated = 0;
    let mut tries = 0;
    while unpromotable < samples && tries < 100 * samples {
        tries += 1;
        let q: Vec<u32> = cands.iter().copied().filter(|_| rng.below(4) == 0).collect();
        if model.can_promote_all_using(&mut scratch, root, &Bits::from_slice(&q)) {
            continue;
        }
        unpromotable += 1;
        for h in 0..root.len() {
            let (movers, pool) = frame_with(model, root, &q, h);
            let stuck = residual(model, &movers, &pool);
            if stuck.is_empty() {
                continue;
            }
            let mut found = vec![core_of(model, root, h, stuck, &pool, &q)];
            found.extend(cores_of(model, root, &q, 4).into_iter().filter(|c| c.frame == h));
            for core in found {
                let s = column_sum_signing(&core, seed(n, h, cores_checked, 3));
                let m = Matroid3::build(model, root, &cands, &s);
                assert!(!m.independent(&q), "n={n}: a core's column-sum signing leaves its set independent");
                cores_checked += 1;
                if generated < 20 {
                    signings.push(s);
                    generated += 1;
                }
            }
        }
    }
    for s in &signings {
        let m = Matroid3::build(model, root, &cands, s);
        for set in &promotable {
            let members: Vec<u32> = set.iter().collect();
            assert!(m.independent(&members), "n={n}: a promotable set is dependent in frame {}", s.frame);
        }
    }
    // Theorem 15: random multipliers never bound below P*; zero multipliers give the GF(2) bound.
    let fm = frame_matroids(model, root, &p.candidates);
    let cap = p.max_total as usize;
    let gf2 = fm.bound(cap).value;
    let mut duality = 0;
    if let Some(opt) = opt {
        let ctx = Ctx::new(n, model, root, &fm, cap, opt as i64, false);
        let fam = Family::new(&ctx, signings.clone());
        let mut stats = Stats::default();
        let empty = Family::new(&ctx, Vec::new());
        let zero = evaluate(&ctx, &empty, &[], &mut stats, 1);
        assert_eq!(zero.value, gf2 * dw::SCALE, "n={n}: L at zero multipliers is not the GF(2) bound");
        for _ in 0..20 {
            let lam: Vec<Vec<f64>> = fam
                .extras
                .iter()
                .map(|ex| {
                    let r = &mut rng;
                    ex.m.elems
                        .iter()
                        .map(|&x| if r.below(10) < 3 { (r.below(2001) as f64 / 1000.0) * f64::from(x) } else { 0.0 })
                        .collect()
                })
                .collect();
            let ev = evaluate(&ctx, &fam, &lam, &mut stats, 1);
            assert!(ev.value.div_euclid(dw::SCALE) >= opt as i64, "n={n}: L below P*");
            duality += 1;
        }
    }
    println!(
        "{{\"n\":{n},\"candidates\":{},\"signings\":{},\"promotable\":{},\"unpromotable\":{unpromotable},\
         \"cores\":{cores_checked},\"duality\":{duality},\"gf2\":{gf2}}}",
        cands.len(),
        signings.len(),
        promotable.len(),
    );
}

// ---------------------------------------------------------------------------------------------------------
// bnb

/// The fields that identify a run's line of `bnb` (`r3` and `label` only where they are given).
const BNB_KEY: [&str; 7] = ["n", "variant", "policy", "k", "mode", "r3", "label"];
/// The fields that identify an audit's line.
const AUDIT_KEY: [&str; 3] = ["n", "variant", "node"];
/// The fields that identify a saved state's line (in its run's file) and a line of `replay`.
const STATE_KEY: [&str; 1] = ["index"];
const REPLAY_KEY: [&str; 4] = ["n", "host", "index", "method"];
/// The fields that identify a core in a harvest's file.
const CORE_KEY: [&str; 3] = ["frame", "members", "columns"];
/// The pricings of each method's trace in the audit (the design's horizon).
const AUDIT_K: usize = 20;
/// The resamples of a bootstrap interval.
const RESAMPLES: usize = 1000;
/// The columns of a line of the per-node log, in order (`-` for none).
const NODE_COLUMNS: [&str; 8] = ["id", "parent", "depth", "excess_gf2", "excess_tier1", "pricings", "method", "cut"];

/// A variant of `bnb` (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Variant {
    /// The GF(2) bound alone.
    V0,
    /// Tier 1 at the root fit's multipliers at every node, never refitted.
    V1,
    /// Tier 1 at the parent's state, then deflected Polyak.
    R1,
    /// Tier 1 at the parent's state, then the warm master.
    R2,
    /// R2 with node-local signings.
    R2c,
    /// R3 at the Tier-1-open nodes its policy selects (`r3-all`, `r3-top:A`), R2 at the others.
    R3,
}

/// How a variant refits.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Method {
    /// Deflected Polyak ([`refit_r1`]).
    R1,
    /// The warm master ([`refit_r2`]), with at most `local` node-local signings a frame.
    R2 { local: usize },
    /// The warm master with the exact checks and the node's separation ([`refit_r3`]).
    R3(R3),
}

impl Variant {
    const ALL: [Variant; 6] = [Variant::V0, Variant::V1, Variant::R1, Variant::R2, Variant::R2c, Variant::R3];
    /// The E3 pilot's variants, `bnb`'s default.
    const PILOT: [Variant; 5] = [Variant::V0, Variant::V1, Variant::R1, Variant::R2, Variant::R2c];

    fn name(self) -> &'static str {
        match self {
            Variant::V0 => "v0",
            Variant::V1 => "v1",
            Variant::R1 => "r1",
            Variant::R2 => "r2",
            Variant::R2c => "r2c",
            Variant::R3 => "r3",
        }
    }

    fn parse(name: &str) -> Result<Variant, String> {
        Variant::ALL.into_iter().find(|v| v.name() == name).ok_or(format!("unknown variant {name:?}"))
    }

    /// How it refits a node of GF(2) excess `excess` under `policy` (`root_gap` the root's GF(2) gap, `r3` R3's
    /// budgets), if it does.
    fn method(self, policy: Policy, excess: i64, root_gap: i64, r3: R3) -> Option<Method> {
        match self {
            Variant::V0 | Variant::V1 => None,
            Variant::R1 => Some(Method::R1),
            Variant::R2 => Some(Method::R2 { local: 0 }),
            Variant::R2c => Some(Method::R2 { local: LOCAL_PER_NODE }),
            Variant::R3 if policy.r3_at(excess, root_gap) => Some(Method::R3(r3)),
            Variant::R3 => Some(Method::R2 { local: 0 }),
        }
    }
}

impl Method {
    /// The method's name in the logs: `r1`, `r2`, `r2c` (R2 with node-local signings) or `r3`.
    fn name(self) -> &'static str {
        match self {
            Method::R1 => "r1",
            Method::R2 { local: 0 } => "r2",
            Method::R2 { .. } => "r2c",
            Method::R3(_) => "r3",
        }
    }

    /// The refit at a node from `warm`, whose Tier-1 evaluation there is `first`, with at most `k` pricings (R1's and
    /// R2's): `fam` is a family of `warm`'s signings at the node (R2 and R3 consume it), `settings` R2's (its budget
    /// in seconds).
    #[allow(clippy::too_many_arguments)]
    fn refit<const W: usize>(
        self,
        ctx: &Ctx<'_, W>,
        fam: Family,
        warm: &Warm,
        first: Eval,
        k: usize,
        settings: &Settings,
        stats: &mut Stats,
    ) -> (i64, Warm, Trace) {
        match self {
            Method::R1 => refit_r1(ctx, &fam, warm, &first, k, stats),
            Method::R2 { local } => refit_r2(ctx, fam, warm, first, k, local, settings, stats),
            Method::R3(r3) => refit_r3(ctx, fam, warm, first, &r3, stats),
        }
    }
}

/// R3's budgets and switches as a run's key and files name them: `t{T₃}-p{P₃}`, then `-noexact` and `-nocores` for
/// the switches that are off and `-coresfirst` for the reference's order.
fn r3_name(r3: &R3) -> String {
    let off = |on: bool, what: &str| if on { String::new() } else { format!("-no{what}") };
    let order = if r3.cores_first { "-coresfirst" } else { "" };
    format!("t{}-p{}{}{}{order}", r3.seconds, r3.pricings, off(r3.exact, "exact"), off(r3.cores, "cores"))
}

/// Which Tier-1-open nodes a variant refits.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Policy {
    /// Every one.
    All,
    /// Those whose GF(2) excess is at least this.
    Excess(i64),
    /// Those whose Tier-1 excess is at most this share of their GF(2) excess.
    Tier1(f64),
    /// For variant r3: every one, by R3.
    R3All,
    /// For variant r3: every one, by R3 where its GF(2) excess is at least this share of the root's GF(2) gap, by R2
    /// elsewhere.
    R3Top(f64),
}

impl Policy {
    fn parse(text: &str) -> Result<Policy, String> {
        let bad = |e: String| format!("policy {text:?}: {e}");
        match text.split_once(':') {
            None if text == "all" => Ok(Policy::All),
            None if text == "r3-all" => Ok(Policy::R3All),
            Some(("excess", e)) => e.parse().map(Policy::Excess).map_err(|e| bad(e.to_string())),
            Some(("tier1", f)) => f.parse().map(Policy::Tier1).map_err(|e| bad(e.to_string())),
            Some(("r3-top", a)) => a.parse().map(Policy::R3Top).map_err(|e| bad(e.to_string())),
            _ => Err(bad("not all, excess:E, tier1:F, r3-all or r3-top:A".into())),
        }
    }

    fn name(self) -> String {
        match self {
            Policy::All => "all".into(),
            Policy::Excess(e) => format!("excess:{e}"),
            Policy::Tier1(f) => format!("tier1:{f}"),
            Policy::R3All => "r3-all".into(),
            Policy::R3Top(a) => format!("r3-top:{a}"),
        }
    }

    /// Whether it is one of variant r3's.
    fn is_r3(self) -> bool {
        matches!(self, Policy::R3All | Policy::R3Top(_))
    }

    /// Whether a Tier-1-open node with the GF(2) bound `gf2` and the Tier-1 bound `t1` is refitted (`target` its
    /// `need − 1`).
    fn refits(self, gf2: i64, t1: i64, target: i64) -> bool {
        match self {
            Policy::All | Policy::R3All | Policy::R3Top(_) => true,
            Policy::Excess(e) => gf2 - target >= e,
            Policy::Tier1(f) => (t1 - target) as f64 <= f * (gf2 - target) as f64,
        }
    }

    /// Whether a node of GF(2) excess `excess` gets R3 (under an r3 policy), `root_gap` being the root's GF(2) gap.
    fn r3_at(self, excess: i64, root_gap: i64) -> bool {
        match self {
            Policy::R3All => true,
            Policy::R3Top(a) => excess as f64 >= a * root_gap as f64,
            _ => false,
        }
    }
}

/// When a tree's run writes its checkpoint.
#[derive(Clone, Copy, Debug)]
enum Every {
    /// When this many minutes have passed since the last.
    Minutes(f64),
    /// After every this many nodes (for the tests, which alone make it).
    #[cfg_attr(not(test), allow(dead_code))]
    Nodes(u64),
}

/// The counters and times of an extra bound over a run.
#[derive(Clone, Debug, Default)]
struct Counters {
    tier1_evals: u64,
    /// Tier-1-open nodes below the root (those `--save-states` counts).
    tier1_open: u64,
    refits: u64,
    /// Pricings of the refits after Tier 1.
    pricings_refit: u64,
    /// Node-local signings the refits opened.
    signings_local: u64,
    /// Refits that ran out of seconds, and that ended at a second breakdown of their master.
    refit_timeouts: u64,
    refit_breakdowns: u64,
    /// R3's refits, those that cut, their pricings and rounds.
    r3_refits: u64,
    r3_cut: u64,
    r3_pricings: u64,
    r3_rounds: u64,
    /// Nodes the audit selected.
    audited: u64,
    /// Times in ms: building the nodes' contexts and families, Tier 1, the refits (R3's apart as well), the audit.
    ms_builds: f64,
    ms_tier1: f64,
    ms_refit: f64,
    ms_r3: f64,
    ms_audit: f64,
}

/// Where a run writes the states of its Tier-1-open nodes (`--save-states`): every `every`-th, to `out`.
struct States {
    every: u64,
    out: Output,
}

/// The node's state as `--save-states` writes it (its run's `host` stem, its number `index` among the run's
/// Tier-1-open nodes below the root): the node (`order`, the promotions in the order made; `cands`, `need`,
/// `max_add`, `depth`, `hash`), its bounds (`gf2`, `excess_gf2`, `excess_tier1`), the state it inherited (`warm`, its
/// node-local signings among its signings) and what the run's refit made of it (`host`: its method, bound, pricings,
/// stop and the digest of its final state's JSON; null if it did not refit).
#[allow(clippy::too_many_arguments)]
fn state_line<const W: usize>(
    n: u32,
    host: &str,
    index: u64,
    v: &NodeView<'_, W>,
    warm: &Warm,
    t1: i64,
    refit: Option<(Method, i64, &Warm, &Trace)>,
) -> Json {
    let target = v.need as i64 - 1;
    let refit = refit.map_or(Json::Null, |(method, bound, state, trace)| {
        let mut pairs = vec![
            ("method", method.name().into()),
            ("bound", bound.into()),
            ("pricings", trace.pricings.into()),
            ("stop", format!("{:?}", trace.stop.expect("a refit's stop")).into()),
            ("digest", digest(&state.to_json().to_string()).into()),
        ];
        if let Method::R3(r3) = method {
            pairs.push(("r3", r3_name(&r3).into()));
        }
        Json::object(pairs)
    });
    Json::object([
        ("n", Json::from(n)),
        ("host", host.into()),
        ("index", index.into()),
        ("hash", format!("{:016x}", node_hash(&v.promoted, &v.cands)).into()),
        ("depth", v.depth.into()),
        ("order", Json::array(v.order.iter().copied())),
        ("cands", Json::array(v.cands.iter())),
        ("need", v.need.into()),
        ("max_add", v.max_add.into()),
        ("gf2", v.gf2.into()),
        ("excess_gf2", (v.gf2 - target).into()),
        ("excess_tier1", (t1 - target).into()),
        ("warm", warm.to_json()),
        ("refit", refit),
    ])
}

/// A digest of a text (FNV-1a, 64 bits) in hexadecimal: what a saved state's refit is compared by.
fn digest(text: &str) -> String {
    let h = text.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3));
    format!("{h:016x}")
}

/// The audit of an extra bound: it selects the nodes whose hash is 0 modulo `every`, gives the reference `seconds`,
/// and writes to `out`.
struct Audit<'a> {
    every: u64,
    seconds: f64,
    out: &'a mut Output,
}

/// The A1 bound in the branch and bound (design S5.2), as an extra bound: Tier 1 at the state inherited, the audit,
/// and the variant's refit where the policy says (see [`ExtraBound::bound`] below).
struct A1<'a, const W: usize> {
    n: u32,
    model: &'a FrameGraph,
    variant: Variant,
    /// The root fit's state.
    root: Rc<Warm>,
    /// The root's GF(2) gap (what `r3-top` measures a node's excess against).
    root_gap: i64,
    /// The most pricings of a refit.
    k: usize,
    policy: Policy,
    /// The refits' settings: their budget in seconds (`refit_r2` sets search mode, the pricings and the cores a frame
    /// itself).
    refit: Settings,
    /// R3's budgets and switches.
    r3: R3,
    audit: Option<Audit<'a>>,
    /// The run's name (its files' stem), the states to save and the harvest's file of cores.
    host: String,
    states: Option<States>,
    harvest: Option<Output>,
    /// The statistics of the run's Tier 1 evaluations and refits (not the audit's).
    stats: Stats,
    counters: Counters,
}

impl<'a, const W: usize> A1<'a, W> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        n: u32,
        model: &'a FrameGraph,
        variant: Variant,
        root: Rc<Warm>,
        root_gap: i64,
        o: &BnbOptions,
        audit: Option<Audit<'a>>,
        host: String,
    ) -> Self {
        let file = |dir: &Path, what: &str| make_dir(dir).join(format!("{host}.{what}.jsonl"));
        let states = o
            .save_states
            .as_deref()
            .map(|dir| States { every: o.every, out: Output::open(Some(&file(dir, "states")), &STATE_KEY) });
        let harvest = o.harvest.as_deref().map(|path| Output::open(Some(path), &CORE_KEY));
        A1 {
            n,
            model,
            variant,
            root,
            root_gap,
            k: o.k,
            policy: o.policy,
            refit: Settings { seconds: o.refit_seconds, ..Settings::default() },
            r3: o.r3,
            audit,
            host,
            states,
            harvest,
            stats: Stats::default(),
            counters: Counters::default(),
        }
    }

    /// The extra bound to hand the search: none for `v0`.
    fn extra(&mut self) -> Option<&mut Self> {
        (self.variant != Variant::V0).then_some(self)
    }

    /// The audit of a Tier-1-open node below the root, if it is on and selects the node: from the same inherited
    /// state `warm` and Tier-1 evaluation `ev` (bound `t1`), the reference (the node's own master with separation from
    /// `warm`'s multipliers, certified, within the audit's seconds) and the trace of each method over [`AUDIT_K`]
    /// pricings, each on its own family and statistics; their line to the audit's output, unless it is there.  It
    /// changes nothing in the run.
    fn audit_node(&mut self, ctx: &Ctx<'_, W>, v: &NodeView<'_, W>, warm: &Warm, ev: &Eval, t1: i64) {
        let Some(audit) = &mut self.audit else { return };
        let hash = node_hash(&v.promoted, &v.cands);
        if !hash.is_multiple_of(audit.every) {
            return;
        }
        self.counters.audited += 1;
        let id = vec![
            ("n", self.n.into()),
            ("variant", self.variant.name().into()),
            ("node", format!("{hash:016x}").into()),
        ];
        if audit.out.done(&Output::key_of(&Json::object(id.clone()), &AUDIT_KEY)) {
            return;
        }
        let t0 = Instant::now();
        let target = ctx.target;
        let r = reference(ctx, warm, audit.seconds);
        let reference = Json::object([
            ("bound", r.bound.into()),
            ("excess", (r.bound - target).into()),
            ("cut", (r.bound <= target).into()),
            ("pricings", r.stats.pricings_mi.into()),
            ("stop", format!("{:?}", r.stop).into()),
            ("ms", rounded(r.seconds * 1e3)),
        ]);
        let mut pairs = id;
        pairs.extend([
            ("depth", v.depth.into()),
            ("target", target.into()),
            ("gf2", v.gf2.into()),
            ("excess_gf2", (v.gf2 - target).into()),
            ("excess_tier1", (t1 - target).into()),
            ("reference", reference),
        ]);
        for method in [Method::R1, Method::R2 { local: 0 }, Method::R2 { local: LOCAL_PER_NODE }] {
            let t = Instant::now();
            let fam = Family::new(ctx, warm.signings.clone());
            let (_, _, trace) = method.refit(ctx, fam, warm, ev.clone(), AUDIT_K, &self.refit, &mut Stats::default());
            let last = *trace.after.last().expect("Tier 1's entry");
            let after: Vec<i64> = (0..=AUDIT_K).map(|i| trace.after.get(i).copied().unwrap_or(last) - target).collect();
            let cut_at = after.iter().position(|&e| e <= 0).map_or(Json::Null, Json::from);
            let mut fields = vec![
                ("after", Json::array(after)),
                ("cut_at", cut_at),
                ("pricings", trace.pricings.into()),
                ("ms", rounded(ms(t))),
            ];
            if let Method::R2 { .. } = method {
                fields.push(("new_signings", trace.new_signings.into()));
            }
            pairs.push((method.name(), Json::object(fields)));
        }
        audit.out.write(&Json::object(pairs).to_string());
        self.counters.ms_audit += ms(t0);
    }

    /// The refit of a Tier-1-open node below the root with `method`, from `warm` and its Tier-1 evaluation `ev`, its
    /// counters kept, the cores its separation found offered to the harvest: the bound, final state and trace.
    fn refit(&mut self, ctx: &Ctx<'_, W>, fam: Family, warm: &Warm, ev: Eval, method: Method) -> (i64, Warm, Trace) {
        let t = Instant::now();
        let (refitted, state, trace) = method.refit(ctx, fam, warm, ev, self.k, &self.refit, &mut self.stats);
        let c = &mut self.counters;
        c.ms_refit += ms(t);
        c.refits += 1;
        c.pricings_refit += trace.pricings as u64;
        c.signings_local += trace.new_signings as u64;
        c.refit_timeouts += u64::from(trace.stop == Some(Stop::Seconds));
        c.refit_breakdowns += u64::from(trace.stop == Some(Stop::Breakdown));
        if let Method::R3(_) = method {
            c.ms_r3 += ms(t);
            c.r3_refits += 1;
            c.r3_cut += u64::from(refitted <= ctx.target);
            c.r3_pricings += trace.pricings as u64;
            c.r3_rounds += trace.rounds as u64;
        }
        if let Some(out) = &mut self.harvest {
            for core in &trace.cores {
                out.write_once(&core.to_json(), &CORE_KEY);
            }
        }
        (refitted, state, trace)
    }
}

/// The reference at a node (E1n's node mode, the audit's): the certified master with separation from `warm`'s
/// signings (frozen) and multipliers, 100 Polyak steps first, within `seconds`.
fn reference<const W: usize>(ctx: &Ctx<'_, W>, warm: &Warm, seconds: f64) -> Outcome {
    bound(ctx, warm.signings.clone(), Some(&warm.lambda), true, &Settings { seconds, ..Settings::default() })
}

impl<const W: usize> ExtraBound<W> for A1<'_, W> {
    type State = Rc<Warm>;

    /// With `target = need − 1`: the state inherited (the root fit's at the root and for `v1`, else the parent's as
    /// the node takes it over, `Warm::child`); Tier 1 there; the audit, at a Tier-1-open node below the root; then,
    /// at such a node, the variant's refit if it has one and the policy selects the node (R3 or R2 for r3, by the
    /// policy); the node's state saved if it is due.  The lower of the bounds and the final state.
    fn bound(&mut self, v: &NodeView<'_, W>, parent: Option<&Rc<Warm>>) -> (i64, Rc<Warm>, NodeLog) {
        let target = v.need as i64 - 1;
        let warm = match parent {
            Some(state) if self.variant != Variant::V1 => {
                Rc::new(state.child(&v.promoted_now.to_vec(), &v.removed.to_vec(), &v.fm.elems, v.max_add))
            }
            _ => Rc::clone(&self.root),
        };
        let t = Instant::now();
        let ctx = Ctx::new(self.n, self.model, v.state, v.fm, v.max_add, target, true);
        let fam = Family::new(&ctx, warm.signings.clone());
        self.counters.ms_builds += ms(t);
        let t = Instant::now();
        let (t1, ev) = tier1(&ctx, &fam, &warm, &mut self.stats);
        self.counters.ms_tier1 += ms(t);
        self.counters.tier1_evals += 1;
        let mut log = NodeLog { excess_tier1: Some(t1 - target), ..NodeLog::default() };
        let open = t1 > target && parent.is_some();
        if !open {
            return (t1, warm, log);
        }
        self.counters.tier1_open += 1;
        self.audit_node(&ctx, v, &warm, &ev, t1);
        let method = self
            .variant
            .method(self.policy, v.gf2 - target, self.root_gap, self.r3)
            .filter(|_| self.policy.refits(v.gf2, t1, target));
        let refit = method.map(|method| (method, self.refit(&ctx, fam, &warm, ev, method)));
        let index = self.counters.tier1_open;
        if let Some(states) = self.states.as_mut().filter(|s| index.is_multiple_of(s.every)) {
            let host = refit.as_ref().map(|(method, (bound, state, trace))| (*method, *bound, state, trace));
            let line = state_line(self.n, &self.host, index, v, &warm, t1, host);
            states.out.write_once(&line, &STATE_KEY);
        }
        let Some((method, (refitted, state, trace))) = refit else {
            return (t1, warm, log);
        };
        log.pricings = trace.pricings;
        log.method = Some(method.name());
        (t1.min(refitted), Rc::new(state), log)
    }
}

/// The audit's hash of a node: splitmix64 folded over its promoted members and then its candidates as `gf3::seed`
/// folds its arguments (`seed(x, part, 0, h)` for the member `x` of part 0 or 1 and the hash `h` so far), so that it
/// depends on the node alone, not on when it is visited.
fn node_hash<const W: usize>(promoted: &Bits<W>, cands: &Bits<W>) -> u64 {
    [promoted, cands].into_iter().enumerate().fold(0, |h, (part, set)| set.iter().fold(h, |h, x| seed(x, part, 0, h)))
}

/// Milliseconds since `t`.
fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1e3
}

/// A time in ms as the lines have it, to 0.1 ms.
fn rounded(ms: f64) -> Json {
    Json::Num((ms * 10.0).round() / 10.0)
}

/// A game of `bnb`: its setup, the incumbent, the root fit and its state, and Tier 1 at the root from it.
struct Game<'g, const W: usize> {
    n: u32,
    p: &'g Prepared<W>,
    graph: &'g MatchGraph,
    /// What the search proves that nothing beats: `P*` (less a handicap in the tests).
    opt: u64,
    fit: &'g SavedFit,
    /// The root fit's state ([`Warm::from_saved`]), which every variant's search starts from.
    root: Rc<Warm>,
    /// Tier 1 at the root from the fit, as the search evaluates it (None when the GF(2) bound cuts the root).
    fit_tier1: Option<i64>,
}

impl<'g, const W: usize> Game<'g, W> {
    fn new(n: u32, p: &'g Prepared<W>, graph: &'g MatchGraph, opt: u64, fit: &'g SavedFit) -> Self {
        let root = Rc::new(Warm::from_saved(fit));
        let mut g = Game { n, p, graph, opt, fit, root, fit_tier1: None };
        let mut root = g.a1(Variant::V1, &BnbOptions::default(), None);
        let record = g.search().step(root.extra()).expect("the root");
        g.fit_tier1 = record.log.excess_tier1.map(|e| e + opt as i64);
        g
    }

    /// A search of the game from its root.
    fn search(&self) -> Search<'g, W, Rc<Warm>> {
        Search::new(self.p, self.graph, self.opt, BoundKind::Gf2)
    }

    /// The extra bound of `variant`, from the root fit, with the audit of `o` if it is on, to `audit`.
    fn a1<'a>(&self, variant: Variant, o: &BnbOptions, audit: Option<&'a mut Output>) -> A1<'a, W>
    where
        'g: 'a,
    {
        let audit = o.audit.zip(audit).map(|(every, out)| Audit { every, seconds: o.audit_seconds, out });
        let gap = self.fit.gf2 - self.opt as i64;
        let stem = self.stem(variant, o);
        A1::new(self.n, &self.p.model, variant, Rc::clone(&self.root), gap, o, audit, stem)
    }

    /// The name of a run's files: `n{N}-{variant}-{policy}-k{K}`, then R3's settings for r3 and the label if any.
    fn stem(&self, variant: Variant, o: &BnbOptions) -> String {
        let mut stem = format!("n{}-{}-{}-k{}", self.n, variant.name(), o.policy.name(), o.k);
        for part in run_id(self.n, variant, o, "").into_iter().filter(|(k, _)| matches!(*k, "r3" | "label")) {
            stem += &format!("-{}", part.1.as_str().expect("a name"));
        }
        stem
    }

    /// The fields that open a run's line (and identify its checkpoint): its key, then the game's figures.
    fn head(&self, variant: Variant, o: &BnbOptions, mode: &str) -> Vec<(&'static str, Json)> {
        let mut pairs = run_id(self.n, variant, o, mode);
        pairs.extend([
            ("g", (self.fit.gf2 - self.opt as i64).into()),
            ("opt", self.opt.into()),
            ("gf2", self.fit.gf2.into()),
            ("fit", self.fit_tier1.map_or(Json::Null, Json::from)),
            ("fit_bound", self.fit.bound.into()),
        ]);
        pairs
    }
}

/// The key fields of a run's line ([`BNB_KEY`]): `r3` (R3's settings, [`r3_name`]) for r3 alone, and `label` if
/// given.
fn run_id(n: u32, variant: Variant, o: &BnbOptions, mode: &str) -> Vec<(&'static str, Json)> {
    let mut id = vec![
        ("n", n.into()),
        ("variant", variant.name().into()),
        ("policy", o.policy.name().into()),
        ("k", o.k.into()),
        ("mode", mode.into()),
    ];
    if variant == Variant::R3 {
        id.push(("r3", r3_name(&o.r3).into()));
    }
    if let Some(label) = &o.label {
        id.push(("label", label.as_str().into()));
    }
    id
}

/// The counters and times of a run: the search's tally, the extra bound's counters and statistics, and the run's
/// time (ms, with the audit's).
#[derive(Clone, Default)]
struct Totals {
    ms: f64,
    tally: Tally,
    counters: Counters,
    stats: Stats,
}

/// A counter or a time of [`Totals`].
enum Slot<'t> {
    Count(&'t mut u64),
    /// A time in ms.
    Ms(&'t mut f64),
    Time(&'t mut Duration),
}

impl Totals {
    /// Every counter and time with its name in the run's line and whether the line has it (the others only the
    /// checkpoints): the one list that the lines, the checkpoints and the resumed runs go by.
    fn slots(&mut self) -> Vec<(&'static str, bool, Slot<'_>)> {
        let Totals { ms, tally: t, counters: c, stats: s } = self;
        vec![
            ("nodes", true, Slot::Count(&mut t.nodes)),
            ("cut_count", true, Slot::Count(&mut t.cut_by_count)),
            ("cut_gf2", true, Slot::Count(&mut t.cut_by_bound)),
            ("cut_tier1", true, Slot::Count(&mut t.cut_tier1)),
            ("cut_refit", true, Slot::Count(&mut t.cut_refit)),
            ("better", true, Slot::Count(&mut t.better)),
            ("branches", true, Slot::Count(&mut t.branches)),
            ("tier1_evals", true, Slot::Count(&mut c.tier1_evals)),
            ("tier1_open", true, Slot::Count(&mut c.tier1_open)),
            ("refits", true, Slot::Count(&mut c.refits)),
            ("pricings_refit", true, Slot::Count(&mut c.pricings_refit)),
            ("signings_local", true, Slot::Count(&mut c.signings_local)),
            ("refit_timeouts", true, Slot::Count(&mut c.refit_timeouts)),
            ("refit_breakdowns", true, Slot::Count(&mut c.refit_breakdowns)),
            ("r3_refits", true, Slot::Count(&mut c.r3_refits)),
            ("r3_cut", true, Slot::Count(&mut c.r3_cut)),
            ("r3_pricings", true, Slot::Count(&mut c.r3_pricings)),
            ("r3_rounds", true, Slot::Count(&mut c.r3_rounds)),
            ("audited", true, Slot::Count(&mut c.audited)),
            ("ms", true, Slot::Ms(ms)),
            ("ms_gf2", true, Slot::Time(&mut t.bound_time)),
            ("ms_gf2_build", true, Slot::Time(&mut t.build_time)),
            ("ms_extra", true, Slot::Time(&mut t.extra_time)),
            ("ms_builds", true, Slot::Ms(&mut c.ms_builds)),
            ("ms_tier1", true, Slot::Ms(&mut c.ms_tier1)),
            ("ms_refit", true, Slot::Ms(&mut c.ms_refit)),
            ("ms_r3", true, Slot::Ms(&mut c.ms_r3)),
            ("ms_audit", true, Slot::Ms(&mut c.ms_audit)),
            ("ms_mi", true, Slot::Ms(&mut s.ms_mi)),
            ("ms_greedy", true, Slot::Ms(&mut s.ms_greedy)),
            ("ms_simplex", true, Slot::Ms(&mut s.ms_simplex)),
            ("ms_exact", true, Slot::Ms(&mut s.ms_exact)),
            ("ms_separate", true, Slot::Ms(&mut s.ms_separate)),
            ("ms_master_build", true, Slot::Ms(&mut s.ms_build)),
            ("checks", true, Slot::Count(&mut s.checks)),
            ("checks_exact", true, Slot::Count(&mut s.checks_exact)),
            ("cuts_exact", true, Slot::Count(&mut s.cuts_exact)),
            ("breakdowns", true, Slot::Count(&mut s.breakdowns)),
            ("saturated", true, Slot::Count(&mut s.saturated)),
            ("warm", false, Slot::Count(&mut t.warm)),
            ("cold", false, Slot::Count(&mut t.cold)),
            ("augmentations", false, Slot::Count(&mut t.augmentations)),
            // The intersections' cross-checks fall on every 50th evaluation of the run.
            ("pricings_mi", false, Slot::Count(&mut s.pricings_mi)),
        ]
    }

    /// The counters and times as a line's fields: those of the line, or every one (`all`, for a checkpoint).
    fn pairs(&self, all: bool) -> Vec<(&'static str, Json)> {
        let mut copy = self.clone();
        let slots = copy.slots();
        slots
            .into_iter()
            .filter(|&(_, in_line, _)| all || in_line)
            .map(|(name, _, slot)| match slot {
                Slot::Count(v) => (name, Json::from(*v)),
                Slot::Ms(v) => (name, rounded(*v)),
                Slot::Time(d) => (name, rounded(d.as_secs_f64() * 1e3)),
            })
            .collect()
    }

    /// The totals of a checkpoint's object (every field of [`pairs`](Self::pairs)`(true)`).
    fn read(j: &Json) -> Totals {
        let mut totals = Totals::default();
        for (name, _, slot) in totals.slots() {
            let v = entry(j, name);
            let x = v.as_f64().unwrap_or_else(|| panic!("a checkpoint's {name} is not a number"));
            match slot {
                Slot::Count(c) => *c = v.as_u64().unwrap_or_else(|| panic!("a checkpoint's {name} is not a count")),
                Slot::Ms(m) => *m = x,
                Slot::Time(d) => *d = Duration::from_secs_f64(x / 1e3),
            }
        }
        totals
    }
}

/// The field `name` of a checkpoint's object.
fn entry<'j>(j: &'j Json, name: &str) -> &'j Json {
    j.get(name).unwrap_or_else(|| panic!("a checkpoint without {name:?}"))
}

/// A tree's checkpoint: the run's head (which a resumed run must have), its totals, the lines of its per-node log,
/// the distinct warm states on the stack and the stack (each entry with the index of its state).
fn checkpoint_json<const W: usize>(
    head: &[(&'static str, Json)],
    totals: &Totals,
    log_lines: Option<u64>,
    stack: &[Pending<W, Rc<Warm>>],
) -> Json {
    let mut states = Vec::new();
    let mut index: HashMap<*const Warm, usize> = HashMap::new();
    let mut entries = Vec::with_capacity(stack.len());
    let bits = |b: &Bits<W>| Json::array(b.iter());
    for e in stack {
        let state = e.extra.as_ref().map(|w| {
            *index.entry(Rc::as_ptr(w)).or_insert_with(|| {
                states.push(w.to_json());
                states.len() - 1
            })
        });
        entries.push(Json::object([
            ("promoted", bits(&e.promoted)),
            ("cands", bits(&e.cands)),
            ("parent_chosen", bits(&e.parent_chosen)),
            ("depth", e.depth.into()),
            ("parent", e.parent.map_or(Json::Null, Json::from)),
            ("promoted_now", bits(&e.promoted_now)),
            ("removed", bits(&e.removed)),
            ("order", Json::array(e.order.iter().copied())),
            ("state", state.map_or(Json::Null, Json::from)),
        ]));
    }
    Json::object([
        ("head", Json::object(head.to_vec())),
        ("log_lines", log_lines.map_or(Json::Null, Json::from)),
        ("totals", Json::object(totals.pairs(true))),
        ("states", Json::Arr(states)),
        ("stack", Json::Arr(entries)),
    ])
}

/// The stack of a checkpoint ([`checkpoint_json`]), each entry's state rebuilt from the root by promoting its path's
/// members one at a time, as the search made it.
fn restore_stack<const W: usize>(j: &Json, p: &Prepared<W>) -> Vec<Pending<W, Rc<Warm>>> {
    let states: Vec<Rc<Warm>> = entry(j, "states")
        .as_arr()
        .expect("a checkpoint's states")
        .iter()
        .map(|s| Rc::new(Warm::from_json(s).unwrap_or_else(|e| panic!("a checkpoint's state: {e}"))))
        .collect();
    let mut scratch = Scratch::<W>::new();
    let stack = entry(j, "stack").as_arr().expect("a checkpoint's stack");
    stack
        .iter()
        .map(|e| {
            let list = |name| entry(e, name).as_u32_vec().unwrap_or_else(|| panic!("{name}: not a list of numbers"));
            let bits = |name| Bits::<W>::from_slice(&list(name));
            let order = list("order");
            let state = state_along(p, &order, &mut scratch);
            let promoted = bits("promoted");
            assert_eq!(Bits::<W>::from_slice(&order), promoted, "a checkpoint's path and promotions differ");
            Pending {
                promoted,
                state,
                cands: bits("cands"),
                parent_chosen: bits("parent_chosen"),
                depth: entry(e, "depth").as_u64().expect("a checkpoint's depth") as usize,
                parent: entry(e, "parent").as_u64(),
                promoted_now: bits("promoted_now"),
                removed: bits("removed"),
                order,
                extra: entry(e, "state").as_u64().map(|i| Rc::clone(&states[i as usize])),
            }
        })
        .collect()
}

/// The state of a node of the branch and bound: the root with the members `order` promoted one at a time, as the
/// search made it ([`Pending::order`]).
fn state_along<const W: usize>(p: &Prepared<W>, order: &[u32], scratch: &mut Scratch<W>) -> FrameSet<W> {
    order.iter().fold(p.root, |state, &x| {
        p.model
            .promote_all_using(scratch, &state, &Bits::EMPTY.with(x))
            .unwrap_or_else(|| panic!("{x} on a node's path cannot be promoted"))
    })
}

/// `dir`, made if it is not there.
fn make_dir(dir: &Path) -> &Path {
    std::fs::create_dir_all(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    dir
}

/// Writes `text` to `path` through a `.tmp` beside it, renamed: an interruption leaves the old file or the new.
fn write_atomically(path: &Path, text: &str) {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    std::fs::write(&tmp, text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    std::fs::rename(&tmp, path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}

/// What became of a node, as the per-node log has it.
fn cut_name(fate: Fate) -> &'static str {
    match fate {
        Fate::Count => "count",
        Fate::Bound => "gf2",
        Fate::Tier1 => "tier1",
        Fate::Refit => "refit",
        Fate::Leaf => "leaf",
        Fate::Branched => "none",
    }
}

/// A tree's per-node log (`--log`): a text line a node ([`NODE_COLUMNS`]) in `{stem}.tmp` as the run goes, made
/// into `{stem}.jsonl` with the subtree sizes at its end.
struct NodeFile {
    tmp: PathBuf,
    jsonl: PathBuf,
    file: BufWriter<File>,
    /// The lines written.
    lines: u64,
}

impl NodeFile {
    /// The log of the run `stem` in `dir`, its `.tmp` cut to its first `lines` lines (those of the checkpoint a run
    /// resumes from; none for a new run).
    fn open(dir: &Path, stem: &str, lines: u64) -> NodeFile {
        let tmp = make_dir(dir).join(format!("{stem}.tmp"));
        let mut keep = 0;
        if lines > 0 {
            let mut reader = BufReader::new(File::open(&tmp).unwrap_or_else(|e| panic!("{}: {e}", tmp.display())));
            let mut line = Vec::new();
            for _ in 0..lines {
                line.clear();
                let read = reader.read_until(b'\n', &mut line).unwrap_or_else(|e| panic!("{}: {e}", tmp.display()));
                assert!(line.ends_with(b"\n"), "{}: fewer lines than the checkpoint's {lines}", tmp.display());
                keep += read as u64;
            }
        }
        let file = OpenOptions::new().create(true).append(true).open(&tmp);
        let file = file.unwrap_or_else(|e| panic!("{}: {e}", tmp.display()));
        file.set_len(keep).unwrap_or_else(|e| panic!("{}: {e}", tmp.display()));
        NodeFile { jsonl: dir.join(format!("{stem}.jsonl")), tmp, file: BufWriter::new(file), lines }
    }

    /// Appends the line of a node.
    fn push<const W: usize>(&mut self, r: &NodeRecord<W>) {
        let or_dash = |v: Option<i64>| v.map_or("-".to_string(), |x| x.to_string());
        let columns = [
            r.id.to_string(),
            or_dash(r.parent.map(|q| q as i64)),
            r.depth.to_string(),
            or_dash(r.excess),
            or_dash(r.log.excess_tier1),
            or_dash(r.log.method.map(|_| r.log.pricings as i64)),
            r.log.method.unwrap_or("-").to_string(),
            cut_name(r.fate).to_string(),
        ];
        writeln!(self.file, "{}", columns.join(" ")).unwrap_or_else(|e| panic!("{}: {e}", self.tmp.display()));
        self.lines += 1;
    }

    fn flush(&mut self) {
        self.file.flush().unwrap_or_else(|e| panic!("{}: {e}", self.tmp.display()));
    }

    /// Writes the `.jsonl` from the `.tmp`, each node with the size of its subtree (the ids run in the order visited
    /// and a child's exceeds its parent's, so going down the ids adds each node's size to its parent's).  Returns
    /// the `.tmp`, to be removed once the run's line is written.
    fn finish(mut self) -> PathBuf {
        self.flush();
        let tmp = &self.tmp;
        let lines = || {
            let file = File::open(tmp).unwrap_or_else(|e| panic!("{}: {e}", tmp.display()));
            BufReader::new(file).lines().map(|l| l.unwrap_or_else(|e| panic!("{}: {e}", tmp.display())))
        };
        let parents: Vec<Option<usize>> = lines()
            .enumerate()
            .map(|(i, line)| {
                let mut words = line.split(' ');
                assert_eq!(words.next(), Some(i.to_string().as_str()), "{}: ids in the order visited", tmp.display());
                words.next().and_then(|q| q.parse().ok())
            })
            .collect();
        let mut size = vec![1u64; parents.len()];
        for i in (0..parents.len()).rev() {
            if let Some(q) = parents[i] {
                let below = size[i];
                size[q] += below;
            }
        }
        let file = File::create(&self.jsonl).unwrap_or_else(|e| panic!("{}: {e}", self.jsonl.display()));
        let mut out = BufWriter::new(file);
        for (line, size) in lines().zip(size) {
            let value = |w: &str| match w {
                "-" => Json::Null,
                w => w.parse::<i64>().map_or_else(|_| Json::from(w), Json::Int),
            };
            let mut pairs: Vec<(&str, Json)> = NODE_COLUMNS.into_iter().zip(line.split(' ').map(value)).collect();
            pairs.push(("subtree", size.into()));
            writeln!(out, "{}", Json::object(pairs)).unwrap_or_else(|e| panic!("{}: {e}", self.jsonl.display()));
        }
        out.flush().unwrap_or_else(|e| panic!("{}: {e}", self.jsonl.display()));
        self.tmp
    }
}

/// The exact tree of `variant` (see the module docs): steps until the stack is empty or a budget ends, with the
/// per-node log and the checkpoints of `o`, resuming from a checkpoint it finds; then its line to `out`, and its
/// checkpoint and `.tmp` removed.  False if it was halted (`o.halt_at`) before its end.
fn run_tree<const W: usize>(
    g: &Game<'_, W>,
    variant: Variant,
    o: &BnbOptions,
    audit: Option<&mut Output>,
    out: &mut Output,
) -> bool {
    let stem = g.stem(variant, o);
    let head = g.head(variant, o, "tree");
    let mut extra = g.a1(variant, o, audit);
    let mut s = g.search();
    let checkpoint = o.checkpoint.as_deref().map(|dir| make_dir(dir).join(format!("{stem}.ckpt.json")));
    // The run's time before the checkpoint it resumes from (ms), and the lines of its log then.
    let (mut before, mut log_lines) = (0.0, None);
    if let Some(text) = checkpoint.as_ref().and_then(|path| std::fs::read_to_string(path).ok()) {
        let j = Json::parse(&text).unwrap_or_else(|e| panic!("{stem}.ckpt.json: {e}"));
        assert_eq!(entry(&j, "head"), &Json::object(head.clone()), "{stem}.ckpt.json: a checkpoint of another run");
        let totals = Totals::read(entry(&j, "totals"));
        s.stack = restore_stack(&j, g.p);
        (before, s.tally, extra.counters, extra.stats) = (totals.ms, totals.tally, totals.counters, totals.stats);
        log_lines = entry(&j, "log_lines").as_u64();
        assert_eq!(log_lines.is_some(), o.log.is_some(), "{stem}: a run resumes with the --log it began with");
        eprintln!("n={} {}: resumed at {} nodes", g.n, variant.name(), s.tally.nodes);
    }
    let mut log = o.log.as_deref().map(|dir| NodeFile::open(dir, &stem, log_lines.unwrap_or(0)));
    let (t0, mut saved) = (Instant::now(), Instant::now());
    let totals = |s: &Search<'_, W, Rc<Warm>>, extra: &A1<'_, W>| Totals {
        ms: before + ms(t0),
        tally: s.tally.clone(),
        counters: extra.counters.clone(),
        stats: extra.stats,
    };
    while !s.stack.is_empty() && o.within(s.tally.nodes, before + ms(t0), &extra.counters) {
        let record = s.step(extra.extra()).expect("a node on the stack");
        if let Some(log) = &mut log {
            log.push(&record);
        }
        if o.halt_at == Some(s.tally.nodes) {
            return false;
        }
        let due = match o.checkpoint_every {
            Every::Minutes(m) => saved.elapsed().as_secs_f64() >= 60.0 * m,
            Every::Nodes(k) => s.tally.nodes.is_multiple_of(k),
        };
        if let Some(path) = checkpoint.as_ref().filter(|_| due) {
            if let Some(log) = &mut log {
                log.flush();
            }
            let j = checkpoint_json(&head, &totals(&s, &extra), log.as_ref().map(|l| l.lines), &s.stack);
            write_atomically(path, &j.to_string());
            saved = Instant::now();
        }
    }
    let tmp = log.map(NodeFile::finish);
    let mut pairs = head;
    pairs.push(("completed", s.stack.is_empty().into()));
    pairs.extend(totals(&s, &extra).pairs(false));
    let line = Json::object(pairs).to_string();
    out.write(&line);
    for path in checkpoint.iter().chain(&tmp).filter(|path| path.exists()) {
        std::fs::remove_file(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
    assert_eq!(s.tally.better, 0, "a completion better than P*: {line}");
    true
}

/// The generator of dive `i` of a Knuth run of `n` with the seed `s`: the same for every variant, so that the
/// variants follow the same dives (a dive's path depends only on the GF(2) children).
fn dive_rng(s: u64, n: u32, i: usize) -> Rng {
    Rng(seed(n, 0, i, s) | 1)
}

/// The generator of the bootstrap of a Knuth run of `n` with the seed `s`.
fn bootstrap_rng(s: u64, n: u32) -> Rng {
    Rng(seed(n, 1, 0, s) | 1)
}

/// Knuth's estimator for `variant` (see the module docs): `dives` paired dives, or those a budget allows, each in the
/// dives' log under `--log` as it ends (a resumed run skips the dives there), then the run's line.
fn run_knuth<const W: usize>(
    g: &Game<'_, W>,
    variant: Variant,
    o: &BnbOptions,
    dives: usize,
    audit: Option<&mut Output>,
    out: &mut Output,
) {
    let mut extra = g.a1(variant, o, audit);
    let mut s = g.search();
    let path = o.log.as_deref().map(|dir| make_dir(dir).join(format!("{}-knuth.jsonl", g.stem(variant, o))));
    let mut done: HashMap<usize, (f64, f64)> = HashMap::new();
    let text = path.as_ref().and_then(|path| std::fs::read_to_string(path).ok()).unwrap_or_default();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let j = Json::parse(line).unwrap_or_else(|e| panic!("{}-knuth.jsonl: {e}", g.stem(variant, o)));
        let number = |name| j.get(name).and_then(Json::as_f64).unwrap_or_else(|| panic!("a dive without {name}"));
        done.insert(number("dive") as usize, (number("gf2"), number("a1")));
    }
    let mut log = path.as_deref().map(|path| Output::open(Some(path), &["dive"]));
    let t0 = Instant::now();
    for i in 0..dives {
        if done.contains_key(&i) {
            continue;
        }
        if !o.within(s.tally.nodes, ms(t0), &extra.counters) {
            break;
        }
        let t = Instant::now();
        let mut rng = dive_rng(o.seed, g.n, i);
        let depth = o.max_depth.unwrap_or(usize::MAX);
        let (gf2, a1) = s.dive_to(extra.extra(), &mut |c| rng.below(c.len() as u64) as usize, depth);
        if let Some(log) = &mut log {
            let dive = [("dive", i.into()), ("gf2", gf2.into()), ("a1", a1.into()), ("ms", rounded(ms(t)))];
            log.write(&Json::object(dive).to_string());
        }
        done.insert(i, (gf2, a1));
    }
    let values: Vec<(f64, f64)> = (0..dives).filter_map(|i| done.get(&i).copied()).collect();
    let estimate = Estimate::of(&values, o.ci, &mut bootstrap_rng(o.seed, g.n));
    let totals = Totals { ms: ms(t0), tally: s.tally.clone(), counters: extra.counters.clone(), stats: extra.stats };
    let mut pairs = g.head(variant, o, "knuth");
    pairs.extend([("dives", values.len().into()), ("seed", o.seed.into()), ("ci", o.ci.into())]);
    pairs.push(("max_depth", o.max_depth.map_or(Json::Null, Json::from)));
    pairs.extend(estimate.pairs());
    pairs.push(("completed", (values.len() == dives).into()));
    pairs.extend(totals.pairs(false));
    let line = Json::object(pairs).to_string();
    out.write(&line);
    assert_eq!(s.tally.better, 0, "a completion better than P*: {line}");
}

/// A sample's mean and median, and the bootstrap interval of its mean.
struct Spread {
    mean: f64,
    median: f64,
    lo: f64,
    hi: f64,
}

/// Knuth's estimates from paired dives: of the GF(2) tree, of the A1 tree, and the ratio of their means.
struct Estimate {
    gf2: Spread,
    a1: Spread,
    ratio: f64,
    ratio_lo: f64,
    ratio_hi: f64,
}

impl Estimate {
    /// The estimates of `values` (per dive, its GF(2) and its A1 estimate), with percentile intervals at level `ci`
    /// from [`RESAMPLES`] resamples of the dives drawn by `rng` (a dive's two estimates together).
    fn of(values: &[(f64, f64)], ci: f64, rng: &mut Rng) -> Estimate {
        let m = values.len();
        let mut boot: [Vec<f64>; 3] = Default::default();
        if m > 0 {
            for _ in 0..RESAMPLES {
                let (mut a, mut b) = (0.0, 0.0);
                for _ in 0..m {
                    let (x, y) = values[rng.below(m as u64) as usize];
                    a += x;
                    b += y;
                }
                boot[0].push(a / m as f64);
                boot[1].push(b / m as f64);
                boot[2].push(a / b);
            }
        }
        for b in &mut boot {
            b.sort_by(f64::total_cmp);
        }
        let (lo, hi) = ((1.0 - ci) / 2.0, (1.0 + ci) / 2.0);
        let spread = |part: fn(&(f64, f64)) -> f64, boot: &[f64]| {
            let mut sorted: Vec<f64> = values.iter().map(part).collect();
            sorted.sort_by(f64::total_cmp);
            let mean = sorted.iter().sum::<f64>() / m as f64;
            Spread { mean, median: quantile(&sorted, 0.5), lo: quantile(boot, lo), hi: quantile(boot, hi) }
        };
        let (gf2, a1) = (spread(|v| v.0, &boot[0]), spread(|v| v.1, &boot[1]));
        Estimate {
            ratio: gf2.mean / a1.mean,
            ratio_lo: quantile(&boot[2], lo),
            ratio_hi: quantile(&boot[2], hi),
            gf2,
            a1,
        }
    }

    /// The estimates as a line's fields.
    fn pairs(&self) -> Vec<(&'static str, Json)> {
        let (g, a) = (&self.gf2, &self.a1);
        [
            ("gf2_mean", g.mean),
            ("gf2_median", g.median),
            ("gf2_lo", g.lo),
            ("gf2_hi", g.hi),
            ("a1_mean", a.mean),
            ("a1_median", a.median),
            ("a1_lo", a.lo),
            ("a1_hi", a.hi),
            ("ratio", self.ratio),
            ("ratio_lo", self.ratio_lo),
            ("ratio_hi", self.ratio_hi),
        ]
        .into_iter()
        .map(|(name, x)| (name, Json::Num(x)))
        .collect()
    }
}

/// The `q`-quantile of `sorted` (linear between its order statistics); NaN if it is empty.
fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let x = q * (sorted.len() - 1) as f64;
    let i = x.floor() as usize;
    let j = (i + 1).min(sorted.len() - 1);
    sorted[i] + x.fract() * (sorted[j] - sorted[i])
}

/// The root fit of `n` for `bnb`: read from `path` if it is there (a directory holding `n{N}.json`, unless `path`
/// names a `.json` file; its game asserted), else the main arm's fit with `settings`, written there if a path is
/// given.
fn root_fit<const W: usize>(
    n: u32,
    p: &Prepared<W>,
    opt: u64,
    path: Option<&Path>,
    frames: &[usize],
    settings: &Settings,
) -> SavedFit {
    let file = path.map(|f| {
        if f.is_dir() || f.extension().is_none_or(|e| e != "json") {
            f.join(format!("n{n}.json"))
        } else {
            f.to_path_buf()
        }
    });
    at_root(n, p, opt, |ctx, gf2| {
        if let Some(f) = file.as_ref().filter(|f| f.exists()) {
            let fit = SavedFit::load(f).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!((fit.n, fit.opt, fit.gf2), (n, opt, gf2), "{}: the fit of another game", f.display());
            return fit;
        }
        let fit = SavedFit::of(n, opt, gf2, settings.mode.name(), &fit_outcome(ctx, frames, settings));
        if let Some(f) = &file {
            fit.save(f).unwrap_or_else(|e| panic!("{e}"));
        }
        fit
    })
}

/// The `bnb` command for the game `n`: the root fit (if a run is to be made), then each variant's run not in `out`.
fn bnb<const W: usize>(n: u32, known: &Known, o: &Options, out: &mut Output, mut audit: Option<&mut Output>) {
    let b = &o.bnb;
    let Some((board, p)) = setup::<W>(n) else { return };
    let Some(opt) = known.opt(n, board.in_the_bag) else {
        eprintln!("n={n}: no known optimum, skipped");
        return;
    };
    let mode = if b.knuth.is_some() { "knuth" } else { "tree" };
    let done = |v: Variant| out.done(&Output::key_of(&Json::object(run_id(n, v, b, mode)), &BNB_KEY));
    let todo: Vec<Variant> = b.variants.iter().copied().filter(|&v| !done(v)).collect();
    if todo.is_empty() {
        return;
    }
    let t = Instant::now();
    let settings = Settings { seconds: b.fit_seconds, ..o.settings.clone() };
    let fit = root_fit(n, &p, opt, b.fit.as_deref(), &o.frames, &settings);
    let graph = MatchGraph::new(&p.model, &p.root);
    let g = Game::new(n, &p, &graph, opt, &fit);
    eprintln!(
        "n={n}: P* {opt}, GF(2) {}, the root fit's bound {} and Tier 1 {:?}, {:.1} s",
        fit.gf2,
        fit.bound,
        g.fit_tier1,
        t.elapsed().as_secs_f64()
    );
    for variant in todo {
        let t = Instant::now();
        match b.knuth {
            Some(dives) => run_knuth(&g, variant, b, dives, audit.as_deref_mut(), out),
            None => {
                run_tree(&g, variant, b, audit.as_deref_mut(), out);
            }
        }
        eprintln!("n={n} {}: {:.1} s", variant.name(), t.elapsed().as_secs_f64());
    }
}

// ---------------------------------------------------------------------------------------------------------
// replay

/// A method of `replay` (see the module docs), with its name as given.
enum Probe {
    /// A refit.
    Refit(Method, String),
    /// The reference, within these seconds.
    Reference(f64, String),
}

impl Probe {
    /// The method named `text`: `r1`, `r2`, `r2c` (with `o.k` pricings), `r3:T`, `r3x:T` (R3 without its separation:
    /// R2 and the exact checks), `r3c:T` (R3 without its exact checks: R2 and the separation) and `r3s:T` (R3 in the
    /// reference's order, the separation first), each with `T₃ = T` seconds (`o.r3`'s if not given) and `o.r3`'s
    /// pricings, or `ref:T`, the reference within `T` seconds (10 if not given).
    fn parse(text: &str, o: &BnbOptions) -> Result<Probe, String> {
        let (kind, seconds) = match text.split_once(':') {
            Some((kind, t)) => (kind, Some(t.parse::<f64>().map_err(|e| format!("method {text:?}: {e}"))?)),
            None => (text, None),
        };
        let r3 = |exact, cores, cores_first| {
            Method::R3(R3 { seconds: seconds.unwrap_or(o.r3.seconds), exact, cores, cores_first, ..o.r3 })
        };
        let method = match kind {
            "r1" => Method::R1,
            "r2" => Method::R2 { local: 0 },
            "r2c" => Method::R2 { local: LOCAL_PER_NODE },
            "r3" => r3(true, true, false),
            "r3x" => r3(true, false, false),
            "r3c" => r3(false, true, false),
            "r3s" => r3(true, true, true),
            "ref" => return Ok(Probe::Reference(seconds.unwrap_or(10.0), text.to_string())),
            _ => return Err(format!("unknown method {text:?}: not r1, r2, r2c, r3:T, r3x:T, r3c:T, r3s:T or ref:T")),
        };
        Ok(Probe::Refit(method, text.to_string()))
    }
}

/// The files of saved states that `paths` name: a file itself, a directory its `*.states.jsonl`, sorted.
fn state_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            let entries = std::fs::read_dir(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let mut found: Vec<PathBuf> = entries
                .map(|e| e.unwrap_or_else(|e| panic!("{}: {e}", path.display())).path())
                .filter(|p| p.to_str().is_some_and(|s| s.ends_with(".states.jsonl")))
                .collect();
            found.sort();
            files.extend(found);
        } else {
            files.push(path.clone());
        }
    }
    files
}

/// `replay` of the states in the file `path` (`bnb --save-states`'s lines, of one game): each state's node made
/// again, its GF(2) bound checked against the line's, Tier 1 at its inherited state, then each of `probes` from that
/// state, on its own statistics; a line per state and probe to `out` (see the module docs).
fn replay(path: &Path, known: &Known, probes: &[Probe], o: &BnbOptions, out: &mut Output) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let lines: Vec<Json> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| Json::parse(l).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
        .collect();
    let Some(n) = lines.first().map(|j| entry(j, "n").as_u32().expect("a state's n")) else { return };
    taxman::with_width!(n, W => replay_states::<W>(n, &lines, known, probes, o, out));
}

fn replay_states<const W: usize>(
    n: u32,
    lines: &[Json],
    known: &Known,
    probes: &[Probe],
    o: &BnbOptions,
    out: &mut Output,
) {
    let (board, p) = setup::<W>(n).expect("a game with candidates");
    let opt = known.opt(n, board.in_the_bag).expect("a known optimum");
    let refit = Settings { seconds: o.refit_seconds, ..Settings::default() };
    let mut scratch = Scratch::<W>::new();
    for j in lines {
        assert_eq!(entry(j, "n").as_u32(), Some(n), "the states of one game a file");
        let host = entry(j, "host").as_str().expect("a state's host").to_string();
        let index = entry(j, "index").as_u64().expect("a state's index");
        let id = |method: &str| -> Vec<(&'static str, Json)> {
            vec![("n", n.into()), ("host", host.as_str().into()), ("index", index.into()), ("method", method.into())]
        };
        let name = |probe: &Probe| match probe {
            Probe::Refit(_, name) | Probe::Reference(_, name) => name.clone(),
        };
        let todo: Vec<&Probe> =
            probes.iter().filter(|q| !out.done(&Output::key_of(&Json::object(id(&name(q))), &REPLAY_KEY))).collect();
        if todo.is_empty() {
            continue;
        }
        let number = |name: &str| entry(j, name).as_i64().unwrap_or_else(|| panic!("a state's {name}"));
        let order = entry(j, "order").as_u32_vec().expect("a state's order");
        let cands = Bits::<W>::from_slice(&entry(j, "cands").as_u32_vec().expect("a state's candidates"));
        let warm = Warm::from_json(entry(j, "warm")).unwrap_or_else(|e| panic!("a state's warm state: {e}"));
        let (need, max_add) = (number("need"), number("max_add") as usize);
        let state = state_along(&p, &order, &mut scratch);
        let t = Instant::now();
        let fm = frame_matroids(&p.model, &state, &cands);
        let gf2 = fm.bound(max_add).value;
        assert_eq!(gf2, number("gf2"), "n={n} {host} state {index}: the GF(2) bound made again differs");
        let target = need - 1;
        assert!(Bits::<W>::from_slice(&order).sum() as i64 + need == opt as i64 + 1, "n={n}: a state of another P*");
        let ctx = Ctx::new(n, &p.model, &state, &fm, max_add, target, true);
        let fam = || Family::new(&ctx, warm.signings.clone());
        let (t1, ev) = tier1(&ctx, &fam(), &warm, &mut Stats::default());
        assert_eq!(t1 - target, number("excess_tier1"), "n={n} {host} state {index}: Tier 1 made again differs");
        let ms_node = ms(t);
        let host_refit = entry(j, "refit");
        let host_of = |field: &str| host_refit.get(field).and_then(Json::as_str).map(str::to_string);
        for probe in todo {
            let t = Instant::now();
            let mut stats = Stats::default();
            let (bound, fields) = match probe {
                Probe::Refit(method, _) => {
                    let (bound, state, trace) = method.refit(&ctx, fam(), &warm, ev.clone(), o.k, &refit, &mut stats);
                    let r3 = match method {
                        Method::R3(r3) => Some(r3_name(r3)),
                        _ => None,
                    };
                    let same =
                        (host_of("method").as_deref() == Some(method.name()) && host_of("r3") == r3).then(|| {
                            host_refit.get("bound").and_then(Json::as_i64) == Some(t1.min(bound))
                                && host_of("digest") == Some(digest(&state.to_json().to_string()))
                        });
                    let fields = vec![
                        ("pricings", trace.pricings.into()),
                        ("rounds", trace.rounds.into()),
                        ("new_signings", trace.new_signings.into()),
                        ("cores", trace.cores.len().into()),
                        ("stop", format!("{:?}", trace.stop.expect("a refit's stop")).into()),
                        ("host_same", same.map_or(Json::Null, Json::from)),
                    ];
                    (t1.min(bound), fields)
                }
                Probe::Reference(seconds, _) => {
                    let r = reference(&ctx, &warm, *seconds);
                    stats = r.stats;
                    let fields = vec![
                        ("pricings", r.stats.pricings_mi.into()),
                        ("rounds", r.rounds.into()),
                        ("new_signings", (r.family.extras.len() - warm.signings.len()).into()),
                        ("cores", r.cores.len().into()),
                        ("stop", format!("{:?}", r.stop).into()),
                        ("host_same", Json::Null),
                    ];
                    (t1.min(r.bound), fields)
                }
            };
            let mut pairs = id(&name(probe));
            pairs.extend([
                ("depth", entry(j, "depth").clone()),
                ("excess_gf2", number("excess_gf2").into()),
                ("excess_tier1", (t1 - target).into()),
                ("bound", bound.into()),
                ("excess", (bound - target).into()),
                ("cut", (bound <= target).into()),
            ]);
            pairs.extend(fields);
            pairs.extend([
                ("cuts_exact", stats.cuts_exact.into()),
                ("checks_exact", stats.checks_exact.into()),
                ("breakdowns", stats.breakdowns.into()),
                ("ms", rounded(ms(t))),
                ("ms_node", rounded(ms_node)),
                ("ms_pricing", rounded(stats.ms_mi + stats.ms_greedy)),
                ("ms_simplex", rounded(stats.ms_simplex)),
                ("ms_exact", rounded(stats.ms_exact)),
                ("ms_separate", rounded(stats.ms_separate)),
                ("ms_build", rounded(stats.ms_build)),
            ]);
            out.write(&Json::object(pairs).to_string());
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// fit

/// The cores of a harvest's file (`bnb --harvest`), in its order.
fn read_cores(path: &Path) -> Vec<Core> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let parse = |l: &str| Json::parse(l).map_err(|e| e.to_string()).and_then(|j| Core::from_json(&j));
    text.lines().filter(|l| !l.trim().is_empty()).map(|l| parse(l).unwrap_or_else(|e| panic!("{e}"))).collect()
}

/// The signings of `fit` that a fit at a root of `frames` frames carries over, ascending: those of the frames it has;
/// with `prune` (`Some(k)`), only those with a positive multiplier, at most `k` a frame, those of the largest sums of
/// multipliers (design T3 and U5: the family pruned to what the multipliers use).
fn carried(fit: &dw::Fit, frames: usize, prune: Option<usize>) -> Vec<usize> {
    let sum = |i: usize| fit.lambda[i].values().sum::<f64>();
    let mut kept: Vec<usize> = (0..fit.signings.len()).filter(|&i| fit.signings[i].frame < frames).collect();
    if let Some(most) = prune {
        kept.retain(|&i| sum(i) > 0.0);
        kept.sort_by(|&a, &b| sum(b).total_cmp(&sum(a)).then(a.cmp(&b)));
        let mut per_frame = vec![0; frames];
        kept.retain(|&i| {
            per_frame[fit.signings[i].frame] += 1;
            per_frame[fit.signings[i].frame] <= most
        });
        kept.sort_unstable();
    }
    kept
}

/// The `fit` command for the game `n` (see the module docs): the main arm's fit at the root, from the family of the
/// fit `--start` (the signings [`carried`], frozen, their multipliers by candidate number) or from the main arm's
/// signings, with the cores of `--offer` placed first, certified with separation within the budget; saved to
/// `--save-fit` (a `.json` file, or a directory for `n{N}.json`), its line to `out`.
fn fit_game<const W: usize>(n: u32, known: &Known, o: &Options, out: &mut Output) {
    let label = o.bnb.label.clone().unwrap_or_else(|| "own".into());
    let id = [("n", Json::from(n)), ("label", Json::from(label.as_str()))];
    if out.done(&Output::key_of(&Json::object(id.clone()), &["n", "label"])) {
        return;
    }
    let Some((board, p)) = setup::<W>(n) else { return };
    let Some(opt) = known.opt(n, board.in_the_bag) else {
        eprintln!("n={n}: no known optimum, skipped");
        return;
    };
    let start = o.start.as_deref().map(|f| SavedFit::load(f).unwrap_or_else(|e| panic!("{e}")));
    let offer = o.offer.as_deref().map(read_cores).unwrap_or_default();
    let optimal = optimal_promotions(n, known, &board, opt);
    at_root(n, &p, opt, |ctx, gf2| {
        let frames = ctx.state.len();
        let (signings, lambda): (Vec<Signing>, Option<Vec<HashMap<u32, f64>>>) = match &start {
            Some(s) => {
                let kept = carried(&s.fit, frames, o.prune);
                let signings = kept.iter().map(|&i| dw::frozen(&s.fit.signings[i])).collect();
                (signings, Some(kept.iter().map(|&i| s.fit.lambda[i].clone()).collect()))
            }
            None => (arm("f1cores", n, frames, &o.frames).0, None),
        };
        let carried = start.as_ref().map(|_| {
            let fit = dw::Fit { signings: signings.clone(), lambda: lambda.clone().unwrap_or_default() };
            fit.bound(ctx, &mut Stats::default())
        });
        let (carried_signings, offered) = (start.as_ref().map(|_| signings.len()), offer.len());
        let r = bound_offered(ctx, signings, lambda.as_deref(), offer.clone(), true, &o.settings);
        assert!(r.bound >= opt as i64, "n={n}: the fit's bound {} is below P* = {opt}", r.bound);
        if let Some(set) = &optimal {
            assert_independent(n, &r.family, set, "the optimal promotions");
            let fit_family = Family::new(ctx, r.fit.signings.clone());
            assert_independent(n, &fit_family, set, "the optimal promotions");
        }
        let fit = SavedFit::of(n, opt, gf2, o.settings.mode.name(), &r);
        if let Some(path) = &o.save_fit {
            let file = if path.extension().is_some_and(|e| e == "json") {
                path.clone()
            } else {
                path.join(format!("n{n}.json"))
            };
            fit.save(&file).unwrap_or_else(|e| panic!("{e}"));
        }
        let offered_placed = r.cores[..offered.min(r.cores.len())].iter().filter(|f| f.assigned.is_some()).count();
        let share = |b: i64| if gf2 > opt as i64 { (gf2 - b) as f64 / (gf2 - opt as i64) as f64 } else { 0.0 };
        let path_name = |p: &Option<PathBuf>| p.as_ref().map_or(Json::Null, |p| Json::from(p.display().to_string()));
        let mut pairs: Vec<(&str, Json)> = id.to_vec();
        pairs.extend([
            ("start", path_name(&o.start)),
            ("offer", path_name(&o.offer)),
            ("opt", opt.into()),
            ("gf2", gf2.into()),
            ("bound", r.bound.into()),
            ("share", Json::Num(share(r.bound))),
            ("carried", carried.map_or(Json::Null, Json::from)),
            ("carried_share", carried.map_or(Json::Null, |b| Json::Num(share(b)))),
            ("carried_signings", carried_signings.map_or(Json::Null, Json::from)),
            ("offered", offered.into()),
            ("offered_placed", offered_placed.into()),
            ("signings", r.family.extras.len().into()),
            ("fit_signings", r.fit.signings.len().into()),
            ("cores_found", r.cores.len().into()),
            ("cores_assigned", r.cores_assigned.into()),
            ("converged", (r.lb > 0.0 && r.l_best - r.lb <= 0.01 * (gf2 - opt as i64) as f64).into()),
            ("stop", format!("{:?}", r.stop).into()),
            ("rounds", r.rounds.into()),
            ("seconds", rounded(r.seconds)),
        ]);
        out.write(&Json::object(pairs).to_string());
        eprintln!(
            "n={n} {label}: bound {} (share {:.3}, carried {carried:?}), {:.1} s",
            r.bound,
            share(r.bound),
            r.seconds
        );
    });
}

// ---------------------------------------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dw::cut_rank;
    use crate::frames::FrameMatroids;
    use crate::lp::{Lp, RowKind, Status};
    use crate::meet::{Reduced, Span};

    /// Every promotable set of a board with few candidates, by exhaustion.
    fn all_promotable<const W: usize>(model: &FrameGraph, root: &FrameSet<W>, cands: &[u32]) -> Vec<Bits<W>> {
        let mut out = Vec::new();
        let mut scratch = Scratch::<W>::new();
        let mut stack = vec![(*root, Bits::<W>::EMPTY, 0usize)];
        while let Some((state, set, from)) = stack.pop() {
            out.push(set);
            for (i, &x) in cands.iter().enumerate().skip(from) {
                if let Some(next) = model.promote_all_using(&mut scratch, &state, &Bits::EMPTY.with(x)) {
                    stack.push((next, set.with(x), i + 1));
                }
            }
        }
        out
    }

    fn signings_of(n: u32, len: usize) -> Vec<Signing> {
        let mut out = Vec::new();
        for h in 0..len {
            out.push(Signing::ones(h));
            out.extend((0..5).map(|i| Signing::random(h, seed(n, h, i, SALT_RANDOM))));
        }
        out
    }

    /// The study's test 3: Theorem 14 at every root with at most 22 candidates, by exhaustion, and at
    /// the roots of 137-400 for the optimal promotions and 100 random maximal promotable sets.
    #[test]
    fn promotable_sets_are_independent_in_every_signed_matroid() {
        let known = Known::load(&[], None).expect("the built-in optimum table");
        let (mut boards, mut sets) = (0, 0);
        for n in 2..=400u32 {
            let Some((board, p)) = setup::<8>(n) else { continue };
            let cands: Vec<u32> = p.candidates.iter().collect();
            let promotable: Vec<Bits<8>> = if cands.len() <= 22 {
                all_promotable(&p.model, &p.root, &cands)
            } else if !is_prime(n) && n >= 137 {
                let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ u64::from(n));
                let mut v: Vec<Bits<8>> =
                    (0..100).map(|_| random_promotable(&p.model, &p.root, &cands, &mut rng)).collect();
                let opt = known.opt(n, board.in_the_bag).expect("a built-in optimum");
                v.push(optimal_promotions(n, &known, &board, opt).expect("the optimal promotions"));
                v
            } else {
                continue;
            };
            boards += 1;
            for s in signings_of(n, p.root.len()) {
                let m = Matroid3::build(&p.model, &p.root, &cands, &s);
                for set in &promotable {
                    let members: Vec<u32> = set.iter().collect();
                    assert!(m.independent(&members), "n={n}: {members:?} dependent in frame {}", s.frame);
                    sets += 1;
                }
            }
        }
        eprintln!("test 3: {boards} boards, {sets} (set, matroid) pairs");
        assert!(boards > 200);
    }

    /// The study's test 6: at every non-prime root of 2-400, 20 random multipliers (30% of the coordinates nonzero,
    /// uniform in [0, 2x]) never give a bound below `P*`; at zero multipliers `L` is the GF(2) bound (no family, or a
    /// family without loops), and never above it.
    #[test]
    fn weak_duality() {
        let known = Known::load(&[], None).expect("the built-in optimum table");
        let (mut roots, mut evals) = (0, 0);
        for n in (2..=400u32).filter(|&n| !is_prime(n)) {
            let Some((board, p)) = setup::<8>(n) else { continue };
            let Some(opt) = known.opt(n, board.in_the_bag) else { continue };
            let fm = frame_matroids(&p.model, &p.root, &p.candidates);
            let cap = p.max_total as usize;
            let gf2 = fm.bound(cap).value;
            let ctx = Ctx::new(n, &p.model, &p.root, &fm, cap, opt as i64, false);
            let mut stats = Stats::default();
            let empty = Family::new(&ctx, Vec::new());
            assert_eq!(evaluate(&ctx, &empty, &[], &mut stats, 1).value, gf2 * dw::SCALE, "n={n}");
            let fam = Family::new(&ctx, signings_of(n, p.root.len()));
            let zero = evaluate(&ctx, &fam, &fam_zeros(&fam), &mut stats, 1).value;
            assert!(zero <= gf2 * dw::SCALE, "n={n}");
            if fam.loops() == 0 {
                assert_eq!(zero, gf2 * dw::SCALE, "n={n}");
            }
            let mut rng = Rng(0xD1B5_4A32_D192_ED03 ^ u64::from(n));
            for _ in 0..20 {
                let lam: Vec<Vec<f64>> = fam
                    .extras
                    .iter()
                    .map(|ex| {
                        ex.m.elems
                            .iter()
                            .map(|&x| {
                                if rng.below(10) < 3 {
                                    (rng.below(1 << 20) as f64 / (1 << 20) as f64) * 2.0 * f64::from(x)
                                } else {
                                    0.0
                                }
                            })
                            .collect()
                    })
                    .collect();
                let v = evaluate(&ctx, &fam, &lam, &mut stats, 1).value;
                assert!(v.div_euclid(dw::SCALE) >= opt as i64, "n={n}: L = {v} below P* = {opt}");
                evals += 1;
            }
            roots += 1;
        }
        eprintln!("test 6: {roots} roots, {evals} evaluations");
        assert!(roots > 250);
    }

    fn fam_zeros(fam: &Family) -> Vec<Vec<f64>> {
        fam.extras.iter().map(|ex| vec![0.0; ex.elem.len()]).collect()
    }

    /// Whether `set` (element indices) is independent in a GF(2) block matroid.
    fn independent_gf2(m: &crate::meet::BlockMatroid, set: &[usize]) -> bool {
        let mut span = Span::new(m);
        set.iter().all(|&e| {
            let free = matches!(span.reduce(m, e), Reduced::Free);
            if free {
                span.insert(m, e);
            }
            free
        })
    }

    /// The study's test 7: on boards with at most 16 candidates, the exact LP (every common independent set of `A`
    /// and `B` of at most `cap` elements and every independent set of each extra matroid as a column) against the
    /// column generation: the certified lower bound and the lowest `L` bracket the LP, and both are within 10⁻⁵ of it,
    /// relatively (run until nothing prices out; the pricing is at multipliers rounded to the grid 1/S, so a column is
    /// missed only by the rounding, and the perturbations of the rows move the point by 10⁻⁶ a row).
    #[test]
    fn column_generation_reaches_the_exact_lp() {
        let (mut instances, mut tighter) = (0, 0);
        // Every board with 4-16 candidates.
        for n in 2..=200u32 {
            let Some((_, p)) = setup::<8>(n) else { continue };
            if (4..=16).contains(&p.candidates.len()) {
                tighter += usize::from(exact_against_generated(n, &p, &p.candidates));
                instances += 1;
            }
        }
        // Their GF(2) bounds are their optima, so the extra matroids cannot cut: also boards of 300-700 restricted to
        // at most 12 candidates, those of the cores of the GF(2) bound's heaviest set first (its members, then the
        // other candidates in its frames).
        for n in (300..=700u32).filter(|&n| !is_prime(n)).step_by(5) {
            let Some((_, p)) = setup::<16>(n) else { continue };
            let fm = frame_matroids(&p.model, &p.root, &p.candidates);
            let heaviest: Vec<u32> = fm.chosen::<16>(&fm.bound(p.max_total as usize)).iter().collect();
            let cores = cores_of(&p.model, &p.root, &heaviest, 1);
            let Some(core) = cores.iter().min_by_key(|c| c.promoted_in.len() + c.promoted_out.len()) else { continue };
            let mut chosen: Vec<u32> = core.promoted_in.iter().chain(&core.promoted_out).copied().collect();
            let frames = [core.frame, core.frame + 1];
            let all: Vec<u32> = p.candidates.iter().collect();
            let more = heaviest.iter().chain(&all).copied();
            for x in more.filter(|&x| frames.contains(&p.model.home(x))) {
                if chosen.len() < 12 && !chosen.contains(&x) {
                    chosen.push(x);
                }
            }
            tighter += usize::from(exact_against_generated(n, &p, &Bits::from_slice(&chosen)));
            instances += 1;
        }
        eprintln!("test 7: {instances} instances, {tighter} with the LP below the GF(2) bound");
        assert!(instances >= 80 && tighter >= 10);
    }

    /// Test 7 on one instance: the board of `n` with the candidates `cands`; the family is the plain and a random
    /// signing of each frame and the column-sum signings of the cores of the GF(2) bound's heaviest set and of three
    /// random sets.  Whether the LP is below the GF(2) bound.
    fn exact_against_generated<const W: usize>(n: u32, p: &taxman::search::Prepared<W>, cands: &Bits<W>) -> bool {
        let fm = frame_matroids(&p.model, &p.root, cands);
        let cap = p.max_total as usize;
        let ctx = Ctx::new(n, &p.model, &p.root, &fm, cap, 0, false);
        let mut signings: Vec<Signing> = (1..p.root.len())
            .flat_map(|h| [Signing::ones(h), Signing::random(h, seed(n, h, 0, SALT_RANDOM))])
            .collect();
        let gf2 = fm.bound(cap);
        let mut rng = Rng(0x5851_F42D_4C95_7F2D ^ u64::from(n));
        let mut sets: Vec<Vec<u32>> = vec![fm.chosen::<W>(&gf2).iter().collect()];
        sets.extend((0..3).map(|_| fm.elems.iter().copied().filter(|_| rng.below(2) == 0).collect::<Vec<u32>>()));
        for q in &sets {
            for core in cores_of(&p.model, &p.root, q, 2) {
                signings.push(column_sum_signing(&core, seed(n, core.frame, signings.len(), 3)));
            }
        }
        let fam = Family::new(&ctx, signings.clone());
        let lp = exact_lp(&ctx, &fam, cap);
        let settings = Settings {
            seconds: 60.0,
            pricings: 100_000,
            polyak: 5,
            check_every: 1,
            close: 1e-9,
            ..Settings::default()
        };
        let r = bound(&ctx, signings, None, false, &settings);
        assert!(r.lb <= lp + 1e-6, "n={n}: the certified bound {} above the LP {lp}", r.lb);
        // The certified point loses what the perturbations (10⁻⁶ a row) and the checks' tolerance leave out.
        let near = 1e-5 * lp.max(1.0);
        assert!(r.lb >= lp - near, "n={n}: the certified bound {} below the LP {lp}", r.lb);
        assert!(r.l_best >= lp - 1e-6, "n={n}: L {} below the LP {lp}", r.l_best);
        assert!(r.l_best <= lp + near, "n={n}: L {} far above the LP {lp}", r.l_best);
        lp < gf2.value as f64 - 1e-6
    }

    /// The LP over `P(A) ∩ P(B) ∩ ⋂ P(N_k)` with `Σ z ≤ cap`, every column enumerated.
    fn exact_lp<const W: usize>(ctx: &Ctx<'_, W>, fam: &Family, cap: usize) -> f64 {
        let e = ctx.len();
        let mut lp = Lp::new();
        let conv0 = lp.add_row(RowKind::Eq, 1.0, &[]);
        let conv: Vec<usize> = fam.extras.iter().map(|_| lp.add_row(RowKind::Eq, 1.0, &[])).collect();
        let mut link = vec![vec![usize::MAX; e]; fam.extras.len()];
        for (k, ex) in fam.extras.iter().enumerate() {
            for &x in &ex.elem {
                link[k][x] = lp.add_row(RowKind::Le, 0.0, &[]);
            }
        }
        let mut empties = Vec::new();
        for mask in 0u32..(1 << e) {
            let set: Vec<usize> = (0..e).filter(|&i| mask >> i & 1 == 1).collect();
            if set.len() > cap || set.iter().any(|&i| fam.banned[i]) {
                continue;
            }
            if !independent_gf2(&ctx.fm.a, &set) || !independent_gf2(&ctx.fm.b, &set) {
                continue;
            }
            let mut entries = vec![(conv0, 1.0)];
            for (k, row) in link.iter().enumerate() {
                let _ = k;
                entries.extend(set.iter().filter(|&&i| row[i] != usize::MAX).map(|&i| (row[i], 1.0)));
            }
            let cost = set.iter().map(|&i| f64::from(ctx.fm.elems[i])).sum();
            let j = lp.add_column(cost, &entries);
            if set.is_empty() {
                empties.push(j);
            }
        }
        for (k, ex) in fam.extras.iter().enumerate() {
            let m = ex.elem.len();
            assert!(m <= 16);
            for mask in 0u32..(1 << m) {
                let numbers: Vec<u32> = (0..m).filter(|&i| mask >> i & 1 == 1).map(|i| ex.m.elems[i]).collect();
                if !ex.m.independent(&numbers) {
                    continue;
                }
                let mut entries = vec![(conv[k], 1.0)];
                entries.extend((0..m).filter(|&i| mask >> i & 1 == 1).map(|i| (link[k][ex.elem[i]], -1.0)));
                let j = lp.add_column(0.0, &entries);
                if mask == 0 {
                    empties.push(j);
                }
            }
        }
        lp.reset(&empties);
        assert_eq!(lp.solve(u64::MAX), Status::Optimal);
        lp.value()
    }

    /// The study's test 8: the crate's intersection against the reference on random reweightings at the roots of
    /// 20 boards of 300-2175 (every evaluation checked).
    #[test]
    fn the_intersection_matches_the_reference() {
        let mut checks = 0;
        for n in spread(300, 2175, 20) {
            checks += taxman::with_width!(n, W => reweighted::<W>(n)).0;
        }
        eprintln!("test 8: {checks} checked intersections");
    }

    /// `count` games spread evenly over `lo..=hi`, each prime moved up by one (so none is prime).
    fn spread(lo: u32, hi: u32, count: u32) -> impl Iterator<Item = u32> {
        (0..count).map(move |i| lo + i * (hi - lo) / (count - 1)).map(|n| if is_prime(n) { n + 1 } else { n })
    }

    /// E2's check of the dispatch to `Meet<3>`: the same at the roots of 3200 (a frame of rank 128: two words), 3300
    /// and 3400 (ranks above 128: three words), where the first is of a game past the table's.
    #[test]
    fn the_intersection_matches_the_reference_past_rank_128() {
        let mut widest = 0;
        for n in [3200u32, 3300, 3400] {
            let (checks, rank) = taxman::with_width!(n, W => reweighted::<W>(n));
            assert!(checks > 0, "n={n}: no intersection was checked");
            widest = widest.max(rank);
        }
        assert!(widest > 128, "no board among them has a frame matroid of rank above 128");
    }

    /// The checked intersections of three reweightings at the root of `n`, and the largest rank of a frame's matroid.
    fn reweighted<const W: usize>(n: u32) -> (u64, usize) {
        let Some((_, p)) = setup::<W>(n) else { return (0, 0) };
        let fm = frame_matroids(&p.model, &p.root, &p.candidates);
        let ctx = Ctx::new(n, &p.model, &p.root, &fm, p.max_total as usize, 0, false);
        let fam = Family::new(&ctx, (1..4.min(p.root.len())).map(Signing::ones).collect());
        let mut stats = Stats::default();
        let mut rng = Rng(0xA076_1D64_78BD_642F ^ u64::from(n));
        for _ in 0..3 {
            let lam: Vec<Vec<f64>> = fam
                .extras
                .iter()
                .map(|ex| ex.m.elems.iter().map(|&x| (rng.below(1000) as f64 / 1000.0) * f64::from(x)).collect())
                .collect();
            evaluate(&ctx, &fam, &lam, &mut stats, 1);
        }
        (stats.checks, fm.ranks.iter().copied().max().unwrap_or(0))
    }

    /// Multipliers far beyond every weight, as an R2 master whose basis broke down proposed them (see
    /// `dw::evaluate_grid`), at the roots of 6 boards of 300-1300 with three signings of each of frames 1-3.  In each
    /// of four rounds a random third of the elements get, in every extra matroid that has them, one of the round's
    /// values (`i64::MAX`, what the grid makes of `λ` from about `1.4·10^14`; `2^62` and `2^61 + 12345`, which two or
    /// three at once take beyond `i64`; `2^40 + 1` and `2^35`, which do not; all of them), the others a random
    /// multiplier below their weight.  Each evaluation is `L` computed in `i128` (the reference algorithm's
    /// intersection at the positive reweighted weights, plus the greedy sets' multipliers), or `i64::MAX` if that is
    /// more, and is at least `P*`; every intersection agrees with the reference algorithm's; some values of `L` are
    /// `i64::MAX` and some are not.  Then the multipliers `∞`, `10^300` and `2.3·10^56` evaluate as `i64::MAX` on the
    /// grid.
    #[test]
    fn multipliers_beyond_the_weights() {
        let known = Known::load(&[], None).expect("the built-in optimum table");
        let (mut evaluations, mut saturated) = (0, 0);
        for n in spread(300, 1300, 6) {
            let (e, s) = taxman::with_width!(n, W => beyond_the_weights::<W>(n, &known));
            (evaluations, saturated) = (evaluations + e, saturated + s);
        }
        eprintln!("{evaluations} evaluations at multipliers beyond the weights, {saturated} of L = i64::MAX");
        assert_eq!(evaluations, 6 * 6);
        assert!(saturated > 0 && saturated < evaluations, "{saturated} of {evaluations} saturated");
    }

    /// One board of [`multipliers_beyond_the_weights`]: the evaluations made, and those of `L = i64::MAX`.
    fn beyond_the_weights<const W: usize>(n: u32, known: &Known) -> (u64, u64) {
        let (board, p) = setup::<W>(n).expect("a board with candidates");
        let opt = known.opt(n, board.in_the_bag).expect("a built-in optimum") as i64;
        let fm = frame_matroids(&p.model, &p.root, &p.candidates);
        let ctx = Ctx::new(n, &p.model, &p.root, &fm, p.max_total as usize, opt, false);
        let random = |h: usize, i: usize| Signing::random(h, seed(n, h, i, SALT_RANDOM));
        let frames = 1..4.min(p.root.len());
        let fam = Family::new(&ctx, frames.flat_map(|h| [Signing::ones(h), random(h, 0), random(h, 1)]).collect());
        let mut stats = Stats::default();
        let mut rng = Rng(0x2545_F491_4F6C_DD1D ^ u64::from(n));
        let rounds: [&[i64]; 4] = [
            &[i64::MAX],
            &[1 << 62, (1 << 61) + 12345],
            &[(1 << 40) + 1, 1 << 35],
            &[i64::MAX, 1 << 62, (1 << 61) + 12345, (1 << 40) + 1, 1 << 35],
        ];
        let mut saturated = 0;
        let weight = |x: u32| i64::from(x) * dw::SCALE;
        let mut far = vec![false; ctx.len()];
        for beyond in rounds {
            far.iter_mut().for_each(|f| *f = rng.below(3) == 0);
            let lhat: Vec<Vec<i64>> = fam
                .extras
                .iter()
                .map(|ex| {
                    let pairs = ex.elem.iter().zip(&ex.m.elems);
                    pairs
                        .map(|(&e, &x)| match far[e] {
                            true => beyond[rng.below(beyond.len() as u64) as usize],
                            false => rng.below(weight(x) as u64) as i64,
                        })
                        .collect()
                })
                .collect();
            let value = dw::evaluate_grid(&ctx, &fam, &lhat, &mut stats, 1).value;
            // L in i128.
            let mut c: Vec<i128> = fm.elems.iter().map(|&x| i128::from(weight(x))).collect();
            for (ex, l) in fam.extras.iter().zip(&lhat) {
                for (&e, &v) in ex.elem.iter().zip(l) {
                    c[e] -= i128::from(v);
                }
            }
            let positive: Vec<i64> =
                (0..ctx.len()).map(|e| if fam.banned[e] || c[e] <= 0 { 0 } else { c[e] as i64 }).collect();
            let mi = i128::from(crate::meet::intersect(&fm.a, &fm.b, &positive, ctx.cap, false).value);
            let greedy: i128 = fam
                .extras
                .iter()
                .zip(&lhat)
                .map(|(ex, l)| ex.m.greedy(l).1.iter().map(|&i| i128::from(l[i])).sum::<i128>())
                .sum();
            let exact = i64::try_from(mi + greedy).unwrap_or(i64::MAX);
            assert_eq!(value, exact, "n={n}: L is not exact");
            assert!(value.div_euclid(dw::SCALE) >= opt, "n={n}: L = {value} below P* = {opt}");
            saturated += u64::from(value == i64::MAX);
        }
        // The grid takes multipliers beyond i64 as i64::MAX.
        let lam: Vec<Vec<f64>> = fam
            .extras
            .iter()
            .map(|ex| {
                ex.elem.iter().map(|&e| if far[e] { [f64::INFINITY, 1e300, 2.3e56][e % 3] } else { 0.0 }).collect()
            })
            .collect();
        let lhat: Vec<Vec<i64>> =
            lam.iter().map(|l| l.iter().map(|&v| if v > 0.0 { i64::MAX } else { 0 }).collect()).collect();
        let on_grid = dw::evaluate_grid(&ctx, &fam, &lhat, &mut stats, 1).value;
        assert_eq!(evaluate(&ctx, &fam, &lam, &mut stats, 1).value, on_grid, "n={n}: the grid beyond i64");
        assert_eq!(stats.checks, stats.pricings_mi, "n={n}: every intersection checked against the reference");
        // The counter of saturated evaluations has the rounds' and `on_grid`'s, which the last two evaluations make.
        let on_grid_saturated = u64::from(on_grid == i64::MAX);
        assert_eq!(stats.saturated, saturated + 2 * on_grid_saturated, "n={n}: the saturated evaluations counted");
        (stats.pricings_mi, saturated + on_grid_saturated)
    }

    /// The intersection where the E3 pilot's tree of r2 at 1472 stopped (`testdata/n1472_r2_intersection.txt`, in the
    /// format of `dw::Instance::from_text`): what `dw::intersection` hands the crate there, 211 elements.  The crate's
    /// heaviest set weighs what the reference algorithm's does, and is common independent within the size limit.
    /// Before the evaluation's sums saturated, 23 more elements came with these, their reweighted weights wrapped
    /// around (16 near `2^63`, 7 just above their `x·S`), and the crate's distances overflowed.  The weights tie in
    /// places, which the crate's contract allows (not decreasing).
    #[test]
    fn the_intersection_where_the_pilot_stopped() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/n1472_r2_intersection.txt");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let instance = dw::Instance::from_text(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let w = &instance.weight;
        assert!(w[0] > 0 && w.windows(2).all(|p| p[0] <= p[1]), "positive weights, not decreasing");
        assert!(w.windows(2).any(|p| p[0] == p[1]), "weights that tie");
        let (value, chosen) = instance.solve::<1>();
        let [a, b] = instance.matroids();
        assert_eq!(
            value,
            crate::meet::intersect(&a, &b, w, instance.cap, false).value,
            "the crate against the reference"
        );
        assert!(chosen.len() <= instance.cap && independent_gf2(&a, &chosen) && independent_gf2(&b, &chosen));
        assert_eq!(chosen.iter().map(|&e| w[e]).sum::<i64>(), value);
    }

    /// The bound on small boards with the main arm: valid, converged, the optimal promotions independent in every
    /// final matroid (the asserts of `roots`).
    #[test]
    fn the_main_arm_on_small_boards() {
        let known = Known::load(&[], None).expect("the built-in optimum table");
        let mut out = Output { file: None, done: HashSet::new() };
        let o = Options {
            command: "roots".into(),
            known: Vec::new(),
            arms: ["gf2", "f1", "f1r1", "cores", "f1cores"].map(String::from).to_vec(),
            modes: Vec::new(),
            settings: Settings { seconds: 20.0, check_every: 1, ..Settings::default() },
            fixed_seconds: 20.0,
            root_seconds: 20.0,
            frames: vec![1, 2, 3],
            out: None,
            census: None,
            save_fit: None,
            exact: None,
            samples: 0,
            games: Vec::new(),
            files: Vec::new(),
            methods: Vec::new(),
            start: None,
            offer: None,
            prune: None,
            bnb: BnbOptions::default(),
        };
        for n in [300u32, 360, 420, 480] {
            taxman::with_width!(n, W => roots::<W>(n, &known, &o, &mut out, None));
        }
    }

    /// The settings of the tests' root fits: search mode within `seconds`.
    fn search_settings(seconds: f64) -> Settings {
        Settings { mode: FitMode::Search, seconds, ..Settings::default() }
    }

    /// Runs `f` at the root of `n` ([`at_root`]), with the main arm's fit in search mode within `seconds` and the
    /// GF(2) bound.
    fn with_root_fit<const W: usize>(n: u32, seconds: f64, f: impl FnOnce(&Prepared<W>, &Ctx<'_, W>, &Outcome, i64)) {
        let known = Known::load(&[], None).expect("the built-in optimum table");
        let (board, p) = setup::<W>(n).expect("a board with candidates");
        let opt = known.opt(n, board.in_the_bag).expect("a built-in optimum");
        at_root(n, &p, opt, |ctx, gf2| {
            let r = fit_outcome(ctx, &[1, 2, 3], &search_settings(seconds));
            assert_eq!(r.lb, 0.0, "n={n}: search mode certifies nothing");
            f(&p, ctx, &r, gf2);
        });
    }

    /// The design's S5.2 test 6, what E3 needs of a root fit: at the roots of 600 and 900, the main arm's fit in
    /// search mode, saved as a fit file (`common/fit.rs`) and loaded, gives the run's bound again as `bnb` starts
    /// from it: Tier 1 at the root from its state (`Warm::from_saved`) is exactly the bound saved.
    #[test]
    fn fit_round_trip() {
        let dir = std::env::temp_dir().join(format!("fields-fit-test-{}", std::process::id()));
        for n in [600u32, 900] {
            taxman::with_width!(n, W => with_root_fit::<W>(n, 10.0, |_, ctx, r, gf2| {
                let path = dir.join(format!("n{n}.json"));
                SavedFit::of(n, ctx.target as u64, gf2, FitMode::Search.name(), r).save(&path).expect("save");
                let saved = SavedFit::load(&path).unwrap_or_else(|e| panic!("{e}"));
                assert_eq!(saved.bound, r.bound, "n={n}: the bound saved is not the run's");
                let warm = Warm::from_saved(&saved);
                let fam = Family::new(ctx, warm.signings.clone());
                let (value, _) = tier1(ctx, &fam, &warm, &mut Stats::default());
                assert_eq!(value, saved.bound, "n={n}: Tier 1 at the root from the fit loaded");
                eprintln!(
                    "n={n}: bound {}, l_best {}, {} signings, {} columns, {} cuts",
                    saved.bound,
                    saved.l_best,
                    warm.signings.len(),
                    warm.columns.len(),
                    warm.cuts.len()
                );
            }));
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Asserts what a refit's results must satisfy: its trace starts at Tier 1's bound `tier`, never falls below
    /// `P*` and never rises, has an entry per pricing after Tier 1's, and ends at the bound returned, which the
    /// returned state's multipliers give again exactly (with `exactly` false, a bound no lower: R3's later rounds can
    /// merge cores into the signings its earlier rounds opened, whose matroids then change, so that the final family's
    /// lowest `L` can lie above an earlier family's).
    fn assert_refit<const W: usize>(ctx: &Ctx<'_, W>, what: &str, tier: i64, r: &(i64, Warm, Trace), exactly: bool) {
        let (value, state, trace) = r;
        let after = &trace.after;
        assert_eq!(after.first(), Some(&tier), "{what}: the trace starts at Tier 1");
        assert!(after.iter().all(|&a| a >= ctx.target), "{what}: below P* = {}: {after:?}", ctx.target);
        assert!(after.windows(2).all(|w| w[1] <= w[0]), "{what}: the trace rises: {after:?}");
        assert_eq!(after.len(), trace.pricings + 1, "{what}: a trace entry per pricing");
        assert_eq!(after.last(), Some(value), "{what}: the bound returned is the trace's last");
        let fam = Family::new(ctx, state.signings.clone());
        let (again, _) = tier1(ctx, &fam, state, &mut Stats::default());
        if exactly {
            assert_eq!(again, *value, "{what}: the state returned gives another bound");
        } else {
            assert!(again >= *value, "{what}: the state returned gives a lower bound than the refit's");
        }
    }

    /// R3 with the budgets of the tests (validity does not depend on them).
    fn test_r3() -> R3 {
        R3 { seconds: 0.5, pricings: 100, ..R3::default() }
    }

    /// R1, R2 (local 0 and 2) with `k = 20` and R3 (with each of its switches off as well) at the roots of the
    /// non-prime boards 300-700 every 40th, from the main arm's search-mode fit (3 s) with every multiplier halved:
    /// valid traces and states (`assert_refit`), R3's at most 2 new signings a frame.
    #[test]
    fn refits_are_valid() {
        let (mut lowered, mut pricings, mut opened) = (0, 0, 0);
        let mut r3_checked = 0;
        for n in (300..=700u32).step_by(40).filter(|&n| !is_prime(n)) {
            taxman::with_width!(n, W => with_root_fit::<W>(n, 3.0, |_, ctx, r, _| {
                let mut warm = Warm::from_outcome(r);
                for m in &mut warm.lambda {
                    m.values_mut().for_each(|v| *v /= 2.0);
                }
                let mut stats = Stats::default();
                let fam = || Family::new(ctx, warm.signings.clone());
                let settings = Settings { seconds: 10.0, ..Settings::default() };
                let f = fam();
                let (tier, first) = tier1(ctx, &f, &warm, &mut stats);
                let r1 = refit_r1(ctx, &f, &warm, &first, 20, &mut stats);
                assert_refit(ctx, &format!("n={n} r1"), tier, &r1, true);
                assert_eq!((r1.2.new_signings, &r1.1.columns, &r1.1.cuts), (0, &warm.columns, &warm.cuts));
                let mut line = format!("n={n}: tier 1 {tier}, r1 {} ({} pricings)", r1.0, r1.2.pricings);
                for local in [0, 2] {
                    let f = fam();
                    let (_, first) = tier1(ctx, &f, &warm, &mut stats);
                    let r2 = refit_r2(ctx, f, &warm, first, 20, local, &settings, &mut stats);
                    let what = format!("n={n} r2 local {local}");
                    assert_refit(ctx, &what, tier, &r2, true);
                    let (state, trace) = (&r2.1, &r2.2);
                    assert!(trace.new_signings <= 2 * ctx.state.len() * local, "{what}");
                    assert_eq!(state.local, warm.local + trace.new_signings, "{what}");
                    assert_eq!(state.signings.len(), warm.signings.len() + trace.new_signings, "{what}");
                    assert!(state.signings.iter().all(|s| !s.mergeable && s.cores.is_empty()), "{what}: frozen");
                    line += &format!(
                        ", r2/{local} {} ({} pricings, {} signings, {:?})",
                        r2.0, trace.pricings, trace.new_signings, trace.stop
                    );
                    lowered += usize::from(r2.0 < tier);
                    pricings += trace.pricings;
                    opened += trace.new_signings;
                }
                let orders = [(true, true, false), (true, false, false), (false, true, false), (true, true, true)];
                for (exact, cores, cores_first) in orders {
                    let f = fam();
                    let (_, first) = tier1(ctx, &f, &warm, &mut stats);
                    let settings = R3 { exact, cores, cores_first, ..test_r3() };
                    let r3 = refit_r3(ctx, f, &warm, first, &settings, &mut stats);
                    let what = format!("n={n} {}", r3_name(&settings));
                    assert_refit(ctx, &what, tier, &r3, false);
                    let (state, trace) = (&r3.1, &r3.2);
                    for h in 0..ctx.state.len() {
                        let new = state.local_in(h) - warm.local_in(h);
                        assert!(new <= LOCAL_PER_NODE, "{what}: {new} new signings of frame {h}");
                    }
                    assert!(cores || trace.new_signings == 0, "{what}: signings without the separation");
                    assert_eq!(state.signings.len(), warm.signings.len() + trace.new_signings, "{what}");
                    assert!(state.signings.iter().all(|s| !s.mergeable && s.cores.is_empty()), "{what}: frozen");
                    line += &format!(
                        ", {} {} ({} pricings, {} rounds, {} signings, {:?})",
                        r3_name(&settings),
                        r3.0,
                        trace.pricings,
                        trace.rounds,
                        trace.new_signings,
                        trace.stop
                    );
                    lowered += usize::from(r3.0 < tier);
                    r3_checked += 1;
                }
                lowered += usize::from(r1.0 < tier);
                pricings += r1.2.pricings;
                eprintln!("{line}");
            }));
        }
        eprintln!("refits: {lowered} lowered the bound, {pricings} pricings, {opened} local signings, {r3_checked} R3");
        assert!(lowered > 0 && opened > 0, "the refits had nothing to do");
    }

    /// At the roots of 300-700 every 50th, each child of the root as `examples/matroid.rs`'s `bnb` makes them (the
    /// GF(2) heaviest set's conflict `k₁ < … < k_r`: child `j` promotes `k₁ … k_{j−1}` and leaves out `k_j`) takes
    /// over the root fit's state (`Warm::child`), which is valid there ([`assert_state_valid`]).
    #[test]
    fn child_keeps_valid_columns() {
        let (mut children, mut columns, mut cuts) = (0, 0, 0);
        for n in (300..=700u32).step_by(50) {
            taxman::with_width!(n, W => with_root_fit::<W>(n, 3.0, |p, _, r, _| {
                let warm = Warm::from_outcome(r);
                let model = &p.model;
                let mut scratch = Scratch::<W>::new();
                let mut rng = Rng(0x2545_F491_4F6C_DD1D ^ u64::from(n));
                let max_add = |state: &FrameSet<W>, promoted: u32, cands: &Bits<W>| {
                    (p.max_total as i32 - promoted as i32).min(state.max_promotions_among(cands)).max(0) as usize
                };
                let cands = p.root.narrow_candidates(&p.candidates);
                let fm = frame_matroids(model, &p.root, &cands);
                let chosen = fm.chosen::<W>(&fm.bound(max_add(&p.root, 0, &cands)));
                let Some(k) = model.conflict_using(&mut scratch, &p.root, &chosen, true) else { return };
                let (mut required, mut rest, mut at) = (Vec::new(), cands, p.root);
                for x in k.iter() {
                    rest.remove(x);
                    let live_cands = at.narrow_candidates(&rest);
                    let limit = max_add(&at, required.len() as u32, &live_cands);
                    let fmc = frame_matroids(model, &at, &live_cands);
                    let child = warm.child(&required, &[x], &fmc.elems, limit);
                    let checked = assert_state_valid(n, model, &at, &fmc, &live_cands, limit, &child, &mut rng);
                    children += 1;
                    columns += checked.0;
                    cuts += checked.1;
                    match model.promote_all_using(&mut scratch, &at, &Bits::EMPTY.with(x)) {
                        Some(next) => {
                            at = next;
                            required.push(x);
                        }
                        None => break,
                    }
                }
            }));
        }
        eprintln!("child_keeps_valid_columns: {children} children, {columns} columns and {cuts} cuts checked");
        assert!(children > 0 && columns > 0 && cuts > 0);
    }

    /// Asserts that `warm`, the state that a node of `n` takes over (the node's `state`, its GF(2) matroids `fm` over
    /// its candidates `cands`, at most `max_add` more members), is valid there: every column is within `max_add` and
    /// common independent in `fm`; every cut is within `fm`'s elements and, with its rank taken in a family of
    /// `warm`'s signings at the node, holds for 20 random maximal promotable sets of the candidates.  The columns and
    /// the cuts checked.
    #[allow(clippy::too_many_arguments)]
    fn assert_state_valid<const W: usize>(
        n: u32,
        model: &FrameGraph,
        state: &FrameSet<W>,
        fm: &FrameMatroids,
        cands: &Bits<W>,
        max_add: usize,
        warm: &Warm,
        rng: &mut Rng,
    ) -> (usize, usize) {
        assert_columns_independent::<W>(n, fm, &warm.columns);
        for c in &warm.columns {
            assert!(c.len() <= max_add, "n={n}: a column of {} members, max_add {max_add}", c.len());
        }
        let ctx = Ctx::new(n, model, state, fm, max_add, 0, false);
        let fam = Family::new(&ctx, warm.signings.clone());
        let live: Vec<u32> = cands.iter().collect();
        let completions: Vec<Vec<u32>> =
            (0..20).map(|_| random_promotable(model, state, &live, rng).iter().collect()).collect();
        for (j, set) in &warm.cuts {
            assert!(set.iter().all(|x| fm.elems.binary_search(x).is_ok()), "n={n}: a cut's member is no element");
            let rank = cut_rank(&fam, *j, set);
            for c in &completions {
                let common = set.iter().filter(|x| c.binary_search(x).is_ok()).count();
                assert!(common <= rank, "n={n}: {common} of a cut of rank {rank} in a promotable set");
            }
        }
        (warm.columns.len(), warm.cuts.len())
    }

    /// Runs `f` on the game `n` set up for `bnb`, with the incumbent `P* − handicap` and a root fit of the main arm
    /// in search mode within `seconds`, and with the optimal game's promotions (if known); not if `P*` is below the
    /// handicap.
    fn with_game<const W: usize>(n: u32, seconds: f64, handicap: u64, f: impl FnOnce(&Game<'_, W>, Option<Bits<W>>)) {
        let known = Known::load(&[], None).expect("the built-in optimum table");
        let Some((board, p)) = setup::<W>(n) else { return };
        let opt = known.opt(n, board.in_the_bag).expect("a built-in optimum");
        let Some(incumbent) = opt.checked_sub(handicap) else { return };
        let optimal = optimal_promotions(n, &known, &board, opt);
        let fit = root_fit(n, &p, opt, None, &[1, 2, 3], &search_settings(seconds));
        let graph = MatchGraph::new(&p.model, &p.root);
        f(&Game::new(n, &p, &graph, incumbent, &fit), optimal);
    }

    /// An extra bound that hands every node to [`A1`] and shows `watch` the node, its parent's state and the bound.
    struct Watched<'a, const W: usize, F> {
        a1: A1<'a, W>,
        watch: F,
    }

    impl<const W: usize, F: FnMut(&NodeView<'_, W>, Option<&Rc<Warm>>, i64)> ExtraBound<W> for Watched<'_, W, F> {
        type State = Rc<Warm>;

        fn bound(&mut self, v: &NodeView<'_, W>, parent: Option<&Rc<Warm>>) -> (i64, Rc<Warm>, NodeLog) {
            let r = self.a1.bound(v, parent);
            (self.watch)(v, parent, r.0);
            r
        }
    }

    /// [`A1`] watched by `watch`.
    fn watched<'a, const W: usize, F: FnMut(&NodeView<'_, W>, Option<&Rc<Warm>>, i64)>(
        a1: A1<'a, W>,
        watch: F,
    ) -> Watched<'a, W, F> {
        Watched { a1, watch }
    }

    /// The design's S5.2 test 1: along 200 random dives of `r2` (10 at each of 20 non-prime games spread over
    /// 300-700), the state that each node below the root takes over from its parent (`Warm::child`) is valid at the
    /// node ([`assert_state_valid`]).
    #[test]
    fn warm_child_keeps_columns_and_cuts_valid() {
        let (mut nodes, mut columns, mut cuts) = (0, 0, 0);
        for n in spread(300, 700, 20) {
            taxman::with_width!(n, W => with_game::<W>(n, 3.0, 0, |g, _| {
                let mut rng = Rng(0x2545_F491_4F6C_DD1D ^ u64::from(n));
                let mut completions = Rng(0x9E37_79B9_7F4A_7C15 ^ u64::from(n));
                let watch = |v: &NodeView<'_, W>, parent: Option<&Rc<Warm>>, _| {
                    let Some(parent) = parent else { return };
                    let child = parent.child(&v.promoted_now.to_vec(), &v.removed.to_vec(), &v.fm.elems, v.max_add);
                    let checked =
                        assert_state_valid(n, &g.p.model, v.state, v.fm, &v.cands, v.max_add, &child, &mut completions);
                    nodes += 1;
                    columns += checked.0;
                    cuts += checked.1;
                };
                let mut extra = watched(g.a1(Variant::R2, &BnbOptions::default(), None), watch);
                let mut s = g.search();
                for _ in 0..10 {
                    s.dive(Some(&mut extra), &mut rng);
                }
            }));
        }
        eprintln!("test 1: {nodes} nodes, {columns} columns and {cuts} cuts checked");
        assert!(nodes > 0 && columns > 0 && cuts > 0);
    }

    /// The options of a test's run of `variant` under `policy`, with R3's budgets of the tests.
    fn options(variant: Variant, policy: Policy) -> BnbOptions {
        BnbOptions { variants: vec![variant], policy, r3: test_r3(), ..BnbOptions::default() }
    }

    /// The pilot's variants (their default policy) and r3 under `r3_policies`, as the tests run them.
    fn runs(r3_policies: &[Policy]) -> Vec<(Variant, BnbOptions)> {
        let pilot = Variant::PILOT.into_iter().map(|v| (v, options(v, Policy::All)));
        pilot.chain(r3_policies.iter().map(|&p| (Variant::R3, options(Variant::R3, p)))).collect()
    }

    /// The design's S5.2 test 2: at the games 300, 350, …, 1300, with the incumbent `P* − 1` (so that no node on the
    /// path of the optimal promotions `S*` is cut by a valid bound), a dive along `S*` (the child whose promotions
    /// are in `S*` and whose member left out is not) for each of v1, r1, r2, r2c and r3 (`r3-all` and
    /// `r3-top:0.25`, U4's test): the extra bound at every node is at least `w(S* − promoted)`, and the dive ends at
    /// a leaf, the GF(2) heaviest set promotable.
    #[test]
    fn bounds_hold_along_the_optimum() {
        let mut nodes = 0;
        for n in (300..=1300u32).step_by(50) {
            taxman::with_width!(n, W => with_game::<W>(n, 3.0, 1, |g, optimal| {
                let optimal = optimal.expect("the optimal promotions");
                for (variant, o) in runs(&[Policy::R3All, Policy::R3Top(0.25)]).into_iter().skip(1) {
                    let what = format!("n={n} {} {}", variant.name(), o.policy.name());
                    let watch = |v: &NodeView<'_, W>, _: Option<&Rc<Warm>>, bound: i64| {
                        assert!(v.promoted.is_subset(&optimal), "{what}: a node off the optimum's path");
                        let rest = (optimal - v.promoted).sum() as i64;
                        assert!(bound >= rest, "{what}: {bound} below w(S* − promoted) = {rest} at depth {}", v.depth);
                        nodes += 1;
                    };
                    let mut extra = watched(g.a1(variant, &o, None), watch);
                    let mut s = g.search();
                    s.dive_by(Some(&mut extra), &mut |children| {
                        let on_path = |c: &Pending<W, Rc<Warm>>| {
                            c.promoted_now.is_subset(&optimal) && !c.removed.intersects(&optimal)
                        };
                        children.iter().position(on_path).expect("a child on the optimum's path")
                    });
                    let t = &s.tally;
                    let cuts = (t.cut_by_count, t.cut_by_bound, t.cut_tier1, t.cut_refit);
                    assert_eq!((t.better, cuts), (1, (0, 0, 0, 0)), "{what}: the dive ends at a leaf");
                }
            }));
        }
        eprintln!("test 2: {nodes} nodes on the optimum's paths");
    }

    /// The design's S5.2 test 3: at every non-prime game 30-600 with `P* > 0`, with the incumbent `P* − 1`, the
    /// branch and bound of each variant (r3 with `r3-top:0.25`, U4's test) finds a completion worth `P*` (a `better`
    /// one) within 200,000 nodes.
    #[test]
    fn handicapped_incumbent_is_found() {
        let mut games = 0;
        for n in (30..=600u32).filter(|&n| !is_prime(n)) {
            taxman::with_width!(n, W => with_game::<W>(n, 2.0, 1, |g, _| {
                for (variant, o) in runs(&[Policy::R3Top(0.25)]) {
                    let mut extra = g.a1(variant, &o, None);
                    let mut s = g.search();
                    while s.tally.better == 0 && s.tally.nodes < 200_000 && s.step(extra.extra()).is_some() {}
                    let nodes = s.tally.nodes;
                    assert!(s.tally.better >= 1, "n={n} {}: nothing worth P* in {nodes} nodes", variant.name());
                }
                games += 1;
            }));
        }
        eprintln!("test 3: {games} games");
        assert!(games > 400);
    }

    /// The records of `variant`'s whole tree under the options `o` in the order visited, which must be complete
    /// within 10⁶ nodes, find nothing better than the incumbent, and have no two nodes alike (promotions, candidates).
    fn tree_records<const W: usize>(g: &Game<'_, W>, variant: Variant, o: &BnbOptions) -> Vec<NodeRecord<W>> {
        let mut extra = g.a1(variant, o, None);
        let mut s = g.search();
        let mut records = Vec::new();
        while let Some(r) = s.step(extra.extra()) {
            records.push(r);
            assert!(records.len() <= 1_000_000, "n={} {}: no complete tree in 10⁶ nodes", g.n, variant.name());
        }
        assert_eq!(s.tally.better, 0, "n={} {}: a completion better than P*", g.n, variant.name());
        let nodes: HashSet<_> = records.iter().map(|r| (r.promoted, r.cands)).collect();
        assert_eq!(nodes.len(), records.len(), "n={} {}: two nodes alike", g.n, variant.name());
        records
    }

    /// The nodes (promotions, candidates) of `variant`'s whole tree under `o` ([`tree_records`]).
    fn tree_nodes<const W: usize>(g: &Game<'_, W>, variant: Variant, o: &BnbOptions) -> HashSet<(Bits<W>, Bits<W>)> {
        tree_records(g, variant, o).iter().map(|r| (r.promoted, r.cands)).collect()
    }

    /// The design's S5.2 test 4 (and U4's): on 20 non-prime games spread over 300-900, the nodes of the whole trees
    /// of r2, r2c and r3 (`r3-top:0.25`) are among those of v0's (the extra bound only cuts).
    #[test]
    fn extra_trees_are_subtrees() {
        for n in spread(300, 900, 20) {
            taxman::with_width!(n, W => with_game::<W>(n, 3.0, 0, |g, _| {
                let v0 = tree_nodes(g, Variant::V0, &BnbOptions::default());
                let mut sizes = vec![v0.len()];
                for (variant, o) in runs(&[Policy::R3Top(0.25)]).into_iter().skip(3) {
                    let nodes = tree_nodes(g, variant, &o);
                    assert!(nodes.is_subset(&v0), "n={n} {}: a node not in v0's tree", variant.name());
                    sizes.push(nodes.len());
                }
                eprintln!("n={n}: v0, r2, r2c and r3 trees of {sizes:?} nodes");
            }));
        }
    }

    /// A dive's outcome with its probability: (probability, GF(2) estimate, A1 estimate).
    type DiveOutcome = (f64, f64, f64);

    /// The exact joint distribution of the two estimates of a paired dive with v1 (`Search::dive`): one outcome per
    /// root-to-leaf path of v0's tree (`v0`, its records in the order visited), with the path's probability (the
    /// product of 1/m over its nodes of m children), its GF(2) estimate (the sum over its nodes of the product of the
    /// numbers of children of the nodes above them) and its A1 estimate (the same sum up to and including the first
    /// node v1 cuts, the nodes `cut` by promotions and candidates).  v1's Tier 1 at a node depends on the node alone,
    /// its state being the root fit's, so the nodes it cuts in its own tree are those it cuts on any dive.
    ///
    /// Why: Knuth's estimator is unbiased but heavy-tailed.  The design's first criterion, each tree's node count
    /// within the 99% bootstrap interval of the mean of 2,000 dives, failed at 1000 and 1215 although the dives follow
    /// this distribution (rare paths carry most of the mean, and 2,000 dives seldom meet them), so the dives are
    /// checked against the distribution itself.
    fn exact_dives<const W: usize>(v0: &[NodeRecord<W>], cut: &HashSet<(Bits<W>, Bits<W>)>) -> Vec<DiveOutcome> {
        let mut children = vec![Vec::new(); v0.len()];
        for (i, r) in v0.iter().enumerate() {
            assert_eq!((r.id, r.parent.is_none()), (i as u64, i == 0), "ids in the order visited, from the root");
            if let Some(q) = r.parent {
                children[q as usize].push(i);
            }
        }
        for r in v0 {
            assert_eq!(children[r.id as usize].len(), r.children, "a node's children are its records' children");
        }
        let mut out = Vec::new();
        // (node, probability, product of the numbers of children above it, estimates so far, whether v1 has not cut)
        let mut stack = vec![(0usize, 1.0, 1.0, 0.0, 0.0, true)];
        while let Some((v, p, product, gf2, a1, alive)) = stack.pop() {
            let (gf2, a1) = (gf2 + product, if alive { a1 + product } else { a1 });
            let alive = alive && !cut.contains(&(v0[v].promoted, v0[v].cands));
            let kids = &children[v];
            if kids.is_empty() {
                out.push((p, gf2, a1));
                continue;
            }
            let m = kids.len() as f64;
            stack.extend(kids.iter().map(|&c| (c, p / m, product * m, gf2, a1, alive)));
        }
        out
    }

    /// The Kolmogorov–Smirnov distance between the sample `xs` and the distribution of `(probability, value)` pairs:
    /// the largest gap between their distribution functions, both steps continuous on the right, so taken at the
    /// jumps.
    fn ks_distance(xs: &[f64], dist: &[(f64, f64)]) -> f64 {
        let mut xs = xs.to_vec();
        xs.sort_by(f64::total_cmp);
        let mut dist = dist.to_vec();
        dist.sort_by(|a, b| a.1.total_cmp(&b.1));
        let (mut i, mut j, mut cdf, mut ks) = (0, 0, 0.0, 0.0f64);
        while i < dist.len() || j < xs.len() {
            let t = dist.get(i).map_or(f64::INFINITY, |d| d.1).min(xs.get(j).copied().unwrap_or(f64::INFINITY));
            while i < dist.len() && dist[i].1 <= t {
                cdf += dist[i].0;
                i += 1;
            }
            while j < xs.len() && xs[j] <= t {
                j += 1;
            }
            ks = ks.max((cdf - j as f64 / xs.len() as f64).abs());
        }
        ks
    }

    /// The design's S5.2 test 5, made sound (run it explicitly; it takes minutes): at 1000, 1100, 1215 and 1300 (root
    /// fits of 30 s), the exact distribution of a paired dive's estimates from the exact trees of v0 and v1
    /// ([`exact_dives`], why there), whose means must be the trees' node counts, against 2,000 paired dives with v1:
    /// the Kolmogorov–Smirnov distance of each estimate below the 1% critical value `1.63/√2000`.  Printed per game:
    /// the counts, the dives' means with their 99% bootstrap intervals, the exact ratio of the counts against the
    /// dives' ratio interval, the distances, and the exact standard deviation of one dive's estimates (the tail).
    #[test]
    #[ignore]
    fn knuth_dives_follow_the_exact_distribution() {
        const DIVES: usize = 2000;
        let critical = 1.63 / (DIVES as f64).sqrt();
        for n in [1000u32, 1100, 1215, 1300] {
            taxman::with_width!(n, W => with_game::<W>(n, 30.0, 0, |g, _| {
                let v0 = tree_records(g, Variant::V0, &BnbOptions::default());
                let v1 = tree_records(g, Variant::V1, &BnbOptions::default());
                let cut: HashSet<_> =
                    v1.iter().filter(|r| r.fate == Fate::Tier1).map(|r| (r.promoted, r.cands)).collect();
                let exact = exact_dives(&v0, &cut);
                let moment = |part: fn(&DiveOutcome) -> f64, power: i32| -> f64 {
                    exact.iter().map(|o| o.0 * part(o).powi(power)).sum()
                };
                let parts: [fn(&DiveOutcome) -> f64; 2] = [|o| o.1, |o| o.2];
                let counts = [v0.len() as f64, v1.len() as f64];
                let means = parts.map(|part| moment(part, 1));
                for (mean, count) in means.iter().zip(counts) {
                    assert!((mean - count).abs() <= 1e-6 * count, "n={n}: the exact mean {mean} of {count} nodes");
                }
                let sds = [0, 1].map(|k| (moment(parts[k], 2) - means[k] * means[k]).sqrt());
                let mut extra = g.a1(Variant::V1, &BnbOptions::default(), None);
                let mut s = g.search();
                let values: Vec<(f64, f64)> =
                    (0..DIVES).map(|i| s.dive(extra.extra(), &mut dive_rng(1, n, i))).collect();
                let samples = [values.iter().map(|v| v.0).collect::<Vec<_>>(), values.iter().map(|v| v.1).collect()];
                let ks = [0, 1].map(|k| {
                    let marginal: Vec<(f64, f64)> = exact.iter().map(|o| (o.0, parts[k](o))).collect();
                    ks_distance(&samples[k], &marginal)
                });
                let e = Estimate::of(&values, 0.99, &mut bootstrap_rng(1, n));
                let ratio = counts[0] / counts[1];
                for (name, k, spread) in [("v0", 0, &e.gf2), ("v1", 1, &e.a1)] {
                    let (count, within) = (counts[k], spread.lo <= counts[k] && counts[k] <= spread.hi);
                    eprintln!(
                        "n={n} {name}: {count} nodes; dives' mean {:.0} in [{:.0}, {:.0}] at 99% (count within: \
                         {within}); KS {:.4} (critical {critical:.4}); one dive's exact sd {:.0}",
                        spread.mean, spread.lo, spread.hi, ks[k], sds[k]
                    );
                }
                let within = e.ratio_lo <= ratio && ratio <= e.ratio_hi;
                eprintln!(
                    "n={n}: exact ratio {ratio:.4}, dives' {:.4} in [{:.4}, {:.4}] (exact within: {within})",
                    e.ratio, e.ratio_lo, e.ratio_hi
                );
                for (name, k) in [("GF(2)", 0), ("A1", 1)] {
                    assert!(ks[k] < critical, "n={n}: the {name} estimates' KS distance {} from the exact", ks[k]);
                }
            }));
        }
    }

    /// The counters of a run's line (its fields but the times), from the line of `out`.
    fn counters_of(out: &Path) -> Vec<(String, Json)> {
        let text = std::fs::read_to_string(out).expect("a run's line");
        let Json::Obj(pairs) = Json::parse(text.trim()).expect("one line") else { panic!("not an object") };
        pairs.into_iter().filter(|(name, _)| !name.starts_with("ms")).collect()
    }

    /// The design's S5.2 test 6 (as this study specifies it): at 840 with r2, a run checkpointed every 50 nodes and
    /// stopped as if killed at 200 nodes, then resumed from its checkpoint to the end, gives the same line (but the
    /// times) and the same per-node log as a run straight through.
    #[test]
    fn resumed_run_is_the_same() {
        let n = 840;
        let dir = std::env::temp_dir().join(format!("fields-bnb-test-{}", std::process::id()));
        taxman::with_width!(n, W => with_game::<W>(n, 3.0, 0, |g, _| {
            let run = |name: &str, checkpoint: bool, halt_at: Option<u64>| {
                let at = make_dir(&dir.join(name)).to_path_buf();
                let o = BnbOptions {
                    variants: vec![Variant::R2],
                    log: Some(at.clone()),
                    checkpoint: checkpoint.then(|| at.clone()),
                    checkpoint_every: Every::Nodes(50),
                    halt_at,
                    ..BnbOptions::default()
                };
                let mut out = Output::open(Some(&at.join("bnb.jsonl")), &BNB_KEY);
                run_tree(g, Variant::R2, &o, None, &mut out)
            };
            assert!(run("straight", false, None), "a run straight through ends");
            assert!(!run("resumed", true, Some(200)), "the run is stopped at 200 nodes");
            let stem = g.stem(Variant::R2, &BnbOptions::default());
            assert!(dir.join("resumed").join(format!("{stem}.ckpt.json")).exists(), "a checkpoint");
            assert!(run("resumed", true, None), "the resumed run ends");
            let file = |name: &str, what: &str| dir.join(name).join(format!("{stem}{what}"));
            assert!(!file("resumed", ".ckpt.json").exists() && !file("resumed", ".tmp").exists(), "left over");
            let line = |name: &str| counters_of(&dir.join(name).join("bnb.jsonl"));
            assert_eq!(line("resumed"), line("straight"), "the lines differ");
            let log = |name: &str| std::fs::read_to_string(file(name, ".jsonl")).expect("a per-node log");
            let (straight, resumed) = (log("straight"), log("resumed"));
            assert!(straight.lines().count() > 200, "a tree of more than 200 nodes");
            assert_eq!(resumed, straight, "the per-node logs differ");
            eprintln!("test 6: the same tree of {} nodes", straight.lines().count());
        }));
        std::fs::remove_dir_all(&dir).expect("the test directory removed");
    }

    /// E2's fit file: the fit of a root's run, written as text and read back (through a file as well), gives the
    /// run's bound again at the root, with the same signings and the same text.
    #[test]
    fn a_saved_fit_gives_its_bound_again() {
        let known = Known::load(&[], None).expect("the built-in optimum table");
        let dir = std::env::temp_dir().join(format!("fields-test-{}", std::process::id()));
        let mut core_signings = 0;
        for n in [600u32, 750, 900] {
            core_signings += taxman::with_width!(n, W => saved_fit::<W>(n, &known, &dir));
        }
        std::fs::remove_dir_all(&dir).ok();
        assert!(core_signings > 0, "no board had a core-derived signing to save");
    }

    /// Fits the root of `n` (the main arm), saves and loads the fit, and checks what comes back; the number of
    /// core-derived signings in it.
    fn saved_fit<const W: usize>(n: u32, known: &Known, dir: &Path) -> usize {
        let (board, p) = setup::<W>(n).expect("a game with candidates");
        let opt = known.opt(n, board.in_the_bag).expect("a known optimum");
        at_root(n, &p, opt, |ctx, gf2| {
            let settings = Settings { seconds: 20.0, check_every: 1, ..Settings::default() };
            let r = fit_outcome(ctx, &[1, 2, 3], &settings);
            let run = r.fit.bound(ctx, &mut Stats::default());
            assert_eq!(run, r.bound, "n={n}: the run's fit does not give its bound");

            let saved = SavedFit::of(n, opt, gf2, "certified", &r);
            let text = saved.to_json().to_string();
            let path = dir.join(format!("n{n}.json"));
            saved.save(&path).expect("save");
            for back in [
                SavedFit::from_json(&Json::parse(&text).expect("JSON")).expect("a fit"),
                SavedFit::load(&path).unwrap(),
            ] {
                assert_eq!(back.bound(ctx), r.bound, "n={n}: the loaded fit does not give the bound");
                assert_eq!(back.to_json().to_string(), text, "n={n}: the fit does not write back as it was read");
                assert_eq!((back.n, back.opt, back.gf2, back.bound), (n, opt, gf2, r.bound));
                assert_eq!(back.fit.signings.len(), r.fit.signings.len());
                for (a, b) in back.fit.signings.iter().zip(&r.fit.signings) {
                    assert_eq!(
                        (a.frame, a.default, &a.fixed),
                        (b.frame, b.default, &b.fixed),
                        "n={n}: a signing differs"
                    );
                    assert!(!a.mergeable && a.cores.is_empty(), "a loaded signing is frozen");
                }
                assert_eq!(back.fit.lambda, r.fit.lambda, "n={n}: the multipliers differ");
                assert!(
                    back.columns.len() <= dw::SAVED_COLUMNS && back.support.is_some_and(|s| s <= back.columns.len())
                );
                assert_columns_independent::<W>(n, ctx.fm, &back.columns);
                assert!(back.cuts.iter().all(|(k, _, _)| *k < back.fit.signings.len()));
            }
            r.fit.signings.iter().filter(|s| s.mergeable).count()
        })
    }

    /// The fits that `bench/fields/fits` holds (E2's, and the E3 pilot's root fits of 1215 and 1472) are intact: each
    /// reads back and writes back the same text, and at the root of its game gives the bound it says, over the GF(2)
    /// bound it says.  Skipped where the directory is not.
    #[test]
    fn the_saved_fits_give_their_bounds() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/fields/fits");
        let Ok(entries) = std::fs::read_dir(&dir) else { return };
        let mut files: Vec<PathBuf> = entries.map(|e| e.expect("a directory entry").path()).collect();
        files.retain(|p| p.extension().is_some_and(|e| e == "json"));
        files.sort();
        for path in &files {
            let saved = SavedFit::load(path).unwrap_or_else(|e| panic!("{e}"));
            let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert!(saved.to_json().to_string() + "\n" == text, "{}: does not write back as read", path.display());
            taxman::with_width!(saved.n, W => check_saved::<W>(&saved));
        }
        eprintln!("{} saved fits checked", files.len());
    }

    fn check_saved<const W: usize>(saved: &SavedFit) {
        let n = saved.n;
        let (_, p) = setup::<W>(n).expect("a game with candidates");
        at_root(n, &p, saved.opt, |ctx, gf2| {
            assert_eq!(gf2, saved.gf2, "n={n}: the GF(2) bound differs from the file's");
            assert_eq!(saved.bound(ctx), saved.bound, "n={n}: the fit does not give its bound");
            assert!(saved.bound >= saved.opt as i64 && saved.bound <= saved.gf2, "n={n}: the bound is out of range");
            assert_columns_independent::<W>(n, ctx.fm, &saved.columns);
        });
    }

    /// Asserts that each of `columns` (candidates) is common independent in the GF(2) matroids `fm` of a state of `n`.
    fn assert_columns_independent<const W: usize>(n: u32, fm: &FrameMatroids, columns: &[Vec<u32>]) {
        for c in columns {
            fm.independent(&Bits::<W>::from_slice(c)).unwrap_or_else(|e| panic!("n={n}: a column {c:?}: {e}"));
        }
    }

    /// Search mode runs no exact check, certifies nothing, and gives a valid bound; the fixed arms have no
    /// separation; `f1cores16` is the main arm with a limit of 16 core-derived signings a frame.
    #[test]
    fn search_mode_certifies_nothing() {
        let known = Known::load(&[], None).expect("the built-in optimum table");
        for n in [750u32, 900] {
            taxman::with_width!(n, W => search_mode::<W>(n, &known));
        }
        assert_eq!(
            (arm_cap("f1cores"), arm_cap("f1cores16"), arm_cap("f1"), arm_cap("cores")),
            (None, Some(16), None, None)
        );
    }

    fn search_mode<const W: usize>(n: u32, known: &Known) {
        let (board, p) = setup::<W>(n).expect("a game with candidates");
        let opt = known.opt(n, board.in_the_bag).expect("a known optimum");
        at_root(n, &p, opt, |ctx, _| {
            for name in ["f1cores", "f1"] {
                let (signings, separate) = arm(name, n, p.root.len(), &[1, 2, 3]);
                let settings = Settings { check_every: 1, ..search_settings(20.0) };
                let r = bound(ctx, signings, None, separate, &settings);
                assert_eq!((r.stats.checks_exact, r.lb), (0, 0.0), "n={n}, {name}: search mode certified");
                assert!(r.bound >= opt as i64, "n={n}, {name}: the bound {} is below P* = {opt}", r.bound);
                assert_eq!(
                    r.fit.bound(ctx, &mut Stats::default()),
                    r.bound,
                    "n={n}, {name}: the fit does not give the bound"
                );
                if separate {
                    assert_eq!(r.stop, Stop::Fitted, "n={n}, {name}");
                    assert!(r.family.extras.iter().any(|ex| ex.signing.mergeable), "n={n}: no core-derived signing");
                } else {
                    assert_eq!(r.stop, Stop::Converged, "n={n}, {name}");
                }
            }
        });
    }

    /// The lines of a file of JSON lines.
    fn json_lines(path: &Path) -> Vec<Json> {
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        text.lines().map(|l| Json::parse(l).expect("a JSON line")).collect()
    }

    /// U4's tests of the saved states, the harvest and the offer, at 840 (trees of a few hundred nodes).  Runs of r2
    /// and of r3 (`r3-all`, budgets that never bind there) save every 3rd Tier-1-open node's state; `replay` of each
    /// state with the host's method gives the host's refit exactly (its bound and the digest of its final state), and
    /// the reference runs from it.  The r3 run harvests the cores its nodes' separation found; the fit of 841's root
    /// from 840's (`fit`), with those cores offered, places some of them, gives a bound of at least `P*` with the
    /// optimal promotions independent in every matroid (which `fit_game` asserts), and its saved fit gives its bound
    /// again.
    #[test]
    fn states_replay_and_cores_carry() {
        let n = 840;
        let known = Known::load(&[], None).expect("the built-in optimum table");
        let dir =
            make_dir(&std::env::temp_dir().join(format!("fields-replay-test-{}", std::process::id()))).to_path_buf();
        let harvest = dir.join("cores.jsonl");
        taxman::with_width!(n, W => with_game::<W>(n, 3.0, 0, |g, _| {
            for (variant, policy, method) in [(Variant::R2, Policy::All, "r2"), (Variant::R3, Policy::R3All, "r3:60")] {
                let o = BnbOptions {
                    variants: vec![variant],
                    policy,
                    r3: R3 { seconds: 60.0, ..R3::default() },
                    save_states: Some(dir.clone()),
                    every: 3,
                    harvest: (variant == Variant::R3).then(|| harvest.clone()),
                    ..BnbOptions::default()
                };
                let mut out = Output::open(Some(&dir.join("bnb.jsonl")), &BNB_KEY);
                assert!(run_tree(g, variant, &o, None, &mut out));
                let states = dir.join(format!("{}.states.jsonl", g.stem(variant, &o)));
                let probes = [method, "ref:5"].map(|m| Probe::parse(m, &o).expect("a method"));
                let replayed = dir.join(format!("{}.replay.jsonl", variant.name()));
                replay(&states, &known, &probes, &o, &mut Output::open(Some(&replayed), &REPLAY_KEY));
                let lines = json_lines(&replayed);
                let saved = json_lines(&states).len();
                assert!(saved >= 5, "{}: {saved} states saved", variant.name());
                assert_eq!(lines.len(), 2 * saved, "{}: a line per state and method", variant.name());
                for j in lines.iter().filter(|j| j.get("method") == Some(&Json::from(method))) {
                    assert_eq!(j.get("host_same"), Some(&Json::Bool(true)), "{}: replay differs: {j}", variant.name());
                }
                eprintln!("{}: {saved} states replayed as the host refitted them", variant.name());
            }
        }));
        let cores = read_cores(&harvest);
        assert!(!cores.is_empty(), "the r3 run's nodes found no core");
        let fits = dir.join("fits");
        let mut o = Options {
            command: "fit".into(),
            known: Vec::new(),
            arms: Vec::new(),
            modes: Vec::new(),
            settings: Settings { seconds: 20.0, ..Settings::default() },
            fixed_seconds: 20.0,
            root_seconds: 20.0,
            frames: vec![1, 2, 3],
            out: None,
            census: None,
            save_fit: Some(fits.join("n840.json")),
            exact: None,
            samples: 0,
            games: Vec::new(),
            files: Vec::new(),
            methods: Vec::new(),
            start: None,
            offer: None,
            prune: None,
            bnb: BnbOptions::default(),
        };
        let lines = dir.join("fits.jsonl");
        let mut out = Output::open(Some(&lines), &["n", "label"]);
        taxman::with_width!(840, W => fit_game::<W>(840, &known, &o, &mut out));
        (o.start, o.offer, o.save_fit) = (Some(fits.join("n840.json")), Some(harvest), Some(fits.join("n841.json")));
        (o.bnb.label, o.prune) = (Some("harvest".into()), Some(24));
        taxman::with_width!(841, W => fit_game::<W>(841, &known, &o, &mut out));
        let lines = json_lines(&lines);
        let placed = lines[1].get("offered_placed").and_then(Json::as_u64).expect("offered_placed");
        assert_eq!(lines[1].get("offered").and_then(Json::as_u64), Some(cores.len() as u64));
        assert!(placed > 0, "none of the {} cores offered was placed", cores.len());
        let saved = SavedFit::load(&fits.join("n841.json")).unwrap_or_else(|e| panic!("{e}"));
        taxman::with_width!(841, W => check_saved::<W>(&saved));
        eprintln!("841 from 840 with {placed} of {} cores placed: {}", cores.len(), lines[1]);
        std::fs::remove_dir_all(&dir).expect("the test directory removed");
    }
}
