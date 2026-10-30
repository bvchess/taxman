//! Tests of the gzip decoder.  The files are made by Python's gzip and zlib (their calls are in the comments), or are
//! written by hand: a deflate stream bit by bit, with a header and a trailer around it, so that the streams no
//! compressor makes can be tried (a match at the greatest distance) and so can the errors of a damaged one.

use taxman_gzip::{crc32, decode, Error, Members};

// --- files made by Python ------------------------------------------------------------------------------------------

/// `gzip.compress(b"", 9, mtime=0)`: a member with no bytes in it.
const EMPTY: &[u8] = &[
    0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00,
];
const EMPTY_PLAIN: &[u8] = b"";

/// `gzip.compress(STORED_PLAIN, 0, mtime=0)`: level 0 stores the bytes as they are.
const STORED: &[u8] = &[
    0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x01, 0x28, 0x00, 0xd7, 0xff, 0x73, 0x74, 0x6f, 0x72,
    0x65, 0x64, 0x3a, 0x20, 0x74, 0x68, 0x65, 0x73, 0x65, 0x20, 0x62, 0x79, 0x74, 0x65, 0x73, 0x20, 0x61, 0x72, 0x65,
    0x20, 0x6b, 0x65, 0x70, 0x74, 0x20, 0x61, 0x73, 0x20, 0x74, 0x68, 0x65, 0x79, 0x20, 0x61, 0x72, 0x65, 0xb5, 0x1a,
    0x15, 0xc8, 0x28, 0x00, 0x00, 0x00,
];
const STORED_PLAIN: &[u8] = b"stored: these bytes are kept as they are";

/// `gzip.compress(FIXED_PLAIN, 9, mtime=0)`: a short text, which zlib codes with the fixed Huffman code.
const FIXED: &[u8] = &[
    0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0x4b, 0x4c, 0x2a, 0x4a, 0x4c, 0x4e, 0x4c, 0x49, 0x04,
    0x52, 0x0a, 0x89, 0x08, 0x36, 0x00, 0x4e, 0x0e, 0x10, 0x05, 0x17, 0x00, 0x00, 0x00,
];
const FIXED_PLAIN: &[u8] = b"abracadabra abracadabra";

/// `gzip.compress(DYNAMIC_PLAIN, 9, mtime=0)`: a longer text, with a code of its own.
const DYNAMIC: &[u8] = &[
    0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0xad, 0x8c, 0x57, 0x15, 0x80, 0x30, 0x0c, 0x45, 0xad,
    0x3c, 0x05, 0x18, 0x40, 0x4d, 0x0b, 0xe9, 0x60, 0x34, 0xdd, 0x05, 0xd4, 0x93, 0x83, 0x06, 0xbe, 0xef, 0xa8, 0x8e,
    0x90, 0x9a, 0x5f, 0x76, 0xe8, 0xcc, 0x23, 0xc0, 0xf0, 0x85, 0xad, 0x9d, 0xb1, 0x80, 0x3b, 0x65, 0x54, 0xc1, 0x87,
    0x7a, 0x6e, 0xac, 0x6c, 0x67, 0x44, 0x25, 0xde, 0x79, 0x43, 0x8b, 0x34, 0x7c, 0x75, 0x30, 0xbe, 0x93, 0xa0, 0x87,
    0x02, 0x0e, 0x9f, 0x1a, 0x67, 0x69, 0x6d, 0x99, 0xbe, 0xec, 0xff, 0xeb, 0x0b, 0xbc, 0xd9, 0x63, 0x8e, 0xac, 0x00,
    0x00, 0x00,
];
const PANGRAMS: &str = "the quick brown fox jumps over the lazy dog; pack my box with five dozen liquor jugs. ";

fn dynamic_plain() -> Vec<u8> {
    PANGRAMS.repeat(2).into_bytes()
}

/// The lines `{"n":i,"score":i*i*7919 % 1000}` for i in 0..60 (see `blocks_plain`), as a raw deflate stream from
/// `zlib.compressobj(9, zlib.DEFLATED, -15, 1)` with a header and a trailer written around it.  The memory level 1
/// makes zlib end a block every 127 symbols, so the stream has many blocks.
const BLOCKS: &[u8] = &[
    0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x4c, 0x90, 0xc1, 0x0e, 0xc2, 0x30, 0x0c, 0x43, 0xef,
    0x7c, 0xc6, 0xce, 0x1c, 0xe6, 0xa4, 0x6d, 0x16, 0x7e, 0x07, 0xed, 0x0a, 0x12, 0x1c, 0x27, 0xfe, 0x1d, 0x90, 0x1a,
    0x7b, 0xb7, 0xea, 0xc9, 0xb2, 0xf3, 0x7a, 0x2c, 0x8f, 0xe5, 0xb6, 0x5e, 0x97, 0xf7, 0xfd, 0xf9, 0xda, 0x7f, 0xaf,
    0xcf, 0xe5, 0xf8, 0x13, 0x90, 0x24, 0x72, 0x32, 0x23, 0x1b, 0x31, 0x26, 0x73, 0x32, 0x0b, 0x4c, 0xd6, 0xc8, 0x62,
    0x6d, 0x93, 0x75, 0xf5, 0x45, 0x9f, 0x6c, 0x90, 0x6d, 0x15, 0x0b, 0x22, 0xaf, 0xb6, 0x4d, 0x29, 0xd4, 0x6a, 0x92,
    0x35, 0xaf, 0xeb, 0x20, 0x89, 0x5c, 0xa9, 0x21, 0x0f, 0x24, 0x93, 0x12, 0x71, 0xaf, 0x4a, 0xf8, 0x69, 0xba, 0xb6,
    0x21, 0x15, 0x58, 0xdd, 0x08, 0xb9, 0x04, 0x5d, 0x20, 0x19, 0x1b, 0x4c, 0x4a, 0xa7, 0x27, 0x3b, 0x25, 0x14, 0x9d,
    0xeb, 0x79, 0x82, 0xfc, 0x6f, 0x19, 0x0d, 0x1a, 0x19, 0x4e, 0x3f, 0xce, 0xa4, 0x8c, 0x22, 0xab, 0xd3, 0x64, 0x84,
    0x5e, 0xeb, 0x26, 0x23, 0x6f, 0x75, 0xe7, 0xb7, 0x8d, 0x3a, 0xc9, 0x81, 0x18, 0x84, 0x81, 0x28, 0x7a, 0x26, 0x70,
    0x39, 0x21, 0xf7, 0xbf, 0x58, 0xab, 0x17, 0xae, 0x5f, 0x0b, 0xb6, 0x08, 0xd9, 0x3c, 0x0f, 0x6c, 0x44, 0x65, 0xd1,
    0x0e, 0x11, 0x37, 0x11, 0x7d, 0xc4, 0x44, 0x24, 0xb2, 0x23, 0x3a, 0x7e, 0x67, 0x21, 0x5a, 0x16, 0x55, 0xf4, 0xc8,
    0xf6, 0x42, 0xe4, 0x22, 0x55, 0x85, 0x72, 0x92, 0x57, 0x80, 0x5c, 0xf8, 0xba, 0xb5, 0xa8, 0x12, 0x34, 0x67, 0x78,
    0x96, 0xbb, 0x5e, 0x27, 0x42, 0xce, 0x59, 0x34, 0xc8, 0x83, 0x24, 0x38, 0x32, 0x47, 0x70, 0x8e, 0x87, 0x53, 0x3b,
    0xf2, 0x4c, 0x4c, 0xc5, 0xf2, 0x78, 0xdc, 0x15, 0x23, 0xe7, 0xb5, 0xd0, 0x6d, 0x7d, 0xf4, 0xc4, 0x78, 0xf8, 0x26,
    0x20, 0xf6, 0x31, 0x3c, 0x5e, 0x5c, 0x01, 0x6a, 0x6f, 0x78, 0x03, 0x6a, 0x83, 0x7a, 0xc5, 0x66, 0xf8, 0xe6, 0x8e,
    0x17, 0x4d, 0xcc, 0x06, 0x24, 0x67, 0x6f, 0x40, 0x87, 0xef, 0xe0, 0x06, 0x6a, 0x40, 0x9f, 0xe9, 0xfd, 0x46, 0x39,
    0x1d, 0xf3, 0xc4, 0xe3, 0x9d, 0x1d, 0xd1, 0xbf, 0xee, 0x3f, 0x2c, 0x01, 0x31, 0x28, 0xd9, 0x04, 0x00, 0x00,
];

fn blocks_plain() -> Vec<u8> {
    (0..60).map(|i| format!("{{\"n\":{i},\"score\":{}}}\n", i * i * 7919 % 1000)).collect::<String>().into_bytes()
}

/// A raw deflate stream (`zlib.compressobj(9, zlib.DEFLATED, -15)` of RAW_PLAIN, one dynamic block), for the headers.
const RAW: &[u8] = &[
    0x85, 0xcc, 0xc1, 0x0d, 0x80, 0x30, 0x0c, 0x03, 0xc0, 0x55, 0x3c, 0x00, 0x1b, 0x30, 0x8d, 0xa5, 0xb8, 0x6a, 0xa5,
    0xd0, 0xa2, 0x34, 0x50, 0xb1, 0x3d, 0x6c, 0xc0, 0xf7, 0x1e, 0x47, 0x54, 0xd1, 0x14, 0x58, 0x2d, 0x2b, 0x74, 0x2b,
    0x1e, 0x8c, 0x33, 0xdb, 0xe8, 0x74, 0x94, 0x26, 0xb7, 0x0d, 0x8c, 0x71, 0x75, 0x03, 0x11, 0x5c, 0x30, 0x15, 0x67,
    0x0a, 0x33, 0x43, 0x3c, 0xf6, 0x4f, 0xff, 0x82, 0x17,
];
const RAW_PLAIN: &[u8] =
    b"a header with every optional field, around a raw deflate stream; a header with every optional field";

/// Every file above: its name, its bytes, and what it decodes to.
fn files() -> Vec<(&'static str, &'static [u8], Vec<u8>)> {
    vec![
        ("empty", EMPTY, EMPTY_PLAIN.to_vec()),
        ("stored", STORED, STORED_PLAIN.to_vec()),
        ("fixed", FIXED, FIXED_PLAIN.to_vec()),
        ("dynamic", DYNAMIC, dynamic_plain()),
        ("blocks", BLOCKS, blocks_plain()),
    ]
}

/// The type of the first block of a gzip file made by Python (its header has 10 bytes): 0 stored, 1 fixed, 2 dynamic.
fn block_type(gzip: &[u8]) -> u8 {
    (gzip[10] >> 1) & 3
}

// --- files written by hand -----------------------------------------------------------------------------------------

const FHCRC: u8 = 2;
const FEXTRA: u8 = 4;
const FNAME: u8 = 8;
const FCOMMENT: u8 = 16;

/// The CRC-32 a bit at a time, to check the crate's table against and to write the checksums of the files by hand.
fn crc32_bitwise(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 { 0xEDB8_8320 ^ (crc >> 1) } else { crc >> 1 };
        }
    }
    !crc
}

/// A gzip header (RFC 1952, 2.3) with the fields these flags say, and the header's CRC-16 if FHCRC is among them.
fn header(flags: u8) -> Vec<u8> {
    let mut header = vec![0x1f, 0x8b, 8, flags, 1, 2, 3, 4, 0, 3];
    if flags & FEXTRA != 0 {
        header.extend([6, 0]);
        header.extend(b"AB\x02\x00hi");
    }
    if flags & FNAME != 0 {
        header.extend(b"strict.jsonl\0");
    }
    if flags & FCOMMENT != 0 {
        header.extend(b"a comment\0");
    }
    if flags & FHCRC != 0 {
        header.extend((crc32_bitwise(&header) as u16).to_le_bytes());
    }
    header
}

/// A gzip member: `header`, the deflate stream `raw`, and the trailer that says it decodes to `plain`.
fn member(header: Vec<u8>, raw: &[u8], plain: &[u8]) -> Vec<u8> {
    [header.as_slice(), raw, &crc32_bitwise(plain).to_le_bytes(), &(plain.len() as u32).to_le_bytes()].concat()
}

/// A deflate stream written bit by bit: numbers go in lowest bit first, Huffman codes highest bit first (RFC 1951,
/// 3.1.1).
#[derive(Default)]
struct Bits {
    bytes: Vec<u8>,
    /// How many bits of the last byte are used, 0 if the next bit starts a byte.
    used: u32,
}

impl Bits {
    fn number(&mut self, value: u32, count: u32) {
        for i in 0..count {
            if self.used == 0 {
                self.bytes.push(0);
            }
            *self.bytes.last_mut().unwrap() |= (((value >> i) & 1) as u8) << self.used;
            self.used = (self.used + 1) % 8;
        }
    }

    /// A Huffman code, given as its value and its length in bits.
    fn code(&mut self, (code, length): (u32, u32)) {
        for i in (0..length).rev() {
            self.number((code >> i) & 1, 1);
        }
    }

    /// A stored block, whose length and bytes start on a byte boundary.
    fn stored(&mut self, last: bool, data: &[u8]) {
        self.number(u32::from(last), 1);
        self.number(0, 2);
        self.used = 0;
        let length = data.len() as u16;
        self.bytes.extend([length.to_le_bytes(), (!length).to_le_bytes()].concat());
        self.bytes.extend(data);
    }

    /// The start of a block of the fixed Huffman code.
    fn fixed(&mut self, last: bool) {
        self.number(u32::from(last), 1);
        self.number(1, 2);
    }
}

/// A literal, length or end symbol of the fixed Huffman code (RFC 1951, 3.2.6), as a code.
fn fixed_code(symbol: u32) -> (u32, u32) {
    match symbol {
        0..=143 => (0x30 + symbol, 8),
        144..=255 => (0x190 + symbol - 144, 9),
        256..=279 => (symbol - 256, 7),
        _ => (0xC0 + symbol - 280, 8),
    }
}

/// A match in a block of the fixed code: a length symbol that has no extra bits (257 for 3, 285 for 258), a distance
/// code (5 bits), and the extra bits of the distance as a number and a count.
fn fixed_match(bits: &mut Bits, length_symbol: u32, distance_code: u32, extra: (u32, u32)) {
    bits.code(fixed_code(length_symbol));
    bits.code((distance_code, 5));
    bits.number(extra.0, extra.1);
}

/// The canonical code (RFC 1951, 3.2.2) of each symbol that has a length: the codes of a length follow in the order of
/// the symbols, and the first of the next length comes after the last of this one, shifted left.
fn canonical(lengths: &[u8]) -> Vec<(u32, u32)> {
    let mut codes = vec![(0, 0); lengths.len()];
    let mut code = 0;
    for length in 1..=15 {
        for (symbol, _) in lengths.iter().enumerate().filter(|&(_, &l)| l == length) {
            codes[symbol] = (code, u32::from(length));
            code += 1;
        }
        code <<= 1;
    }
    codes
}

/// `size` code lengths, 0 but for these.
fn lengths(size: usize, used: &[(usize, u8)]) -> Vec<u8> {
    let mut lengths = vec![0; size];
    for &(symbol, length) in used {
        lengths[symbol] = length;
    }
    lengths
}

/// The start of a dynamic block (3.2.7) that has these code lengths.  It spells them with a code that gives each length
/// from 0 to 15 a code of 4 bits, the length itself, and the repeat symbols (16 to 18) none.
fn dynamic(bits: &mut Bits, last: bool, literals: &[u8], distances: &[u8]) {
    bits.number(u32::from(last), 1);
    bits.number(2, 2);
    bits.number(literals.len() as u32 - 257, 5);
    bits.number(distances.len() as u32 - 1, 5);
    bits.number(15, 4);
    for symbol in [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15] {
        bits.number(if symbol < 16 { 4 } else { 0 }, 3);
    }
    for &length in literals.iter().chain(distances) {
        bits.code((u32::from(length), 4));
    }
}

/// The start of a dynamic block with 257 literal and length codes and one distance code, up to the code that spells
/// their lengths (3.2.7): `spelling` has the lengths of that code, for the symbols 16, 17, 18, 0, 8, 7, and so on, as
/// many as the block gives.  The caller goes on with the lengths themselves.
fn dynamic_spelling(bits: &mut Bits, spelling: &[u32]) {
    bits.number(1, 1);
    bits.number(2, 2);
    bits.number(0, 5);
    bits.number(0, 5);
    bits.number(spelling.len() as u32 - 4, 4);
    for &length in spelling {
        bits.number(length, 3);
    }
}

/// SplitMix64, to make bytes and choose what to damage, the same every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Decodes `gzip`, which must fail in member `member` with `text` in its message; returns the error.
fn expect_error(gzip: &[u8], member: usize, text: &str) -> Error {
    let error = decode(gzip).expect_err(text);
    assert_eq!(error.member, member, "{error}");
    assert!(error.message.contains(text), "expected {text:?}: {error}");
    assert!(error.offset <= gzip.len(), "{error}");
    let shown = error.to_string();
    assert!(shown.contains(&format!("member {member}, byte {}", error.offset)), "{shown}");
    error
}

// --- what it decodes -----------------------------------------------------------------------------------------------

#[test]
fn the_files_have_the_blocks_they_are_for() {
    assert_eq!([block_type(STORED), block_type(FIXED), block_type(DYNAMIC), block_type(BLOCKS)], [0, 1, 2, 2]);
    assert_eq!(BLOCKS[10] & 1, 0, "the first block of the many is not the last");
}

#[test]
fn an_empty_member_decodes_to_nothing() {
    assert_eq!(decode(EMPTY).unwrap(), EMPTY_PLAIN);
}

#[test]
fn a_stored_block_decodes() {
    assert_eq!(decode(STORED).unwrap(), STORED_PLAIN);
}

#[test]
fn a_fixed_huffman_block_decodes() {
    assert_eq!(decode(FIXED).unwrap(), FIXED_PLAIN);
}

#[test]
fn a_dynamic_huffman_block_decodes() {
    assert_eq!(decode(DYNAMIC).unwrap(), dynamic_plain());
}

#[test]
fn a_stream_of_many_blocks_decodes() {
    assert_eq!(decode(BLOCKS).unwrap(), blocks_plain());
}

#[test]
fn several_members_decode_to_their_bytes_one_after_another() {
    let mut file = Vec::new();
    let mut plain = Vec::new();
    for (_, gzip, bytes) in files().into_iter().chain(files().into_iter().rev()) {
        file.extend(gzip);
        plain.extend(bytes);
    }
    assert_eq!(decode(&file).unwrap(), plain);
}

/// A stored block of `prefix` random bytes, then a fixed block that matches 258 bytes at distance 32,768 and ends the
/// stream, as a deflate stream and what it decodes to.  zlib, and so Python, stops 262 bytes short of the greatest
/// distance, so the stream is written by hand, but the format allows it.
fn match_at_the_greatest_distance(prefix: usize) -> (Vec<u8>, Vec<u8>) {
    let mut rng = Rng(7);
    let data: Vec<u8> = (0..prefix).map(|_| rng.next() as u8).collect();
    let mut bits = Bits::default();
    bits.stored(false, &data);
    bits.fixed(true);
    fixed_match(&mut bits, 285, 29, (32768 - 24577, 13)); // distance code 29 is 24,577 and 13 bits more
    bits.code(fixed_code(256));
    let plain = [data.clone(), data[..258].to_vec()].concat();
    (bits.bytes, plain)
}

#[test]
fn a_match_of_258_bytes_at_distance_32768_decodes() {
    let (raw, plain) = match_at_the_greatest_distance(32768);
    assert_eq!(plain.len(), 32768 + 258);
    assert_eq!(decode(&member(header(0), &raw, &plain)).unwrap(), plain);
}

#[test]
fn a_match_that_reaches_one_byte_past_the_start_is_an_error() {
    let (raw, plain) = match_at_the_greatest_distance(32767);
    let error = expect_error(&member(header(0), &raw, &plain), 0, "past the start");
    assert!(error.message.contains("32768") && error.message.contains("32767"), "{error}");
}

#[test]
fn a_match_may_not_reach_back_into_the_member_before() {
    let mut bits = Bits::default();
    bits.fixed(true);
    fixed_match(&mut bits, 257, 4, (0, 1)); // 3 bytes at distance 5 (distance code 4 is 5 and 1 bit more)
    bits.code(fixed_code(256));
    let first = member(header(0), STORED_RAW_TEN, b"0123456789");
    let second = member(header(0), &bits.bytes, b"012");
    let file = [first, second].concat();
    expect_error(&file, 1, "past the start");
    let mut members = Members::new(&file);
    assert_eq!(members.next().unwrap().unwrap(), b"0123456789");
    assert_eq!(members.next().unwrap().unwrap_err().member, 1);
    assert!(members.next().is_none());
}

/// A stored block of ten bytes, the last of its stream.
const STORED_RAW_TEN: &[u8] = &[1, 10, 0, 245, 255, b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9'];

#[test]
fn dynamic_blocks_with_codes_of_one_symbol_and_none_decode() {
    // Literals a and b, a length of 3 and the end, in 2 bits each; one distance code of 1 bit, for distance 2.
    let literals = lengths(258, &[(97, 2), (98, 2), (256, 2), (257, 2)]);
    let (codes, distance_codes) = (canonical(&literals), canonical(&[0, 1]));
    let mut bits = Bits::default();
    dynamic(&mut bits, false, &literals, &[0, 1]);
    for code in [codes[97], codes[98], codes[257], distance_codes[1], codes[256]] {
        bits.code(code);
    }
    // A block that matches nothing needs no distance code: its distance code has no symbol at all.
    let literals = lengths(257, &[(99, 1), (256, 1)]);
    let codes = canonical(&literals);
    dynamic(&mut bits, true, &literals, &[0]);
    for code in [codes[99], codes[99], codes[256]] {
        bits.code(code);
    }
    assert_eq!(decode(&member(header(0), &bits.bytes, b"ababacc")).unwrap(), b"ababacc");
}

#[test]
fn a_distance_code_that_a_one_symbol_code_lacks_is_an_error() {
    let literals = lengths(258, &[(97, 2), (98, 2), (256, 2), (257, 2)]);
    let codes = canonical(&literals);
    let mut bits = Bits::default();
    dynamic(&mut bits, true, &literals, &[0, 1]);
    for code in [codes[97], codes[98], codes[257]] {
        bits.code(code);
    }
    bits.number(1, 1); // the code of length 1 has the value 0, and nothing starts with a 1
    expect_error(&member(header(0), &bits.bytes, b""), 0, "invalid code");
}

// --- headers -------------------------------------------------------------------------------------------------------

#[test]
fn every_combination_of_header_fields_decodes() {
    // The five flags a header may have: FTEXT (bit 0, which changes nothing), FHCRC, FEXTRA, FNAME and FCOMMENT.
    for flags in 0..32 {
        let gzip = member(header(flags), RAW, RAW_PLAIN);
        assert_eq!(decode(&gzip).unwrap(), RAW_PLAIN, "flags {flags:#04x}");
    }
}

#[test]
fn a_header_checksum_that_is_wrong_is_an_error() {
    let mut gzip = member(header(FHCRC | FNAME), RAW, RAW_PLAIN);
    gzip[14] ^= 1; // a byte of the file name, which the header's CRC-16 covers
    expect_error(&gzip, 0, "header CRC-16 mismatch");
    let mut gzip = member(header(FHCRC), RAW, RAW_PLAIN);
    gzip[10] ^= 0x80; // the CRC-16 itself
    expect_error(&gzip, 0, "header CRC-16 mismatch");
}

#[test]
fn a_header_that_is_wrong_is_an_error() {
    let mut gzip = EMPTY.to_vec();
    gzip[2] = 7;
    expect_error(&gzip, 0, "compression method 7");
    for reserved in [0x20, 0x40, 0x80] {
        gzip = EMPTY.to_vec();
        gzip[3] = reserved;
        expect_error(&gzip, 0, "reserved flag bits");
    }
    let whole = member(header(FEXTRA), RAW, RAW_PLAIN);
    expect_error(&whole[..11], 0, "truncated input"); // the extra field's length, cut
    expect_error(&whole[..14], 0, "truncated input"); // the extra field, cut
    let whole = member(header(FNAME), RAW, RAW_PLAIN);
    expect_error(&whole[..10 + 5], 0, "truncated input"); // a file name with no end
}

// --- damage --------------------------------------------------------------------------------------------------------

#[test]
fn a_damaged_byte_is_a_checksum_error_where_the_data_cannot_notice_it() {
    let mut gzip = STORED.to_vec();
    gzip[20] ^= 0x04; // a byte of the stored text
    let error = expect_error(&gzip, 0, "CRC-32 mismatch");
    assert_eq!(error.offset, gzip.len() - 8, "the trailer");
    gzip = STORED.to_vec();
    let at = gzip.len() - 6; // a byte of the CRC-32
    gzip[at] ^= 1;
    expect_error(&gzip, 0, "CRC-32 mismatch");
}

#[test]
fn a_wrong_length_is_an_error() {
    let mut gzip = DYNAMIC.to_vec();
    let at = gzip.len() - 4;
    gzip[at] += 1;
    let error = expect_error(&gzip, 0, "ISIZE mismatch");
    assert_eq!(error.offset, gzip.len() - 4);
}

#[test]
fn input_that_ends_early_is_an_error() {
    for (name, gzip, _) in files() {
        for cut in 0..gzip.len() {
            let error = decode(&gzip[..cut]).expect_err(&format!("{name} cut to {cut} bytes"));
            assert_eq!(error.member, 0);
            assert!(error.offset <= cut, "{name} cut to {cut}: {error}");
        }
        expect_error(&gzip[..gzip.len() - 1], 0, "truncated input");
        expect_error(&gzip[..5], 0, "truncated input");
    }
}

#[test]
fn input_that_is_not_gzip_is_an_error() {
    let mut gzip = FIXED.to_vec();
    gzip[1] = 0x8a;
    expect_error(&gzip, 0, "bad magic");
    let error = expect_error(b"not a gzip file", 0, "bad magic");
    assert_eq!(error.to_string(), "gzip member 0, byte 0: bad magic: not a gzip file");
    expect_error(b"", 0, "bad magic");
}

#[test]
fn bytes_after_the_last_member_that_are_not_a_member_are_an_error() {
    for trailer in [&b"garbage"[..], b"\0", b"\x1f", b"\x1f\x8b", b"\x1f\x8b\x08"] {
        let gzip = [FIXED, DYNAMIC, trailer].concat();
        let error = decode(&gzip).unwrap_err();
        assert_eq!(error.member, 2, "{trailer:?}: {error}");
        assert!(error.offset >= FIXED.len() + DYNAMIC.len(), "{error}");
    }
    let error = expect_error(&[FIXED, b"garbage"].concat(), 1, "after the last member");
    assert_eq!(error.offset, FIXED.len());
}

#[test]
fn a_stored_block_whose_length_and_complement_differ_is_an_error() {
    let mut gzip = STORED.to_vec();
    gzip[13] ^= 1; // the low byte of NLEN
    expect_error(&gzip, 0, "complement");
    let mut gzip = STORED.to_vec();
    gzip[11] += 1; // the low byte of LEN
    expect_error(&gzip, 0, "complement");
    let mut bits = Bits::default();
    bits.stored(true, b"0123456789");
    bits.bytes.truncate(8); // the block says 10 bytes and has 3
    expect_error(&member(header(0), &bits.bytes, b"0123456789"), 0, "truncated input");
}

#[test]
fn a_block_of_type_3_is_an_error() {
    let mut bits = Bits::default();
    bits.number(1, 1);
    bits.number(3, 2);
    expect_error(&member(header(0), &bits.bytes, b""), 0, "invalid block type 3");
}

#[test]
fn symbols_that_deflate_does_not_use_are_errors() {
    let mut bits = Bits::default();
    bits.fixed(true);
    bits.code(fixed_code(286));
    expect_error(&member(header(0), &bits.bytes, b""), 0, "invalid length code 286");
    let mut bits = Bits::default();
    bits.fixed(true);
    fixed_match(&mut bits, 257, 30, (0, 0));
    expect_error(&member(header(0), &bits.bytes, b""), 0, "invalid distance code 30");
    let mut bits = Bits::default();
    bits.fixed(true);
    fixed_match(&mut bits, 257, 0, (0, 0)); // a match with nothing before it
    expect_error(&member(header(0), &bits.bytes, b""), 0, "past the start");
}

#[test]
fn codes_that_cannot_be_made_are_errors() {
    // Literals and lengths: 97, 98 and the end each 1 bit is too many; 2 bits each is too few.
    for (used, text) in
        [([1, 1, 1], "over-subscribed code for literals and lengths"), ([2, 2, 2], "incomplete code for literals")]
    {
        let literals = lengths(257, &[(97, used[0]), (98, used[1]), (256, used[2])]);
        let mut bits = Bits::default();
        dynamic(&mut bits, true, &literals, &[0]);
        expect_error(&member(header(0), &bits.bytes, b""), 0, text);
    }
    for (used, text) in
        [([1, 1, 1], "over-subscribed code for distances"), ([2, 2, 2], "incomplete code for distances")]
    {
        let mut bits = Bits::default();
        dynamic(
            &mut bits,
            true,
            &lengths(257, &[(97, 1), (256, 1)]),
            &lengths(3, &[(0, used[0]), (1, used[1]), (2, used[2])]),
        );
        expect_error(&member(header(0), &bits.bytes, b""), 0, text);
    }
    let mut bits = Bits::default();
    dynamic(&mut bits, true, &lengths(257, &[(97, 1), (98, 1)]), &[0]);
    expect_error(&member(header(0), &bits.bytes, b""), 0, "no code for its end");
}

#[test]
fn a_bad_sequence_of_code_lengths_is_an_error() {
    let mut bits = Bits::default();
    dynamic_spelling(&mut bits, &[1; 19]); // 19 codes of 1 bit
    expect_error(&member(header(0), &bits.bytes, b""), 0, "over-subscribed code for code lengths");
    let mut bits = Bits::default();
    dynamic_spelling(&mut bits, &[2, 2, 2, 0]); // 3 codes of 2 bits, for the symbols 16, 17, 18
    expect_error(&member(header(0), &bits.bytes, b""), 0, "incomplete code for code lengths");
    let mut bits = Bits::default();
    dynamic_spelling(&mut bits, &[1, 1, 0, 0]); // the symbols 16 and 17: 0 and 1
    bits.number(0, 1); // 16: repeat the length before, where there is none
    bits.number(0, 2);
    expect_error(&member(header(0), &bits.bytes, b""), 0, "repeats the one before it");
    let mut bits = Bits::default();
    dynamic_spelling(&mut bits, &[0, 1, 1, 0]); // the symbols 17 and 18: 0 and 1
    for _ in 0..2 {
        bits.number(1, 1); // 18: 138 zeros, twice, of the 258 lengths the block has
        bits.number(127, 7);
    }
    expect_error(&member(header(0), &bits.bytes, b""), 0, "go past the 258 codes");
}

#[test]
fn damaged_input_gives_an_error_or_the_right_bytes_and_never_panics() {
    let mut rng = Rng(0x5EED);
    for (name, gzip, plain) in files() {
        // Every bit of the small files flipped, one at a time.
        for at in 0..gzip.len().min(100) {
            for bit in 0..8 {
                let mut bad = gzip.to_vec();
                bad[at] ^= 1 << bit;
                if let Ok(out) = decode(&bad) {
                    assert_eq!(out, plain, "{name}, bit {bit} of byte {at}");
                }
            }
        }
        // Up to three bytes of the file changed to anything, and the file cut anywhere.
        for round in 0..100 {
            let mut bad = gzip.to_vec();
            for _ in 0..=rng.below(3) {
                let at = rng.below(bad.len());
                bad[at] ^= 1 + rng.below(255) as u8;
            }
            if round % 4 == 0 {
                bad.truncate(rng.below(bad.len() + 1));
            }
            if let Ok(out) = decode(&bad) {
                assert_eq!(out, plain, "{name}, round {round}");
            }
        }
    }
}

// --- member by member ----------------------------------------------------------------------------------------------

#[test]
fn members_one_at_a_time_give_the_bytes_that_decode_gives() {
    let files = files();
    let file: Vec<u8> = files.iter().flat_map(|(_, gzip, _)| gzip.iter().copied()).collect();
    let members: Vec<Vec<u8>> = Members::new(&file).collect::<Result<_, _>>().unwrap();
    assert_eq!(members.len(), files.len());
    for (member, (name, _, plain)) in members.iter().zip(&files) {
        assert_eq!(member, plain, "{name}");
    }
    assert_eq!(members.concat(), decode(&file).unwrap());
}

#[test]
fn members_that_keep_their_bytes_give_the_bytes_of_members_that_borrow_them() {
    let files = files();
    let file: Vec<u8> = files.iter().flat_map(|(_, gzip, _)| gzip.iter().copied()).collect();
    let borrowed: Vec<_> = Members::new(&file).collect();
    // The decoder holds the file, so it can outlive the variable the file was read into.
    let mut owned = Members::owned(file.clone());
    drop(file);
    assert_eq!(owned.by_ref().collect::<Vec<_>>(), borrowed);
    assert!(owned.next().is_none());
    // An error ends it as it ends the borrowing one.
    let mut owned = Members::owned([FIXED, b"garbage", FIXED].concat());
    assert_eq!(owned.next().unwrap().unwrap(), FIXED_PLAIN);
    assert_eq!(owned.next().unwrap().unwrap_err().member, 1);
    assert!(owned.next().is_none());
}

#[test]
fn members_can_be_left_before_the_end_of_the_file() {
    // The third member is damaged and the fourth is not a member, but a reader that stops after the second never knows.
    let mut damaged = DYNAMIC.to_vec();
    let at = damaged.len() - 9;
    damaged[at] ^= 0xFF;
    let file = [FIXED, STORED, &damaged, b"garbage"].concat();
    let mut members = Members::new(&file);
    assert_eq!(members.next().unwrap().unwrap(), FIXED_PLAIN);
    assert_eq!(members.next().unwrap().unwrap(), STORED_PLAIN);
    assert!(decode(&file).is_err());
}

#[test]
fn members_give_nothing_more_after_an_error() {
    let file = [FIXED, b"garbage", FIXED].concat();
    let mut members = Members::new(&file);
    assert_eq!(members.next().unwrap().unwrap(), FIXED_PLAIN);
    assert_eq!(members.next().unwrap().unwrap_err().member, 1);
    assert!(members.next().is_none() && members.next().is_none());
    let mut members = Members::new(b"");
    assert!(members.next().unwrap().is_err());
    assert!(members.next().is_none());
    let mut members = Members::new(EMPTY);
    assert_eq!(members.next().unwrap().unwrap(), EMPTY_PLAIN);
    assert!(members.next().is_none());
}

// --- the checksum --------------------------------------------------------------------------------------------------

#[test]
fn the_crc32_is_the_standard_one() {
    assert_eq!(crc32(b""), 0);
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926); // the check value of the CRC-32 of IEEE 802.3
    let mut rng = Rng(3);
    let data: Vec<u8> = (0..4099).map(|_| rng.next() as u8).collect();
    for length in (0..=40).chain([255, 256, 4099]) {
        assert_eq!(crc32(&data[..length]), crc32_bitwise(&data[..length]), "{length} bytes");
    }
}
