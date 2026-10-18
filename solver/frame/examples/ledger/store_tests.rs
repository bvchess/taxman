//! The bounds store's tests: the format, the export of a ledger (by hand, with every rule, and of a run), the second
//! export that appends nothing, a run that starts from the file, and the two referees (`ledger verify-bounds` and
//! `solver/tools/check_bounds.py`) on the file and on files with a game, a root bound, a relation or a citation
//! tampered with.

use super::book::{upper_by, Book, Entry, Lower, Rule, Side, Upper};
use super::export::{self, civil_date, Options};
use super::schedule::{self, read_ledger, Settings};
use super::store::{self, Best, Chain, Game, Line, Store};
use super::support::{TempDir, QUICK};
use super::tests::{book_of, give, opt, opt_game, write_results, LAST};
use super::verify;
use super::STRICT;
use std::path::{Path, PathBuf};
use std::process::Command;
use taxman::board::Board;
use taxman::json::Json;

fn options() -> Options<'static> {
    Options { source: "test-run", date: "2026-10-17", checkpoints: Some("s3://bucket/run/ckpt/") }
}

/// The optimal game of `n` as promotions: its value, moves and the set.
fn promoted(n: u32) -> (u64, Vec<u32>, Vec<u32>) {
    let set = taxman::with_width!(n, W => Board::<W>::for_n(n).seed(&opt_game(n)).to_vec());
    let (value, moves) = taxman::store::expand(n, &set).unwrap();
    assert_eq!(value, opt(n), "n={n}: the promotions of the optimal game play it");
    (value, moves, set)
}

/// The boards of the hand-made books: 901..960, and two islands with no board next to them, 975 and 976 (which the
/// run found: its bound is the better for 975, which has nothing below it) and 980 (which the relations leave as it
/// was, with its start game).
fn boards() -> Vec<u32> {
    (901..=960).chain([975, 976, 980]).collect()
}

/// What the run found in the hand-made books, and the chain's games they are given.
const FOUND: [u32; 3] = [925, 950, 976];
const GIVEN: [u32; 2] = [920, 940];

/// A book of the boards `ns` as a run might leave them partway: every board has its start game (no promotions) and a
/// bound above the optimum (the root's bound for every tenth board, else a stopped search's), the boards `found` have
/// their optimal game (a search's) and an exhausted search, `given` boards are the chain's games, and the relations
/// have carried the bounds between the boards.
fn partial_book(ns: &[u32], given: &[u32], found: &[u32]) -> Book {
    let mut book = book_of(ns.iter().copied());
    for &n in ns {
        let moves = taxman::with_width!(n, W => Board::<W>::for_n(n).plain);
        let value = moves.iter().map(|&m| u64::from(m)).sum();
        let start = Lower { value, moves, promotions: Some(Vec::new()), rule: Rule::Start, from: None };
        book.offer_lower(n, start, false).unwrap();
        let upper = if n % 10 == 0 {
            Upper { value: verify::root_upper(n).unwrap(), rule: Rule::Root, from: None }
        } else if n == 975 {
            // So loose that the bound of 976, which the run found, is the better (monotone: nothing is below 975).
            Upper { value: opt(n) + 5000, rule: Rule::Dual, from: None }
        } else if n % 11 == 3 {
            // A search under a floor that completed having found nothing: the floor is the bound.
            Upper { value: opt(n) + 4 + u64::from(n % 5), rule: Rule::Floor, from: None }
        } else if n % 13 == 2 {
            Upper { value: opt(n) + 6 + u64::from(n % 3), rule: Rule::Nogoods, from: None }
        } else {
            Upper { value: opt(n) + 5 + u64::from(n % 7), rule: Rule::Dual, from: None }
        };
        book.offer_upper(n, upper, false).unwrap();
    }
    for &n in found {
        let (value, moves, set) = promoted(n);
        let lower = Lower { value, moves, promotions: Some(set), rule: Rule::Search, from: None };
        book.offer_lower(n, lower, false).unwrap();
        book.offer_upper(n, Upper { value, rule: Rule::Exhausted, from: None }, false).unwrap();
    }
    for &n in given {
        give(&mut book, n);
    }
    book.propagate(ns.iter().copied()).unwrap();
    book
}

fn entries(book: &Book) -> Vec<Entry> {
    book.entries.values().cloned().collect()
}

fn write_lines(path: &Path, lines: &[Line]) {
    std::fs::write(path, lines.iter().map(|line| format!("{}\n", line.to_json())).collect::<String>()).unwrap();
}

/// What `solver/tools/check_bounds.py` says of `file`: its exit status and what it printed.
fn python(file: &Path, games: Option<&Path>, strict: &Path) -> (Option<i32>, String) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/check_bounds.py");
    let mut command = Command::new("python3");
    command.arg("-I").arg(script).arg(file).arg("--strict").arg(strict);
    if let Some(games) = games {
        command.arg("--games").arg(games);
    }
    let run = command.output().expect("python3 runs solver/tools/check_bounds.py");
    (run.status.code(), String::from_utf8_lossy(&run.stdout).into_owned())
}

/// A store made by hand from [`partial_book`]: the chain has 920 and 940, the run found 925 and 950, and the file has
/// every rule of an upper bound and a lower bound.
struct Fixture {
    dir: TempDir,
    strict: PathBuf,
    bounds: PathBuf,
    games: PathBuf,
    lines: Vec<Line>,
}

fn fixture(name: &str) -> Fixture {
    let dir = TempDir::new(name);
    let (strict, bounds, games) = (dir.join("strict.jsonl"), dir.join("bounds.jsonl"), dir.join("games.jsonl"));
    write_results(&strict, &[920, 940]);
    let book = partial_book(&boards(), &GIVEN, &FOUND);
    let chain = store::chain(&strict).unwrap();
    let (lines, _) = export::plan(&entries(&book), &Store::default(), &chain, &options()).unwrap();
    Store::append(&bounds, &lines).unwrap();
    verify::games(&bounds, &mut std::fs::File::create(&games).unwrap(), 4).unwrap();
    Fixture { dir, strict, bounds, games, lines }
}

fn line_of(lines: &[Line], pick: impl Fn(&Line) -> bool) -> usize {
    lines.iter().position(pick).expect("a line of the kind in the file")
}

/// The dates: days since 1970-01-01 to a civil date, at known days.
#[test]
fn the_date_of_an_export_is_the_civil_date() {
    assert_eq!(civil_date(0), "1970-01-01");
    assert_eq!(civil_date(19_000), "2022-01-08");
    assert_eq!(civil_date(11_016), "2000-02-29");
    assert_eq!(civil_date(20_743), "2026-10-17");
    assert!(store::is_date(&export::today()) && !store::is_date("2026-13-01") && !store::is_date("2026-1-01"));
}

/// The arithmetic of the relations is the book's: what `upper_by` gives is what the lemmas say, from the boards they
/// relate and no others.
#[test]
fn the_arithmetic_of_the_relations_is_one_function_that_refuses_boards_they_do_not_relate() {
    assert_eq!(upper_by(Rule::Ceiling, 900, 901, 1000, None), Ok(1901));
    assert_eq!(upper_by(Rule::Monotone, 901, 900, 1000, None), Ok(1000));
    // 907 is prime, the largest prime below it 887 (by 20, the exchange of the opening 887 for 907), and the other way.
    assert_eq!(upper_by(Rule::Prime, 906, 907, 1000, None), Ok(1000 + 907 - 887));
    assert_eq!(upper_by(Rule::Prime, 907, 906, 1000 + 20, None), Ok(1000));
    assert_eq!(upper_by(Rule::Repeat, 900, 910, 1000, Some((300, 350))), Ok(1050));
    for (rule, source, target) in
        [(Rule::Ceiling, 901, 900), (Rule::Monotone, 900, 901), (Rule::Prime, 900, 901), (Rule::Repeat, 900, 900)]
    {
        assert!(upper_by(rule, source, target, 1, Some((1, 1))).is_err(), "{rule:?} {source} {target}");
    }
    assert!(upper_by(Rule::Repeat, 900, 910, 1, None).is_err() && upper_by(Rule::Root, 900, 901, 1, None).is_err());
}

fn sample(side: Side, rule: Rule) -> Line {
    let (game, rests_on) = match side {
        Side::Lower => (Some(Game::Promotions(vec![3, 5])), Vec::new()),
        Side::Upper => (None, vec![901]),
    };
    let relation = rule.is_relation();
    Line {
        n: 901,
        side,
        value: 5000,
        rule,
        from: relation.then_some(900),
        game,
        from_value: (relation && side == Side::Upper).then_some(4000),
        bags: (rule == Rule::Repeat && side == Side::Upper).then_some((10, 20)),
        rests_on,
        checkpoint: None,
        source: "run".into(),
        date: "2026-10-17".into(),
    }
}

/// A line is written compactly and read back as it was; a line with a field not in the format, no `rests_on`, a rule its
/// side does not have, a relation with no board, a game on an upper bound, or promotions out of order is refused.
#[test]
fn a_line_is_written_and_read_back_and_a_malformed_one_is_refused() {
    let read = |line: &Line| Line::from_json(&Json::parse(&line.to_json().to_string()).unwrap());
    let mut lines =
        vec![sample(Side::Lower, Rule::Lns), sample(Side::Lower, Rule::Given), sample(Side::Upper, Rule::Dual)];
    lines[1].game = Some(Game::Moves(vec![7, 6, 5]));
    lines[2].checkpoint = Some("s3://bucket/n901.ckpt".into());
    // 900 -> 901 is a ceiling, and 902 -> 901 is monotone; a repeat has its two bags.
    let mut monotone = sample(Side::Upper, Rule::Monotone);
    monotone.from = Some(902);
    // A game cited, not written: 900 -> 901 is monotone (901 is composite).
    let carried = Line { game: None, from_value: Some(4000), ..sample(Side::Lower, Rule::Monotone) };
    lines.extend([
        sample(Side::Lower, Rule::Ceiling),
        sample(Side::Upper, Rule::Ceiling),
        monotone,
        sample(Side::Upper, Rule::Repeat),
        carried,
    ]);
    for line in &lines {
        assert_eq!(read(line).as_ref(), Ok(line), "{line:?}");
    }
    let text = lines[0].to_json().to_string();
    assert_eq!(
        text,
        "{\"schema\":1,\"n\":901,\"side\":\"lower\",\"value\":5000,\"rule\":\"lns\",\"source\":\"run\",\
         \"date\":\"2026-10-17\",\"promotions\":[3,5]}"
    );

    let broken: Vec<(Line, &str)> = vec![
        (Line { rests_on: Vec::new(), ..sample(Side::Upper, Rule::Dual) }, "rests_on"),
        (Line { rests_on: vec![902], ..sample(Side::Upper, Rule::Dual) }, "rests on that board alone"),
        (Line { rule: Rule::Root, side: Side::Lower, ..sample(Side::Lower, Rule::Lns) }, "no such rule"),
        (Line { from: None, ..sample(Side::Upper, Rule::Ceiling) }, "board it came from"),
        (Line { from: Some(901), ..sample(Side::Upper, Rule::Ceiling) }, "its own board"),
        (Line { from: Some(895), ..sample(Side::Upper, Rule::Ceiling) }, "cannot come from"),
        (Line { game: Some(Game::Moves(vec![2])), ..sample(Side::Upper, Rule::Dual) }, "has a game"),
        (Line { game: None, ..sample(Side::Lower, Rule::Lns) }, "needs its game"),
        (Line { game: Some(Game::Promotions(vec![5, 3])), ..sample(Side::Lower, Rule::Lns) }, "ascending"),
        (Line { bags: None, ..sample(Side::Upper, Rule::Repeat) }, "two bags"),
        (Line { checkpoint: Some("x".into()), ..sample(Side::Upper, Rule::Exhausted) }, "checkpoint"),
        (Line { date: "2026-10".into(), ..sample(Side::Lower, Rule::Lns) }, "date"),
        (Line { n: 1, ..sample(Side::Lower, Rule::Lns) }, "from 2 to 8191"),
        (Line { n: 8192, ..sample(Side::Lower, Rule::Lns) }, "from 2 to 8191"),
        (Line { game: None, ..sample(Side::Lower, Rule::Lns) }, "needs its game"),
        (Line { game: None, from_value: Some(7), ..sample(Side::Lower, Rule::Lns) }, "does not carry"),
        (Line { from_value: Some(7), ..sample(Side::Lower, Rule::Monotone) }, "has a game and cites one"),
        (Line { game: None, from_value: Some(7), ..sample(Side::Lower, Rule::Repeat) }, "does not carry"),
        (
            Line { game: None, from_value: Some(7), from: Some(895), ..sample(Side::Lower, Rule::Monotone) },
            "does not carry",
        ),
        (
            Line { game: None, from_value: Some(7), from: Some(900), ..sample(Side::Lower, Rule::Ceiling) },
            "does not carry",
        ),
    ];
    for (line, expected) in broken {
        let err = Line::from_json(&Json::parse(&line.to_json().to_string()).unwrap()).unwrap_err();
        assert!(err.contains(expected), "{expected}: {err}");
    }
    let mut json = lines[0].to_json();
    if let Json::Obj(fields) = &mut json {
        fields.push(("extra".into(), Json::from(1u32)));
    }
    assert!(Line::from_json(&json).unwrap_err().contains("not in the format"));
    let schema2 = text.replace("\"schema\":1", "\"schema\":2");
    assert!(Line::from_json(&Json::parse(&schema2).unwrap()).unwrap_err().contains("schema"));
}

/// An export puts the lower bounds first and each upper bound after the bounds it cites; leaves out the boards the chain
/// has proven and cites them where a relation comes from one; and records what each upper bound rests on.  A second export of
/// the same ledger appends nothing; after more is found, an export appends exactly the improvements, which pass the checks.
#[test]
fn an_export_appends_the_improvements_only_and_a_second_export_appends_nothing() {
    let dir = TempDir::new("store-export");
    let (strict, bounds) = (dir.join("strict.jsonl"), dir.join("bounds.jsonl"));
    write_results(&strict, &[920, 940]);
    let chain = store::chain(&strict).unwrap();
    let mut book = partial_book(&boards(), &GIVEN, &FOUND);

    let (lines, summary) = export::plan(&entries(&book), &Store::default(), &chain, &options()).unwrap();
    assert!(lines.iter().all(|l| l.n != 920 && l.n != 940), "the chain's boards have no line");
    assert_eq!(summary.chain, 2);
    let first_upper = line_of(&lines, |l| l.side == Side::Upper);
    assert!(lines[..first_upper].iter().all(|l| l.side == Side::Lower));
    assert!(lines[first_upper..].iter().all(|l| l.side == Side::Upper));
    let lowers = lines.iter().filter(|l| l.side == Side::Lower).count();
    assert_eq!((summary.lines, summary.lower_first, summary.upper_first), (lines.len(), lowers, lines.len() - lowers));
    assert_eq!((summary.lower_better, summary.upper_better), (0, 0));
    // A game carried by a relation from the game of another board is cited (`from_value`), after the line it cites.
    let cited: Vec<usize> =
        (0..lines.len()).filter(|&i| lines[i].side == Side::Lower && lines[i].game.is_none()).collect();
    assert!(cited.len() > 5, "{} cited games", cited.len());
    for &i in &cited {
        let (from, value) = (lines[i].from.unwrap(), lines[i].from_value.unwrap());
        assert!(matches!(lines[i].rule, Rule::Monotone | Rule::Ceiling | Rule::Prime));
        assert!(lines[..i].iter().any(|l| l.side == Side::Lower && (l.n, l.value) == (from, value)), "{:?}", lines[i]);
    }
    // None cites a game of the chain, which a run need not have: the boards next to 920 and 940 have their games written.
    assert!(lines.iter().all(|l| l.side != Side::Lower || l.game.is_some() || ![920, 940].contains(&l.from.unwrap())));

    // Every rule of the format is there, with what it rests on.
    for rule in [Rule::Start, Rule::Search, Rule::Monotone, Rule::Ceiling, Rule::Prime, Rule::Repeat] {
        assert!(lines.iter().any(|l| l.side == Side::Lower && l.rule == rule), "no lower bound by {rule:?}");
    }
    for rule in [
        Rule::Root,
        Rule::Dual,
        Rule::Exhausted,
        Rule::Floor,
        Rule::Nogoods,
        Rule::Ceiling,
        Rule::Monotone,
        Rule::Prime,
        Rule::Repeat,
    ] {
        assert!(lines.iter().any(|l| l.side == Side::Upper && l.rule == rule), "no upper bound by {rule:?}");
    }
    for line in lines.iter().filter(|l| l.side == Side::Upper) {
        match line.rule {
            Rule::Root | Rule::Dual | Rule::Exhausted | Rule::Floor | Rule::Nogoods => {
                assert_eq!(line.rests_on, [line.n]);
                assert_eq!(line.from, None);
            }
            Rule::Repeat => assert!(line.rests_on.contains(&line.n) && line.rests_on.len() >= 2, "{line:?}"),
            _ => assert!(!line.rests_on.is_empty()),
        }
        assert_eq!(
            line.checkpoint.is_some(),
            line.rule == Rule::Dual
                && book.entries[&line.n].upper.unwrap().rule == Rule::Dual
                && !book.entries[&line.n].proven()
        );
    }
    let dual = lines.iter().find(|l| l.rule == Rule::Dual).unwrap();
    assert_eq!(dual.checkpoint.as_deref(), Some(format!("s3://bucket/run/ckpt/n{}.ckpt", dual.n).as_str()));
    // A relation from a board of the chain cites it: 921 is a ceiling of 920 or a repeat of it, resting on 920.
    let from_chain: Vec<&Line> = lines.iter().filter(|l| l.from.is_some_and(|f| f == 920 || f == 940)).collect();
    assert!(
        from_chain.iter().any(|l| l.side == Side::Upper && l.rests_on.contains(&l.from.unwrap())),
        "{from_chain:?}"
    );
    // The proven boards: the two found, and what the relations settled around them and the chain's games.
    let proven = Best::of(&lines).proven();
    assert!(proven.contains(&925) && proven.contains(&950) && proven.len() > 6, "{proven:?}");
    assert_eq!(summary.proven, proven.len());

    Store::append(&bounds, &lines).unwrap();
    let on_disk = Store::read(&bounds).unwrap();
    assert_eq!(on_disk.lines, lines);
    let text = std::fs::read_to_string(&bounds).unwrap();
    let (again, summary) = export::plan(&entries(&book), &on_disk, &chain, &options()).unwrap();
    assert!(again.is_empty() && summary.lines == 0 && summary.newly_proven.is_empty(), "{again:?}");
    assert_eq!(summary.proven, proven.len());

    // More is found: 930's optimal game and an exhausted search, and 910's optimal game; the relations carry them.
    for n in [930, 910] {
        let (value, moves, set) = promoted(n);
        book.offer_lower(n, Lower { value, moves, promotions: Some(set), rule: Rule::Search, from: None }, false)
            .unwrap();
    }
    book.offer_upper(930, Upper { value: opt(930), rule: Rule::Exhausted, from: None }, false).unwrap();
    book.propagate([930, 910]).unwrap();
    let before = on_disk.best();
    let (more, summary) = export::plan(&entries(&book), &on_disk, &chain, &options()).unwrap();
    assert!(more.len() > 2 && summary.newly_proven.contains(&930), "{summary}");
    for line in &more {
        match line.side {
            Side::Lower => assert!(before.lower.get(&line.n).is_none_or(|b| b.value < line.value), "{line:?}"),
            Side::Upper => assert!(before.upper.get(&line.n).is_none_or(|b| b.value > line.value), "{line:?}"),
        }
    }
    assert!(
        more.iter().any(|l| l.n == 930 && l.side == Side::Lower)
            && more.iter().any(|l| l.n == 930 && l.side == Side::Upper)
    );
    // Each upper bound comes after the bound it cites: 931's ceiling from the exhausted 930 is after it.
    let at = |pick: &dyn Fn(&Line) -> bool| more.iter().position(pick);
    if let (Some(cited), Some(citing)) = (
        at(&|l| l.n == 930 && l.side == Side::Upper),
        at(&|l| l.n == 931 && l.side == Side::Upper && l.from == Some(930)),
    ) {
        assert!(cited < citing);
    }
    Store::append(&bounds, &more).unwrap();
    assert!(std::fs::read_to_string(&bounds).unwrap().starts_with(&text), "a line is never changed");
    let all = Store::read(&bounds).unwrap();
    let mut checks = Chain::new(&chain);
    for line in &all.lines {
        checks.add(line).unwrap_or_else(|e| panic!("{line:?}: {e}"));
    }
    let (nothing, _) = export::plan(&entries(&book), &all, &chain, &options()).unwrap();
    assert!(nothing.is_empty());
    let (text, ok) = verify::verify(&bounds, &strict, 4).unwrap();
    assert!(ok, "{text}");
}

/// A stretch's worth of boards each of which cites the next one above (upper bounds by monotonicity, down from one dual
/// bound) is exported, each line after the line it cites, however deep the citations go.
#[test]
fn a_long_chain_of_citations_is_exported_in_order() {
    let dir = TempDir::new("store-deep");
    let strict = dir.join("strict.jsonl");
    write_results(&strict, &[]);
    let (first, top) = (2000, 5000);
    let entries: Vec<Entry> = (first..=top)
        .map(|n| {
            let (rule, from) = if n == top { (Rule::Dual, None) } else { (Rule::Monotone, Some(n + 1)) };
            Entry { n, upper: Some(Upper { value: 7_000_000, rule, from }), ..Entry::default() }
        })
        .collect();
    let chain = store::chain(&strict).unwrap();
    let (lines, summary) = export::plan(&entries, &Store::default(), &chain, &options()).unwrap();
    assert_eq!((lines.len(), summary.upper_first), ((top - first + 1) as usize, lines.len()));
    for (k, line) in lines.iter().enumerate() {
        assert_eq!((line.n, line.side), (top - k as u32, Side::Upper), "line {k}");
    }
    assert_eq!(lines[1].from_value, Some(7_000_000));
    assert!(lines.iter().all(|l| l.rests_on == [top]));
    // Both referees follow the citations along the file, without recursion (Python's limit is 1,000 calls deep, a test
    // thread's stack 2 MB): 3,001 lines cite the line before them.
    let bounds = dir.join("bounds.jsonl");
    Store::append(&bounds, &lines).unwrap();
    let (text, ok) = verify::verify(&bounds, &strict, 2).unwrap();
    assert!(ok && text.contains("3001 lines"), "{text}");
    let (status, report) = python(&bounds, None, &strict);
    assert_eq!(status, Some(0), "{report}");
}

/// A file whose last line is torn is not appended to; a missing file is made; the old lines stay as they were.
#[test]
fn append_refuses_a_torn_file_and_keeps_the_lines_it_has() {
    let dir = TempDir::new("store-append");
    let path = dir.join("bounds.jsonl");
    let (a, b) = (sample(Side::Lower, Rule::Lns), Line { value: 6000, ..sample(Side::Lower, Rule::Lns) });
    Store::append(&path, std::slice::from_ref(&a)).unwrap();
    let one = std::fs::read_to_string(&path).unwrap();
    Store::append(&path, std::slice::from_ref(&b)).unwrap();
    assert!(std::fs::read_to_string(&path).unwrap().starts_with(&one));
    assert_eq!(Store::read(&path).unwrap().lines, [a.clone(), b.clone()]);
    std::fs::OpenOptions::new().append(true).open(&path).unwrap().set_len(one.len() as u64 + 20).unwrap();
    assert!(Store::append(&path, &[a]).unwrap_err().contains("torn"));
    assert!(Store::read(&path).unwrap_err().contains("line 2"));
    let none = dir.join("none.jsonl");
    assert!(Store::read_or_new(&none).unwrap().lines.is_empty());
    assert!(Store::read(&none).unwrap_err().contains("none.jsonl"));
}

fn all_proven_run(dir: &Path, first: u32, last: u32, proven: Option<&Path>, bounds: Option<&Path>) -> PathBuf {
    let settings = Settings {
        workers: 3,
        cpu_budget: 1e9,
        base: 0.002,
        climb_limit: 0.004,
        lns_seconds: 0.0,
        save_every: 0.5,
        bounds: bounds.map(Path::to_path_buf),
        ..Settings::default()
    };
    let run_dir = dir.join(format!("run-{first}-{last}-{}", u8::from(bounds.is_some())));
    assert_eq!(schedule::run(first, last, &run_dir, &settings, proven).unwrap(), "all proven");
    run_dir
}

/// A run of 901..`LAST` (901..960) with the games of 900 and one in the middle (930) given (the chain's, so the export
/// leaves them out) and the rest hidden: its export has every other board proven, a second export appends nothing, and
/// both referees accept the file, the Python one with the games `ledger games` writes.
#[test]
fn a_run_is_exported_and_both_referees_accept_the_file() {
    let dir = TempDir::new("store-run");
    let (strict, bounds, games) = (dir.join("strict.jsonl"), dir.join("bounds.jsonl"), dir.join("games.jsonl"));
    let (middle, proven) = (if QUICK { 906 } else { 930 }, (LAST - 901) as usize);
    write_results(&strict, &[900, middle]);
    let run_dir = all_proven_run(&dir, 901, LAST, Some(&strict), None);
    let ledger = read_ledger(&run_dir).unwrap();
    assert!(ledger.iter().all(Entry::proven));

    let summary = export::export(&run_dir, &bounds, &strict, &options()).unwrap();
    assert_eq!((summary.chain, summary.proven, summary.newly_proven.len()), (2, proven, proven), "{summary}");
    assert_eq!((summary.boards, summary.lower_better, summary.upper_better), (proven, 0, 0), "{summary}");
    let ranges = format!("901-{}, {}-{LAST}", middle - 1, middle + 1);
    assert!(summary.to_string().contains(&format!("{proven} boards newly proven ({ranges})")), "{summary}");
    let bytes = std::fs::read(&bounds).unwrap();
    let again = export::export(&run_dir, &bounds, &strict, &options()).unwrap();
    assert_eq!((again.lines, again.boards, again.newly_proven.len(), again.proven), (0, 0, 0, proven));
    assert_eq!(std::fs::read(&bounds).unwrap(), bytes, "nothing appended, nothing changed");

    let (text, ok) = verify::verify(&bounds, &strict, 4).unwrap();
    assert!(ok, "{text}");
    assert!(
        text.contains(&format!("proven (best lower bound = best upper bound): {proven} boards: {ranges}")),
        "{text}"
    );
    let count = verify::games(&bounds, &mut std::fs::File::create(&games).unwrap(), 4).unwrap();
    assert!(count > proven / 3, "{count} promotion sets");
    let (status, report) = python(&bounds, Some(&games), &strict);
    assert_eq!(status, Some(0), "{report}");
    assert!(report.contains("all checks passed") && report.contains(&format!("{proven} boards")), "{report}");
    // Without the games the Python referee cannot replay the promotion sets, and says so.
    let (status, report) = python(&bounds, None, &strict);
    assert_eq!(status, Some(1), "{report}");
    assert!(report.contains("was not replayed"), "{report}");
    // Bytes a board: the whole file over the boards it has.
    let per_board = bytes.len() as f64 / proven as f64;
    eprintln!(
        "{} lines, {} bytes, {per_board:.0} bytes a board",
        Store::read(&bounds).unwrap().lines.len(),
        bytes.len()
    );
}

/// What both referees say when the file is as the export wrote it, and when a line is tampered with: the line is named,
/// and the Rust referee, which recomputes what the Python one takes on record (a root bound, the bags and search
/// problems of repeats), finds more.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is both_referees_refuse_a_sample_of_the_tampered_files_and_name_the_line"
)]
fn both_referees_refuse_a_tampered_file_and_name_the_line() {
    both_referees_refuse_the_tampered_files(|_| true);
}

/// [`both_referees_refuse_a_tampered_file_and_name_the_line`] for three of its cases: the third, eighth and last (a root
/// bound lowered, a repeat's bag changed and an upper bound below the chain's score), the first two of which the Python
/// referee cannot find.
#[test]
fn both_referees_refuse_a_sample_of_the_tampered_files_and_name_the_line() {
    both_referees_refuse_the_tampered_files(|case| case % 5 == 2);
}

/// The test of the cases `sampled` picks (by their index).
fn both_referees_refuse_the_tampered_files(sampled: impl Fn(usize) -> bool) {
    let f = fixture("store-tamper");
    let ok = verify::verify(&f.bounds, &f.strict, 4).unwrap();
    assert!(ok.1, "{}", ok.0);
    let (status, report) = python(&f.bounds, Some(&f.games), &f.strict);
    assert_eq!(status, Some(0), "{report}");
    // The bounds of floors and nogoods are taken on record, and both referees list them with the others.
    for rule in ["exhausted", "dual", "floor", "nogoods"] {
        let lines = f.lines.iter().filter(|l| l.side == Side::Upper && l.rule.name() == rule).count();
        let listed = format!("on record, not recomputed: {lines} {rule} lines; best upper bound of ");
        assert!(lines > 0 && ok.0.contains(&listed) && report.contains(&listed), "{rule}: {}\n{report}", ok.0);
    }

    // Each case edits the lines and says which line (by index) the referees must name, whether the Python one finds it too
    // (it replays games and checks arithmetic; it cannot recompute a root bound or a repeat's problem), and a part of the
    // Rust message.
    type Edit = Box<dyn Fn(&mut Vec<Line>) -> usize>;
    let lower_moves = |l: &Line| l.side == Side::Lower && matches!(l.game, Some(Game::Moves(_)));
    let lower_promoted = |l: &Line| l.side == Side::Lower && l.rule == Rule::Search;
    let upper_by = |rule: Rule| move |l: &Line| l.side == Side::Upper && l.rule == rule;
    let cases: Vec<(&str, Edit, bool, &str)> = vec![
        (
            "a game of moves with its first two moves swapped",
            Box::new(move |lines| {
                let k = line_of(lines, lower_moves);
                if let Some(Game::Moves(moves)) = &mut lines[k].game {
                    moves.swap(0, 1);
                }
                k
            }),
            true,
            "the game",
        ),
        (
            "a promotion set's value raised by one",
            Box::new(move |lines| {
                let k = line_of(lines, lower_promoted);
                lines[k].value += 1;
                k
            }),
            true,
            "scores",
        ),
        (
            "a root bound lowered by one",
            Box::new(move |lines| {
                let k = line_of(lines, upper_by(Rule::Root));
                lines[k].value -= 1;
                k
            }),
            false,
            "the root bound is",
        ),
        (
            "a ceiling bound lowered by one",
            Box::new(move |lines| {
                let k = line_of(lines, upper_by(Rule::Ceiling));
                lines[k].value -= 1;
                k
            }),
            true,
            "but the line says",
        ),
        (
            "the line a relation cites removed",
            Box::new(move |lines| {
                let k = line_of(lines, |l| l.side == Side::Upper && l.rule == Rule::Ceiling && l.from == Some(925));
                let cited = line_of(lines, |l| l.side == Side::Upper && l.n == 925);
                lines.remove(cited);
                k - usize::from(cited < k)
            }),
            true,
            "bounds board 925",
        ),
        (
            "the board a repeat's bound rests on dropped",
            Box::new(move |lines| {
                let k = line_of(lines, |l| l.side == Side::Upper && l.rule == Rule::Repeat && l.from != Some(920));
                let from = lines[k].from.unwrap();
                lines[k].rests_on.retain(|&b| b != from);
                k
            }),
            true,
            "leaves out",
        ),
        (
            "a repeat's own board dropped from what it rests on",
            Box::new(move |lines| {
                let k = line_of(lines, upper_by(Rule::Repeat));
                let n = lines[k].n;
                lines[k].rests_on.retain(|&b| b != n);
                if lines[k].rests_on.is_empty() {
                    lines[k].rests_on.push(0);
                }
                k
            }),
            true,
            "leaves out",
        ),
        (
            "a repeat's bag changed",
            Box::new(move |lines| {
                let k = line_of(lines, upper_by(Rule::Repeat));
                let (bag, from_bag) = lines[k].bags.unwrap();
                lines[k].bags = Some((bag - 1, from_bag));
                k
            }),
            false,
            "the bags are",
        ),
        (
            "the value that a carried game cites changed",
            Box::new(move |lines| {
                let k = line_of(lines, |l| l.side == Side::Lower && l.game.is_none());
                lines[k].from_value = lines[k].from_value.map(|value| value + 1);
                k
            }),
            true,
            "no line before it gives a game",
        ),
        (
            "a carried game moved before the line it cites",
            Box::new(move |lines| {
                let k = line_of(lines, |l| l.side == Side::Lower && l.game.is_none());
                let (from, value) = (lines[k].from.unwrap(), lines[k].from_value.unwrap());
                let cited = line_of(lines, |l| l.side == Side::Lower && (l.n, l.value) == (from, value));
                lines.swap(cited, k);
                cited
            }),
            true,
            "no line before it gives a game",
        ),
        (
            "the game that another line cites is not a game",
            Box::new(move |lines| {
                let k = line_of(lines, |l| l.side == Side::Lower && l.game.is_none());
                let (from, value) = (lines[k].from.unwrap(), lines[k].from_value.unwrap());
                let cited = line_of(lines, |l| l.side == Side::Lower && (l.n, l.value) == (from, value));
                lines[cited].value += 1;
                cited
            }),
            true,
            "the game",
        ),
        (
            "a line that improves nothing",
            Box::new(move |lines| {
                let k = line_of(lines, upper_by(Rule::Dual));
                lines.push(lines[k].clone());
                lines.len() - 1
            }),
            true,
            "does not improve",
        ),
        (
            "an upper bound below the chain's score",
            Box::new(move |lines| {
                let mut line = lines[line_of(lines, upper_by(Rule::Dual))].clone();
                (line.n, line.value, line.rests_on) = (920, opt(920) - 1, vec![920]);
                lines.push(line);
                lines.len() - 1
            }),
            true,
            "n=920: the upper bound",
        ),
    ];
    for (case, (what, edit, python_finds, expected)) in cases.into_iter().enumerate() {
        if !sampled(case) {
            continue;
        }
        let mut lines = f.lines.clone();
        let k = edit(&mut lines);
        let path = f.dir.join("tampered.jsonl");
        write_lines(&path, &lines);
        // A board of the chain that the file's bounds do not enclose is named by its board, with no line.
        let named =
            if expected.starts_with("n=") { expected.to_owned() } else { format!("line {} (n={})", k + 1, lines[k].n) };
        let (text, ok) = verify::verify(&path, &f.strict, 4).unwrap();
        assert!(!ok && text.contains(&named) && text.contains(expected), "{what}: {named}, {expected}\n{text}");
        let (status, report) = python(&path, Some(&f.games), &f.strict);
        assert_eq!(status, Some(i32::from(python_finds)), "{what}\n{report}");
        if python_finds {
            assert!(report.contains(&named), "{what}: {named}\n{report}");
        }
    }
}

/// A run started from the file does not map the boards the file has both bounds of, and ends where a run without it
/// ends: every board proven at the optimum.  The file is by hand, with loose bounds (a start game and a stopped
/// search's bound above the optimum) for every board, so that none is proven at the start.
#[test]
fn a_run_starts_from_the_file_and_skips_the_maps_of_boards_with_both_bounds() {
    let dir = TempDir::new("store-start");
    let (strict, bounds) = (dir.join("strict.jsonl"), dir.join("bounds.jsonl"));
    // The stretch `first..=last`, the first board that the file has, the last board of the file and the boards that the
    // run found: 901..960, 910, 960, and 925 and 950; in a quick run 115..124, 119, 127 and 121 (small boards, whose runs
    // are short, with a game beside the one it cites in the repeat class of 121).
    let (first, split, last, top, found): (u32, u32, u32, u32, &[u32]) =
        if QUICK { (115, 119, 124, 127, &[121]) } else { (901, 910, 960, 960, &[925, 950]) };
    write_results(&strict, &[first - 1]);
    let book = partial_book(&(first - 1..=top).collect::<Vec<_>>(), &[first - 1], found);
    let chain = store::chain(&strict).unwrap();
    let (lines, _) = export::plan(&entries(&book), &Store::default(), &chain, &options()).unwrap();
    // The file has the bounds of every board from `split` on and none before it.
    let lines: Vec<Line> = lines.into_iter().filter(|l| l.n >= split).collect();
    write_lines(&bounds, &lines);
    let best = Store::read(&bounds).unwrap();
    let both = |n: u32| best.best().lower.contains_key(&n) && best.best().upper.contains_key(&n);
    assert!((split..=top).all(both) && !(first..split).any(both));

    let inits = |run_dir: &Path| -> Vec<u32> {
        let events = std::fs::read_to_string(schedule::events_path(run_dir)).unwrap();
        let events = events.lines().map(|line| Json::parse(line).unwrap());
        events
            .filter(|e| e.get("action").and_then(Json::as_str) == Some("init"))
            .filter_map(|e| e.get("n").and_then(Json::as_u32))
            .collect()
    };
    let fresh = all_proven_run(&dir, first, last, Some(&strict), None);
    let started = all_proven_run(&dir, first, last, Some(&strict), Some(&bounds));
    for run_dir in [&fresh, &started] {
        for e in read_ledger(run_dir).unwrap() {
            assert!(e.proven() && e.upper.unwrap().value == opt(e.n), "n={}", e.n);
        }
    }
    let (fresh_maps, started_maps) = (inits(&fresh), inits(&started));
    assert!(fresh_maps.iter().any(|&n| n >= split), "a fresh run maps boards of the file: {fresh_maps:?}");
    assert!(started_maps.iter().all(|&n| n < split), "boards of the file are not mapped: {started_maps:?}");
    assert!(!started_maps.is_empty(), "the boards below the file's are");
    let events = std::fs::read_to_string(schedule::events_path(&started)).unwrap();
    assert!(events.contains("\"bounds_file\""), "the start event names the file");

    // A game of the file that is not a game of its board stops the run at the start, naming the board; a game that
    // other lines cite stops it at a line that cites it (here 923, carried from 924, carried from 925).
    let cited: Vec<(u32, u64)> =
        lines.iter().filter(|l| l.side == Side::Lower).filter_map(|l| Some((l.from?, l.from_value?))).collect();
    let alone = |l: &Line| {
        l.side == Side::Lower && matches!(l.game, Some(Game::Promotions(_))) && !cited.contains(&(l.n, l.value))
    };
    let settings =
        Settings { bounds: Some(bounds.clone()), cpu_budget: 1e9, lns_seconds: 0.0, workers: 2, ..Settings::default() };
    let mut bad = lines.clone();
    let k = line_of(&bad, alone);
    bad[k].value += 1;
    write_lines(&bounds, &bad);
    let err = schedule::run(first, last, &dir.join("run-bad"), &settings, Some(&strict)).unwrap_err();
    assert!(err.contains(&format!("n={}: the game", bad[k].n)), "{err}");
    let mut bad = lines.clone();
    let k = line_of(&bad, |l| l.side == Side::Lower && cited.contains(&(l.n, l.value)));
    bad[k].value += 1;
    write_lines(&bounds, &bad);
    let err = schedule::run(first, last, &dir.join("run-worse"), &settings, Some(&strict)).unwrap_err();
    assert!(err.contains("which no line before it has") || err.contains("the game"), "{err}");
}

/// The Python referee needs only the standard library: it runs under `python3 -I` and prints a report.
#[test]
fn the_python_referee_prints_its_usage() {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/check_bounds.py");
    let run = Command::new("python3").arg("-I").arg(script).arg("--help").output().expect("python3 runs the referee");
    assert!(run.status.success() && String::from_utf8_lossy(&run.stdout).contains("--games"));
}

/// The committed store, `results/bounds.jsonl`, checked against the chain `STRICT` (`results/strict.jsonl`).
const COMMITTED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../results/bounds.jsonl");

/// The lines of the committed store that the quick check verifies: an export writes its lower bounds first, so these
/// are games replayed by the referee, and the whole file, its upper bounds with it, is the long test's (and is checked
/// at every export).
const QUICK_LINES: usize = 10;

/// The first lines of the committed store pass the verifier.  A prefix of an append-only store is a store too: a line
/// cites only lines before it, so the first `QUICK_LINES` pass as the whole file does.
#[test]
fn the_start_of_the_committed_bounds_passes_the_verifier() {
    let dir = TempDir::new("committed-bounds");
    let prefix = dir.join("bounds.jsonl");
    let text: String = std::fs::read_to_string(COMMITTED).unwrap().split_inclusive('\n').take(QUICK_LINES).collect();
    assert_eq!(text.lines().count(), QUICK_LINES, "the committed store has that many lines");
    std::fs::write(&prefix, &text).unwrap();
    let (report, ok) = verify::verify(&prefix, Path::new(STRICT), 4).unwrap();
    assert!(ok, "{report}");
}

/// The whole committed store passes the verifier.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is the_start_of_the_committed_bounds_passes_the_verifier")]
fn the_committed_bounds_pass_the_verifier() {
    let (report, ok) = verify::verify(Path::new(COMMITTED), Path::new(STRICT), 4).unwrap();
    assert!(ok, "{report}");
}
