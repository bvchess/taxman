# Faster nodes, same trees (2026-10)

Part of the [lab notebook](../../README.md).

The inner loop, made faster without changing any answer or tree: every
change keeps the node counts of every game and the fingerprints of the
probe (`examples/perf.rs`, see DESIGN.md, "Performance"), and was measured in cycles and
instructions retired of the process (other load moves wall times by up to
3×), on one thread, plain release build.  Before, at the 1215 proof: the
Hungarian repairs about 50% of the time, the peels 19%, the bounds from the
potentials 5%, the nogood table 8%, level copies 4%.

| commit | change | 1215 window k cycles/step | 936 G cycles | 1184 | 1215 proof |
|---|---|---|---|---|---|
| e3b8ddc | before | 22.1 | 1.59 | 4.33 | 151.3 |
| b4cb2b6 | set widths of 20, 24 and 28 words (see "The engine", below) | 20.4 | 1.59 | 4.05 | 142.3 |
| ab76772 | radix heap's bucket 0 a bitset, the search's per-vertex fields in one slot, a level settled only when something changed | 18.8 | 1.47 | 3.65 | 129.3 |
| 130e4ec | `MAX_FRAMES` 12 → 8 (every board up to 4095 has at most 8) | 18.7 | 1.46 | 3.58 | 125.9 |
| 9bfb4d6 | nogood lookups exact over compact member indices, not signatures and member loops | 18.2 | 1.43 | 3.52 | 123.2 |
| 319753b | no peel for a promotion whose new move has a factor no other move uses (a sixth of them) | 18.0 | 1.41 | 3.53 | 122.5 |
| ad1237c | nogood rule-outs over the compact sets too | 17.8 | 1.35 | 3.49 | 118.9 |
| 4b6ef38 | the radix heap's push and pop and `add_inner` inlined (they were calls, 300 a step) | 16.1 | 1.23 | 3.16 | 106.2 |

In all 30% fewer cycles (1.42×) and 28% fewer instructions on the proof of
1215 (33 s instead of 47 s), 27% at 1184, 23% at 936; the probe's windows
27% (1215) and 26% (936).  Profile-guided optimization, 12% on the code
before, now adds 0–2% (see DESIGN.md, "Performance").  The strict chain 1..1231 on 8
threads takes 279 s of game time (1201–1231: 145 s), every score as
committed.  Where the time goes now (1215): the Hungarian repairs 46%, the
peels 24%, the bounds from the potentials 7%, the node's own work 7%,
level copies 4%, nogood lookups 3%.

Measured and not kept (cycles, one thread):

* An undo trail for the matching levels instead of a copy per push: pushes
  cost little, but logging every change of a repair costs more (windows
  +0.8% and +0.9%, instructions +4%).
* The peel in frame-local form (each frame's movers and factors numbered
  within it, kept in the frame set): with the bit-parallel peel in rounds the
  checks of single promotions were 17–26% cheaper, but the frames of forced
  sets need many rounds (windows +8% and +16%); with the counting peel on
  local indices, +0.5% and +0.8%.
* Caching frame checks: 14–17% of the peels in a window repeat an earlier
  one exactly, but only 5% one of the last eight.
* The no-peel test above for the several moves of a forced set; promoting a
  forced set straight into the child's state; stopping the bound from the
  potentials once its first part decides; the Dijkstra horizon (no events
  later than a known settling one: windows +0.4% and +2.5%, as in an earlier
  measurement); one branch for `reach`'s two tests: each within ±0.5% or
  worse.
* Unchecked indexing in the Hungarian search: −4% instructions but −0.7%
  cycles, not worth `unsafe`.
* Widths of 12 and 48 words: 3–4% at 512–767 (solved) and some at 2048–3071,
  for 12 s more of every build.
* Not done: `i32` potentials (no bound on the potentials is proven, so not
  exact by construction); a warm-started unit rebuild (another optimal
  matching can change the tree after a split or a resume); candidate-index
  sets for every search-level set (a large refactor); watched members for
  the nogoods (now 3% of the time).
