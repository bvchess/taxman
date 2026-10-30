//! The search's part of a checkpoint: its header ([`Header`]), which identifies the game and the search so that a
//! checkpoint is resumed only by the search that made it.

use super::searcher::put_bits;
use super::setup::Prepared;
use super::tactics::{MatchingPath, SearchTactics};
use crate::bits::Bits;
use crate::engine;
use crate::model::FrameSet;

/// What a checkpoint records about the search it belongs to, so that a resumed run can check it is the same
/// search (the same game, frames, candidates and tactics).
///
/// Encoded (little-endian) as the magic `TXSH`, the format [`Header::VERSION`] (u32), the width `W` (u32),
/// `n` (u32), the digests of the candidates, of the frames and of the one-sided candidates (u64 each), `max_total`
/// (u32) and the tactics' bits (u16, [`SearchTactics::bits`]); in version 6, then the tactics that tune the GF(2) search
/// (16 bytes, [`SearchTactics::tuning`]).  This build writes version 5 for a search whose tuning is that of the builds
/// before those tactics ([`SearchTactics::legacy_tuning`]: `force_window=80` and the other tuning tactics at their
/// defaults), which a header of version 5 implies, so that such a search writes the header of those builds; and
/// version 6 for one whose tuning is not, the default's since `force_window` became `all`.  It reads versions 4 to 6,
/// and a checkpoint of version 4 or 5 resumes only a search with the tuning it implies (`--tactic force_window=80`).
/// Version 4, the headers of the builds before the extended problem's search, has no digest of the one-sided
/// candidates (`max_total` and the tactics follow the digest of the frames): those builds searched only problems
/// without one-sided candidates, so a version-4 header reads as version 5 with the digest of none, and a checkpoint of a
/// plain search resumes with every other check as for version 5; one asked to resume a search with one-sided candidates
/// is refused, saying why.  Older headers (magic `TXS2`, or `TXSH` with versions 3 and below) recorded tactics that are
/// gone, and are refused with one message.  A header of any version read records whether the search used the matching
/// path ([`MatchingPath`]), which this build no longer has, and the checkpoint of such a search is refused, saying so,
/// once the rest of the header is found to be the game's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Header<const W: usize> {
    pub(super) n: u32,
    /// [`Header::digest_candidates`] of the root candidates.
    pub(super) candidates: u64,
    /// [`Header::digest_frames`] of the frames as built.
    pub(super) frames: u64,
    /// [`Header::digest_one_sided`] of the one-sided candidates of the model (those of the extended problem), each
    /// with its own frame: what tells a search of the extended problem from another with the same candidates.
    pub(super) one_sided: u64,
    pub(super) max_total: u32,
    pub(super) tactics: SearchTactics,
    /// What the tactics' bits record of the matching path (none, for a header made by [`Header::of`]); encoded in
    /// them.
    pub(super) matching: MatchingPath,
    /// The format the header was read from (for one made by [`Header::of`], the one it is written in); not encoded.
    pub(super) version: u32,
}

impl<const W: usize> Header<W> {
    pub(super) const MAGIC: &'static [u8; 4] = b"TXSH";
    pub(super) const VERSION: u32 = 5;
    pub(super) const LEN: usize = 4 + 4 + 4 + 4 + 8 + 8 + 8 + 4 + 2;
    /// The format before the digest of the one-sided candidates, which this build still reads, and its length.
    pub(super) const VERSION_4: u32 = 4;
    pub(super) const LEN_4: usize = Self::LEN - 8;
    /// The format with the tactics that tune the GF(2) search, and its length.
    pub(super) const VERSION_6: u32 = 6;
    pub(super) const LEN_6: usize = Self::LEN + SearchTactics::TUNING_LEN;

    pub(super) fn of(prepared: &Prepared<W>) -> Self {
        Header {
            n: prepared.n,
            candidates: Self::digest_candidates(&prepared.candidates),
            frames: Self::digest_frames(&prepared.root),
            one_sided: Self::digest_one_sided(prepared.model.one_sided()),
            max_total: prepared.max_total,
            tactics: prepared.tactics,
            matching: MatchingPath::default(),
            version: if prepared.tactics.tuning_is_legacy() { Self::VERSION } else { Self::VERSION_6 },
        }
    }

    pub(super) fn digest_candidates(candidates: &Bits<W>) -> u64 {
        let mut bytes = Vec::with_capacity(8 * W);
        put_bits(&mut bytes, candidates);
        engine::checksum(&bytes)
    }

    /// A digest of every frame's moves, factors and freedoms, front first.
    pub(super) fn digest_frames(frames: &FrameSet<W>) -> u64 {
        let mut bytes = Vec::with_capacity(frames.len() * (16 * W + 4));
        for frame in frames.frames() {
            put_bits(&mut bytes, &frame.moves);
            put_bits(&mut bytes, &frame.factors);
            bytes.extend_from_slice(&frame.freedoms.to_le_bytes());
        }
        engine::checksum(&bytes)
    }

    /// A digest of the one-sided candidates, each as its number and its own frame (u32 each); of none, the checksum of
    /// nothing.
    pub(super) fn digest_one_sided(one_sided: &[(u32, usize)]) -> u64 {
        let bytes: Vec<u8> =
            one_sided.iter().flat_map(|&(s, frame)| [s, frame as u32]).flat_map(u32::to_le_bytes).collect();
        engine::checksum(&bytes)
    }

    pub(super) fn encode(&self) -> Vec<u8> {
        let mut out = Self::MAGIC.to_vec();
        let tuned = !self.tactics.tuning_is_legacy();
        for word in [if tuned { Self::VERSION_6 } else { Self::VERSION }, W as u32, self.n] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out.extend_from_slice(&self.candidates.to_le_bytes());
        out.extend_from_slice(&self.frames.to_le_bytes());
        out.extend_from_slice(&self.one_sided.to_le_bytes());
        out.extend_from_slice(&self.max_total.to_le_bytes());
        out.extend_from_slice(&self.tactics.bits().to_le_bytes());
        if tuned {
            out.extend_from_slice(&self.tactics.tuning());
        }
        out
    }

    pub(super) fn decode(bytes: &[u8]) -> Result<Self, String> {
        let u32_at = |i: usize| bytes.get(i..i + 4).map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")));
        let version = if bytes.starts_with(b"TXS2") {
            Some(2)
        } else if bytes.starts_with(Self::MAGIC) {
            u32_at(4)
        } else {
            return Err("not a checkpoint of a Taxman search".into());
        };
        // Where the fields after the digest of the frames start: version 4 has no digest of the one-sided candidates.
        let (version, len, rest) = match version {
            Some(Self::VERSION) => (Self::VERSION, Self::LEN, 40),
            Some(Self::VERSION_6) => (Self::VERSION_6, Self::LEN_6, 40),
            Some(Self::VERSION_4) => (Self::VERSION_4, Self::LEN_4, 32),
            _ => {
                let version = version.map_or_else(|| "unreadable".to_owned(), |v| v.to_string());
                return Err(format!(
                    "the checkpoint's header has format version {version}, and this build reads versions {} to {} \
                     only: finish the search with the build that made it, or delete the checkpoint",
                    Self::VERSION_4,
                    Self::VERSION_6
                ));
            }
        };
        if bytes.len() != len {
            return Err(format!("the checkpoint's header has {} bytes, not {len}", bytes.len()));
        }
        let width = u32_at(8).expect("length checked") as usize;
        if width != W {
            return Err(format!("the checkpoint was made with Bits<{width}>, not Bits<{W}>"));
        }
        let u64_at = |i: usize| u64::from_le_bytes(bytes[i..i + 8].try_into().expect("8 bytes"));
        let one_sided = if version == Self::VERSION_4 { Self::digest_one_sided(&[]) } else { u64_at(32) };
        let bits = u16::from_le_bytes([bytes[rest + 4], bytes[rest + 5]]);
        let tactics = SearchTactics::from_bits(bits)?;
        let tactics = if version == Self::VERSION_6 { tactics.with_tuning(&bytes[Self::LEN..])? } else { tactics };
        Ok(Header {
            n: u32_at(12).expect("length checked"),
            candidates: u64_at(16),
            frames: u64_at(24),
            one_sided,
            max_total: u32_at(rest).expect("length checked"),
            tactics,
            matching: MatchingPath::of_bits(bits),
            version,
        })
    }

    /// Why a checkpoint with this header cannot resume a search with one-sided candidates, if it cannot: a header of
    /// version 4 was written by a build before the extended problem's search, so for a search without them.
    pub(super) fn one_sided_refusal(&self) -> Option<String> {
        (self.version == Self::VERSION_4).then(|| {
            "the checkpoint is of format version 4, made by a build before the search of the extended problem, so of \
             a search without one-sided candidates: it cannot resume one with them (open sources)"
                .into()
        })
    }

    /// What differs between a checkpoint's header (`self`) and the one the run expects, if anything; for a run with
    /// one-sided candidates, [`one_sided_refusal`](Self::one_sided_refusal) first.  A header of the run's game whose
    /// search used the matching path ([`MatchingPath`]) differs from every header of this build by that.
    pub(super) fn mismatch(&self, expected: &Self) -> Option<String> {
        if expected.one_sided != Self::digest_one_sided(&[]) {
            if let Some(refusal) = self.one_sided_refusal() {
                return Some(refusal);
            }
        }
        let what = if self.n != expected.n {
            format!("the game n={}, not n={}", self.n, expected.n)
        } else if self.candidates != expected.candidates {
            "other promotion candidates".into()
        } else if self.frames != expected.frames {
            "other frames (made by a build with another frame builder?)".into()
        } else if self.one_sided != expected.one_sided {
            "other one-sided candidates (another extended problem)".into()
        } else if self.max_total != expected.max_total {
            format!("max_total {}, not {}", self.max_total, expected.max_total)
        } else if self.matching != expected.matching {
            format!(
                "a search on the matching path ({}), which this build no longer has",
                self.matching.tactics().join(", ")
            )
        } else if !self.tactics.same_recorded(&expected.tactics) {
            format!("the search tactics {:?}, not {:?}", self.tactics, expected.tactics)
        } else {
            return None;
        };
        Some(format!("the checkpoint is of {what}"))
    }
}
