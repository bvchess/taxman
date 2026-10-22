# The lab (`lab.sh`)

Part of the [runbook](../RUNBOOK.md).

The lab studies the most expensive subtrees of a running search on the spare threads of its machine, to find rules that
make the search faster.  A search keeps its unexplored work as *units*, each the root of a subtree that can be solved
by itself; the [gap](../../../docs/GLOSSARY.md#bounds) of a unit's node predicts its size.  The example
[`tune`](../../frame/examples/tune.rs) solves units one thread to a unit, so two settings compare in steps with no
noise.  A rule that shrinks the largest units is a candidate for the search; whole games decide it.  A person drives
the lab with the commands below, or the chain starts a *study*, an agent session in a sandbox that does the same work
and reports ([Studies](#studies)).

## Rules

* **The chain's files are read-only.**  The lab copies from the chain's checkout and from `~/taxman-work`, writes
  nothing in them, and starts, stops and changes nothing of the chain.  It writes only in `~/taxman-lab`.
* **The lab is light.**  Its processes run at `nice -n 19`, a unit takes one thread, and `LAB_THREADS` (default 16)
  bounds the units at once.  With a chain run under `--background` ([the chain](chain.md#run)), the lab's processes
  at `nice -n 19` still run before the search's workers, and the search takes the threads the lab leaves.

## The commands

`solver/tools/lab/lab.sh` runs from your machine with `LAB_HOST=USER@HOST` set, each command one ssh call.  On the
machine itself, `--local` (or a `LAB_HOST` that names it) runs the commands in place, with no ssh.  The machine needs
git, a rustup toolchain and jq.

* `setup [COMMIT]` clones the repository to `src/` at COMMIT (default origin/master) and builds `taxman` and `tune`.
* `snapshot [N] [--from DIR]` copies the checkpoint and clock of game N in flight (default: the longest own search) to
  `snap/N-STAMP/`, whose name is SNAPDIR below.  DIR is where to copy from, read only; the default is the chain's.
* `units SNAPDIR` writes `units.jsonl` in the snapshot, and prints the summary and the 20 units with the largest gaps.
* `run SNAPDIR IDS` solves the units IDS (ids as `units` printed them, separated by commas), `LAB_THREADS` at a time.
* `reports [N]`, `proof`, `studies [on|off]`, `ask STUDY "QUESTION"` and `request ["TEXT"]` are for the studies.

Arguments after `units` or `run` go to `tune` as they are: `--known FILE`, `--tactic NAME=VALUE`, `--node-limit L` and
`--cpu-limit SECONDS` ([the top of `tune.rs`](../../frame/examples/tune.rs) lists them).  A leading `~/` in one is the
machine's home.

## Reading the results

* `run` prints a row for each unit as it ends: steps, CPU seconds of its thread, wall seconds, the best promotion sum,
  and the state, `done` or `limit` (a limit stopped it).  Compare CPU seconds: at nice 19 the wall seconds run longer.
* A step costs more CPU in some units than in others, so choose `--cpu-limit` to bound the time and `--node-limit` to
  compare settings: a node limit stops at the same step every time, a CPU limit does not.  `tune` writes a progress
  line to stderr every 30 seconds.
* `runs.jsonl` in the snapshot gets the result line of each unit, with the `commit` of the build, and only grows.  If
  the connection drops, the units started go on to their end and add their lines; `pkill -f 'taxman-lab/.*/[t]une'` on
  the machine stops them.
* A unit starts from the checkpoint's incumbent; `--known` with a results file that holds the game starts it from the
  optimum, which makes the unit a proof.

## Studies

The chain tells a program when a game's search passes `--hook-after` seconds, and when that game ends (`--hook`,
[the lab channel](../design/lab.md#the-hook)).  The program is `solver/tools/lab/on-event.sh`, and it starts a study:
an agent session that follows `study.md`.  A study first estimates the work left from the game's units, and decides
whether to steer the search through the command folder (`--lab-dir`, `~/taxman-lab/channel`); the solver validates
every command.  Then it studies what it judges most likely to make the searches faster, from the standing requests to
ideas of its own, and writes a report, with its recommendations ranked, and an entry in a journal.

A study can steer a search only while the search runs, so start it early: `--hook-after 600` in `EXTRA_ARGS` starts one
ten minutes into a game's search, after the chain has written a checkpoint for it to copy.  The default, an hour, suits
a chain whose games take many hours.

**What governs a study comes from the chain's checkout.**  The hook is `~/taxman/solver/tools/lab/on-event.sh`, reviewed
code of the chain's checkout, never a copy in `~/taxman-lab/src`, whose files a study may change.  The settings are
rendered from `settings.template.json` beside it at every start, and the procedure, `study.md`, is read from there at
every start.  Start the chain with `LAB_HOOK` and `LAB_DIR` ([chain.md](chain.md)) naming them.

**The sandbox.**  A study's commands run in the agent program's sandbox, and in nothing else: if it cannot start, the
study does not.

| | |
|---|---|
| may write | `~/taxman-lab/work` (where it starts: the journal, the requests, the reports), `src`, `snap`, `channel` |
| may only read | everything else: the chain's checkout, `~/taxman-work`, `~/taxman-archive`, `ctl/` (the settings, the log, the proof) and `studies/` (the record of each session) |
| network | github.com, for `git fetch` |
| session | the tools Bash, Read, Edit, Write, Glob and Grep; no settings read from any directory it can write; no hooks, MCP servers, memory files or web; a clean environment; permission checks never bypassed |

**The proof.**  `lab.sh proof` runs two short sessions that try what a study must not be able to do, and checks the
files and the exit statuses outside the sandbox: writes to the home directory, to `ctl/`, to the lab's top directory,
to `~/taxman-work`, `~/taxman` and `~/taxman-archive`; appending to the program, `study.md`, the template and its own
settings; planting a settings file in `work/.claude`; and, as controls, writes where it may write and the network.  It
costs a few cents and takes under a minute, and it refuses while a study runs.  Every study needs a passing proof for
the current settings and the installed versions of `claude` and `bwrap`.  A person runs the first proof, and the next
after a failure; when the proof passed for other settings or other versions, as `claude` updates itself, the next study
or ask runs it again, once it holds the lock and the day's limit allows it, goes on if it passes, and logs "proof
renewed".

**Starting, limits and the log.**  A study starts if studies are on, the proof holds, nothing that governs it lies where
the sandbox may write, and the daily limit is not reached.  One study runs at a time: a `long` event is skipped while
one runs.  A `done` event starts a study only if `studies/` holds a study of that game's `long` event, which it compares
the outcome with, so the studies for `done` events are no more than those for `long` events; it waits for the study
that runs.  `LAB_MAX_PER_DAY` (default 8) bounds the studies for `long` events and the asks together per UTC day;
`LAB_MAX_TURNS` (80), `LAB_ASK_TURNS` (30), `LAB_MODEL` (opus) and `LAB_MAX_HOURS` (3) bound a session, which ends
shortly after its first result, with anything it left running in the sandbox.  One command of a session may run for an
hour (`BASH_DEFAULT_TIMEOUT_MS`; a headless session stops a background command after ten minutes otherwise).
`ctl/on-event.log` records every start, skip, refusal and exit; `lab.sh studies` shows how things stand.  To turn
studies off, run `lab.sh studies off` (`on` turns them on again), or start the chain without `LAB_HOOK`.

**Reports, requests and questions.**  `lab.sh reports [N]` lists the reports and prints the newest, with the end of the
journal.  `lab.sh request "TEXT"` adds a dated line to `work/requests.md`, which every study reads after the journal and
marks in place when it has handled it; a request is guidance, and changes neither the settings nor the sandbox.
`lab.sh ask STUDY "QUESTION"` puts a question to a finished study (the name of its folder in `studies/`): its session
goes on with its whole context, under the same sandbox, settings and rules, while no study runs; the answer is printed
and kept in the study's folder (`lab.sh studies` lists the latest names).  The sessions themselves are kept in the agent
program's own store, `~/.claude/projects`, on the machine.

`solver/tools/lab/test.sh` and `test-studies.sh` test the scripts with no machine and no agent session; the second needs
`flock` for its checks of the lock.
