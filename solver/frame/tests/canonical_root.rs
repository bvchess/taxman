//! Setup builds the root problem Π_n of the theory (docs/theory/frame-model.md): U\* (the sources of the frames as
//! setup forms them), the candidates, and the moves and factors of each frame of the model at the root are those of
//! [`taxman::root_problem::build`], which builds Π_n from the definitions with no code of `frames` or `model`.  The
//! census `examples/structure.rs` at commit 3ca11bf (check I) did the same on every board the solver supports.

use taxman::frames::numbering;
use taxman::model::FrameGraph;
use taxman::numtheory::Tables;
use taxman::root_problem::{self, RootProblem};
use taxman::with_width;

/// The root problem of the board of `t` as setup and the model build it.
fn built_by_setup<const W: usize>(t: &Tables) -> RootProblem {
    let (numbered, frames) = numbering::<W>(t);
    let mut ustar: Vec<u32> = numbered.frames.iter().flat_map(|frame| frame.sources.to_vec()).collect();
    ustar.sort_unstable();
    let (_, root) = FrameGraph::build::<W>(t, &frames.columns()).expect("the model of setup's frames");
    let root_frames = root.frames().iter().map(|frame| (frame.moves.to_vec(), frame.factors.to_vec())).collect();
    RootProblem { ustar, candidates: frames.candidates.to_vec(), frames: root_frames }
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
