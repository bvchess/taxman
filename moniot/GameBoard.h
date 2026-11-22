// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT


// ------------------------------------------------------//
//                                                       //
//                  GameBoard class                      //
//                                                       //
// ------------------------------------------------------//

// GameBoard: class for maintaining and updating state of the game.


#ifndef GAMEBOARD_H
#define GAMEBOARD_H
#include "defs.h"
#include <vector>
#include <algorithm>

typedef std::vector<bool> onBoard_t;

class GameBoard {
private:
  int gameSize;			// Number of numbers on the board initially
  onBoard_t onBoard;		// onBoard[x] is true if x is not taken yet
  std::vector<int> playerMoves;	// List of player's picks
  score_t playerScore;	// Sum of player's picks
  score_t taxmanScore;	// Sum of Taxman's takes
public:

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
  // taxes.  In case it is called with a non-playable sequence, it
  // includes checks to avoid counting pick or tax twice.

  void takePickAndTaxes( int pick, std::vector<int>& taxes ) {
    if ( pick != 0 ) {		// Can happen for n=1
      if ( onBoard.at(pick) ) {
	onBoard.at(pick) = false;
	playerMoves.push_back(pick);
	playerScore += pick;
      }
      
      for (int tax : taxes ) {
	if ( onBoard.at(tax) ) {
	  onBoard.at(tax) = false;
	  taxmanScore += tax;
	}
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

  // This function is used when heuristic is UpperBound, and moves are
  // not put into a playable sequence, which may not be possible.  In
  // that case the numbers used are put into sorted order.
  void sortPlayerMoves() {
    std::sort(playerMoves.begin(),playerMoves.end());
  }

  // Get and set methods
  int game_size() {
    return gameSize;
  }

  onBoard_t on_board() {
    return onBoard;
  }

  std::vector<int> player_moves() {
    return playerMoves;
  }

  score_t player_score() {
    return playerScore;
  }

  score_t taxman_score() {
    return taxmanScore;
  }
    
};				// End of class GameBoard declaration

#endif
