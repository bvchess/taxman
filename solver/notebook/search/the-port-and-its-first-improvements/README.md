# The port and its first improvements

Part of the [lab notebook](../../README.md).

Started 2026-10-11.  Concluded 2026-10-11.

**Question.**  The Rust solver reimplements the Java solver's algorithm rather than translating it ([the Java solver and
the frame solver](../../../docs/design/frame-model.md#the-java-solver-and-the-frame-solver)).  Its setup had to
reproduce the Java's values exactly (the golden data in `testdata/golden/` is the check) and every final score had to
equal optimal.json, while the search was designed for speed from scratch.  Is the Java's search sound as written?
**Answer.**  The setup reproduces the Java's values and every final score equals optimal.json, but the Java's upper
bound is not sound: its runner-up gave 358 at the root of n = 114, below the best completion's 360.  The port fixes the
bound.

## Results

**The upper bound's runner-up** (`UpperBoundEstimator.getRunnerUp`; the Java's bound is `legacy_bound` in the port).
When `largestThatFitInFrames` skips candidates because frame 1 or the last frame is at its cap, the Java replaces the
failing number with the next candidate below the smallest bid member.  A skipped candidate from the failing number's
frame can be larger, so the Java bound can be too low: exhaustive checks find it below the best completion, as at the
root of n = 114 (358 against 360).

Two smaller Java details need no port: `Memo.test` computes its value twice (moot, `Memo` is not ported), and
`TxSet.MAX_VALUE = 1023` is gone, the set width being chosen from n.

## Decision

**Kept: the sound runner-up.**  It is the heaviest set within the caps that leaves out some bid member at or above the
failure, which by matroid exchange is the bid with one member swapped for the best outside candidate the caps allow.

**Not kept: the largest candidate not in the bid.**  It is also sound, but it swaps skipped candidates of a full frame
in for members of other frames, and n = 700 took over 10 minutes instead of 18 s.  Re-solving n = 2..760 in Java with
this simple fix changed no answers (n = 740 timed out).

The Java's bound (`legacy_bound`) stayed in the port as a switch, to reproduce the Java exactly, and was removed in
2026-10 ([which tactics earn their keep](../which-tactics-earn-their-keep-2026-10/README.md)).

## Files

* `README.md`: this record; the entry has no data file.

## Commits

* a1dcb51: the Rust solver's design, crate foundation and the Java's golden data, with `legacy_bound` and the runner-up
  defect.
* cfc72ab: the matching bound in the search, where the comparison of the two fixes (n = 700) and the re-solve of
  n = 2..760 in Java were first written down.
