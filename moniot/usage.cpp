// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

//  usage: print usage of program

#include "defs.h"
#include "heuristics.h"
#include "printformats.h"	// Print format names & descriptions


// This function is called when program is invoked with incorrect
// parameters or the -? parameter.  It prints the help page
// and exits.  The exit_status should be EXIT_FAILURE if the reason is
// reporting an error, and EXIT_SUCCESS if requested to print the
// help page.

void usage(const char *progname, int exit_status) {

  std::cout << "Usage:" << std::endl;
  std::cout << progname << " [args]" << std::endl;
  std::cout << "   where args include:" << std::endl;
  std::cout << "     -?: print this usage page and exit" << std::endl;
  std::cout << "     -d: describe the selected heuristic and exit" << std::endl;
  std::cout << "     -f: print fraction of pot won" << std::endl;
  std::cout << "     -h HEURISTIC: use given heuristic to play" << std::endl;
  std::cout << "     -m: print move sequence" << std::endl;
  std::cout << "     -p FORMAT: print output in given format" << std::endl;
  std::cout << "     -s: print score" << std::endl;
  std::cout << " HEURISTIC can be one of: (case insensitive)" << std::endl;
  for ( std::size_t h = 0; h < heuristicList.size(); h++ ) {
    std::cout << "   " << heuristicList.at(h).name;
        if ( heuristicList.at(h).id == defaultHeuristic ) std::cout << " (default)";
    std::cout << std::endl;
  }
  std::cout << " FORMAT can be one of: (case insensitive)" << std::endl;
  for ( std::size_t pf = 0; pf < printFormatList.size(); pf++ ) {
    std::cout << "   " << printFormatList.at(pf).name << ": " << printFormatList.at(pf).desc;
   if ( printFormatList.at(pf).id == defaultPrintFormat ) std::cout << " (default)";
    std::cout << std::endl;
  }
  std::cout << " Default output selection if none specified is score and moves." << std::endl;
  std::cout << " Note that for UpperBound, moves are not necessarily playable." << std::endl;
  std::cout << std::endl;
  std::cout << "To run, enter game size N or range minN-maxN or list N1,N2,.. or combination" << std::endl;

  exit(exit_status);
}
