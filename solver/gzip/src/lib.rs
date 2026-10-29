//! A gzip decoder (RFC 1952, with deflate, RFC 1951) that needs nothing outside the standard library.  The programs of
//! the project read their results file with it, a gzip file of several members, one after another.
//!
//! * [`decode`] gives the bytes of every member, one after another.
//! * [`Members`] gives them a member at a time, so that a reader can stop at the member it wants.
//! * Every member's CRC-32 and length are checked, and so is its header's CRC-16 if it has one, so that a fault in the
//!   decoder or the input is an [`Error`] and never wrong bytes.
//! * The input must be members and nothing else.  Bytes after the last member that do not start a member are an error,
//!   and so is an empty input.
//!
//! The decoder puts no limit on the size of what it decodes, which deflate can make a thousand times the input's.

mod inflate;
mod reader;

use reader::Reader;
use std::fmt;

// The flag bits of a gzip header (RFC 1952, 2.3.1): a header CRC-16, an extra field, a file name, a comment.  The bits
// above them are reserved, and the lowest says the file is text, which changes nothing here.
const FHCRC: u8 = 2;
const FEXTRA: u8 = 4;
const FNAME: u8 = 8;
const FCOMMENT: u8 = 16;
const RESERVED: u8 = 0xE0;

/// What went wrong in decoding, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    /// The index of the member in which it went wrong, 0 for the first (for bytes after the last member that are not a
    /// member, the index the member would have had).
    pub member: usize,
    /// The offset in the input, in bytes, of the place where it was found (the end of the input, if it ended too soon).
    pub offset: usize,
    /// What is wrong, in words.
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "gzip member {}, byte {}: {}", self.member, self.offset, self.message)
    }
}

impl std::error::Error for Error {}

/// The bytes of every member of `bytes`, one after another.
pub fn decode(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let mut members = Members::new(bytes);
    let mut out = Vec::new();
    while let Some(result) = members.append_next(&mut out) {
        result?;
    }
    Ok(out)
}

/// The members of a gzip file, decoded one at a time: the bytes of each, in order.  After an error it gives nothing
/// more, and neither does it after the last member.
pub struct Members<'a> {
    bytes: &'a [u8],
    /// Where the next member starts.
    offset: usize,
    /// The index of the next member.
    index: usize,
    done: bool,
}

impl<'a> Members<'a> {
    /// The members of `bytes`, before the first is decoded.
    pub fn new(bytes: &'a [u8]) -> Self {
        Members { bytes, offset: 0, index: 0, done: false }
    }

    /// Decodes the next member and appends its bytes to `out`, or says there is none (`None`).  What it appends before
    /// an error is not a member's bytes.
    fn append_next(&mut self, out: &mut Vec<u8>) -> Option<Result<(), Error>> {
        if self.done || (self.index > 0 && self.offset == self.bytes.len()) {
            self.done = true;
            return None;
        }
        let mut reader = Reader::new(self.bytes, self.offset, self.index);
        let result = read_member(&mut reader, out);
        match result {
            Ok(()) => (self.offset, self.index) = (reader.offset(), self.index + 1),
            Err(_) => self.done = true,
        }
        Some(result)
    }
}

impl Iterator for Members<'_> {
    type Item = Result<Vec<u8>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut out = Vec::new();
        Some(self.append_next(&mut out)?.map(|()| out))
    }
}

/// Reads one member at the reader, header to trailer, and appends its bytes to `out`.
fn read_member(reader: &mut Reader, out: &mut Vec<u8>) -> Result<(), Error> {
    let start = out.len();
    read_header(reader)?;
    inflate::inflate(reader, out, start)?;
    reader.align();
    let trailer = reader.offset();
    let (crc, size) = (reader.u32()?, reader.u32()?);
    let decoded = &out[start..];
    let found = crc32(decoded);
    if crc != found {
        let message = format!("CRC-32 mismatch: the member says {crc:08x}, its bytes have {found:08x}");
        return Err(Error { member: reader.member(), offset: trailer, message });
    }
    if size != decoded.len() as u32 {
        let message =
            format!("ISIZE mismatch: the member says {size} bytes, it has {} (modulo 2^32)", decoded.len() as u32);
        return Err(Error { member: reader.member(), offset: trailer + 4, message });
    }
    Ok(())
}

/// Reads and checks a member's header (RFC 1952, 2.3), and leaves the reader at the deflate stream after it.
fn read_header(reader: &mut Reader) -> Result<(), Error> {
    let start = reader.offset();
    if !reader.rest().starts_with(&[0x1f, 0x8b]) {
        return Err(reader.fail(if reader.member() == 0 {
            "bad magic: not a gzip file"
        } else {
            "bad magic: the bytes after the last member are not a member"
        }));
    }
    let fixed = reader.bytes(10)?;
    if fixed[2] != 8 {
        return Err(reader.fail(format!("compression method {} is not 8, deflate", fixed[2])));
    }
    let flags = fixed[3];
    if flags & RESERVED != 0 {
        return Err(reader.fail(format!("reserved flag bits set in {flags:#04x}")));
    }
    if flags & FEXTRA != 0 {
        let length = reader.u16()?;
        reader.bytes(usize::from(length))?;
    }
    for flag in [FNAME, FCOMMENT] {
        if flags & flag != 0 {
            reader.zero_terminated()?;
        }
    }
    if flags & FHCRC != 0 {
        // The header's CRC-16 is the low half of the CRC-32 of the header before it.
        let header = reader.since(start);
        let (found, said) = (crc32(header) as u16, reader.u16()?);
        if said != found {
            return Err(reader.fail(format!("header CRC-16 mismatch: the header says {said:04x}, it has {found:04x}")));
        }
    }
    Ok(())
}

/// The tables of the CRC-32 (the polynomial 0xEDB88320 of IEEE 802.3, reflected).  Table 0 gives the CRC of a byte, and
/// table k the CRC of a byte followed by k zero bytes, so that eight bytes take eight lookups, none waiting for
/// another.
const CRC_TABLES: [[u32; 256]; 8] = {
    let mut tables = [[0; 256]; 8];
    let mut n = 0;
    while n < 256 {
        let mut crc = n as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 == 1 { 0xEDB8_8320 ^ (crc >> 1) } else { crc >> 1 };
            bit += 1;
        }
        tables[0][n] = crc;
        n += 1;
    }
    let mut table = 1;
    while table < 8 {
        n = 0;
        while n < 256 {
            let before = tables[table - 1][n];
            tables[table][n] = (before >> 8) ^ tables[0][(before & 0xFF) as usize];
            n += 1;
        }
        table += 1;
    }
    tables
};

/// The CRC-32 of `bytes`, as gzip has it.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    let (words, rest) = bytes.as_chunks::<8>();
    for &word in words {
        let word = u64::from_le_bytes(word) ^ u64::from(crc);
        crc = (0..8).fold(0, |crc, i| crc ^ CRC_TABLES[7 - i][(word >> (8 * i)) as usize & 0xFF]);
    }
    for &byte in rest {
        crc = CRC_TABLES[0][usize::from(crc as u8 ^ byte)] ^ (crc >> 8);
    }
    !crc
}
