//! Experiment: how much does capping the upper half at W(U*) tighten the matching bound?  For λ = k/8,
//! opt(n) <= max over matchings of [(1−λ)·upper + lower] + λ·W(U*), valid because every playable upper set
//! weighs at most W(U*) (certified per game by the span check).  `lagrange N...` prints the bound for each λ.

use taxman_evidence::game::{Board, MatchingBound, Role};
use taxman_evidence::instance::Instance;
use taxman_evidence::primes::smallest_prime_factors;
use taxman_evidence::span::span_check;

fn main() {
    let ns: Vec<u32> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let spf = smallest_prime_factors(*ns.iter().max().unwrap() as usize);
    for n in ns {
        let board = Board::new(n, &spf);
        let span = span_check(&Instance::upper_half(u64::from(n), &spf));
        assert!(span.violations.is_empty());
        let cap = span.kept_sum as i64;
        let mut roles = vec![Role::Free; n as usize + 1];
        roles[0] = Role::Barred;
        roles[1] = Role::Barred;
        let mut row = format!("n={n} W(U*)={cap}:");
        for k in 0..=8i64 {
            let mut m = MatchingBound::new(&board);
            m.set_scale(8 - k, 8);
            let (v, _) = m.solve(&board, &roles).unwrap();
            let bound = (v as i64 + k * cap) as f64 / 8.0;
            row += &format!(" {bound:.0}");
        }
        println!("{row}");
    }
}
