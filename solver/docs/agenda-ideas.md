# Ideas from other domains

Part of the [research agenda](AGENDA.md).

Each with the piece that transfers.  The sources were surveyed for this agenda; "read" marks those read in full or in
the relevant pages, the others were seen in abstracts.

* **Matrix theory: zero patterns over fields of three or more elements.**  D. Hershkowitz and H. Schneider, "Ranks of
  zero patterns and sign patterns", Linear and Multilinear Algebra 34 (1993) 3-19 (read): over a field of at least three
  elements, an r × n zero-nonzero pattern has independent rows for every choice of nonzero entries exactly when its
  bipartite graph has a *constrained* r-matching (the only one between its vertex sets: a uniquely restricted matching),
  and their Example 3.6 shows it fails over GF(2).  With [the counting theorem
  (C2)](../../docs/theory/franklin-moniot.md#the-counting-theorem), a set of moves is playable exactly when it is
  independent under every GF(3) signing.  *Transfers:* the completeness of the signed GF(3) family, and its constructive
  proof (column sums made to vanish), which turns each core the search meets into a signing under which the core is
  dependent (A1, closed).  The minimum-rank literature (zero forcing bounds the maximum nullity over every field;
  minimum rank depends on the field for some graphs) is the general setting.
* **Integer linear algebra.**  A set's rows are independent over GF(p) exactly when p does not divide the gcd of its
  maximal minors, the last determinantal divisor of its Smith normal form (R. P. Stanley, "Smith normal form in
  combinatorics", JCTA 144 (2016), arXiv:1602.00166).  *Transfers:* which fixed fields to add (measurements 7.2's census
  of determinants), and the fact that cores whose divisor is 1 escape every fixed-entry field, so signings are needed.
* **Coding theory, the erasure channel.**  The cores are *stopping sets* of a parity-check matrix whose variable nodes
  are movers and check nodes factors; the even cores are supports of codewords; peeling is the erasure decoder.
  Stopping sets that contain no codeword support, the odd cores, have a name: *coverable* stopping sets (Yakimenka,
  Skachek, Bocharova, Kudryashov, "Stopping redundancy hierarchy beyond the minimum distance", arXiv:1804.06770, read in
  part), the ones redundant checks are added to remove (Schwartz and Vardy, IEEE Trans. Inf. Theory 52 (2006)).  Small
  stopping sets are enumerated by tree search (Rosnes and Ytrehus, IEEE Trans. Inf. Theory 55 (2009); Wang, Kulkarni and
  Poor, IEEE Trans. Inf. Theory 55 (2009), all sets to size 13 at about 500 variables) and by integer programming
  (Velasquez, Subramani and Drager, ISCO 2018, to size 48), and LP decoding on the erasure channel succeeds exactly when
  peeling does (Feldman, Wainwright, Karger, IEEE Trans. Inf. Theory 51 (2005)).  *Transfers:* the enumerators, to list
  a frame's odd cores, for a census of them and for the signings of A1; and the mirror image of A1: there the code is
  fixed and the decoder gains checks, here the game is fixed and the relaxation gains fields.
* **Random XORSAT and hypergraph 2-cores.**  For random 3-XORSAT the 2-core appears at density about 0.818 and full rank
  lasts to about 0.918 (Dubois and Mandler, FOCS 2002; Pittel and Sorkin, CPC 25 (2016)): in between, a non-empty core
  of full GF(2) rank, an odd core, is the typical case.  *Transfers:* a density-evolution model of the frames (movers
  per factor, the share with three live factors) to predict how the cores, and so the gap, grow with n (conjecture C5);
  and the lesson that the odd-core gap is generic, not an accident of a few boards.
* **Uniquely restricted matchings.**  The maximum uniquely restricted matching is NP-hard on bipartite graphs (Golumbic,
  Hirst, Lewenstein, Algorithmica 31 (2001); Müller, Order 7 (1990)), not approximable within n^(1/3 − ε) unless NP =
  ZPP (Mishra, 2011), and fixed-parameter tractable in treewidth in 2^O(tw²) n (Chaudhary, Sau, Zehavi,
  arXiv:2508.12004, 2025).  No integer-programming formulation or polyhedral study of it was found.  *Transfers:* a good
  polynomial bound must use the arithmetic, as GF(2) does; the core inequalities of A1 and the anatomy are new ground;
  the treewidth recursion could close small components exactly after branching.
* **Algebraic combinatorics.**  The weak Lefschetz property of monomial ideals in three variables: the multiplication
  map between degrees has a determinant that counts lozenge tilings, and fails in characteristic p when p divides the
  count.  *Transfers:* product formulas for those counts, to predict which primes see the three-prime cores (conjecture
  C2).  Not surveyed: the statement is to check before relying on it.
* **Matroid optimization.**  Lagrangian decomposition (Guignard and Kim, Math. Programming 39 (1987)) reaches the LP
  over the intersection of the matroid polytopes with two-matroid subproblems; for three matroids that LP's integrality
  gap is 2 (Linhares, Olver, Swamy, Zenklusen, Math. Programming 2020), so it is a bound, not the integer optimum.  The
  k best common independent sets can be ranked (Camerini and Hamacher, SIAM J. Discrete Math. 2 (1989)), and
  intersection under small changes of the matroids has fast algorithms (Blikstad, Mukhopadhyay, Nanongkai, Tu, STOC
  2023).  *Transfers:* the decomposition for A1, with a volume or bundle method for the multipliers (the plain
  subgradient stalled in the per-frame study); ranking for incumbents, and for A3's sizes (A3, closed).
* **AND/OR search** (graphical models).  Branch on a separator, solve the sides independently.  *Transfers:* the frame
  path's homes as separators (A5, closed).
* **Proof logging.**  Gaussian elimination certified in pseudo-Boolean proofs (Gocht and Nordström, AAAI 2021), branch
  and bound certified by aggregation and disjunction steps (VIPR: Cheung, Gleixner, Steffy, IPCO 2017), dominance and
  symmetry breaking certified (Bogaerts, Gocht, McCreesh, Nordström).  No certificate format for matroid-intersection
  duality exists; Frank's weight split with greedy-optimality certificates per matroid is the natural one, checkable
  with XORs and comparisons.  *Transfers:* a log of the solver's tree with each leaf's certificate (a weight split, a
  conflict, a symmetric pair), checked by a small program, would make the code's correctness checkable without the
  prover.  It does not touch the frame model's assumptions, which is why A2 ranks above it.
* **Number theory: the asymptotic constant.**  D. Hensley, "A winning strategy at Taxman", Fibonacci Quarterly 26 (1988)
  262-270 (read): opt(n) / (n²/2) tends to a constant C with 1/2 < C ≤ 35/48, so the taxman keeps a fixed share of the
  pot; and for every prime p, L_p ≤ C ≤ L_p + 2/p, where L_p is a sum over the p-smooth numbers j of the optimal scores
  of the games on the p-smooth numbers up to j.  Nothing later improves it.  Franklín and Moniot (read) compute no value
  of opt(n) themselves (their data are this project's, to 1000), prove that their born-free matching takes at least
  1724/3375 of the pot asymptotically, and pose no open problems; nothing cites them yet.  *Transfers:* a new use for
  exact solvers, proven bounds on C from games on smooth boards (about a thousand 7-smooth numbers below 10^6), a result
  in its own right and a check of the asymptotic regime; and Hensley's blocks {kd : d p-smooth} (play in block k never
  disturbs a larger block), a decomposition of the board by the rough part of each number.  For the gap's growth,
  Landau's count of numbers with k prime factors, x (log log x)^(k−1) / ((k − 1)! log x), says how fast the movers with
  three distinct primes, where all odd cores live, thicken in frames 1-3.

Still open from [the pilots of ideas from other
fields](../notebook/search/ideas-from-other-fields-the-pilots-2026-10/README.md) (2026-10), measured on the matching
path:

* **Knuth's estimate of a tree's size** (rank 3, not piloted): a game's cost before running it, and which units to split
  first ([its section
  5](../notebook/search/ideas-from-other-fields-the-pilots-2026-10/README.md#5-every-idea-considered), "Tree-size
  estimation").
* **Reliability branching** (rank 4): pseudocosts in place of strong branching's exact solves, built only if they
  predict the chosen member and the forced set ([its section
  5](../notebook/search/ideas-from-other-fields-the-pilots-2026-10/README.md#5-every-idea-considered)).
* **The tree's growth at many workers, and the nogood period at 192 threads** (rank 5): A7.
* **The ideas considered and not piloted**, each with its cheapest experiment: [its section
  5](../notebook/search/ideas-from-other-fields-the-pilots-2026-10/README.md#5-every-idea-considered).
