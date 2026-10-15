//! The search's part of a checkpoint: its header ([`Header`]), which identifies the game and the search so that a
//! checkpoint is resumed only by the search that made it.

use super::searcher::put_bits;
use super::setup::Prepared;
use super::tactics::SearchTactics;
use crate::bits::Bits;
use crate::engine;
use crate::model::FrameSet;

/// What a checkpoint records about the search it belongs to, so that a resumed run can check it is the same
/// search (the same game, frames, candidates and tactics).
///
/// Encoded (little-endian) as the magic `TXSH`, the format [`Header::VERSION`] (u32), the width `W` (u32),
/// `n` (u32), the digests of the candidates and of the frames (u64 each), `max_total` (u32) and the tactics' bits
/// (u16, [`SearchTactics::bits`]).  This build reads version 4 only: the headers of older builds (magic `TXS2`, or
/// `TXSH` with versions 3 and below) recorded tactics that are gone, and are refused with one message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Header<const W: usize> {
    pub(super) n: u32,
    /// [`Header::digest_candidates`] of the root candidates.
    pub(super) candidates: u64,
    /// [`Header::digest_frames`] of the frames as built.
    pub(super) frames: u64,
    pub(super) max_total: u32,
    pub(super) tactics: SearchTactics,
}

impl<const W: usize> Header<W> {
    pub(super) const MAGIC: &'static [u8; 4] = b"TXSH";
    pub(super) const VERSION: u32 = 4;
    pub(super) const LEN: usize = 4 + 4 + 4 + 4 + 8 + 8 + 4 + 2;

    pub(super) fn of(prepared: &Prepared<W>) -> Self {
        Header {
            n: prepared.n,
            candidates: Self::digest_candidates(&prepared.candidates),
            frames: Self::digest_frames(&prepared.root),
            max_total: prepared.max_total,
            tactics: prepared.tactics,
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

    pub(super) fn encode(&self) -> Vec<u8> {
        let mut out = Self::MAGIC.to_vec();
        for word in [Self::VERSION, W as u32, self.n] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out.extend_from_slice(&self.candidates.to_le_bytes());
        out.extend_from_slice(&self.frames.to_le_bytes());
        out.extend_from_slice(&self.max_total.to_le_bytes());
        out.extend_from_slice(&self.tactics.bits().to_le_bytes());
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
        if version != Some(Self::VERSION) {
            let version = version.map_or_else(|| "unreadable".to_owned(), |v| v.to_string());
            return Err(format!(
                "the checkpoint's header has format version {version}, and this build reads version {} only: \
                 finish the search with the build that made it, or delete the checkpoint",
                Self::VERSION
            ));
        }
        if bytes.len() != Self::LEN {
            return Err(format!("the checkpoint's header has {} bytes, not {}", bytes.len(), Self::LEN));
        }
        let width = u32_at(8).expect("length checked") as usize;
        if width != W {
            return Err(format!("the checkpoint was made with Bits<{width}>, not Bits<{W}>"));
        }
        let u64_at = |i: usize| u64::from_le_bytes(bytes[i..i + 8].try_into().expect("8 bytes"));
        Ok(Header {
            n: u32_at(12).expect("length checked"),
            candidates: u64_at(16),
            frames: u64_at(24),
            max_total: u32_at(32).expect("length checked"),
            tactics: SearchTactics::from_bits(u16::from_le_bytes([bytes[36], bytes[37]]))?,
        })
    }

    /// What differs between a checkpoint's header (`self`) and the one the run expects, if anything.
    pub(super) fn mismatch(&self, expected: &Self) -> Option<String> {
        let what = if self.n != expected.n {
            format!("the game n={}, not n={}", self.n, expected.n)
        } else if self.candidates != expected.candidates {
            "other promotion candidates".into()
        } else if self.frames != expected.frames {
            "other frames (made by a build with another frame builder?)".into()
        } else if self.max_total != expected.max_total {
            format!("max_total {}, not {}", self.max_total, expected.max_total)
        } else if self.tactics != expected.tactics {
            format!("the search tactics {:?}, not {:?}", self.tactics, expected.tactics)
        } else {
            return None;
        };
        Some(format!("the checkpoint is of {what}"))
    }
}
