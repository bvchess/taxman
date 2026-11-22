// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT


// ------------------------------------------------------//
//                                                       //
//                TaxmanGraph class                      //
//                                                       //
// ------------------------------------------------------//

#ifndef TAXMANGRAPH_H
#define TAXMANGRAPH_H

#include "defs.h"
#include "gamedefs.h"
#include "graphdefs.h"


// Two constructors are defined: TaxmanGraph(int n) for the full graph,
// and TaxmanGraph(int r, TaxmanGraph g) for the rank subgraph.  For
// both, vertices are represented by integers from 1 to n.  Each
// vertex is joined by edges to its prime multiples.  For the full
// graph, all edges go from even rank to odd rank.  For the rank
// subgraph, edges go from smaller to larger vertex.  The weight of an
// edge is the value of the larger node.  The full graph includes
// source and sink edges used by the max-weight matching.


class TaxmanGraph {
private:
  struct directedArc { int to; int cap; weight_t cost; };
  typedef std::vector<directedArc> arcList;
  typedef std::vector<vertexList> adjacencyList;
  typedef enum {fullGraph,rankGraph} GraphType;
  GraphType graphType;
  int numVertices;	// Vertices are numbered from 1 to numVertices
  int numEdges;		// Does not include source/sink edges

  // arcs = list of target vertices, with flow capacity and cost.
  // Entries come in pairs: even-index entry is outgoing arc,
  // following odd-index entry is the reverse arc.  The constructor
  // fills the list with the actual graph edges.  Then for max-weight
  // matching, add_source_and_sink adds edges between source and left
  // vertices or sink and right vertices as the final 2N entries.
  arcList arcs;
  // For vertex v, adjArc[v] is a list of indices in arcs vector with
  // edges from v.  Index 0 is for source, N+1 is for sink.
  adjacencyList adjArc;
  // edgeArc is the list of unique edges.  edgaArc[i] is index in arcs of
  // the outgoing edge.
  vertexList edgeArc;

  int source, sink;		// indices of source (0) and sink (N+1)

  using EdgeIter = vertexList::const_iterator; // for edge_iterator
  using AdjIter =  vertexList::const_iterator; // for adjacency_iterator
  
  // Add arcs for an edge u->v with capacity cap and weight cost
  void addArc( int u, int v, int cap, weight_t cost ) {
    adjArc[u].push_back((int)arcs.size());
    arcs.push_back({v,cap,cost});
    adjArc[v].push_back((int)arcs.size());
    if ( graphType == fullGraph ) { // cap is capacity for max-wt matching
      arcs.push_back({u,0,-cost});
    }
    else {			// cap holds matching info
      arcs.push_back({u,cap,cost});
    }
  }
  
  // Private function to locate an arc given its endpoints u, v
  // Returns arcs.size() if not found.

  // This is not real efficient, O(#nbrs(u)).  By looping over the
  // list of neighbors of u, which is normally much smaller than
  // numEdges, it is faster than scanning edgeArc vector, which would
  // be O(numEdges)

  int find_arc( int u, int v ) {

    for ( int nbr = 0; nbr < (int)adjArc[u].size(); nbr ++ ) {
      int arc_index = adjArc[u][nbr];
      if ( arcs[arc_index].to == v ) {
	return arc_index;
      }
    }

    return (int)arcs.size();			// arc not found
  }

public:

  // Constructor for full Taxman graph.  This has edges (x,p*x) for
  // each x in 1..n and each prime p for which p*x <= n.  The graph is
  // bipartite: left-side vertices have even rank, right-side odd
  // rank.  The cost and capacity of each edge are filled in with
  // values needed by the maximum-weight matching.
  explicit TaxmanGraph(int n) : numVertices(n) {
    graphType = fullGraph;
    adjArc.resize(n+2);
    source = 0;
    sink = n+1;

    numEdges = 0;
    // Add the taxman graph edges
    for (int i = 1; i <= n/2; i++) {
      for (int p : primes ) {
	int u = i;
	int v = p*i;
	int wt = v;		// Weight is the larger vertex
	if (v > n ) break;
	if ( omega[u]%2 == 1 ) { // Ensure even rank on the left
	  std::swap(u,v);
	}
	edgeArc.push_back((int)arcs.size());
	++numEdges;
	addArc( u, v, 1, -wt );	// Cost = -weight so min cost = max weight
      }
    }
  } // end constructor for full graph

  
  // Constructor for rank subgraph: r is rank, g is parent full graph.
  // Rank subgraph is bipartite: left-side vertices have rank r-1,
  // right-side r.

  TaxmanGraph(int r, TaxmanGraph& g) {
    graphType = rankGraph;
    numEdges = 0;
    numVertices = g.numVertices;
    source = 0;			// Not used for subgraphs, but initialize them.
    sink = numVertices+1;
    adjArc.resize( numVertices + 2 );

    for (int ei = 0; ei < g.numEdges; ++ei) {
      int u = g.arcs[g.edgeArc[ei]].to;
      int v = g.arcs[g.edgeArc[ei]^1].to;
      if ( u > v ) std::swap(u,v); // make right largest
      if (omega[v] == r) {	   // select edges of rank r-1 to r
	edgeArc.push_back((int)arcs.size());
	++numEdges;
	// copy the matching from full graph to subgraph
	addArc( u, v, g.arcs[g.edgeArc[ei]].cap, v);
      }
    }
  } // end constructor for rank subgraph


  // Method used by maximum_weight_matching to add source and sink and
  // connect them to the vertices.  These are needed for min-cost
  // max-flow algorithm.  The other matchings do not need these.
  
  void add_source_and_sink() {
        
    // Add connections to source and sink
    for (int v = 1; v <= numVertices; ++v) {
      if ( omega[v]%2 == 0 ) {	// parity of rank(v)
	addArc(source, v, 1, 0); // even parity edges are next to source
      }
      else {
	addArc(v, sink, 1, 0);
      }
    }
  }

  // Get and set methods.  The edgeHandle is simply an index into the arcs
  // vector.  The caller is responsible for ensuring it is within range.

  int num_edges() {
    return numEdges;
  }

  int num_vertices() {
    return numVertices;
  }

   
  int get_cap(edgeHandle h) {
    return arcs[h].cap;
  }

  int get_cost(edgeHandle h) {
    return arcs[h].cost;
  }

  int get_to(edgeHandle h) {
    return arcs[h].to;
  }

  void set_cap(edgeHandle h, int capacity) {
    arcs[h].cap = capacity;
  }

  void set_cost(edgeHandle h, int cost) {
    arcs[h].cost = cost;
  }

  // Look up an edge by its endpoints (u,v).  This returns a pair
  // {edgeHandle,success} where success indicates whether the given
  // edge was found.
  std::pair<edgeHandle,bool> find_edge( int u, int v ) {
    edgeHandle arc_index = find_arc( u, v );
    return {arc_index, arc_index != (edgeHandle)arcs.size()};
  }

  // Return an edge struct with values filled in.
  edge get_edge( edgeHandle h ) {
    edge e;
    if ( h >= 0 && h < (edgeHandle)arcs.size() ) {
      e = {arcs[h^1].to, arcs[h].to, !arcs[h].cap, arcs[h].cost};
    }
    else {
      e = {0,0,0,0};		// should not happen except for incorrect usage
    }

    return e;
  }

  // User method to get source vertex of an edge.
  int get_source( edgeHandle h ) {
    return arcs[h^1].to;
  }
    
  // User method to get target vertex of an edge.  Same as get_to
  // above, but named for clarity in different context.
  int get_target( edgeHandle h ) {
    return arcs[h].to;
  }
  
  // User method to make an edge matched.  Both the out-arc and return
  // arc need to have the same matching value.
  void match_edge( edgeHandle h ) {
    arcs[h].cap = 0;
    arcs[h^1].cap = 0;
  }
  
  // User method to make an edge unmatched.  Both the out-arc and
  // return arc need to have the same matching value.
  void unmatch_edge( edgeHandle h ) {
    arcs[h].cap = 1;
    arcs[h^1].cap = 1;
  }

  // User method to set all edges to unmatched.  This is used by the
  // solvent heuristic to reset the matching before each new candidate
  // pick is tested.  Since this is used on full graph, to avoid
  // uselessly dealing with source and sink edges, get only true edges
  // from edgeArc list.
  void unmatch_all() {
    for (int h : edgeArc ) {
      unmatch_edge(h);
    }
  }
  
  // User method to set the weight of an edge.  This is used by the
  // Demetrescu & Finocchi algorithm.
  void set_weight( edgeHandle h, weight_t new_wt ) {
    arcs[h].cost = new_wt;
    arcs[h^1].cost = new_wt;
  }

  
  // Iterator over edges of graph via edgeHandle
  class edge_iterator {
  private:
    EdgeIter m_e;		// points into edgeArc array

  public:
    using iterator_concept = std::input_iterator_tag;
    using iterator_category = std::input_iterator_tag;
    using value_type        = int;
    using difference_type   = std::ptrdiff_t;
    using pointer           = void; // Standard layout requires pointer/reference types
    using reference         = value_type; // Returning by value because it's a proxy pair

    edge_iterator() : m_e() {}
    edge_iterator(EdgeIter e) : m_e(e) {}

    reference operator*() const {
      int h = *m_e;		  // edgeHandle is entry in edgeArc
      return h;
    }

    edge_iterator& operator++() {
      ++m_e;
      return *this;
    }
    
    edge_iterator operator++(int) {
      edge_iterator tmp = *this;
      ++*this;
      return tmp;
    }

    friend bool operator==(const edge_iterator& a, const edge_iterator& b) {
      return a.m_e == b.m_e;
    }
    
    friend bool operator!=(const edge_iterator& a, const edge_iterator& b) {
      return !(a == b);
    }
  
  };				// end edge_iterator

    // Proxy range view of edge_iterator
    struct EdgeValueRange {
      edge_iterator b, e;
      edge_iterator begin() const { return b; }
      edge_iterator end() const { return e; }
    };
  
  EdgeValueRange edges() const {
    return EdgeValueRange{
      edge_iterator(edgeArc.begin()),
      edge_iterator(edgeArc.end())
    };
  }

  
  // Iterator over vertices adjacent to a vertex
  class adjacency_iterator {
  private:
    AdjIter m_a;

  public:
    using iterator_concept = std::input_iterator_tag;
    using iterator_category = std::input_iterator_tag;
    using value_type        = int;
    using difference_type   = std::ptrdiff_t;
    using pointer           = void; // Standard layout requires pointer/reference types
    using reference         = value_type; // Returning by value because it's a proxy pair

    adjacency_iterator() : m_a() {}
    adjacency_iterator(AdjIter a) : m_a(a) {}

    reference operator*() const {
      int h = *m_a;
      return h;
    }

    adjacency_iterator& operator++() {
      ++m_a;
      return *this;
    }
    
    adjacency_iterator operator++(int) {
      adjacency_iterator tmp = *this;
      ++*this;
      return tmp;
    }

    friend bool operator==(const adjacency_iterator& a, const adjacency_iterator& b) {
      return a.m_a == b.m_a;
    }
    
    friend bool operator!=(const adjacency_iterator& a, const adjacency_iterator& b) {
      return !(a == b);
    }
  
  };				// end adjacency_iterator

    // Proxy range view of adjacency_iterator
    struct AdjacencyValueRange {
      adjacency_iterator b, e;
      adjacency_iterator begin() const { return b; }
      adjacency_iterator end() const { return e; }
    };
  
  AdjacencyValueRange neighbors(int v) const {
    return AdjacencyValueRange{
      adjacency_iterator(adjArc[v].begin()),
      adjacency_iterator(adjArc[v].end())
    };
  }

};				// end class TaxmanGraph

#endif
