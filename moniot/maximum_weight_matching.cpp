// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT


// ------------------------------------------------------//
//                                                       //
//                Maximum-Weight Matching                //
//                                                       //
// ------------------------------------------------------//


#include "defs.h"
#include "graphdefs.h"
#include "TaxmanGraph.h"


// maximum_weight_matching(TaxmanGraph g) - solves the maximum weighted
// matching problem: find a set of edges having no vertices in common,
// with maximum total weight.  The implementation uses the equivalence
// to a minimum-cost maximum flow problem, since the graph is
// bipartite (edges only connect vertices of even rank to odd rank)
// and the weights are strictly positive.
// Edge weight = max(left, right) label.  Each edge is stored as {left, right},
// so the left/right side of every vertex is already known from construction.
//
// Method: successive shortest augmenting paths (min-cost flow) on
// source -> left vertices -> right vertices -> sink, unit capacities,
// with cost = -weight on the middle edges.  Path costs are
// non-decreasing, so we stop at the first augmenting path that no
// longer increases total weight (a max-weight matching need not be a
// maximum-cardinality one).
//
// Complexity: O(M * E) worst case with SPFA (M = matching size <=
// N/2, E = number of edges), which is fast for sparse graphs.
//
// The graph is initialized with L->R arcs having capacity 1 and cost
// = -weight, the reverse arcs R->L capacity 0, and cost = weight.
// The graph also includes a source vertex 0 connected to all the L
// vertices by arcs of cost 0, and a sink vertex n+1 connected to all
// the R vertices by arcs of cost 0.  Thus the paths from source to
// sink via a L->R edge correspond to edges in the matching and the
// total cost of the flow is the negative of the weight of the
// matching.

// Outline of algorithm:
// Array dist[v] is distance (cost) of path from source to v.
// Loop:
//   Initialize dist[source] = 0 and all other dist[v] = INF.
//   Push source onto FIFO queue.
//   While queue is not empty do:
//     Pop u from queue
//     For each neighbor v of u do:
//       If (u,v) has positive capacity and dist[u] + cost(u,v) < dist[v] then:
//         Let dist[v] = dist[u] + cost(u,v)
//         Record (u,v) on path to v
//         Push v onto queue if not already on it
//   If dist[sink] >= 0 quit Loop
//   For each edge (u,v) on path back from sink to source do:
//     decrease capacity of u->v by 1
//     increase capacity of v->u by 1

// Acknowledgement: this algorithm, as well as the data structures for
// representing the graph in the TaxmanGraph class, were written by
// Claude Code, although several details have been modified to embed it
// in this program.

void maximum_weight_matching( TaxmanGraph& g ) {

  int n = g.num_vertices();
  int source = 0;
  int sink = n + 1;

  // Add the source and sink vertices for SPFA algorithm
  g.add_source_and_sink();
  
  const weight_t INF = INF_WT / 4;
  std::vector<weight_t> dist(n + 2);
  std::vector<int> prevArc(n + 2);
  std::vector<char> inQ(n + 2);	// tracks whether vertex is in queue

  for (;;) {
    std::fill(dist.begin(), dist.end(), INF);
    std::fill(prevArc.begin(), prevArc.end(), -1);
    std::deque<int> q;
    dist[source] = 0; q.push_back(source); inQ[source] = 1;

    while (! q.empty()) {	// SPFA keeps active vertices in a queue
      int u = q.front(); q.pop_front();
      inQ[u] = 0;
      for ( edgeHandle h : g.neighbors(u) ) {
	int v = g.get_to(h); // get arc info
	int cap = g.get_cap(h);
	int cost = g.get_cost(h);
	if ( cap > 0 && dist[u] + cost < dist[v] ) {
	  dist[v] = dist[u] + cost;
	  prevArc[v] = h;
	  if (! inQ[v]) { inQ[v] = 1; q.push_back(v); }
	}
      }
    } // end while(! q.empty())

    // If no improved path from source to sink was found, dist[sink] is still INF
    // If any path was found, dist will be < 0 since all costs are negative.
    if ( dist[sink] >= 0 ) break;      // unreachable or no gain: done

    // Trace lowest-cost path from sink back to source, setting
    // capacity of forward arcs to 0 and reverse arcs to 1.
    for (int v = sink; v != source; v = g.get_to(prevArc[v] ^ 1)) {
      int newcap;
      newcap = g.get_cap(prevArc[v]) - 1;
      g.set_cap(prevArc[v], newcap);
      newcap = g.get_cap(prevArc[v] ^ 1) + 1;
      g.set_cap(prevArc[v] ^ 1, newcap);
    }
  } // end for (;;)

    // At this point, arcs[h].cap == 0 if and only if the arc is in the matching
 
} // end maximum_weight_matching
