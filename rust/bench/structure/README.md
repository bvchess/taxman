# The census of the frames' structure: the data

What the census checks and why: the module comment of `examples/structure.rs` (it defines every field of a line) and
heuristics/THEORY.md, "The frame model: what is proven and what remains" (Lemma Ω, N and Theorem M, whose hypotheses
it checks).  It ran on the M1 Max, `nice -n 10`, on the code of commit 631e027 (`cargo build --release --example
structure`, from `rust/`).

| file | command | what it is |
|---|---|---|
| `structure_2_8191.jsonl.gz` | `structure 2 8191 --threads 4 --out structure_2_8191.jsonl`, then `gzip -9`; 156 s of wall time, 624 s summed over the boards (0.27 s at most, n = 8129) | one line per board 2..8191, every board the solver supports: the checks A, C, D, E, F and G (no violation on any board), the frames by layer, the layers of every number, the claims of `unzip`, the back claims whose factor has a composite among the factors, the reach of `Home::MergedAway` |
| `summary.txt` | the summary the same run printed | `PASS`: every check holds on every board; frame 0 is layer 3 on 8,100 boards, layer 2 on 80 (all at most 186: exactly the boards where a frame of layer 2 has freedoms), and there is none on the 10 boards with no frame with freedoms; on every board the numbers follow the layers (frame h is the frames with freedoms of layer 3 + h, or 2 + h); `Home::MergedAway` is reached on one board (114), twice, and never counts |

To resume or extend the run into the file, `gunzip -k structure_2_8191.jsonl.gz` first: `--out` appends to a file and
skips the boards it already has.

Setup has numbered the frames by layer since (docs/EXPERIMENTS.md, "The frame model by layer, and the extended
problem"); the census run again on that code (one thread) reproduces every line of the file but for `secs`, and the
summary but for the seconds, so the data stand for both numberings, which are the same on every board here.

The census has since recorded, per board, the frames of the extended problem with every source of three or more prime
factors open (`extended_frames`, the most any set of open sources gives; docs/EXPERIMENTS.md, "One-sided candidates in
the search"), and the files here are its run on that code (`structure 2 8191 --threads 1`, 814 s summed over the
boards): every line of the earlier file is in the new one, field for field but for `secs`, with `extended_frames`
added, and the summary has two rows more.  The extended problem needs 9 frames on 2048..4095 and 10 on every board from
4096, which a frame set (9 at most) cannot hold: certificates past 4095 wait for a tenth frame.

Since Lemma M5⁺ (heuristics/THEORY.md; docs/EXPERIMENTS.md, "The claims of `unzip` in the extended problem") the
census also checks that the extended problem with nothing open and the claims of `unzip` is the frame model's problem
(check H), and records per board the back claims at risk of the lemma's check and those that some set of open sources
refuses (`m5_at_risk`, `m5_refused`, with their counts).  The files here are its run on the code of that change
(`structure 2 8191 --threads 1`, 1,271 s summed over the boards): every field of every line of the earlier file is
reproduced, but for `secs`, with the four fields added; check H holds on every board; the summary has a row for it and
four more.
