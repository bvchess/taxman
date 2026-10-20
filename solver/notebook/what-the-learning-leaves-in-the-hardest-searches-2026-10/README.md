# What the learning leaves in the hardest searches (2026-10)

Part of the [lab notebook](../README.md).

**Question.**  Once the search learns its conflicts' images under the cheap exchanges, where do a hard unit's steps
go, and which conflicts does it still have to compute: images of conflicts it met, under exchanges the learning does
not make, or something else?

**Answer, on two units of the old tuning suite.**  The learning leaves the anatomy of a tree as it was: the same
exclusions on a path, at the same prices, closing at the same gap.  It removes breadth, not depth: fewer alternatives
for each exclusion.  The conflicts it still computes are mostly ones it met before.  At 3300-14, 57% are a nogood the
search learned and then forgot at a clearing of its table, and 26% are a conflict met before whose nogood no longer
fits the node.  Exchanges outside the learning's class explain at most 8%.  So a wider closure has little to catch.
Keeping the nogoods used since the last clearing (the tactic `nogood_keep=hit`) is no answer either: it moves the trees
by the same factor with the learning as without it, 0.36-0.37 at 3300-14 and 1.46 at 3441-1262.

The study follows [conflicts closed under cheap exchanges](../conflicts-closed-under-cheap-exchanges-2026-10/README.md),
whose learning took 20.6% off the steps of the tuning suite and only 4.5% off the CPU of its whole games.  It has three
stages: the instruments, built and tried on two units of the old [tuning
suite](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md) (this section); the same questions on a suite
of the chain's hardest searches near 4,500; and tests of the ideas they suggest.

## Words

* *The learning* is `wall images --learn --closure`: whenever a check computes a conflict, the search learns the
  conflicts within its images under the cheap twin swaps and paired exchanges, and the images of those, up to 64 sets
  ([cheap exchanges, "Words"](../conflicts-closed-under-cheap-exchanges-2026-10/README.md#words)).
* *Unit*, *exclusion*, *price* and *home* are [the anatomy's](../the-anatomy-of-a-hard-subtree-2026-10/README.md#words);
  *conflict*, *nogood* and *forced member* are the [glossary's](../../../docs/GLOSSARY.md).
* A check's conflict is *computed* when the searcher's table of nogoods gives none within the heaviest set, and *known*
  when it does.  A known conflict comes from the search's *own* nogoods (those of the conflicts it computed) or from
  *image* nogoods (those the learning added).
* A nogood is a conflict of a node together with some of the node's promotions, enough to keep a frame of the root
  unwinnable: its *witness*.  Every nogood of these units lies in one frame: its members are of two adjacent homes.

## What was run

**The instrument.**  `wall anatomy` takes `--learn [--closure]` and `--root` (examples/wall/anatomy.rs, with its
tests).  With `--learn` the search learns as `wall images --learn` makes it, and the anatomy's trace of each step is
written as before.  Two things are new, with or without the learning:

* The hook keeps a mirror of the searcher's nogood table, with the source of each nogood, own or image.  The trace
  says, for each known conflict, which source gave it.  The mirror is exact: on every run here it explained every
  known conflict.
* `ID.nogoods.tsv` lists every nogood the search learned, in order, with the step, the conflict, and the node's
  promotions, heaviest set and candidates.

`residual.py` reads them.  `profile` puts two searches of a unit side by side, as the anatomy took one apart.  `misses`
takes each computed conflict and looks, among the nogoods learned before it, for one that a few moves of members turn
into a nogood that would have given the conflict at its node.  It names each move by the factorizations: a twin swap
(m·p for m·q), a kin move (m·u for m·v, u and v at most 16), a rescaled payer (m·p for m′·p), or unrelated.  A twin swap
is in the learning's class when the learning would have proposed it at the earlier nogood's node; if not, `misses` says
why (a swap up, a dear swap, a promoted member moved, a twin that was no candidate there).

**The units**: 3300-14, the old suite's biggest, and 3441-5459, its small contrast at the same gap
([the anatomy](../the-anatomy-of-a-hard-subtree-2026-10/README.md#what-was-run)).  Each was solved on one thread from
the known optimum with fresh tables, on this laptop under `nice -n 10`, with and without the learning, from the build
of `--profile test`.  The trees are those of the earlier studies, step for step: 1,248,609 and 790,609 steps at
3300-14, 52,855 and 27,073 at 3441-5459.  Every search found the optimum and completed.

## Where the steps go

The learning changes how many leaves a tree has, and little else:

| | 3300-14 default | learning | 3441-5459 default | learning |
|---|---|---|---|---|
| steps | 1,248,609 | 790,609 | 52,855 | 27,073 |
| leaves | 104,400 | 66,012 | 4,706 | 2,473 |
| exclusions on a path to a leaf, median (p10-p90) | 7 (5-10) | 7 (5-10) | 5 (4-7) | 5 (3-7) |
| price of an exclusion, median | 125 | 123 | 153 | 153 |
| exclusions priced under 50 | 19.9% | 19.7% | 3.7% | 6.2% |
| gap at which a leaf closes, median | 94 | 91 | 169 | 173 |
| leaves^(1/exclusions): alternatives an exclusion | 5.2 | 4.9 | 5.4 | 4.8 |
| a decade of subtree per ... points of gap | 238 | 241 | 245 | 262 |
| conflicts computed, of the checks with one | 0.8% | 0.5% | 0.8% | 0.7% |
| known from image nogoods (alone or with own) | 0 | 15.6% | 0 | 14.8% |
| distinct conflicts branched on; branchings a conflict | 20,780; 35.4 | 15,114; 30.8 | 2,027; 15.0 | 1,096; 14.4 |

* **What did not change.**  A path still pays the gap in 7 (or 5) exclusions at a median price of 123-153.  A fifth
  of 3300-14's exclusions are still cheap, under 50: the learning does not make the bound pay more for leaving a member
  out.  The subtree of a node still grows a decade for every 240-260 points of its gap.
* **What changed.**  The alternatives for each exclusion: leaves^(1/exclusions) falls from 5.2 to 4.9 and from 5.4 to
  4.8, and with 7 exclusions on a path that is the whole gain (0.94^7 ≈ 0.63).  The image nogoods give 15% of the known
  conflicts, and the same share of the conflicts the branching resolves; the rest of their work is ruling candidates
  out of children, which the trace does not count.
* **What a check computes.**  Under 1% of the checks that find a conflict compute it, with the learning or without; the
  rest find it in the table.  The learning cuts the computed conflicts by 58% at 3300-14, more than the steps (37%).

## What the learning misses

Each conflict the search still computes, in the first group that holds it (`residual.py misses`):

| group | 3300-14 default | learning | 3441-5459 default | learning |
|---|---|---|---|---|
| conflicts computed | 6,095 | 2,549 | 233 | 115 |
| the same nogood, met before and forgotten | 61.1% | 57.0% | 8.6% | 0.0% |
| the same conflict, another nogood | 28.4% | 26.0% | 37.8% | 34.8% |
| a cheap twin swap, the learning's class | 0.7% | 0.1% | 3.4% | 0.9% |
| one twin swap outside the class | 0.9% | 1.8% | 2.6% | 4.3% |
| two twin swaps | 1.9% | 2.6% | 6.4% | 6.1% |
| three twin swaps | 0.2% | 0.4% | 3.4% | 0.9% |
| kin moves or rescaled payers | 2.4% | 3.0% | 3.4% | 7.8% |
| unrelated moves, or none within 3 moves | 4.3% | 9.1% | 34.3% | 45.2% |

* **Forgotten nogoods.**  The searcher clears its table every 50,000 steps.  At 3300-14, 57% of the conflicts the
  learning still computes are nogoods it had learned before a clearing.  3441-5459 is solved in fewer steps than that.
* **The same conflict with another witness.**  A quarter to a third are a conflict the search had computed before,
  whose nogood no longer fits: its witness holds promotions the node has not made, and the node has others that
  serve.  Of the 662 such conflicts at 3300-14 with the learning, the closest earlier nogood of the same conflict
  holds one member that the node has neither promoted nor in its heaviest set in 57%, two in 14%, and none in 26%
  (that nogood would have fitted, and was forgotten); at 3441-5459, one in 34 of 40.  For example, the conflict {936} at 3300-14 had a witness with 1215 = 3⁵·5 in it; at a node without 1215 its
  new nogood holds 810 = 2·3⁴·5, 1215's twin, and 1404 instead.
* **Exchanges outside the learning's class** explain 8% at 3300-14 and 19% at 3441-5459 together, none of them more
  than 8%.  The largest are two twin swaps of one prime pair on two payers (2.6-6.1%), kin moves, twin swaps up, dear
  twin swaps, and twin swaps of a member that the earlier node had promoted.  Three-prime cycles and rescaled payers are
  rare.  Proposing every twin swap of a conflict's members at its node would cost 44-48 sets a computed conflict at
  3300-14, against the closure's 15.
* **The calibration.**  Without the learning, 0.7% (3300-14) and 3.4% (3441-5459) of the computed conflicts are first
  explained by a cheap twin swap that the learning proposes; with it, 0.1% and 0.9%.  So the instrument sees what the
  learning removes.

## A first test: keeping the used nogoods at a clearing

The largest group is the forgotten nogoods, so the cheapest test keeps them.  The tactic `nogood_keep=hit` keeps, at a
clearing, the nogoods used since the last one.  [Keeping and
forgetting](../keeping-and-forgetting-nogoods-at-the-end-of-the-chain-2026-10/README.md) found that it makes ten whole
games 28-31% larger on one thread.  The question here was whether the learning, whose classes a clearing throws away,
changes that.  It does not: keeping moves each unit by the same factor with the learning or without it, and the factor
swings from a third to one and a half.

| unit | default | `nogood_keep=hit` | change | learning | learning, `nogood_keep=hit` | change |
|---|---|---|---|---|---|---|
| 3300-14 | 1,248,609 | 464,165 | −62.8% | 790,609 | 453,073 | −42.7% |
| 3441-1262 | 1,099,410 | 1,602,571 | +45.8% | 648,846 | 950,397 | +46.5% |

The cells are steps; the CPU seconds follow them (308, 118, 204 and 117 at 3300-14; 342, 506, 220 and 326 at
3441-1262).  The best and the conclusion are the optimum's in all eight.  A unit's tree is a chaotic function of the
nogoods it holds, as [the tuning campaign](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md) found, so
two units cannot say whether keeping pays on average; the earlier entry says it does not.  The run of 3440-12 was
stopped before it ended, to save the budget.

## Next

* **Stage 2** applies `profile` and `misses` to the suite of the chain's hardest searches, and to its whole games,
  to ask why whole games gain less than units.
* **The witness.**  A quarter of what the learning computes is a conflict it knew under another witness.  Each nogood
  lies in one frame, and its witness is promotions of the two homes that touch that frame.  A test: with each
  computed conflict, also learn its nogood with one witness member exchanged for a twin or kin among the root's
  candidates, each checked by a peel from the root.  That costs a few hundred peels for each computed conflict, and
  computed conflicts are under 1% of the checks.
* **Not a wider closure.**  The exchanges the closure lacks explain at most 8% of the computed conflicts at 3300-14.

## Files

* `residual.py`: `profile` and `misses`, the tables of this entry; its module comment has the usage.
* `runs.jsonl`: the lines of `wall anatomy` (the four traced searches) and of `wall images` (the tests of keeping).
* The traces, 125-190 MB a big unit, are not kept, because a one-thread solve of a unit is the same every time.  From
  `solver/frame`, after `cargo build -j 2 --profile test --example wall --features studies`:
  `target/debug/examples/wall anatomy --suite ../notebook/the-tuning-campaign-at-the-end-of-the-chain-2026-10/suite.jsonl
  --known ../../results/strict.jsonl --ids 3300-14 [--learn --closure] --trace DIR 3300`, and the same with `--ids
  3441-5459` and `3441`; then `python3 -I residual.py profile DIR_DEFAULT DIR_LEARNING 3300-14 3441-5459` and
  `python3 -I residual.py misses DIR 3300-14 3441-5459`.  The tests of keeping are `wall images` with `--learn --closure`
  or `--count`, and `--tactic nogood_keep=hit`.

Commits: the commit of this entry (`wall anatomy --learn`, `--closure`, `--root`, the mirror, the nogood log, with
their tests).
