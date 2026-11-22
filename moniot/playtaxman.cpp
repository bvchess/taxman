// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT
//
// Program to play Taxman using the chosen strategy.
//
// Several heuristic strategies are implemented.  Use the command
//   playtaxman '-?'
// for a list of the heuristic strategies.  To see the description
// of a heuristic HEURISTIC, use
//   playtaxman -d -h HEURISTIC

//
// References:
// L. A. Carmony and R. L. Holliday, "An Example from Artificial
// Intelligence for CS1," SIGCSE Bulletin vol. 25 iss. 1 (1993).
//
// R. Moniot, "The Taxman Game," Math Horizons 14, Feb. (2007).
// J. Trono, "Taxman Revisited," SIGSCE Bulletin, vol. 26, no. 4 (Dec. 1994)/
//
// The heuristics UpperBound, BreakCycles, and BornFree are based
// on the Taxman graph and described in the following reference:
//
// A. F. Franklin and R. K. Moniot. The difficulty of beating the
// Taxman.  Discrete Applied Mathematics 339 (2023), 166-171.
//
// The Solvent heuristic is described in the documentation here:
//
// https://github.com/bvchess/taxman


// To test correctness, you should obtain the following scores for n=21:
// BornFree 123
// BreakCycles 135
// GreedyOneTax 144
// MaxPick 75
// MaxTurn 135
// MaxTurn+ 135
// OddsEvens 121
// OneTax 144
// PureOneTax 144
// Solvent 144
// UpperBound 145


#include "defs.h"		// Shared local definitions
#include "heuristics.h"		// Heuristic names and descriptions
#include "printformats.h"	// Print format names & descriptions
#include "gamedefs.h"		// Definitions for GameBoard heuristics
#include "GameBoard.h"		// Definition of GameBoard class

// Global variables declared here, declared extern in defs.h


std::vector<int> omega;
std::vector<int> primes;


int main( int argc, char *argv[] ) {

  // Variables controlling heuristic and print format options
  HeuristicID heuristic = defaultHeuristic;
  PrintFormatID printFormat = defaultPrintFormat;

  int describe_heuristic = 0;	// Flag for -d option

  int print_score = 0;		// Flag for -s option
  int print_moves = 0;		// Flag for -m option
  int print_fraction = 0;	// Flag for -f option

  bool default_output = true;	// Flag to detect whether output options given

  
  // Grab the command-line options

  // For the sake of portability, since MSVC does not support getopt,
  // we use our own code to get the options.  This provides somewhat less
  // functionality than getopt or getopt_long: no long --arguments
  // and single-letter arguments cannot be bunched together.

  // Define progname as bare name of program for usage calls.
  // On non-*nix systems the path component separator is something other than
  // '/': in that case use the full path as progname.
  const char* progname = std::strrchr(argv[0],'/'); // strip off leading path components
  if (progname == NULL) {		// '/' not found
    progname = std::strrchr(argv[0],'\\'); // Try MS-style path
  }
  if (progname == NULL) 	// if still not found, use full path as name
    progname = argv[0]; else ++progname;


  // Case-insensitive string compare that is portable.
  // Returns true if strings are equal, ignoring case.
  auto stringequali = [] (std::string str1, std::string str2) {
    if (str1.size() != str2.size()) {
      return false;
    }
    return std::equal(str1.begin(), str1.end(), str2.begin(), 
		      [](unsigned char a, unsigned char b) {
			return std::tolower(a) == std::tolower(b);		      });
  };

  int opt_char;
  for ( int iarg=1; iarg < argc; iarg++ ) {
    bool heuristicFound = false;   // for -h option error checking
    bool printFormatFound = false; // for -p option error checking
    int pos;
    if (argv[iarg][0] != '-') {
      usage(progname,EXIT_FAILURE);
    }
    else {
      opt_char = argv[iarg][1];	// the option letter
      switch( opt_char ) {
      case '\0':		// solitary "-"
	usage(progname,EXIT_FAILURE);
	break;
      case '?':
	usage(progname,EXIT_SUCCESS);
	break;
      case 'd':
	describe_heuristic = 1;
	break;
      case 'f':
	print_fraction = 1;
	default_output = false;
	break;
      case 'h':
	pos = 2;
	if ( argv[iarg][pos] == '\0' ) { // -h option followed by space: value is in next arg
	  ++iarg;
	  pos = 0;
	  if ( iarg >= argc ) {
	    usage(progname,EXIT_FAILURE);
	  }
	}
	for ( std::size_t h = 0; h < heuristicList.size(); h++ ) {
	  // case-insensitive string compare
	  if (stringequali(&argv[iarg][pos],heuristicList.at(h).name)) {
	    heuristicFound = true;
	    heuristic = heuristicList.at(h).id;
	    break;
	  }
	}
	if ( ! heuristicFound ) {    	// no heuristic matched to argument
	  std::cout << "Heuristic " << argv[iarg]+pos << " unknown" << std::endl;
	  usage( progname, EXIT_FAILURE );
	}
	break;
      case 'm':
	print_moves = 1;
	default_output = false;
	break;
      case 's':
	print_score = 1;
	default_output = false;
	break;
      case 'p':
	pos = 2;
	if ( argv[iarg][pos] == '\0' ) { // -p option followed by space: value is in next arg
	  ++iarg;
	  pos = 0;
	  if ( iarg >= argc ) {
	    usage(progname,EXIT_FAILURE);
	  }
	}
	
	for ( std::size_t pf = 0; pf < printFormatList.size(); pf++ ) {
	  // case-insensitive string compare
	  if (stringequali(&argv[iarg][pos],printFormatList.at(pf).name)) {
	    printFormatFound = true;
	    printFormat = printFormatList.at(pf).id;
	    break;
	  }
	}
	if ( ! printFormatFound ) { // no print format matched to argument
	  std::cerr << "Print format " << argv[iarg]+pos << " unknown" << std::endl;
	  usage(progname,EXIT_FAILURE);
	}
	break;
      default:
	usage(progname,EXIT_FAILURE);
	break;
      }
    }
  }
 
  // If -d  was given as an option, print the description of the
  // selected heuristic and exit.

  if (describe_heuristic) {
    Heuristic h = find_Heuristic(heuristic); // get list entry from ID
    std::cout << "Heuristic: " << h.name << std::endl;
    for (const char *line : h.desc) { // Go thru lines of description
      std::cout << line << std::endl;
    }
    exit(0);
  }


  // If user did not specify a selection of output, set the default of score & moves
  if (default_output) {
    print_fraction = 0;
    print_moves = 1;
    print_score = 1;
  }
  
  // Accept input of n or min_n-max_n or n1,n2,n3,...

  // Human format includes prompting for input
  if (printFormat == PrintFormatID::Human) std::cout << "How many numbers on the board? ";
  
  std::vector<int> nvalues;
  get_nvalues( nvalues );

  
  //  Fill lists of primes and prime omegas.  If multiple n values will
  //  be processed, lists are large enough for largest n.
  calcOmega(nvalues.back());

  
  // Opening delimiter for JSON or mathematica print format
  if ( printFormat == PrintFormatID::Math ) std::cout << "{" << std::endl;
  else if ( printFormat == PrintFormatID::JSON ) std::cout << "[" << std::endl;

  // Human or CSV print format: give the name of the heuristic used
  else {
    Heuristic h = find_Heuristic(heuristic);
    std::cout << "Heuristic: " << h.name << std::endl;
  }

  // output header row for CSV print format
  if ( printFormat == PrintFormatID::CSV ) {
    std::cout << "n";
    if ( print_score ) std::cout << ",score";
    if ( print_fraction ) std::cout << ",fraction";
    if ( print_moves ) std::cout << ",moves";
    std::cout << std::endl;
  }
  
  bool first_n = true; // to manage commas at end of multiple lines in json or math format

  for ( int n : nvalues ) {
    
    // Initialize the game board
    GameBoard b(n);

    // Play the game for this n using the chosen heuristic
    
    bool gameOn = true;
    switch ( heuristic ) {
     case HeuristicID::OneTax:
     case HeuristicID::PureOneTax:
       while ( gameOn ) {
	 gameOn = takeOneTax(b, heuristic);
       }
       break;
     case HeuristicID::GreedyOneTax:
       while ( gameOn ) {
	 gameOn = takeGreedyOneTax(b);
       }
       break;
     case HeuristicID::MaxTurn:
     case HeuristicID::MaxTurnPlus:
       while ( gameOn ) {
	 gameOn = takeMaxTurn(b);
       }
       if ( heuristic == HeuristicID::MaxTurnPlus ) {
	 gameOn = insertFreebies(b);
       }
       break;
     case HeuristicID::OddsEvens:
       gameOn = playOddsEvens(b);
       break;
    case  HeuristicID::MaxPick:
       while ( gameOn ) {
	 gameOn = takeMaxPick(b);
       }
       break;

       // Heuristics based on the Taxman graph
    case HeuristicID::UpperBound:
    case HeuristicID::BreakCycles:
    case HeuristicID::BornFree:
    case HeuristicID::Solvent:
      gameOn = playGraphHeuristic(b,heuristic);
      break;
      
     default:
       // Can only get here when a new heuristic is being added but an
       // implementation is not yet provided.
       std::cout << "Heuristic " << find_Heuristic(heuristic).name << " not supported at this time" << std::endl;
       gameOn = false;
       exit(EXIT_FAILURE);
    }


    // Game over: Taxman takes all remaining numbers
    b.takeLeftovers();

    // Output results for this n
 
    double fractOfPot = 2.0*(b.player_score())/((double)n*(n+1)); // player's score as fraction of total

    if ( printFormat == PrintFormatID::CSV ) {
      std::cout << n;
      if ( print_score ) {
	std::cout << "," << b.player_score();
      }
      if ( print_fraction ) {
	std::cout << "," << fractOfPot;
      }
      if ( print_moves ) {
	for ( int p : b.player_moves() ) {
        std::cout << "," << p;
	}
      }
      std::cout << std::endl;
    }
    else if (printFormat == PrintFormatID::Math) {
      // Mathematica format
      // score only: {n, score}
      // score only with fraction: {n, score, fraction}
      // moves only: {n, {pick, pick, ...}}
      // both: {n, {score, {pick, pick, ...}}
      // both with fraction: {n, {score, fraction, {pick, pick, ...}}
      // for multi-n output, add a comma at the end of output line except the last.
      if( !first_n && (nvalues.size() > 1) ) {
	std::cout << "," << std::endl;
      }
      first_n = false;

      std::cout << "  { " << n << ", ";
      if( print_moves && (print_score || print_fraction) ) std::cout << "{";
      if ( print_score ) {
	std::cout << b.player_score();
      }
      if ( print_fraction ) {
	if ( print_score ) std::cout << ", ";
	std::cout << fractOfPot;
      }
      if ( print_moves ) {
	int comma = 0;
	if ( print_score || print_fraction ) std::cout << ", ";
	std::cout << "{";
	for (int p : b.player_moves() ) {
	  if( comma ) std::cout << ", ";
	  comma = 1;
	  std::cout << p;
	}
	std::cout << "}";
      }
      if ( print_moves && (print_score || print_fraction) ) std::cout << "}";
      std::cout << " }";	// no newline; it will be put in with comma before printing next line
    }

    else if (printFormat == PrintFormatID::JSON) {
      // JSON format
      // score only: { "n": n, "score": score}
      // score only with fraction: { "n": n, "score": score, "fraction": fraction}
      // moves only: { "n": n, "moves": [pick, pick, ... ]}
      // both: {"n": n, "score": score, "moves": [pick, pick, ...]}
      // both with fraction: {"n": n, "score": score, "fraction": fraction, "moves": [pick, pick, ...]}
      if( !first_n && (nvalues.size() > 1) ) { // comma at end of line
	std::cout << "," << std::endl;
      }
      first_n = false;

      std::cout << "  { \"n\": " << n << ", ";
      if ( print_score ) {
	std::cout << "\"score\": " << b.player_score();
      }
      if ( print_fraction ) {
	if (print_score) std::cout << ", ";
	std::cout << "\"fraction\": " << fractOfPot;
      }
      if ( print_moves ) { // format {n, {pick, pick, ...}}
	int comma = 0;
	if ( print_score || print_fraction ) std::cout << ", ";
	std::cout << "\"moves\": [";
	for (int p : b.player_moves() ) {
	  if( comma ) std::cout << ", ";
	  comma = 1;
	  std::cout << p;
	}
	std::cout << "]";
      }
      std::cout << " }";
    }
    else {			// standard human-oriented format

      std::cout << "N=" << n << std::endl;
      if ( print_moves ) {
	if ( heuristic == HeuristicID::UpperBound ) {
	  std::cout << "Numbers used:";
	}
	else
	  std::cout << "Sequence:";
	if (b.player_moves().empty()) {
	  std::cout << " none";
	} else {
	  for (int x : b.player_moves()) {
	    std::cout << " " << x;
	  }
	}
	std::cout << std::endl;
      }
      // Since upper bound is not necessarily achievable, tailor the wording.
      if ( print_score ) {
	if ( heuristic == HeuristicID::UpperBound )
	  std::cout << "Upper bound: ";
	else 
	  std::cout << "Player score: ";
	std::cout << b.player_score() << std::endl;
	if ( heuristic == HeuristicID::UpperBound )
	  std::cout << "Points left: ";
	else 
	  std::cout << "Taxman score: ";
	std::cout << b.taxman_score() << std::endl;
      }
      if (print_fraction) {
	std::cout << "Fract of pot: " << fractOfPot << std::endl;
      }
    }
  }

  // Closing delimiter for JSON or mathematica output
  if ( printFormat == PrintFormatID::Math ) std::cout << std::endl << "}" << std::endl;
  else if ( printFormat == PrintFormatID::JSON ) std::cout << std::endl << "]" << std::endl;

  return 0;
}
