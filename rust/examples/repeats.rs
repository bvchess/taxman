//! The games whose search repeats an earlier game's (tactic `same_search`, DESIGN.md "Same search"), found from the
//! setups alone, with no search: for each board size of a range that is neither prime nor trivial, the search
//! problem of its strict setup (`revise_freedoms` off, so frame 0 as built), and the latest earlier board size of
//! the range with the same problem.  A strict chain over the range concludes exactly these games without a search.
//!
//! ```text
//! cargo run --release --example repeats -- FROM TO [--known FILE]
//! ```
//!
//! Prints a line per repeat (n, the game it repeats, how far back that is) and a summary: how many, how far back
//! the farthest reaches, and the setups' time.  With `--known FILE` (a results file, such as `results/strict.jsonl`)
//! each line also has the search seconds the file records for the repeat (`seconds.search_all_runs`, over all its
//! invocations), and the summary their sum, by range: what a rerun of the file's chain would not spend.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::time::Instant;
use taxman::board::Board;
use taxman::json::Json;
use taxman::search::{Problem, Revise};

/// The search seconds of each completed game of a results file, by board size.
fn search_seconds(path: &str) -> Result<BTreeMap<u32, f64>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let mut seconds = BTreeMap::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let json = Json::parse(line).map_err(|e| format!("{path}: {e}"))?;
        if json.get("completed").and_then(Json::as_bool) != Some(true) {
            continue;
        }
        let n = json.get("n").and_then(Json::as_u32).ok_or("a line without n")?;
        let s = json.get("seconds").and_then(|s| s.get("search_all_runs").or_else(|| s.get("search")));
        seconds.insert(n, s.and_then(Json::as_f64).unwrap_or(0.0));
    }
    Ok(seconds)
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = || "usage: repeats FROM TO [--known FILE]".to_owned();
    let (from, to) = match &args[..2.min(args.len())] {
        [from, to] => (from.parse::<u32>().map_err(|_| usage())?, to.parse::<u32>().map_err(|_| usage())?),
        _ => return Err(usage()),
    };
    let known = match &args[2..] {
        [] => None,
        [flag, path] if flag == "--known" => Some(search_seconds(path)?),
        _ => return Err(usage()),
    };

    let start = Instant::now();
    // The problems met so far, by digest, with the latest board size that posed each.
    let mut latest: HashMap<u64, Vec<(Problem, u32)>> = HashMap::new();
    let mut repeats: Vec<(u32, u32)> = Vec::new();
    let mut setups = 0;
    for n in from..=to {
        let problem = taxman::with_width!(n, W => {
            let board = Board::<W>::for_n(n);
            if board.tables.is_prime(n) || board.is_trivial() {
                continue;
            }
            board.problem(Revise::Off)?
        });
        setups += 1;
        let same = latest.entry(problem.digest()).or_default();
        match same.iter_mut().find(|(earlier, _)| *earlier == problem) {
            Some((_, m)) => {
                repeats.push((n, *m));
                *m = n;
            }
            None => same.push((problem, n)),
        }
    }
    let elapsed = start.elapsed().as_secs_f64();

    let seconds = |n: u32| known.as_ref().and_then(|k| k.get(&n).copied());
    for &(n, m) in &repeats {
        let s = seconds(n).map_or_else(String::new, |s| format!("  {s:.3} s"));
        println!("{n} repeats {m} ({} back){s}", n - m);
    }
    let farthest = repeats.iter().map(|&(n, m)| n - m).max().unwrap_or(0);
    println!(
        "{} repeats in {from}..={to}; the farthest reaches {farthest} back; {setups} setups took {elapsed:.2} s",
        repeats.len()
    );
    if let Some(known) = &known {
        let total = |lo: u32, hi: u32| -> (f64, f64) {
            let all: f64 = known.range(lo..=hi).map(|(_, s)| s).sum();
            let saved: f64 =
                repeats.iter().filter(|&&(n, _)| (lo..=hi).contains(&n)).filter_map(|&(n, _)| seconds(n)).sum();
            (saved, all)
        };
        let mut bounds = vec![from];
        bounds.extend([1001, 1232].into_iter().filter(|&b| b > from && b <= to));
        bounds.push(to + 1);
        for pair in bounds.windows(2) {
            let (saved, all) = total(pair[0], pair[1] - 1);
            let share = if all > 0.0 { 100.0 * saved / all } else { 0.0 };
            println!(
                "{}..={}: the repeats searched {saved:.1} s of the {all:.1} s recorded ({share:.1}%)",
                pair[0],
                pair[1] - 1
            );
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(2);
    }
}
