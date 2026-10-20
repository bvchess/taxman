//! The `.nodes` fixtures of `testdata/perf/` (search nodes of a proof, each its promotions and candidates), and the
//! hex text they write sets in, for the examples that read or write sets (`#[path = "common/fixtures.rs"] mod
//! fixtures;`; the probe, `examples/perf.rs`, is the first).

use std::path::Path;
use taxman::bits::Bits;
use taxman::search::TaxUnit;

/// A set as hex digits, digit `k` holding the members `4k..4k + 3` (lowest bit first), trailing zeros left out
/// (the empty set is "0").
pub fn to_hex<const W: usize>(set: &Bits<W>) -> String {
    let len = set.max().map_or(1, |x| x / 4 + 1);
    (0..len)
        .map(|k| {
            let nibble = (0..4).filter(|b| set.contains(4 * k + b)).fold(0, |acc, b| acc | 1 << b);
            char::from_digit(nibble, 16).expect("a nibble")
        })
        .collect()
}

pub fn from_hex<const W: usize>(text: &str) -> Result<Bits<W>, String> {
    let mut set = Bits::EMPTY;
    for (k, c) in text.chars().enumerate() {
        let nibble = c.to_digit(16).ok_or_else(|| format!("{c:?} is not a hex digit"))?;
        for b in (0..4).filter(|b| nibble >> b & 1 == 1) {
            let x = 4 * k as u32 + b;
            if x > Bits::<W>::MAX_VALUE {
                return Err(format!("{x} does not fit in Bits<{W}>"));
            }
            set.insert(x);
        }
    }
    Ok(set)
}

/// The nodes of a `.nodes` fixture: (n, incumbent, units).
pub type Nodes<const W: usize> = (u32, Bits<W>, Vec<TaxUnit<W>>);

pub fn read_nodes<const W: usize>(path: &Path) -> Result<Nodes<W>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (mut n, mut incumbent, mut units) = (None, Bits::EMPTY, Vec::new());
    for (i, line) in text.lines().enumerate().filter(|(_, line)| !line.starts_with('#') && !line.is_empty()) {
        let at = |e: String| format!("{}: line {}: {e}", path.display(), i + 1);
        let fields: Vec<&str> = line.split(' ').collect();
        match fields[..] {
            ["n", value] => n = Some(value.parse().map_err(|e| at(format!("{e}")))?),
            ["incumbent", set] => incumbent = from_hex(set).map_err(at)?,
            ["node", promoted, cands] => {
                units.push(TaxUnit { promoted: from_hex(promoted).map_err(at)?, cands: from_hex(cands).map_err(at)? })
            }
            _ => return Err(at(format!("unexpected line {line:?}"))),
        }
    }
    Ok((n.ok_or_else(|| format!("{}: no n line", path.display()))?, incumbent, units))
}
