//! Scratch: which games of a range have the same search problem (frame columns and candidates) as an earlier one.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use taxman::frames::build_frames;
use taxman::numtheory::Tables;

fn main() {
    let args: Vec<u32> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let (lo, hi) = (args[0], args[1]);
    let mut seen: HashMap<u64, u32> = HashMap::new();
    for n in lo..=hi {
        let tables = Tables::new(n);
        let frames = build_frames::<32>(&tables);
        let columns: Vec<Vec<u32>> =
            frames.frames.iter().flat_map(|f| [f.factors.to_vec(), f.moves.to_vec()]).collect();
        let mut h = DefaultHasher::new();
        columns.hash(&mut h);
        frames.candidates.to_vec().hash(&mut h);
        let key = h.finish();
        match seen.get(&key) {
            Some(first) => println!("n={n}: same frames and candidates as n={first}"),
            None => {
                seen.insert(key, n);
                println!("n={n}: new ({} candidates)", frames.candidates.len());
            }
        }
    }
}
