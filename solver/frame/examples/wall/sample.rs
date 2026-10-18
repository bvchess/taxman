//! The search nodes that `choice-prepare` samples for a game, from three sources: the fixture `--fixtures DIR` has for
//! the game (`nNNNN.nodes`, as the probe's, made by `perf make-nodes`: units that the proof seeded with the optimum
//! handed its searcher), the units of the game in the tuning suite `--suite FILE`
//! (`solver/notebook/the-tuning-campaign-at-the-end-of-the-chain-2026-10/suite.jsonl`), and `--paths K` nodes on the
//! way to the optimum (default 50): the optimum's promotions less a random 30 to 70% of them, its other candidates
//! narrowed, whose best completion is at least the optimum less what they promote, which is a check of the bound's
//! validity.

use super::case::Case;
use crate::fixtures::{from_hex, read_nodes};
use crate::rng::Rng;
use std::path::PathBuf;
use taxman::bits::Bits;
use taxman::json::Json;
use taxman::model::Scratch;
use taxman::search::TaxUnit;

/// Where the nodes of a game come from.
#[derive(Clone, Copy)]
pub(super) enum Source {
    Fixture,
    Suite,
    Path,
}

impl Source {
    pub(super) fn name(self) -> &'static str {
        match self {
            Source::Fixture => "fixture",
            Source::Suite => "suite",
            Source::Path => "path",
        }
    }
}

/// A node to measure: the unit, and for a suite unit the nodes of its subtree.
pub(super) struct Sampled<const W: usize> {
    pub(super) unit: TaxUnit<W>,
    pub(super) subtree: Option<u64>,
}

/// The nodes of game `n` from the fixture in `dir` (none if it has no file for the game), checking that its
/// incumbent is worth `opt`.
pub(super) fn fixture_nodes<const W: usize>(
    dir: &Option<PathBuf>,
    n: u32,
    opt: u64,
) -> Result<Vec<Sampled<W>>, String> {
    let Some(path) = dir.as_ref().map(|dir| dir.join(format!("n{n:04}.nodes"))).filter(|path| path.exists()) else {
        return Ok(Vec::new());
    };
    let (m, incumbent, units) = read_nodes::<W>(&path)?;
    if m != n || incumbent.sum() != opt {
        return Err(format!(
            "{}: game {m} from an incumbent worth {}, not {n} from {opt}",
            path.display(),
            incumbent.sum()
        ));
    }
    Ok(units.into_iter().map(|unit| Sampled { unit, subtree: None }).collect())
}

/// The units of game `n` in the tuning suite `suite` (none without one).
pub(super) fn suite_nodes<const W: usize>(suite: &Option<PathBuf>, n: u32) -> Result<Vec<Sampled<W>>, String> {
    let Some(path) = suite else { return Ok(Vec::new()) };
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = Vec::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let json = Json::parse(line).map_err(|e| format!("{}: {e:?}", path.display()))?;
        let field = |key: &str| json.get(key).and_then(Json::as_str).unwrap_or_default();
        if field("kind") != "unit" || json.get("game").and_then(Json::as_u32) != Some(n) {
            continue;
        }
        let unit = TaxUnit { promoted: from_hex(field("promoted"))?, cands: from_hex(field("cands"))? };
        let default = json.get("default");
        let completed = default.and_then(|d| d.get("completed")).and_then(Json::as_bool) == Some(true);
        let subtree = default.and_then(|d| d.get("nodes")).and_then(Json::as_u64).filter(|_| completed);
        out.push(Sampled { unit, subtree });
    }
    Ok(out)
}

/// `count` nodes on the way to the optimum `known` of `case`: `known` less a random 30 to 70% of its members promoted,
/// the root's other candidates narrowed in the state they reach (which keeps the rest of `known`, promotable there).
pub(super) fn path_nodes<const W: usize>(case: &Case<'_, W>, known: &Bits<W>, count: usize) -> Vec<Sampled<W>> {
    let p = case.p;
    let mut rng = Rng(0x9A7E_0000 + u64::from(case.n()));
    let mut scratch = Scratch::new();
    let members = known.to_vec();
    (0..count)
        .filter_map(|_| {
            let keep = 0.3 + 0.4 * rng.unit();
            let promoted: Bits<W> = members.iter().copied().filter(|_| rng.unit() < keep).collect();
            let state = p.model.promote_all_using(&mut scratch, &p.root, &promoted)?;
            let cands = p.model.narrow(&state, &(p.candidates - promoted));
            let unit = TaxUnit { promoted, cands };
            (*known - promoted).is_subset(&cands).then_some(Sampled { unit, subtree: None })
        })
        .collect()
}
