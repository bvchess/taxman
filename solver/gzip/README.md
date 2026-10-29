# The gzip decoder

The crate `taxman-gzip` decodes gzip files, and uses nothing but Rust's standard library.  It exists to read the results
file `results/optimal.jsonl.gz`, which both Rust programs, the frame solver and the prover, need.

## Why it exists

The results file holds one JSON line per game, compressed with gzip.  The frame solver builds it into its binary, and
the prover reads it from disk.  The project takes no third-party library, so the two programs share this decoder.

Both programs rely on what the decoder gives them, so it must fail loudly.  Every gzip *member* (one compressed stream
with its own header and trailer, [RFC 1952](https://www.rfc-editor.org/rfc/rfc1952)) carries the CRC-32 and the length
of its bytes, and the decoder checks both.  A bug in the decoder, or damage to the file, gives an error and never wrong
bytes.

The results file is several members, one after another.  Games are appended as a new member, which leaves the earlier
bytes as they were.  A reader that wants game n decodes members in order until it has the line of game n, so the crate
decodes member by member and lets the reader stop.

## Using it

```rust
use taxman_gzip::{decode, Members};

let everything: Vec<u8> = decode(&bytes)?;   // every member, one after another

for member in Members::new(&bytes) {          // or a member at a time, in order
    let lines: Vec<u8> = member?;
    // ... stop here when the wanted line is found
}
```

`decode` and `Members` take the whole file as a byte slice.  `Members` gives nothing after its first error.  `crc32` is
the checksum the trailers use.

## What it checks

An `Error` says what is wrong, in which member (0 for the first) and at which byte of the input.  The decoder returns
one for each of these:

* a header that is not gzip's: bad magic, a method other than 8 (deflate), reserved flag bits, or a header CRC-16
  (FHCRC) that does not match;
* input that ends early;
* damaged [deflate](https://www.rfc-editor.org/rfc/rfc1951) data: an invalid block type, a stored block whose length and
  complement differ, a code that is over-subscribed or incomplete, a bad sequence of code lengths, a symbol that deflate
  does not use, or a match that reaches back past the start of the member's bytes;
* a CRC-32 or a length (ISIZE) in the trailer that does not match the bytes decoded;
* bytes after the last member that do not start a member, and an empty input.

## Limits

* It decodes and does not compress.
* It decodes each member into memory, whole, and sets no limit on the size of the bytes it gives: a small member can
  decode to a very large one.
* The deflate data is read as the format defines it, with these codes accepted as incomplete: one with no symbol (a
  block that has no matches) and one with a single symbol of length 1 (a block that has one distance).

## Commands

Cargo commands run in `solver/gzip`.  `solver/tools/quick_check.py` runs the formatting check, the lints and the tests
of the crate, with those of the frame solver.  The tests decode files made by Python's `gzip` and `zlib`, and streams
and headers written by hand, which no compressor makes.

The example `gunzip` decodes a file to standard output and says on standard error how long the decoding took:

```
cargo run --release --example gunzip FILE.gz > FILE
```
