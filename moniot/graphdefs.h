// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// Prototypes of functions in graphheuristics.cpp

#ifndef GRAPHDEFS_H
#define GRAPHDEFS_H


class TaxmanGraph;		// Forward references
class GameBoard;

// In maximum_weight_matching.cpp
void maximum_weight_matching( TaxmanGraph& g );

// In bornfree.cpp
void born_free_matching( TaxmanGraph& g );

// In breakcycles.cpp
void breakAlternatingCycles( TaxmanGraph& g_r );

// In solvent.cpp
ListOfVertexList maximal_factor_table( TaxmanGraph& g );
void solvent_matching( TaxmanGraph& g, const ListOfVertexList& mf );

// In findplayorder.cpp
bool findPlayOrder( TaxmanGraph& g_r, GameBoard& b );

#endif
