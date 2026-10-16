//! Certificates of U: a recorded score proven optimal on the whole board, game by game, by a search of the extended
//! problem (`taxman certify`; heuristics/THEORY.md, "The extended problem", Theorem M⁺ and its corollary).
//!
//! A game of `n` whose proof uses the frame model (an exhausted search, a trivial board, a repeat) is optimal on the
//! whole board if some optimal game plays every source of three or more prime factors that setup keeps, U\*_{≥3}
//! (the statement U).  The prover's barred parity bounds (`rust/prover/src/bin/upper.rs`, one line per game) settle
//! most of U\*_{≥3} against the game's score V: a member is *certified* when no set that respects Theorem B and leaves
//! it out weighs more than V; the others are *open*, O.  The *extended problem* Π⁺(O)
//! ([`build_extended`](crate::frames::build_extended)) keeps the certified ones fixed and leaves the open ones
//! optional (one-sided candidates of the search), and by the corollary of Theorem M⁺, if no promotion set of Π⁺(O)
//! is worth more than V − `in_the_bag⁺`, then opt(n) = V.
//!
//! [`certify_game`] does that for one game: it takes V and the recorded game from the results files, checks that the
//! game plays U\* above n/2 and is the promotion set (K\*, L\*) = (O, its moves up to n/2) of Π⁺(O), worth V, and
//! searches Π⁺(O) from it, in extended mode ([`SearchTactics::extended`]), with the ceiling n + V(n − 1) −
//! `in_the_bag⁺`.  The verdict is [`Verdict::Certified`] when the search ends with nothing better (opt(n) = V), and
//! [`Verdict::Beaten`] when it finds a promotion set worth more: then F ∪ K ∪ L is playable (Theorem M⁺), and the game
//! that plays it, in the order of [`play_order`] (which shares no code with the frame model) and finished with the
//! largest legal moves left, is refereed by [`replay_finished`] and scores more than V: a counterexample to the score,
//! and to U if the score was proven on the frame model.  A search stopped by a limit or a signal keeps its checkpoint
//! and is [`Verdict::Incomplete`]; the same call resumes it.
//!
//! The certificate holds only if the open members were certified against a score of at most V (members certified
//! against more cover only the sets worth more than that), so a game whose `upper` line has a larger score is not
//! certified; a promotion set that beats V is a counterexample whatever the line says.  Research code: the solver
//! does not use it.

use crate::bits::{words_for, Bits, WIDTHS};
use crate::board::{promotion_cap, Board};
use crate::engine::EngineConfig;
use crate::frames::ExtendedFrames;
use crate::game::{legal_moves_left, replay, replay_finished, Optimal};
use crate::gf2::Gf2Bound;
use crate::json::Json;
use crate::model::Scratch;
use crate::numtheory::Tables;
use crate::playable::play_order;
use crate::results::{append_line, build_json, open_locked, sibling_path};
use crate::search::{self, SearchTactics};
use crate::solver::{digest_text, spill_dir, SolveOptions, MAX_N};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------------------------------------
// The barred bounds

/// What a certificate takes from a line of the prover's barred bounds (`upper`): the board, the score the open members
/// were found against, whether the game rests on the frame model, and the open members of U\*_{≥3}.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Upper {
    pub n: u32,
    /// The score V the bounds were compared with.
    pub score: u64,
    /// Whether the game needs a certificate: `n` is not prime and V is below n + V(n − 1), so it rests on the frame
    /// model, not on the game of n − 1.
    pub needs: bool,
    /// The members of U\*_{≥3} whose barred bound is above V, ascending (none on a line that does not need it).
    pub open: Vec<u32>,
}

impl Upper {
    /// The line `json` of an `upper` file: `n`, `score`, `needs` (0 or 1) and, where it needs a certificate, `open`, a
    /// list of `[u, Ω(u), margin]`.
    pub fn parse(json: &Json) -> Result<Upper, String> {
        let field = |name: &str| json.get(name).ok_or_else(|| format!("no \"{name}\""));
        let n = field("n")?.as_u32().ok_or("a bad \"n\"")?;
        let at = |e: &str| format!("n={n}: {e}");
        let score = field("score").map_err(|e| at(&e))?.as_u64().ok_or_else(|| at("a bad \"score\""))?;
        let needs = match field("needs").map_err(|e| at(&e))?.as_u64() {
            Some(0) => false,
            Some(1) => true,
            _ => return Err(at("a bad \"needs\"")),
        };
        let mut open = Vec::new();
        if needs {
            for member in field("open").map_err(|e| at(&e))?.as_arr().ok_or_else(|| at("a bad \"open\""))? {
                let u = member.as_arr().and_then(|m| m.first()).and_then(Json::as_u32);
                open.push(u.ok_or_else(|| at("a bad member of \"open\""))?);
            }
            open.sort_unstable();
        }
        Ok(Upper { n, score, needs, open })
    }
}

/// The lines of the `upper` files `paths`, by board size.  A board in two files must have the same line in both.
///
/// # Errors
///
/// If a file cannot be read, a line is not an `upper` line, or two lines of one board differ.
pub fn read_upper(paths: &[PathBuf]) -> Result<BTreeMap<u32, Upper>, String> {
    let mut lines = BTreeMap::new();
    for path in paths {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        for (i, line) in text.lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
            let at = |e: String| format!("{}: line {}: {e}", path.display(), i + 1);
            let json = Json::parse(line).map_err(|e| at(e.to_string()))?;
            let upper = Upper::parse(&json).map_err(at)?;
            if let Some(before) = lines.insert(upper.n, upper.clone()) {
                if before != upper {
                    return Err(at(format!("n={} differs from an earlier line of the files", upper.n)));
                }
            }
        }
    }
    Ok(lines)
}

// ---------------------------------------------------------------------------------------------------------
// One game

/// How a certificate ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The search ended and nothing beats the recorded score: it is optimal.
    Certified,
    /// The search found a game that scores more than the recorded score.
    Beaten,
    /// A limit or a signal stopped the search, which its checkpoint resumes.
    Incomplete,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Certified => "certified",
            Verdict::Beaten => "beaten",
            Verdict::Incomplete => "incomplete",
        }
    }

    /// The verdict a line names, if it names one.
    pub fn parse(name: &str) -> Option<Verdict> {
        [Verdict::Certified, Verdict::Beaten, Verdict::Incomplete].into_iter().find(|v| v.as_str() == name)
    }
}

/// What [`certify_game`] found, as `taxman certify` writes it: one JSON line a game.
#[derive(Clone, Debug)]
pub struct Certificate {
    pub n: u32,
    /// The recorded score, V.
    pub score: u64,
    /// The open members O, and how many sources greedy rejected beside them (R): the one-sided candidates are O ∪ R.
    pub open: Vec<u32>,
    pub rejected: u32,
    /// The size of Π⁺(O): frames, two-sided and one-sided candidates; and its `in_the_bag⁺` (ΣF).
    pub frames: usize,
    pub two_sided: u32,
    pub one_sided: u32,
    pub in_the_bag: u64,
    /// The search: its nodes over all its runs, this run's seconds, and the seconds of all its runs.
    pub nodes: u64,
    pub seconds: f64,
    pub seconds_all_runs: f64,
    pub threads: usize,
    /// The set width the search ran at, in words: the board's own, or the widest where the GF(2) bound needs it.
    pub width: usize,
    pub verdict: Verdict,
    /// The digest of Π⁺(O) as the search was given it ([`search::Problem::digest`]).
    pub digest: u64,
    /// With [`Verdict::Beaten`], the game found: its score and its moves.
    pub beaten_by: Option<(u64, Vec<u32>)>,
}

impl Certificate {
    pub fn to_json(&self) -> Json {
        let seconds = |s: f64| Json::from((s * 1e4).round() / 1e4);
        let mut pairs = vec![
            ("n", Json::from(self.n)),
            ("score", Json::from(self.score)),
            ("verdict", Json::from(self.verdict.as_str())),
            ("open", Json::array(self.open.iter().copied())),
            ("rejected", Json::from(self.rejected)),
            ("frames", Json::from(self.frames)),
            ("two_sided", Json::from(self.two_sided)),
            ("one_sided", Json::from(self.one_sided)),
            ("in_the_bag", Json::from(self.in_the_bag)),
            ("nodes", Json::from(self.nodes)),
            ("seconds", seconds(self.seconds)),
            ("seconds_all_runs", seconds(self.seconds_all_runs)),
            ("threads", Json::from(self.threads)),
            ("width", Json::from(self.width)),
            ("digest", Json::from(digest_text(self.digest))),
        ];
        if let Some((score, moves)) = &self.beaten_by {
            pairs.push((
                "beaten_by",
                Json::object([("score", Json::from(*score)), ("moves", Json::array(moves.iter().copied()))]),
            ));
        }
        pairs.push(("build", build_json()));
        Json::object(pairs)
    }

    /// The summary line of the game, for stdout.
    pub fn summary_line(&self) -> String {
        let beaten = self.beaten_by.as_ref().map_or_else(String::new, |(score, _)| format!(" by a game of {score}"));
        format!(
            "n={:<5} V={:<9} |O|={:<3} |R|={:<2} frames {} candidates {}+{}  nodes={:<12} {:9.3} s  {}{beaten}  \
             problem {}",
            self.n,
            self.score,
            self.open.len(),
            self.rejected,
            self.frames,
            self.two_sided,
            self.one_sided,
            self.nodes,
            self.seconds,
            self.verdict.as_str(),
            digest_text(self.digest)
        )
    }
}

/// Certifies the recorded game of `n` (see the module docs): `known` gives the scores and games of `n` and `n − 1`
/// (the results files, over the table of optimal games), `upper` the open members; the search runs with `opts`'
/// threads, checkpoint (resumed if it exists), limits and progress, in extended mode whatever `opts`' tactics say.
///
/// # Errors
///
/// If the game is not one a certificate can be made for (no recorded game, a game that does not replay to its score,
/// does not play U\* above n/2, or is not a promotion set of Π⁺(O)), if Π⁺(O) cannot be set up (an open member that
/// is no kept source of three or more prime factors, more frames than a frame set holds), if the search fails, or if
/// the search found nothing better but `upper`'s score is above the recorded one (no certificate then).
pub fn certify_game(n: u32, known: &Optimal, upper: &Upper, opts: &SolveOptions) -> Result<Certificate, String> {
    let beyond = || format!("n={n} is beyond the largest supported board, n={MAX_N}");
    // The set width: the board's own, unless the GF(2) bound of Π⁺(O) needs more block words than its engines have (an
    // open source adds one to its frame's rank); then the widest, whose engines have the most.  The choice depends on
    // nothing but the board and O, so a resumed search finds the width its checkpoint was made at.
    let words = crate::with_width!(n, W => {
        let open: Bits<W> = upper.open.iter().copied().collect();
        Board::<W>::for_n(n).extended_words(&open)
    }, else Err(beyond()))
    .map_err(|e| format!("n={n}: {e}"))?;
    let fits = crate::with_width!(n, W => words <= Gf2Bound::<W>::MOST_WORDS, else false);
    let width = if fits { words_for(n) } else { WIDTHS[WIDTHS.len() - 1] };
    crate::with_words!(width, W => certify_at::<W>(n, known, upper, opts), else Err(beyond()))
}

fn certify_at<const W: usize>(
    n: u32,
    known: &Optimal,
    upper: &Upper,
    opts: &SolveOptions,
) -> Result<Certificate, String> {
    let at = |e: String| format!("n={n}: {e}");
    if upper.n != n {
        return Err(at(format!("the barred bounds given are those of n={}", upper.n)));
    }
    let (Some(score), Some(moves)) = (known.score(n), known.moves(n)) else {
        return Err(at("the results have no game of n".into()));
    };
    let replayed = replay(n, moves).map_err(|e| at(format!("the recorded game is illegal: {e}")))?;
    if replayed != score {
        return Err(at(format!("the recorded game scores {replayed}, not the {score} recorded")));
    }
    let board = Board::<W>::for_n(n);
    let kept: Bits<W> = board.plain.iter().copied().filter(|&x| 2 * x > n).collect();
    let recorded: Bits<W> = moves.iter().copied().collect();
    let above: Bits<W> = recorded.iter().filter(|&x| 2 * x > n).collect();
    if above != kept {
        let (more, fewer) = ((above - kept).to_vec(), (kept - above).to_vec());
        return Err(at(format!(
            "the recorded game does not play U*, the sources setup keeps, above n/2: it plays {more:?} besides, and \
             not {fewer:?}"
        )));
    }

    let open: Bits<W> = upper.open.iter().copied().collect();
    let checkpoint = opts.checkpoint_file(n);
    let resuming = checkpoint.as_ref().is_some_and(|path| path.exists());
    let (extended, prepared) = match &checkpoint {
        Some(path) if resuming => board.prepare_extended_for_resume(&open, path),
        _ => board.prepare_extended_with(&open, SearchTactics::extended()),
    }
    .map_err(at)?;

    // The recorded game as a promotion set of Π⁺(O): the open sources, and its moves up to n/2.
    let start = open | (recorded - kept);
    let promotable = start.is_subset(&prepared.candidates)
        && prepared.model.can_promote_all_using(&mut Scratch::new(), &prepared.root, &start);
    if !promotable {
        return Err(at("the recorded game is not a promotion set of Π⁺(O) (Theorem M⁺ says a playable one is)".into()));
    }
    if extended.in_the_bag + start.sum() != score {
        let worth = extended.in_the_bag + start.sum();
        return Err(at(format!("the recorded game is worth {worth} in Π⁺(O), not its score {score}")));
    }
    let cap = known.score(n - 1).and_then(|prev| promotion_cap(n, extended.in_the_bag, prev));

    let (spill, own) = spill_dir(opts, n);
    let cfg = EngineConfig {
        threads: opts.threads,
        queue_mem_cap: opts.queue_mem_cap,
        spill_dir: Some(spill.clone()),
        checkpoint: checkpoint.clone().map(|path| (path, opts.checkpoint_interval)),
        // Deleted by the caller once the line is safe.
        keep_completed_checkpoint: true,
        progress: opts.progress,
        time_limit: opts.time_limit,
        node_limit: opts.node_limit,
        label: format!("n={n}"),
        cores: None,
        control: None,
    };
    let found = search::search(&prepared, &cfg, Some(&start), cap, resuming);
    if own {
        let _ = fs::remove_dir(&spill);
    }
    let found = found.map_err(|e| at(format!("the search failed: {e}")))?;

    let verdict = if !found.completed {
        Verdict::Incomplete
    } else if found.sum > start.sum() {
        Verdict::Beaten
    } else {
        if upper.score > score {
            return Err(at(format!(
                "nothing beats {score}, but the open members were found against {}, more: not a certificate",
                upper.score
            )));
        }
        Verdict::Certified
    };
    let beaten_by = if verdict == Verdict::Beaten {
        Some(beating_game(&board.tables, &extended, &found.promotions, score).map_err(at)?)
    } else {
        None
    };
    let one_sided = extended.one_sided.len() as u32;
    Ok(Certificate {
        n,
        score,
        open: upper.open.clone(),
        rejected: extended.rejected.len(),
        frames: prepared.root.len(),
        two_sided: extended.candidates.len(),
        one_sided,
        in_the_bag: extended.in_the_bag,
        nodes: found.stats.nodes,
        seconds: found.time.as_secs_f64(),
        seconds_all_runs: found.time_all_runs.as_secs_f64(),
        threads: opts.threads,
        width: W,
        verdict,
        digest: prepared.problem().digest(),
        beaten_by,
    })
}

/// The game of a promotion set `promotions` of Π⁺(O) worth more than `score`: the set S = F ∪ K ∪ L played in the
/// order of [`play_order`] (the peel of THEORY.md, Theorem 2, which uses nothing of the frame model), then the largest
/// legal move left, again and again, until none is left, so that the game is over; refereed by [`replay_finished`].
fn beating_game<const W: usize>(
    t: &Tables,
    extended: &ExtendedFrames<W>,
    promotions: &Bits<W>,
    score: u64,
) -> Result<(u64, Vec<u32>), String> {
    let set = (extended.fixed | *promotions).to_vec();
    let mut moves = play_order(t, &set)
        .ok_or_else(|| format!("the set of the promotion set {:?} is not playable", promotions.to_vec()))?;
    loop {
        let left = legal_moves_left(t.n, &moves)?;
        match left.last() {
            Some(&m) => moves.push(m),
            None => break,
        }
    }
    let found = replay_finished(t.n, &moves).map_err(|e| format!("the game found is refused: {e}"))?;
    if found <= score {
        return Err(format!("the game found scores {found}, not more than {score}"));
    }
    Ok((found, moves))
}

// ---------------------------------------------------------------------------------------------------------
// The file of certificates

/// The file `taxman certify --out` appends to, one [`Certificate`] a line, flushed and synced.  Opening it locks it, as
/// a results file is locked (`results::ResultsFile`), and reads which games it has a final verdict for (`certified` or
/// `beaten`), which a run skips; a game with only `incomplete` lines is resumed from its checkpoint.  A torn last line
/// (a run killed while writing it) is cut off.
pub struct CertificateFile {
    path: PathBuf,
    file: File,
    verdicts: BTreeMap<u32, Verdict>,
}

impl CertificateFile {
    /// Opens (creating if need be) and locks the file at `path`, and reads its verdicts.
    ///
    /// # Errors
    ///
    /// If it cannot be opened, another process holds it, or a complete line is not a certificate.
    pub fn open(path: &Path) -> Result<CertificateFile, String> {
        let (file, text) = open_locked(path)?;
        let at = |i: usize, e: &str| format!("{}: line {}: {e}", path.display(), i + 1);
        let complete = text.rfind('\n').map_or(0, |end| end + 1);
        let mut verdicts = BTreeMap::new();
        for (i, line) in text[..complete].lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
            let json = Json::parse(line).map_err(|e| at(i, &e.to_string()))?;
            let n = json.get("n").and_then(Json::as_u32).ok_or_else(|| at(i, "no board size"))?;
            let verdict = json.get("verdict").and_then(Json::as_str).and_then(Verdict::parse);
            match verdict.ok_or_else(|| at(i, "no verdict"))? {
                Verdict::Incomplete => {}
                last => {
                    verdicts.insert(n, last);
                }
            }
        }
        if complete < text.len() {
            file.set_len(complete as u64).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        Ok(CertificateFile { path: path.to_owned(), file, verdicts })
    }

    /// The final verdict the file has for `n`, if any.
    pub fn verdict(&self, n: u32) -> Option<Verdict> {
        self.verdicts.get(&n).copied()
    }

    /// Appends `certificate` and makes sure it reaches the disk.
    pub fn append(&mut self, certificate: &Certificate) -> Result<(), String> {
        append_line(&mut self.file, &certificate.to_json()).map_err(|e| format!("{}: {e}", self.path.display()))?;
        if certificate.verdict != Verdict::Incomplete {
            self.verdicts.insert(certificate.n, certificate.verdict);
        }
        Ok(())
    }

    /// Where the certificates of the games that beat their recorded score go as well: `<out>.counterexample.jsonl`
    /// (with a `.jsonl` ending of the file dropped first, as for a results file's rejected games).
    pub fn counterexample_path(&self) -> PathBuf {
        sibling_path(&self.path, "counterexample")
    }

    /// Appends `certificate` to the file of [`counterexample_path`](Self::counterexample_path), synced.
    pub fn append_counterexample(&self, certificate: &Certificate) -> Result<PathBuf, String> {
        let path = self.counterexample_path();
        let line = certificate.to_json();
        let written =
            OpenOptions::new().create(true).append(true).open(&path).and_then(|mut f| append_line(&mut f, &line));
        written.map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(path)
    }
}

/// Dropping releases the lock at once, as a results file's does.
impl Drop for CertificateFile {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}
