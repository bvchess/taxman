//! The committed results pass the checks that `taxman verify-results` runs (see `taxman::results`): every game
//! replays legally to its recorded score, is over and opens with the largest prime, keeps to the chain from
//! n − 1, and the arithmetic and provenance of its line hold; the files agree on every game they share.  The
//! counts asserted here are the ones `results/README.md` quotes.

use std::path::PathBuf;
use taxman::game::Optimal;
use taxman::results::{verify_files, Certificate, FileReport, Proof};

fn crate_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// How many games of the report have the proof, the certificate, or an empty taint.
fn proofs(file: &FileReport, proof: Proof) -> usize {
    file.contents.games.values().filter(|g| g.proof == proof).count()
}

fn certificates(file: &FileReport, certificate: Certificate) -> usize {
    file.contents.games.values().filter(|g| g.certificate == Some(certificate)).count()
}

fn clean(file: &FileReport) -> usize {
    file.contents.games.values().filter(|g| g.taint.is_empty()).count()
}

#[test]
fn the_committed_results_pass_the_verifier() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(crate_path("results"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .collect();
    paths.sort();
    let names: Vec<_> = paths.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["default.jsonl", "strict.jsonl"], "a new results file needs its counts here and in the README");
    // The chain of the cost-curve study is the strict chain 1001-1231 built by a later commit.
    paths.push(crate_path("bench/curve/chain.jsonl"));

    let optimal = Optimal::load_default().unwrap();
    let report = verify_files(&paths, Some(&optimal)).unwrap_or_else(|e| panic!("{e}"));
    println!("{report}");
    let [default, strict, chain] = &report.files[..] else { panic!("three files") };
    for (file, lo, hi) in [(default, 1001, 1231), (strict, 1001, 1219), (chain, 1001, 1231)] {
        let games = &file.contents.games;
        assert!(games.keys().copied().eq(lo..=hi), "{}: not the games {lo}..={hi}", file.path.display());
        assert_eq!((file.contents.incomplete_lines, file.contents.torn), (0, false), "{}", file.path.display());
    }
    // Every game the files share has the same score in all of them.
    assert_eq!(report.shared, 231);

    // default.jsonl: derived by reuse_prev where it could be, which most of its games rest on.
    let counts =
        |f: &FileReport| [Proof::PrimeFormula, Proof::Cap, Proof::Exhausted, Proof::ReusePrev].map(|p| proofs(f, p));
    assert_eq!(counts(default), [34, 0, 78, 119]);
    assert_eq!(clean(default), 19);
    // strict.jsonl and the chain: no unproven tactic, and nothing unproven anywhere below them.
    assert_eq!(counts(strict), [31, 66, 122, 0]);
    assert_eq!(counts(chain), [34, 70, 127, 0]);
    assert_eq!((clean(strict), clean(chain)), (219, 231));
    // Scores at a proven ceiling given the game of n − 1 (what certifies a game whatever found it).
    let ceilings = |f: &FileReport| [Certificate::NPlusPrev, Certificate::PrimeFormula].map(|c| certificates(f, c));
    assert_eq!(ceilings(default), [70, 34]);
    assert_eq!(ceilings(strict), [66, 31]);
    assert_eq!(ceilings(chain), [70, 34]);
}
