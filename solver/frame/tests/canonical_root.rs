//! Setup builds the root problem Π_n of the theory (docs/theory/frame-model.md): U\* (the sources of the frames as
//! setup forms them), the candidates, and the moves and factors of each frame of the model at the root
//! ([`taxman::frames::root_problem_of_setup`]) are those of [`taxman::root_problem::build`], which builds Π_n from the
//! definitions with no code of `frames` or `model`.  The census `examples/structure.rs` at commit 3ca11bf (check I)
//! did the same on every board to 8191, and the census of the boards past 8191 (`census_of_the_boards_past_8191` in
//! the tests of `src/frames/mod.rs`) does it on every board from 8192 to the largest supported.

use taxman::frames::{numbering, root_problem_of_setup};
use taxman::model::FrameGraph;
use taxman::numtheory::Tables;
use taxman::root_problem::{self, RootProblem};
use taxman::with_width;

/// The root problem of the board of `t` as setup and the model build it.
fn built_by_setup<const W: usize>(t: &Tables) -> RootProblem {
    let (numbered, frames) = numbering::<W>(t);
    let (_, root) = FrameGraph::build::<W>(t, &frames.columns()).expect("the model of setup's frames");
    root_problem_of_setup(&numbered, &frames, &root)
}

fn check_boards(boards: impl IntoIterator<Item = u32>) {
    for n in boards {
        let t = Tables::new(n);
        let built = with_width!(n, W => built_by_setup::<W>(&t));
        assert_eq!(built, root_problem::build(&t), "n={n}");
    }
}

/// Setup builds Π_n on every board to 200.
#[test]
fn setup_builds_the_root_problem_on_the_boards_to_200() {
    check_boards(2..=200);
}

/// Setup builds Π_n on every board from 201 to 1000 (2 s in the test profile).
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is setup_builds_the_root_problem_on_the_boards_to_200")]
fn setup_builds_the_root_problem() {
    check_boards(201..=1000);
}
