//! A seeded generator, for the code that samples: the large neighborhood search of [`crate::search::finder`] picks
//! its neighborhoods with it, and the tuning benchmark (`examples/tune.rs`) samples the units of a checkpoint.

/// SplitMix64: a small, good generator, seeded.
pub struct Rng(pub u64);

impl Rng {
    /// The next 64 random bits.
    #[allow(clippy::should_implement_trait)] // a generator, not an iterator: it never ends
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n` (`n` positive; the bias of the remainder is negligible for the small `n` used here).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    /// A number in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// `items` in a random order (Fisher-Yates).
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            items.swap(i, self.below(i + 1));
        }
    }
}
