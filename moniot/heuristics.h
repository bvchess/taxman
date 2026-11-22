// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// Definitions of heuristic class

#ifndef HEURISTICS_H
#define HEURISTICS_H

#include <vector>

// Note: Other heuristic strategies may be added in future.  To add
// a new one, add an ID for it in the enum HeuristicID, add an entry in
// heuristicList, write an implementation function, and add a case for
// it in the switch ( heuristic ) in main.


// Supported heuristics for playing Taxman
enum class HeuristicID {
  BornFree,
  BreakCycles,
  GreedyOneTax,
  MaxPick,
  MaxTurn,
  MaxTurnPlus,
  OddsEvens,
  OneTax,
  PureOneTax,
  Solvent,
  UpperBound,
};

class Heuristic {
public:
  const HeuristicID id;
  const char* name;
  std::vector<const char*> desc;
};


// The supported heuristics and default are defined in heuristics.cpp
extern std::vector<Heuristic> heuristicList;
extern const HeuristicID defaultHeuristic; // This is the default strategy

// Function defined in heuristics.cpp

const Heuristic& find_Heuristic(HeuristicID heuristic);

#endif
