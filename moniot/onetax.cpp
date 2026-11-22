// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// OneTax and PureOneTax heuristics

#include "defs.h"
#include "gamedefs.h"
#include "GameBoard.h"

// PureOneTax is the unmodified Strategy #3 of Trono.  It is a
// rather simple strategy, and performs very well.  OneTax improves
// on it by allowing the Taxman to take 2 numbers occasionally.

// Trono adds a different tweak to produce his Strategy #4: when 5 or
// fewer numbers are left that can be picked, switch to MaxTurn.
// This does not improve it as much as the tweak used here.

// Play one move of either the pure one-tax heuristic (no tweak) or
// the tweaked version, depending on argument heuristic.

bool takeOneTax(GameBoard& b, HeuristicID heuristic) {
  int pick = 0;
  numList taxes;

  for (int x = 2; x <= b.game_size(); x++) {
    numList divisors = getRemainingDivisors(x, b);
    if (divisors.size() == 1) {
      pick = x;  // this loop will choose pick as the largest with 1 divisor.
      taxes = divisors;
    }
  }
  if (pick == 0) return false;  // nothing found (implies game over)

  if (heuristic == HeuristicID::OneTax) { 	// Skip this for PureOneTax
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

  }
  
  // Take the pick and give Taxman its divisor(s).
  b.takePickAndTaxes( pick, taxes );

  return true;
}

