//! The finder alone: how soon it reaches a game's final best from the start a search begins at.
//!
//! ```text
//! cargo run --release --features studies --example finder -- GAME --target SUM [--known FILE]... [--seconds S,S,...]
//!     [--seeds K,K,...] [--no-seed] [--start-only]
//! ```
//!
//! A search of game GAME starts from an incumbent (the greedy set and what survives of the seed, the game of `n − 1`,
//! then the dive and the local search: `search::start_incumbent`) and, with `finder_after`, runs the finder beside it
//! ([`taxman::search::finder`]).  This program makes that start once, on one thread, and then runs the finder alone,
//! on one thread, for each budget of `--seconds` (default 5,60) and each random seed of `--seeds` (default 0,100,200:
//! the rounds of a run draw with the seed plus 1, 2, ...), from a control that holds the start as the incumbent, as the
//! search would.  `--known FILE` (repeatable) names the results files whose completed games, with `optimal.json`'s,
//! are the known games: the game of `n − 1` is the seed (`--no-seed`: none, the finder starts from the incumbent
//! alone).  `--target SUM` is the final best promotion sum of the game (the score less `in_the_bag`), which a run is
//! said to reach or not.
//!
//! One JSON line a run: `game`, `target`, `start` (the start incumbent's promotion sum), `seconds` and `random` (the
//! budget and the seed), `seeded`, `best` (the best promotion sum the finder reached), `reached` (whether it is at
//! least `target`), `found_s` (the seconds at which it reached `best`), `improved` (its sets that beat the control's
//! incumbent) and `ran_s`.  The line of the start comes first, with its time (`start_s`).  `--start-only` makes the
//! start and prints that line.  Every set the finder offers passes the check of `search::offer_found`.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use taxman::board::Board;
use taxman::engine::Control;
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::matchbound::MatchGraph;
use taxman::numtheory::Tables;
use taxman::search::finder::{self, Plan};
use taxman::search::{payload_of, start_incumbent, SearchTactics};

struct Options {
    game: u32,
    target: Option<u64>,
    known: Vec<PathBuf>,
    seconds: Vec<f64>,
    seeds: Vec<u64>,
    seeded: bool,
    start_only: bool,
}

fn parse() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let mut options = Options {
        game: 0,
        target: None,
        known: Vec::new(),
        seconds: vec![5.0, 60.0],
        seeds: vec![0, 100, 200],
        seeded: true,
        start_only: false,
    };
    let mut game = None;
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        fn list<T: std::str::FromStr>(name: &str, text: &str) -> Result<Vec<T>, String> {
            text.split(',').map(|item| item.parse().map_err(|_| format!("{name}: cannot read {item:?}"))).collect()
        }
        match arg.as_str() {
            "--target" => {
                let text = value("--target")?;
                options.target = Some(text.parse().map_err(|_| format!("--target: cannot read {text:?}"))?);
            }
            "--known" => options.known.push(PathBuf::from(value("--known")?)),
            "--seconds" => options.seconds = list("--seconds", &value("--seconds")?)?,
            "--seeds" => options.seeds = list("--seeds", &value("--seeds")?)?,
            "--no-seed" => options.seeded = false,
            "--start-only" => options.start_only = true,
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            _ => game = Some(arg.parse::<u32>().map_err(|_| format!("GAME: cannot read {arg:?}"))?),
        }
    }
    options.game =
        game.ok_or("usage: finder GAME --target SUM [--known FILE]... (see the top of examples/finder.rs)")?;
    Ok(options)
}

fn run<const W: usize>(options: &Options) -> Result<(), String> {
    let n = options.game;
    let optimal = Optimal::built_in().with_results_files(&options.known)?;
    let board = Board::<W>::new(Tables::new(n));
    let p = board.prepare(SearchTactics::default())?;
    let seed = match (options.seeded, optimal.moves(n - 1)) {
        (true, Some(moves)) => Some(board.seed(moves)),
        (true, None) => return Err(format!("no known game of {} to seed with (--known, or --no-seed)", n - 1)),
        (false, _) => None,
    };
    let began = Instant::now();
    let graph = MatchGraph::new(&p.model, &p.root);
    let start = start_incumbent(&p, &graph, &Control::new(), 1, seed.as_ref());
    let line = |fields: Vec<(&str, Json)>| println!("{}", Json::object(fields));
    line(vec![
        ("game", Json::from(n)),
        ("target", options.target.map_or(Json::Null, Json::from)),
        ("start", Json::from(start.sum())),
        ("start_s", Json::Num(began.elapsed().as_secs_f64())),
        ("seeded", Json::Bool(seed.is_some())),
    ]);
    if options.start_only {
        return Ok(());
    }
    for &seconds in &options.seconds {
        for &random in &options.seeds {
            let control = Control::new();
            control.offer(start.sum(), &payload_of(&start));
            let (_alive, ended) = mpsc::channel::<()>();
            let plan = Plan { after: 1, budget: Duration::from_secs_f64(seconds), seed, random };
            let report = finder::rounds(&p, &control, &plan, 0, &ended, None);
            line(vec![
                ("game", Json::from(n)),
                ("target", options.target.map_or(Json::Null, Json::from)),
                ("start", Json::from(start.sum())),
                ("seconds", Json::Num(seconds)),
                ("random", Json::from(random)),
                ("seeded", Json::Bool(seed.is_some())),
                ("best", Json::from(report.best)),
                ("reached", options.target.map_or(Json::Null, |target| Json::Bool(report.best >= target))),
                ("found_s", Json::Num(report.found)),
                ("improved", Json::from(report.improved)),
                ("ran_s", Json::Num(report.seconds)),
            ]);
        }
    }
    Ok(())
}

fn main() -> std::process::ExitCode {
    let result = parse().and_then(|options| taxman::with_width!(options.game, W => run::<W>(&options)));
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("finder: {message}");
            std::process::ExitCode::from(2)
        }
    }
}
