# The claims of `unzip` in the extended problem (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  32.2 of the fifth consultation of the theory track (2026-10-16): Π⁺(O) claims nothing, which costs time
(stage 1 measured +11% time on Π⁺(∅)) and keeps repeats apart (3427 has 3425's search problem but another Π⁺,
differing only by the pair (3426, 1713) that `unzip` claims in Π_n; 115 repeats of the work list, 361 thread-hours,
3427 alone 223, are deferred for that reason).  Can Π⁺(O) keep the claims where they are still moves of every
instance, so that Π⁺(∅) is Π_n and more repeats rest on the game they repeat?

*The rule, as designed* (8c833a1).  `build_extended(t, open, claims)` takes the frame model's claims
(`SearchFrames::claims`, in the order made) and gives each the frame, as setup forms it, of its source.  A frame keeps
its claims if it holds no member of O ∪ R and the factor f of each of its back claims passes the check
(`frames::ClaimCheck`): f is no two-sided candidate of Π⁺(O), and no candidate has f among its maximal factors; its
pairs then leave the moves and the factors of its frame of Π⁺(O) before the root is reduced.  The candidates do not
change (a claimed factor is none: debug-asserted), nor do the freedoms or in_the_bag⁺.  `ExtendedFrames` records the
claims kept and those refused.  THEORY.md: Lemma M5⁺ (the claims kept are front and back moves of every instance of
Π⁺(O), so Theorem M⁺ holds with them out), its corollary (the two formulations have the same promotion sets, so a
certificate in one bounds the heaviest promotion set in the other), and Π⁺(∅) with the claims is Π_n.  The
formulation is a parameter of `Board::extended` and of everything that sets Π⁺(O) up.

*Tests* (8b0d0d4, 6146627).  Theorem M⁺ against the whole-board oracle with the claims kept, as before without them:
every set both ways on every board 2..80 with O empty and three random O (1.66 million sets), 2..60 with every source
open (4.2 million), and, ignored, 81..100 and 61..69; random sets at 300, 600, 1000, 1300, at 741, 1100 and 1200 (a
rejected source in some sets) and, ignored, at 2000, 3000 and 4095; the search's optimum against the oracle on every
board to 80 (60 with every source open): no mismatch.  The mutation that skips the check (the refused pairs taken out
anyway) is caught on every problem to 80 where the check refuses a claim, 24, 25 and 26 with all of U\*_{≥3} open but
one frame's sources (2 to 8 playable sets the mutant does not take: at 24 with 16, 20 and 24 open the candidate 12
lists 6, which 18 claims, and the mutant loses {8, 12}), while the true problem agrees with the oracle on every set
there.  Π⁺(∅) with the claims poses Π_n's problem (frames, candidates, root, greedy set, `max_total`) on every board
to 300 in the unit tests and on every board 2..8191 in the census (check H, below), and its search reproduces every
row of the tripwires' default table node for node, the matching rows too.

*The census* (`examples/structure.rs`, one thread, 1,271 s summed; `solver/notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/`, replacing the earlier run, every
field of which it reproduces).  Check H holds on every board 2..8191.  Where Π⁺(U\*_{≥3}) fits in nine frames (to
4095) it records the back claims at risk (their factor fails the check against the candidates of Π⁺(U\*_{≥3}), the
most any O has) and those refused for some O (exactly: for each frame holding one at risk, the largest O free of it,
since the candidates only grow with O).  The check refuses claims on 10 boards to 500 (24-26 and 343-349), on most
from 507 and on every one from 1501, and a growing share of them: 2.5% of the back claims of 501..1000, 5.3% of
1501..2000, 9.2% of 3001..3500, 11.3% of 3501..4095 (at the worst O for each frame).

*Certificates* (3841753).  Lines record `"claims": true|false` (absent: false, every line made before);
`certify --no-claims` makes the old formulation.  The rule of the same problem compares the two problems whole in the
run's formulation, whatever formulation m's line was made in (by the corollary m's certificate bounds the heaviest
promotion set of either): a line of the other formulation is passed over unless its frames and candidates, which the
claims do not change, are n's, and is otherwise set up again in its own formulation (it must reproduce its line) and
then in the run's.  `verify-certificates` sets each line up in its own formulation and checks the rule in the
formulation of the line that rests on the other.
* *600..1300*, one thread, with the claims (`solver/notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/certificates_claims_600_1300.jsonl`): the 377 games
  certified again, none otherwise; the rule settles 101 of them against 81 (the same 81 and 20 more, 611 on 609 to 1147
  on 1145), each as a direct search of the game in the same formulation does; the 276 searched take exactly the nodes
  they took without the claims (296,743) and 29.4 s against 30.5 s.  `verify-certificates` passes the file beside the
  one without the claims, and a file that mixes the two.
* *Past 1300* (`solver/notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/problems_claims_600_3440.jsonl`, every certificate's Π⁺(O) set up with the claims, 78
  s): the claims change the problem of 346 of the 1,575 games that need a certificate (231 of 377 in 600..1300, 43 of
  490 in 1301..2175, 72 of 708 in 2176..3440, none past 2688), and the rule finds 361 repeats against 340: one more
  past 1300 (1371).  None of the 115 deferred repeats is settled, 3427 neither: 3425 and 3427 keep their problems of
  before.  At 2112 and 2156 (|O| 69 and 70), certified three times each way, alternating, on one thread: the same
  problem (digest) with and without the claims, so the same trees (15,119 and 43,599 nodes) and times (3.42 and 7.70
  s).
* *Why.*  The frame of layer 3 that holds most of a board's sources (joined through the factors 2p, 3p, 6, 9, 10, 15,
  ...) holds open sources as soon as some are open, and then every claim of it is refused (no certificate's problem
  keeps a claim past 2688), the pair (3426, 1713) among them,
  although its check passes (1713 has no maximal composite but 3426, and layer 2 is closed): 3426 = 6 · 571 shares the
  factor 6 with that frame, which at 3427 holds 1719 = 9 · 191, open (3426, 6, 1866 = 6 · 311, 933, 2799 = 9 · 311, 9,
  1719 is a path of sources and their maximal factors).  The condition
  "holds no member of O ∪ R" is what the lemma's proof uses to know which members of the instance list a claimed
  factor; it does not need the whole frame free.

*Kept claim by claim: measured, not kept* (a change of the design, for the next consultation).  The proof needs, for a
back claim (s, f), only that no member that can be in an instance lists f but s, after the earlier pairs, and for a
front claim only that s lists f alone after them.  So each claim can be checked on its own, in the order made: a claim
(s, f) with s fixed (not open) is kept if, after the claims kept before it, (front) every maximal factor of s but f is
the factor of a claim kept before it, or (back) f passes the check and every maximal composite of f other than s that
is a source of s's frame or a member of R in it is the source of a claim kept before it.  The proof is Lemma M5⁺'s, by
induction on the claims kept: a kept front claim's s, fixed, lists at most f once the earlier kept pairs are made, and
f ∉ L as in the lemma; a kept back claim's f is not in L (the check), and of the members that can list it (the sources
of s's frame in F, or in K with O ∪ R, the members of L, the members of K of other frames) every one but s is the
source of an earlier kept claim, fixed and so already paired, or ruled out by the check or by its own frame.  With O
empty the conditions are those under which `unzip` claimed, so every claim is kept and Π⁺(∅) is still Π_n.  It
replaces the frame-wide test of `build_extended` (twenty lines; `max_factors` and `max_composites` of the tables) and
was measured on this code, then taken out:
* every default and ignored test of `tests/model_extended.rs` passes with it (the oracle both ways, the search, the
  mutation caught);
* the problems of 600..3440 with it: 465 repeats settled against 340 without claims and 361 with the rule as designed,
  among them 60 of the 115 deferred, 278 of their 361 thread-hours, 3427 on 3425 (one problem, d0b64c26c38bf1b2), and
  every problem changed by the claims;
* 2112 and 2156, three times each way, alternating: the same trees, 3.22 s against 3.53 s (8.7% less) and 7.60 s
  against 7.97 s (4.6% less) a certificate.

*Checks.*  No tree of the default mode changed: the probe's fingerprints 7e8e724717d26545 and f3deaac0dbdc030a
(`gf2=on`), the tripwires, `regress --to 1000 --threads 1` (9.2 s), the one-thread chain 1..1300 line for line equal,
but for the times and the build, to the base commit's binary (ed09ac0; 290,503 nodes), both suites, clippy and fmt.
The laptop ran one process at a time, on one thread, `nice`; nothing ran on AWS.

## Files

What the census checks and why: the module comment of `examples/structure.rs` (it defines every field of a line) and
docs/THEORY.md, "The frame model: what is proven and what remains" (Lemma Ω, N and Theorem M, whose hypotheses
it checks).  It ran on the M1 Max, `nice -n 10`, on the code of commit 631e027 (`cargo build --release --example
structure`, from `rust/`; since the feature `studies` was added, `--features studies` too).

| file | command | what it is |
|---|---|---|
| `structure_2_8191.jsonl.gz` | `structure 2 8191 --threads 4 --out structure_2_8191.jsonl`, then `gzip -9`; 156 s of wall time, 624 s summed over the boards (0.27 s at most, n = 8129) | one line per board 2..8191, every board the solver supports: the checks A, C, D, E, F and G (no violation on any board), the frames by layer, the layers of every number, the claims of `unzip`, the back claims whose factor has a composite among the factors, the reach of `Home::MergedAway` |
| `summary.txt` | the summary the same run printed | `PASS`: every check holds on every board; frame 0 is layer 3 on 8,100 boards, layer 2 on 80 (all at most 186: exactly the boards where a frame of layer 2 has freedoms), and there is none on the 10 boards with no frame with freedoms; on every board the numbers follow the layers (frame h is the frames with freedoms of layer 3 + h, or 2 + h); `Home::MergedAway` is reached on one board (114), twice, and never counts |

To resume or extend the run into the file, `gunzip -k structure_2_8191.jsonl.gz` first: `--out` appends to a file and
skips the boards it already has.

Setup has numbered the frames by layer since ([The frame model by layer, and the extended
problem](../the-frame-model-by-layer-and-the-extended-problem-2026-10/README.md)); the census run again on
that code (one thread) reproduces every line of the file but for `secs`, and the summary but for the seconds, so the
data stand for both numberings, which are the same on every board here.

The census has since recorded, per board, the frames of the extended problem with every source of three or more prime
factors open (`extended_frames`, the most any set of open sources gives; [One-sided candidates in
the search](../one-sided-candidates-in-the-search-2026-10/README.md)), and the files here are its run on
that code (`structure 2 8191 --threads 1`, 814 s summed over the boards): every line of the earlier file is in the new
one, field for field but for `secs`, with `extended_frames` added, and the summary has two rows more.  The extended
problem needs 9 frames on 2048..4095 and 10 on every board from 4096, which a frame set (9 at most) cannot hold:
certificates past 4095 wait for a tenth frame.

Since Lemma M5⁺ (docs/THEORY.md; solver/notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/README.md) the
census also checks that the extended problem with nothing open and the claims of `unzip` is the frame model's problem
(check H), and records per board the back claims at risk of the lemma's check and those that some set of open sources
refuses (`m5_at_risk`, `m5_refused`, with their counts).  The files here are its run on the code of that change
(`structure 2 8191 --threads 1`, 1,271 s summed over the boards): every field of every line of the earlier file is
reproduced, but for `secs`, with the four fields added; check H holds on every board; the summary has a row for it and
four more.

### The certificates and the extended problems with the claims

Made on the certificates' data (the work list of [the certify tools](../../tools/certify/README.md), `U` being its four `--upper` files, `solver/noframe/data/upper_*.jsonl`):

| file | command | what it is |
|---|---|---|
| `certificates_claims_600_1300.jsonl` | `taxman certify 600-1300 --results results/strict.jsonl --upper solver/noframe/data/upper_2_1300.jsonl --out certificates_claims_600_1300.jsonl --threads 1 --progress 0` (code of commit cd959d1, the extended problem with the claims of `unzip` that Lemma M5⁺ keeps; 29 s of search) | the same 377 games, every one `certified` again, each line with `"claims": true`: 276 by a search (the very nodes of the searches above, 296,743) and 101 by the same problem, the 81 above and 20 more whose problems differ only by claims (611 on 609, ...); `verify-certificates` passes it beside the file above |
| `problems_claims_600_3440.jsonl` | `extended problems 600 3440 U --known results/strict.jsonl --out problems_claims_600_3440.jsonl` (code of commit cd959d1, with the claims; 78 s) | `problems_600_3440.jsonl` again in the formulation with the claims (each line with `"claims": true`): 361 games with a `same_as` against 340 (the 20 of 600..1300 and 1371); the claims change the problem of 346 of the 1,575 games (231 of the 377 of 600..1300, 43 of the 490 of 1301..2175, 72 of the 708 of 2176..3440, none past 2688): the frame of layer 3 that holds most sources holds open ones and keeps no claim, so none of the 115 repeats the rule could not settle (3427 among them) is settled (solver/notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/README.md) |
