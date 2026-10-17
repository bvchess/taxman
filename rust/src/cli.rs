//! Command-line parsing shared by the subcommands: the error type, a small flag parser, the solver options
//! the flags describe, and output that survives a closed stdout or stderr.
//!
//! There is no parser crate.  A command line is positional arguments and flags; a flag is `--name VALUE` or
//! `--name=VALUE` if it takes a value and `--name` if it is a switch, each command says which of its flags are which
//! ([`Args::parse`]), and any other `--flag` is a usage error.  The flags every solving command takes
//! ([`SOLVER_FLAGS`]: `--threads`, `--tactic`) are turned into [`SolveOptions`] by [`solver_options`], and with the
//! flag of the commands that solve a range ([`WINDOW_FLAG`]) by [`range_options`]; the others (`--out`,
//! `--time-limit`, `--use-prev`, ...) are each command's own.  A usage error makes `main` print the usage and
//! exit with status 2; a command that ran and failed exits with status 1 ([`CliError`]).

use std::collections::BTreeMap;
use std::fmt;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use taxman::solver::{SolveOptions, MAX_N};

/// Write one line, ignoring write errors: a closed pipe (EPIPE, say a `head` or a `tee` that has exited) must
/// not kill a run whose results and checkpoints are safe on disk.  (`println!` panics on it, and release
/// builds abort on a panic.)  Use it through [`outln!`] and [`errln!`].
pub fn write_line(mut to: impl Write, line: fmt::Arguments<'_>) {
    let _ = to.write_fmt(format_args!("{line}\n"));
}

/// `println!`, through [`write_line`].
macro_rules! outln {
    ($($arg:tt)*) => {
        $crate::cli::write_line(std::io::stdout().lock(), format_args!($($arg)*))
    };
}

/// `eprintln!`, through [`write_line`].
macro_rules! errln {
    ($($arg:tt)*) => {
        $crate::cli::write_line(std::io::stderr().lock(), format_args!($($arg)*))
    };
}

pub(crate) use {errln, outln};

/// The most seconds a flag takes (about 31 years): more is surely a mistake, and much more would not fit a
/// `Duration` or the clock.
const MAX_SECONDS: f64 = 1e9;

#[derive(Debug)]
pub enum CliError {
    /// Bad command line: print the message and the usage, exit 2.
    Usage(String),
    /// The command ran and failed: print the message, exit 1.
    Failed(String),
    /// `taxman certify` found a game that beats a recorded score: print the message, exit 2.
    Counterexample(String),
    /// `taxman solve` stopped at the wall (`--game-hours`): the command has said so on stdout; exit [`WALL_STATUS`].
    Wall,
}

/// The exit status of a `solve` that a game passing the wall (`--game-hours`) ended: a status of its own, so that a
/// script (`rust/tools/cloud/chain.sh`) tells it from an error (1, 2), a signal (130, 143) and the deadline of its
/// own (124).
pub const WALL_STATUS: u8 = 4;

pub type CliResult = Result<(), CliError>;

/// Flags every solving command takes (each followed by a value).
pub const SOLVER_FLAGS: [&str; 2] = ["--threads", "--tactic"];

/// The command line after the subcommand name: flags with their values, and positional arguments.
#[derive(Debug)]
pub struct Args {
    named: Vec<(String, Option<String>)>,
    positional: Vec<String>,
}

impl Args {
    /// Splits `args` into flags and positionals.  `with_value` lists the flags that take a value (`--flag VALUE`
    /// or `--flag=VALUE`) and `switches` the ones that do not; any other `--flag` is an error.
    pub fn parse(args: &[String], with_value: &[&str], switches: &[&str]) -> Result<Args, CliError> {
        let mut parsed = Args { named: Vec::new(), positional: Vec::new() };
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            if !arg.starts_with("--") {
                parsed.positional.push(arg.clone());
                continue;
            }
            let (name, inline) = match arg.split_once('=') {
                Some((name, value)) => (name, Some(value.to_owned())),
                None => (arg.as_str(), None),
            };
            if switches.contains(&name) {
                if inline.is_some() {
                    return Err(CliError::Usage(format!("{name} takes no value")));
                }
                parsed.named.push((name.to_owned(), None));
            } else if with_value.contains(&name) {
                let value = match inline {
                    Some(value) => value,
                    None => rest.next().ok_or_else(|| CliError::Usage(format!("{name} needs a value")))?.clone(),
                };
                parsed.named.push((name.to_owned(), Some(value)));
            } else {
                return Err(CliError::Usage(format!("unknown option {name}")));
            }
        }
        Ok(parsed)
    }

    pub fn positional(&self) -> &[String] {
        &self.positional
    }

    /// Whether the switch was given.
    pub fn has(&self, name: &str) -> bool {
        self.named.iter().any(|(given, _)| given == name)
    }

    /// Every value given for the flag, in order.
    pub fn values(&self, name: &str) -> Vec<&str> {
        self.named.iter().filter(|(given, _)| given == name).filter_map(|(_, value)| value.as_deref()).collect()
    }

    /// The last value given for the flag.
    pub fn value(&self, name: &str) -> Option<&str> {
        self.values(name).last().copied()
    }

    /// The flag's value parsed as a `T`.
    pub fn parsed<T: FromStr>(&self, name: &str) -> Result<Option<T>, CliError> {
        self.value(name)
            .map(|text| text.parse().map_err(|_| CliError::Usage(format!("{name}: cannot read {text:?}"))))
            .transpose()
    }

    /// The flag's value as a number of seconds (fractions allowed), from 0 to [`MAX_SECONDS`].
    pub fn seconds(&self, name: &str) -> Result<Option<Duration>, CliError> {
        let Some(secs) = self.parsed::<f64>(name)? else { return Ok(None) };
        if !(0.0..=MAX_SECONDS).contains(&secs) {
            return Err(CliError::Usage(format!("{name}: {secs} is not a number of seconds from 0 to {MAX_SECONDS}")));
        }
        Ok(Some(Duration::from_secs_f64(secs)))
    }

    /// [`seconds`](Self::seconds) that must be more than 0 (as it comes out in nanoseconds).
    pub fn positive_seconds(&self, name: &str) -> Result<Option<Duration>, CliError> {
        match self.seconds(name)? {
            Some(time) if time.is_zero() => Err(CliError::Usage(format!("{name} must be more than 0 seconds"))),
            time => Ok(time),
        }
    }

    /// The flag's value as a number of hours (fractions allowed), more than 0 (as it comes out in nanoseconds) and at
    /// most [`MAX_SECONDS`] seconds.
    pub fn positive_hours(&self, name: &str) -> Result<Option<Duration>, CliError> {
        let Some(hours) = self.parsed::<f64>(name)? else { return Ok(None) };
        let seconds = hours * 3600.0;
        if !(seconds > 0.0 && seconds <= MAX_SECONDS) {
            return Err(CliError::Usage(format!(
                "{name}: {hours} is not a number of hours from 0 to {}",
                MAX_SECONDS / 3600.0
            )));
        }
        let time = Duration::from_secs_f64(seconds);
        if time.is_zero() {
            return Err(CliError::Usage(format!("{name} must be more than 0 hours")));
        }
        Ok(Some(time))
    }
}

/// The flag of the commands that solve a range (`solve`, `regress`) besides [`SOLVER_FLAGS`]: the games in flight
/// ([`SolveOptions::window`]), which [`range_options`] reads.
pub const WINDOW_FLAG: &str = "--window";

/// The solver options the flags of [`SOLVER_FLAGS`] describe, on top of the defaults.
pub fn solver_options(args: &Args) -> Result<SolveOptions, CliError> {
    let mut opts = SolveOptions::default();
    if let Some(threads) = count(args, "--threads")? {
        opts.threads = threads;
    }
    for assignment in args.values("--tactic") {
        opts.tactics.set(assignment).map_err(CliError::Usage)?;
    }
    Ok(opts)
}

/// [`solver_options`] with [`WINDOW_FLAG`], for a command that solves a range.
pub fn range_options(args: &Args) -> Result<SolveOptions, CliError> {
    let mut opts = solver_options(args)?;
    if let Some(window) = count(args, WINDOW_FLAG)? {
        opts.window = window;
    }
    Ok(opts)
}

/// The flag's value as a count, which must be at least 1.
pub fn count(args: &Args, name: &str) -> Result<Option<usize>, CliError> {
    match args.parsed::<usize>(name)? {
        Some(0) => Err(CliError::Usage(format!("{name} must be at least 1"))),
        count => Ok(count),
    }
}

/// `N` or `A-B`.
pub fn parse_range(text: &str) -> Result<(u32, u32), CliError> {
    let bad = || CliError::Usage(format!("expected a board size N or a range A-B, got {text:?}"));
    let (lo, hi) = match text.split_once('-') {
        Some((lo, hi)) => (lo.parse().map_err(|_| bad())?, hi.parse().map_err(|_| bad())?),
        None => {
            let n = text.parse().map_err(|_| bad())?;
            (n, n)
        }
    };
    if lo < 1 || lo > hi {
        return Err(CliError::Usage(format!("the range {text:?} is empty or starts below 1")));
    }
    Ok((lo, hi))
}

/// The boards of a list file, `text`, in its order: one board size a line, from 1 to [`MAX_N`], none twice.  Blank
/// lines and lines whose first non-blank character is `#` are skipped.  `name` is the file, for the messages, which
/// also name the line.
pub fn parse_board_list(name: &str, text: &str) -> Result<Vec<u32>, CliError> {
    let mut first_line: BTreeMap<u32, usize> = BTreeMap::new();
    let mut boards = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = |why: String| CliError::Usage(format!("{name}: line {}: {why}", i + 1));
        let digits = line.bytes().all(|b| b.is_ascii_digit());
        let n: u32 =
            line.parse().ok().filter(|_| digits).ok_or_else(|| at(format!("expected a board size, got {line:?}")))?;
        if !(1..=MAX_N).contains(&n) {
            return Err(at(format!("{n} is not a board size from 1 to {MAX_N}")));
        }
        if let Some(first) = first_line.insert(n, i + 1) {
            return Err(at(format!("the board {n} is listed again (first on line {first})")));
        }
        boards.push(n);
    }
    if boards.is_empty() {
        return Err(CliError::Usage(format!("{name}: no board is listed")));
    }
    Ok(boards)
}

/// [`parse_board_list`] of the file at `path`.
pub fn read_board_list(path: &Path) -> Result<Vec<u32>, CliError> {
    let name = path.display().to_string();
    let text = std::fs::read_to_string(path).map_err(|e| CliError::Usage(format!("{name}: {e}")))?;
    parse_board_list(&name, &text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    fn usage(e: CliError) -> String {
        match e {
            CliError::Usage(message) => message,
            CliError::Failed(message) | CliError::Counterexample(message) => {
                panic!("expected a usage error, got {message:?}")
            }
            CliError::Wall => panic!("expected a usage error, got the wall"),
        }
    }

    #[test]
    fn flags_and_positionals() {
        let given =
            strings(&["480", "--threads", "4", "--tactic=match_refine=0", "--tactic", "prev_cap=off", "--show"]);
        let args = Args::parse(&given, &SOLVER_FLAGS, &["--show"]).unwrap();
        assert_eq!(args.positional(), ["480"]);
        assert!(args.has("--show") && !args.has("--force-search"));
        assert_eq!(args.values("--tactic"), ["match_refine=0", "prev_cap=off"]);
        let opts = solver_options(&args).unwrap();
        assert_eq!(opts.threads, 4);
        assert_eq!(opts.tactics.description(), "match_refine=0,prev_cap=off");
    }

    #[test]
    fn tactics_with_values_other_than_on_and_off() {
        let options = |tactics: &[&str]| {
            let given: Vec<&str> = tactics.iter().flat_map(|t| ["--tactic", t]).collect();
            let args = Args::parse(&strings(&given), &SOLVER_FLAGS, &[]).unwrap();
            solver_options(&args).map(|opts| opts.tactics).map_err(usage)
        };
        let t = options(&["match_refine=2", "nogoods=off"]).unwrap();
        assert_eq!(t.description(), "match_refine=2,nogoods=off");
        // The last assignment of a tactic wins, as for the switches.
        assert_eq!(options(&["match_refine=3", "match_refine=1"]).unwrap().description(), "default");
        for bad in ["match_refine=9", "match_refine=two", "match_refine=-1"] {
            let message = options(&[bad]).unwrap_err();
            assert!(message.starts_with(&format!("tactic {}: expected ", bad.split('=').next().unwrap())), "{message}");
        }
    }

    #[test]
    fn bad_flags_are_usage_errors() {
        let parse =
            |items: &[&str]| Args::parse(&strings(items), &SOLVER_FLAGS, &["--show"]).map(|_| ()).map_err(usage);
        assert!(parse(&["--nope"]).unwrap_err().contains("unknown option --nope"));
        assert!(parse(&["--threads"]).unwrap_err().contains("needs a value"));
        assert!(parse(&["--show=1"]).unwrap_err().contains("takes no value"));
        let args = Args::parse(&strings(&["--threads", "x"]), &SOLVER_FLAGS, &[]).unwrap();
        assert!(usage(solver_options(&args).unwrap_err()).contains("--threads"));
        let args = Args::parse(&strings(&["--threads", "0"]), &SOLVER_FLAGS, &[]).unwrap();
        assert!(solver_options(&args).is_err());
        let args = Args::parse(&strings(&["--tactic", "nonsense=on"]), &SOLVER_FLAGS, &[]).unwrap();
        assert!(usage(solver_options(&args).unwrap_err()).contains("unknown tactic"));
    }

    #[test]
    fn the_window_is_a_count_that_the_commands_of_a_range_read() {
        let flags = [&SOLVER_FLAGS[..], &[WINDOW_FLAG]].concat();
        let options = |items: &[&str]| range_options(&Args::parse(&strings(items), &flags, &[]).unwrap());
        let opts = options(&["--window", "8", "--threads", "4"]).unwrap();
        assert_eq!((opts.window, opts.threads), (8, 4));
        assert_eq!(options(&[]).unwrap().window, 1);
        assert!(usage(options(&["--window", "0"]).unwrap_err()).contains("--window must be at least 1"));
        assert!(usage(options(&["--window", "x"]).unwrap_err()).contains("--window: cannot read"));
        // A command that does not solve a range does not take it.
        assert!(Args::parse(&strings(&["--window", "2"]), &SOLVER_FLAGS, &[]).is_err());
    }

    #[test]
    fn seconds() {
        let args = Args::parse(&strings(&["--progress", "0.5"]), &["--progress"], &[]).unwrap();
        assert_eq!(args.seconds("--progress").unwrap(), Some(Duration::from_millis(500)));
        assert_eq!(args.seconds("--time-limit").unwrap(), None);
        assert!(Args::parse(&strings(&["--progress", "-1"]), &["--progress"], &[])
            .unwrap()
            .seconds("--progress")
            .is_err());
        let solver = Args::parse(&strings(&["--optimal", "t.json"]), &SOLVER_FLAGS, &[]).map(|_| ()).map_err(usage);
        assert!(solver.unwrap_err().contains("unknown option --optimal"));
    }

    #[test]
    fn seconds_are_bounded_and_intervals_positive() {
        let given = |value: &str| Args::parse(&strings(&["--every", value]), &["--every"], &[]).unwrap();
        for bad in ["1e300", "1e10", "-0.5", "inf", "-inf", "nan", "NaN", "soon"] {
            assert!(usage(given(bad).seconds("--every").unwrap_err()).contains("--every"), "{bad}");
        }
        assert_eq!(given("1e9").seconds("--every").unwrap(), Some(Duration::from_secs(1_000_000_000)));
        assert_eq!(given("0").seconds("--every").unwrap(), Some(Duration::ZERO));
        for zero in ["0", "0.0", "1e-12"] {
            assert!(usage(given(zero).positive_seconds("--every").unwrap_err()).contains("more than 0"), "{zero}");
        }
        assert_eq!(given("0.001").positive_seconds("--every").unwrap(), Some(Duration::from_millis(1)));
        assert_eq!(given("1").positive_seconds("--other").unwrap(), None);
    }

    #[test]
    fn hours_are_bounded_and_positive() {
        let given = |value: &str| Args::parse(&strings(&["--wall", value]), &["--wall"], &[]).unwrap();
        for bad in ["0", "-1", "1e9", "inf", "NaN", "soon", "1e-20"] {
            assert!(usage(given(bad).positive_hours("--wall").unwrap_err()).contains("--wall"), "{bad}");
        }
        assert_eq!(given("24").positive_hours("--wall").unwrap(), Some(Duration::from_secs(86_400)));
        assert_eq!(given("0.5").positive_hours("--wall").unwrap(), Some(Duration::from_secs(1_800)));
        assert_eq!(given("0.001").positive_hours("--other").unwrap(), None);
    }

    #[test]
    fn ranges() {
        assert_eq!(parse_range("480").ok(), Some((480, 480)));
        assert_eq!(parse_range("100-200").ok(), Some((100, 200)));
        for bad in ["", "0", "5-4", "a", "1-b", "1-2-3"] {
            assert!(parse_range(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_list_of_boards_keeps_its_order_and_skips_comments_and_blank_lines() {
        let list = |text: &str| parse_board_list("boards.txt", text).map_err(usage);
        assert_eq!(list("1304\n1302\n1309\n"), Ok(vec![1304, 1302, 1309]));
        assert_eq!(list("# these three\n\n  1304  \n\t# 99\n1302\n   \n1\n8191"), Ok(vec![1304, 1302, 1, 8191]));
        assert_eq!(list("7\r\n5\r\n"), Ok(vec![7, 5]));
    }

    #[test]
    fn a_list_of_boards_refuses_what_is_none_naming_the_file_and_the_line() {
        let list = |text: &str| parse_board_list("boards.txt", text).map(|_| ()).map_err(usage);
        for (text, why) in [
            ("12\nx\n", "boards.txt: line 2: expected a board size, got \"x\""),
            ("12\n13 14\n", "boards.txt: line 2: expected a board size, got \"13 14\""),
            ("12\n1302 # a note\n", "boards.txt: line 2: expected a board size, got \"1302 # a note\""),
            ("12\n-5\n", "boards.txt: line 2: expected a board size, got \"-5\""),
            ("+5\n", "boards.txt: line 1: expected a board size, got \"+5\""),
            ("12\n4294967296\n", "boards.txt: line 2: expected a board size, got \"4294967296\""),
            ("0\n", "boards.txt: line 1: 0 is not a board size from 1 to 8191"),
            ("# c\n12\n8192\n", "boards.txt: line 3: 8192 is not a board size from 1 to 8191"),
            ("12\n\n30\n12\n", "boards.txt: line 4: the board 12 is listed again (first on line 1)"),
            ("", "boards.txt: no board is listed"),
            ("# only a comment\n\n", "boards.txt: no board is listed"),
        ] {
            assert_eq!(list(text), Err(why.to_owned()), "{text:?}");
        }
    }

    #[test]
    fn a_list_file_that_is_not_there_is_named() {
        let missing = std::env::temp_dir().join("taxman-no-such-list.txt");
        let message = usage(read_board_list(&missing).unwrap_err());
        assert!(message.contains("taxman-no-such-list.txt"), "{message}");
    }
}
