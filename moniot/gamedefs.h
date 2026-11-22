// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// Definitions for the various game strategies

#ifndef GAMEDEFS_H
#define GAMEDEFS_H

#include "defs.h"
#include "heuristics.h"		// Heuristic names
#include "GameBoard.h"

// Prototypes of functions defined in various files

// In common.cpp:

numList getRemainingDivisors( int x, GameBoard& b );
numList getRemainingMultiples( int x, GameBoard& b );
score_t sumVector( const numList& v );
bool isSubset( const numList& setA, const numList& setB );

// In maxturn.cpp:
bool takeMaxTurn(GameBoard& b);

// In maxturnplus.cpp
bool insertFreebies(GameBoard& b);

// In oddsevens.cpp
bool playOddsEvens(GameBoard& b);

// In onetax.cpp
bool takeOneTax(GameBoard& b, HeuristicID heuristic);

// In greedyonetax.cpp
bool takeGreedyOneTax(GameBoard& b);

// In maxpick.cpp
bool takeMaxPick(GameBoard& b);

// In graphheuristics.cpp
bool playGraphHeuristic( GameBoard& b, HeuristicID heuristic );

#endif
