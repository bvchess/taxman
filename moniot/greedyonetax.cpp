// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT


// GreedyOneTax

#include "defs.h"
#include "gamedefs.h"
#include "GameBoard.h"

// A variant of the OneTax heuristic.  For N = 1..1000, it does better
// than OneTax 301 times, worse 505 times, and the same 194 times.
// When it is not the same, the difference is always less than 1%.

// Play one move of the GreedyOneTax heuristic.

bool takeGreedyOneTax(GameBoard& b) {
  int pick = 0;
  numList taxes;
  score_t maxgain = INT_MIN;

  for (int x = 2; x <= b.game_size(); x++) {
    numList divisors = getRemainingDivisors(x, b);
    if (divisors.size() == 1) {
      score_t gain = x - sumVector(divisors);
      if (gain >= maxgain) {
	pick = x;
	taxes = divisors;
	maxgain = gain;
      }
    }
  }
  if (pick == 0) return false;  // nothing found (implies game over)

  // Now apply the tweak.
  
    // Check if this pick has only one active multiple that has no other divisors
    // besides this pick and its single tax, and no active multiples
    // of its own.  If so, take that instead, giving the Taxman the
    // pick and its divisor.
  numList multiples = getRemainingMultiples(pick, b);
  
  for ( int m : multiples ) {
    numList mdivisors = getRemainingDivisors(m, b);
    numList mmultiples = getRemainingMultiples(m, b);

    if (mmultiples.size() == 0 && mdivisors.size() == 2) {
      pick = m;
      taxes = mdivisors;
    }

  }
  
  // Take the pick and give Taxman its divisor(s).
  b.takePickAndTaxes( pick, taxes );

  return true;
}
