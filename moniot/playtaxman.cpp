// Program to play Taxman using the chosen strategy.
//
// Copyright (C) Robert K. Moniot
// June 2026
// SPDX-License-Identifier: MIT
//
// Several heuristic strategies are implemented.  See HeuristicList below
// for a list of strategies and their definitions.
//
// References:
// L. A. Carmony and R. L. Holliday, "An Example from Artificial Intelligence for
// CS1," SIGCSE Bulletin vol. 25 iss. 1 (1993).
// R. Moniot, "The Taxman Game," Math Horizons 14, Feb. (2007).
// J. Trono, "Taxman Revisited," SIGSCE Bulletin, vol. 26, no. 4 (Dec. 1994)/


#include <iostream>
#include <vector>
#include <getopt.h>
#include <sstream>
#include <string>
using namespace std;

// Supported heuristics for playing Taxman
typedef enum {
  OneTax,
  PureOneTax,
  GreedyOneTax,
  MaxTurn,
  MaxTurnPlus,
  OddsEvens,
  MaxPick,
} HeuristicID;

class Heuristic {
public:
  const HeuristicID id;
  const char* name;
  vector<const char*> desc;
};

// The order of heuristics in the below list need not match the enum
// HeuristicID.  The order in which they are listed here is the order
// they appear in the usage page.  Descriptions are NULL-terminated
// lists of strings.
vector<Heuristic> heuristicList = {
  { OneTax, "OneTax", {
      "Pick the largest number that has only one remaining divisor in",
      "play.  Note that this automatically picks the largest prime as the",
      "first pick, and often the largest square of a prime as the second,",
      "which are commonly the optimal first two moves. One tweak is made:",
      "If the chosen pick has a multiple that is going to be rendered",
      "unpickable when this pick is taken (i.e., it has no other divisors",
      "than this pick and its tax) and which is not a divisor of any other",
      "active number, then take that multiple instead, despite the fact",
      "that this gives the Taxman two numbers on that turn.  This always",
      "does at least as well as PureOneTax, and usually does significantly",
      "better.  It is the best-performing overall of the supported heuristics,",
      "so it is the default strategy.",
      NULL
    }
  },
  { PureOneTax, "PureOneTax", {
      "Pick the largest number that has only one remaining divisor in",
      "play.  Note that this automatically picks the largest prime as the",
      "first pick, and often the largest square of a prime as the second,",
      "which are commonly the optimal first two moves.  The OneTax strategy",
      "modifies this pure OneTax strategy for improved performance.",
      NULL
    }
  },
  { GreedyOneTax, "GreedyOneTax", {
      "Like OneTax, pick a number that has only one divisor in play.  Differs",
      "from OneTax in that if two or more numbers have only one remaining divisor,",
      "instead of choosing the laregest, choose the one with the largest",
      "difference between the player's take and the Taxman's take, like MaxTurn.",
      "Like OneTax, also include the tweak of taking a multiple of the pick",
      "if it will become unpickable. The score is mostly the same as OneTax,",
      "sometimes a little better, on average a little worse.",
      NULL
    }
  },
  { MaxTurn, "MaxTurn", {
      "Choose the pick that maximizes the difference between the player's",
      "take (the pick) and the taxman's take (the sum of the remaining",
      "active proper divisors of the pick) on each turn.  If two picks are",
      "tied for the maximum difference, take the larger one.",
      NULL
    }
  },
  { MaxTurnPlus, "MaxTurn+", {
      "Start with the move sequence obtained by MaxTurn, and then look",
      "for \"freebies,\"  which  are picks that can be taken along the way",
      "that would otherwise go to the Taxman, and which do not prevent any",
      "other picks in the future move sequence from being taken.  There are two",
      "categories of freebies: (1) a number that will be taken as tax when",
      "the next pick is taken, and whose divisors (including itself) are",
      "a proper subset of the tax on that pick; and (2) a number that is",
      "a multiple of a divisor of the next pick whose divisors are a proper",
      "subset of the tax on that pick, and that will otherwise be left to the",
      "Taxman at the end of the game.  If there are two or more such",
      "freebies for a given pick, select the largest.",
      NULL
    }
  },
  { OddsEvens, "OddsEvens", {
      "This heuristic has 3 stages:",
      "Stage 1: pick the largest prime.",
      "Stage 2: pick the available odd numbers from largest down, stopping before N/2.",
      "Stage 3: pick the available even numbers from smalles up, starting after N/2.",
      NULL
    }
  },
  { MaxPick, "MaxPick", {
      "On each turn, take the largest number that can legally be picked.",
      "An example of a greedy strategy that usually loses.",
      NULL
    }
  }
};

const HeuristicID defaultHeuristic = OneTax; // This is the default strategy

// Note: Other heuristic strategies may be added in future.  To add
// a new one, add an ID for it in the enum HeuristicID, add an entry in
// heuristicList, write an implementation function, and add a case for
// it in the switch ( heuristic ) in main.


// Supported print formats
typedef enum {
  Human,			// Human-friendly
  CSV,				// Comma-separated values for spreadsheet
  Math,				// Mathematica list or C/C++ struct
  JSON,				// JSON object
} PrintFormatID;
// Note: Human format implies prompting for N value(s); the other formats do
// not prompt since they are intended for redirecting output to a file or app.

class PrintFormat {
public:
  PrintFormatID id;
  const char* name;
  const char* desc;
};

vector<PrintFormat> printFormatList = {
  {Human, "Human", "Descriptive format"},
  {CSV, "CSV", "Comma-separated values for spreadsheet input"},
  {Math, "Math", "Initialization expressions in Mathematica or C++"},
  {JSON, "JSON", "JavaScript Object Notation"},
};

const PrintFormatID defaultPrintFormat = Human; // Default print format



// GameBoard: class for maintaining and updating state of the game.

typedef vector<bool> onBoard_t;  // Type for onBoard which tracks active status of numbers
typedef vector<int> numList_t;	 // Type for lists of numbers e.g. picks or divisors

class GameBoard {
public:
  int gameSize;			// Number of numbers on the board initially
  onBoard_t onBoard;		// onBoard[x] is true if x is not taken yet
  numList_t playerMoves;	// List of player's picks
  long long playerScore;	// Sum of player's picks
  long long taxmanScore;	// Sum of Taxman's takes

  GameBoard( int n ) {		// constructor
    // Initialize the game for a given n
    gameSize = n;
    onBoard = onBoard_t(n+1, true);	// indexing 1 to n, not using 0
    playerMoves = {};
    playerScore = 0;
    taxmanScore = 0;
  }

  // Function that returns true if x is still on the board.
  bool inPlay( int x ) {
    return onBoard.at(x);
  }
  
  // Function to take the player's pick and its taxes off the board.
  // This is just to do the bookkeeping, not validate the pick or the
  // taxes.

  void takePickAndTaxes( int pick, numList_t taxes ) {
    if ( pick != 0 ) {
      onBoard.at(pick) = false;
      playerMoves.push_back(pick);
      playerScore += pick;

      for (int tax : taxes ) {
	onBoard.at(tax) = false;
	taxmanScore += tax;
      }
    }
  }

  // This function is called at the end of the game to give the Taxman all
  // the remaining numbers.
  void takeLeftovers(void) {
    for (int i = 1; i <= gameSize; ++i) {
      if (inPlay(i)) {
	onBoard.at(i) = false;
	taxmanScore += i;
      }
    }
  }
};				// End of class GameBoard declaration


// This function is called when program is invoked with incorrect
// parameters or the -? | --help parameter.  It prints the help page
// and exits.  The exit_status should be EXIT_FAILURE if the reason is
// reporting an error, and EXIT_SUCCESS if requested to print the
// help page.

void usage(char *path, int exit_status) {
  char *progname = rindex(path,'/'); // strip off leading path components
  if (progname == NULL) progname = path; else ++progname;

  cout << "Usage:" << endl;
  cout << progname << " [args]" << endl;
  cout << "   where args include:" << endl;
  cout << "     -? | --help: print this usage page and exit" << endl;
  cout << "     -d | --describe: describe the selected heuristic and exit" << endl;
  cout << "     -f | --fraction: print fraction of pot won" << endl;
  cout << "     -h | --heuristic=HEURISTIC: use given heuristic to play" << endl;
  cout << "     -m | --moves: print move sequence" << endl;
  cout << "     -p | --print=FORMAT: print output in given format" << endl;
  cout << "     -s | --score: print score" << endl;
  cout << " HEURISTIC can be one of: (case insensitive)" << endl;
  for ( int h = 0; h < heuristicList.size(); h++ ) {
    cout << "   " << heuristicList.at(h).name;
    if ( h == defaultHeuristic ) cout << " (default)";
    cout << endl;
  }
  cout << " FORMAT can be one of: (case insensitive)" << endl;
  for ( int pf = 0; pf < printFormatList.size(); pf++ ) {
    cout << "   " << printFormatList.at(pf).name << ": " << printFormatList.at(pf).desc;
    if ( pf == defaultPrintFormat ) cout << " (default)";
    cout << endl;
  }
  cout << " Default output selection if none specified is score and moves." << endl;
  cout << endl;
  cout << "To run, enter game size N or range minN-maxN or list N1,N2,.. or combination" << endl;

  exit(exit_status);
}

// This routine is called by the section that inputs the values of N, if the
// input is not valid.  It exits with failure status.
void inputError(void) {
  cout << "Input error" << endl;
  exit(EXIT_FAILURE);
}


// Functions used by several heuristics

// Return a (possibly empty) list of divisors of x that are still on
// the board, sorted in ascending order.

numList_t getRemainingDivisors(int x, GameBoard& g) {
    numList_t divisors;

    for (int d = 1; d <= x/d; ++d) {
        if (x % d == 0) {
            int q = x / d;

            if (d < x && g.inPlay(d)) {divisors.push_back(d);}
            if (q != d && q < x && g.inPlay(q)) {divisors.push_back(q);}
        }
    }
    sort(divisors.begin(), divisors.end());
    return divisors;
}

// Return a (possibly empty) list of multiples of x that are still on
// the board, in ascending order.
numList_t getRemainingMultiples(int x, GameBoard& g) {
  numList_t multiples;
  for(int m = x+x; m <= g.gameSize; m += x) {
    if( g.inPlay(m) ) multiples.push_back(m);
  }
  return multiples;
}

// Sum the elements of a vector of integers

int sumVector( numList_t v ) {
  int sum=0;
  for (int x : v) sum += x;
  return sum;
}

// Function returns true if setA is a subset of setB
bool isSubset( numList_t setA, numList_t setB ) {
  bool subsetQ = true;
  for ( int y : setA ) {
    bool y_in_setB = false;
    for ( int x : setB ) {
      if ( x == y ) {
	y_in_setB = true;
	break;
      }
    }
    if ( ! y_in_setB ) {
      subsetQ = false;
      break;
    }
  }
  return subsetQ;
}

// This routine is used by playOddsEvens to find the largest prime on the board.
// It is not very efficient, but is used only by stage 1.  If another heuristic
// is implemented that makes heavy use of this function, it should be implemented
// in a more efficient way.

bool isPrime(int x) {
    if (x < 2) return false;
    for (int i = 2; i <= x/i; ++i) {
        if (x % i == 0) return false;
    }
    return true;
}



// Heuristic strategy implementations start here


// MaxTurn

// Described and named Maxturn by Carmony and Holliday.  Strategy #2 of Trono.

// Play one move using MaxTurn heuristic.  Return true if a pick is found.

bool takeMaxTurn(GameBoard& g) {
  int pick = 0;
  numList_t taxes;
  int maxgain = INT_MIN;

  for (int x = 2; x <= g.gameSize; x++) {
    numList_t divisors = getRemainingDivisors(x, g);
    if (divisors.size() > 0 ) {
      int gain = x - sumVector(divisors);
      if (gain >= maxgain) {
	pick = x;
	taxes = divisors;
	maxgain = gain;
      }
    }
  }
  if (pick == 0) return false;  // nothing found (implies game over)

  // Take the pick and give Taxman its divisors.
  g.takePickAndTaxes(pick, taxes);

  return true;
}


// MaxTurn+

// Described by Moniot as an "improved greedy" heuristic.  Carmony & Holliday
// give an example where MaxTurn misses out on a freebie, but they don't
// pursue that as an improvement on the strategy.

// Caller first plays the game using MaxTurn heuristic, then calls
// insertFreebies to make it MaxTurn+.

bool insertFreebies(GameBoard& g) {
  
  // Find numbers left over at the end of the game.  These will go to
  // the taxman and are eligible to be freebies if pickable at the time
  // the freebie is evaluated.

  // At the time this function is called, the game is over but the
  // leftovers are still on the board.

  onBoard_t leftOver = g.onBoard;

  numList_t origMoves = g.playerMoves; // save original play sequence and clear the board

  g = GameBoard(g.gameSize);		       // Re-initialize the game

  for (int pick : origMoves ) {

    // Get the Taxman's take for this pick
    numList_t taxes_on_pick = getRemainingDivisors(pick, g);

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
      numList_t candidates = {};
      for (int t : taxes_on_pick ) {
	// if t has active divisors, it is legally pickable.
	if ( getRemainingDivisors(t, g).size() > 0 ) candidates.push_back(t);

	for (int m : getRemainingMultiples(t, g)) { // loop through multiples of t
	  if ( leftOver.at(m) &&		 // Only consider leftovers
	       getRemainingDivisors(m, g).size() > 0 ) { // must be pickable
	    candidates.push_back(m);
	  }
	}
      }
      if ( candidates.size() > 0 ) {
	// sort and remove duplicates
	sort(candidates.begin(), candidates.end());
	auto new_end = unique(candidates.begin(), candidates.end());
	candidates.erase(new_end, candidates.end());
	
	// Go through candidates, looking for freebies
	int freebie = 0;
	numList_t taxes_on_freebie = {};
	for ( int c : candidates ) {
	  numList_t taxes_on_candidate = getRemainingDivisors(c, g);
	  // Include candidate in taxes if it is in pick taxes
	  if( pick%c == 0 ) taxes_on_candidate.push_back(c);
	  // If taxes are a proper subset of pick taxes, it is a freebie
	  if ( taxes_on_candidate.size() < taxes_on_pick.size() &&
	       isSubset(taxes_on_candidate, taxes_on_pick ) ) {
	    freebie = c;
	    taxes_on_freebie = taxes_on_candidate;
	  }
	  // Don't break when one is found; possibly a larger one will be found
	}
	if ( freebie != 0 ) {	// Freebie was found
	  g.takePickAndTaxes(freebie, taxes_on_freebie);
	  taxes_on_pick = getRemainingDivisors(pick, g); // update after taking freebie
	  improvementQ = true;
	}
      }	// end processing candidates
    }	// end loop while improvementQ

    // Done taking freebies: take the original pick
    g.takePickAndTaxes(pick, taxes_on_pick);

  } // end loop on picks
  
  return true;
}


// OddsEvens

// Strategy #7 of Trono.  Extremely simple but it wins reliably.
// Note that this heuristic does not perform nearly as well as Trono
// claims.  He credits Kevin Purcell and Robert Connelly for
// discovering it, and claims it outperforms OneTax, which it does not.
// It mostly scores quite a bit lower than OneTax or MaxTurn.

// Play entire game using the OddsEvens strategy.

bool playOddsEvens(GameBoard& g) {
  int pick = 0;
  numList_t taxes;
  int n = g.gameSize;

		// stage 1: pick largest prime.  Always pickable and tax = 1
  for ( int x = n; x > 1; --x ) {
    if ( isPrime(x) ) {
      numList_t divisors = {1};
      pick = x;
      taxes = divisors;
      break;
    }
  }
  g.takePickAndTaxes(pick,taxes);

		// stage 2: pick available odd numbers going down, >= n/2
  int largestOdd = (n%2 == 1? n: n-1);
  for ( int x = largestOdd; x >= n/2; x -= 2 ) {
    numList_t divisors = getRemainingDivisors(x, g);
    if ( divisors.size() > 0 ) { // legally pickable
      pick = x;
      taxes = divisors;
      g.takePickAndTaxes(pick,taxes);
    }
  }
  
		// stage 3: pick available even numbers going up, > n/2
  int smallestEven = n/2+1;
  if ( smallestEven%2 == 1 ) ++smallestEven;
  for ( int x = smallestEven; x <= n; x += 2 ) {
    numList_t divisors = getRemainingDivisors(x, g);
    if ( divisors.size() > 0 ) { // legally pickable
      pick = x;
      taxes = divisors;
      g.takePickAndTaxes(pick,taxes);
    }
  }
  
  return (pick != 0);		// (result unused in implementation below)

}



// OneTax and PureOneTax

// PureOneTax is the unmodified Strategy #3 of Trono.  It is a
// rather simple strategy, and performs very well.  OneTax improves
// on it by allowing the Taxman to take 2 numbers occasionally.

// Trono adds a different tweak to produce his Strategy #4: when 5 or
// fewer numbers are left that can be picked, switch to MaxTurn.
// This does not improve it as much as the tweak used here.

// Play one move of either the pure one-tax heuristic (no tweak) or
// the tweaked version, depending on argument heuristic.

bool takeOneTax(GameBoard& g, HeuristicID heuristic) {
  int pick = 0;
  numList_t taxes;

  for (int x = 2; x <= g.gameSize; x++) {
    numList_t divisors = getRemainingDivisors(x, g);
    if (divisors.size() == 1) {
      pick = x;  // this loop will choose pick as the largest with 1 divisor.
      taxes = divisors;
    }
  }
  if (pick == 0) return false;  // nothing found (implies game over)

  if (heuristic == OneTax) { 	// Skip this for PureOneTax
    // Check if this pick has only one active multiple that has no other divisors
    // besides this pick and its single tax.  If so, take that instead, giving
    // the Taxman the pick and its divisor.
    numList_t multiples = getRemainingMultiples(pick, g);
  
    for ( int m : multiples ) {
      numList_t divisors = getRemainingDivisors(m, g);
      if (divisors.size() == 2) {
	pick = m;
	taxes = divisors;
	break;
      }
    }
  }
  
  // Take the pick and give Taxman its divisor(s).
  g.takePickAndTaxes( pick, taxes );

  return true;
}


// GreedyOneTax

// A variant of the OneTax heuristic.  For N = 1..1000, it does better
// than OneTax 301 times, worse 505 times, and the same 194 times.
// When it is not the same, the difference is always less than 1%.

// Play one move of the GreedyOneTax heuristic.

bool takeGreedyOneTax(GameBoard& g) {
  int pick = 0;
  numList_t taxes;
  int maxgain = INT_MIN;

  for (int x = 2; x <= g.gameSize; x++) {
    numList_t divisors = getRemainingDivisors(x, g);
    if (divisors.size() == 1) {
      int gain = x - sumVector(divisors);
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
  // besides this pick and its single tax.  If so, take that instead.
  numList_t multiples = getRemainingMultiples(pick, g);
  
  for ( int m : multiples ) {
    numList_t divisors = getRemainingDivisors(m, g);
    if (divisors.size() == 2) {
      pick = m;
      taxes = divisors;
      break;
    }

  }
  
  // Take the pick and give Taxman its divisor(s).
  g.takePickAndTaxes( pick, taxes );

  return true;
}


// Strategy #1 of Trono.  Carmony and Holliday discuss it as an example
// of a greedy strategy that does not work well, since it usually
// loses.  It wins only for N=2, 5, and 11, and ties for N=3.

// Play one move of the MaxPick heuristic.

bool takeMaxPick(GameBoard& g) {
  int pick = 0;
  numList_t taxes;

  for (int x = g.gameSize; x > 0; --x) {
    numList_t divisors = getRemainingDivisors(x, g);
    if (divisors.size() > 0 ) {
      pick = x;
      taxes = divisors;
      break;
    }
  }

  if (pick == 0) return false;  // nothing found (implies game over)

  g.takePickAndTaxes( pick, taxes );
  
  return true;
}



// Main program starts here

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
  
  static struct option options[] = {
    {"help", no_argument, NULL, '?'},
    {"describe", no_argument, NULL, 'd'},
    {"fraction", no_argument, NULL, 'f'},
    {"heuristic", required_argument, NULL, 'h'},
    {"moves", no_argument, NULL, 'm'},
    {"print", required_argument, NULL, 'p'},
    {"score", no_argument, NULL, 's'},
    {NULL,0,NULL,0},
  };

  char opt_char;

  while ((opt_char=getopt_long(argc, argv, "?dfh:mp:s", options, NULL)) != -1) {
    bool heuristicFound = false;   // for -h option error checking
    bool printFormatFound = false; // for -p option error checking

    switch (opt_char) {
    case '?':
      usage( argv[0], EXIT_SUCCESS );
      break;
    case 'd':
      describe_heuristic = 1;
      break;
    case 'f':
      print_fraction = 1;
      default_output = false;
      break;
    case 'm':
      print_moves = 1;
      default_output = false;
      break;
    case 's':
      print_score = 1;
      default_output = false;
      break;
    case 'h':
      for ( int h = 0; h < heuristicList.size(); h++ ) {
	if (strcasecmp(optarg,heuristicList.at(h).name) == 0) {
	  heuristicFound = true;
	  heuristic = heuristicList.at(h).id;
	  break;
	}
      }
      if ( ! heuristicFound ) {		// no heuristic matched to argument
	cout << "Heuristic " << optarg << " unknown" << endl;
	usage( argv[0], EXIT_FAILURE );
      }
      break;
    case 'p':
      for ( int pf = 0; pf < printFormatList.size(); pf++ ) {
	if (strcasecmp(optarg,printFormatList.at(pf).name) == 0) {
	  printFormatFound = true;
	  printFormat = printFormatList.at(pf).id;
	  break;
	}
      }
      if ( ! printFormatFound ) { // no print format matched to argument
	cout << "Print format " << optarg << " unknown" << endl;
	usage( argv[0], EXIT_FAILURE );
      }
      break;
    default:
      usage( argv[0], EXIT_FAILURE );
      break;
    }
  }

  // If -d | --describe was given as an option, print the description of the
  // selected heuristic and exit.

  if (describe_heuristic) {
    struct Heuristic h = heuristicList.at(heuristic);
    cout << "Heuristic: " << h.name << endl;
    for (int line=0; h.desc[line] != NULL; ++line) { // Go thru lines of description
      cout << h.desc[line] << endl;
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
  
  int min_n, max_n;
  numList_t nvalues;
  string input_line;
  char sep;
  
  // Human format includes prompting for input
  if (printFormat == Human) cout << "How many numbers on the board? ";
  
  getline( cin, input_line );
  stringstream input_ss( input_line );
  if ( !(input_ss >> min_n) ) {
    inputError();
    exit(1);
  }
  nvalues.push_back(min_n);
  while ( input_ss >> sep ) {
    if( !(input_ss >> max_n) ) {
      inputError();
    }
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

  sort(nvalues.begin(), nvalues.end()); // process n values in ascending order regardless of input order
  
  // Opening delimiter for JSON or mathematica print format
  if ( printFormat == Math ) cout << "{" << endl;
  else if ( printFormat == JSON ) cout << "[" << endl;
  // Human or CSV print format: give the name of the heuristic used
  else cout << "Heuristic: " << heuristicList.at(heuristic).name << endl;

  // output header row for CSV print format
  if ( printFormat == CSV ) {
    cout << "n";
    if ( print_score ) cout << ",score";
    if ( print_fraction ) cout << ",fraction";
    if ( print_moves ) cout << ",moves";
    cout << endl;
  }
  
  bool first_n = true; // to manage commas at end of multiple lines in json or math format

  for ( int n : nvalues ) {
    
    // Initialize the game
    GameBoard g(n);

    // Play the game for this n using the chosen heuristic
    
    bool gameOn = true;
    switch ( heuristic ) {
     case OneTax:
     case PureOneTax:
       while ( gameOn ) {
	 gameOn = takeOneTax(g, heuristic);
       }
       break;
     case GreedyOneTax:
       while ( gameOn ) {
	 gameOn = takeGreedyOneTax(g);
       }
       break;
     case MaxTurn:
     case MaxTurnPlus:
       while ( gameOn ) {
	 gameOn = takeMaxTurn(g);
       }
       if ( heuristic == MaxTurnPlus ) {
	 gameOn = insertFreebies(g);
       }
       break;
     case OddsEvens:
       gameOn = playOddsEvens(g);
       break;
     case MaxPick:
       while ( gameOn ) {
	 gameOn = takeMaxPick(g);
       }
       break;
     default:		
       cout << "Heuristic " << heuristicList.at(heuristic).name
	    << " not supported at this time" << endl;
       gameOn = false;
       break;
    }


    // Game over: Taxman takes all remaining numbers
    g.takeLeftovers();

    // Output results for this n
 
    double fractOfPot = 2.0*g.playerScore/(n*(n+1)); // player's score as fraction of total

    if ( printFormat == CSV ) {
      cout << n;
      if ( print_score ) {
	cout << "," << g.playerScore;
      }
      if ( print_fraction ) {
	cout << "," << fractOfPot;
      }
      if ( print_moves ) {
	for ( int p : g.playerMoves ) {
        cout << "," << p;
	}
      }
      cout << endl;
    }
    else if (printFormat == Math) {
      // Mathematica format
      // score only: {n, score}
      // score only with fraction: {n, score, fraction}
      // moves only: {n, {pick, pick, ...}}
      // both: {n, {score, {pick, pick, ...}}
      // both with fraction: {n, {score, fraction, {pick, pick, ...}}
      // for multi-n output, add a comma at the end of output line except the last.
      if( !first_n && (nvalues.size() > 1) ) {
	cout << "," << endl;
      }
      first_n = false;

      cout << "  { " << n << ", ";
      if( print_moves && (print_score || print_fraction) ) cout << "{";
      if ( print_score ) {
	cout << g.playerScore;
      }
      if ( print_fraction ) {
	if ( print_score ) cout << ", ";
	cout << fractOfPot;
      }
      if ( print_moves ) {
	int comma = 0;
	if ( print_score || print_fraction ) cout << ", ";
	cout << "{";
	for (int p : g.playerMoves ) {
	  if( comma ) cout << ", ";
	  comma = 1;
	  cout << p;
	}
	cout << "}";
      }
      if ( print_moves && (print_score || print_fraction) ) cout << "}";
      cout << " }";	// no newline; it will be put in with comma before printing next line
    }

    else if (printFormat == JSON) {
      // JSON format
      // score only: { "n": n, "score": score}
      // score only with fraction: { "n": n, "score": score, "fraction": fraction}
      // moves only: { "n": n, "moves": [pick, pick, ... ]}
      // both: {"n": n, "score": score, "moves": [pick, pick, ...]}
      // both with fraction: {"n": n, "score": score, "fraction": fraction, "moves": [pick, pick, ...]}
      if( !first_n && (nvalues.size() > 1) ) { // comma at end of line
	cout << "," << endl;
      }
      first_n = false;

      cout << "  { \"n\": " << n << ", ";
      if ( print_score ) {
	cout << "\"score\": " << g.playerScore;
      }
      if ( print_fraction ) {
	if (print_score) cout << ", ";
	cout << "\"fraction\": " << fractOfPot;
      }
      if ( print_moves ) { // format {n, {pick, pick, ...}}
	int comma = 0;
	if ( print_score || print_fraction ) cout << ", ";
	cout << "\"moves\": [";
	for (int p : g.playerMoves ) {
	  if( comma ) cout << ", ";
	  comma = 1;
	  cout << p;
	}
	cout << "]";
      }
      cout << " }";
    }
    else {			// standard human-oriented format

      cout << "N=" << n << endl;
      if ( print_moves ) {
	cout << "Sequence:";
	if (g.playerMoves.empty()) {
	  cout << " none";
	} else {
	  for (int x : g.playerMoves) {
	    cout << " " << x;
	  }
	}
	cout << endl;
      }
      if ( print_score ) {
	cout << "Player score: " << g.playerScore << endl;
	cout << "Taxman score: " << g.taxmanScore << endl;
      }
      if (print_fraction) {
	cout << "Fract of pot: " << fractOfPot << endl;
      }
    }
  }

  // Closing delimiter for JSON or mathematica output
  if ( printFormat == Math ) cout << endl << "}" << endl;
  else if ( printFormat == JSON ) cout << endl << "]" << endl;

  return 0;
}
