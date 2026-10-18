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
//! certified; a promotion set that beats V is a counterexample whatever the line says.
//!
//! The extended problem has two *formulations* ([`Board::extended`]): with the claims of `unzip` that Lemma M5⁺ allows
//! to keep (`claims`, the default), and without any, as every certificate made before that lemma was.  A run searches
//! in one of them (`taxman certify --no-claims` in the second), and a certificate's line records which
//! ([`Certificate::claims`]; a line with no such field is read as without).  The two have the same promotion sets and
//! so the same heaviest promotion set (heuristics/THEORY.md, the corollary of Lemma M5⁺), so a certificate made in one
//! bounds the heaviest promotion set of the problem in the other.
//!
//! Consecutive games often pose the same Π⁺ (the same [`search::Problem`], which has `n` nowhere in it), as they often
//! pose the same search problem in the solver (`solver::SAME_SEARCH_WINDOW`).  If an earlier game m of the last
//! `SAME_SEARCH_WINDOW` board sizes is certified, and Π⁺(n) equals Π⁺(m) whole, then the heaviest promotion set of
//! Π⁺(n) is worth what m's certificate proved the heaviest worth, V(m) − `in_the_bag⁺`(m), and a recorded game of n
//! that is worth the same has nothing above it: [`certify_game_with`] concludes `certified` with no search, recording
//! the game m it rests on ([`SameProblem`]).  The problems are compared whole in the run's formulation, whatever
//! formulation m's certificate was made in (a line of the other formulation is set up again in the run's): m's
//! certificate bounds the heaviest promotion set, which does not depend on the formulation, so the rule proves what a
//! search of Π⁺(n) would, and n's own barred bounds then give opt(n) = V(n) by the corollary of Theorem M⁺.  A game
//! worth less than m's certificate says is searched (it is not optimal, and the search finds the better set); one
//! worth more contradicts the certificate and is refused.  Research code: the solver does not use it.
//!
//! The files `taxman certify` writes are checked again, line by line, by [`verify`] (`taxman verify-certificates`),
//! which sets each certificate's problem up again, applies the same checks of the recorded game and the same rule of
//! the same problem, and says how far the proofs of the recorded scores reach (the prover's, the certificates' and the
//! ceilings').

use crate::bits::{Bits, WIDTHS};
use crate::board::{promotion_cap, Board};
use crate::engine::EngineConfig;
use crate::frames::ExtendedFrames;
use crate::game::{legal_moves_left, replay, replay_finished, Optimal};
use crate::gf2::{Gf2Bound, MAX_WORDS};
use crate::json::Json;
use crate::model::Scratch;
use crate::numtheory::Tables;
use crate::playable::play_order;
use crate::results::{append_line, build_json, open_locked, sibling_path, Fields};
use crate::search::{self, Prepared, Problem, SearchResult, SearchTactics};
use crate::solver::{digest_text, spill_dir, SolveOptions, MAX_N, SAME_SEARCH_WINDOW};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

pub mod verify;

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

/// The earlier game whose certificate settles a game with no search (see the module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SameProblem {
    /// The earlier game m, whose certificate says the heaviest promotion set of the problem is worth `best`.
    pub n: u32,
    /// The digest of the problem, which m's line records and this game's problem has too.
    pub digest: u64,
    /// What m's certificate proved the heaviest promotion set of the problem is worth: V(m) − `in_the_bag⁺`(m).
    pub best: u64,
    /// What this game's recorded game is worth in the problem: V(n) − `in_the_bag⁺`(n), which equals `best`.
    pub worth: u64,
}

impl SameProblem {
    fn to_json(&self) -> Json {
        Json::object([
            ("n", Json::from(self.n)),
            ("digest", Json::from(digest_text(self.digest))),
            ("best", Json::from(self.best)),
            ("worth", Json::from(self.worth)),
        ])
    }

    fn from_json(json: &Json) -> Result<SameProblem, String> {
        let f = Fields(json);
        Ok(SameProblem { n: f.u32("n")?, digest: f.digest("digest")?, best: f.u64("best")?, worth: f.u64("worth")? })
    }
}

/// What [`certify_game`] found, as `taxman certify` writes it: one JSON line a game.
#[derive(Clone, Debug, PartialEq)]
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
    /// The formulation Π⁺(O) was set up in ([`Board::extended`]): true with the claims of `unzip` that Lemma M5⁺
    /// keeps, false with none, as every certificate made before it (whose lines have no `claims`, and read as false).
    pub claims: bool,
    pub verdict: Verdict,
    /// The digest of Π⁺(O) in that formulation, as the search was given it ([`search::Problem::digest`]).
    pub digest: u64,
    /// With [`Verdict::Certified`] by the rule of the module docs, no search (`nodes` and the seconds are 0), the
    /// earlier game it rests on.
    pub same_problem: Option<SameProblem>,
    /// With [`Verdict::Beaten`], the game found: its score and its moves.
    pub beaten_by: Option<(u64, Vec<u32>)>,
}

impl Certificate {
    /// The line `json` of a file of certificates, as [`to_json`](Self::to_json) writes it (but for the `build`, which is
    /// ignored).
    ///
    /// # Errors
    ///
    /// If a field is missing or has the wrong type, or the verdict is none of the three: the message names the field.
    pub fn from_json(json: &Json) -> Result<Certificate, String> {
        let n = json.get("n").and_then(Json::as_u32).ok_or("no board size")?;
        let verdict = json.get("verdict").and_then(Json::as_str).and_then(Verdict::parse).ok_or("no verdict")?;
        let f = Fields(json);
        // The fields in the order `to_json` writes them, so that the first one wrong is the one named.
        let mut certificate = Certificate {
            n,
            score: f.u64("score")?,
            open: f.u32s("open")?,
            rejected: f.u32("rejected")?,
            frames: f.usize("frames")?,
            two_sided: f.u32("two_sided")?,
            one_sided: f.u32("one_sided")?,
            in_the_bag: f.u64("in_the_bag")?,
            nodes: f.u64("nodes")?,
            seconds: f.f64("seconds")?,
            seconds_all_runs: f.f64("seconds_all_runs")?,
            threads: f.usize("threads")?,
            width: f.usize("width")?,
            claims: f.bool_or("claims", false)?,
            verdict,
            digest: f.digest("digest")?,
            same_problem: None,
            beaten_by: None,
        };
        if let Some(same) = json.get("same_problem") {
            certificate.same_problem = Some(SameProblem::from_json(same).map_err(|_| Fields::bad("same_problem"))?);
        }
        if let Some(beating) = json.get("beaten_by") {
            let (beating, bad) = (Fields(beating), || Fields::bad("beaten_by"));
            let (score, moves) = (beating.u64("score").map_err(|_| bad())?, beating.u32s("moves").map_err(|_| bad())?);
            certificate.beaten_by = Some((score, moves));
        }
        Ok(certificate)
    }

    /// What the recorded game is worth in Π⁺(O), V − `in_the_bag⁺` (which, for a certified line, is the most a
    /// promotion set of the problem is worth); `None` for a score below its own `in_the_bag⁺`, which no certificate
    /// has.
    pub fn worth(&self) -> Option<u64> {
        self.score.checked_sub(self.in_the_bag)
    }

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
            ("claims", Json::Bool(self.claims)),
            ("digest", Json::from(digest_text(self.digest))),
        ];
        if let Some(same) = &self.same_problem {
            pairs.push(("same_problem", same.to_json()));
        }
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
        let verdict = match &self.same_problem {
            Some(same) => format!("{} (same problem as {})", self.verdict.as_str(), same.n),
            None => self.verdict.as_str().to_owned(),
        };
        let formulation = if self.claims { "" } else { " (no claims)" };
        format!(
            "n={:<5} V={:<9} |O|={:<3} |R|={:<2} frames {} candidates {}+{}  nodes={:<12} {:9.3} s  {}{beaten}  \
             problem {}{formulation}",
            self.n,
            self.score,
            self.open.len(),
            self.rejected,
            self.frames,
            self.two_sided,
            self.one_sided,
            self.nodes,
            self.seconds,
            verdict,
            digest_text(self.digest)
        )
    }
}

/// Certifies the recorded game of `n` (see the module docs): `known` gives the scores and games of `n` and `n − 1`
/// (the results files, over the table of optimal games), `upper` the open members; the search runs with `opts`'
/// threads, checkpoint (resumed if it exists), limits and progress, in extended mode whatever `opts`' tactics say.
/// `claims` is the formulation of Π⁺(O) ([`Board::extended`]): with the claims of `unzip` that Lemma M5⁺ keeps, or
/// without any.
///
/// # Errors
///
/// If the game is not one a certificate can be made for (no recorded game, a game that does not replay to its score,
/// does not play U\* above n/2, or is not a promotion set of Π⁺(O)), if Π⁺(O) cannot be set up (an open member that
/// is no kept source of three or more prime factors, more frames than a frame set holds), if the search fails, or if
/// the search found nothing better but `upper`'s score is above the recorded one (no certificate then).
pub fn certify_game(
    n: u32,
    known: &Optimal,
    upper: &Upper,
    claims: bool,
    opts: &SolveOptions,
) -> Result<Certificate, String> {
    certify_game_with(n, known, upper, claims, opts, &mut EarlierCertificates::default())
}

/// [`certify_game`] with the certified games `earlier`, which a game may rest on instead of a search (the rule of the
/// module docs): after the checks of the recorded game and before the search, a game whose problem an earlier game
/// of the last `SAME_SEARCH_WINDOW` board sizes posed, and whose recorded game is worth what that game's certificate
/// proved the most, is certified with no search, its [`Certificate::same_problem`] naming that game; the earlier game
/// may have been certified in the other formulation.  A certified game, by search or by the rule, is added to
/// `earlier`.
///
/// # Errors
///
/// As [`certify_game`], and if the recorded game is worth more than an earlier certificate for the same problem proved
/// the most a promotion set is worth (one of them is wrong, and nothing is certified on it).
pub fn certify_game_with(
    n: u32,
    known: &Optimal,
    upper: &Upper,
    claims: bool,
    opts: &SolveOptions,
    earlier: &mut EarlierCertificates,
) -> Result<Certificate, String> {
    let beyond = || format!("n={n} is beyond the largest supported board, n={MAX_N}");
    // A problem that no set width can search is set up at the widest, whose refusal says why.
    let width = certificate_width(n, &upper.open, claims)?.1.unwrap_or(WIDTHS[WIDTHS.len() - 1]);
    crate::with_words!(width, W => certify_at::<W>(n, known, upper, claims, opts, earlier), else Err(beyond()))
}

/// The block words the GF(2) bound of Π⁺(`open`) of the board `1..=n` needs at its root ([`Board::extended_words`]: an
/// open source adds one to its frame's rank), and the set width `taxman certify` searches the game at: the board's own
/// width where its engines have that many words, else the widest, whose engines have the most ([`MAX_WORDS`]); `None`
/// if no width does.  `claims` is the formulation of Π⁺(`open`) ([`Board::extended`]), which can change the words.  The
/// width depends on nothing but the board, `open` and `claims`, so a resumed search finds the width its checkpoint was
/// made at, and a certificate's line records it (`width`).
///
/// # Errors
///
/// If `n` is beyond the widest set, or the extended problem cannot be built ([`Board::extended_words`]); the message
/// names `n`.
pub fn certificate_width(n: u32, open: &[u32], claims: bool) -> Result<(usize, Option<usize>), String> {
    let beyond = || format!("n={n} is beyond the largest supported board, n={MAX_N}");
    let (words, own, fits) = crate::with_width!(n, W => {
        let open: Bits<W> = open.iter().copied().collect();
        let words = Board::<W>::for_n(n).extended_words(&open, claims).map_err(|e| format!("n={n}: {e}"))?;
        Ok((words, W, words <= Gf2Bound::<W>::MOST_WORDS))
    }, else Err(beyond()))?;
    let widest = WIDTHS[WIDTHS.len() - 1];
    let width = if fits {
        Some(own)
    } else if words <= MAX_WORDS {
        Some(widest)
    } else {
        None
    };
    Ok((words, width))
}

fn certify_at<const W: usize>(
    n: u32,
    known: &Optimal,
    upper: &Upper,
    claims: bool,
    opts: &SolveOptions,
    earlier: &mut EarlierCertificates,
) -> Result<Certificate, String> {
    let at = |e: String| format!("n={n}: {e}");
    if upper.n != n {
        return Err(at(format!("the barred bounds given are those of n={}", upper.n)));
    }
    let (Some(score), Some(moves)) = (known.score(n), known.moves(n)) else {
        return Err(at("the results have no game of n".into()));
    };
    let checkpoint = opts.checkpoint_file(n).filter(|path| path.exists());
    let resuming = checkpoint.is_some();
    let Recorded { board, extended, prepared, posed, start, worth } =
        check_recorded_game::<W>(n, score, moves, &upper.open, claims, checkpoint.as_deref())?;
    let cap = known.score(n - 1).and_then(|prev| promotion_cap(n, extended.in_the_bag, prev));

    // An earlier certified game that posed this very problem has settled it: no search.
    let same = earlier.find(n, &posed.problem, claims, worth).map_err(at)?;
    let found = match &same {
        Some(_) => None,
        None => Some(search_from(n, opts, &prepared, &start, cap, resuming).map_err(at)?),
    };
    let verdict = match &found {
        Some(found) if !found.completed => Verdict::Incomplete,
        Some(found) if found.sum > worth => Verdict::Beaten,
        _ => Verdict::Certified,
    };
    if verdict == Verdict::Certified && upper.score > score {
        return Err(at(format!(
            "nothing beats {score}, but the open members were found against {}, more: not a certificate",
            upper.score
        )));
    }
    let beaten_by = match &found {
        Some(found) if verdict == Verdict::Beaten => {
            Some(beating_game(&board.tables, &extended, &found.promotions, score).map_err(at)?)
        }
        _ => None,
    };
    let (nodes, seconds, seconds_all_runs) = found.as_ref().map_or((0, 0.0, 0.0), |found| {
        (found.stats.nodes, found.time.as_secs_f64(), found.time_all_runs.as_secs_f64())
    });
    let certificate = Certificate {
        n,
        score,
        open: upper.open.clone(),
        rejected: posed.rejected,
        frames: posed.frames,
        two_sided: posed.two_sided,
        one_sided: posed.one_sided,
        in_the_bag: posed.in_the_bag,
        nodes,
        seconds,
        seconds_all_runs,
        threads: opts.threads,
        width: W,
        claims,
        verdict,
        digest: posed.problem.digest(),
        same_problem: same,
        beaten_by,
    };
    if verdict == Verdict::Certified {
        earlier.record(&certificate, posed.problem);
    }
    Ok(certificate)
}

/// What the checks of a recorded game ([`check_recorded_game`]) leave for the search, or for a verifier of the
/// certificate: the board, the extended problem and the search set up for it, the problem as posed, the recorded game
/// as a promotion set of it and what that is worth.
struct Recorded<const W: usize> {
    board: Board<W>,
    extended: ExtendedFrames<W>,
    prepared: Prepared<W>,
    posed: Posed,
    /// The recorded game as a promotion set of Π⁺(O): the open sources, and its moves up to n/2.
    start: Bits<W>,
    /// What `start` is worth, which with `in_the_bag⁺` is the recorded score.
    worth: u64,
}

/// The checks a certificate makes of the recorded game of `n` before it searches or settles anything, shared by
/// [`certify_game_with`] and the verifier of a certificate's line (`verify`): the game replays to `score`, plays U\*
/// above n/2 and is a promotion set of Π⁺(`open`) worth `score`, set up at the set width `W`, in the formulation
/// `claims` ([`Board::extended`]).  The search is set up to resume the checkpoint `resume`, if there is one.
///
/// # Errors
///
/// If one of the checks fails, if Π⁺(`open`) cannot be set up, or the checkpoint does not belong to it (another
/// formulation's does not); the message names `n`.
fn check_recorded_game<const W: usize>(
    n: u32,
    score: u64,
    moves: &[u32],
    open: &[u32],
    claims: bool,
    resume: Option<&Path>,
) -> Result<Recorded<W>, String> {
    let at = |e: String| format!("n={n}: {e}");
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

    let open: Bits<W> = open.iter().copied().collect();
    let (extended, prepared) = match resume {
        Some(checkpoint) => board.prepare_extended_for_resume(&open, claims, checkpoint),
        None => board.prepare_extended_with(&open, claims, SearchTactics::extended()),
    }
    .map_err(at)?;
    let posed = Posed::new(&extended, prepared.problem());
    debug_assert_eq!(posed.frames, prepared.root.len(), "n={n}: the problem's frames are the root's");

    let start = open | (recorded - kept);
    let promotable = start.is_subset(&prepared.candidates)
        && prepared.model.can_promote_all_using(&mut Scratch::new(), &prepared.root, &start);
    if !promotable {
        return Err(at("the recorded game is not a promotion set of Π⁺(O) (Theorem M⁺ says a playable one is)".into()));
    }
    let worth = start.sum();
    if extended.in_the_bag + worth != score {
        return Err(at(format!(
            "the recorded game is worth {} in Π⁺(O), not its score {score}",
            extended.in_the_bag + worth
        )));
    }
    Ok(Recorded { board, extended, prepared, posed, start, worth })
}

/// Searches the extended problem `prepared` from the recorded game `start`, with the ceiling `cap`, on `opts`' threads
/// and with its checkpoint (resumed if `resuming`), limits and progress.
fn search_from<const W: usize>(
    n: u32,
    opts: &SolveOptions,
    prepared: &Prepared<W>,
    start: &Bits<W>,
    cap: Option<u64>,
    resuming: bool,
) -> Result<SearchResult<W>, String> {
    let (spill, own) = spill_dir(opts, n);
    let cfg = EngineConfig {
        threads: opts.threads,
        queue_mem_cap: opts.queue_mem_cap,
        spill_dir: Some(spill.clone()),
        checkpoint: opts.checkpoint_file(n).map(|path| (path, opts.checkpoint_interval)),
        // Deleted by the caller once the line is safe.
        keep_completed_checkpoint: true,
        progress: opts.progress,
        time_limit: opts.time_limit,
        node_limit: opts.node_limit,
        label: format!("n={n}"),
        progress_note: None,
        cores: None,
        control: None,
    };
    let found = search::search(prepared, &cfg, Some(start), cap, resuming);
    if own {
        let _ = fs::remove_dir(&spill);
    }
    found.map_err(|e| format!("the search failed: {e}"))
}

// ---------------------------------------------------------------------------------------------------------
// The problem a certificate poses, and the games it may rest on

/// Π⁺(O) of a board as a certificate records it: the search problem, and the sizes and `in_the_bag⁺` that a
/// certificate's line has beside its digest, so that a problem set up again from the line can be checked against it.
#[derive(Clone, Debug)]
pub struct Posed {
    pub problem: Problem,
    pub frames: usize,
    pub two_sided: u32,
    pub one_sided: u32,
    pub rejected: u32,
    pub in_the_bag: u64,
}

impl Posed {
    /// The problem `problem` of the extended problem `extended`, with its sizes.
    fn new<const W: usize>(extended: &ExtendedFrames<W>, problem: Problem) -> Posed {
        Posed {
            frames: problem.size().frames as usize,
            two_sided: extended.candidates.len(),
            one_sided: extended.one_sided.len() as u32,
            rejected: extended.rejected.len(),
            in_the_bag: extended.in_the_bag,
            problem,
        }
    }

    /// Whether the problem and its sizes are those the certificate `line` records: its digest, frames, two-sided and
    /// one-sided candidates, rejected sources and `in_the_bag⁺`.  The problem must have been set up in the line's own
    /// formulation ([`Certificate::claims`]), as the digest of the other would not be the line's.
    pub fn reproduces(&self, line: &Certificate) -> bool {
        self.problem.digest() == line.digest
            && (self.frames, self.two_sided, self.one_sided, self.rejected, self.in_the_bag)
                == (line.frames, line.two_sided, line.one_sided, line.rejected, line.in_the_bag)
    }
}

/// Π⁺(`open`) of the board `1..=n` in the formulation `claims` ([`Board::extended`]), set up without a search: the
/// problem a certificate for `n` with the open members `open` poses.  The board is taken at its own set width, which
/// the problem does not depend on (its form has game numbers only), so a game that was searched at the widest width
/// has the same problem.
///
/// # Errors
///
/// If `n` is beyond the widest set, or `open` holds a number that cannot be open or the problem cannot be built
/// ([`Board::extended_problem`]); the message names `n`.
pub fn extended_problem(n: u32, open: &[u32], claims: bool) -> Result<Posed, String> {
    crate::with_width!(n, W => {
        let open: Bits<W> = open.iter().copied().collect();
        Board::<W>::for_n(n)
            .extended_problem(&open, claims)
            .map(|(extended, problem)| Posed::new(&extended, problem))
            .map_err(|e| format!("n={n}: {e}"))
    }, else Err(format!("n={n} is beyond the largest supported board, n={MAX_N}")))
}

/// The problem of the certified line `line` in the formulation `claims`, set up again from the line: first in the
/// line's own formulation, where it must reproduce the line ([`Posed::reproduces`]), then, if `claims` is the other
/// formulation, in that one.  `None` if either setup fails or the line is not reproduced: the line is no certificate
/// of the problem it says.
fn problem_in(line: &Certificate, claims: bool) -> Option<Problem> {
    let posed = extended_problem(line.n, &line.open, line.claims).ok().filter(|posed| posed.reproduces(line))?;
    if claims == line.claims {
        return Some(posed.problem);
    }
    extended_problem(line.n, &line.open, claims).ok().map(|posed| posed.problem)
}

/// The certified games a game may rest on (the rule of the module docs): the certificates of the file `taxman certify`
/// appends to and of the games this run certified, and the problems this run set up for the latter.
#[derive(Default)]
pub struct EarlierCertificates {
    /// The certificates with the verdict `certified`, by board size.
    games: BTreeMap<u32, Certificate>,
    /// The problems of the games this run certified ([`record`](Self::record)), each in the formulation of its
    /// game's certificate, for the last `SAME_SEARCH_WINDOW` board sizes: what a comparison with an earlier game needs,
    /// so that it need not set that game up again.
    problems: BTreeMap<u32, Problem>,
}

impl EarlierCertificates {
    /// The games the file `file` has a certified certificate for.
    pub fn of_file(file: &CertificateFile) -> Self {
        let mut earlier = Self::default();
        for certificate in file.certified() {
            earlier.insert(certificate.clone());
        }
        earlier
    }

    /// Adds the certificate of a certified game, replacing one of the same board size (and forgetting the problem
    /// remembered for it, which the new certificate may not be of); a certificate with another verdict is ignored.
    pub fn insert(&mut self, certificate: Certificate) {
        if certificate.verdict == Verdict::Certified {
            self.problems.remove(&certificate.n);
            self.games.insert(certificate.n, certificate);
        }
    }

    /// Adds the certificate of a game this run certified, with the problem it posed (in the certificate's formulation),
    /// and forgets the problems of the boards more than `SAME_SEARCH_WINDOW` sizes before the latest one remembered: no
    /// game that follows looks that far back.
    fn record(&mut self, certificate: &Certificate, problem: Problem) {
        debug_assert_eq!(certificate.verdict, Verdict::Certified);
        self.games.insert(certificate.n, certificate.clone());
        self.problems.insert(certificate.n, problem);
        let latest = self.problems.keys().next_back().copied().unwrap_or(certificate.n);
        self.problems.retain(|&m, _| latest - m <= SAME_SEARCH_WINDOW);
    }

    /// The latest certified game m of the last `SAME_SEARCH_WINDOW` board sizes before `n` whose problem is `ours`,
    /// the problem of the recorded game of `n` in the formulation `claims`, worth `worth`, and what that means for `n`:
    /// `Some` with the record of m if `worth` is what m's certificate proved the most, `None` if there is no such game
    /// or if `worth` is less (`n` is searched, then).
    ///
    /// A game of the same formulation whose digest is not `ours`' is passed over.  A game of the other formulation has
    /// a digest that says nothing of `ours`, so it is passed over unless the sizes that do not depend on the
    /// formulation, the frames and the candidates, are those of `ours`.  For a game that is not passed over, its
    /// problem in the formulation `claims` (the one this run set up when it certified it, if its formulation is
    /// `claims`, else set up again from its board size and open members, [`problem_in`]) is compared with `ours`
    /// itself, so that the answer never rests on two digests alone.  The first game whose problem is `ours` decides.  A
    /// game that cannot be set up again, or whose problem set up again is not the one its line records (the
    /// formulation, the digest and the sizes), or whose score is below its `in_the_bag⁺`, is forgotten.
    ///
    /// # Errors
    ///
    /// If `worth` is more than what the certificate of the game says the most is: one of the two is wrong.
    fn find(&mut self, n: u32, ours: &Problem, claims: bool, worth: u64) -> Result<Option<SameProblem>, String> {
        let (digest, size) = (ours.digest(), ours.size());
        let (frames, candidates) = (size.frames as usize, size.candidates);
        let may_pose = |line: &Certificate| {
            if line.claims == claims {
                line.digest == digest
            } else {
                (line.frames, line.two_sided + line.one_sided) == (frames, candidates)
            }
        };
        for m in (n.saturating_sub(SAME_SEARCH_WINDOW)..n).rev() {
            let Some(line) = self.games.get(&m).filter(|line| may_pose(line)) else { continue };
            let theirs = match self.problems.get(&m) {
                Some(problem) if line.claims == claims => Some(Cow::Borrowed(problem)),
                _ => problem_in(line, claims).map(Cow::Owned),
            };
            let (Some(best), Some(theirs)) = (line.worth(), theirs) else {
                self.games.remove(&m);
                continue;
            };
            match apply_rule(line, best, &theirs, ours, worth)? {
                Rule::Other => continue,
                Rule::Searched => return Ok(None),
                Rule::Settled(same) => return Ok(Some(same)),
            }
        }
        Ok(None)
    }
}

/// What the certified game m, whose problem is `theirs` and whose certificate proved the most a promotion set of it is
/// worth, `best`, says of a later game: its problem `ours`, in which its recorded game is worth `worth`.
#[derive(Debug, PartialEq, Eq)]
enum Rule {
    /// The problems differ: m says nothing of the game.
    Other,
    /// The same problem, and the recorded game is worth less than m's certificate proved the most: the game is
    /// searched.
    Searched,
    /// The same problem, and the recorded game is worth what m's certificate proved the most: the game is certified
    /// with no search, resting on m.
    Settled(SameProblem),
}

/// The rule of the module docs, for the certified line `m` (see [`Rule`]); the one place it is written, which
/// [`EarlierCertificates::find`] applies and the verifier of a line applies again to check it.  The problems `theirs`
/// and `ours` are compared whole, so they must be in the same formulation, whichever `m`'s certificate was made in.
///
/// # Errors
///
/// If `worth` is more than `best` for the same problem: one of the two is wrong.
fn apply_rule(m: &Certificate, best: u64, theirs: &Problem, ours: &Problem, worth: u64) -> Result<Rule, String> {
    if theirs != ours {
        return Ok(Rule::Other);
    }
    match worth.cmp(&best) {
        Ordering::Equal => Ok(Rule::Settled(SameProblem { n: m.n, digest: ours.digest(), best, worth })),
        Ordering::Less => Ok(Rule::Searched),
        Ordering::Greater => Err(format!(
            "the recorded game is worth {worth} in Π⁺(O), which is the problem of n={0} too, and the certificate of \
             n={0} says no promotion set of it is worth more than {best}: it contradicts the certificate of n={0}",
            m.n
        )),
    }
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
/// a results file is locked (`results::ResultsFile`), and reads the last certificate it has with a final verdict
/// (`certified` or `beaten`) for each game, which a run skips and the certified ones of which a later game may rest on
/// ([`EarlierCertificates`]); a game with only `incomplete` lines is resumed from its checkpoint.  A torn last line (a
/// run killed while writing it) is cut off.
pub struct CertificateFile {
    path: PathBuf,
    file: File,
    finals: BTreeMap<u32, Certificate>,
}

impl CertificateFile {
    /// Opens (creating if need be) and locks the file at `path`, and reads its certificates.
    ///
    /// # Errors
    ///
    /// If it cannot be opened, another process holds it, or a complete line is not a certificate
    /// ([`Certificate::from_json`]): the message names the file and the line.
    pub fn open(path: &Path) -> Result<CertificateFile, String> {
        let (file, text) = open_locked(path)?;
        let at = |i: usize, e: &str| format!("{}: line {}: {e}", path.display(), i + 1);
        let complete = text.rfind('\n').map_or(0, |end| end + 1);
        let mut finals = BTreeMap::new();
        for (i, line) in text[..complete].lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
            let json = Json::parse(line).map_err(|e| at(i, &e.to_string()))?;
            let certificate = Certificate::from_json(&json).map_err(|e| at(i, &e))?;
            if certificate.verdict != Verdict::Incomplete {
                finals.insert(certificate.n, certificate);
            }
        }
        if complete < text.len() {
            file.set_len(complete as u64).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        Ok(CertificateFile { path: path.to_owned(), file, finals })
    }

    /// The final verdict the file has for `n`, if any.
    pub fn verdict(&self, n: u32) -> Option<Verdict> {
        self.certificate(n).map(|certificate| certificate.verdict)
    }

    /// The last certificate with a final verdict the file has for `n`, if any.
    pub fn certificate(&self, n: u32) -> Option<&Certificate> {
        self.finals.get(&n)
    }

    /// The certificates with the verdict `certified` that the file has a final one for, by board size.
    pub fn certified(&self) -> impl Iterator<Item = &Certificate> {
        self.finals.values().filter(|certificate| certificate.verdict == Verdict::Certified)
    }

    /// Appends `certificate` and makes sure it reaches the disk.
    pub fn append(&mut self, certificate: &Certificate) -> Result<(), String> {
        append_line(&mut self.file, &certificate.to_json()).map_err(|e| format!("{}: {e}", self.path.display()))?;
        if certificate.verdict != Verdict::Incomplete {
            self.finals.insert(certificate.n, certificate.clone());
        }
        Ok(())
    }

    /// Where the certificates of the games that beat their recorded score go as well: [`counterexample_path`] of the
    /// file.
    pub fn counterexample_path(&self) -> PathBuf {
        counterexample_path(&self.path)
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

/// Where the certificates of the games that beat their recorded score go as well, for the file of certificates
/// `certificates`: `<out>.counterexample.jsonl` (with a `.jsonl` ending of the file dropped first, as for a results
/// file's rejected games).
pub fn counterexample_path(certificates: &Path) -> PathBuf {
    sibling_path(certificates, "counterexample")
}

/// Dropping releases the lock at once, as a results file's does.
impl Drop for CertificateFile {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All of U\*_{≥3} of the board `1..=n`: the most open members `taxman certify` can have.
    fn from_3(n: u32) -> Vec<u32> {
        crate::with_width!(n, W => {
            let board = Board::<W>::for_n(n);
            board.plain.iter().copied().filter(|&x| 2 * x > n && board.tables.omega(x) >= 3).collect()
        })
    }

    /// The certificate of board `n`, made by hand, whose problem is `posed` (set up with the open members `open`, with
    /// the claims of `unzip`: the default formulation) and whose game is worth `best` in it.
    fn line(n: u32, posed: &Posed, open: &[u32], best: u64) -> Certificate {
        Certificate {
            n,
            score: posed.in_the_bag + best,
            open: open.to_vec(),
            rejected: posed.rejected,
            frames: posed.frames,
            two_sided: posed.two_sided,
            one_sided: posed.one_sided,
            in_the_bag: posed.in_the_bag,
            nodes: 17,
            seconds: 0.25,
            seconds_all_runs: 0.5,
            threads: 1,
            width: 8,
            claims: true,
            verdict: Verdict::Certified,
            digest: posed.problem.digest(),
            same_problem: None,
            beaten_by: None,
        }
    }

    /// The certified games `lines`, with `problems` as the problems this run set up for them.
    fn remembering(lines: &[Certificate], problems: &[(u32, &Problem)]) -> EarlierCertificates {
        let mut earlier = EarlierCertificates::default();
        for line in lines {
            earlier.games.insert(line.n, line.clone());
        }
        for &(m, problem) in problems {
            earlier.problems.insert(m, problem.clone());
        }
        earlier
    }

    /// The block words the GF(2) bound needs grow with the open sources (each adds one to its frame's rank), and the
    /// set width follows: the board's own while the engines of that width have as many words (three up to 64 words),
    /// the widest (five) beyond that, none past five.
    #[test]
    fn the_width_follows_the_block_words_the_open_sources_need() {
        // The same in both formulations (the claims take pairs out of frames the open sources add to).
        for claims in [false, true] {
            let width = |n: u32, open: &[u32]| certificate_width(n, open, claims).unwrap();
            assert_eq!(width(600, &from_3(600)), (2, Some(16)));
            assert_eq!(width(1000, &from_3(1000)), (3, Some(16)));
            assert_eq!(width(1500, &from_3(1500)), (4, Some(128)));
            assert_eq!(width(2000, &from_3(2000)), (5, Some(128)));
            assert_eq!(width(3000, &from_3(3000)), (7, None));
            assert_eq!(width(1500, &[]).1, Some(24), "with nothing open the board's own width");
            assert!(certificate_width(1 << 20, &[], claims)
                .unwrap_err()
                .contains("beyond the largest supported board"));
            assert!(certificate_width(120, &[7], claims).unwrap_err().starts_with("n=120: "));
        }
    }

    #[test]
    fn a_certificate_reads_back_as_it_was_written() {
        let open = from_3(120);
        let posed = extended_problem(120, &open, true).unwrap();
        let plain = line(120, &posed, &open, 77);
        let same = SameProblem { n: 119, digest: plain.digest, best: 77, worth: 77 };
        let beaten = Certificate { verdict: Verdict::Beaten, beaten_by: Some((1001, vec![7, 4, 6])), ..plain.clone() };
        let rule = Certificate { same_problem: Some(same), nodes: 0, ..plain.clone() };
        let without = Certificate { claims: false, ..plain.clone() };
        let all = [plain, without, beaten, rule.clone(), Certificate { beaten_by: Some((5, vec![])), ..rule }];
        assert!(all[0].claims && !all[1].claims, "both formulations are written");
        for certificate in all {
            let json = Json::parse(&certificate.to_json().to_string()).unwrap();
            assert_eq!(Certificate::from_json(&json), Ok(certificate));
        }
    }

    #[test]
    fn a_line_that_is_no_certificate_is_refused_naming_the_field() {
        let open = from_3(120);
        let posed = extended_problem(120, &open, true).unwrap();
        let same = SameProblem { n: 119, digest: posed.problem.digest(), best: 77, worth: 77 };
        let whole = Certificate {
            same_problem: Some(same),
            beaten_by: Some((1001, vec![7, 4, 6])),
            ..line(120, &posed, &open, 77)
        };
        let json = whole.to_json();
        let pairs = json.as_obj().unwrap();
        let without = |key: &str| Json::object(pairs.iter().filter(|(k, _)| k != key).cloned());
        let with = |key: &str, value: Json| {
            Json::object(pairs.iter().map(|(k, v)| (k.clone(), if k == key { value.clone() } else { v.clone() })))
        };
        let refused = |json: Json| Certificate::from_json(&json).unwrap_err();

        assert_eq!(refused(without("n")), "no board size");
        assert_eq!(refused(with("n", Json::from("x"))), "no board size");
        assert_eq!(refused(without("verdict")), "no verdict");
        assert_eq!(refused(with("verdict", Json::from("maybe"))), "no verdict");
        let fields = ["score", "open", "rejected", "frames", "two_sided", "one_sided", "in_the_bag", "nodes"];
        let more = ["seconds", "seconds_all_runs", "threads", "width", "digest"];
        for key in fields.into_iter().chain(more) {
            assert_eq!(refused(without(key)), format!("missing \"{key}\""));
            assert_eq!(refused(with(key, Json::from("x"))), format!("bad \"{key}\""));
        }
        // The formulation is written right after the width; a line without it is one made before the claims existed,
        // and reads as without them, but one with a `claims` that is no boolean is refused.
        let keys: Vec<&str> = pairs.iter().map(|(key, _)| key.as_str()).collect();
        let at = keys.iter().position(|&key| key == "claims").expect("a line records its formulation");
        assert_eq!((keys[at - 1], keys[at + 1]), ("width", "digest"));
        assert!(whole.claims);
        assert_eq!(Certificate::from_json(&without("claims")), Ok(Certificate { claims: false, ..whole.clone() }));
        for bad in [Json::from("x"), Json::from(1u32), Json::Null] {
            assert_eq!(refused(with("claims", bad)), "bad \"claims\"");
        }
        assert_eq!(refused(with("digest", Json::from("feed"))), "bad \"digest\"", "a digest has 16 digits");
        assert_eq!(refused(with("same_problem", Json::Null)), "bad \"same_problem\"");
        assert_eq!(refused(with("same_problem", Json::object([("n", Json::from(119u32))]))), "bad \"same_problem\"");
        assert_eq!(refused(with("beaten_by", Json::Null)), "bad \"beaten_by\"");
        assert_eq!(refused(with("beaten_by", Json::object([("score", Json::from(5u64))]))), "bad \"beaten_by\"");
        assert_eq!(Certificate::from_json(&json), Ok(whole));
    }

    #[test]
    fn a_problem_set_up_again_reproduces_its_line_and_nothing_else() {
        let open = from_3(120);
        let posed = extended_problem(120, &open, true).unwrap();
        let line = line(120, &posed, &open, 77);
        assert!(extended_problem(120, &open, true).unwrap().reproduces(&line));
        let others = [
            Certificate { digest: line.digest ^ 1, ..line.clone() },
            Certificate { frames: line.frames + 1, ..line.clone() },
            Certificate { two_sided: line.two_sided + 1, ..line.clone() },
            Certificate { one_sided: line.one_sided + 1, ..line.clone() },
            Certificate { rejected: line.rejected + 1, ..line.clone() },
            Certificate { in_the_bag: line.in_the_bag + 1, ..line.clone() },
        ];
        assert!(others.iter().all(|other| !posed.reproduces(other)));
        // Other open members are another problem, and a number that cannot be open or a board that is too large an error.
        assert!(!extended_problem(120, &open[1..], true).unwrap().reproduces(&line));
        assert!(extended_problem(120, &[7], true).unwrap_err().starts_with("n=120: "));
        assert!(extended_problem(1 << 20, &[], true).unwrap_err().contains("beyond the largest supported board"));
    }

    #[test]
    fn the_window_never_looks_past_the_last_eight_sizes() {
        let open = from_3(120);
        let posed = extended_problem(120, &open, true).unwrap();
        let (n, worth, digest) = (200, 77, posed.problem.digest());
        let found = |m: u32| Some(SameProblem { n: m, digest, best: worth, worth });
        for (m, expected) in [(n - 9, None), (n - 8, found(n - 8)), (n - 1, found(n - 1)), (n, None), (n + 1, None)] {
            let mut earlier = remembering(&[line(m, &posed, &open, worth)], &[(m, &posed.problem)]);
            assert_eq!(earlier.find(n, &posed.problem, true, worth), Ok(expected), "the game of {m}");
        }
        // A game this run did not certify is set up again from its line: the board of the line, with its open members.
        let mut earlier = EarlierCertificates::default();
        earlier.insert(line(120, &posed, &open, worth));
        assert_eq!(earlier.find(121, &posed.problem, true, worth), Ok(found(120)));
        assert_eq!(earlier.find(128, &posed.problem, true, worth), Ok(found(120)));
        assert_eq!(earlier.find(129, &posed.problem, true, worth), Ok(None));
    }

    #[test]
    fn the_first_game_with_the_same_problem_decides() {
        let open = from_3(120);
        let posed = extended_problem(120, &open, true).unwrap();
        let (n, worth, digest) = (200, 77, posed.problem.digest());
        let lines = |latest: u64, before: u64| [line(n - 1, &posed, &open, latest), line(n - 2, &posed, &open, before)];
        let remembered = [(n - 1, &posed.problem), (n - 2, &posed.problem)];

        // The latest decides, whatever an earlier one says.
        let mut earlier = remembering(&lines(worth, worth + 1), &remembered);
        let same = SameProblem { n: n - 1, digest, best: worth, worth };
        assert_eq!(earlier.find(n, &posed.problem, true, worth), Ok(Some(same)));
        // A game worth less than the certificate says is not settled by the rule (and the window is not searched on).
        let mut earlier = remembering(&lines(worth + 1, worth), &remembered);
        assert_eq!(earlier.find(n, &posed.problem, true, worth), Ok(None));
        // A game worth more contradicts it: an error that names both games' values.
        let mut earlier = remembering(&lines(worth - 1, worth), &remembered);
        let e = earlier.find(n, &posed.problem, true, worth).unwrap_err();
        assert!(e.contains(&format!("n={}", n - 1)) && e.contains("contradicts the certificate"), "{e}");
        assert!(e.contains(&worth.to_string()) && e.contains(&(worth - 1).to_string()), "{e}");
    }

    #[test]
    fn a_game_that_cannot_be_used_is_passed_over_and_the_window_goes_on() {
        let open = from_3(120);
        let posed = extended_problem(120, &open, true).unwrap();
        let other = extended_problem(100, &from_3(100), true).unwrap();
        assert_ne!(other.problem, posed.problem);
        let (n, worth) = (200, 77);
        let good = line(n - 4, &posed, &open, worth);
        let found = Ok(Some(SameProblem { n: n - 4, digest: posed.problem.digest(), best: worth, worth }));

        // Another digest: passed over.
        let elsewhere = Certificate { digest: other.problem.digest(), ..line(n - 1, &posed, &open, worth) };
        let mut earlier = remembering(&[elsewhere, good.clone()], &[(n - 1, &other.problem), (n - 4, &posed.problem)]);
        assert_eq!(earlier.find(n, &posed.problem, true, worth), found);
        assert!(earlier.games.contains_key(&(n - 1)));

        // The digest of the problem but another problem (a collision of 64 bits): passed over, and the line kept.
        let forged = Certificate { digest: posed.problem.digest(), ..line(n - 1, &other, &from_3(100), worth) };
        let mut earlier =
            remembering(&[forged.clone(), good.clone()], &[(n - 1, &other.problem), (n - 4, &posed.problem)]);
        assert_eq!(earlier.find(n, &posed.problem, true, worth), found);
        assert!(earlier.games.contains_key(&(n - 1)));

        // The same line, whose problem is not remembered, set up again from the line gives another digest than the
        // line records (the board 199 with these members open poses another problem, or cannot have them open):
        // dropped.
        let mut earlier = remembering(&[forged, good.clone()], &[(n - 4, &posed.problem)]);
        assert_eq!(earlier.find(n, &posed.problem, true, worth), found);
        assert!(!earlier.games.contains_key(&(n - 1)), "a line that does not reproduce is forgotten");

        // The problem set up again from the line is the problem, but the line's sizes are not its: dropped.
        let mut earlier = EarlierCertificates::default();
        earlier.insert(Certificate { two_sided: posed.two_sided + 1, ..line(120, &posed, &open, worth) });
        assert_eq!(earlier.find(125, &posed.problem, true, worth), Ok(None));
        assert!(earlier.games.is_empty(), "a line that does not reproduce is forgotten");

        // A score below its own in_the_bag⁺ cannot be a certificate: dropped.
        let below = Certificate { score: posed.in_the_bag - 1, ..line(n - 1, &posed, &open, worth) };
        let mut earlier = remembering(&[below, good], &[(n - 1, &posed.problem), (n - 4, &posed.problem)]);
        assert_eq!(earlier.find(n, &posed.problem, true, worth), found);
        assert!(!earlier.games.contains_key(&(n - 1)));
    }

    /// What a line records of the problem besides its digest, as [`Posed`] has it.
    fn sizes(posed: &Posed) -> (usize, u32, u32, u32, u64) {
        (posed.frames, posed.two_sided, posed.one_sided, posed.rejected, posed.in_the_bag)
    }

    /// A small board and open members for which the two formulations pose different problems of the same sizes: the
    /// claims take pairs out of the frames, and change neither the number of frames nor the candidates.
    fn claims_change_the_problem() -> (u32, Vec<u32>) {
        let mut boards = (60..=200).flat_map(|n| [(n, Vec::new()), (n, from_3(n))]);
        boards
            .find(|(n, open)| {
                let (without, with) = (extended_problem(*n, open, false), extended_problem(*n, open, true));
                let (without, with) = (without.unwrap(), with.unwrap());
                without.problem != with.problem && sizes(&without) == sizes(&with)
            })
            .expect("a small board whose claims change its problem")
    }

    #[test]
    fn a_problem_is_set_up_again_in_the_formulation_asked_for_whichever_the_line_is_of() {
        let (n, open) = claims_change_the_problem();
        let without = extended_problem(n, &open, false).unwrap();
        let with = extended_problem(n, &open, true).unwrap();
        let from_without = Certificate { claims: false, ..line(n, &without, &open, 77) };
        let from_with = line(n, &with, &open, 77);
        assert_eq!(problem_in(&from_without, false), Some(without.problem.clone()));
        assert_eq!(problem_in(&from_without, true), Some(with.problem.clone()));
        assert_eq!(problem_in(&from_with, true), Some(with.problem.clone()));
        assert_eq!(problem_in(&from_with, false), Some(without.problem.clone()));

        // A line that is not reproduced in its own formulation is no line of any problem: the digest of the other
        // formulation, a size, a digest that is nothing's.
        let others = [
            Certificate { digest: with.problem.digest(), ..from_without.clone() },
            Certificate { frames: from_without.frames + 1, ..from_without.clone() },
            Certificate { digest: from_without.digest ^ 1, ..from_without.clone() },
            Certificate { claims: true, ..from_without },
        ];
        for other in &others {
            assert_eq!((problem_in(other, false), problem_in(other, true)), (None, None), "{other:?}");
        }
    }

    #[test]
    fn a_line_of_the_other_formulation_settles_a_game_whose_problem_in_the_runs_is_the_same() {
        let (m, open) = claims_change_the_problem();
        let without = extended_problem(m, &open, false).unwrap();
        let with = extended_problem(m, &open, true).unwrap();
        let (n, worth) = (m + 5, 77);
        let found = |ours: &Posed| Ok(Some(SameProblem { n: m, digest: ours.problem.digest(), best: worth, worth }));

        // A line made without the claims settles a game of the run with them, whether or not the run remembers the
        // problem it set up for the line (the one without the claims, which is not the problem to compare).
        let from_without = Certificate { claims: false, ..line(m, &without, &open, worth) };
        for remembered in [vec![], vec![(m, &without.problem)]] {
            let mut earlier = remembering(std::slice::from_ref(&from_without), &remembered);
            assert_eq!(earlier.find(n, &with.problem, true, worth), found(&with), "{} remembered", remembered.len());
            assert_eq!(earlier.find(n, &without.problem, true, worth), Ok(None), "not the problem with the claims");
            assert_eq!(earlier.find(n, &without.problem, false, worth), found(&without), "the same formulation");
        }
        // And the other way round.
        let from_with = line(m, &with, &open, worth);
        for remembered in [vec![], vec![(m, &with.problem)]] {
            let mut earlier = remembering(std::slice::from_ref(&from_with), &remembered);
            assert_eq!(earlier.find(n, &without.problem, false, worth), found(&without));
            assert_eq!(earlier.find(n, &with.problem, false, worth), Ok(None));
        }
        // A game worth more than the certificate allows contradicts it, whichever formulation the certificate is of.
        let mut earlier = remembering(&[from_without], &[]);
        let e = earlier.find(n, &with.problem, true, worth + 1).unwrap_err();
        assert!(e.contains(&format!("contradicts the certificate of n={m}")), "{e}");
    }

    #[test]
    fn a_line_of_the_other_formulation_is_passed_over_by_its_sizes_and_forgotten_if_it_does_not_reproduce() {
        let (m, open) = claims_change_the_problem();
        let without = extended_problem(m, &open, false).unwrap();
        let with = extended_problem(m, &open, true).unwrap();
        let (n, worth) = (m + 5, 77);
        let line = Certificate { claims: false, ..line(m, &without, &open, worth) };
        let digest = with.problem.digest();

        // The frames or the candidates are not the problem's: passed over with no setup (the line is kept, even if it
        // would not have reproduced), whatever its digest says.
        let sized_apart = [
            Certificate { frames: line.frames + 1, ..line.clone() },
            Certificate { two_sided: line.two_sided + 1, ..line.clone() },
            Certificate { one_sided: line.one_sided + 1, ..line.clone() },
            Certificate { digest, frames: line.frames + 1, ..line.clone() },
        ];
        for apart in sized_apart {
            let mut earlier = remembering(&[apart], &[]);
            assert_eq!(earlier.find(n, &with.problem, true, worth), Ok(None));
            assert!(earlier.games.contains_key(&m), "a line of the other formulation is kept when passed over");
        }
        // The sizes of the problem, but not a line of any problem: not reproduced in its own formulation, forgotten.
        // The digest the problem has in the run's formulation is not its digest in the line's, so it is no line either.
        for forged in [
            Certificate { digest: line.digest ^ 1, ..line.clone() },
            Certificate { digest, ..line.clone() },
            Certificate { rejected: line.rejected + 1, ..line.clone() },
            Certificate { in_the_bag: line.in_the_bag + 1, ..line.clone() },
        ] {
            let mut earlier = remembering(&[forged], &[]);
            assert_eq!(earlier.find(n, &with.problem, true, worth), Ok(None));
            assert!(earlier.games.is_empty(), "a line that does not reproduce is forgotten");
        }
        // The line as it is: used.
        let mut earlier = remembering(&[line], &[]);
        assert!(matches!(earlier.find(n, &with.problem, true, worth), Ok(Some(_))));
    }

    #[test]
    fn only_the_problems_of_the_last_eight_sizes_are_kept() {
        let open = from_3(120);
        let posed = extended_problem(120, &open, true).unwrap();
        let mut earlier = EarlierCertificates::default();
        for m in 100..=112 {
            earlier.record(&line(m, &posed, &open, 77), posed.problem.clone());
        }
        let kept = |earlier: &EarlierCertificates| earlier.problems.keys().copied().collect::<Vec<_>>();
        assert_eq!(kept(&earlier), (104..=112).collect::<Vec<_>>(), "the latest, and the eight sizes before it");
        assert_eq!(earlier.games.len(), 13, "every certificate stays");
        // A game recorded out of order does not push the others out.
        earlier.record(&line(90, &posed, &open, 77), posed.problem.clone());
        assert_eq!(kept(&earlier), (104..=112).collect::<Vec<_>>());
    }

    #[test]
    fn only_certified_certificates_are_inserted() {
        let open = from_3(120);
        let posed = extended_problem(120, &open, true).unwrap();
        let certified = line(120, &posed, &open, 77);
        let mut earlier = EarlierCertificates::default();
        for verdict in [Verdict::Beaten, Verdict::Incomplete] {
            earlier.insert(Certificate { verdict, ..certified.clone() });
        }
        assert!(earlier.games.is_empty());

        // A game this run certified: its certificate and its problem. Inserting another certificate for the game
        // replaces the first and forgets the problem, which the second may not be of; another verdict changes nothing.
        earlier.record(&certified, posed.problem.clone());
        assert_eq!((earlier.games.len(), earlier.problems.len()), (1, 1));
        earlier.insert(Certificate { verdict: Verdict::Beaten, ..certified.clone() });
        assert_eq!((earlier.games[&120].verdict, earlier.problems.len()), (Verdict::Certified, 1));
        let replacement = Certificate { score: certified.score + 1, ..certified };
        earlier.insert(replacement.clone());
        assert_eq!(earlier.games[&120], replacement);
        assert!(earlier.problems.is_empty());
    }
}
