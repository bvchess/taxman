# The lab (`lab.sh`)

Part of the [runbook](../RUNBOOK.md).

The lab studies the most expensive subtrees of a running search on the spare threads of its machine, to find rules that
make the search faster.  A search keeps its unexplored work as *units*, each the root of a subtree that can be solved
by itself; the [gap](../../../docs/GLOSSARY.md#bounds) of a unit's node predicts its size.  The example
[`tune`](../../frame/examples/tune.rs) solves units one thread to a unit, so two settings compare in steps with no
noise.  A rule that shrinks the largest units is a candidate for the search; whole games decide it.  A person drives
the lab with the commands below, or the chain starts a *study*, an agent session in a sandbox that does the same work
and reports ([Studies](#studies)).

## The machine's folders

A machine that runs the chain and the lab keeps all of it in one folder of the home directory, `taxman_work`.  Its name
is `TAXMAN_HOME`, relative to the home; [`layout.sh`](../../tools/lab/layout.sh) derives every other folder from it for
the lab's scripts and for `wall.sh status --host`, and no script names them anywhere else.

| folder | what it is |
|---|---|
| `~/taxman_work/chain` | the chain's checkout, which it builds and runs from |
| `~/taxman_work/chain-data` | the chain's results, checkpoints (`ckpt/`), logs, profiles and redeploy builds |
| `~/taxman_work/lab` | the lab: `channel/`, `ctl/`, `snap/`, `src/`, `studies/` and `work/` |
| `~/taxman_work/archive` | archived data, read only |
| `~/taxman_work/research` | a second checkout, for work on the repository beside the chain |
| `~/taxman_work/run.sh` | the script that starts the chain: a local file, not in the repository |

`run.sh` starts `chain/solver/tools/cloud/chain.sh` ([chain.md](chain.md)) with `TAXMAN_WORK` set to the chain's data
folder, `$HOME/taxman_work/chain-data`.  `chain.sh` takes its results file, checkpoints, logs and the worktrees of a
redeploy (`builds/COMMIT`) from `TAXMAN_WORK`, and needs no other variable for the layout.  For the lab, `run.sh` also
sets `LAB_DIR` to the lab's `channel`, `$HOME/taxman_work/lab/channel`, and `LAB_HOOK` to the checkout's
`$HOME/taxman_work/chain/solver/tools/lab/on-event.sh` ([Studies](#studies)); a profile directory in `EXTRA_ARGS`
(`--profile`) goes in `chain-data` too.

## Rules

* **The chain's files are read-only.**  The lab copies from the chain's checkout and from its data folder, writes
  nothing in them, and starts, stops and changes nothing of the chain.  It writes only in its own folder, `lab`.
* **The lab is light.**  Its processes run at `nice -n 19`, a unit takes one thread, and `LAB_THREADS` (default 16)
  bounds the units at once.  With a chain run under `--background` ([the chain](chain.md#run)), the lab's processes
  at `nice -n 19` still run before the search's workers, and the search takes the threads the lab leaves.

## The commands

`solver/tools/lab/lab.sh` runs from your machine with `LAB_HOST=USER@HOST` set, each command one ssh call.  On the
machine itself, `--local` (or a `LAB_HOST` that names it) runs the commands in place, with no ssh.  The machine needs
git, a rustup toolchain and jq.

* `setup [COMMIT]` clones the repository to `src/` at COMMIT (default origin/master) and builds `taxman` and `tune`.
  Run it again with the chain's commit whenever the chain moves to a new build: `tune` refuses a checkpoint whose
  tactics it does not know, and a study would spend its first quarter hour rebuilding.
* `snapshot [N] [--from DIR]` copies the checkpoint and clock of game N in flight (default: the longest own search) to
  `snap/N-STAMP/`, whose name is SNAPDIR below.  DIR is where to copy from, read only; the default is the chain's.
* `units SNAPDIR` writes `units.jsonl` in the snapshot, and prints the summary and the 20 units with the largest gaps.
* `run SNAPDIR IDS` solves the units IDS (ids as `units` printed them, separated by commas), `LAB_THREADS` at a time.
* `reports [N]`, `proof`, `studies [on|off]`, `ask STUDY "QUESTION"` and `request ["TEXT"]` are for the studies.

Arguments after `units` or `run` go to `tune` as they are: `--known FILE`, `--tactic NAME=VALUE`, `--gate GAP`,
`--node-limit L` and `--cpu-limit SECONDS` ([the top of `tune.rs`](../../frame/examples/tune.rs) lists them).  The gate
is the gap from which conflict branching chooses ([the lab channel](../design/lab.md#the-command-folder) sets it for a
search in flight), so a unit can be solved under another gate than the default.  A leading `~/` in one is the
machine's home.

## Reading the results

* `run` prints a row for each unit as it ends: steps, CPU seconds of its thread, wall seconds, the best promotion sum,
  and the state, `done` or `limit` (a limit stopped it).  Compare CPU seconds: at nice 19 the wall seconds run longer.
* A step costs more CPU in some units than in others, so choose `--cpu-limit` to bound the time and `--node-limit` to
  compare settings: a node limit stops at the same step every time, a CPU limit does not.  `tune` writes a progress
  line to stderr every 30 seconds.
* `runs.jsonl` in the snapshot gets the result line of each unit, with the `commit` of the build, and only grows.  If
  the connection drops, the units started go on to their end and add their lines; `pkill -f
  'taxman_work/lab/.*/[t]une'` on the machine stops them.
* A unit starts from the checkpoint's incumbent; `--known` with a results file that holds the game starts it from the
  optimum, which makes the unit a proof.

## Studies

The chain tells a program when a game's search passes `--hook-after` seconds, and when that game ends (`--hook`, [the
lab channel](../design/lab.md#the-hook)).  The program is `solver/tools/lab/on-event.sh`, and it starts a study: an
agent session that follows `study.md`.  A study first estimates the work left from the game's units, and decides whether
to steer the search through the command folder (`--lab-dir`, the lab's `channel`); the solver validates every
command.  It steers on evidence: it solves about 40 of the game's units under the setting in force and under one or two
candidates, and reads [the record of past searches](#the-record-of-past-searches).  Then it studies what it judges most
likely to make the searches faster, from the standing requests to ideas of its own, and writes a report, with its
recommendations ranked, and an entry in a journal.

To set the branching of the games to come as well as of the game in flight, send `{"command": "policy", "from": N,
"conflict_branching": "on", "conflict_look": "on", "gate": 150}` in the same way, with any of the three settings ([the
lab channel](../design/lab.md#a-policy-for-the-games-to-come) has the rest).  A study sends a `policy` only when the
history shows the setting winning on whole games, and none while a request says that an A/B test of the policy is
running.

A study can steer a search only while the search runs, so start it early: `--hook-after 600` in `EXTRA_ARGS` starts one
ten minutes into a game's search, after the chain has written a checkpoint for it to copy.  The default, an hour, suits
a chain whose games take many hours.

**What governs a study comes from the chain's checkout.**  The hook is `solver/tools/lab/on-event.sh` of the chain's
checkout, reviewed code, never a copy in the lab's `src/`, whose files a study may change.  The settings are rendered
from `settings.template.json` beside it at every start, with the folders of `layout.sh`, and the procedure, `study.md`,
is read from there at every start.  Start the chain with `LAB_HOOK` and `LAB_DIR` ([the machine's
folders](#the-machines-folders)) naming them.

**The sandbox.**  A study's commands run in the agent program's sandbox, and in nothing else: if it cannot start, the
study does not.

| | |
|---|---|
| may write | the lab's `work/` (where it starts: the journal, the requests, the reports), `src/`, `snap/` and `channel/` |
| may only read | everything else: the chain's checkout, its data folder, the archive, the research checkout, and the lab's `ctl/` (the settings, the log, the proof) and `studies/` (the record of each session) |
| network | github.com, for `git fetch` |
| session | the tools Bash, Read, Edit, Write, Glob and Grep; no settings read from any directory it can write; no hooks, MCP servers, memory files or web; a clean environment; permission checks never bypassed |

**The proof.**  `lab.sh proof` runs two short sessions that try what a study must not be able to do, and checks the
files and the exit statuses outside the sandbox: writes to the home directory, to `ctl/`, to the lab's top directory,
to the root of the machine's folders and to each of the chain's checkout, its data folder, the archive and the research
checkout; appending to the program, `layout.sh`, `study.md`, the template and its own settings; planting a settings
file in `work/.claude`; and, as controls, writes where it may write and the network.  It
costs a few cents and takes under a minute, and it refuses while a study runs.  Every study needs a passing proof for
the current settings and the installed versions of `claude` and `bwrap`.  A person runs the first proof, and the next
after a failure; when the proof passed for other settings or other versions, as `claude` updates itself, the next study
or ask runs it again, once it holds the lock and the day's limit allows it, goes on if it passes, and logs "proof
renewed".

**Starting, limits and the log.**  A study starts if studies are on, the proof holds, nothing that governs it lies where
the sandbox may write, and the daily limit is not reached.  One study runs at a time: a `long` event is skipped while
one runs.  A `done` event starts a study only if `studies/` holds a study of that game's `long` event, which it compares
the outcome with, so the studies for `done` events are no more than those for `long` events; it waits for the study
that runs.  A run that is stopped also sends `done` for each game in flight, with an incomplete line, and that starts
nothing: the next run resumes the game and sends its `done` when it ends.  `LAB_MAX_PER_DAY` (default 8) bounds the
studies for `long` events and the asks together per UTC day; `LAB_MAX_TURNS` (80), `LAB_ASK_TURNS` (30), `LAB_MODEL`
(opus) and `LAB_MAX_HOURS` (3) bound a session, which ends shortly after its first result, with anything it left
running in the sandbox.  One command of a session may run for an hour (`BASH_DEFAULT_TIMEOUT_MS`; a headless session
stops a background command after ten minutes otherwise).
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

## The record of past searches

Two files in the lab's `work/history/` keep what the searches did and what the studies measured, so that a study
can read the games before its own.  Each holds JSON lines, an object a line, only appended to and never rewritten, and
a reader skips a line that is not JSON.  The hook writes the first file outside the sandbox, and the studies write the
second inside it (they may write in `work/`).

**`games.jsonl`** has a line for each game that ended on the chain.  `on-event.sh` adds a `game` line for every `done`
event whose game's line is complete, before any of the gates that may keep a study from starting (studies off, a study
running, the day's limit, no study of the game's `long` event), so that the file has every game the hook hears of.  A
run that is stopped sends `done` for each game in flight with an incomplete line; those are not recorded, and the next
run sends `done` when the game ends.  A failure to add a line is logged in `ctl/on-event.log` and never stops the hook.

| field of a `game` line | what it holds |
|---|---|
| `kind`, `n`, `at` | `game`; the game; the time of the event, in UTC |
| `score`, `method`, `proof` | the game's line, as [the reference](../REFERENCE.md#a-results-line---out-schema-5) has them |
| `nodes`, `own_seconds`, `threads` | the search's steps over every run of it; the game's own search time; its threads |
| `tactics` | the tactics the search ran with, as the line describes them |
| `lab` | the line's `lab` key: what the lab did to the game (its restarts, its last settings of `conflict_branching`, `conflict_look` and `gate`, the policy that reached it); null for a game the lab did not steer |
| `near_repeat_cut` | the count of nodes that the cut of a near repeat closed, if the line has one; null |

The setting a game ran under is its `tactics` (the run's flags), changed by `lab.policy`, changed by the `lab` settings
of its own, as [the lab channel](../design/lab.md#a-policy-for-the-games-to-come) orders them; a gate that none of them
gives is the default.

The study of a game's end adds one line to the same file, with a single append: a `line_ratio` line, which says how the
game's steps after its snapshot compare with the notebook's fit.  A study takes a *snapshot* of a game in flight
(`lab.sh snapshot`), and the *snapshot line* is the sum, over the snapshot's units, of the steps that the fit gives each
from its gap ([the study procedure](../../tools/lab/study.md) has the formula).  Both numbers are in the study's journal
entry (`snapshot:`).

| field of a `line_ratio` line | what it holds |
|---|---|
| `kind`, `n`, `study`, `at` | `line_ratio`; the game; the name of the study; the time, in UTC |
| `steps_after` | the steps the game took after its snapshot: its steps at its end less its steps at the snapshot |
| `line` | the snapshot line |
| `line_ratio` | `steps_after` over `line`: 3.2 to 3.7 for games 4884 to 4901 |

**`units.jsonl`** has a line for each unit that a study solved to compare settings, and for each whole game it solved to
check a setting.  A study adds the lines after it weighs the settings, whether or not the run ended, under every setting
it ran.  The fields come from the snapshot's `units.jsonl` and from the result lines of `runs.jsonl` that `lab.sh run`
adds beside it.

| field of a `unit` line | what it holds |
|---|---|
| `kind`, `n`, `study`, `at` | `unit`; the game; the name of the study; the time, in UTC |
| `unit` | the unit's id in the snapshot (`4901-17`) |
| `digest` | the first 16 hexadecimal digits of the SHA-256 of the unit's promotions, a colon and its candidates, both as hexadecimal as the snapshot's units file has them: two units with the same sets, as the units of a near repeat have, share it |
| `gap`, `depth`, `cands` | the unit's gap (the bound less what the incumbent needs), the number of its promotions and the number of its candidates |
| `left_out` | how many members of the snapshot's incumbent the unit has left out: in neither its promotions nor its candidates |
| `setting` | the run's changes from the defaults as `name=value`, separated by commas: its tactics (`conflict_look=on`) and `gate=150`; `default` for none |
| `steps`, `cpu_seconds` | the unit's steps and its CPU seconds under that setting |
| `ended` | whether the run finished the unit (false: a limit stopped it, and `steps` is a lower bound) |

A `whole` line is for a game solved from its root to check a setting (`tune solve N --ids root`).  It has `kind`, `n`,
`study`, `at`, `setting`, `steps`, `cpu_seconds` and `ended` as above, and `threads`, the threads the run used.
