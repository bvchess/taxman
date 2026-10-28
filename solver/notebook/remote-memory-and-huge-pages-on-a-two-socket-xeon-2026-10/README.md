# Remote memory and huge pages on a two-socket Xeon (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-28.  Concluded 2026-10-28.

**Question.**  Would the chain's steps get cheaper on its two-socket machine if the engine kept each worker on one NUMA
node, or if the searchers' memory were in huge pages?  **Answer.**  No.  A step whose memory is all on the other socket
costs 3 to 4% more CPU, and about a tenth of the chain's CPU time runs against memory on the other socket, so keeping
each worker on one node would save about 0.4%.  Huge pages for all the memory that malloc hands out save about 1%.
Neither reaches the 3% that would pay for the code, and nothing was built.

## The machine

The chain's machine has two sockets of Xeon E5-2699 v4 (Broadwell), each with 22 cores of two hardware threads, 256 KB
of L2 a core and 55 MB of L3 a socket.  Its 756 GiB are two NUMA nodes: CPUs 0-21 and 44-65 are node 0, and 22-43 and
66-87 node 1.  It runs Linux 5.15 with the kernel's automatic NUMA balancing on (`kernel.numa_balancing` is 1), which
samples the pages each thread touches and moves pages and threads toward each other.  Transparent huge pages are given
only where a program asks for them (`madvise`), and the C library is glibc 2.35.

The chain's solver runs 87 engine threads under the idle scheduling policy (`SCHED_IDLE`), and the engine does not pin
them.  Each searcher allocates its state on its own thread, so the first touch puts that state on the node where the
thread runs at the time ([choosing the machine](../../docs/runbook/machine.md#instance)).  The thread may later run on
the other socket.

## Method

The measurements used `tune solve` at commit ff89271, built with `cargo build --release --features studies --example
tune` (no `-C target-cpu=native`), on the 24 units of game 4984 that the step rounds of the [root
gap](../search/the-root-gap-and-the-size-of-a-game-2026-10/README.md) used.  The settings were the chain's since game
4995: conflict branching on at a gate of 1,200, price reuse, the warm start, and the carried store of 144 units of 4983.
Each pair solved a unit twice at once on the two hardware threads of one core, so that both saw the same core, caches
and clock, and the two swapped hardware threads on even runs.  There were four runs of the 24 units, on the cores of
CPUs 2 and 3 (node 0) and 24 and 25 (node 1), while the chain ran at idle priority on the other CPUs.

1. *What remote memory costs a step.*  In each pair one solve had its memory bound to its core's node and the other to
   the other node.  `tools/membind.py` sets the memory policy `MPOL_BIND` (`set_mempolicy`) and then runs `tune`; the
   kernel's NUMA balancing does not move pages under such a policy.  During the runs, `tools/where.py` found every
   anonymous page of each solve, 57 to 63 MiB, on the node it was bound to.
2. *What share of the chain's time runs remote.*  `tools/share.py` read, every 5 s for 5 minutes, each engine thread's
   CPU and CPU time and the kernel's count of its NUMA faults by the node of the page (`/proc/PID/task/TID/sched`).
   These hinting faults are the kernel's sample of the pages a thread touches.  The share is the CPU time between
   samples, each thread's weighted by the part of its faults on the node other than its CPU's.  It ran twice, from
   08:17 to 08:25 PDT, 9 to 17 minutes after the chain had started, in its searches of games 5049 and 5052.
3. *Huge pages.*  Both solves of a pair ran the same binary, one under glibc's tunables.  `glibc.malloc.hugetlb=1` asks
   for huge pages (`madvise`) on the main heap, which holds what the main thread allocated: the game, the carried store
   and the shared tables.  It does not, in glibc 2.35, reach the arena of a worker thread, where a searcher's state is
   (`tools/huge.py` found 38 of 67 MiB of anonymous memory in huge pages, none of it in the worker's arena of 21 MiB).
   With `glibc.malloc.arena_max=1` as well, every allocation comes from the main heap, and 56 of 64 MiB were in huge
   pages.  The arena setting alone was the control.

## Results

Every pair took the same steps and found the same best.  The CPU of the second solve over the first:

| pair | CPU | geometric mean | a unit | the four runs |
|---|---|---|---|---|
| memory on the other node, over memory on the core's own | 1.039 | 1.033 | 0.87-1.32 | 1.020-1.080 |
| huge pages on the main heap only, over none | 0.997 | 0.998 | 0.97-1.11 | 0.994-1.000 |
| huge pages for all of malloc's memory (one arena), over none | 0.989 | 0.984 | 0.92-1.02 | 0.983-0.994 |
| one arena without huge pages, over the default | 1.001 | 0.998 | 0.95-1.01 | 0.997-1.005 |

The share of the chain's time that ran remote:

* 9.4% and 9.6% of the engine threads' CPU time, weighted by the part of their faults on the other node.  98.6% of the
  faults were on pages private to the thread, and by those alone the share was 9.1%.
* Within the 5 minutes, 78 of 128 and 80 of 107 engine threads ran on both nodes.  A thread was on another node than at
  the sample before 0.7 to 0.9 times a minute, and 7 to 9% of the samples found a thread off the node it ran on most.
* Over the whole machine, the kernel took 74 to 82 thousand hinting faults a second, 91.5 to 92% of them on the faulting
  CPU's node, and moved 2.6 to 3.0 thousand pages a second (10 to 12 MB/s), nearly all of them the engine threads'.  The
  chain's system time was 0.16% of its user time over 30 s, so the balancing itself costs little.

## Decision

Neither change was built.  Keeping each worker on one node would save the remote share times the remote cost, about
0.095 × 0.035 to 0.04: 0.3 to 0.4% of the chain's CPU.  Huge pages save 1.1% even for all of malloc's memory, and
`madvise` on the searcher's largest allocations could save no more than that.  The 3% threshold failed in both.

A step's waits on memory, which made cycle counters overstate earlier changes two to three times, are therefore mostly
not on the other socket's memory, nor on page walks; the caches near the core are the likelier place.  Placement by node
would be worth measuring again on a machine where the other socket costs much more, as on the two-socket Graviton4 of
[scaling to 192 threads](../scaling-to-192-threads-2026-10/README.md), whose second socket made a step 7 to 10%
dearer, or if the kernel's NUMA balancing were off, which would leave a moved thread's memory remote for good.

## Files

* `tools/membind.py NODE COMMAND...`: runs a command with its memory bound to one node, as `numactl --membind` would.
* `tools/pairnuma.sh IDS OUT REPS BINARY`: the pairs of item 1, the files `OUT/local-ID-REP.json` and `remote-...`.  Run
  as `pairnuma.sh set24.txt pen1 4 base-tune`.
* `tools/pairenv.sh IDS OUT REPS TAG_A BIN_A ENV_A TAG_B BIN_B ENV_B`: the pairs of item 3.  Run as `pairenv.sh
  set24.txt thpB 4 base base-tune X=1 thp base-tune GLIBC_TUNABLES=glibc.malloc.hugetlb=1:glibc.malloc.arena_max=1`, and
  so for `thpA` (`thpmain`, `glibc.malloc.hugetlb=1`) and `thpC` (`arena1`, `glibc.malloc.arena_max=1`).
* `set24.txt`: the 24 units of 4984.
* `solves.jsonl`: every solve of the four sets of pairs, one line each (`run`, `tag`, `id`, `rep`, `cpu_s`, `nodes`,
  `best`, `completed`), made by `tools/collect.py pen1 thpA thpB thpC`; `tools/summary.py solves.jsonl` prints the table
  above.
* `share-2.jsonl.gz`, `share-3.jsonl.gz`: the two samplings of item 2, a line a sample, by `tools/share.py PID 300 5
  OUT` (the second also records each thread's private faults apart); `tools/analyze.py` prints the share from either.
* `tools/where.py NAME`, `tools/huge.py NAME`: the checks of where a running solve's pages are, and of its huge pages.
