// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// Driver for heuristics based on Taxman Graph.  (These don't fit well
// into the structure of the non-graph-based routines, so this routine
// serves as an interface.)  The GameBoard is filled with the results.


#include "defs.h"
#include "heuristics.h"		// Heuristic names & descriptions
#include "gamedefs.h"
#include "GameBoard.h"
#include "graphdefs.h"
#include "TaxmanGraph.h"

bool playGraphHeuristic( GameBoard& b, HeuristicID heuristic ) {

  // Initialize the Taxman graph
  int n = b.game_size();
  TaxmanGraph g(n);

  
  // In the below matching routines, the matching is stored in the
  // graph g as labels on the edges.
    
  if ( heuristic == HeuristicID::BornFree ) {
    // Create the born-free matching
    born_free_matching(g);

  } else if ( heuristic == HeuristicID::Solvent ) {
    // Create the solvent matching
    ListOfVertexList mf = maximal_factor_table(g);

    solvent_matching(g,mf);

  }
  else { // UpperBound and BreakCycles

    // Compute the maximum-weight matching on the graph

    maximum_weight_matching(g);

  }

  
    // At this point we have a matching.  The born-free matching and
    // solvent matching are free of flat alternating cycles by
    // construction.  The maximum-weight matching may have cycles.  If
    // cycle-break heuristic is requested, remove matched edges to
    // break flat alternating cycles.

  if ( heuristic == HeuristicID::UpperBound ) {
    // Upper bound is generally not playable, so simply mark the picks
    // as taken, with no tax.  Taxman will take everything at the end.
    std::vector<int> notax;	// empty list
    for ( edgeHandle h : g.edges() ) {
      edge e = g.get_edge(h);
      if ( e.matched ) {
	b.takePickAndTaxes(std::max(e.left,e.right),notax);
      }
    }
    b.sortPlayerMoves();	// Since not necessarily playable, list in order

  } else {			// BreakCycles, Solvent, BornFree

    // For Solvent and BornFree, the matching is playable, but needs
    // to be put in play order.  For BreakCycles, the matching may
    // need to be made playable by breaking cycles.  Breaking cycles
    // is done within each rank subgraph since only flat cycles
    // matter.  Finding play order is also done within each rank
    // subgraph since picks go in ascending rank order, and
    // restricting to rank reduces the complexity of the task.
      
    // Here we restrict the graph to subgraphs by rank.  For each
    // subgraph, for BreakCycles we first break the cycles.  Then for
    // all playable strategies, we find a move sequence for a legal
    // game.
    int maxrank = maxRank(n);

    // We only need to eliminate flat cycles (involving only
    // vertices of same rank and one less), so loop through ranks and
    // find subgraphs containing only edges joining vertices of rank
    // r-1 and r.  Paths within these subgraphs will be flat.
    for (int r = 1; r <= maxrank; ++r) {
      TaxmanGraph g_r(r, g);

      if ( heuristic == HeuristicID::BreakCycles ) {
	// Remove some matched edges to break alternating cycles.
	// Note that this routine MODIFIES g_r by removing edges
	// from the matching to break cycles.
	breakAlternatingCycles( g_r );
      }

      // Add the moves from this rank to the play sequence. This
      // routine MODIFIES g_r by unmatching matched edges as they
      // are used to provide picks.  This should always succeed
      // because by this point the matching is free of alternating
      // cycles.  Warn if not, it means there is a bug.
      if ( ! findPlayOrder( g_r, b ) ) {
	std::cerr << "Oops: n=" << n << " rank " << r << " not playable" << std::endl;
	exit(EXIT_FAILURE);
      }
	
    }	// end loop on r
  } // end if ( BornFree || Solvent || BreakCycles )

    
  return true;
}
