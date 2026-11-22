// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT


// ------------------------------------------------------//
//                                                       //
//                    findPlayOrder                      //
//                                                       //
// ------------------------------------------------------//


#include "defs.h"
#include "gamedefs.h"
#include "GameBoard.h"
#include "graphdefs.h"
#include "TaxmanGraph.h"

// Function to find playable sequence of matched edges in rank
// subgraph g_r.  The moves are appended to sequence.  Returns true if
// successful.  Note that the function MODIFIES the graph dg
// (including the in_ and out_edges) and the sequence.

// This is used by all the matching-based heuristics, except of course
// UpperBound, which does not necessarily have a playable sequence.

// The algorithm here is not quite the same as in Franklin & Moniot,
// but it is logically equivalent.  It repeatedly finds a vertex that
// is at the upstream end of an alternating path.  Picking that vertex
// will not prevent any other matched vertices from being picked
// later.

// Function is supposed to be called only on graphs with no
// alternating cycles, but it includes a test to detect if the path it
// is following is too long to be acyclic.  It returns false in
// that case.  The sequence of picks is returned in argument sequence.

bool findPlayOrder( TaxmanGraph& g_r, GameBoard& b ) {

  while( true ) {		// loop on matched edges till all deleted

    // First, find a matched edge from which to start
    edge matched_edge;
    bool matched_edge_found = false;
    edgeHandle matched_edge_handle;

    for (edgeHandle h : g_r.edges()) {
      edge e = g_r.get_edge(h);
      if ( e.matched ) {
	matched_edge = e;
	matched_edge_handle = h;
	matched_edge_found = true;
	break;
      }
    }
    

    if ( ! matched_edge_found ) break; // no matched edges left

    // Follow alternating path upstream from starting matched vertex.
    // When it ends at a vertex with no in-edges, we have our pick in
    // the last matched edge.
    bool step_successful = true;
    int steps = 0;		// To catch a cycle in case of bug
    edge current_edge = matched_edge;

    while ( step_successful ) {
      step_successful = false;
      // Get started in the upstream direction, thereafter follow directed arcs
      int u;
      if (steps == 0) {
	u = std::max(current_edge.left,current_edge.right);
      }
      else {
	u = current_edge.right;
      }
      // This loop will look for two edges u->v (unmatched) and v->w
      // (matched).  This code takes advantage of the fact that an
      // edgeHandle given by v in neighbors(u) is the same as for edge
      // (u,v).
      for ( edgeHandle uv_h : g_r.neighbors(u) ) { // look for u->v
	int v = g_r.get_target(uv_h);
	edge uv = g_r.get_edge(uv_h); // the edge {u,v,matched,wt}
	if ( ! uv.matched ) { // enforce alternating; require unmatched
	  current_edge = uv;
	  ++steps;
	  for ( edgeHandle vw_h : g_r.neighbors(v) ) { // look for v->w
	    edge vw = g_r.get_edge(vw_h); // the edge {v,w,matched,wt}
	    if ( vw.matched ) { // enforce alternating: require matched
	      step_successful = true;
	      current_edge = matched_edge = vw;
	      matched_edge_handle = vw_h;

	      if ( ++steps > g_r.num_edges() ) {		// path too long
		return false;
	      }
	      break;	// Exit loop on w
	    }
	  } // end loop on neighbors w of v
	  if ( step_successful ) break; // exit loop on v
	}
      }	// end loop on neighbors v of u
    } // end while( step_successful )
    
    // Path has been followed to a terminal matched edge: use it for
    // pick and unmatch it.
    int pick = matched_edge.left;
    int dvsr = matched_edge.right;
    g_r.unmatch_edge(matched_edge_handle);
    if ( pick < dvsr ) std::swap(pick,dvsr);
    // For our purposes of marking the pick as taken in the GameBoard,
    // it is not necessary for the tax to be complete.  Use just the divisor.
    std::vector<int> tax={dvsr};
    b.takePickAndTaxes(pick,tax);
  } // end while (true) loop on all edges of graph

  return true;
}
