// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT


// ------------------------------------------------------//
//                                                       //
//                Born-Free Heuristic                    //
//                                                       //
// ------------------------------------------------------//

#include "defs.h"
#include "gamedefs.h"
#include "GameBoard.h"
#include "graphdefs.h"
#include "TaxmanGraph.h"



// See Franklin & Moniot for a description of this matching and proof
// that it is free of flat alternating cycles by construction.  For
// each prime p less than or equal to n, in descending order, add to
// the matching all edges of the form (x,p*x) such that neither x nor
// p*x has yet been included in a matched edge.

void born_free_matching(TaxmanGraph& g) {
  int n = g.num_vertices();
  std::vector<bool> ismatched(n + 1,0);

  // Run through the primes <= n in descending order.  Note that the
  // list of primes can include some > current n if program is run for
  // multiple values of n.  We skip those.
  for ( auto pi = primes.rbegin(); pi != primes.rend(); ++pi ) {
    int p = *pi;
    if ( p > n ) continue;
    
    // Add to matching all pairs (x,p*x) not already used
    for ( int u = n/p; u >= 1; --u ) {
      int v = p*u;
      // It is not sufficient to test the edge, we have to check both
      // vertices individually to make sure they have not been used in
      // a match.
      if( !( ismatched[u] || ismatched[v] ) ) {
	ismatched[u] = ismatched[v] = 1;
	auto edgepair = g.find_edge(u,v);
	// edgepair.second = success, but it always works so ignore
	edgeHandle h = edgepair.first;
	g.match_edge(h);
      }
    }
  }
} // end born_free_matching()
