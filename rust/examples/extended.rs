//! A chain of games searched on the extended problem Π⁺(∅): the frame model without `unzip`'s claims
//! ([`taxman::frames::build_extended`], [`Board::prepare_extended`]).  Research code for the extended problem
//! (heuristics/THEORY.md, "The extended problem"); the solver does not use it.  By Theorems M and M⁺ the promotion sets
//! of Π⁺(∅) are those of the frame model, so a chain searched on it must score every game as `optimal.json` and
//! `results/strict.jsonl` do.  Its root frames keep the pairs `unzip` would claim (only the root's reduction removes
//! some), so its trees can differ from the default problem's: how many nodes it takes against them is the
//! measurement.
//!
//! ```text
//! cargo run --release --example extended -- chain FROM TO [--compare] [--known FILE] [--out FILE]
//! cargo run --release --example extended -- words FROM TO --upper FILE...
//! cargo run --release --example extended -- problems FROM TO --upper FILE... [--known FILE] [--out FILE]
//! ```
//!
//! For each board `n` in `FROM..=TO` (2 at least, `TO` at most the largest board the solver supports), one after the
//! other on one thread, each game built from the game before it: the game of `FROM − 1` is the one of `optimal.json`
//! (or of the results file given with `--known`, for a board past its end), and after that it is the one this run
//! found.  A game is set up as `taxman solve` sets it up, but on Π⁺(∅) (`Board::prepare_extended`).  If the problem has
//! no candidates the game is the one with no promotions (method `trivial`).  Otherwise the problem is searched to the
//! end (method `search`), primes included, seeded with the previous game's moves that are candidates and capped at
//! n + the previous score − `in_the_bag` (`Board::promotion_cap`).  `--compare` searches the default problem
//! (`Board::prepare`) the same way as well.
//!
//! **The checks.**  A game that fails one is printed and stops the run with status 1.
//!
//! * Π⁺(∅) fixes exactly the sources the game with no promotions plays: its `in_the_bag` is the board's.
//! * The game the promotions make (`accelerate::solution_for_promotions`) is legal and scores `in_the_bag` plus their
//!   sum (`game::replay`).
//! * Its score is the known score of `n`, where one is known.
//! * With `--compare`, the default problem's best promotions sum to the same as Π⁺(∅)'s.
//!
//! **The output.**  One line a game on stdout: `n`, the method, the score, the nodes and the seconds of the search of
//! Π⁺(∅), and with `--compare` the same of the default problem and the ratio of the nodes (extended over default).
//! With `--out FILE` a JSON object per game is appended to FILE as well: `n`, `method`, `score`, `in_the_bag`, `sum`
//! (of the promotions), `reached_cap` (the search ended at the ceiling), `nodes` and `secs` of the search of Π⁺(∅), and
//! with `--compare` `default_nodes` and `default_secs`.  At the end a summary: the games by method, how many scores
//! are known (all equal), the nodes and seconds in all, with `--compare` the games where Π⁺(∅) took more than 10% more
//! nodes than the default problem and those where it took more than 10% fewer (their count and the ten with the
//! largest, and the smallest, ratios), and the benchmark games (README.md, DESIGN.md) that were in range.  Exit status:
//! 0 if every game passed, 1 if one failed, 2 for a usage error.
//!
//! **`words`**: for each game of `FROM..=TO` that the prover's barred bounds (`--upper`, `prover/data/upper_*.jsonl`)
//! say needs a certificate, how many block words the GF(2) bound of Π⁺(O), O its open members, needs at its root
//! (`Board::extended_words`): an open source adds one to its frame's rank, so `taxman certify` searches a game whose
//! words exceed the engines of its set width (three up to 64 words, rank 192) at the widest width (128 words, five
//! engine words, rank 320), and cannot search one that needs more (`certify::certificate_width`).  One line a game (n,
//! |O|, the words, the width `taxman certify` would use or `none`), then the games by words and the widths needed.
//!
//! **`problems`**: for each game of `FROM..=TO` that the barred bounds (`--upper`) say needs a certificate, one after
//! the other on one thread, the extended problem Π⁺(O) set up without a search (`certify::extended_problem`) and the
//! width it takes (`certify::certificate_width`), with V the recorded score (the table of optimal games, over the
//! results file `--known` gives, for a board past its end; it must be the barred bounds' score).  One JSON line a game,
//! on stdout and appended to `--out FILE` if given:
//!
//! ```text
//! {"n":..,"score":V,"open":|O|,"rejected":|R|,"frames":..,"two_sided":..,"one_sided":..,"in_the_bag":..,
//!  "worth":V-in_the_bag,"words":..,"width":W|null,"digest":"<16 hex>","same_as":m|null}
//! ```
//!
//! `same_as` is the latest game m of the 8 board sizes before n (`SAME_SEARCH_WINDOW`) among the games of the run whose
//! problem equals n's whole (the digest, then the problems), whatever the two games are worth: where the worths are
//! equal as well, `taxman certify` settles n from m with no search.  At the end, on stderr: the games, those with a
//! `same_as`, those of them with equal worth, the games by width, and the seconds.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};
use taxman::accelerate::solution_for_promotions;
use taxman::bits::{width_for, words_for, Bits};
use taxman::board::Board;
use taxman::certify::{certificate_width, extended_problem, read_upper};
use taxman::engine::EngineConfig;
use taxman::game::{replay, Optimal};
use taxman::gf2::MAX_WORDS;
use taxman::json::Json;
use taxman::search::{search_with, Prepared, Problem, SearchTactics};
use taxman::solver::{digest_text, MAX_N, SAME_SEARCH_WINDOW};

const USAGE: &str = "usage: extended chain FROM TO [--compare] [--known FILE] [--out FILE]
       extended words FROM TO --upper FILE...
       extended problems FROM TO --upper FILE... [--known FILE] [--out FILE]   (2 <= FROM <= TO)";

/// The benchmark games, whose node counts the summary lists when they are in range.
const BENCHMARK_GAMES: [u32; 7] = [936, 1215, 1050, 1100, 1133, 1170, 1184];

/// How many games of each kind the summary lists.
const LISTED: usize = 10;

/// The subcommands.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Command {
    Chain,
    Words,
    Problems,
}

/// What the command line asks for.
struct Args {
    command: Command,
    from: u32,
    to: u32,
    compare: bool,
    known: Option<PathBuf>,
    out: Option<PathBuf>,
    upper: Vec<PathBuf>,
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let [command, from, to, flags @ ..] = args else { return Err(USAGE.to_owned()) };
    let board = |text: &String| text.parse::<u32>().map_err(|_| USAGE.to_owned());
    let command = match command.as_str() {
        "chain" => Command::Chain,
        "words" => Command::Words,
        "problems" => Command::Problems,
        _ => return Err(USAGE.to_owned()),
    };
    let (from, to) = (board(from)?, board(to)?);
    let (mut compare, mut known, mut out, mut upper) = (false, None, None, Vec::new());
    let mut flags = flags.iter();
    while let Some(flag) = flags.next() {
        match flag.as_str() {
            "--compare" => compare = true,
            "--known" => known = Some(PathBuf::from(flags.next().ok_or(USAGE)?)),
            "--out" => out = Some(PathBuf::from(flags.next().ok_or(USAGE)?)),
            "--upper" => upper.push(PathBuf::from(flags.next().ok_or(USAGE)?)),
            _ => return Err(USAGE.to_owned()),
        }
    }
    if from < 2 || from > to || to > MAX_N {
        return Err(format!("{USAGE}\nthe boards must be from 2 to {MAX_N}"));
    }
    // `chain` has no barred bounds; the others need them, and `words` takes no other flag.
    let allowed = match command {
        Command::Chain => upper.is_empty(),
        Command::Words => !upper.is_empty() && !compare && known.is_none() && out.is_none(),
        Command::Problems => !upper.is_empty() && !compare,
    };
    if !allowed {
        return Err(USAGE.to_owned());
    }
    Ok(Args { command, from, to, compare, known, out, upper })
}

/// The file `--out` names, opened for appending.
fn open_out(path: Option<&PathBuf>) -> Result<Option<File>, String> {
    let open = |path: &PathBuf| {
        OpenOptions::new().create(true).append(true).open(path).map_err(|e| format!("{}: {e}", path.display()))
    };
    path.map(open).transpose()
}

/// Appends `line` to the `--out` file, if there is one.
fn append(out: &mut Option<File>, line: &Json) -> Result<(), String> {
    match out {
        Some(file) => writeln!(file, "{line}").and_then(|()| file.flush()).map_err(|e| format!("--out: {e}")),
        None => Ok(()),
    }
}

/// What a search cost.
#[derive(Clone, Copy, Default)]
struct Cost {
    nodes: u64,
    time: Duration,
}

fn secs(time: Duration) -> f64 {
    time.as_secs_f64()
}

/// The nodes of `a` over those of `b`: 1 if neither took any, infinite if only `b` took none.
fn ratio(a: u64, b: u64) -> f64 {
    match (a, b) {
        (0, 0) => 1.0,
        (_, 0) => f64::INFINITY,
        _ => a as f64 / b as f64,
    }
}

/// A game of the chain: its score and its moves, from which the next game is seeded and capped.
struct Prev {
    score: u64,
    moves: Vec<u32>,
}

/// What settling a problem found: the best promotions, whether they reached the ceiling, and the search's cost.
struct Settled<const W: usize> {
    promotions: Bits<W>,
    reached_cap: bool,
    cost: Cost,
}

impl<const W: usize> Settled<W> {
    fn sum(&self) -> u64 {
        self.promotions.sum()
    }
}

/// The best promotions of the problem `prepared` poses: none if it has no candidates, else the result of a search to
/// the end on one thread, seeded with the moves of `prev` that are candidates and capped at `cap`.
fn settle<const W: usize>(prepared: &Prepared<W>, prev: &Prev, cap: u64) -> Result<Settled<W>, String> {
    if prepared.candidates.is_empty() {
        return Ok(Settled { promotions: Bits::EMPTY, reached_cap: false, cost: Cost::default() });
    }
    let seed = prev.moves.iter().copied().collect::<Bits<W>>() & prepared.candidates;
    let cfg = EngineConfig { threads: 1, ..EngineConfig::default() };
    let found = search_with(prepared, &cfg, Some(&seed), Some(cap), false, None).map_err(|e| format!("search: {e}"))?;
    if !found.completed {
        return Err("the search did not complete".to_owned());
    }
    let cost = Cost { nodes: found.stats.nodes, time: found.time };
    Ok(Settled { promotions: found.promotions, reached_cap: found.reached_cap, cost })
}

/// One game of the chain, as the output records it.
struct Game {
    n: u32,
    method: &'static str,
    score: u64,
    in_the_bag: u64,
    /// The sum of the promotions.
    sum: u64,
    reached_cap: bool,
    /// The search of Π⁺(∅).
    extended: Cost,
    /// The search of the default problem, with `--compare`.
    default: Option<Cost>,
    /// Whether a score is known for the board (and, as every game passed, equal to this one).
    known: bool,
}

/// Plays game `n` of the chain after `prev`: settles Π⁺(∅) (and, with `compare`, the default problem), makes the game
/// of the promotions and referees it.  The error says which check failed.
fn play<const W: usize>(n: u32, prev: &Prev, compare: bool, optimal: &Optimal) -> Result<(Game, Prev), String> {
    let board = Board::<W>::for_n(n);
    let (extended, prepared) = board.prepare_extended(SearchTactics::default())?;
    if extended.in_the_bag != board.in_the_bag {
        return Err(format!(
            "n={n}: Π⁺(∅) fixes {} but the game with no promotions scores {}",
            extended.in_the_bag, board.in_the_bag
        ));
    }
    let cap = board.promotion_cap(prev.score).ok_or_else(|| {
        format!("n={n}: the game of {} scores {}, less than the game with no promotions of {n}", n - 1, prev.score)
    })?;
    let settled = settle(&prepared, prev, cap)?;
    let sum = settled.sum();
    let promotions = settled.promotions.to_vec();
    let moves = solution_for_promotions(&board.tables, settled.promotions);
    let score = replay(n, &moves)
        .map_err(|e| format!("n={n}: the game is illegal: {e}\n  promotions {promotions:?}\n  moves {moves:?}"))?;
    if score != board.in_the_bag + sum {
        let why = format!("in_the_bag {} + the promotions' {sum}", board.in_the_bag);
        return Err(format!(
            "n={n}: the game scores {score}, not {why}\n  promotions {promotions:?}\n  moves {moves:?}"
        ));
    }
    let known = optimal.score(n);
    if let Some(known) = known.filter(|&known| known != score) {
        return Err(format!(
            "n={n}: the game scores {score}, the known game {known}\n  promotions {promotions:?}\n  moves {moves:?}"
        ));
    }
    let default = if compare {
        let default = settle(&board.prepare(SearchTactics::default())?, prev, cap)?;
        if default.sum() != sum {
            let (theirs, ours) = (default.promotions.to_vec(), promotions);
            return Err(format!(
                "n={n}: the default problem's best promotions sum to {} {theirs:?}, Π⁺(∅)'s to {sum} {ours:?}",
                default.sum()
            ));
        }
        Some(default.cost)
    } else {
        None
    };
    let method = if prepared.candidates.is_empty() { "trivial" } else { "search" };
    let game = Game {
        n,
        method,
        score,
        in_the_bag: board.in_the_bag,
        sum,
        reached_cap: settled.reached_cap,
        extended: settled.cost,
        default,
        known: known.is_some(),
    };
    Ok((game, Prev { score, moves }))
}

/// The line of a game on stdout.
fn describe(game: &Game) -> String {
    let Game { n, method, score, extended, default, .. } = game;
    let mut line =
        format!("n={n:5} {method:7} score {score:9} nodes {:10} {:8.3} s", extended.nodes, secs(extended.time));
    if let Some(default) = default {
        let ratio = ratio(extended.nodes, default.nodes);
        line += &format!(" | default nodes {:10} {:8.3} s ratio {ratio:.3}", default.nodes, secs(default.time));
    }
    line
}

/// The JSON object of a game, for `--out`.
fn json_line(game: &Game) -> Json {
    let seconds = |time: Duration| Json::from((secs(time) * 1e6).round() / 1e6);
    let mut pairs = vec![
        ("n", Json::from(game.n)),
        ("method", Json::from(game.method)),
        ("score", Json::from(game.score)),
        ("in_the_bag", Json::from(game.in_the_bag)),
        ("sum", Json::from(game.sum)),
        ("reached_cap", Json::Bool(game.reached_cap)),
        ("nodes", Json::from(game.extended.nodes)),
        ("secs", seconds(game.extended.time)),
    ];
    if let Some(default) = &game.default {
        pairs.push(("default_nodes", Json::from(default.nodes)));
        pairs.push(("default_secs", seconds(default.time)));
    }
    Json::object(pairs)
}

/// The nodes and the time of several searches together.
fn total(costs: impl Iterator<Item = Cost>) -> Cost {
    costs.fold(Cost::default(), |sum, cost| Cost { nodes: sum.nodes + cost.nodes, time: sum.time + cost.time })
}

/// A game's nodes with Π⁺(∅) and with the default problem.
type Compared = (u32, u64, u64);

/// Prints how many of `changed` there are and the `LISTED` with the largest (or the smallest) ratios.
fn print_changed(what: &str, mut changed: Vec<Compared>, largest_first: bool) {
    println!("  {} games where Π⁺(∅) took more than 10% {what} nodes than the default problem", changed.len());
    changed.sort_by(|a, b| ratio(a.1, a.2).total_cmp(&ratio(b.1, b.2)));
    if largest_first {
        changed.reverse();
    }
    for (n, extended, default) in changed.into_iter().take(LISTED) {
        println!("    n={n:5} extended {extended:10} default {default:10} ratio {:.3}", ratio(extended, default));
    }
}

fn print_summary(games: &[Game], compare: bool) {
    println!("summary: {} games, n = {}..={}", games.len(), games[0].n, games[games.len() - 1].n);
    let trivial = games.iter().filter(|game| game.method == "trivial").count();
    println!("  by method: trivial {trivial}, search {}", games.len() - trivial);
    let known = games.iter().filter(|game| game.known).count();
    println!("  scores: {known} of the games have a known score, and all of them equal it");
    let extended = total(games.iter().map(|game| game.extended));
    println!("  Π⁺(∅): {} nodes, {:.1} s", extended.nodes, secs(extended.time));
    if compare {
        let default = total(games.iter().filter_map(|game| game.default));
        let by = ratio(extended.nodes, default.nodes);
        println!(
            "  default problem: {} nodes, {:.1} s (Π⁺(∅) over default: {by:.3} in nodes)",
            default.nodes,
            secs(default.time)
        );
        let compared: Vec<Compared> =
            games.iter().filter_map(|game| game.default.map(|d| (game.n, game.extended.nodes, d.nodes))).collect();
        print_changed("more", compared.iter().copied().filter(|&(_, e, d)| 10 * e > 11 * d).collect(), true);
        print_changed("fewer", compared.iter().copied().filter(|&(_, e, d)| 10 * e < 9 * d).collect(), false);
    }
    let benchmarks: Vec<&Game> =
        BENCHMARK_GAMES.iter().filter_map(|&n| games.iter().find(|game| game.n == n)).collect();
    if !benchmarks.is_empty() {
        println!("  the benchmark games in range:");
        for game in benchmarks {
            println!("    {}", describe(game));
        }
    }
}

fn run(args: &Args) -> Result<(), String> {
    let optimal = Optimal::built_in().with_results_files(args.known.as_slice())?;
    let before = args.from - 1;
    let mut prev = match (optimal.score(before), optimal.moves(before)) {
        (Some(score), Some(moves)) => Prev { score, moves: moves.to_vec() },
        _ => return Err(format!("the game of {before} is not known; --known FILE can give it")),
    };
    let mut out = open_out(args.out.as_ref())?;
    let mut games = Vec::new();
    for n in args.from..=args.to {
        let (game, next) = taxman::with_width!(n, W => play::<W>(n, &prev, args.compare, &optimal))?;
        println!("{}", describe(&game));
        append(&mut out, &json_line(&game))?;
        games.push(game);
        prev = next;
    }
    print_summary(&games, args.compare);
    Ok(())
}

/// The `words` subcommand (see the module docs).
fn words(args: &Args) -> Result<(), String> {
    let upper = read_upper(&args.upper)?;
    let (mut by_words, mut widest, mut beyond) = (BTreeMap::<usize, usize>::new(), Vec::new(), Vec::new());
    for line in upper.range(args.from..=args.to).map(|(_, line)| line).filter(|line| line.needs) {
        let n = line.n;
        let (words, width) = certificate_width(n, &line.open)?;
        let own = width_for(words_for(n));
        let shown = match width {
            Some(width) if Some(width) == own => width.to_string(),
            Some(width) => {
                widest.push(n);
                width.to_string()
            }
            None => {
                beyond.push(n);
                "none".to_owned()
            }
        };
        println!("n={n:5} |O|={:4} words {words} width {shown}", line.open.len());
        *by_words.entry(words).or_default() += 1;
    }
    let counts: Vec<String> = by_words.iter().map(|(words, games)| format!("{words}: {games}")).collect();
    println!("games by block words: {}", counts.join(", "));
    println!("at the widest width ({}): {widest:?}", widest.len());
    println!("beyond the {MAX_WORDS} words of any engine ({}): {beyond:?}", beyond.len());
    Ok(())
}

/// A game of the run of `problems`, as a later game compares itself with it.
struct Posing {
    digest: u64,
    problem: Problem,
    /// The recorded game's worth in the problem, V − `in_the_bag⁺`.
    worth: u64,
}

/// What `problems` counts over its run, for the summary.
#[derive(Default)]
struct Tally {
    games: usize,
    same_as: usize,
    equal_worth: usize,
    by_width: BTreeMap<Option<usize>, usize>,
}

impl Tally {
    fn print(&self, start: Instant) {
        let widths: Vec<String> = self
            .by_width
            .iter()
            .map(|(width, games)| format!("{}: {games}", width.map_or("none".to_owned(), |width| width.to_string())))
            .collect();
        eprintln!(
            "{} games, {} with a same_as, of them {} with equal worth (certify settles those with no search); \
             by width: {}; {:.1} s",
            self.games,
            self.same_as,
            self.equal_worth,
            widths.join(", "),
            start.elapsed().as_secs_f64()
        );
    }
}

/// The `problems` subcommand (see the module docs).
fn problems(args: &Args) -> Result<(), String> {
    let start = Instant::now();
    let optimal = Optimal::built_in().with_results_files(args.known.as_slice())?;
    let upper = read_upper(&args.upper)?;
    let mut out = open_out(args.out.as_ref())?;
    let (mut earlier, mut tally) = (BTreeMap::<u32, Posing>::new(), Tally::default());
    for line in upper.range(args.from..=args.to).map(|(_, line)| line).filter(|line| line.needs) {
        let n = line.n;
        let score =
            optimal.score(n).ok_or_else(|| format!("n={n}: the game is not known; --known FILE can give it"))?;
        if score != line.score {
            return Err(format!("n={n}: the recorded score is {score}, the barred bounds' is {}", line.score));
        }
        let posed = extended_problem(n, &line.open)?;
        let (words, width) = certificate_width(n, &line.open)?;
        let worth =
            score.checked_sub(posed.in_the_bag).ok_or_else(|| format!("n={n}: the score is below in_the_bag"))?;
        let digest = posed.problem.digest();
        let same_as = (n.saturating_sub(SAME_SEARCH_WINDOW)..n)
            .rev()
            .find(|m| earlier.get(m).is_some_and(|m| m.digest == digest && m.problem == posed.problem));
        let json = Json::object([
            ("n", Json::from(n)),
            ("score", Json::from(score)),
            ("open", Json::from(line.open.len())),
            ("rejected", Json::from(posed.rejected)),
            ("frames", Json::from(posed.frames)),
            ("two_sided", Json::from(posed.two_sided)),
            ("one_sided", Json::from(posed.one_sided)),
            ("in_the_bag", Json::from(posed.in_the_bag)),
            ("worth", Json::from(worth)),
            ("words", Json::from(words)),
            ("width", width.map_or(Json::Null, Json::from)),
            ("digest", Json::from(digest_text(digest))),
            ("same_as", same_as.map_or(Json::Null, Json::from)),
        ]);
        println!("{json}");
        append(&mut out, &json)?;

        tally.games += 1;
        *tally.by_width.entry(width).or_default() += 1;
        if let Some(m) = same_as {
            tally.same_as += 1;
            tally.equal_worth += usize::from(earlier[&m].worth == worth);
        }
        earlier.insert(n, Posing { digest, problem: posed.problem, worth });
        earlier.retain(|&m, _| n - m <= SAME_SEARCH_WINDOW);
    }
    tally.print(start);
    Ok(())
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
    let ran = match args.command {
        Command::Chain => run(&args),
        Command::Words => words(&args),
        Command::Problems => problems(&args),
    };
    match ran {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
