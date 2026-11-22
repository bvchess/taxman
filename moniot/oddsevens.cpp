// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// OddsEvens heuristic

#include "defs.h"
#include "gamedefs.h"
#include "GameBoard.h"

// Strategy #7 of Trono.  Extremely simple but it wins reliably.
// Note that this heuristic does not perform nearly as well as Trono
// claims.  He credits Kevin Purcell and Robert Connelly for
// discovering it, and claims it outperforms OneTax, which it does not.
// It mostly scores quite a bit lower than OneTax or MaxTurn.

// Play entire game using the OddsEvens strategy.

bool playOddsEvens(GameBoard& b) {
  int pick = 0;
  numList taxes;
  int n = b.game_size();

		// stage 1: pick largest prime.  Always pickable and tax = 1
  
  // The list of primes may be longer than n for current game, so scan the
  // list in reverse to find the first prime <= n.
  
  int largest_prime = 0;  // ignored by takePickAndTaxes (in case n=1)
  for ( auto pp = primes.rbegin(); pp != primes.rend(); ++pp ) {
    int p = *pp;
    if ( p <= n ) {
      largest_prime = p;
      break;
    }
  }
  numList divisors = {1};
  pick = largest_prime;
  taxes = divisors;
  b.takePickAndTaxes(pick,taxes);

		// stage 2: pick available odd numbers going down, >= n/2
  
  int largestOdd = (n%2 == 1? n: n-1);
  for ( int x = largestOdd; x >= n/2; x -= 2 ) {
    numList divisors = getRemainingDivisors(x, b);
    if ( divisors.size() > 0 ) { // legally pickable
      pick = x;
      taxes = divisors;
      b.takePickAndTaxes(pick,taxes);
    }
  }
  
		// stage 3: pick available even numbers going up, > n/2
  
  int smallestEven = n/2+1;
  if ( smallestEven%2 == 1 ) ++smallestEven;
  for ( int x = smallestEven; x <= n; x += 2 ) {
    numList divisors = getRemainingDivisors(x, b);
    if ( divisors.size() > 0 ) { // legally pickable
      pick = x;
      taxes = divisors;
      b.takePickAndTaxes(pick,taxes);
    }
  }
  
  return (pick != 0);

}

