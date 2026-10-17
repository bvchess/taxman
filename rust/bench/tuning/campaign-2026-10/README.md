# The tuning campaign of 2026-10-16

The data of the campaign that chose `force_window=all` as the default: the settings it tried, the scripts that ran it
on one `c8g.16xlarge`, and what came out.  The method, the tables and the findings are in the notebook entry
[the tuning campaign at the end of the chain](../../../docs/experiments/the-tuning-campaign-at-the-end-of-the-chain-2026-10.md);
the harness it used is [the parent directory's](../README.md).

It ran on the build of commit `c70f8f1`, whose default was `force_window=80`.  So the `default` of every file here is
that build's default, `force_window=80`, and so is the `default` of the suite at that commit (`git show
c70f8f1:rust/bench/tuning/suite.jsonl`): the suite in the parent directory has since been measured again with the new
default, and `tune.py --report` on these results with it says, for every problem, that the default's nodes are not the
suite's.  The times in the logs are UTC; the files' times are those of the laptop they were fetched to (UTC-7).

| file | what it is |
|---|---|
| `stageA.json` | stage A's configurations: 33 settings, each against the default (`force_window` 20 to 400, `all` and percentages; `branching`; `strong_limit`; `strong_depth` with `deep_branching`; `more_forced`; `lagrange`; the nogood knobs; `symmetry=off`) |
| `stageB.json` | the first list for stage B (16 settings on top of `force_window=all`), run for about a minute and stopped; nothing of it was kept |
| `stageB2.json` | the list stage B ran (11 settings, without those stage A showed far worse) |
| `campaign.sh` | the script of the machine: stage A, then (it waited for them) stage B and stage C from files put in the bucket; its own stage C was replaced by `stageC.sh` |
| `stageB2.sh` | stage B as it ran, started by hand |
| `stageC.sh` | stage C as it ran: complete proofs of seven games on 64 threads for four settings |
| `out/stageA.jsonl` | stage A's results as the driver wrote them, a line a job (269 jobs ended) |
| `out/stageA_report.txt` | the driver's report on them (at 10 problems for the default) |
| `out/stageB2.jsonl`, `out/stageB2_report.txt` | stage B's results (312 jobs) and report |
| `out/stageC_default.txt`, `stageC_fw_all.txt`, `stageC_all_sl4.txt`, `stageC_all_sl4_mfo.txt` | the ladder's output of stage C for each setting, a line a game |
| `out/stageC.log`, `out/campaign.log` | the machine's logs: the start and end of each run, stage A's report and what stopped |
| `tables.py` | recomputes the tables of the entry from these files: `python3 -I bench/tuning/campaign-2026-10/tables.py` from `rust/` |

A line of a results file is `{"config", "problem", "rung", "settings", "ok", "result": {"nodes", "cpu_s", "wall_s",
"completed", "best", ...}}` (`tune.py` documents them); the cut-off jobs, if any, have `completed` false, and there
were none.  No job found another best sum than the default's.  The scripts are as they ran but for two words in
comments (the project's word for the hardest games is no longer "frontier", which its docs test refuses).
