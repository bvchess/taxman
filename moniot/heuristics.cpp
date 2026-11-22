// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// Definitions of supported heuristics

#include "heuristics.h"


// The order of heuristics in the below list need not match the enum
// HeuristicID.  The order in which they are listed here is the order
// they appear in the usage page.  Descriptions are lists of strings.
std::vector<Heuristic> heuristicList = {
  { HeuristicID::BornFree, "BornFree", {
      "This heuristic is called \"born free\" because it is based on a",
      "matching on the Taxman graph that is free of flat alternating cycles",
      "by construction, and therefore always yields a playable",
      "move sequence.  It has been mathematically proven to win for all",
      "game sizes greater than 13.",
    }
  },
  { HeuristicID::BreakCycles, "BreakCycles", {
      "Starting with the maximum matching on the Taxman graph, break any",
      "alternating cycles using the Demetrescu-Finocchi algorithm to",
      "remove some edges from the matching, aiming to minimize the",
      "weight removed.  The result is a matching that is playable.",
    }
  },
  {  HeuristicID::GreedyOneTax, "GreedyOneTax", {
      "Like OneTax, pick a number that has only one divisor in play.  Differs",
      "from OneTax in that if two or more numbers have only one remaining divisor,",
      "instead of choosing the largest, choose the one with the largest",
      "difference between the player's take and the Taxman's take, like MaxTurn.",
      "Like OneTax, also include the tweak of taking a multiple of the pick",
      "if it will become unpickable. The score is mostly the same as OneTax,",
      "sometimes a little better, on average a little worse.",
    }
  },
  {  HeuristicID::MaxPick, "MaxPick", {
      "On each turn, take the largest number that can legally be picked.",
      "An example of a greedy strategy that usually loses.",
    }
  },
  {  HeuristicID::MaxTurn, "MaxTurn", {
      "Choose the pick that maximizes the difference between the player's",
      "take (the pick) and the taxman's take (the sum of the remaining",
      "active proper divisors of the pick) on each turn.  If two picks are",
      "tied for the maximum difference, take the larger one.",
    }
  },
  {  HeuristicID::MaxTurnPlus, "MaxTurn+", {
      "Start with the move sequence obtained by MaxTurn, and then look",
      "for \"freebies,\" which are picks that can be taken along the way",
      "that would otherwise go to the Taxman, and which do not prevent any",
      "other picks in the future move sequence from being taken.  There are two",
      "categories of freebies: (1) a number that will be taken as tax when",
      "the next pick is taken, and whose divisors (including itself) are",
      "a proper subset of the tax on that pick; and (2) a number that is",
      "a multiple of a divisor of the next pick whose divisors are a proper",
      "subset of the tax on that pick, and that will otherwise be left to the",
      "Taxman at the end of the game.  If there are two or more such",
      "freebies for a given pick, select the largest.",
    }
  },
  {  HeuristicID::OddsEvens, "OddsEvens", {
      "This heuristic has 3 stages:",
      "Stage 1: pick the largest prime.",
      "Stage 2: pick the available odd numbers from largest down, stopping at N/2.",
      "Stage 3: pick the available even numbers from smallest up, starting after N/2.",
    }
  },
  {  HeuristicID::OneTax, "OneTax", {
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
    }
  },
  {  HeuristicID::PureOneTax, "PureOneTax", {
      "Pick the largest number that has only one remaining divisor in",
      "play.  Note that this automatically picks the largest prime as the",
      "first pick, and often the largest square of a prime as the second,",
      "which are commonly the optimal first two moves.  The OneTax strategy",
      "modifies this pure OneTax strategy for improved performance.",
    }
  },
  { HeuristicID::Solvent, "Solvent", {
      "Brian Chess's solvent algorithm.  Try to include each number",
      "starting from the largest and working downward.  For each candidate",
      "for the pick set, include it if it has exactly one usable factor",
      "that is a neighbor on the Taxman graph, or if there is a neighboring",
      "factor that can only go to this candidate.  This implies that the set",
      "of picks remains playable as each number is added.  Once added to the",
      "set of picks, a number is never removed.",
    }
  },
  { HeuristicID::UpperBound, "UpperBound", {
      "Upper bound on the optimal score, equal to the weight of the",
      "maximum matching on the Taxman graph.  The sequence of picks",
      "is generally not playable, i.e., this heuristic does not",
      "provide a strategy for playing the game. If the bound is",
      "achievable, the BreakCycles heuristic will give the move sequence.",
    }
  },
};

const HeuristicID defaultHeuristic = HeuristicID::OneTax; // This is the default strategy

// Function to look up a heuristic in heuristicList, given its HeuristicID.

const Heuristic& find_Heuristic(HeuristicID heuristic) {
  for (std::size_t i = 0; i < heuristicList.size(); ++i) {
    if (heuristicList[i].id == heuristic) {
      return heuristicList[i];
    }
  }

  // This point is only reachable if there is a bug, e.g. a
  // HeuristicID enum value has been defined but no entry for it made
  // in heuristicList.  To keep the compiler from warning about
  // non-void function not returning a value, return the first
  // heuristic in the list.
  return heuristicList[0];
}
