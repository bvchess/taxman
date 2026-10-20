# The root fits

`n{N}.json` is the fit of the multi-field bound (A1) at the root of game `N`: the signings, the multipliers of the
lowest L that the run evaluated, the final master's columns and its rank inequalities.  The module comment of
[`examples/wall/fields/fit.rs`](../../examples/wall/fields/fit.rs) (`SavedFit`) specifies the format.  A file reads back
(`SavedFit::load`) and gives its `bound` again at the root (`SavedFit::bound`), which the run that wrote it asserted
first.  The 52 files, 21 MB, come from the study of the multi-field bound ([the
entry](../../../notebook/search/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md) has the commands and
the tables).

Two things read them by their place here:

* the test `the_study_fits_give_their_bounds_again` (`examples/wall/fields/dw.rs`; `cargo test --features studies` in
  `solver/frame`) takes the fits of 987, 1215, 2700 and 3300 and checks that each gives the bound the study recorded;
* the node-choice measurement's script
  (`solver/notebook/search/the-node-choice-where-the-strong-bound-pays-2026-10/cloud.sh`, with `A1=1`) takes the fits of
  this directory before it fits a root itself (its `FITS`).

## Where the fits come from

* **E2's 17**: the main arm's run of 600 s at the roots 2200 to 3400 by 100, and 3176, 3333, 3335 and 3336 (14 MB).
  Five of the 17 did not converge in 600 s (3100, 3176, 3300, 3336, 3400): their bounds are valid and their
  multipliers close to the LP's.  The study did not keep the fits of the arm with a limit of 16 signings a frame
  (12 MB): it found the limit of 8 the better at the dear roots.
* **The E3 pilot's two**, `n1215.json` and `n1472.json` ([the
  entry](../../../notebook/search/the-a1-bound-in-a-branch-and-bound-the-e3-pilot-2026-10/README.md)): the main arm,
  certified, 300 s; every variant of a game started from its file.  The pilot wrote them in its first format, and a
  conversion to this one kept every value, so the pilot's trees and logs come out the same from them.
* **U4's 33** (987 to 1352, apart from 1215): the roots of the slope test and of its patches ([the
  entry](../../../notebook/search/r3-and-the-slope-test-u4-of-a1-2026-10/README.md)): the main arm, certified, 60 s; all
  but 1350's converged.
