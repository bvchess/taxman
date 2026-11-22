// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// MaxTurn heuristic


#include "defs.h"
#include "gamedefs.h"
#include "GameBoard.h"

// Described and named Maxturn by Carmony and Holliday.  Strategy #2 of Trono.

// Play one move using MaxTurn heuristic.  Return true if a pick is found.

bool takeMaxTurn(GameBoard& b) {
  int pick = 0;
  numList taxes;
  score_t maxgain = INT_MIN;

  for (int x = 2; x <= b.game_size(); x++) {
    numList divisors = getRemainingDivisors(x, b);
    if (divisors.size() > 0 ) {
      score_t gain = x - sumVector(divisors);
      if (gain >= maxgain) {
	pick = x;
	taxes = divisors;
	maxgain = gain;
      }
    }
  }
  if (pick == 0) return false;  // nothing found (implies game over)

  // Take the pick and give Taxman its divisors.
  b.takePickAndTaxes(pick, taxes);

  return true;
}

