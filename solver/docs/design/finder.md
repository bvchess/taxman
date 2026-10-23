# The finder

Part of the [design](../DESIGN.md).

A search that starts well below its final best spends most of its steps before the incumbent catches up, and a finder
catches it up sooner.  A search that ends within a few seconds has little to gain, so a finder runs only once a search
has taken some steps: with `--finder-after NODES` (default 1000000, a few seconds on many threads; 0 starts none),
when a game's search has taken NODES steps (over every run of it, a resumed search counting the runs before), one
thread of its own starts beside the workers, for the seconds of `--finder-seconds` (default 60), and ends sooner if
the search does.  Each game of a window decides for itself.

The thread runs *rounds* of large neighbourhood search ([glossary](../../../docs/GLOSSARY.md), the finder;
`search::finder`).  A round fixes a random share of a set's promotions (70 to 90%), searches the rest exactly as a unit
of the engine on one thread for at most 1,024 steps, takes a better set, and repeats; a round lasts 15 seconds and draws
with a seed of its own, and the generator is seeded, so the same game and seed make the same rounds.  The first round
and every odd one starts from what survives of a seed of the search, the seeds in turn (the survivors are offered as
they are, being a game of their own); the others start from the search's incumbent as it is, and go on from a better one
the search finds meanwhile.  A search without a seed has the second kind of round only.  A resumed search has no seed in
its start, but its finder has the game of n − 1 all the same.

*Why it cannot make an answer wrong.*  The finder only offers sets.  A set it offers is checked as a checkpoint's
incumbent is on resuming ([trust.md](trust.md), the paragraph "What identifies a checkpoint"): a set of candidates that
can be promoted from the root, worth what it claims (`search::offer_found`).  It then reaches the incumbent the way a
worker's improvement does (`Control::offer`), and the workers prune against it as against any promotable set; so the
search stays exhaustive, and its proof is the search's own.  A set that fails the check is refused and ends the finder
(`refused` in the line).  The finder keeps nothing in the checkpoint's header: a checkpoint resumes whatever the
finder's settings are.

*What it costs and what it records.*  The finder's units are searched in a space of their own (its matching graph, its
GF(2) matroids, its symmetric pairs: a second of setup at n near 4,900), so its counters are not the search's.  It takes
one core beside the workers for its seconds.  The result line records it where it ran (`engine.finder`: the search's
steps when it started, the incumbent then, the best it reached, how many of its sets became the incumbent, its seconds
and when it reached its best), and `options` records the flags where given.
