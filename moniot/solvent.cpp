// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// ------------------------------------------------------//
//                                                       //
//                  Solvent Heuristic                    //
//                                                       //
// ------------------------------------------------------//


#include "defs.h"
#include "graphdefs.h"
#include "TaxmanGraph.h"

// This heuristic finds a feasible set of picks and their respective
// pay, but not in play order.  The edges (pay[p],p) for each pick p
// define a cycle-free matching, which is returned by solvent.
// Putting the picks into play order uses the same function as
// BreakCycles and BornFree.

// This implementation closely follows the structure of the python
// version in GitHub at bvchess/taxman.  The peel routine is in
// heuristics/core.py and the other routines are in
// heuristics/strategies/solvent.py.  Note, however, that the
// operations may not proceed in the same order as in the Python
// implementation, or on different C++ platforms, due to differences
// in implementation of unordered sets and maps.  The picks and scores
// will be the same.  The play order of the picks may differ since we
// use a different algorithm to find the sequence.

// Block comments and some inline comments are from the Brian Chess Python code.

// solvent: matching-based full-game approximation of optimal Taxman play.

// The full-game generalization of "take the highest prime": consider
// n, n-1, ..., 2 and accept each number if the pick set stays playable.
// Per candidate the decision has three outcomes:

//   * the incremental augmenting search fails -> reject outright.  By
//     Berge's lemma (a matching is maximum iff no augmenting path exists
//     from a free vertex), the failure already proves no matching covers
//     the candidate; nothing can overturn it.
//   * the search succeeds and the updated matching's precedence relation
//     is acyclic -> accept.
//   * the search succeeds but the matching has gone precedence-cyclic ->
//     ask peel, which decides exactly.  This path is load-bearing:
//     some sets have perfect matchings, every one of them cyclic (first
//     at n=21), and peel correctly refuses those -- it is a
//     playability oracle, strictly stronger than a matching test.

// A rejection is permanent either way: playable sets are downward-closed,
// so a set unplayable now can never become playable by adding more.

// The key fact (sharpened by the lifting lemma to maximal factors only):

//     A set S of selections can all be played, in some order, if and only
//     if there is a matching that assigns each selection a distinct
//     maximal factor outside S, such that the precedence relation "a
//     before b whenever a's assigned factor divides b, or a divides b" is
//     acyclic.

//     Playing any topological order works: a selection's assigned factor
//     cannot be swept earlier (everything divisible by it comes later), and
//     a selection that is itself someone's divisor is played before being
//     swept.  Conversely, a real game induces such a matching (any paid
//     divisor lifts to a surviving maximal factor) and its play order is a
//     linear extension.

// Under the hood this is classic bipartite maximum matching, applied
// greedily.  _augment is Kuhn's algorithm (the Hungarian-style
// alternating DFS); the accept loop is plain descending greedy: keep
// what stays playable.  Matching gives a hard ceiling (every game's
// picks pair with distinct outside factors, so no game's upper half can
// outscore the max-weight matchable set); this strategy's playability
// test is stricter than matching, so its upper picks sit at or below
// that ceiling, and that they equal the optimal game's upper half
// exactly is verified for n <= 1000, not proven.
// _is_acyclic and _playable_order are Kahn's
// topological sort doing cycle detection and schedule construction.  The
// one nonstandard ingredient is the interplay between matching and
// scheduling: matchability alone is NOT playability, which is exactly why
// the cyclic path above must consult peel.

// Convenience typedefs used by solvent routines
typedef std::unordered_set<int> vertexSet;
typedef std::pair<int,int> vertexPair;
typedef std::vector<vertexPair> ListOfVertexPair;
typedef std::unordered_map<int,int> intDict;
typedef std::unordered_map<int,vertexSet> setDict;
typedef std::unordered_map<int,vertexList> listDict;


// This routine creates the table of maximal factors for each x from 1
// to n.  The maximal factors are those divisors d of x such x/d is a prime.
// This routine takes advantage of the fact that the Taxman graph is precisely
// the graph whose edges link each vertex to its maximal factors.
// The entry in the table for x is a list of the maximal divisors of x.
ListOfVertexList maximal_factor_table( TaxmanGraph& g ) {
  ListOfVertexList all_maximal_factors;
  int n = g.num_vertices();

  all_maximal_factors = ListOfVertexList(n+1); // index 1 to n
  all_maximal_factors[1] = vertexList();	      // 1 has no factors
  for ( int c = 2; c <= n; c++ ) {
    vertexList factors = vertexList();
    // Loop through vertices adjacent to c, taking those that are
    // smaller, so they are factors, not multiples.
    for ( edgeHandle h : g.neighbors(c) ) {
      edge e = g.get_edge(h);
      int u = e.left;
      int v = e.right;
      if ( u > v ) std::swap(u,v); // make u the smaller
      if ( u > 0 && u < c ) {	   // u is a factor of c
	factors.push_back(u);
      }
    }
    all_maximal_factors[c] = factors;
  }
  return all_maximal_factors;
}


bool augment(int v,
	     intDict& owner,
	     vertexSet& selected,
	     const ListOfVertexList& mf,
	     vertexSet& visited,
	     ListOfVertexPair& trail) {
    // Kuhn's augmenting search: find v a candidate factor, reassigning others.

    // The textbook alternating-path DFS for bipartite matching: try each of
    // v's factors; a free factor ends the path, an owned one recurses on its
    // holder to re-route it.  Success extends the matching by one; failure
    // (with the full `visited` set explored) proves, via Berge's lemma, that
    // no matching covers v alongside the current picks -- callers treat that
    // as a conclusive rejection.  `mf` is the candidate-payment pool (maximal
    // factors); every owner change is recorded on the trail so a failed
    // composite operation can be rolled back exactly, undo-log style.

  vertexList mfvrev(mf[v].size()); // put factor list into descending order
  std::reverse_copy(mf[v].begin(), mf[v].end(), mfvrev.begin());
  for ( int f : mfvrev ) {
    if (selected.count(f) || visited.count(f)) continue;
    visited.insert(f);
    auto holderi = owner.find(f);
    int holder;
    if (holderi == owner.end()) holder = 0; // 0 signifies None
    else holder = (*holderi).second;
    if (holder == 0 ||
	augment(holder,owner,selected,mf,visited,trail)) {
      trail.push_back({f, holder});
      owner[f] = v;
      return true;
    }
  }
  return false;

}


bool is_acyclic(vertexSet& selected, intDict& owner, int n) {
  intDict indeg;
  listDict succ;
  intDict match;
  for (int c : selected ) {
    indeg[c] = 0;
    succ[c] = {};
  }
  for (const std::pair<int,int> o : owner) {
    int f = o.first;
    int c = o.second;
    if (selected.count(c)) match[c] = f;
  }
  for (int a : selected) {
    vertexSet seen;
    for (int start : {match.at(a), a}) {
      for (int b = 2 * start; b <= n; b += start) {
	if (selected.count(b) && b != a && !seen.count(b)) {
	  seen.insert(b);
	  succ[a].push_back(b);
	  ++indeg[b];
	}
      }
    }
  }

  
  vertexList ready;
  for (int c : selected) {
    if (indeg[c] == 0) ready.push_back(c);
  }

  std::size_t done = 0;
  while (!ready.empty()) {
    int a = ready.back();
    ready.pop_back();
    ++done;
    for (int b : succ[a]) {
      if (--indeg[b] == 0) ready.push_back(b);
    }
  }

  return (done == selected.size());
}

void rollback(intDict& owner, ListOfVertexPair& trail) {
  for (vertexPair step : trail) {
    int f = step.first;
    int previous = step.second;
    if (previous == 0) 		// None
      owner.erase(f);
    else
      owner[f] = previous;
  }
}


// peel determines whether a given set of picks is playable.  It does
// not find the play sequence, but determines whether the set satisfies
// the conditions to permit an acyclic matching to be found.
// On success, returns the matching as an intDict mapping pick -> pay.
// On failure, returns an empty matching.

intDict peel(TaxmanGraph& g,
	     vertexSet& selections,
	     vertexSet& factors,
	     setDict& avail
	  ) {
    // Order the selections of a factor game so every one can be taken.

    // Implements the recursive procedure from the wiki iteratively:

    //     peel(C, F):
    //         if C is empty: return []
    //         if some c in C has only a single factor f in F:
    //             return [c] + peel(C - {c}, F - {f})
    //         if some f in F is a maximal factor of only one c in C:
    //             return peel(C - {c}, F - {f}) + [c]
    //         ERROR: cannot select all members of C using F

    // Implementation note (why this is fast): the naive reading rescans
    // every member at every level, O(|C| * E).  Instead, `single_factor`
    // and `single_comp` are worklists of vertices whose live degree just
    // became 1 -- the same pattern as Kahn's topological sort maintaining
    // its in-degree-zero queue.  Each edge is deleted exactly once inside
    // remove_pair, doing O(1) work plus possibly enqueueing a neighbor, so
    // the whole run is O(V + E).  With E = sum of distinct-prime counts
    // (average ~2 per member), this is by far the cheapest operation in
    // the project -- linear in a very sparse graph.

    // Semantics note (easy to get wrong): failure here does NOT simply
    // mean "no perfect matching exists".  Some sets have perfect payment
    // matchings, every one of which is precedence-cyclic and hence
    // unplayable; the forced-move discipline rejects those too.  Both
    // directions are theorems (THEORY.md: "The schedulability theorem",
    // "The completeness theorem"): with maximal-factor pools, success
    // here is exactly playability.  See the n=21 example.

  // Make copies of these since they get clobbered but caller needs them
  // to stay the same.
  vertexSet c_set(selections);
  vertexSet f_set(factors);

  // Live bipartite adjacency between selections and factors.
  setDict fac_of;		// Available factors of candidate pick c
  setDict comps_of;		// Potential payers if factor f is taken
  for (int c : c_set) {
    for (int f : avail[c] ) {
      if (f_set.count(f)) {
	fac_of[c].insert(f);
      }
    }
  }
  for (int f : f_set ) comps_of[f];
  for (int c : c_set) {
    for (int f : fac_of.at(c)) {
      comps_of[f].insert(c);
    }
  }
  
  vertexSet single_factor;
  vertexSet single_comp;
  for (int c : c_set) {
    if (fac_of[c].size() == 1) single_factor.insert(c);
  }
  for (int f : f_set) {
    if (comps_of[f].size() == 1) single_comp.insert(f);
  }

  auto remove_pair = [&](int c, int f) {
    c_set.erase(c);
    f_set.erase(f);
    single_factor.erase(c);
    single_comp.erase(f);
  
    vertexSet c_facs = std::move(fac_of.at(c));
    fac_of.erase(c);
    for (int other_f : c_facs) {
      if (other_f == f) continue;
      vertexSet& comps = comps_of.at(other_f);
      comps.erase(c);
      if (comps.size() == 1) single_comp.insert(other_f);
    }
  
    vertexSet f_comps = std::move(comps_of.at(f));
    comps_of.erase(f);
    for (int other_c : f_comps) {
      if (other_c == c) continue;
      vertexSet& facs = fac_of.at(other_c);
      facs.erase(f);
      if (facs.size() == 1) {
	single_factor.insert(other_c);
      } else if (facs.empty()) {
        return false;		// c has no remaining factor
      }
    }
    return true;
  };				// end remove_pair definition


  // Remove one member of a vertexSet, returning the value removed
  auto pop_any = [](vertexSet& s) {
    auto it = s.begin();
    int x = *it;
    s.erase(it);
    return x;
  };

  // Form the matching pick -> pay.  The matched edges are also
  // recorded in the graph.

  intDict matching;

  // Loop on selected picks, looking for those with a single remaining
  // factor or whose maximal factor is unique to it.  When a pair
  // {f,c} is found meeting one of these criteria, it is added to the
  // matching and the vertices c and f are removed from c_set and f_set.
  while (!c_set.empty()) {
    while (!single_factor.empty()) {
      int c = pop_any(single_factor);
      if (fac_of.at(c).empty()) {
	return intDict();
      }
      int f = *fac_of.at(c).begin();
      matching[c] = f;
      // Record the match in the graph
      auto edgepair = g.find_edge(f,c);
      edgeHandle h = edgepair.first;
      g.match_edge(h);
      if (!remove_pair(c, f)) return intDict();
    }
    if (c_set.empty()) break;
    if (!single_comp.empty()) {
      int f = pop_any(single_comp);
      if (!f_set.count(f) || comps_of.at(f).size() != 1) continue;  // stale entry
      int c = *comps_of.at(f).begin();
      matching[c] = f;
      // Record the match in the graph
      auto edgepair = g.find_edge(f,c);
      edgeHandle h = edgepair.first;
      g.match_edge(h);
      if (!remove_pair(c, f)) return intDict();
    } else {
      return intDict();
    }
  }

  return matching;
}

    // Decide playability of `target` exactly via the factor-game reduction.

    // Tier 1's cycle retry only reshuffles the candidate's own coupon; a real
    // cycle can require OTHER picks' coupons to move too, which falsely
    // vetoes playable candidates (see strategies.seteval.SetEval, which
    // proves this out).  This is the complete tier: reduce target to a
    // bipartite factor game -- each pick maps to the set of its candidate
    // factors (maximal factors) NOT in target -- and let peel decide
    // exactly, returning a full pick -> factor matching, or None if no
    // assignment covers every pick.
 
intDict complete_matching(TaxmanGraph& g,
		       vertexSet& target,
		       const ListOfVertexList& mf) {
  setDict avail;
  vertexSet factors;
  for (int c : target) {
    vertexSet& s = avail[c];
    for (int d : mf.at(c)) {
      if (!target.count(d)) s.insert(d);
    }
    factors.insert(s.begin(),s.end()); // union of all avail values
  }

  return peel(g,target,factors,avail);

}

    // Solvent matching approximation of a full game; returns the sequence.

    // `mf` is the candidate-payment pool: each pick's maximal factors (f with
    // pick/f prime), as built by core.maximal_factor_lists.  This is the
    // correct pool for the matching question ("can every pick reserve a
    // distinct payment?") -- the lifting lemma proves it matching-equivalent
    // to the full proper-divisor pool, and feeding maximal factors instead of
    // all divisors was verified bit-identical (set and score) for every n in
    // 1..1000.

// Function has no return value: matching is stored in the graph g
// rather than returned as result.

void solvent_matching(TaxmanGraph& g, const ListOfVertexList& mf) {
  vertexSet selected;
  intDict owner;		// factor -> selection paying it
  int n = g.num_vertices();

  // Lambda function to attempt to add m to the selection of picks
  auto try_select = [&] (int m) {
    
    ListOfVertexPair pre_trail;
    auto own_m = owner.find(m);
    if (own_m != owner.end()) { // m in owner

      // m currently serves as someone's tax; try to re-route that
      // holder onto a different divisor without touching m.
      int holder = (*own_m).second; // pop the holder
      owner.erase(own_m);
      vertexSet visited = {m};
      if (augment(holder, owner, selected, mf, visited, pre_trail)) {
	pre_trail.push_back({m,holder}); // rollback restores m's owner
      }
      else {
	owner[m] = holder;	// undo the pop
	return false;
      }
    }
    
    selected.insert(m);		// blocks m from being used as anyone's tax

    ListOfVertexPair trail;
    vertexSet visited = {};
    if (augment(m, owner, selected, mf, visited, trail)) {
      if (is_acyclic(selected, owner, n)) {
	return true;		// fast, common-case silent success
      }

            //  Augment succeeded but the precedence is cyclic: this is NOT
            //  conclusive, so the complete tier must decide.  Here peel
            //  is a playability oracle, not a matching oracle -- a set can be
            //  matchable yet unschedulable (its every matching cyclic), and
            //  peel's forced-peeling structure correctly rejects such
            //  sets.  See the n=21 counterexample in the lean-solvent
            //  experiment docstring (candidate 10 augments but yields an
            //  unplayable 145-point set).  This path is unchanged from the
            //  canonical two-tier code.
      rollback(owner, trail);
      selected.erase(m);
      rollback(owner, pre_trail);
      selected.insert(m);
      intDict matching = complete_matching(g,selected, mf);
      if (matching.size() == 0) { // failed to find a matching including m
	selected.erase(m);
	return false;
      }
      intDict owner_candidate;
      for (auto match : matching) {
	int c = match.first;
	int f = match.second;
	owner_candidate[f] = c;
      }
      // Replace old owner list with new.  Using move skips copying and
      // just swaps pointers.
      owner = std::move(owner_candidate);
      return true;
    } // end augment attempt

    // Conclusive reject (Berge): a failed augmenting search from the free
    // vertex m proves no matching saturates selected | {m}.
    selected.erase(m);
    rollback(owner, pre_trail);
    return false;
  };				// end try_select definition

  for (int m = n; m >= 1; --m) {
    if ( ! mf[m].size() ) continue;
    try_select(m);
  }

  intDict matching;
  for (auto own : owner) {
    int f = own.first;
    int c = own.second;
    if ( selected.count(c) ) matching[f] = c;
  }

  // Store the matching in the graph
  g.unmatch_all();
  for (auto m : matching) {
    int pick = m.second;
    int tax = m.first;
    auto edgepair = g.find_edge(tax,pick);
    edgeHandle h = edgepair.first;
    g.match_edge(h);
  }

}

