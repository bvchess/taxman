// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// ------------------------------------------------------//
//                                                       //
//                Break-Cycles Heuristic                  //
//                                                       //
// ------------------------------------------------------//


#include "defs.h"
#include "graphdefs.h"
#include "TaxmanGraph.h"


// This heuristic takes a TaxmanGraph labeled with the maximum-weight
// matching and checks it for flat alternating cycles.  (A flat
// alternating cycle is a cycle all of whose edges go between two
// adjacent ranks, alternating between matched and unmatched edges.
// Franklin & Moniot have shown that a matching on the Taxman graph
// corresponds to a legal move sequence if and only if it has no flat
// alternating cycles.)  If there are none, then the matching is
// playable, but for many game sizes, and all game sizes above 90, the
// maximum-weight matching induces cycles.  This heuristic breaks such
// cycles, attempting to minimize the loss of points incurred.

// It should be noted that this heuristic can in principle give different
// results depending on the implementation.  The maximum-weight matching
// is not always unique, and the order in which cycles are examined to
// break them is arbitrary.  However, the implementation here should
// give the same results when compiled on any C++ standard-conforming
// platform.

// The breakAlternatingCycles function uses the algorithm described here:
//
// C. Demetrescu and I. Finocchi.  Combinatorial algorithms for
// feedback problems in directed graphs.  Information Processing
// Letters 86 (2003) 129-136.
//


// Function to perform a depth-first search on the directed graph
// starting from vertex v.  It returns true if it detects a cycle.  In
// that case, the argument cycle contains the edges of the cycle.

// Vertex coloring: WHITE = unvisited, GRAY = visited but will be
// revisited, BLACK = visited, and won't be revisited.
enum class visitedStatus { WHITE, GRAY, BLACK };
typedef std::vector<visitedStatus> vstatusList;

bool dfsDetectCycle( TaxmanGraph& g_r, edgeHandle h_v, vstatusList& vstatus, edgeHandleList& cycle ) {

  int v = g_r.get_target(h_v);	// Get current node
  bool v_matched = g_r.get_edge(h_v).matched;
  
  // Mark the current node as in process
  vstatus.at(v) = visitedStatus::GRAY;

  // Explore all alternating edges from v.
  for ( edgeHandle h : g_r.neighbors(v) ) {
    edge e = g_r.get_edge(h);
    if ( e.matched != v_matched ) { // alternate
      if ( vstatus.at(e.right) == visitedStatus::GRAY ) {
	// Active vertex encountered: cycle exists.  Save the edge to cycle.
	cycle.push_back(h);
	return true;
      }
    
      if ( vstatus.at(e.right) == visitedStatus::WHITE ) {
	// Unvisited neighbor: explore recursively
	if ( dfsDetectCycle( g_r, h, vstatus, cycle) ) {
	  cycle.push_back(h);
	  return true;
	}
      }
    }

    // if neighbor is BLACK, it is done, skip it.  No cycle there.
  }
  
  // no more neighbors, mark vertex as done
  vstatus.at(v) = visitedStatus::BLACK;
  return false;

}


// Function to find an alternating cycle within the graph g_r, if
// there is one.  This is to be called on rank subgraphs because only
// flat alternating cycles matter.

edgeHandleList findAlternatingCycle( TaxmanGraph& g_r ) {

  edgeHandleList cycle;		// Initialize to empty cycle

  // Initialize status of all vertices to unvisited
  vstatusList vstatus = vstatusList(g_r.num_vertices()+1,visitedStatus::WHITE);
  
  // Find a matched edge from which to start.  It needs to be a
  // vertex connected to at least one edge, so use the source vertex
  // of an edge.  Since it may not be at the root of the dfs tree,
  // restart until all vertices are visited.
  
  for (edgeHandle h : g_r.edges()) {
    edge e = g_r.get_edge(h);
    if ( e.matched ) {
      if ( vstatus.at(e.right) == visitedStatus::WHITE ) {
	if ( dfsDetectCycle( g_r, h, vstatus, cycle ) ) {
	  // Cycle detected.
	  // DFS returns cycle in reverse order.  Put it in direct order.
	  std::reverse(cycle.begin(),cycle.end());
	  break;			// Cycle found: stop looking and return
	}
      }
    }
  }

  return cycle;
}

// Run the Demetrescu & Finocchi algorithm to find a good
// solution to the minimum feedback arc set.  (The optimal solution is
// NP-hard, so we settle for a good approximation.)
//
// The argument g_r is a rank-restricted directed subgraph of the full
// Taxman graph, with matched edges reversed.  The algorithm is
// modified to only remove matched edges.  (It does not actually
// remove any edges from the graph, but simply removes them from the
// matching.)
//
// Here is Demetrescu & Finocchi's original algorithm, called FAS for
// Feedback Arc Set.  G is the graph, w the weights.  F is the FAS.
//
// Algorithm FAS ( G=(V,A); w:A -> R+ )
// 1 begin
// 2   F := 0
// 3   while ( (V,A\F) is not acyclic )  {Phase 1}
// 4     begin
// 5       Let C be a simple cycle in (V,A\F)
// 6       Let (x,y) be a minimum weight arc in C and let eps be its weight
// 7       for each (u,v) in C
// 8         w(u,v) := w(u,v) - eps
// 9         if w(u,v) = 0
//10         then F := F U { (u,v) }
//11     end
//12   for each (u,v) in F              {Phase 2}
//13     if (V,A\F U { (u,v) } is acyclic
//14     then F := F \ { (u,v) }
//15   return F
//16 end
//
// Removing F from the matching yields a matching that is free of
// cycles.  By limiting the cycles to alternating cycles, and the
// graph to one rank at a time, and removing only matched edges, the
// result will be a graph with no flat alternating cycles.

// The graph g_r is modified as edges are unmatched.  (They are not
// removed from the graph.)  Function has no return value: the
// acyclic matching is stored in g_r.

void breakAlternatingCycles( TaxmanGraph& g_r ) {

  edgeHandleList deleted_edges;	// The FAS that breaks the cycles
  
  //   Phase 1: find and break alternating cycles

  bool acyclic = false;		// flag to control Phase 1 loop

  while ( ! acyclic ) {
    
    edgeHandleList cycle = findAlternatingCycle( g_r );
    
    acyclic = (cycle.size() == 0);

    // Traverse the cycle, finding minimum weight matched edge
    if ( ! acyclic ) {
      int min_wt = INT_MAX;
      edgeHandle min_edge_handle = -1;   // initialize so -Wall doesn't complain
      for ( const edgeHandle h : cycle ) { // loop through edges of cycle
	edge e = g_r.get_edge(h);
	//  Skip unmatched edges since only matched edges count in score
	if ( e.matched ) {
	  int wt = e.wt;
	  if ( wt < min_wt ) {
	    min_wt = wt;
	    min_edge_handle = h;
	  }
	}
      }

      // The above loop only runs if cycle is nonempty, and it always
      // starts with a matched edge, so min_edge_handle always gets assigned
      // a valid value.  Put a check here in case a bug creeps in.
      if (min_edge_handle < 0 ) {
	std::cerr << "Assertion failed: min_edge_handle >= 0" << std::endl;
	exit(EXIT_FAILURE);
      }
      

      // Now update the weights, reducing them by min_wt.
      for ( edgeHandle h : cycle ) { // loop through edges of cycle
	edge e = g_r.get_edge(h);
	if ( e.matched ) {
	  g_r.set_weight(h, e.wt - min_wt);
	}
      }

      // Unmatch the edge.  Save the edge handle in list in case it
      // gets restored in phase 2.

      deleted_edges.push_back(min_edge_handle);
      
      g_r.unmatch_edge(min_edge_handle);
    }
  } // End while ( acyclic )

  //   Phase 2: attempt to restore some deleted edges
  // The reason is that edges may be in 2 cycles, and perhaps some
  // other edge sufficed to break both.
  for ( edgeHandle h : deleted_edges ) {
    edgeHandleList cycle;
    // Restore matched status of the edge.
    g_r.match_edge(h);
    
    
    // See if adding the edge back created a cycle
    cycle = findAlternatingCycle(g_r);
    
    if ( cycle.size() != 0 ) {
      g_r.unmatch_edge(h);
    }
  }
}

