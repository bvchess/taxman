// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// get_nvalues reads in the set of N values to be processed


#include "defs.h"

const int MAX_SANE_N = 40000;  // change this if you wish, may cause crash


// This routine makes sure the input value of n is valid. MAX_SANE_N should
// ensure that the calculation n*(n+1) is < max int.  For 32-bit signed
// ints that is n <~ 46 340.
void validate_n(int n) {
  if ( n < 1 || n > MAX_SANE_N ) {
    std::cout << "Require 1 <= N <= " << MAX_SANE_N << std::endl;
    exit(EXIT_FAILURE);
  }
}


// This routine is called by get_nvalues if the
// input is not valid.  It exits with failure status.
void inputError( void ) {
  std::cout << "Input error" << std::endl;
  exit(EXIT_FAILURE);
}


// Function to fill the array of n values
void get_nvalues( std::vector<int>& nvalues ) {
  int min_n, max_n;
  std::string input_line;
  char sep;
  
  std::getline( std::cin, input_line );
  
  std::stringstream input_ss( input_line );
  if ( !(input_ss >> min_n) ) {
    inputError();
  }
  validate_n(min_n);
  nvalues.push_back(min_n);
  while ( input_ss >> sep ) {
    if( !(input_ss >> max_n) ) {
      inputError();
    } 
    validate_n(max_n);
    switch( sep ) {
    case '-':			// minN-maxN range
      if ( max_n < min_n ) {
	inputError();
      }
      for (int i = min_n+1; i <= max_n; i++) nvalues.push_back(i);
      min_n = max_n;
      break;
    case ',':			// N1,N2,... list
      nvalues.push_back( max_n );
      min_n = max_n;  // in case next sep is '-'
      break;
    default:
      inputError();
      break;
    }
  }
  
  // process n values in ascending order regardless of input order
  std::sort(nvalues.begin(), nvalues.end());
  // in case user entered overlapping ranges, remove duplicates
  nvalues.erase(std::unique(nvalues.begin(), nvalues.end()), nvalues.end());  
}
