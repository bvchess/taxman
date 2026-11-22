// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// Various shared definitions: typedefs and function prototypes


#ifndef DEFS_H
#define DEFS_H

// Library function headers
#include <iostream>
#include <algorithm>
#include <climits>
#include <cstring>
#include <cstdlib>
#include <cstdio>
#include <sstream>
#include <utility>
#include <vector>
#include <deque>
#include <unordered_set>
#include <unordered_map>
#include <cstddef>
#include <iterator>
#include <cctype>
#include <cstdint>
#include <string>

// Shorthand type definitions

// Overflow of 32-bit integers may occur for 32-bit n values when
// n*(n+1) is calculated.  Large n may also run into alloc issues.
typedef std::int64_t score_t;		// for holding scores

typedef std::vector<int> numList;

typedef int weight_t;			    // Data type of weights
const weight_t INF_WT = INT_MAX;	    // Largest value of same

// These are used by the graph-based heuristics.
typedef std::vector<int> vertexList;
typedef std::vector<vertexList> ListOfVertexList;
typedef struct {int left; int right; int matched; weight_t wt;} edge; // Directed edge left -> right
typedef std::vector<edge> edgeList;	    // Representation of graph as list of edges
typedef int edgeHandle;			    // For find_edge function
typedef std::vector<edgeHandle> edgeHandleList;


// Global variables
extern std::vector<int> omega;
extern std::vector<int> primes;


// Prototypes for functions and classes defined in their own source files.
// Note that the graph-based function prototypes are in graphdefs.h.


// In get_nvalues.cpp:
void get_nvalues( std::vector<int>& nvalues );

// In usage.cpp:
void usage( const char *progname, int exit_status );


// In math.cpp
void calcOmega(int n);
int maxRank(int n);

#endif
