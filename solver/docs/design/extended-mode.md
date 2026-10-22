# Extended mode: one-sided candidates

Part of the [design](../DESIGN.md).

The search ([search.md](search.md)) can also take the *extended problem* Π⁺(O), the problem that a certificate of U
searches ([certificates.md](certificates.md)).  It is the frame model with a set O of the sources of three or more prime
factors that setup keeps made optional (THEORY.md, ["The extended problem"](../../../docs/theory/extended-problem.md),
and the corollary of Theorem M⁺; `frames::build_extended`).  Besides the candidates of the frame model, which are
*two-sided* (out of one frame, into the one in front), it has *one-sided* candidates: the members of O and the sources
that greedy rejected beside them, each a possible move of its *own* frame.  It is research code: the solver's own path
never builds an extended problem, and with no one-sided candidate every piece below is what it is without them.

*The claims of `unzip`* (THEORY.md, Lemma M5⁺).  The problem comes in two formulations, which have the same promotion
sets (the lemma's corollary): without any claim, and, the default, with the claims of the frame model
(`SearchFrames::claims`) that the lemma keeps.  `build_extended` gives each claim the frame, as setup forms it, of its
source.  A frame keeps its claims if it holds no member of O ∪ R and the factor of each of its back claims passes the
check (`frames::ClaimCheck`): it is no two-sided candidate, and no candidate has it among its maximal factors.  The
pairs of a frame that keeps them leave the moves and the factors before the root is reduced; the freedoms and the
candidates do not change.  `ExtendedFrames` records the claims kept and those refused (`refused`).  With nothing open
every claim is kept, and Π⁺(∅) is Π_n: the same problem, so the same trees (`tests/tripwires.rs`; the census's check
H).  `Board::extended(open, claims)` is the one place the board's claims are passed.

The check matters.  A frame that holds an open source has more freedoms, and can take on composites of a claimed factor
that Π_n could not.  At 24 with 16, 20 and 24 open, 12 becomes a candidate that lists 6, which 18 claims; taking the
pair out anyway loses the promotion set {8, 12}, which the oracle catches.  The census counts, game by game, the
claims that some O refuses (`m5_refused`).

* **The model** (`model`, "One-sided candidates").  `FrameGraph::build_extended` gives each one-sided candidate its own
  frame (`own`), and the factor graph lists its maximal factors there, as it does a move's.  Promoting one adds it to
  that frame's moves if the frame stays winnable; it takes no factor, and spends one freedom of that frame only
  (`frames_touched`).  The frame-by-frame check of a set, the reduction, conflicts and nogoods need nothing more: a
  one-sided member is one more move of its frame.  The state does not record which were promoted (a promoted
  two-sided candidate is no longer a factor of its home), so each is promoted once along a path, which the search's
  candidates ensure.  `FrameGraph::narrow` keeps a one-sided candidate while its frame has a freedom, and
  `FrameGraph::max_promotions_among` adds their number to the freedoms' count (sound, not tight).
* **The GF(2) bound** (`gf2`, "One-sided candidates"; THEORY.md, Theorem 11 for Π⁺).  A one-sided candidate's element
  has the vector of its maximal factors in its own frame's matroid, and is a coloop of the other: its own unit vector,
  in a *free block* after the frames' blocks, `64·words` of them to a block, so that hundreds of them widen no vector.
* **The search.**  `Board::prepare_extended_with(open)` (`search::prepare_extended`) makes them candidates of the
  search.  The problem's form ends with them, after a tag that no plain problem's form has, and the checkpoint header
  (format 5) records their digest, so neither a repeat nor a resumed checkpoint can take an extended problem for
  another.  A checkpoint of version 4 resumes only a search without them ([trust.md](trust.md)).
* **What it refuses** (`SearchTactics::refusal_with_one_sided`).  With one-sided candidates the search needs `gf2`: the
  matching bound gives a one-sided candidate no vertex, so it is no bound there (the dive, which rounds the root's
  matching for a starting incumbent, still runs and never picks one).  It refuses `lagrange`, which is built on the
  matching bound, and `symmetry`: Theorem 13 is proven for the frame model, and for Π⁺ each prime exchange would have
  to map the one-sided candidates of each frame onto themselves, which nothing checks.  `SearchTactics::extended` is
  the default without `symmetry`.  Π⁺(∅) has no one-sided candidate and takes any tactics, and its search is the
  default engine's on the same problem, node for node (`tests/tripwires.rs` has a table of its own).
* **Its frames.**  Π⁺(O) has a frame for each layer from the lowest with freedoms to the highest: nine at most up to
  4095 (layers 3 to 11), whatever O is.  From 4096 it has ten with every source open, which `build_extended` refuses
  (`MAX_FRAMES`), naming the game and the layers.  The census of the frames records the count with every such source
  open for every game to 8191 ([the claims of `unzip` in the extended
  problem](../../notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/README.md)).  Opening a source also adds
  one to its frame's rank, so a problem with many open sources can need more words of the GF(2) bound than the narrow
  set widths compile (three, rank 192) or the widest (five, rank 320): `search::prepare_extended` refuses it, saying so,
  rather than fail in the search.
