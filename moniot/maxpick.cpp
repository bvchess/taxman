// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// MaxPick heuristic

#include "defs.h"
#include "gamedefs.h"
#include "GameBoard.h"

// Strategy #1 of Trono.  Carmony and Holliday discuss it as an example
// of a greedy strategy that does not work well, since it usually
// loses.  It wins only for N=2, 5, and 11, and ties for N=3.

// Play one move of the MaxPick heuristic.

bool takeMaxPick(GameBoard& b) {
  int pick = 0;
  numList taxes;

  for (int x = b.game_size(); x > 0; --x) {
    numList divisors = getRemainingDivisors(x, b);
    if (divisors.size() > 0 ) {
      pick = x;
      taxes = divisors;
      break;
    }
  }

  if (pick == 0) return false;  // nothing found (implies game over)

  b.takePickAndTaxes( pick, taxes );
  
  return true;
}
