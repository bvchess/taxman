// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// Functions used by several heuristics

#include "defs.h"
#include "gamedefs.h"
#include "GameBoard.h"

// Return a (possibly empty) list of divisors of x that are still on
// the board, sorted in ascending order.

numList getRemainingDivisors(int x, GameBoard& b) {
    numList divisors;

    for (int d = 1; d <= x/d; ++d) {
        if (x % d == 0) {
            int q = x / d;

            if (d < x && b.inPlay(d)) {divisors.push_back(d);}
            if (q != d && q < x && b.inPlay(q)) {divisors.push_back(q);}
        }
    }
    std::sort(divisors.begin(), divisors.end());
    return divisors;
}

// Return a (possibly empty) list of multiples of x that are still on
// the board, in ascending order.
numList getRemainingMultiples(int x, GameBoard& b) {
  numList multiples;
  for(int m = x+x; m <= b.game_size(); m += x) {
    if( b.inPlay(m) ) multiples.push_back(m);
  }
  return multiples;
}

// Sum the elements of a vector of integers

score_t sumVector( const numList& v ) {
  score_t sum = 0;
  for (int x : v) sum += x;
  return sum;
}

// Function returns true if setA is a subset of setB, where the
// sets are stored in vectors.
bool isSubset( const numList& setA, const numList& setB ) {
  bool subsetQ = true;
  for ( int y : setA ) {
    bool y_in_setB = false;
    for ( int x : setB ) {
      if ( x == y ) {
	y_in_setB = true;
	break;
      }
    }
    if ( ! y_in_setB ) {
      subsetQ = false;
      break;
    }
  }
  return subsetQ;
}

