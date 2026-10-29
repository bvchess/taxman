//! The reader of the input: bytes for the gzip header and trailer and for stored blocks, bits (the lowest bit of a byte
//! first, RFC 1951, 3.1.1) for the rest of deflate.  It remembers the member it reads, so that its errors name it.

use crate::Error;

/// A position in the input, and up to 64 bits loaded ahead of it.
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    /// The next byte to load into `buf`.
    pos: usize,
    /// The bits loaded and not yet used, the next one in the lowest place.
    buf: u64,
    /// How many bits of `buf` are loaded.
    count: u32,
    /// The index of the member being read.
    member: usize,
}

impl<'a> Reader<'a> {
    /// A reader at byte `offset` of `data`, which is where member number `member` starts.
    pub fn new(data: &'a [u8], offset: usize, member: usize) -> Self {
        Reader { data, pos: offset, buf: 0, count: 0, member }
    }

    /// The member being read.
    pub fn member(&self) -> usize {
        self.member
    }

    /// The offset of the byte that holds the next unread bit.
    pub fn offset(&self) -> usize {
        self.pos - (self.count as usize).div_ceil(8)
    }

    /// An error at the current offset.
    pub fn fail(&self, message: impl Into<String>) -> Error {
        Error { member: self.member, offset: self.offset(), message: message.into() }
    }

    /// The error of reading past the end of the input.
    fn truncated(&self) -> Error {
        Error { member: self.member, offset: self.data.len(), message: "truncated input".into() }
    }

    /// Loads bytes until `buf` holds more than 56 bits, or the input has no more.
    fn refill(&mut self) {
        while self.count <= 56 && self.pos < self.data.len() {
            self.buf |= u64::from(self.data[self.pos]) << self.count;
            self.pos += 1;
            self.count += 8;
        }
    }

    /// The next `n` (at most 32) bits, not taken; the missing ones past the end of the input read as zeros, which
    /// [`skip`](Self::skip) reports.
    pub fn peek(&mut self, n: u32) -> u32 {
        self.refill();
        (self.buf & ((1u64 << n) - 1)) as u32
    }

    /// Takes `n` bits that [`peek`](Self::peek) showed.
    pub fn skip(&mut self, n: u32) -> Result<(), Error> {
        if n > self.count {
            return Err(self.truncated());
        }
        self.buf >>= n;
        self.count -= n;
        Ok(())
    }

    /// The next `n` (at most 32) bits as a number, the first bit read the lowest.
    pub fn bits(&mut self, n: u32) -> Result<u32, Error> {
        let value = self.peek(n);
        self.skip(n)?;
        Ok(value)
    }

    /// Moves to the next byte boundary, giving the bytes loaded ahead back to the input.
    pub fn align(&mut self) {
        self.pos -= (self.count / 8) as usize;
        self.buf = 0;
        self.count = 0;
    }

    /// What is left of the input, from the next byte boundary.
    pub fn rest(&mut self) -> &'a [u8] {
        self.align();
        &self.data[self.pos..]
    }

    /// The next `n` bytes, from the next byte boundary.
    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let found = self.rest().get(..n).ok_or_else(|| self.truncated())?;
        self.pos += n;
        Ok(found)
    }

    /// The bytes from `start` to the next byte boundary (the part of the input already read, if `start` is before it).
    pub fn since(&mut self, start: usize) -> &'a [u8] {
        self.align();
        &self.data[start..self.pos]
    }

    /// A two-byte number, little-endian, from the next byte boundary.
    pub fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(self.bytes(2)?.try_into().unwrap()))
    }

    /// A four-byte number, little-endian, from the next byte boundary.
    pub fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    /// A string of bytes that ends at a zero byte, which is read too, from the next byte boundary.
    pub fn zero_terminated(&mut self) -> Result<&'a [u8], Error> {
        let length = self.rest().iter().position(|&b| b == 0).ok_or_else(|| self.truncated())?;
        self.bytes(length + 1)
    }
}
