# Morse matchings and Taxman: literature study

The desk study behind [the entry](README.md), copied in as it was written (2026-10-18).

Primary texts were read for Franklín–Moniot (FM), Joswig–Pfetsch (JP), Benedetti–Lutz (BL), Adiprasito–Benedetti–Lutz (ABL), Björner, Severs–White and the SCIP 10 paper. Other items rest on abstracts or HTML summaries. "Inference" means mine, not the source's.

## 1. Is the equivalence right?

**Yes as combinatorics, with two caveats (below).**

- **FM Thm 1** ([2211.00461](https://arxiv.org/abs/2211.00461)): the graph's edges are cover relations of a finite graded poset, an edge carries the weight of its upper end, and the optimum is a max-weight matching with no flat alternating cycle (all vertices in ranks n, n+1).
- **JP §2, §5** ([math/0408331](https://arxiv.org/abs/math/0408331)): a Morse matching is a Hasse-diagram matching whose reversed-arc digraph H(M) is acyclic. Their Observation: every directed cycle lies in one level H_i. MaxMM is matching inequalities plus per-level cycle inequalities, and weights are "easily extended". So "flat" means "adjacent levels" exactly. Either side may be matched, and 1 plays the empty face.
  Inference: M is playable iff each M_i is alternating-cycle-free in H_i. The levels couple only through vertex-disjointness.
- **Caveat 1, graph-theory "acyclic" or "uniquely restricted" matchings** ([Golumbic–Hirst–Lewenstein 2001](https://cris.haifa.ac.il/en/publications/uniquely-restricted-matchings/); [2307.05446](https://arxiv.org/abs/2307.05446); [2508.12004](https://arxiv.org/abs/2508.12004)) constrain the whole induced subgraph G[V(M)]. That is strictly stronger across levels. Example: pairs (1,2),(3,6) are playable (pick 2, then 6), yet 1-2-6-3 is an alternating 4-cycle over ranks 0,1,2,1. Their results transfer only per level. Golumbic et al. were themselves motivated by rank lower bounds for zero/nonzero patterns, which is your Hershkowitz–Schneider route.
- **Caveat 2, not a cell complex.** Acyclic matchings are defined on any poset (Kozlov Def. 11.1, quoted as Def. 3.1 in [Severs–White](https://arxiv.org/abs/1012.3387)). The topological theorem needs a CW complex. [Björner](https://arxiv.org/abs/1101.5704): the squarefree part is a simplicial complex Δ_n, a wedge of spheres, so torsion-free, with Σβ ≈ 2n/π². The whole divisor poset is the containment poset of a non-regular CW complex Δ̃_n (a cell per integer, dim Ω−1, full squares are spheres, Σβ ≈ n/3).
  Inference from Björner §6: with either of his cell strings, Hasse edges that raise a prime exponent to an even value (p to p², p³ to p⁴, ...) have incidence 0. No chain complex with ∂²=0 and every Hasse entry nonzero exists, since 1<p<p² has one path. So Taxman may match edges that are not Morse-legal in Δ̃_n, and its Betti numbers bound only matchings that avoid them.
- **Weights on general digraphs:** [Philip–Vågset, SoCG 2026](https://arxiv.org/abs/2603.05406) use vertex weights on arbitrary digraphs ("feedback Morse matching"). [Minian](https://arxiv.org/abs/2205.11062) extends Morse theory to all finite posets via order complexes. I did not check whether it gives bounds here.

**Inference on your bounds.** |M_i| ≤ rank_F(B_i) holds for any realization of the level pattern that is nonzero on Hasse edges. It needs no homology. JP's Betti inequality (8) is only the sum of two adjacent per-level bounds, so yours are stronger. On the squarefree stratum the ranks equal the Betti-derived ones for every field, because Δ_n has no torsion. They are not homological elsewhere. In pattern language you bound the triangle number tri (the largest triangular submatrix). [Deaett–Grace 2025](https://arxiv.org/abs/2511.02935) prove tri ≤ matroid-min-rank ≤ min-rank over every field, with strict gaps. Their Example 2.2 is an 8×7 pattern with tri 4 but rank ≥5 over every field. So a residual gap is possible for any rank bound, and closing the tri-versus-rank gap is described there as open.

## 2. Exact methods since 2006

- **JP** (above): SCIP and CPLEX 9, 2004 hardware. Matching, Betti, cycle (polynomial separation, facet-defining, Thm 5.2) and Gomory cuts. Non-tight cases: dunce (99 arcs) 385 nodes with cuts against 168,367 without. c-ns (152 arcs) 28 against 16.6M. Poincaré (1112 variables) unsolved after a week. About 1500 variables when Betti is tight, about 150 when not. Cycle and Gomory effects are not separated. *Helps:* the cut family, not the scale.
- **BL** ([1303.6422](https://arxiv.org/abs/1303.6422), Exp. Math. 2014): calls JP the only exact implementation besides full backtracking. Otherwise heuristics, certified only by Betti, π₁ or knot-group bounds. *No help* for exactness.
- **Treewidth DP.** [Burton et al.](https://arxiv.org/abs/1303.7037) give 2^O(w²) for 3-manifolds. [Chaudhary–Sau–Zehavi](https://arxiv.org/abs/2508.12004) give 2^O(tw²) for uniquely restricted matching. [Philip–Vågset](https://arxiv.org/abs/2603.05406) give 2^O(k log k)·n, ETH-tight, on the Hasse diagram's treewidth. I found no benchmarks. *Likely no help:* inference, the 5-prime divisor Hasse graph should have large treewidth. A min-fill bound would show this in minutes.
- **Hardness:** [Bauer–Rathod](https://arxiv.org/abs/1801.08380), [2109.04529](https://arxiv.org/abs/2109.04529) and [Tancer](https://arxiv.org/abs/1211.6254). *No help.*
- **SAT, MaxSAT, PB, CP on Morse, uniquely restricted or acyclic matching:** several searches found none. The analogues are [PACE 2022 DFVS](https://arxiv.org/abs/2208.01119), where a SCIP-ILP entry with lazy cycle constraints and, as reported, an incremental MaxSAT entry both placed well. [CPBayes](https://arxiv.org/abs/2106.12269) is CP with an acyclicity propagator, reported to compare favourably with the SCIP-based GOBNILP. [Grötschel–Jünger–Reinelt 2022](https://www.zib.de/groetschel/pubnew/Juenger-Groetschel-FeedbackArcSet.pdf) report that a modern implementation of their 1985 cutting-plane method is competitive with, mostly better than, lazy cycle enumeration for feedback arc set. [Müller 1990](https://link.springer.com/article/10.1007/BF00383169) proves the single-level problem (alternating-cycle-free matching, via jump number) NP-complete; I found no large exact computations.

## 3. Bounds beyond Betti

Nothing published is directly usable.

- **Topological** (BL §4): c₁ ≥ rank π₁, knot-group and Heegaard bounds. They need a real space. *No help.*
- **Free-face obstruction** (BL §4.1; ABL Lemma 21, [1404.4239](https://arxiv.org/abs/1404.4239)): the dunce hat forces ≥2r+|β| critical cells, proved by peeling. Non-shellability is no obstruction, since JP's MNSB and MNSS are Betti-tight. *Idea transfers.*
- **JP cycle inequalities** are the only computable relaxation that handles the dunce hat. They are level-local, which is what rank bounds miss. *Plausibly helps.* Inference: they are not implied by your rank cuts, and they need edge variables.
- **Inference, peeling inequalities.** In game order, the last-picked member of any set W of same-rank picks has a lower end that no other member of W covers. FM's Thm 6 proof uses the same peeling. So if every lower neighbour of W is covered by ≥2 members (a "stopping set" in LDPC terms), then x(W) ≤ |W|−1, in pick variables only. Every dependent row set of any realization that is nonzero on Hasse edges is such a set. Sharper still is x(S) ≤ tri(S), the largest peelable subset. This is a candidate for the missing non-algebraic part. It is untested.

## 4. Generic solvers

No head-to-head evidence exists for this problem class, so none is shown better suited. Weak analogies in §2 favour lazy cycle cuts in a MIP, or a CP/SAT acyclicity propagator.

- **SCIP 10 exact** ([2511.18580](https://arxiv.org/abs/2511.18580)): MILP only, exact Gomory cuts only, VIPR certificates from conical combinations, Chvátal–Gomory rounding and disjunctions. Presolve is uncertified, and the only separator available in exact mode is Gomory. Inference: certifying a run would need the cycle inequalities placed in the model up front.
- **Pseudo-Boolean:** [VeriPB with RoundingSat and Sat4j](https://drops.dagstuhl.de/entities/document/10.4230/LIPIcs.CP.2025.21) logs proofs. I could not confirm Exact. I found no proof logging for acyclicity constraints.
- **CP-SAT and HiGHS:** no proof logging found.
- **SAT acyclicity encodings** ([Rankooh–Rintanen](https://arxiv.org/abs/2105.12908)) are aimed at sparse graphs, with no certified form found.
- **Inference:** GF(p) rank reasoning has no native counterpart in any of these. Only linear cuts can carry it.

## Verdict

**Ruled out, or very unlikely:**
- Homology, π₁ and knot bounds beyond what you have.
- Treewidth DP, pending a 1-minute min-fill check on the 900-pick graph.
- A SAT or CP-SAT rewrite as a source of better bounds. Nothing in the literature suggests it.
- Betti numbers of Björner's Δ̃_n as a general Taxman bound (inference: valid only for matchings avoiding even-exponent edges).

**Worth a small experiment:** measure root-bound strength on one hard instance with a known optimum. Use HiGHS on the LP only, no search.
1. Add JP edge variables and per-level cycle inequalities, separated by JP §5.2.2 or by enumerating 6- and 8-cycles, to your matching and rank rows. Report the fraction of the 700–1600 gap closed, against your 42%.
2. Add peeling cuts x(W) ≤ |W|−1. Separate by peeling the support of the fractional solution to its maximal stopping set. Compare against GF(2) and GF(3) circuits.

A step 1 or 2 closure well above 42% justifies putting them into the B&B. If not, the gap is not local acyclicity and further search is better spent on branching.
