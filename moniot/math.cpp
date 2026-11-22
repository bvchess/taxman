// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// Routines to fill the omega and primes lists and find the max rank


#include "defs.h"

// This routine uses a sieve approach to calculate the prime omegas
// (aka ranks) of 2 to n and build a list of primes at the same time.
// This is called once at the beginning before the loop on n values,
// so it needs to be called with the largest value of n that will be
// analyzed.

void calcOmega(int n) {
  omega = std::vector<int>(n+1,0);
  for (int p = 2; p <= n; p++) {
    if (omega.at(p) == 0) {		// p is prime
      primes.push_back(p);		// save primes in list
      for (int pwr = p; pwr <= n; pwr *= p) { // loop on powers of prime p
	for (int j = pwr; j <= n; j += pwr) { // loop on multiples of power
	  omega.at(j) += 1;
	}
      }
    }
  }
}


// Function to return the largest rank (omega) of the numbers 1 to n.
// It is equal to log2 of the highest power of 2 <= n.  This function
// needs to be recalculated for each n if there are multiple n values.
// We could search the omega array, but generally it is quicker -
// O(log n) - to calculate it by doublings, avoiding float arithmetic.
int maxRank(int n) {
  int maxrank = 0;
  for (int pwr2 = 2; pwr2 <= n; pwr2 *= 2) {
    ++maxrank;
  }
  return maxrank;
}
