//! The game that `choice-prepare` and `choice` play: their options ([`Params`]), the CPU clock and budget they work
//! under, the board set up as `taxman solve` sets it up ([`Case`]), and the play of a command over the games of the
//! arguments ([`run`]).
//!
//! The commands take the games as `N` or `A-B`, play them one after the other on one thread of one process, and print
//! a JSON line a game as it ends.  A board with nothing to promote has the line `{"n":N,"trivial":true}`.  The budgets
//! are CPU seconds of the process (the laptop is shared, and the wall time would not be the work), which is why one
//! process plays one game at a time.  The commands, and what their lines hold, are defined in `choice.rs`.

use super::{beyond, counters, set_up, Line, Options};
use crate::{choice, decompose, lns};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::Control;
use taxman::game::Optimal;
use taxman::matchbound::MatchGraph;
use taxman::search::{start_incumbent, Prepared, SearchSpace};

// ---------------------------------------------------------------------------------------------------------
// Options

/// The options of `choice-prepare` and `choice`, with their defaults.
#[derive(Clone)]
pub struct Params {
    /// CPU seconds the finder and the sound search each run for, to harvest the pool of nogoods (`--cpu`).
    pub cpu: f64,
    /// CPU seconds after which `choice` stops each unit's solve (none: it goes on to the end).
    pub cap: Option<f64>,
    /// How the finder searches, and the seed of its random choices (`--seed`, `--fix`, `--unit-nodes`).
    pub lns: lns::Options,
    /// The subgradient steps of the fit at the root (`--steps`).
    pub steps: usize,
    /// Where `choice-prepare` takes its nodes: a directory of `.nodes` fixtures, the tuning suite, and how many nodes
    /// on the way to an optimal set it makes.
    pub fixtures: Option<PathBuf>,
    pub suite: Option<PathBuf>,
    pub paths: usize,
    /// The directory of the games' prepared files (`--prep`), how many nodes `choice-prepare` keeps of each source
    /// (`--keep`), the directory `choice` writes its results to and its worker's number there (`--results`,
    /// `--worker`), the directory where workers claim units (`--claims`), the subgradient steps of a node's refit
    /// (`--node-steps`), and whether it is repeated with the nogoods of the unit's own solve too (`--sub-nogoods`).
    pub prep: Option<PathBuf>,
    pub keep: usize,
    pub results: Option<PathBuf>,
    pub worker: usize,
    pub claims: Option<PathBuf>,
    pub node_steps: usize,
    pub sub: bool,
    /// The multi-field bound (`fields.rs`): the seconds of the root fit `choice-prepare` makes for a game
    /// (`--fit-seconds`; 0: none, so no multi-field bounds), the directory of fits it takes before fitting (`--fits`),
    /// whether `choice` measures the multi-field bounds (`--a1`), the pricings of the warm refit (`--a1-pricings`) and
    /// the seconds of the node's own master (`--a1-seconds`).
    pub fit_seconds: f64,
    pub fits: Option<PathBuf>,
    pub a1: bool,
    pub a1_pricings: usize,
    pub a1_seconds: f64,
    /// `decompose`: the pauses of the game's proof that supply nodes, the steps between them, the steps at which a
    /// node's subtree solve and its best-completion solve are stopped (`--pauses`, `--span`, `--unit-cap`,
    /// `--best-cap`; no limit on the subtree, no best completion, by default), and the evaluations at which the OR
    /// part of the AND/OR search stops, for the nodes of at least `and_or_min` steps (`--and-or-cap`, `--and-or-min`;
    /// the search is not run by default).
    pub pauses: usize,
    pub span: u64,
    pub unit_cap: Option<u64>,
    pub best_cap: u64,
    pub and_or_cap: u64,
    pub and_or_min: u64,
}

impl Default for Params {
    fn default() -> Params {
        Params {
            cpu: 5.0,
            cap: None,
            lns: lns::Options::default(),
            steps: 200,
            fixtures: None,
            suite: None,
            paths: 50,
            prep: None,
            keep: 40,
            results: None,
            worker: 0,
            claims: None,
            node_steps: 50,
            sub: false,
            fit_seconds: 0.0,
            fits: None,
            a1: false,
            a1_pricings: 20,
            a1_seconds: 30.0,
            pauses: 0,
            span: 1024,
            unit_cap: None,
            best_cap: 0,
            and_or_cap: 0,
            and_or_min: 500,
        }
    }
}

/// A range `LO-HI` of shares of one.
pub fn fix_range(text: &str) -> Result<(f64, f64), String> {
    let bad = |e: &dyn std::fmt::Display| format!("--fix {text}: {e}");
    let (lo, hi) = text.split_once('-').ok_or_else(|| bad(&"LO-HI"))?;
    let (lo, hi): (f64, f64) = (lo.parse().map_err(|e| bad(&e))?, hi.parse().map_err(|e| bad(&e))?);
    if (0.0..=hi).contains(&lo) && hi <= 1.0 {
        Ok((lo, hi))
    } else {
        Err(bad(&"0 <= LO <= HI <= 1"))
    }
}

// ---------------------------------------------------------------------------------------------------------
// Budgets

/// Elapsed CPU seconds of this process (wall seconds where the system does not say).
pub struct Clock {
    cpu: Option<f64>,
    wall: Instant,
}

impl Clock {
    pub fn start() -> Clock {
        Clock { cpu: counters::cpu_seconds(), wall: Instant::now() }
    }

    pub fn elapsed(&self) -> f64 {
        match (self.cpu, counters::cpu_seconds()) {
            (Some(before), Some(after)) => after - before,
            _ => self.wall.elapsed().as_secs_f64(),
        }
    }
}

/// What a method works under: the control that stops its searches when the CPU budget is up, whether it is, and the
/// CPU clock of the budget.
pub struct Budget {
    pub control: Arc<Control>,
    expired: AtomicBool,
    pub clock: Clock,
}

impl Budget {
    pub fn expired(&self) -> bool {
        self.expired.load(Ordering::Acquire)
    }
}

/// `work` under a budget of `cap` CPU seconds (none: no limit), and the CPU seconds it took.  A watchdog thread stops
/// the budget's control once the budget is spent, which ends the search in progress; it sleeps in between, so it costs
/// nothing.
pub fn budgeted<T>(cap: Option<f64>, work: impl FnOnce(&Budget) -> T) -> (T, f64) {
    let budget = Budget { control: Control::new(), expired: AtomicBool::new(false), clock: Clock::start() };
    let done = AtomicBool::new(false);
    let (budget, done) = (&budget, &done);
    let value = std::thread::scope(|scope| {
        if let Some(cap) = cap {
            scope.spawn(move || {
                while !done.load(Ordering::Acquire) {
                    if budget.clock.elapsed() >= cap {
                        budget.expired.store(true, Ordering::Release);
                        budget.control.stop();
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
            });
        }
        let value = work(budget);
        done.store(true, Ordering::Release);
        value
    });
    (value, budget.clock.elapsed())
}

// ---------------------------------------------------------------------------------------------------------
// The game

/// A game, set up, with what the commands use.
pub struct Case<'a, const W: usize> {
    pub board: &'a Board<W>,
    pub p: &'a Prepared<W>,
    pub space: &'a SearchSpace<'a, W>,
    pub params: &'a Params,
    pub optimal: &'a Optimal,
}

impl<const W: usize> Case<'_, W> {
    pub fn n(&self) -> u32 {
        self.board.n()
    }

    /// The game a standalone `solve` starts from, and the CPU seconds it took.
    pub fn start(&self) -> (Bits<W>, f64) {
        let clock = Clock::start();
        let graph = self.space.match_graph;
        (start_incumbent(self.p, graph, &Control::new(), 1, None), clock.elapsed())
    }
}

// ---------------------------------------------------------------------------------------------------------
// The commands

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// `wall choice-prepare` (`choice.rs`): a game's nogood pool, its root multipliers and its sampled nodes, in a file.
    ChoicePrepare,
    /// `wall choice` (`choice.rs`): the Lagrangian bound refitted at each node of a prepared file, against the node's
    /// subtree.
    Choice,
    /// `wall decompose` (`decompose.rs`): how the conflicts of a node lie against each separator home.
    Decompose,
}

impl Command {
    /// The command of that name, if it is one of these.
    pub fn named(name: &str) -> Option<Command> {
        match name {
            "choice-prepare" => Some(Command::ChoicePrepare),
            "choice" => Some(Command::Choice),
            "decompose" => Some(Command::Decompose),
            _ => None,
        }
    }
}

/// What a command made of a game: its line, and the faults found (a bound below what a node reaches, a nogood that
/// can be promoted, ...: bugs, reported after the line).
pub struct Played {
    pub line: Line,
    pub faults: Vec<String>,
}

/// Game `n` at the set width `W`, played by `command`; a board with nothing to promote has only its line.
fn play<const W: usize>(command: Command, n: u32, o: &Options, optimal: &Optimal) -> Result<Played, String> {
    let (board, prepared) = set_up::<W>(n);
    let Some(p) = prepared else {
        let mut line = Line::of(n);
        line.field("trivial", true);
        return Ok(Played { line, faults: Vec::new() });
    };
    let graph = MatchGraph::new(&p.model, &p.root);
    let mut space = SearchSpace::new(&p, &graph);
    if command == Command::ChoicePrepare {
        // Every nogood a searcher learns is wanted, so none is forgotten (the unit searches of the finder are shorter
        // than the period anyway; the control's table would be cleared every NOGOOD_PERIOD steps).  `choice` keeps
        // the period: it counts the steps of a unit's solve as `tune` does.
        space.forgetting.period = None;
    }
    let case = Case { board: &board, p: &p, space: &space, params: &o.params, optimal };
    match command {
        Command::ChoicePrepare => choice::prepare(&case),
        Command::Choice => choice::measure(&case),
        Command::Decompose => decompose::measure(&case),
    }
}

/// Game `n`, played by `command`, at the set width the solver picks for it.
fn game(command: Command, n: u32, o: &Options, optimal: &Optimal) -> Result<Played, String> {
    taxman::with_width!(n, W => play::<W>(command, n, o, optimal), else Err(beyond(n)))
}

/// `command` on every game of `o`: a line each, printed as its game ends.  A game with a fault is an error after its
/// line.
pub fn run(command: Command, o: &Options, optimal: &Optimal) -> Result<(), String> {
    for &n in &o.games {
        let played = game(command, n, o, optimal)?;
        played.line.print();
        if let Some(fault) = played.faults.into_iter().next() {
            return Err(fault);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// Tests: `cargo test --features studies` runs them (the example is `test = true` in Cargo.toml)

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use taxman::json::Json;

    /// The options of a command with finders that run for `cpu` seconds.
    fn options(command: &str, cpu: f64) -> Options {
        let mut o = Options::new(command.to_owned());
        o.params.cpu = cpu;
        o
    }

    /// The line of game `n` as JSON, played by `command` with the options `o`, with no fault found.
    fn line_of(command: Command, n: u32, o: &Options) -> Json {
        let played = game(command, n, o, Optimal::built_in()).expect("a board the solver supports");
        assert!(played.faults.is_empty(), "{:?}", played.faults);
        Json::parse(&played.line.render()).expect("JSON")
    }

    fn number(line: &Json, name: &str) -> u64 {
        line.get(name).and_then(Json::as_u64).unwrap_or_else(|| panic!("{name} in {line}"))
    }

    fn yes(line: &Json, name: &str) -> bool {
        line.get(name).and_then(Json::as_bool).unwrap_or_else(|| panic!("{name} in {line}"))
    }

    /// Game 936 prepared for `choice` (a few of the probe's nodes and some on the way to an optimal set) and measured:
    /// every unit has its line in the worker's file, and a run again goes on where it was.
    #[test]
    fn a_game_prepared_for_choice_is_measured_and_a_run_again_goes_on_where_it_was() {
        let dir = crate::support::TempDir::new("wall-choice");
        let mut o = options("choice-prepare", 0.2);
        (o.params.steps, o.params.paths, o.params.keep) = (30, 3, 6);
        o.params.fit_seconds = 3.0;
        o.params.fixtures = Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/perf"));
        o.params.prep = Some(dir.0.join("prep"));
        let prepared = line_of(Command::ChoicePrepare, 936, &o);
        let (incumbent, root, pool) =
            (number(&prepared, "incumbent"), number(&prepared, "root"), number(&prepared, "pool"));
        assert!(yes(&prepared, "known") && incumbent <= root && pool > 0, "{prepared}");
        assert_eq!((number(&prepared, "fixture_kept"), number(&prepared, "path_kept")), (6, 3), "{prepared}");
        assert!(number(&prepared, "fixture_open") >= 6, "{prepared}");
        assert!(dir.0.join("prep/n0936.choice").exists() && dir.0.join("prep/n0936.fit.json").exists());
        let (a1_bound, a1_source) = (prepared.get("a1_bound").and_then(Json::as_i64), prepared.get("a1_source"));
        assert!(a1_bound >= Some(incumbent as i64) && a1_bound <= Some(root as i64), "{prepared}");
        assert_eq!(a1_source.and_then(Json::as_str), Some("fitted"), "{prepared}");

        let mut m = options("choice", 0.0);
        m.params.prep = o.params.prep.clone();
        m.params.node_steps = 15;
        m.params.sub = true;
        m.params.a1 = true;
        m.params.a1_seconds = 3.0;
        m.params.results = Some(dir.0.join("out"));
        m.params.claims = Some(dir.0.join("claims"));
        let line = line_of(Command::Choice, 936, &m);
        assert_eq!((number(&line, "units"), number(&line, "measured")), (9, 9), "{line}");
        let file = dir.0.join("out/worker-0.jsonl");
        let lines: Vec<Json> =
            std::fs::read_to_string(&file).unwrap().lines().map(|line| Json::parse(line).expect("JSON")).collect();
        assert_eq!(lines.len(), 9);
        for unit in &lines {
            let source = unit.get("source").and_then(Json::as_str).unwrap();
            let strong = unit.get("strong").unwrap();
            assert_eq!(number(unit, "pool"), pool, "{unit}");
            assert!(!strong.get("lows").and_then(Json::as_arr).unwrap().is_empty(), "{unit}");
            let a1 = unit.get("a1").filter(|a1| !a1.is_null()).unwrap_or_else(|| panic!("a1 in {unit}"));
            for variant in ["r2", "full"] {
                let found = a1.get(variant).unwrap_or_else(|| panic!("{variant} in {unit}"));
                let (bound, cut) =
                    (found.get("bound").and_then(Json::as_i64), found.get("cut").and_then(Json::as_bool));
                // A node on the way to an optimal set is cut by nothing: its target is what its best completion
                // reaches.
                let cuts = (source != "path").then(|| bound < unit.get("need").and_then(Json::as_i64));
                assert!(bound.is_some() && cut == Some(cuts.unwrap_or(false)), "{unit}");
            }
            if source == "path" {
                assert!(matches!(unit.get("subtree"), Some(Json::Null)) && number(unit, "lower") > 0, "{unit}");
            } else {
                let gap = unit.get("gap").and_then(Json::as_i64);
                assert!(yes(unit, "completed") && gap >= Some(0) && number(unit, "subtree") >= 1, "{unit}");
                assert!(unit.get("sub").is_some_and(|sub| !sub.is_null()), "{unit}");
            }
        }

        // Run again: every unit is in the results. With the results gone, the claims keep the units from a second worker.
        let again = line_of(Command::Choice, 936, &m);
        assert_eq!((number(&again, "measured"), number(&again, "skipped")), (0, 9), "{again}");
        std::fs::remove_dir_all(dir.0.join("out")).unwrap();
        let again = line_of(Command::Choice, 936, &m);
        assert_eq!((number(&again, "measured"), number(&again, "elsewhere")), (0, 9), "{again}");
        std::fs::remove_dir_all(dir.0.join("claims")).unwrap();
        m.params.worker = 1;
        let again = line_of(Command::Choice, 936, &m);
        assert_eq!(number(&again, "measured"), 9, "{again}");
        assert!(dir.0.join("out/worker-1.jsonl").exists());
    }

    #[test]
    fn a_board_with_nothing_to_promote_has_a_line_of_its_own() {
        let o = options("choice-prepare", 0.1);
        let line = line_of(Command::ChoicePrepare, 2, &o);
        assert!(yes(&line, "trivial"), "{line}");
    }

    #[test]
    fn the_fix_range_is_read_as_written() {
        assert_eq!(fix_range("0.7-0.9"), Ok((0.7, 0.9)));
        assert!(fix_range("0.9-0.7").is_err() && fix_range("0.5").is_err() && fix_range("0.5-1.5").is_err());
    }
}
