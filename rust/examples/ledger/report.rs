//! The report of a run, and its check against known optima.

use crate::book::{Entry, Rule};
use crate::schedule::{events_path, read_ledger};
use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use taxman::game::{replay_finished, Optimal};
use taxman::json::Json;

/// What the check found.
#[derive(Debug, Default)]
pub struct Checked {
    /// Boards with a known optimum.
    pub known: usize,
    /// Proven boards whose value is the known optimum.
    pub proven_right: usize,
    /// Everything wrong: a game the referee refuses or that scores other than its bound, a bound on the wrong side of
    /// the known optimum, a proven value that is not it.
    pub violations: Vec<String>,
}

/// Checks every board of `entries`: its game replays to its lower bound, and with a known optimum (`known`), L <= opt
/// <= U, with equality for a proven board.
pub fn check(entries: &[Entry], known: &Optimal) -> Checked {
    let mut c = Checked::default();
    for e in entries {
        let n = e.n;
        if let Some(l) = &e.lower {
            match replay_finished(n, &l.moves) {
                Ok(score) if score == l.value => {}
                Ok(score) => {
                    c.violations.push(format!("n={n}: the game of the lower bound {} scores {score}", l.value))
                }
                Err(err) => c.violations.push(format!("n={n}: the game of the lower bound: {err}")),
            }
        }
        let Some(opt) = known.score(n) else { continue };
        c.known += 1;
        if let Some(l) = e.lower.as_ref().filter(|l| l.value > opt) {
            c.violations.push(format!("n={n}: L = {} ({}) is above the optimum {opt}", l.value, l.rule.name()));
        }
        if let Some(u) = e.upper.filter(|u| u.value < opt) {
            c.violations.push(format!("n={n}: U = {} ({}) is below the optimum {opt}", u.value, u.rule.name()));
        }
        if e.proven() {
            if e.upper.map(|u| u.value) == Some(opt) {
                c.proven_right += 1;
            } else {
                c.violations.push(format!("n={n}: proven at {:?}, but the optimum is {opt}", e.upper.map(|u| u.value)));
            }
        }
    }
    c
}

/// The quantiles 0, 10, 25, 50, 75, 90 and 100 % of `values` (sorted here).
fn quantiles(values: &mut [i64]) -> String {
    if values.is_empty() {
        return "none".into();
    }
    values.sort_unstable();
    let at = |p: usize| values[(values.len() - 1) * p / 100];
    format!(
        "min {}, p10 {}, p25 {}, median {}, p75 {}, p90 {}, max {}",
        at(0),
        at(10),
        at(25),
        at(50),
        at(75),
        at(90),
        at(100)
    )
}

/// The report of the run in `dir` as text, and whether the check against the known optima (optimal.json, with the
/// completed games of the results files `known`) found nothing wrong.
pub fn report(dir: &Path, known: &[PathBuf]) -> Result<(String, bool), String> {
    let entries = read_ledger(dir)?;
    let events = fs::read_to_string(events_path(dir)).unwrap_or_default();
    let events: Vec<Json> = events.lines().filter_map(|line| Json::parse(line).ok()).collect();
    // The last event's CPU, which is rounded to the millisecond, or a board's time of proof if that is later.
    let cpu = events.iter().rev().find_map(|e| e.get("cpu")?.as_f64()).unwrap_or(0.0);
    let cpu = entries.iter().filter_map(|e| e.proven_cpu).fold(cpu, f64::max);
    let mut out = String::new();
    let w = &mut out;
    let proven: Vec<&Entry> = entries.iter().filter(|e| e.proven()).collect();
    let (first, last) = (entries.first().map_or(0, |e| e.n), entries.last().map_or(0, |e| e.n));
    let _ = writeln!(w, "boards {first}..{last}: {} in the ledger, {} proven", entries.len(), proven.len());
    let searches = events.iter().filter(|e| e.get("action").and_then(Json::as_str) == Some("search")).count();
    let search_s: f64 = entries.iter().map(|e| e.search_s).sum();
    let _ =
        writeln!(w, "CPU {cpu:.0} s ({:.2} core-hours); {searches} searches, {search_s:.0} s of them", cpu / 3600.0);
    // The climbs of each way (a run before the ways had only the climbs up, and no `way`).
    for way in ["up", "down"] {
        let text = |e: &Json, field: &str| e.get(field).and_then(Json::as_str).map(str::to_owned);
        let climbs: Vec<&Json> = events
            .iter()
            .filter(|e| text(e, "job").as_deref() == Some("climb") && text(e, "way").as_deref().unwrap_or("up") == way)
            .collect();
        let by_outcome = |name: &str| climbs.iter().filter(|e| text(e, "outcome").as_deref() == Some(name)).count();
        let climb_s = climbs.iter().filter_map(|e| e.get("seconds")?.as_f64()).sum::<f64>().round() as u64;
        let _ = writeln!(
            w,
            "climbs {way}: {} among the searches, {climb_s} s of them: {} proved their board, {} parked, {} moot (the \
             board was proven meanwhile)",
            climbs.len(),
            by_outcome("proved"),
            by_outcome("parked"),
            by_outcome("moot"),
        );
    }

    // The finder: its runs (a map or a search with an `lns_found`), those that found a better game than they began
    // from, and the boards whose lower bound at the end is its game.
    let jobs = || events.iter().filter(|e| matches!(e.get("action").and_then(Json::as_str), Some("init" | "search")));
    let runs: Vec<(u64, u64)> =
        jobs().filter_map(|e| Some((e.get("lns_from")?.as_u64()?, e.get("lns_found")?.as_u64()?))).collect();
    let better: Vec<u64> =
        runs.iter().filter(|&&(from, found)| found > from).map(|&(from, found)| found - from).collect();
    let lns_s = jobs().filter_map(|e| e.get("lns_s")?.as_f64()).sum::<f64>();
    let ends_with_it: Vec<&Entry> =
        entries.iter().filter(|e| e.lower.as_ref().is_some_and(|l| l.rule == Rule::Lns)).collect();
    let _ = writeln!(
        w,
        "finder: {} runs, {lns_s:.0} s of them; {} found a better game than they began from (by {} in all); {} boards \
         end with its game as the lower bound ({} of them proven)",
        runs.len(),
        better.len(),
        better.iter().sum::<u64>(),
        ends_with_it.len(),
        ends_with_it.iter().filter(|e| e.proven()).count(),
    );

    // How the proven boards were proven: the rules of their two bounds.
    let mut pairs: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for e in &proven {
        let (l, u) = (e.lower.as_ref().expect("proven"), e.upper.expect("proven"));
        *pairs.entry((l.rule.name(), u.rule.name())).or_default() += 1;
    }
    let _ = writeln!(w, "proven by (lower rule, upper rule):");
    for ((l, u), count) in &pairs {
        let _ = writeln!(w, "  {l:<10} {u:<10} {count}");
    }
    let by_upper = |rules: &[Rule]| proven.iter().filter(|e| rules.contains(&e.upper.expect("proven").rule)).count();
    let by_relations = proven.iter().filter(|e| e.upper.expect("proven").rule.is_relation()).count();
    let _ = writeln!(
        w,
        "by the upper bound: search {} (exhausted {}, dual {}), relations {} (ceiling {}, prime {}, repeat {}, \
         monotone {}), other {}",
        by_upper(&[Rule::Exhausted, Rule::Dual]),
        by_upper(&[Rule::Exhausted]),
        by_upper(&[Rule::Dual]),
        by_relations,
        by_upper(&[Rule::Ceiling]),
        by_upper(&[Rule::Prime]),
        by_upper(&[Rule::Repeat]),
        by_upper(&[Rule::Monotone]),
        by_upper(&[Rule::Trivial, Rule::Given, Rule::Root]),
    );

    // The curve: boards proven against CPU.
    let mut times: Vec<f64> = proven.iter().filter_map(|e| e.proven_cpu).collect();
    times.sort_by(f64::total_cmp);
    let _ = writeln!(w, "proven against CPU seconds:");
    for step in 1..=10 {
        let at = cpu * f64::from(step) / 10.0;
        let count = times.iter().filter(|&&t| t <= at).count();
        let _ = writeln!(w, "  {at:>9.0} s  {count}");
    }

    let mut gaps: Vec<i64> = entries.iter().filter(|e| !e.proven()).filter_map(Entry::gap).collect();
    let _ = writeln!(w, "U - L of the {} unproven boards: {}", gaps.len(), quantiles(&mut gaps));
    let _ = writeln!(w, "U - L summed: {}", entries.iter().filter_map(Entry::gap).sum::<i64>());

    let optimal = Optimal::built_in().with_results_files(known)?;
    let c = check(&entries, &optimal);
    let _ = writeln!(
        w,
        "check: {} boards with a known optimum, {} proven at it; {} violations",
        c.known,
        c.proven_right,
        c.violations.len()
    );
    for v in c.violations.iter().take(20) {
        let _ = writeln!(w, "  {v}");
    }
    Ok((out, c.violations.is_empty()))
}
