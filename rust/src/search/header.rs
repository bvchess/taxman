//! The search's part of a checkpoint: its header ([`Header`]), which identifies the game and the search so that a
//! checkpoint is resumed only by the search that made it.

use super::searcher::put_bits;
use super::setup::Prepared;
use super::tactics::SearchTactics;
use crate::bits::Bits;
use crate::engine;
use crate::model::FrameSet;
use std::path::Path;

/// What a checkpoint records about the search it belongs to, so that a resumed run can check it is the same
/// search (the same game, frames, candidates and tactics) and skip revising frame 0 again.
///
/// Encoded (little-endian) as the magic `TXSH`, the format [`Header::VERSION`] (u32), the width `W` (u32),
/// `n` (u32), the digests of the candidates and of the frames (u64 each), whether there is a frame 0 (u8) and
/// its revised freedoms (i32), `max_total` (u32) and the tactics' bits (u16).  Version 2 (magic `TXS2`, made
/// before 2026-10-12) recorded neither `n` nor the frames, so a checkpoint of another game with the same
/// candidates passed for this one; it is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Header<const W: usize> {
    pub(super) n: u32,
    /// [`Header::digest_candidates`] of the root candidates.
    pub(super) candidates: u64,
    /// [`Header::digest_frames`] of the frames as built (before revising frame 0).
    pub(super) frames: u64,
    /// Frame 0's freedoms in the search's root (`None` without frames).
    pub(super) frame0_freedoms: Option<i32>,
    pub(super) max_total: u32,
    pub(super) tactics: SearchTactics,
}

impl<const W: usize> Header<W> {
    pub(super) const MAGIC: &'static [u8; 4] = b"TXSH";
    pub(super) const VERSION: u32 = 3;
    pub(super) const LEN: usize = 4 + 4 + 4 + 4 + 8 + 8 + 1 + 4 + 4 + 2;

    pub(super) fn of(prepared: &Prepared<W>) -> Self {
        Header {
            n: prepared.n,
            candidates: Self::digest_candidates(&prepared.candidates),
            frames: Self::digest_frames(&prepared.unrevised),
            frame0_freedoms: prepared.root.frames().first().map(|f| f.freedoms),
            max_total: prepared.max_total,
            tactics: prepared.tactics,
        }
    }

    pub(super) fn digest_candidates(candidates: &Bits<W>) -> u64 {
        let mut bytes = Vec::with_capacity(8 * W);
        put_bits(&mut bytes, candidates);
        engine::checksum(&bytes)
    }

    /// A digest of every frame's moves, factors and freedoms, bottom first.
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
        out.push(u8::from(self.frame0_freedoms.is_some()));
        out.extend_from_slice(&self.frame0_freedoms.unwrap_or(0).to_le_bytes());
        out.extend_from_slice(&self.max_total.to_le_bytes());
        out.extend_from_slice(&self.tactics.bits().to_le_bytes());
        out
    }

    pub(super) fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.starts_with(b"TXS2") {
            return Err("the checkpoint was made by an older build, whose checkpoints do not record the board size \
                        or the frames, so it cannot be checked against this game; finish it with that build, or \
                        delete it"
                .into());
        }
        if !bytes.starts_with(Self::MAGIC) {
            return Err("not a checkpoint of a Taxman search".into());
        }
        let u32_at = |i: usize| bytes.get(i..i + 4).map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")));
        let version = u32_at(4).unwrap_or(0);
        if version != Self::VERSION {
            return Err(format!("the checkpoint's header has format version {version}, not {}", Self::VERSION));
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
            frame0_freedoms: (bytes[32] != 0).then(|| u32_at(33).expect("length checked") as i32),
            max_total: u32_at(37).expect("length checked"),
            tactics: SearchTactics::from_bits(u16::from_le_bytes([bytes[41], bytes[42]]))?,
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
        } else if self.frame0_freedoms != expected.frame0_freedoms {
            format!("frame 0's freedoms {:?}, not {:?}", self.frame0_freedoms, expected.frame0_freedoms)
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

/// The set width `W` a checkpoint of a search was made with, if `path` holds one whose header this build can read
/// (an older build may have chosen another width for the game than this one does).
pub fn checkpoint_width(path: &Path) -> Option<usize> {
    let bytes = engine::read_checkpoint_header(path).ok()?;
    let u32_at = |i: usize| bytes.get(i..i + 4).map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")));
    let ours = bytes.starts_with(Header::<1>::MAGIC) && u32_at(4) == Some(Header::<1>::VERSION);
    ours.then(|| u32_at(8)).flatten().map(|width| width as usize)
}
