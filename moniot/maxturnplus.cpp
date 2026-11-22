// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// MaxTurn+ heuristic


#include "defs.h"
#include "gamedefs.h"
#include "GameBoard.h"

// Described by Moniot as an "improved greedy" heuristic.  Carmony & Holliday
// give an example where MaxTurn misses out on a freebie, but they don't
// pursue that as an improvement on the strategy.

// Caller first plays the game using MaxTurn heuristic, then calls
// insertFreebies to make it MaxTurn+.

bool insertFreebies(GameBoard& b) {
  
  // Find numbers left over at the end of the game.  These will go to
  // the taxman and are eligible to be freebies if pickable at the time
  // the freebie is evaluated.

  // At the time this function is called, the game is over but the
  // leftovers are still on the board.

  onBoard_t leftOver = b.on_board();

  numList origMoves = b.player_moves(); // save original play sequence and clear the board

  b = GameBoard(b.game_size());		       // Re-initialize the game

  for (int pick : origMoves ) {
    // Get the Taxman's take for this pick
    numList taxes_on_pick = getRemainingDivisors(pick, b);
    if (taxes_on_pick.size() == 0) return false;  // Can only happen if given invalid move sequence

  // Look for freebies that can be taken before this pick.  It is
  // possible that more than one freebie can be taken for a given
  // pick, so a loop is used to find them all.

    bool improvementQ = true;
    while ( improvementQ ) {
      improvementQ = false;	// will be set to true if a freebie is found

      // Freebies are found among the numbers taken by Taxman on this turn,
      // or multiples thereof that will no longer be pickable.  Put these
      // into list of candidates.
      numList candidates = {};
      for (int t : taxes_on_pick ) {
	// if t has active divisors, it is legally pickable.
	if ( getRemainingDivisors(t, b).size() > 0 ) {
	  candidates.push_back(t);
	}

	for (int m : getRemainingMultiples(t, b)) { // loop through multiples of t
	  if ( leftOver.at(m) &&		 // Only consider leftovers
	       getRemainingDivisors(m, b).size() > 0 ) { // must be pickable
	    candidates.push_back(m);
	  }
	}
      }
      if ( candidates.size() > 0 ) {
	// sort and remove duplicates
	std::sort(candidates.begin(), candidates.end());
	auto new_end = std::unique(candidates.begin(), candidates.end());
	candidates.erase(new_end, candidates.end());
	
	// Go through candidates, looking for freebies.  Note that the
	// name taxes_on_candidate is a bit of a misnomer, since
	// candidates that are divisors of pick are included as well.
	// This is needed for the subset test to work right.
	int freebie = 0;
	numList taxes_on_freebie = {};
	for ( int c : candidates ) {
	  numList taxes_on_candidate = getRemainingDivisors(c, b);
	  // Include candidate in taxes if it is in pick taxes.
	  if( pick%c == 0 ) {
	    taxes_on_candidate.push_back(c);
	  }
	  // If taxes are a proper subset of pick taxes, it is a freebie
	  if ( taxes_on_candidate.size() < taxes_on_pick.size() &&
	       isSubset(taxes_on_candidate, taxes_on_pick ) ) {
	    freebie = c;
	    taxes_on_freebie = getRemainingDivisors(freebie,b);
	  }
	  // Don't break when one is found; possibly a larger one will be found
	}
	if ( freebie != 0 ) {	// Freebie was found
	  b.takePickAndTaxes(freebie, taxes_on_freebie);
	  taxes_on_pick = getRemainingDivisors(pick, b); // update after taking freebie
	  improvementQ = true;
	}
      }	// end processing candidates
    }	// end loop while improvementQ

    // Done taking freebies: take the original pick
    b.takePickAndTaxes(pick, taxes_on_pick);

  } // end loop on picks
  
  return true;
}

