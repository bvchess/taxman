//! The lab channel of a range: the hook that tells a lab when a game runs long (`--hook`), and the command folder
//! through which a lab steers the games in flight (`--lab-dir`).  solver/docs/design/lab.md has the design and the
//! argument that nothing the lab does can make a result wrong; solver/docs/REFERENCE.md has the format of the events
//! and the commands.  The driver of the window ([`window`](super::window)) acts on both; this module has what they
//! are made of: the options ([`LabOptions`]), the commands and their checks ([`Command`]), the folder ([`Folder`]),
//! the hook's events ([`spawn_hook`]), what the lab did to a game, kept beside its checkpoint ([`GameLab`]), and what it
//! gave for the games to come, kept beside them ([`Policies`]).
//!
//! Without `--lab-dir` and `--hook` nothing here runs.  With an empty folder, the cost is one directory read at each
//! progress tick, on the driver's thread.

use super::SolveOptions;
use crate::json::Json;
use crate::results::build;
use crate::search::{BranchSettings, SearchTactics, MAX_GATE};
use crate::tactics::Tactics;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The time a game's own search runs before the hook is told that it runs long, unless `--hook-after` says otherwise.
pub const HOOK_AFTER: Duration = Duration::from_secs(3600);

/// The interval at which the driver reads the command folder when the run prints no progress lines (`--progress 0`).
pub const LAB_TICK: Duration = Duration::from_secs(60);

/// How a range talks to a lab (`--lab-dir`, `--hook`, `--hook-after`).  None of it is set by default, and then a
/// range runs exactly as without it.
#[derive(Clone, Debug)]
pub struct LabOptions {
    /// The command folder: commands in `in/`, outcomes in `out/`, commands done with in `done/`.
    pub dir: Option<PathBuf>,
    /// The program told of a game that runs long, and of its end.
    pub hook: Option<PathBuf>,
    /// A game's own search time after which the hook is told it runs long.
    pub hook_after: Duration,
}

impl Default for LabOptions {
    fn default() -> Self {
        LabOptions { dir: None, hook: None, hook_after: HOOK_AFTER }
    }
}

// ---------------------------------------------------------------------------------------------------------
// Commands

/// A command of the lab, as read from a file of `in/` and checked for its form (the driver checks the rest against
/// the run: whether the game is in flight, whether its search runs, ...).
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Command {
    /// Write every running search's checkpoint now.
    Checkpoint,
    /// Offer game `n`'s search the promotions `set`, claimed to be worth `value` (their sum if not given), through
    /// `search::offer_found`.
    Offer { n: u32, set: Vec<u32>, value: Option<u64> },
    /// Switch `conflict_branching` and `conflict_look`, those given, on or off in game `n`'s search, and set the
    /// `gate`, the gap from which conflict branching chooses, for the units it starts from now on.
    Branching { n: u32, conflict_branching: Option<bool>, conflict_look: Option<bool>, gate: Option<u64> },
    /// Set `conflict_branching`, `conflict_look` and `gate`, those given, for every game from `from` on: the games set
    /// up later, when they are, and the games in flight, at once.
    Policy { from: u32, conflict_branching: Option<bool>, conflict_look: Option<bool>, gate: Option<u64> },
    /// Abandon game `n`'s search and start it fresh with the run's search tactics changed by `tactics` (`name=value`).
    Restart { n: u32, tactics: Vec<String> },
    /// The run's budget of threads.
    Threads { total: usize },
    /// Game `n`'s search keeps this many workers, outside the policy that shares the threads (`None`: back to it).
    Workers { n: u32, workers: Option<usize> },
    /// Run a finder beside game `n`'s search for this long.
    Finder { n: u32, budget: Duration },
    /// Stop the run with final checkpoints, so that the chain's script builds `commit` and runs it.
    Redeploy { commit: String },
}

/// The most threads a `threads` command may ask for.
pub(super) const MAX_THREADS: usize = 4096;

/// The longest finder a command may start.
const MAX_FINDER: Duration = Duration::from_secs(24 * 3600);

impl Command {
    /// The command a file holds, or why it is not one.  A command is a JSON object whose `command` names it; every
    /// other field must be one that command takes (so a misspelt field is refused, not ignored).
    pub(super) fn parse(json: &Json) -> Result<Command, String> {
        let fields = json.as_obj().ok_or("a command is a JSON object")?;
        let name = json.get("command").and_then(Json::as_str).ok_or("a command needs a \"command\" field, a string")?;
        let takes: &[&str] = match name {
            "checkpoint" => &[],
            "offer" => &["n", "set", "value"],
            "branching" => &["n", "conflict_branching", "conflict_look", "gate"],
            "policy" => &["from", "conflict_branching", "conflict_look", "gate"],
            "restart" => &["n", "tactics"],
            "threads" => &["total", "n", "workers"],
            "finder" => &["n", "seconds"],
            "redeploy" => &["commit"],
            "advice" => return Err("advice (branch a unit on a given conflict) is reserved, and not taken yet".into()),
            _ => return Err(format!("unknown command {name:?}")),
        };
        if let Some((key, _)) = fields.iter().find(|(key, _)| key != "command" && !takes.contains(&key.as_str())) {
            return Err(format!("{name} takes no field {key:?}"));
        }
        let game = |key: &str| {
            json.get(key).and_then(Json::as_u32).filter(|&n| n >= 1).ok_or(format!("{name} needs {key}, a game"))
        };
        match name {
            "checkpoint" => Ok(Command::Checkpoint),
            "offer" => {
                let set = json.get("set").and_then(Json::as_u32_vec).ok_or("offer needs set, a list of numbers")?;
                let value = match json.get("value") {
                    None => None,
                    Some(value) => Some(value.as_u64().ok_or("value is a whole number")?),
                };
                Ok(Command::Offer { n: game("n")?, set, value })
            }
            "branching" => {
                let Given { conflict_branching, conflict_look, gate } = switches(json, name)?;
                Ok(Command::Branching { n: game("n")?, conflict_branching, conflict_look, gate })
            }
            "policy" => {
                let Given { conflict_branching, conflict_look, gate } = switches(json, name)?;
                Ok(Command::Policy { from: game("from")?, conflict_branching, conflict_look, gate })
            }
            "restart" => {
                let tactics = match json.get("tactics") {
                    None => Vec::new(),
                    Some(Json::Str(text)) => assignments(text),
                    Some(_) => return Err("tactics is a string of name=value, separated by commas".into()),
                };
                Ok(Command::Restart { n: game("n")?, tactics })
            }
            "threads" => match (json.get("total"), json.get("n")) {
                (Some(total), None) if json.get("workers").is_none() => {
                    let total = total.as_u64().and_then(|t| usize::try_from(t).ok());
                    match total.filter(|t| (1..=MAX_THREADS).contains(t)) {
                        Some(total) => Ok(Command::Threads { total }),
                        None => Err(format!("total is a whole number from 1 to {MAX_THREADS}")),
                    }
                }
                (None, Some(_)) => {
                    let workers = match json.get("workers") {
                        Some(Json::Null) => None,
                        Some(workers) => Some(
                            workers
                                .as_u64()
                                .and_then(|w| usize::try_from(w).ok())
                                .filter(|&w| w <= MAX_THREADS)
                                .ok_or(format!("workers is a whole number from 0 to {MAX_THREADS}, or null"))?,
                        ),
                        None => return Err("threads with n needs workers, a number or null".into()),
                    };
                    Ok(Command::Workers { n: game("n")?, workers })
                }
                _ => Err("threads takes total, or n and workers".into()),
            },
            "finder" => {
                let seconds = json.get("seconds").and_then(Json::as_f64).filter(|s| s.is_finite() && *s > 0.0);
                let budget = seconds
                    .map(Duration::from_secs_f64)
                    .filter(|&budget| budget <= MAX_FINDER)
                    .ok_or(format!("finder needs seconds, more than 0 and at most {}", MAX_FINDER.as_secs()))?;
                Ok(Command::Finder { n: game("n")?, budget })
            }
            "redeploy" => {
                let commit = json.get("commit").and_then(Json::as_str).unwrap_or_default();
                let hex = commit.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
                if !hex || !(7..=40).contains(&commit.len()) {
                    return Err("redeploy needs commit, 7 to 40 lowercase hexadecimal digits".into());
                }
                Ok(Command::Redeploy { commit: commit.to_owned() })
            }
            _ => unreachable!("every command is named above"),
        }
    }
}

/// A switch as a command or a policy file gives it: `"on"` or `"off"`.
fn on_off(value: &Json, key: &str) -> Result<bool, String> {
    match value.as_str() {
        Some("on") => Ok(true),
        Some("off") => Ok(false),
        _ => Err(format!("{key} is \"on\" or \"off\"")),
    }
}

/// A gate as a command or a lab file gives it: a whole number from 0 to [`MAX_GATE`].
fn gate_value(value: &Json) -> Result<u64, String> {
    value.as_u64().filter(|&gate| gate <= MAX_GATE).ok_or(format!("gate is a whole number from 0 to {MAX_GATE}"))
}

/// The settings a `branching` or `policy` command gives; one it leaves out is `None`.
struct Given {
    conflict_branching: Option<bool>,
    conflict_look: Option<bool>,
    gate: Option<u64>,
}

/// The settings that the command `name` gives: `conflict_branching` and `conflict_look`, each `"on"` or `"off"`, and
/// `gate`; at least one.  A gate is the gap from which conflict branching chooses, so a command that gives one cannot
/// turn conflict branching off.
fn switches(json: &Json, name: &str) -> Result<Given, String> {
    let given = |key: &str| json.get(key).map(|value| on_off(value, key)).transpose();
    let (conflict_branching, conflict_look) = (given("conflict_branching")?, given("conflict_look")?);
    let gate = json.get("gate").map(gate_value).transpose()?;
    if conflict_branching.is_none() && conflict_look.is_none() && gate.is_none() {
        return Err(format!("{name} needs conflict_branching, conflict_look or gate"));
    }
    if gate.is_some() && conflict_branching == Some(false) {
        return Err(format!("{name} gives a gate, the gap from which conflict branching chooses, and turns it off"));
    }
    Ok(Given { conflict_branching, conflict_look, gate })
}

/// A switch as the lab's files and a results line write it: `"on"`, `"off"`, or null where it was not given.
fn setting_json(on: Option<bool>) -> Json {
    on.map_or(Json::Null, |on| Json::from(if on { "on" } else { "off" }))
}

/// A gate as the lab's files and a results line write it: the number, or null where it was not given.
fn gate_json(gate: Option<u64>) -> Json {
    gate.map_or(Json::Null, Json::from)
}

/// The gate in a lab file's object `json`: none where it is null or missing, as in a file from before the gate.
fn gate_field(json: &Json) -> Result<Option<u64>, String> {
    match json.get("gate") {
        None | Some(Json::Null) => Ok(None),
        Some(value) => gate_value(value).map(Some),
    }
}

/// The assignments of a list `name=value,name=value` (blank pieces left out).
fn assignments(text: &str) -> Vec<String> {
    text.split(',').map(str::trim).filter(|piece| !piece.is_empty()).map(str::to_owned).collect()
}

/// The search tactics of a restart: the run's (`run`), changed by `assignments`, each a search tactic's.
pub(super) fn restart_tactics(run: &Tactics, assignments: &[String]) -> Result<SearchTactics, String> {
    let mut tactics = *run;
    for assignment in assignments {
        tactics.set(assignment)?;
        if (Tactics { search: tactics.search, ..*run }) != tactics {
            return Err(format!("{assignment} is not a search tactic: a restart changes the search's tactics only"));
        }
    }
    Ok(tactics.search)
}

// ---------------------------------------------------------------------------------------------------------
// The folder

/// How a command ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Status {
    Applied,
    Refused,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Applied => "applied",
            Status::Refused => "refused",
        }
    }
}

/// A command taken from `in/`: the name of its file, and what it says (or why it says nothing).
pub(super) struct Taken {
    pub(super) name: String,
    pub(super) json: Option<Json>,
    pub(super) command: Result<Command, String>,
}

/// The command folder: `in/`, where the lab puts commands, one JSON object a file; `out/`, where the driver writes
/// the outcome of each, under the same name; `done/`, where it moves each command once taken.
#[derive(Clone, Debug)]
pub(super) struct Folder {
    inbox: PathBuf,
    out: PathBuf,
    done: PathBuf,
}

impl Folder {
    /// The folder `dir`, with its three subfolders made if they are missing.
    pub(super) fn open(dir: &Path) -> Result<Folder, String> {
        let folder = Folder { inbox: dir.join("in"), out: dir.join("out"), done: dir.join("done") };
        for sub in [&folder.inbox, &folder.out, &folder.done] {
            fs::create_dir_all(sub).map_err(|e| format!("the lab folder {}: {e}", sub.display()))?;
        }
        Ok(folder)
    }

    /// The commands in `in/`, in the order of their names, each moved to `done/` as it is taken.  A file whose name
    /// starts with a dot is one being written (write a command under such a name and rename it), and is left alone.
    /// With nothing there, this is one read of the directory.
    pub(super) fn take(&self) -> Vec<Taken> {
        let Ok(entries) = fs::read_dir(&self.inbox) else { return Vec::new() };
        let mut names: Vec<String> = entries
            .flatten()
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| !name.starts_with('.'))
            .collect();
        names.sort();
        let mut taken = Vec::new();
        for name in names {
            let from = self.inbox.join(&name);
            let text = fs::read(&from);
            if let Err(e) = fs::rename(&from, self.free_name(&self.done, &name)) {
                // A command that cannot be moved would be taken again at every tick: say so once, and leave it.
                say(format_args!("cannot move {} to {}: {e}; it is not taken", from.display(), self.done.display()));
                continue;
            }
            let json = text
                .map_err(|e| format!("cannot read it: {e}"))
                .and_then(|bytes| String::from_utf8(bytes).map_err(|_| "it is not UTF-8 text".to_owned()))
                .and_then(|text| Json::parse(&text).map_err(|e| e.to_string()));
            let command = json.as_ref().map_err(Clone::clone).and_then(Command::parse);
            taken.push(Taken { name, json: json.ok(), command });
        }
        taken
    }

    /// A path for `name` in `dir` that holds no file yet: `name`, else `name.1`, `name.2`, ...
    fn free_name(&self, dir: &Path, name: &str) -> PathBuf {
        let mut path = dir.join(name);
        let mut k = 0;
        while path.exists() {
            k += 1;
            path = dir.join(format!("{name}.{k}"));
        }
        path
    }

    /// Writes the outcome of the command `taken` in `out/`, under its name: what it said, how it ended, and why.
    pub(super) fn answer(&self, name: &str, json: Option<&Json>, status: Status, reason: &str) {
        let outcome = Json::object([
            ("command", json.cloned().unwrap_or(Json::Null)),
            ("status", Json::from(status.as_str())),
            ("reason", Json::from(reason)),
            ("at", Json::from(utc(SystemTime::now()))),
        ]);
        self.write(name, &outcome);
        say(format_args!("{name}: {} ({reason})", status.as_str()));
    }

    /// Writes `json` to `out/name`, whole: written aside under a dotted name and renamed.
    pub(super) fn write(&self, name: &str, json: &Json) {
        write_whole(&self.out.join(name), &format!("{json}\n"));
    }
}

/// Writes `text` to `path`, whole: written aside, synced and renamed, so that a reader sees the old file or the new,
/// never half of one.  A failure is said on stderr: the lab's files are no part of a result.
fn write_whole(path: &Path, text: &str) {
    let name = path.file_name().map_or_else(String::new, |name| name.to_string_lossy().into_owned());
    let aside = path.with_file_name(format!(".{name}.tmp"));
    let written = fs::File::create(&aside)
        .and_then(|mut file| {
            file.write_all(text.as_bytes())?;
            file.sync_all()
        })
        .and_then(|()| fs::rename(&aside, path));
    if let Err(e) = written {
        say(format_args!("cannot write {}: {e}", path.display()));
    }
}

/// A line on stderr about the lab channel, ignoring a stderr that cannot be written to.
pub(super) fn say(what: std::fmt::Arguments<'_>) {
    let _ = writeln!(std::io::stderr(), "lab: {what}");
}

// ---------------------------------------------------------------------------------------------------------
// The hook

/// Starts `program` with `event` as its one argument, detached: in a process group of its own, with no input or
/// output, never waited for by the run (a thread of its own collects it when it ends), its failure ignored.  Each
/// start, and each failure to start, is said on stderr.
pub(super) fn spawn_hook(program: &Path, event: &Json) {
    let what = event.get("event").and_then(Json::as_str).unwrap_or("?");
    let n = event.get("n").map_or_else(String::new, ToString::to_string);
    let mut command = Process::new(program);
    command.arg(event.to_string()).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    match command.spawn() {
        Ok(mut child) => {
            let _ =
                writeln!(std::io::stderr(), "hook: {what} n={n}: started {} (pid {})", program.display(), child.id());
            // Collected when it ends, so that it leaves no zombie; the run does not wait for it.
            let _ = thread::Builder::new().name("hook".into()).spawn(move || child.wait());
        }
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "hook: {what} n={n}: cannot start {}: {e}", program.display());
        }
    }
}

/// What the hook is told, and when.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Event {
    /// The game's own search has passed `--hook-after`.
    Long,
    /// The game, which had `Long`, has its line.
    Done,
}

/// A game's search as the hook is told of it: its steps over every run, its best promotion sum so far, and its own
/// search time (the clock of the wall).
#[derive(Clone, Copy, Debug)]
pub(super) struct Progress {
    pub(super) nodes: Option<u64>,
    pub(super) best: Option<u64>,
    pub(super) own: Duration,
}

/// The one argument of the hook (solver/docs/design/lab.md, "The hook"): `event`, `n`, `nodes`, `best`,
/// `own_seconds`, `checkpoint_dir` and `lab_dir` (absolute paths, or null), `commit` (the build's), and for `done`
/// `result`, the game's line; then `pid` and `at`, the time in UTC.
pub(super) fn hook_event(event: Event, n: u32, progress: Progress, opts: &SolveOptions, result: Option<Json>) -> Json {
    let path = |dir: Option<&PathBuf>| {
        let dir = dir.map(|dir| std::path::absolute(dir).unwrap_or_else(|_| dir.clone()));
        dir.map_or(Json::Null, |dir| Json::from(dir.display().to_string()))
    };
    let name = match event {
        Event::Long => "long",
        Event::Done => "done",
    };
    let fields = [
        ("event", Json::from(name)),
        ("n", Json::from(n)),
        ("nodes", progress.nodes.map_or(Json::Null, Json::from)),
        ("best", progress.best.map_or(Json::Null, Json::from)),
        ("own_seconds", Json::Num(progress.own.as_secs_f64())),
        ("checkpoint_dir", path(opts.checkpoint_dir.as_ref())),
        ("lab_dir", path(opts.lab.dir.as_ref())),
        ("commit", build().commit.map_or(Json::Null, Json::from)),
    ];
    let result = result.map(|line| ("result", line));
    let more = [("pid", Json::from(std::process::id())), ("at", Json::from(utc(SystemTime::now())))];
    Json::object(fields.into_iter().chain(result).chain(more))
}

// ---------------------------------------------------------------------------------------------------------
// What the lab did to a game

/// A restart of a game's search by the lab.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Restarted {
    /// When, in UTC.
    pub(super) at: String,
    /// The game's own search time then (its wall clock, "The wall" in `window`), in seconds.
    pub(super) own_seconds: f64,
    /// The steps of the search abandoned, over all its runs, if known.
    pub(super) nodes: Option<u64>,
    /// The tactics it was started again with, as a results line writes them (`default`, or the changes).
    pub(super) tactics: String,
    /// The name its checkpoint was given, aside in the checkpoint directory, if it had one.
    pub(super) aside: Option<String>,
}

impl Restarted {
    /// As the game's record and its line write it.
    fn to_json(&self) -> Json {
        Json::object([
            ("at", Json::from(self.at.as_str())),
            ("own_seconds", Json::Num(self.own_seconds)),
            ("nodes", self.nodes.map_or(Json::Null, Json::from)),
            ("tactics", Json::from(self.tactics.as_str())),
            ("aside", self.aside.as_deref().map_or(Json::Null, Json::from)),
        ])
    }
}

/// The file of the lab's record in the checkpoint directory called `name`, if the run has the directory.
fn lab_file(opts: &SolveOptions, name: &str) -> Option<PathBuf> {
    opts.checkpoint_dir.as_ref().map(|dir| dir.join(name))
}

/// What `read` makes of the JSON in the lab's file `path`, if there is one.  A file that is not one names itself and
/// says what to do (`advice`).
fn read_lab_file<T>(
    path: Option<PathBuf>,
    advice: &str,
    read: impl FnOnce(&Json) -> Result<T, String>,
) -> Result<Option<T>, String> {
    let Some(path) = path.filter(|path| path.exists()) else { return Ok(None) };
    let at = |e: String| format!("{}: {e} ({advice})", path.display());
    let json = Json::from_file(&path).map_err(at)?;
    read(&json).map(Some).map_err(at)
}

/// Keeps `json` in the lab's file `path`, whole; a failure is said on stderr.
fn keep_lab_file(path: Option<PathBuf>, json: &Json) {
    if let Some(path) = path {
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        write_whole(&path, &format!("{json}\n"));
    }
}

/// The name of the file of the run's policies in the checkpoint directory.
const POLICY_FILE: &str = "run.lab";

/// A `policy` command as the run keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PolicyEntry {
    /// The first game it applies to.
    pub(super) from: u32,
    /// When it was given, in UTC, and the name of the file of `in/` that held it.
    pub(super) at: String,
    pub(super) file: String,
    /// The settings it gave; a setting it left out is `None`.
    pub(super) conflict_branching: Option<bool>,
    pub(super) conflict_look: Option<bool>,
    pub(super) gate: Option<u64>,
}

impl PolicyEntry {
    fn to_json(&self) -> Json {
        Json::object([
            ("from", Json::from(self.from)),
            ("at", Json::from(self.at.as_str())),
            ("file", Json::from(self.file.as_str())),
            ("conflict_branching", setting_json(self.conflict_branching)),
            ("conflict_look", setting_json(self.conflict_look)),
            ("gate", gate_json(self.gate)),
        ])
    }

    fn from_json(json: &Json) -> Result<PolicyEntry, String> {
        let text = |key: &str| json.get(key).and_then(Json::as_str).map(str::to_owned).ok_or(format!("no {key}"));
        let setting = |key: &str| match json.get(key) {
            None | Some(Json::Null) => Ok(None),
            Some(value) => on_off(value, key).map(Some),
        };
        let from = json.get("from").and_then(Json::as_u32).filter(|&from| from >= 1).ok_or("no from, a game")?;
        Ok(PolicyEntry {
            from,
            at: text("at")?,
            file: text("file")?,
            conflict_branching: setting("conflict_branching")?,
            conflict_look: setting("conflict_look")?,
            gate: gate_field(json)?,
        })
    }
}

/// The policy in force for one game: the settings of the policies given for it, each over the last, and the `from` of
/// the last of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Policy {
    pub(super) from: u32,
    pub(super) conflict_branching: Option<bool>,
    pub(super) conflict_look: Option<bool>,
    pub(super) gate: Option<u64>,
}

impl Policy {
    /// The tactics `run` with this policy's switches (its gate is no tactic: [`GameLab::gate`]).
    fn over(&self, run: &SearchTactics) -> SearchTactics {
        SearchTactics {
            conflict_branching: self.conflict_branching.unwrap_or(run.conflict_branching),
            conflict_look: self.conflict_look.unwrap_or(run.conflict_look),
            ..*run
        }
    }

    fn to_json(self) -> Json {
        Json::object([
            ("from", Json::from(self.from)),
            ("conflict_branching", setting_json(self.conflict_branching)),
            ("conflict_look", setting_json(self.conflict_look)),
            ("gate", gate_json(self.gate)),
        ])
    }
}

/// The policies the lab gave for the games to come (`policy`), in the order given.  They live in `run.lab` in the
/// checkpoint directory, beside the games' records, so that they outlive the process.  Deleting the file returns the
/// run to its flags.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Policies {
    entries: Vec<PolicyEntry>,
}

impl Policies {
    /// The run's policies, from its file: none if there is none.
    pub(super) fn load(opts: &SolveOptions) -> Result<Policies, String> {
        let advice = "the lab's policies for the games to come; delete it to return the run to its flags";
        let read = |json: &Json| {
            let list = json.get("policies").and_then(Json::as_arr).ok_or("no policies, a list")?;
            Ok(Policies { entries: list.iter().map(PolicyEntry::from_json).collect::<Result<_, _>>()? })
        };
        Ok(read_lab_file(lab_file(opts, POLICY_FILE), advice, read)?.unwrap_or_default())
    }

    /// Keeps them in the run's file, whole; a failure is said on stderr.
    pub(super) fn keep(&self, opts: &SolveOptions) {
        let entries = self.entries.iter().map(PolicyEntry::to_json);
        keep_lab_file(lab_file(opts, POLICY_FILE), &Json::object([("policies", Json::Arr(entries.collect()))]));
    }

    /// Adds a policy, after those given.
    pub(super) fn give(&mut self, entry: PolicyEntry) {
        self.entries.push(entry);
    }

    /// The policy for game `n`: the policies with `from` at most `n`, in the order given, each setting over the last;
    /// none if no policy reaches it.
    pub(super) fn for_game(&self, n: u32) -> Option<Policy> {
        self.entries.iter().filter(|entry| entry.from <= n).fold(None, |policy, entry| {
            let before = policy.unwrap_or(Policy {
                from: entry.from,
                conflict_branching: None,
                conflict_look: None,
                gate: None,
            });
            Some(Policy {
                from: entry.from,
                conflict_branching: entry.conflict_branching.or(before.conflict_branching),
                conflict_look: entry.conflict_look.or(before.conflict_look),
                gate: entry.gate.or(before.gate),
            })
        })
    }
}

/// What the lab did to a game, kept beside its checkpoint (`n<NNNN>.lab`) so that it outlives the process, and
/// written into the game's line: its restarts, the search tactics of the last one, and its settings of
/// `conflict_branching`, `conflict_look` and `gate`.  It also keeps whether the hook was told that the game runs long,
/// so that it is told once.  The policy the game is searched under is in it for the line, but not in its file:
/// `run.lab` keeps the policies, and a game set up again looks its own up.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct GameLab {
    /// The search tactics the lab restarted the game with, if it did: the game's search runs with them, and so does a
    /// resume of it.
    pub(super) tactics: Option<SearchTactics>,
    /// The lab's settings of `conflict_branching`, `conflict_look` and `gate`, if it gave one: each holds over the
    /// tactics, and over the policy.
    pub(super) conflict_branching: Option<bool>,
    pub(super) conflict_look: Option<bool>,
    pub(super) gate: Option<u64>,
    /// How many `branching` commands the lab gave.
    pub(super) branching_changes: u32,
    pub(super) restarts: Vec<Restarted>,
    /// Whether the hook was told that the game runs long.
    pub(super) long: bool,
    /// The policy for the games to come that the game is searched under, if one reaches it ([`apply_policy`]).
    ///
    /// [`apply_policy`]: GameLab::apply_policy
    pub(super) policy: Option<Policy>,
}

impl GameLab {
    /// The file of game `n`, if there is a checkpoint directory.
    fn path(opts: &SolveOptions, n: u32) -> Option<PathBuf> {
        lab_file(opts, &format!("n{n:04}.lab"))
    }

    /// What the lab did to game `n`, from its file: nothing if there is none.
    pub(super) fn load(opts: &SolveOptions, n: u32) -> Result<GameLab, String> {
        let advice = "the lab's record of the game; delete it to start the game over";
        Ok(read_lab_file(Self::path(opts, n), advice, Self::from_json)?.unwrap_or_default())
    }

    /// Keeps it in game `n`'s file, whole; a failure is said on stderr.
    pub(super) fn keep(&self, opts: &SolveOptions, n: u32) {
        keep_lab_file(Self::path(opts, n), &self.to_json());
    }

    /// Deletes game `n`'s file, once its line is safe.
    pub(super) fn forget(opts: &SolveOptions, n: u32) {
        if let Some(path) = Self::path(opts, n) {
            let _ = fs::remove_file(path);
        }
    }

    /// Whether the lab changed how the game was searched: a restart, a policy for the games to come, or a setting of
    /// `conflict_branching`, `conflict_look` or `gate`.
    pub(super) fn steered(&self) -> bool {
        !self.restarts.is_empty()
            || self.policy.is_some()
            || self.conflict_branching.is_some()
            || self.conflict_look.is_some()
            || self.gate.is_some()
    }

    /// Takes the policy for game `n` from `policies`: none for a game that was restarted, which names its tactics
    /// outright.
    pub(super) fn apply_policy(&mut self, policies: &Policies, n: u32) {
        self.policy = policies.for_game(n).filter(|_| self.tactics.is_none());
    }

    /// The search tactics of the game: the lab's, else `run`'s changed by the policy for the games to come, with the
    /// lab's settings of `conflict_branching` and `conflict_look` over them.
    pub(super) fn search_tactics(&self, run: &SearchTactics) -> SearchTactics {
        let tactics = self.tactics.unwrap_or_else(|| self.policy.map_or(*run, |policy| policy.over(run)));
        SearchTactics {
            conflict_branching: self.conflict_branching.unwrap_or(tactics.conflict_branching),
            conflict_look: self.conflict_look.unwrap_or(tactics.conflict_look),
            ..tactics
        }
    }

    /// The gate of the game, the gap from which conflict branching chooses, if the lab set one: its own, else the
    /// policy's.  The gate is no search tactic, so it reaches the searchers through [`branch_settings`].
    ///
    /// [`branch_settings`]: GameLab::branch_settings
    pub(super) fn gate(&self) -> Option<u64> {
        self.gate.or(self.policy.and_then(|policy| policy.gate))
    }

    /// The gate the game's searchers start with, if anything sets one: the lab's, else `run`, the run's own (`--gate`);
    /// none: the default gate.
    pub(super) fn starting_gate(&self, run: Option<u64>) -> Option<u64> {
        self.gate().or(run)
    }

    /// The branching settings the game's searchers start with: those of [`search_tactics`] and the game's
    /// [`starting_gate`], `run_gate` being the run's own.
    ///
    /// [`search_tactics`]: GameLab::search_tactics
    /// [`starting_gate`]: GameLab::starting_gate
    pub(super) fn branch_settings(&self, run: &SearchTactics, run_gate: Option<u64>) -> BranchSettings {
        BranchSettings::of(&self.search_tactics(run)).with_gate(self.starting_gate(run_gate))
    }

    /// What a results line records of it (`lab`), if the lab changed how the game was searched.
    pub(super) fn record(&self) -> Option<Json> {
        self.steered().then(|| {
            Json::object([
                ("restarts", Json::Arr(self.restarts.iter().map(Restarted::to_json).collect())),
                ("conflict_branching", setting_json(self.conflict_branching)),
                ("conflict_look", setting_json(self.conflict_look)),
                ("gate", gate_json(self.gate)),
                ("branching_changes", Json::from(self.branching_changes)),
                ("policy", self.policy.map_or(Json::Null, Policy::to_json)),
            ])
        })
    }

    fn to_json(&self) -> Json {
        // Every search tactic, by name, so that the record means the same to a build whose defaults differ.
        let tactics = self.tactics.map_or(Json::Null, |search| {
            let all = Tactics { search, ..Tactics::default() };
            Json::array(all.settings().map(|(name, value)| Json::from(format!("{name}={value}"))))
        });
        Json::object([
            ("tactics", tactics),
            ("conflict_branching", self.conflict_branching.map_or(Json::Null, Json::Bool)),
            ("conflict_look", self.conflict_look.map_or(Json::Null, Json::Bool)),
            ("gate", gate_json(self.gate)),
            ("branching_changes", Json::from(self.branching_changes)),
            ("restarts", Json::Arr(self.restarts.iter().map(Restarted::to_json).collect())),
            ("long", Json::Bool(self.long)),
        ])
    }

    fn from_json(json: &Json) -> Result<GameLab, String> {
        let tactics = match json.get("tactics") {
            None | Some(Json::Null) => None,
            Some(list) => {
                let mut all = Tactics::default();
                for setting in list.as_arr().ok_or("tactics is not a list")? {
                    all.set(setting.as_str().ok_or("a tactic is not a string")?)?;
                }
                Some(all.search)
            }
        };
        let switch = |key: &str| match json.get(key) {
            None | Some(Json::Null) => Ok(None),
            Some(on) => on.as_bool().map(Some).ok_or(format!("{key} is not true, false or null")),
        };
        let mut restarts = Vec::new();
        for r in json.get("restarts").and_then(Json::as_arr).unwrap_or_default() {
            let text = |key: &str| r.get(key).and_then(Json::as_str).map(str::to_owned);
            restarts.push(Restarted {
                at: text("at").ok_or("a restart without its time")?,
                own_seconds: r.get("own_seconds").and_then(Json::as_f64).unwrap_or(0.0),
                nodes: r.get("nodes").and_then(Json::as_u64),
                tactics: text("tactics").unwrap_or_default(),
                aside: text("aside"),
            });
        }
        Ok(GameLab {
            tactics,
            conflict_branching: switch("conflict_branching")?,
            conflict_look: switch("conflict_look")?,
            gate: gate_field(json)?,
            branching_changes: json.get("branching_changes").and_then(Json::as_u32).unwrap_or(0),
            restarts,
            long: json.get("long").and_then(Json::as_bool).unwrap_or(false),
            policy: None,
        })
    }
}

/// `time` in UTC, as `2026-10-21T12:34:56Z`.
pub(super) fn utc(time: SystemTime) -> String {
    let seconds = time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rest) = (seconds / 86_400, seconds % 86_400);
    // The civil date of a day count since 1970-01-01 (H. Hinnant's days_from_civil, inverted).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rest / 3600, rest / 60 % 60, rest % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::CONFLICT_BRANCHING_GAP;
    use crate::test_support::TempDir;

    fn command(text: &str) -> Result<Command, String> {
        Command::parse(&Json::parse(text).unwrap())
    }

    #[test]
    fn commands_are_read_with_their_fields_and_refused_with_a_reason() {
        assert_eq!(command(r#"{"command":"checkpoint"}"#), Ok(Command::Checkpoint));
        assert_eq!(
            command(r#"{"command":"offer","n":700,"set":[350,351],"value":701}"#),
            Ok(Command::Offer { n: 700, set: vec![350, 351], value: Some(701) })
        );
        assert_eq!(
            command(r#"{"command":"branching","n":9,"conflict_branching":"off"}"#),
            Ok(Command::Branching { n: 9, conflict_branching: Some(false), conflict_look: None, gate: None })
        );
        assert_eq!(
            command(r#"{"command":"branching","n":9,"conflict_look":"on"}"#),
            Ok(Command::Branching { n: 9, conflict_branching: None, conflict_look: Some(true), gate: None })
        );
        assert_eq!(
            command(r#"{"command":"branching","n":9,"conflict_branching":"on","conflict_look":"off"}"#),
            Ok(Command::Branching { n: 9, conflict_branching: Some(true), conflict_look: Some(false), gate: None })
        );
        assert_eq!(
            command(r#"{"command":"policy","from":4905,"conflict_branching":"on","conflict_look":"on"}"#),
            Ok(Command::Policy { from: 4905, conflict_branching: Some(true), conflict_look: Some(true), gate: None })
        );
        assert_eq!(
            command(r#"{"command":"policy","from":4905,"conflict_look":"off"}"#),
            Ok(Command::Policy { from: 4905, conflict_branching: None, conflict_look: Some(false), gate: None })
        );
        // The gate, alone or with the switches; 0 and 4,000 are the ends of its range.
        assert_eq!(
            command(r#"{"command":"branching","n":9,"gate":150}"#),
            Ok(Command::Branching { n: 9, conflict_branching: None, conflict_look: None, gate: Some(150) })
        );
        assert_eq!(
            command(r#"{"command":"branching","n":9,"conflict_branching":"on","conflict_look":"on","gate":0}"#),
            Ok(Command::Branching { n: 9, conflict_branching: Some(true), conflict_look: Some(true), gate: Some(0) })
        );
        assert_eq!(
            command(r#"{"command":"policy","from":4905,"gate":4000}"#),
            Ok(Command::Policy { from: 4905, conflict_branching: None, conflict_look: None, gate: Some(4000) })
        );
        assert_eq!(
            command(r#"{"command":"policy","from":4905,"conflict_branching":"on","gate":500}"#),
            Ok(Command::Policy { from: 4905, conflict_branching: Some(true), conflict_look: None, gate: Some(500) })
        );
        assert_eq!(
            command(r#"{"command":"restart","n":9,"tactics":"symmetry=off, gf2=on"}"#),
            Ok(Command::Restart { n: 9, tactics: vec!["symmetry=off".into(), "gf2=on".into()] })
        );
        assert_eq!(command(r#"{"command":"restart","n":9}"#), Ok(Command::Restart { n: 9, tactics: vec![] }));
        assert_eq!(command(r#"{"command":"threads","total":64}"#), Ok(Command::Threads { total: 64 }));
        assert_eq!(
            command(r#"{"command":"threads","n":9,"workers":0}"#),
            Ok(Command::Workers { n: 9, workers: Some(0) })
        );
        assert_eq!(
            command(r#"{"command":"threads","n":9,"workers":null}"#),
            Ok(Command::Workers { n: 9, workers: None })
        );
        assert_eq!(
            command(r#"{"command":"finder","n":9,"seconds":1.5}"#),
            Ok(Command::Finder { n: 9, budget: Duration::from_millis(1500) })
        );
        assert_eq!(
            command(r#"{"command":"redeploy","commit":"c2b6d9b"}"#),
            Ok(Command::Redeploy { commit: "c2b6d9b".into() })
        );
        for (text, why) in [
            ("[1]", "a JSON object"),
            (r#"{"n":3}"#, "\"command\" field"),
            (r#"{"command":"close","n":3}"#, "unknown command"),
            (r#"{"command":"advice","n":3}"#, "reserved"),
            (r#"{"command":"checkpoint","now":true}"#, "takes no field \"now\""),
            (r#"{"command":"offer","set":[1]}"#, "needs n"),
            (r#"{"command":"offer","n":0,"set":[1]}"#, "needs n"),
            (r#"{"command":"offer","n":5,"set":[1,-2]}"#, "list of numbers"),
            (r#"{"command":"offer","n":5,"set":[1],"value":-1}"#, "whole number"),
            (r#"{"command":"branching","n":5,"conflict_branching":true}"#, "\"on\" or \"off\""),
            (r#"{"command":"branching","n":5,"conflict_look":"maybe"}"#, "conflict_look is \"on\" or \"off\""),
            (r#"{"command":"branching","n":5,"conflict_look":null}"#, "conflict_look is \"on\" or \"off\""),
            (r#"{"command":"branching","n":5,"conflict_branching":"on","conflict_look":1}"#, "\"on\" or \"off\""),
            (r#"{"command":"branching","n":5}"#, "needs conflict_branching, conflict_look or gate"),
            (r#"{"command":"branching","n":5,"gate":-1}"#, "gate is a whole number from 0 to 4000"),
            (r#"{"command":"branching","n":5,"gate":4001}"#, "gate is a whole number from 0 to 4000"),
            (r#"{"command":"branching","n":5,"gate":300.5}"#, "gate is a whole number from 0 to 4000"),
            (r#"{"command":"branching","n":5,"gate":"300"}"#, "gate is a whole number from 0 to 4000"),
            (r#"{"command":"branching","n":5,"gate":null}"#, "gate is a whole number from 0 to 4000"),
            (r#"{"command":"branching","n":5,"gate":150,"conflict_branching":"off"}"#, "gate"),
            (
                r#"{"command":"branching","n":5,"gate":150,"conflict_branching":"off","conflict_look":"on"}"#,
                "turns it off",
            ),
            (r#"{"command":"policy","from":5,"gate":99999}"#, "gate is a whole number from 0 to 4000"),
            (r#"{"command":"policy","from":5,"gate":150,"conflict_branching":"off"}"#, "turns it off"),
            (r#"{"command":"branching","n":5,"look":"on"}"#, "takes no field \"look\""),
            (r#"{"command":"policy","conflict_look":"on"}"#, "policy needs from, a game"),
            (r#"{"command":"policy","from":0,"conflict_look":"on"}"#, "policy needs from, a game"),
            (r#"{"command":"policy","from":"next","conflict_look":"on"}"#, "policy needs from, a game"),
            (r#"{"command":"policy","from":5}"#, "policy needs conflict_branching, conflict_look or gate"),
            (r#"{"command":"policy","from":5,"conflict_branching":true}"#, "conflict_branching is \"on\" or \"off\""),
            (r#"{"command":"policy","from":5,"conflict_look":null}"#, "conflict_look is \"on\" or \"off\""),
            (r#"{"command":"policy","from":5,"conflict_look":"on","n":5}"#, "policy takes no field \"n\""),
            (r#"{"command":"branching","from":5,"conflict_look":"on"}"#, "branching takes no field \"from\""),
            (r#"{"command":"restart","n":5,"tactics":["gf2=off"]}"#, "string of name=value"),
            (r#"{"command":"threads","total":0}"#, "from 1 to"),
            (r#"{"command":"threads","total":4,"n":3}"#, "total, or n and workers"),
            (r#"{"command":"threads","n":3}"#, "needs workers"),
            (r#"{"command":"threads","n":3,"workers":-1}"#, "workers is a whole number"),
            (r#"{"command":"finder","n":3,"seconds":0}"#, "needs seconds"),
            (r#"{"command":"redeploy","commit":"HEAD"}"#, "hexadecimal"),
            (r#"{"command":"redeploy","commit":"abc"}"#, "hexadecimal"),
        ] {
            let refused = command(text).unwrap_err();
            assert!(refused.contains(why), "{text}: {refused}");
        }
    }

    #[test]
    fn a_restart_changes_the_search_tactics_only() {
        let run = Tactics::default();
        let tactics = restart_tactics(&run, &["symmetry=off".into(), "strong_depth=30".into()]).unwrap();
        assert_eq!(tactics, SearchTactics { symmetry: false, strong_depth: 30, ..run.search });
        assert_eq!(restart_tactics(&run, &[]), Ok(run.search));
        assert!(restart_tactics(&run, &["prev_cap=off".into()]).unwrap_err().contains("not a search tactic"));
        assert!(restart_tactics(&run, &["symmetry=maybe".into()]).unwrap_err().contains("on or off"));
    }

    #[test]
    fn a_games_record_is_kept_beside_its_checkpoint_and_read_back() {
        let dir = TempDir::new("game-lab");
        let opts = SolveOptions { checkpoint_dir: Some(dir.join("ckpt")), ..SolveOptions::default() };
        assert_eq!(GameLab::load(&opts, 77), Ok(GameLab::default()));
        let lab = GameLab {
            tactics: Some(SearchTactics { symmetry: false, ..SearchTactics::default() }),
            conflict_branching: Some(false),
            conflict_look: Some(true),
            gate: Some(150),
            branching_changes: 3,
            restarts: vec![Restarted {
                at: utc(SystemTime::now()),
                own_seconds: 12.5,
                nodes: Some(1234),
                tactics: "symmetry=off".into(),
                aside: Some("n0077.ckpt.restart1".into()),
            }],
            long: true,
            policy: None,
        };
        lab.keep(&opts, 77);
        assert_eq!(GameLab::load(&opts, 77), Ok(lab.clone()));
        let tactics = lab.search_tactics(&SearchTactics::default());
        let expected = SearchTactics {
            symmetry: false,
            conflict_branching: false,
            conflict_look: true,
            ..SearchTactics::default()
        };
        assert_eq!(tactics, expected);
        let record = lab.record().unwrap();
        assert_eq!(record.get("conflict_branching").and_then(Json::as_str), Some("off"));
        assert_eq!(record.get("conflict_look").and_then(Json::as_str), Some("on"));
        assert_eq!(record.get("gate").and_then(Json::as_u64), Some(150));
        // The gate is in the file, and reaches the branching settings of the game's searchers.
        let file = Json::from_file(dir.join("ckpt/n0077.lab")).unwrap();
        assert_eq!(file.get("gate").and_then(Json::as_u64), Some(150));
        let settings = lab.branch_settings(&SearchTactics::default(), None);
        assert_eq!(settings, BranchSettings { conflict_branching: false, gate: 150, conflict_look: true });
        // A setting of the look alone steers the game, and leaves the branching and the gate to the tactics.
        let looking = GameLab { conflict_look: Some(true), ..GameLab::default() };
        assert!(looking.steered());
        assert!(looking.search_tactics(&SearchTactics::default()).conflict_look);
        assert_eq!(looking.record().unwrap().get("conflict_branching"), Some(&Json::Null));
        assert_eq!(looking.record().unwrap().get("gate"), Some(&Json::Null));
        assert_eq!(
            looking.branch_settings(&SearchTactics::default(), None).conflict_gap(),
            Some(CONFLICT_BRANCHING_GAP)
        );
        // A setting of the gate alone steers the game too, and changes the gap of the tactics and nothing else.
        let gated = GameLab { gate: Some(500), ..GameLab::default() };
        assert!(gated.steered());
        let run = SearchTactics::default();
        assert_eq!(gated.search_tactics(&run), run);
        assert_eq!(gated.branch_settings(&run, None).conflict_gap(), Some(500));
        let off = SearchTactics { conflict_branching: false, ..run };
        assert_eq!(
            gated.branch_settings(&off, None),
            BranchSettings { conflict_branching: false, gate: 500, ..BranchSettings::of(&off) }
        );
        // A record from before the look and the gate has no setting of them.
        let old = Json::parse(r#"{"conflict_branching":false,"branching_changes":1}"#).unwrap();
        assert_eq!(GameLab::from_json(&old).map(|lab| (lab.conflict_look, lab.gate)), Ok((None, None)));
        // A gate out of its range in a record is an error, whichever way it is written.
        for gate in ["4001", "-3", "\"150\"", "1.5"] {
            let bad = Json::parse(&format!(r#"{{"gate":{gate}}}"#)).unwrap();
            assert!(GameLab::from_json(&bad).unwrap_err().contains("gate is a whole number"), "{gate}");
        }
        assert_eq!(record.get("restarts").and_then(Json::as_arr).map(<[_]>::len), Some(1));
        GameLab::forget(&opts, 77);
        assert_eq!(GameLab::load(&opts, 77), Ok(GameLab::default()));
        // A record of the hook alone changes nothing of the line.
        assert_eq!(GameLab { long: true, ..GameLab::default() }.record(), None);
        // A record that is not one is an error, which names the file.
        fs::write(dir.join("ckpt/n0078.lab"), "{\"tactics\":[\"gf2=maybe\"]}").unwrap();
        assert!(GameLab::load(&opts, 78).unwrap_err().contains("n0078.lab"));
    }

    fn entry(
        from: u32,
        conflict_branching: Option<bool>,
        conflict_look: Option<bool>,
        gate: Option<u64>,
    ) -> PolicyEntry {
        PolicyEntry { from, at: utc(UNIX_EPOCH), file: format!("{from}.json"), conflict_branching, conflict_look, gate }
    }

    #[test]
    fn the_policy_for_a_game_is_the_policies_that_reach_it_each_over_the_last() {
        let mut policies = Policies::default();
        assert_eq!(policies.for_game(5000), None);
        policies.give(entry(4905, Some(false), None, Some(150)));
        policies.give(entry(4910, None, Some(true), None));
        // Given later, but reaching the games before the one above: it comes after it in the order given.
        policies.give(entry(4900, Some(true), None, None));
        let policy = |from, conflict_branching, conflict_look, gate| {
            Some(Policy { from, conflict_branching, conflict_look, gate })
        };
        assert_eq!(policies.for_game(4899), None);
        assert_eq!(policies.for_game(4900), policy(4900, Some(true), None, None));
        // The gate of the policy before it is kept by the one that leaves it out, as the switches are: the policy of
        // 4900, given last, leaves it out and keeps the 150 that the policy of 4905 gave before it.
        assert_eq!(policies.for_game(4905), policy(4900, Some(true), None, Some(150)));
        assert_eq!(policies.for_game(4910), policy(4900, Some(true), Some(true), Some(150)));
        policies.give(entry(4920, None, None, Some(500)));
        policies.give(entry(4930, Some(false), None, None));
        assert_eq!(policies.for_game(4920), policy(4920, Some(true), Some(true), Some(500)));
        assert_eq!(policies.for_game(4930), policy(4930, Some(false), Some(true), Some(500)));
        // A policy over the run's tactics changes the switches it gives and nothing else.
        let run = SearchTactics { symmetry: false, ..SearchTactics::default() };
        let over = policy(4900, Some(false), None, Some(150)).unwrap().over(&run);
        assert_eq!(over, SearchTactics { conflict_branching: false, ..run });
        assert_eq!(policy(4900, None, None, None).unwrap().over(&run), run);
    }

    #[test]
    fn the_policies_are_kept_in_the_checkpoint_directory_and_read_back() {
        let dir = TempDir::new("policies");
        let opts = SolveOptions { checkpoint_dir: Some(dir.join("ckpt")), ..SolveOptions::default() };
        assert_eq!(Policies::load(&opts), Ok(Policies::default()));
        let mut policies = Policies::default();
        policies.give(entry(4905, Some(false), None, Some(150)));
        policies.give(entry(4910, Some(true), Some(false), None));
        policies.keep(&opts);
        assert_eq!(Policies::load(&opts), Ok(policies.clone()));
        // The file is a JSON object with the list, an entry with the command's name and the settings as it gave them.
        let json = Json::from_file(dir.join("ckpt/run.lab")).unwrap();
        let list = json.get("policies").and_then(Json::as_arr).unwrap();
        assert_eq!(list.len(), 2);
        let (first, second) = (&list[0], &list[1]);
        assert_eq!(first.get("from").and_then(Json::as_u32), Some(4905));
        assert_eq!(first.get("file").and_then(Json::as_str), Some("4905.json"));
        assert_eq!(first.get("at").and_then(Json::as_str), Some("1970-01-01T00:00:00Z"));
        assert_eq!(first.get("conflict_branching").and_then(Json::as_str), Some("off"));
        assert_eq!(first.get("conflict_look"), Some(&Json::Null));
        assert_eq!(first.get("gate").and_then(Json::as_u64), Some(150));
        assert_eq!(second.get("conflict_look").and_then(Json::as_str), Some("off"));
        assert_eq!(second.get("gate"), Some(&Json::Null));
        // A file from before the gate has none.
        let old = r#"{"policies":[{"from":7,"at":"x","file":"y","conflict_look":"on"}]}"#;
        fs::write(dir.join("ckpt/run.lab"), old).unwrap();
        assert_eq!(
            Policies::load(&opts).map(|policies| policies.for_game(7).map(|policy| policy.gate)),
            Ok(Some(None))
        );
        // A run without a checkpoint directory keeps nothing and reads nothing.
        let none = SolveOptions::default();
        policies.keep(&none);
        assert_eq!(Policies::load(&none), Ok(Policies::default()));
        // A file that is not one is an error, which names the file and says how to be rid of it.
        for text in [
            r#"{"policies":[{"from":0,"at":"x","file":"y"}]}"#,
            r#"{"policies":3}"#,
            r#"{"policies":[{"from":7,"at":"x","file":"y","conflict_look":true}]}"#,
            r#"{"policies":[{"from":7,"at":"x","file":"y","gate":4001}]}"#,
            r#"{"policies":[{"from":7,"at":"x","file":"y","gate":"150"}]}"#,
        ] {
            fs::write(dir.join("ckpt/run.lab"), text).unwrap();
            let error = Policies::load(&opts).unwrap_err();
            assert!(error.contains("run.lab") && error.contains("delete it"), "{text}: {error}");
        }
    }

    #[test]
    fn a_games_tactics_are_the_runs_then_the_policy_then_its_own() {
        // A run with the look off (it is on by default), so that the policy's look changes its tactics.
        let run = SearchTactics { conflict_look: false, ..SearchTactics::default() };
        assert!(run.conflict_branching);
        let mut policies = Policies::default();
        policies.give(entry(10, Some(false), Some(true), Some(150)));
        let mut lab = GameLab::default();
        // No policy reaches game 9: it is searched as the run is, and its line says nothing of a lab.
        lab.apply_policy(&policies, 9);
        assert_eq!((lab.search_tactics(&run), lab.steered(), lab.record()), (run, false, None));
        assert_eq!((lab.gate(), lab.branch_settings(&run, None)), (None, BranchSettings::of(&run)));
        // The policy changes the run's tactics for game 10, and its line records it.
        lab.apply_policy(&policies, 10);
        let policy = Policy { from: 10, conflict_branching: Some(false), conflict_look: Some(true), gate: Some(150) };
        assert_eq!(lab.policy, Some(policy));
        assert_eq!(lab.search_tactics(&run), SearchTactics { conflict_branching: false, conflict_look: true, ..run });
        // The gate is no tactic: it reaches the searchers through the branching settings, kept while conflict
        // branching is off.
        assert_eq!(lab.gate(), Some(150));
        let settings = BranchSettings { conflict_branching: false, gate: 150, conflict_look: true };
        assert_eq!(lab.branch_settings(&run, None), settings);
        let record = lab.record().unwrap();
        let recorded = record.get("policy").unwrap();
        assert_eq!(recorded.get("from").and_then(Json::as_u32), Some(10));
        assert_eq!(recorded.get("conflict_branching").and_then(Json::as_str), Some("off"));
        assert_eq!(recorded.get("conflict_look").and_then(Json::as_str), Some("on"));
        assert_eq!(recorded.get("gate").and_then(Json::as_u64), Some(150));
        assert_eq!(record.get("conflict_branching"), Some(&Json::Null));
        assert_eq!(record.get("gate"), Some(&Json::Null));
        // The game's own setting of a switch holds over the policy, whichever came first; the other switch keeps it.
        let own = GameLab { conflict_branching: Some(true), ..lab.clone() };
        assert_eq!(own.search_tactics(&run), SearchTactics { conflict_look: true, ..run });
        // So does its own gate, and the policy's gate stays for a game that sets none.
        let gated = GameLab { gate: Some(500), ..lab.clone() };
        assert_eq!(gated.gate(), Some(500));
        assert_eq!(gated.branch_settings(&run, None), BranchSettings { gate: 500, ..settings });
        assert_eq!(own.gate(), Some(150));
        // The run's own gate (`--gate`) starts a game that nothing else gives one, and the game's own and its policy's
        // hold over it.
        let plain = GameLab::default();
        assert_eq!(plain.starting_gate(Some(1200)), Some(1200));
        assert_eq!(plain.branch_settings(&run, Some(1200)), BranchSettings { gate: 1200, ..BranchSettings::of(&run) });
        assert_eq!(plain.starting_gate(None), None);
        assert_eq!(lab.branch_settings(&run, Some(1200)), settings);
        assert_eq!(gated.branch_settings(&run, Some(1200)).gate, 500);
        let mut gateless = Policies::default();
        gateless.give(entry(10, Some(false), Some(true), None));
        let mut reached = GameLab::default();
        reached.apply_policy(&gateless, 10);
        assert_eq!(reached.branch_settings(&run, Some(1200)), BranchSettings { gate: 1200, ..settings });
        // The tactics of a restart are named outright: the policy does not reach the game, and its line has none.
        let again = Restarted {
            at: utc(UNIX_EPOCH),
            own_seconds: 0.0,
            nodes: None,
            tactics: "symmetry=off".into(),
            aside: None,
        };
        let mut restarted = GameLab {
            tactics: Some(SearchTactics { symmetry: false, ..run }),
            restarts: vec![again],
            ..GameLab::default()
        };
        restarted.apply_policy(&policies, 10);
        assert_eq!(restarted.policy, None);
        assert_eq!(restarted.gate(), None);
        assert_eq!(restarted.search_tactics(&run), SearchTactics { symmetry: false, ..run });
        assert_eq!(restarted.record().unwrap().get("policy"), Some(&Json::Null));
        // The policy is not in the game's file: it is looked up again, by whoever sets the game up.
        assert!(lab.to_json().get("policy").is_none());
    }

    #[test]
    fn the_folder_takes_commands_in_order_and_answers_each() {
        let dir = TempDir::new("lab-folder");
        let folder = Folder::open(&dir.0).unwrap();
        assert!(folder.take().is_empty());
        fs::write(dir.join("in/2-b.json"), r#"{"command":"checkpoint"}"#).unwrap();
        fs::write(dir.join("in/1-a.json"), "not json").unwrap();
        fs::write(dir.join("in/.3-c.json"), r#"{"command":"checkpoint"}"#).unwrap();
        let taken = folder.take();
        assert_eq!(taken.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), ["1-a.json", "2-b.json"]);
        assert!(taken[0].command.as_ref().unwrap_err().contains("JSON syntax error"));
        assert_eq!(taken[1].command, Ok(Command::Checkpoint));
        assert!(dir.join("done/1-a.json").exists() && dir.join("in/.3-c.json").exists());
        // A name taken again goes beside the first in done/.
        fs::write(dir.join("in/2-b.json"), r#"{"command":"checkpoint"}"#).unwrap();
        assert_eq!(folder.take().len(), 1);
        assert!(dir.join("done/2-b.json.1").exists());
        folder.answer("2-b.json", taken[1].json.as_ref(), Status::Refused, "no checkpoint directory");
        let outcome = Json::from_file(dir.join("out/2-b.json")).unwrap();
        assert_eq!(outcome.get("status").and_then(Json::as_str), Some("refused"));
        assert_eq!(outcome.get("reason").and_then(Json::as_str), Some("no checkpoint directory"));
    }

    #[test]
    fn times_are_written_in_utc() {
        assert_eq!(utc(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        assert_eq!(utc(UNIX_EPOCH + Duration::from_secs(951_782_400 + 3661)), "2000-02-29T01:01:01Z");
        assert_eq!(utc(UNIX_EPOCH + Duration::from_secs(1_792_540_800)), "2026-10-21T00:00:00Z");
    }
}
