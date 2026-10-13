//! A minimal JSON reader and writer, so that the crate needs no dependencies.  It is enough for what the solver
//! reads and writes as JSON: the Java's table of optimal games `optimal.json` ([`crate::game::Optimal`]), the
//! results files, one object per line ([`crate::results`]), and the golden test data.  (Checkpoints are
//! binary, not JSON: see [`crate::engine`] and the search's header.)
//!
//! [`Json::parse`] reads one document (RFC 8259: strict about commas, leading zeros and control characters
//! in strings) into a [`Json`] tree; [`Json::write_to`] and `Display` write the compact form.  The accessors
//! (`get`, `as_u32`, `as_arr`, ...) return `None` rather than panicking when the shape is not what the caller
//! expects, so a corrupt file becomes an error message and not a crash.
//!
//! * Integers (numbers with no fraction or exponent that fit in an `i64`) parse to [`Json::Int`], everything
//!   else numeric to [`Json::Num`]; an integral `Num` such as `1e3` still reads as an integer through
//!   [`Json::as_i64`].
//! * Objects keep their keys in source order; if a key is repeated, [`Json::get`] returns the first
//!   occurrence.
//! * Nesting deeper than 256 is refused, so hostile input cannot overflow the stack.
//! * The writer emits no whitespace.  Floats are written in the shortest form that reads back as the same
//!   float; `NaN` and infinities, which JSON cannot express, become `null`.

use std::fmt;
use std::path::Path;

/// Nesting limit for the parser, so hostile input cannot overflow the stack.
const MAX_DEPTH: usize = 256;

/// A JSON value.  Numbers come in two kinds so that scores and moves survive exactly: `Int` for what fits an
/// `i64`, `Num` (an `f64`) for the rest.  An object is a list of pairs, in source order.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Int(i64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

/// A syntax error, with the byte offset where it was detected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub offset: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON syntax error at byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for ParseError {}

// ---------------------------------------------------------------------------------------------------------
// Accessors and builders

impl Json {
    /// Parses a complete JSON document (surrounding whitespace allowed, trailing content not).
    pub fn parse(text: &str) -> Result<Json, ParseError> {
        let mut p = Parser { src: text, bytes: text.as_bytes(), pos: 0 };
        let value = p.value(0)?;
        p.skip_ws();
        if p.pos != p.bytes.len() {
            return p.err("unexpected trailing content");
        }
        Ok(value)
    }

    /// Reads and parses a file; errors mention the path.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Json, String> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Json::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The value for `key`, if this is an object that has it.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The `i`-th element, if this is an array that long.
    pub fn at(&self, i: usize) -> Option<&Json> {
        self.as_arr()?.get(i)
    }

    /// Whether this is `null`.
    pub fn is_null(&self) -> bool {
        matches!(self, Json::Null)
    }

    /// The value of a `true` or `false`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// An integer value.  A [`Json::Num`] counts if it is integral and in range (so `1e3` reads as 1000).
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Json::Int(i) => Some(i),
            Json::Num(f) if f.fract() == 0.0 && f.abs() < 9.0e18 => Some(f as i64),
            _ => None,
        }
    }

    /// A non-negative integer (see [`as_i64`](Self::as_i64)).
    pub fn as_u64(&self) -> Option<u64> {
        u64::try_from(self.as_i64()?).ok()
    }

    /// An integer in `0..=u32::MAX`.
    pub fn as_u32(&self) -> Option<u32> {
        u32::try_from(self.as_i64()?).ok()
    }

    /// Any number, as a float.
    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Json::Int(i) => Some(i as f64),
            Json::Num(f) => Some(f),
            _ => None,
        }
    }

    /// The text of a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    /// The elements of an array.
    pub fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(items) => Some(items),
            _ => None,
        }
    }

    /// The `(key, value)` pairs of an object, in source order.
    pub fn as_obj(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Obj(pairs) => Some(pairs),
            _ => None,
        }
    }

    /// An array of integers that all fit in a `u32`.
    pub fn as_u32_vec(&self) -> Option<Vec<u32>> {
        self.as_arr()?.iter().map(Json::as_u32).collect()
    }

    /// An object from `(key, value)` pairs, keeping their order.
    pub fn object<K: Into<String>>(pairs: impl IntoIterator<Item = (K, Json)>) -> Json {
        Json::Obj(pairs.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    /// An array from anything convertible to JSON values.
    pub fn array<T: Into<Json>>(items: impl IntoIterator<Item = T>) -> Json {
        Json::Arr(items.into_iter().map(Into::into).collect())
    }

    /// Writes the compact form to `out`.
    pub fn write_to<W: fmt::Write + ?Sized>(&self, out: &mut W) -> fmt::Result {
        match self {
            Json::Null => out.write_str("null"),
            Json::Bool(b) => out.write_str(if *b { "true" } else { "false" }),
            Json::Int(i) => write!(out, "{i}"),
            // `{:?}` is the shortest representation that round-trips, and always reads back as a float.
            // JSON has no infinities or NaN.
            Json::Num(f) if f.is_finite() => write!(out, "{f:?}"),
            Json::Num(_) => out.write_str("null"),
            Json::Str(s) => write_string(out, s),
            Json::Arr(items) => {
                out.write_char('[')?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.write_char(',')?;
                    }
                    item.write_to(out)?;
                }
                out.write_char(']')
            }
            Json::Obj(pairs) => {
                out.write_char('{')?;
                for (i, (key, value)) in pairs.iter().enumerate() {
                    if i > 0 {
                        out.write_char(',')?;
                    }
                    write_string(out, key)?;
                    out.write_char(':')?;
                    value.write_to(out)?;
                }
                out.write_char('}')
            }
        }
    }
}

/// The compact form; `to_string()` comes with it.
impl fmt::Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_to(f)
    }
}

/// Writes `s` as a JSON string literal: quotes and backslashes escaped, control characters as `\n`, `\t`, ... or
/// `\u00XX`; everything else (non-ASCII included) as it is.
fn write_string<W: fmt::Write + ?Sized>(out: &mut W, s: &str) -> fmt::Result {
    out.write_char('"')?;
    for c in s.chars() {
        match c {
            '"' => out.write_str("\\\"")?,
            '\\' => out.write_str("\\\\")?,
            '\n' => out.write_str("\\n")?,
            '\r' => out.write_str("\\r")?,
            '\t' => out.write_str("\\t")?,
            '\u{08}' => out.write_str("\\b")?,
            '\u{0c}' => out.write_str("\\f")?,
            c if u32::from(c) < 0x20 => write!(out, "\\u{:04x}", u32::from(c))?,
            c => out.write_char(c)?,
        }
    }
    out.write_char('"')
}

impl From<bool> for Json {
    fn from(b: bool) -> Json {
        Json::Bool(b)
    }
}

impl From<i64> for Json {
    fn from(i: i64) -> Json {
        Json::Int(i)
    }
}

impl From<u32> for Json {
    fn from(i: u32) -> Json {
        Json::Int(i64::from(i))
    }
}

/// Values above `i64::MAX` become (lossy) floats.
impl From<u64> for Json {
    fn from(i: u64) -> Json {
        i64::try_from(i).map_or(Json::Num(i as f64), Json::Int)
    }
}

impl From<usize> for Json {
    fn from(i: usize) -> Json {
        Json::from(i as u64)
    }
}

impl From<f64> for Json {
    fn from(f: f64) -> Json {
        Json::Num(f)
    }
}

impl From<&str> for Json {
    fn from(s: &str) -> Json {
        Json::Str(s.to_string())
    }
}

impl From<String> for Json {
    fn from(s: String) -> Json {
        Json::Str(s)
    }
}

impl From<Vec<Json>> for Json {
    fn from(items: Vec<Json>) -> Json {
        Json::Arr(items)
    }
}

// ---------------------------------------------------------------------------------------------------------
// Parser: recursive descent over the bytes of a `&str`.

struct Parser<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn err<T>(&self, message: impl Into<String>) -> Result<T, ParseError> {
        Err(ParseError { offset: self.pos, message: message.into() })
    }

    #[inline]
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    #[inline]
    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    /// Consumes `byte` if it is next.
    #[inline]
    fn eat(&mut self, byte: u8) -> bool {
        let found = self.peek() == Some(byte);
        self.pos += usize::from(found);
        found
    }

    fn value(&mut self, depth: usize) -> Result<Json, ParseError> {
        self.skip_ws();
        match self.peek() {
            None => self.err("unexpected end of input"),
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') => self.literal("true", Json::Bool(true)),
            Some(b'f') => self.literal("false", Json::Bool(false)),
            Some(b'n') => self.literal("null", Json::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(c) => self.err(format!("unexpected character {:?}", char::from(c))),
        }
    }

    fn literal(&mut self, word: &str, value: Json) -> Result<Json, ParseError> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(value)
        } else {
            self.err(format!("expected `{word}`"))
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        self.pos - start
    }

    fn number(&mut self) -> Result<Json, ParseError> {
        let start = self.pos;
        self.eat(b'-');
        // An integer part is `0` or a nonzero digit followed by digits (no leading zeros).
        if !self.eat(b'0') && self.digits() == 0 {
            return self.err("expected a digit");
        }
        let mut is_float = false;
        if self.eat(b'.') {
            is_float = true;
            if self.digits() == 0 {
                return self.err("expected a digit after the decimal point");
            }
        }
        if self.eat(b'e') || self.eat(b'E') {
            is_float = true;
            if !self.eat(b'+') {
                self.eat(b'-');
            }
            if self.digits() == 0 {
                return self.err("expected a digit in the exponent");
            }
        }
        let text = &self.src[start..self.pos];
        if !is_float {
            if let Ok(i) = text.parse::<i64>() {
                return Ok(Json::Int(i));
            }
            // Too big for an i64: fall through to a float.
        }
        match text.parse::<f64>() {
            Ok(f) => Ok(Json::Num(f)),
            Err(_) => self.err(format!("bad number {text:?}")),
        }
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
        let mut v = 0;
        for _ in 0..4 {
            let Some(d) = self.peek().and_then(|c| char::from(c).to_digit(16)) else {
                return self.err("expected four hex digits after \\u");
            };
            v = v * 16 + d;
            self.pos += 1;
        }
        Ok(v)
    }

    /// Parses a string literal, starting at its opening quote.
    fn string(&mut self) -> Result<String, ParseError> {
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            // Copy the run of ordinary characters up to the next quote, backslash or control character.
            let start = self.pos;
            while matches!(self.peek(), Some(c) if c != b'"' && c != b'\\' && c >= 0x20) {
                self.pos += 1;
            }
            out.push_str(&self.src[start..self.pos]);
            match self.peek() {
                None => return self.err("unterminated string"),
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    let Some(esc) = self.peek() else {
                        return self.err("unterminated string");
                    };
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{08}'),
                        b'f' => out.push('\u{0c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => out.push(self.unicode_escape()?),
                        _ => {
                            self.pos -= 1;
                            return self.err("invalid escape");
                        }
                    }
                }
                Some(_) => return self.err("control character in string"),
            }
        }
    }

    /// The character for a `\uXXXX` escape (the `\u` is already consumed), joining surrogate pairs.
    fn unicode_escape(&mut self) -> Result<char, ParseError> {
        let first = self.hex4()?;
        let code = match first {
            0xD800..=0xDBFF => {
                if !(self.eat(b'\\') && self.eat(b'u')) {
                    return self.err("high surrogate not followed by a low surrogate");
                }
                let second = self.hex4()?;
                if !(0xDC00..=0xDFFF).contains(&second) {
                    return self.err("high surrogate not followed by a low surrogate");
                }
                0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
            }
            0xDC00..=0xDFFF => return self.err("unpaired low surrogate"),
            _ => first,
        };
        // Surrogates were handled above, so this cannot fail.
        char::from_u32(code).map_or_else(|| self.err("invalid code point"), Ok)
    }

    fn array(&mut self, depth: usize) -> Result<Json, ParseError> {
        if depth >= MAX_DEPTH {
            return self.err("nesting too deep");
        }
        self.pos += 1; // '['
        let mut items = Vec::new();
        self.skip_ws();
        if self.eat(b']') {
            return Ok(Json::Arr(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.skip_ws();
            if self.eat(b',') {
                continue;
            }
            if self.eat(b']') {
                return Ok(Json::Arr(items));
            }
            return self.err("expected `,` or `]`");
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, ParseError> {
        if depth >= MAX_DEPTH {
            return self.err("nesting too deep");
        }
        self.pos += 1; // '{'
        let mut pairs = Vec::new();
        self.skip_ws();
        if self.eat(b'}') {
            return Ok(Json::Obj(pairs));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return self.err("expected a string key");
            }
            let key = self.string()?;
            self.skip_ws();
            if !self.eat(b':') {
                return self.err("expected `:`");
            }
            pairs.push((key, self.value(depth + 1)?));
            self.skip_ws();
            if self.eat(b',') {
                continue;
            }
            if self.eat(b'}') {
                return Ok(Json::Obj(pairs));
            }
            return self.err("expected `,` or `}`");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Json {
        Json::parse(s).unwrap_or_else(|e| panic!("{s:?}: {e}"))
    }

    fn fails(s: &str) -> ParseError {
        Json::parse(s).expect_err(s)
    }

    #[test]
    fn scalars() {
        assert_eq!(parse("null"), Json::Null);
        assert_eq!(parse("true"), Json::Bool(true));
        assert_eq!(parse(" false\n"), Json::Bool(false));
        assert_eq!(parse("0"), Json::Int(0));
        assert_eq!(parse("-0"), Json::Int(0));
        assert_eq!(parse("42"), Json::Int(42));
        assert_eq!(parse("-17"), Json::Int(-17));
        assert_eq!(parse("9223372036854775807"), Json::Int(i64::MAX));
        assert_eq!(parse("-9223372036854775808"), Json::Int(i64::MIN));
        assert_eq!(parse("\"hi\""), Json::Str("hi".into()));
        assert_eq!(parse("\"\""), Json::Str(String::new()));
    }

    #[test]
    fn numbers_with_fraction_or_exponent_are_floats() {
        assert_eq!(parse("1.5"), Json::Num(1.5));
        assert_eq!(parse("-0.25"), Json::Num(-0.25));
        assert_eq!(parse("1e3"), Json::Num(1000.0));
        assert_eq!(parse("1E+3"), Json::Num(1000.0));
        assert_eq!(parse("2.5e-2"), Json::Num(0.025));
        assert_eq!(parse("1.0"), Json::Num(1.0));
        // Too big for an i64, so a float.
        assert_eq!(parse("9223372036854775808"), Json::Num(9223372036854775808.0));
        // An integral float still reads as an integer through the accessor.
        assert_eq!(parse("1e3").as_i64(), Some(1000));
        assert_eq!(parse("1.5").as_i64(), None);
    }

    #[test]
    fn bad_numbers() {
        for s in ["01", "-", "+1", "1.", ".5", "1e", "1e+", "--1", "0x10", "1.2.3", "NaN", "Infinity"] {
            assert!(Json::parse(s).is_err(), "{s}");
        }
    }

    #[test]
    fn nested_structures() {
        let j = parse(r#" { "n": 5, "moves": [3, 4, 5], "tags": {"a": null, "b": [] , "c": {}}, "ok": true } "#);
        assert_eq!(j.get("n").and_then(Json::as_i64), Some(5));
        assert_eq!(j.get("moves").and_then(Json::as_u32_vec), Some(vec![3, 4, 5]));
        assert_eq!(j.get("moves").and_then(|m| m.at(1)).and_then(Json::as_u32), Some(4));
        assert_eq!(j.get("ok").and_then(Json::as_bool), Some(true));
        let tags = j.get("tags").unwrap();
        assert!(tags.get("a").unwrap().is_null());
        assert_eq!(tags.get("b"), Some(&Json::Arr(vec![])));
        assert_eq!(tags.get("c"), Some(&Json::Obj(vec![])));
        assert_eq!(tags.get("missing"), None);
        assert_eq!(j.as_obj().unwrap().len(), 4);
        assert_eq!(j.as_obj().unwrap()[0].0, "n"); // source order kept
        assert_eq!(j.at(0), None); // not an array
        assert_eq!(parse("[1,[2,[3]]]").as_arr().unwrap().len(), 2);
    }

    #[test]
    fn duplicate_keys_return_the_first() {
        assert_eq!(parse(r#"{"a":1,"a":2}"#).get("a"), Some(&Json::Int(1)));
    }

    #[test]
    fn accessors_reject_wrong_types() {
        let j = parse(r#"{"s":"x","n":-3,"a":[1,-1,2.5,70000000000]}"#);
        assert_eq!(j.get("s").unwrap().as_i64(), None);
        assert_eq!(j.get("n").unwrap().as_str(), None);
        assert_eq!(j.get("n").unwrap().as_u64(), None);
        assert_eq!(j.get("n").unwrap().as_u32(), None);
        assert_eq!(j.get("n").unwrap().as_f64(), Some(-3.0));
        assert_eq!(j.get("s").unwrap().as_arr(), None);
        assert_eq!(j.get("a").unwrap().as_u32_vec(), None); // -1, 2.5 and 70000000000 don't fit
        assert_eq!(parse("[]").as_u32_vec(), Some(vec![]));
        assert_eq!(parse("[1,\"2\"]").as_u32_vec(), None);
        assert_eq!(Json::Null.get("x"), None);
    }

    #[test]
    fn string_escapes() {
        assert_eq!(parse(r#""a\"b\\c\/d\n\r\t\b\f""#), Json::Str("a\"b\\c/d\n\r\t\u{8}\u{c}".into()));
        assert_eq!(parse(r#""Aé中""#), Json::Str("Aé中".into()));
        // A surrogate pair is one character outside the BMP.
        assert_eq!(parse(r#""😀""#), Json::Str("😀".into()));
        // Raw non-ASCII passes through.
        assert_eq!(parse("\"héllo 中\""), Json::Str("héllo 中".into()));
    }

    #[test]
    fn bad_strings() {
        for s in [
            r#""abc"#,
            r#""\x""#,
            r#""\u12""#,
            r#""\ud83d""#,
            r#""\ud83dx""#,
            r#""\ud83dA""#,
            r#""\ude00""#,
            "\"line\nbreak\"",
            "\"tab\there\"",
            r#""\"#,
        ] {
            assert!(Json::parse(s).is_err(), "{s:?}");
        }
    }

    #[test]
    fn syntax_errors_have_offsets() {
        assert_eq!(fails("").message, "unexpected end of input");
        assert_eq!(fails("[1,]").offset, 3);
        assert_eq!(fails("[1 2]").offset, 3);
        assert_eq!(fails("{\"a\" 1}").offset, 5);
        assert_eq!(fails("{a:1}").offset, 1);
        assert_eq!(fails("{\"a\":1,}").offset, 7);
        assert_eq!(fails("nul").offset, 0);
        assert_eq!(fails("[1] x").offset, 4);
        assert!(fails("[1] x").to_string().contains("byte 4"));
        for s in ["[", "{", "[1,", "{\"a\":", "tru", "]", "}", ",", "'a'", "[1,,2]"] {
            assert!(Json::parse(s).is_err(), "{s:?}");
        }
    }

    #[test]
    fn nesting_limit() {
        let ok = format!("{}{}", "[".repeat(MAX_DEPTH), "]".repeat(MAX_DEPTH));
        assert!(Json::parse(&ok).is_ok());
        let deep = format!("{}{}", "[".repeat(MAX_DEPTH + 1), "]".repeat(MAX_DEPTH + 1));
        assert_eq!(fails(&deep).message, "nesting too deep");
    }

    #[test]
    fn writer_is_compact_and_escapes() {
        let j = Json::object([
            ("n", Json::from(5u32)),
            ("name", Json::from("a\"b\\c\n\u{1}é")),
            ("moves", Json::array([3u32, 4, 5])),
            ("empty", Json::Arr(vec![])),
            ("none", Json::Null),
            ("ok", Json::from(true)),
            ("x", Json::from(1.5)),
        ]);
        assert_eq!(
            j.to_string(),
            r#"{"n":5,"name":"a\"b\\c\n\u0001é","moves":[3,4,5],"empty":[],"none":null,"ok":true,"x":1.5}"#
        );
    }

    #[test]
    fn writer_floats() {
        assert_eq!(Json::Num(1.0).to_string(), "1.0"); // stays a float on re-read
        assert_eq!(parse(&Json::Num(1.0).to_string()), Json::Num(1.0));
        assert_eq!(Json::Num(f64::NAN).to_string(), "null");
        assert_eq!(Json::Num(f64::INFINITY).to_string(), "null");
        for f in [0.1, -2.5e-9, 1.0e300, 123456789.125, f64::MIN_POSITIVE] {
            assert_eq!(parse(&Json::Num(f).to_string()), Json::Num(f), "{f}");
        }
    }

    #[test]
    fn round_trip() {
        let text = r#"{"a":[1,2,{"b":"c\nd","e":[true,false,null,-5,2.5]}],"f":{},"g":[]}"#;
        let j = parse(text);
        assert_eq!(j.to_string(), text);
        assert_eq!(parse(&j.to_string()), j);
    }

    #[test]
    fn from_conversions() {
        assert_eq!(Json::from(7u64), Json::Int(7));
        assert_eq!(Json::from(u64::MAX), Json::Num(u64::MAX as f64));
        assert_eq!(Json::from(3usize), Json::Int(3));
        assert_eq!(Json::from(-4i64), Json::Int(-4));
        assert_eq!(Json::from(String::from("s")), Json::Str("s".into()));
        assert_eq!(Json::from(vec![Json::Null]), Json::Arr(vec![Json::Null]));
    }

    #[test]
    fn from_file_reports_the_path() {
        let err = Json::from_file("/nonexistent/definitely/not/here.json").unwrap_err();
        assert!(err.contains("not/here.json"), "{err}");
    }
}
