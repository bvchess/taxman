//! Deflate (RFC 1951): the blocks of one stream, stored, fixed-Huffman and dynamic-Huffman, decoded through a lookup
//! table for each code.

use crate::reader::Reader;
use crate::Error;

/// The longest code, in bits.
const MAX_BITS: usize = 15;

/// The order in which a dynamic block lists the lengths of the codes that spell its own code lengths (3.2.7).
const CODE_LENGTH_ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

/// For each length code 257..=285, the shortest length it stands for and the number of extra bits that add to it.
const LENGTH_BASE: [u16; 29] =
    [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258];
const LENGTH_EXTRA: [u32; 29] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0];

/// The same for the distance codes 0..=29.
const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145,
    8193, 12289, 16385, 24577,
];
const DISTANCE_EXTRA: [u32; 30] =
    [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13];

/// A Huffman code as a table with an entry for each value of the next `bits` bits of the input, `bits` being the length
/// of the longest code.  An entry is a symbol and the length of its code, `symbol << 4 | length`, in every place whose
/// low bits are that code (a code is read from its first bit, the highest of the code); it is 0 in the places that no
/// code starts, so that a symbol decodes in one lookup.
struct Code {
    table: Vec<u16>,
    bits: u32,
}

impl Code {
    /// The canonical code (3.2.2) of these code lengths, 0 for a symbol that has none.  A set of lengths that needs
    /// more codes than there are is over-subscribed, and one that leaves some unused is incomplete.  The first is an
    /// error.  So is the second, except for a code of no symbol or of one symbol of length 1, which a block with no
    /// matches or one distance has (3.2.7).
    fn new(lengths: &[u8]) -> Result<Code, &'static str> {
        let mut count = [0usize; MAX_BITS + 1];
        for &length in lengths {
            count[length as usize] += 1;
        }
        count[0] = 0;
        let (mut next, mut first, mut room) = ([0usize; MAX_BITS + 1], 0, 1isize);
        for length in 1..=MAX_BITS {
            first = (first + count[length - 1]) << 1;
            next[length] = first;
            room = (room << 1) - count[length] as isize;
            if room < 0 {
                return Err("over-subscribed code");
            }
        }
        let symbols: usize = count.iter().sum();
        if room > 0 && symbols != 0 && !(symbols == 1 && count[1] == 1) {
            return Err("incomplete code");
        }
        let bits = (0..=MAX_BITS).rev().find(|&length| count[length] > 0).unwrap_or(0) as u32;
        let mut table = vec![0u16; 1 << bits];
        for (symbol, &length) in lengths.iter().enumerate().filter(|&(_, &length)| length > 0) {
            let code = next[length as usize] as u32;
            next[length as usize] += 1;
            // The table is indexed by the bits as they come, the first of the code lowest.
            let first_slot = (code.reverse_bits() >> (32 - length)) as usize;
            for slot in (first_slot..table.len()).step_by(1 << length) {
                table[slot] = (symbol as u16) << 4 | u16::from(length);
            }
        }
        Ok(Code { table, bits })
    }

    /// The code of the fixed-Huffman blocks (3.2.6): the literals and lengths, and the distances.
    fn fixed() -> (Code, Code) {
        let mut lengths = [8u8; 288];
        lengths[144..256].fill(9);
        lengths[256..280].fill(7);
        (Code::new(&lengths).unwrap(), Code::new(&[5; 32]).unwrap())
    }

    /// The next symbol of the input.
    fn decode(&self, reader: &mut Reader) -> Result<usize, Error> {
        let entry = self.table[reader.peek(self.bits) as usize];
        if entry == 0 {
            return Err(reader.fail("invalid code"));
        }
        reader.skip(u32::from(entry & 15))?;
        Ok(usize::from(entry >> 4))
    }
}

/// Appends the bytes of the deflate stream at the reader to `out`, to the end of its last block, which a match may not
/// reach back from past `out[start]`.
pub(crate) fn inflate(reader: &mut Reader, out: &mut Vec<u8>, start: usize) -> Result<(), Error> {
    loop {
        let last = reader.bits(1)? == 1;
        match reader.bits(2)? {
            0 => stored(reader, out)?,
            1 => {
                let (literals, distances) = Code::fixed();
                block(reader, out, start, &literals, &distances)?;
            }
            2 => {
                let (literals, distances) = read_codes(reader)?;
                block(reader, out, start, &literals, &distances)?;
            }
            _ => return Err(reader.fail("invalid block type 3")),
        }
        if last {
            return Ok(());
        }
    }
}

/// A stored block (3.2.4): its length, the complement of its length, and the bytes.
fn stored(reader: &mut Reader, out: &mut Vec<u8>) -> Result<(), Error> {
    let length = reader.u16()?;
    if reader.u16()? != !length {
        return Err(reader.fail(format!("stored block of {length} bytes: its length and the complement of it differ")));
    }
    out.extend_from_slice(reader.bytes(length as usize)?);
    Ok(())
}

/// The codes at the start of a dynamic block (3.2.7): the literal and length code, and the distance code.
fn read_codes(reader: &mut Reader) -> Result<(Code, Code), Error> {
    let literals = reader.bits(5)? as usize + 257;
    let distances = reader.bits(5)? as usize + 1;
    let spelling = reader.bits(4)? as usize + 4;
    if literals > 286 || distances > 30 {
        return Err(reader.fail(format!("too many codes: {literals} literal and length, {distances} distance")));
    }
    let mut lengths = [0u8; 19];
    for &symbol in &CODE_LENGTH_ORDER[..spelling] {
        lengths[symbol] = reader.bits(3)? as u8;
    }
    let spelling = Code::new(&lengths).map_err(|message| reader.fail(format!("{message} for code lengths")))?;
    // The lengths of both codes are one sequence, in which a repeat may run from one code into the other.
    let mut lengths = [0u8; 286 + 30];
    let mut filled = 0;
    while filled < literals + distances {
        let (value, repeat) = match spelling.decode(reader)? {
            symbol @ 0..=15 => (symbol as u8, 1),
            16 if filled == 0 => return Err(reader.fail("a code length repeats the one before it, and none is")),
            16 => (lengths[filled - 1], 3 + reader.bits(2)? as usize),
            17 => (0, 3 + reader.bits(3)? as usize),
            _ => (0, 11 + reader.bits(7)? as usize),
        };
        if filled + repeat > literals + distances {
            return Err(
                reader.fail(format!("the code lengths go past the {} codes of the block", literals + distances))
            );
        }
        lengths[filled..filled + repeat].fill(value);
        filled += repeat;
    }
    if lengths[256] == 0 {
        return Err(reader.fail("a block with no code for its end"));
    }
    let code =
        |lengths: &[u8], what: &str| Code::new(lengths).map_err(|message| reader.fail(format!("{message} for {what}")));
    Ok((
        code(&lengths[..literals], "literals and lengths")?,
        code(&lengths[literals..literals + distances], "distances")?,
    ))
}

/// The symbols of a block, to its end code.
fn block(reader: &mut Reader, out: &mut Vec<u8>, start: usize, literals: &Code, distances: &Code) -> Result<(), Error> {
    loop {
        let symbol = literals.decode(reader)?;
        if symbol < 256 {
            out.push(symbol as u8);
            continue;
        }
        if symbol == 256 {
            return Ok(());
        }
        let index = symbol - 257;
        if index >= LENGTH_BASE.len() {
            return Err(reader.fail(format!("invalid length code {symbol}")));
        }
        let length = usize::from(LENGTH_BASE[index]) + reader.bits(LENGTH_EXTRA[index])? as usize;
        let index = distances.decode(reader)?;
        if index >= DISTANCE_BASE.len() {
            return Err(reader.fail(format!("invalid distance code {index}")));
        }
        let distance = usize::from(DISTANCE_BASE[index]) + reader.bits(DISTANCE_EXTRA[index])? as usize;
        if distance > out.len() - start {
            let message = format!(
                "a match reaches {distance} bytes back, past the start of the {} bytes so far",
                out.len() - start
            );
            return Err(reader.fail(message));
        }
        let from = out.len() - distance;
        if distance >= length {
            out.extend_from_within(from..from + length);
        } else {
            // The match overlaps what it copies: the bytes it adds repeat the last `distance` ones.
            for i in from..from + length {
                out.push(out[i]);
            }
        }
    }
}
