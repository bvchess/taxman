//! The lab channel of a range: the hook that tells a lab when a game runs long (`--hook`), and the command folder
//! through which a lab steers the games in flight (`--lab-dir`).  solver/docs/design/lab.md has the design and the
//! argument that nothing the lab does can make a result wrong; solver/docs/REFERENCE.md has the format of the events
//! and the commands.  The driver of the window ([`window`](super::window)) acts on both; this module has what they
//! are made of: the options ([`LabOptions`]), the commands and their checks ([`Command`]), the folder ([`Folder`]),
//! the hook's events ([`spawn_hook`]), and what the lab did to a game, kept beside its checkpoint ([`GameLab`]).
//!
//! Without `--lab-dir` and `--hook` nothing here runs.  With an empty folder, the cost is one directory read at each
//! progress tick, on the driver's thread.

use super::SolveOptions;
use crate::json::Json;
use crate::results::build;
use crate::search::SearchTactics;
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
    /// Switch `conflict_branching` and `conflict_look`, those given, on or off in game `n`'s search, for the units it
    /// starts from now on.
    Branching { n: u32, conflict_branching: Option<bool>, conflict_look: Option<bool> },
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
            "branching" => &["n", "conflict_branching", "conflict_look"],
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
        let n = || json.get("n").and_then(Json::as_u32).filter(|&n| n >= 1).ok_or(format!("{name} needs n, a game"));
        match name {
            "checkpoint" => Ok(Command::Checkpoint),
            "offer" => {
                let set = json.get("set").and_then(Json::as_u32_vec).ok_or("offer needs set, a list of numbers")?;
                let value = match json.get("value") {
                    None => None,
                    Some(value) => Some(value.as_u64().ok_or("value is a whole number")?),
                };
                Ok(Command::Offer { n: n()?, set, value })
            }
            "branching" => {
                let switch = |key: &str| match json.get(key) {
                    None => Ok(None),
                    Some(value) => match value.as_str() {
                        Some("on") => Ok(Some(true)),
                        Some("off") => Ok(Some(false)),
                        _ => Err(format!("{key} is \"on\" or \"off\"")),
                    },
                };
                let (conflict_branching, conflict_look) = (switch("conflict_branching")?, switch("conflict_look")?);
                if conflict_branching.is_none() && conflict_look.is_none() {
                    return Err("branching needs conflict_branching or conflict_look, \"on\" or \"off\"".into());
                }
                Ok(Command::Branching { n: n()?, conflict_branching, conflict_look })
            }
            "restart" => {
                let tactics = match json.get("tactics") {
                    None => Vec::new(),
                    Some(Json::Str(text)) => assignments(text),
                    Some(_) => return Err("tactics is a string of name=value, separated by commas".into()),
                };
                Ok(Command::Restart { n: n()?, tactics })
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
                    Ok(Command::Workers { n: n()?, workers })
                }
                _ => Err("threads takes total, or n and workers".into()),
            },
            "finder" => {
                let seconds = json.get("seconds").and_then(Json::as_f64).filter(|s| s.is_finite() && *s > 0.0);
                let budget = seconds
                    .map(Duration::from_secs_f64)
                    .filter(|&budget| budget <= MAX_FINDER)
                    .ok_or(format!("finder needs seconds, more than 0 and at most {}", MAX_FINDER.as_secs()))?;
                Ok(Command::Finder { n: n()?, budget })
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

/// What the lab did to a game, kept beside its checkpoint (`n<NNNN>.lab`) so that it outlives the process, and
/// written into the game's line: its restarts, the search tactics of the last one, and its settings of
/// `conflict_branching` and `conflict_look`.  It also keeps whether the hook was told that the game runs long, so that
/// it is told once.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct GameLab {
    /// The search tactics the lab restarted the game with, if it did: the game's search runs with them, and so does a
    /// resume of it.
    pub(super) tactics: Option<SearchTactics>,
    /// The lab's settings of `conflict_branching` and `conflict_look`, if it gave one: each holds over the tactics.
    pub(super) conflict_branching: Option<bool>,
    pub(super) conflict_look: Option<bool>,
    /// How many `branching` commands the lab gave.
    pub(super) branching_changes: u32,
    pub(super) restarts: Vec<Restarted>,
    /// Whether the hook was told that the game runs long.
    pub(super) long: bool,
}

impl GameLab {
    /// The file of game `n`, if there is a checkpoint directory.
    fn path(opts: &SolveOptions, n: u32) -> Option<PathBuf> {
        opts.checkpoint_dir.as_ref().map(|dir| dir.join(format!("n{n:04}.lab")))
    }

    /// What the lab did to game `n`, from its file: nothing if there is none.
    pub(super) fn load(opts: &SolveOptions, n: u32) -> Result<GameLab, String> {
        let Some(path) = Self::path(opts, n).filter(|path| path.exists()) else { return Ok(GameLab::default()) };
        let at = |e: String| {
            format!("{}: {e} (the lab's record of the game; delete it to start the game over)", path.display())
        };
        let json = Json::from_file(&path).map_err(at)?;
        Self::from_json(&json).map_err(at)
    }

    /// Keeps it in game `n`'s file, whole; a failure is said on stderr.
    pub(super) fn keep(&self, opts: &SolveOptions, n: u32) {
        if let Some(path) = Self::path(opts, n) {
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            write_whole(&path, &format!("{}\n", self.to_json()));
        }
    }

    /// Deletes game `n`'s file, once its line is safe.
    pub(super) fn forget(opts: &SolveOptions, n: u32) {
        if let Some(path) = Self::path(opts, n) {
            let _ = fs::remove_file(path);
        }
    }

    /// Whether the lab changed how the game was searched: a restart, or a setting of `conflict_branching` or
    /// `conflict_look`.
    pub(super) fn steered(&self) -> bool {
        !self.restarts.is_empty() || self.conflict_branching.is_some() || self.conflict_look.is_some()
    }

    /// The search tactics of the game: the lab's, else `run`'s, with the lab's settings of `conflict_branching` and
    /// `conflict_look`.
    pub(super) fn search_tactics(&self, run: &SearchTactics) -> SearchTactics {
        let tactics = self.tactics.unwrap_or(*run);
        SearchTactics {
            conflict_branching: self.conflict_branching.unwrap_or(tactics.conflict_branching),
            conflict_look: self.conflict_look.unwrap_or(tactics.conflict_look),
            ..tactics
        }
    }

    /// What a results line records of it (`lab`), if the lab changed how the game was searched.
    pub(super) fn record(&self) -> Option<Json> {
        let restarts = self.restarts.iter().map(|r| {
            Json::object([
                ("at", Json::from(r.at.as_str())),
                ("own_seconds", Json::Num(r.own_seconds)),
                ("nodes", r.nodes.map_or(Json::Null, Json::from)),
                ("tactics", Json::from(r.tactics.as_str())),
                ("aside", r.aside.as_deref().map_or(Json::Null, Json::from)),
            ])
        });
        let setting = |on: Option<bool>| on.map_or(Json::Null, |on| Json::from(if on { "on" } else { "off" }));
        self.steered().then(|| {
            Json::object([
                ("restarts", Json::Arr(restarts.collect())),
                ("conflict_branching", setting(self.conflict_branching)),
                ("conflict_look", setting(self.conflict_look)),
                ("branching_changes", Json::from(self.branching_changes)),
            ])
        })
    }

    fn to_json(&self) -> Json {
        // Every search tactic, by name, so that the record means the same to a build whose defaults differ.
        let tactics = self.tactics.map_or(Json::Null, |search| {
            let all = Tactics { search, ..Tactics::default() };
            Json::array(all.settings().map(|(name, value)| Json::from(format!("{name}={value}"))))
        });
        let restarts = self.restarts.iter().map(|r| {
            Json::object([
                ("at", Json::from(r.at.as_str())),
                ("own_seconds", Json::Num(r.own_seconds)),
                ("nodes", r.nodes.map_or(Json::Null, Json::from)),
                ("tactics", Json::from(r.tactics.as_str())),
                ("aside", r.aside.as_deref().map_or(Json::Null, Json::from)),
            ])
        });
        Json::object([
            ("tactics", tactics),
            ("conflict_branching", self.conflict_branching.map_or(Json::Null, Json::Bool)),
            ("conflict_look", self.conflict_look.map_or(Json::Null, Json::Bool)),
            ("branching_changes", Json::from(self.branching_changes)),
            ("restarts", Json::Arr(restarts.collect())),
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
            branching_changes: json.get("branching_changes").and_then(Json::as_u32).unwrap_or(0),
            restarts,
            long: json.get("long").and_then(Json::as_bool).unwrap_or(false),
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
            Ok(Command::Branching { n: 9, conflict_branching: Some(false), conflict_look: None })
        );
        assert_eq!(
            command(r#"{"command":"branching","n":9,"conflict_look":"on"}"#),
            Ok(Command::Branching { n: 9, conflict_branching: None, conflict_look: Some(true) })
        );
        assert_eq!(
            command(r#"{"command":"branching","n":9,"conflict_branching":"on","conflict_look":"off"}"#),
            Ok(Command::Branching { n: 9, conflict_branching: Some(true), conflict_look: Some(false) })
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
            (r#"{"command":"branching","n":5}"#, "needs conflict_branching or conflict_look"),
            (r#"{"command":"branching","n":5,"look":"on"}"#, "takes no field \"look\""),
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
            branching_changes: 3,
            restarts: vec![Restarted {
                at: utc(SystemTime::now()),
                own_seconds: 12.5,
                nodes: Some(1234),
                tactics: "symmetry=off".into(),
                aside: Some("n0077.ckpt.restart1".into()),
            }],
            long: true,
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
        // A setting of the look alone steers the game, and leaves the branching to the tactics.
        let looking = GameLab { conflict_look: Some(true), ..GameLab::default() };
        assert!(looking.steered());
        assert!(looking.search_tactics(&SearchTactics::default()).conflict_look);
        assert_eq!(looking.record().unwrap().get("conflict_branching"), Some(&Json::Null));
        // A record from before the look has no setting of it.
        let old = Json::parse(r#"{"conflict_branching":false,"branching_changes":1}"#).unwrap();
        assert_eq!(GameLab::from_json(&old).map(|lab| lab.conflict_look), Ok(None));
        assert_eq!(record.get("restarts").and_then(Json::as_arr).map(<[_]>::len), Some(1));
        GameLab::forget(&opts, 77);
        assert_eq!(GameLab::load(&opts, 77), Ok(GameLab::default()));
        // A record of the hook alone changes nothing of the line.
        assert_eq!(GameLab { long: true, ..GameLab::default() }.record(), None);
        // A record that is not one is an error, which names the file.
        fs::write(dir.join("ckpt/n0078.lab"), "{\"tactics\":[\"gf2=maybe\"]}").unwrap();
        assert!(GameLab::load(&opts, 78).unwrap_err().contains("n0078.lab"));
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
