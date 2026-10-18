# The port and its first improvements

Part of the [lab notebook](../README.md).

The Rust solver reimplements the algorithm of the Java solver rather than translating it (DESIGN.md, "The Java
solver and this one").  Its setup had to reproduce the Java's values exactly (the golden data in
`testdata/golden/` is the check) and every final score had to equal optimal.json; the search itself was
designed for speed from scratch.  One defect of the Java was found and fixed on the way:

* **Upper bound runner-up (a defect of the Java's bound, fixed in the port; `legacy_bound` brings it back)**
  (`UpperBoundEstimator.getRunnerUp`): when `largestThatFitInFrames` skips candidates because frame 1 or the last frame
  is at its cap, the Java replaces the failing number with the next candidate below the smallest bid member.  A skipped
  candidate from the failing number's frame can be larger, so the Java bound can be too low (exhaustive checks find it
  below the best completion, e.g. at the root of n=114: 358 against 360).  Sound version: the heaviest set within the
  caps that leaves out some bid member at or above the failure, which by matroid exchange is the bid with one member
  swapped for the best outside candidate the caps allow.  (Simply taking the largest candidate not in the bid is also
  sound but swaps skipped candidates of a full frame in for members of other frames; it made n=700 take over 10 minutes
  instead of 18 s.)  (Re-solving n=2..760 in Java with the simple fix changed no answers; n=740 timed out.)
* `Memo.test` computes its value twice — moot, `Memo` is not ported.
* `TxSet.MAX_VALUE = 1023` — gone; set width is chosen from n.
