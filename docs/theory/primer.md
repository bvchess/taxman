# Primer

This primer explains the standard mathematics that the proofs of [the theory](../THEORY.md) use.  It advances nothing:
each fact is in the textbooks, and the primer proves it where the proof is short and teaches something, and otherwise
cites it ([the references](#references) come last).  The examples are Taxman's.  A file can be read with the files it
names.  A reader who knows proofs, sets, graphs and real linear algebra can follow all of it, but for one quoted fact,
linear programming duality.

## Conventions

For a set Y and an element x, Y + x is Y ∪ {x} and Y − x is Y ∖ {x}.  A weight function w on a set E
gives Y ⊆ E the weight w(Y), the sum of w(y) over y in Y.  A set is *heavier* than another if its weight is larger, and
a *heaviest* set among some sets has the largest weight of them: a maximum-weight set, in the standard term.  Where the
theory names no other weights, a number's weight is its value, so a heaviest set is one of largest sum, never one of
most members.  A number s has a *vector* v(s) over a list of coordinates, with a 1 at each of its [maximal
factors](../GLOSSARY.md#the-game) in the list and a 0 elsewhere (6 has the maximal factors 3 and 2); the theory calls it
the incidence vector ([independence](playability.md#independence)).

## The files

| file | read it for |
|---|---|
| [linear-algebra.md](primer/linear-algebra.md) | fields and GF(2), linear independence, rank, circuits, quotients, and the forests of a graph as vectors |
| [greedy.md](primer/greedy.md) | sums over thresholds, the greedy algorithm and its optimality criterion, and matroids, the general setting where the greedy algorithm works |
| [graphs.md](primer/graphs.md) | matchings, permanents and determinants, and topological orders |
| [bounds.md](primer/bounds.md) | sets independent in two families at once, weight splits, weak duality and Lagrangian relaxation |

## References

* J. Oxley, *Matroid Theory*, 2nd edition, Oxford University Press, 2011: Sections 1.1, 1.8, 3.1 and 5.1.
* A. Schrijver, *Combinatorial Optimization: Polyhedra and Efficiency*, Springer, 2003: Theorem 40.1, Corollary 40.2b,
  Section 41.3, Theorem 41.7, Section 41.4 and Corollary 41.12b.
* B. Korte and J. Vygen, *Combinatorial Optimization: Theory and Algorithms*, 2nd edition, Springer, 2002: Chapter 3
  (linear programming duality) and Sections 13.4 (Theorem 13.23), 13.5 and 13.7.
* J. Edmonds, "Submodular functions, matroids, and certain polyhedra", in *Combinatorial Structures and Their
  Applications* (Calgary, 1969), Gordon and Breach, New York, 1970, pp. 69–87.
* J. Edmonds, "Matroids and the greedy algorithm", *Mathematical Programming* 1 (1971) 127–136.
* J. Edmonds, "Matroid intersection", in *Discrete Optimization I*, Annals of Discrete Mathematics 4, North-Holland,
  1979, pp. 39–49.
* E. L. Lawler, "Matroid intersection algorithms", *Mathematical Programming* 9 (1975) 31–56.
* A. Frank, "A weighted matroid intersection algorithm", *Journal of Algorithms* 2 (1981) 328–336.
* M. Guignard and S. Kim, "Lagrangean decomposition: a model yielding stronger Lagrangean bounds", *Mathematical
  Programming* 39 (1987) 215–228.
