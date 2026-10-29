//! A root fit on disk: what `dw::bound` found at a game's root, as one JSON object, for later experiments to start
//! from (`wall/fields.rs`: `choice-prepare` writes `nNNNN.fit.json` beside a game's prepared file, and reads the fits of
//! `testdata/fits` (`fits/n{N}.json`) when it is given them).
//!
//! ```text
//! n, opt, gf2          the game, its optimum promotion sum P*, the GF(2) bound at its root
//! bound                ⌊L⌋ of the fit below at the root (the lowest L the run evaluated): reproducible
//! l_best               the lowest L of the run's final family of signings (at least `bound`)
//! mode                 "certified" or "search" (`dw::FitMode`)
//! signings             per extra (independence in a frame under a GF(3) signing; see `dw`), its signing as
//!                      `gf3::Signing::to_json` writes it
//! lambda               per signing, its positive multipliers as `dw::multipliers_to_json` writes them
//! support, columns     the final master's columns, each a list of candidates (sets independent in every frame over
//!                      GF(2); at most `dw::SAVED_COLUMNS`), the first `support` of them its support (null in the
//!                      pilot study E3's two fits, written before the format had it)
//! cuts                 the final master's rank inequalities [signing, [candidates], rank]; the signing is an index
//!                      into `signings` (cuts of an extra the fit does not have are left out), the rank that in the
//!                      final family: a reader takes the ranks again in the signings it loads
//! ```
//!
//! A loaded signing is frozen (`mergeable` false, no cores).  `SavedFit::bound` rebuilds the frame vectors at a root
//! and evaluates `L` exactly as the run did, so a round trip checks itself.

use super::dw::{multipliers_from_json, multipliers_to_json, Ctx, Fit, Outcome, Stats};
use super::gf3::Signing;
use std::path::Path;
use taxman::json::Json;

/// A fit as saved: the game, the bound it gave, the fit and the final master's columns and inequalities.
pub struct SavedFit {
    pub n: u32,
    pub opt: u64,
    pub gf2: i64,
    pub bound: i64,
    pub l_best: f64,
    pub mode: String,
    pub fit: Fit,
    pub columns: Vec<Vec<u32>>,
    /// How many of `columns`, the first, are the master's support (`None` where the file does not say).
    pub support: Option<usize>,
    pub cuts: Vec<(usize, Vec<u32>, usize)>,
}

impl SavedFit {
    /// The fit of `outcome` at the root of `n` (optimum `opt`, GF(2) bound `gf2`), found in `mode`.
    pub fn of(n: u32, opt: u64, gf2: i64, mode: &str, outcome: &Outcome) -> SavedFit {
        let have = outcome.fit.signings.len();
        SavedFit {
            n,
            opt,
            gf2,
            bound: outcome.bound,
            l_best: outcome.l_best,
            mode: mode.to_string(),
            fit: outcome.fit.clone(),
            columns: outcome.master_columns.clone(),
            support: Some(outcome.support.len().min(outcome.master_columns.len())),
            cuts: outcome.master_cuts.iter().filter(|(k, _, _)| *k < have).cloned().collect(),
        }
    }

    /// `⌊L⌋` of the saved fit at the state of `ctx` (the root it was fitted at): `bound`, if the file is intact.
    pub fn bound<const W: usize>(&self, ctx: &Ctx<'_, W>) -> i64 {
        self.fit.bound(ctx, &mut Stats::default())
    }

    /// The fit as the file's object (see the module docs).
    pub fn to_json(&self) -> Json {
        let numbers = |v: &[u32]| Json::array(v.iter().copied());
        let cuts =
            self.cuts.iter().map(|(k, set, rank)| Json::Arr(vec![Json::from(*k), numbers(set), Json::from(*rank)]));
        Json::object([
            ("n", Json::from(self.n)),
            ("opt", Json::from(self.opt)),
            ("gf2", Json::from(self.gf2)),
            ("bound", Json::from(self.bound)),
            ("l_best", Json::Num(self.l_best)),
            ("mode", Json::Str(self.mode.clone())),
            ("signings", Json::Arr(self.fit.signings.iter().map(Signing::to_json).collect())),
            ("lambda", multipliers_to_json(&self.fit.lambda)),
            ("support", self.support.map_or(Json::Null, Json::from)),
            ("columns", Json::Arr(self.columns.iter().map(|c| numbers(c)).collect())),
            ("cuts", Json::Arr(cuts.collect())),
        ])
    }

    /// The fit of the file's object ([`to_json`](Self::to_json)'s).
    pub fn from_json(json: &Json) -> Result<SavedFit, String> {
        let field = |name: &str| json.get(name).ok_or_else(|| format!("missing \"{name}\""));
        let number = |name: &str| field(name)?.as_i64().ok_or_else(|| format!("\"{name}\" is not an integer"));
        let list = |name: &str| field(name)?.as_arr().ok_or_else(|| format!("\"{name}\" is not an array"));
        let signings = list("signings")?.iter().map(Signing::from_json).collect::<Result<Vec<_>, _>>()?;
        let lambda = multipliers_from_json(field("lambda")?, signings.len())?;
        let support = match json.get("support") {
            None | Some(Json::Null) => None,
            Some(s) => Some(s.as_u64().ok_or("\"support\" is not a count")? as usize),
        };
        let columns = list("columns")?
            .iter()
            .map(|c| c.as_u32_vec().ok_or("a column is not a list of candidates"))
            .collect::<Result<Vec<_>, _>>()?;
        let cuts = list("cuts")?
            .iter()
            .map(|c| {
                match (
                    c.at(0).and_then(Json::as_u64),
                    c.at(1).and_then(Json::as_u32_vec),
                    c.at(2).and_then(Json::as_u64),
                ) {
                    (Some(k), Some(set), Some(rank)) => Ok((k as usize, set, rank as usize)),
                    _ => Err("a cut is not [signing, [candidates], rank]".to_string()),
                }
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(SavedFit {
            n: number("n")? as u32,
            opt: number("opt")? as u64,
            gf2: number("gf2")?,
            bound: number("bound")?,
            l_best: field("l_best")?.as_f64().ok_or("\"l_best\" is not a number")?,
            mode: field("mode")?.as_str().ok_or("\"mode\" is not a string")?.to_string(),
            fit: Fit { signings, lambda },
            columns,
            support,
            cuts,
        })
    }

    /// Writes the fit to `path` whole (to a temporary file, then renamed), creating the directory.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let err = |e: std::io::Error| format!("{}: {e}", path.display());
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(err)?;
        }
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, self.to_json().to_string() + "\n").map_err(err)?;
        std::fs::rename(&temporary, path).map_err(err)
    }

    /// The fit of the file `path`.
    pub fn load(path: &Path) -> Result<SavedFit, String> {
        SavedFit::from_json(&Json::from_file(path)?).map_err(|e| format!("{}: {e}", path.display()))
    }
}
