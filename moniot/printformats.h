// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// Definitions of output format class

#ifndef PRINTFORMATS_H
#define PRINTFORMATS_H

#include <vector>

// Supported print formats
enum class PrintFormatID {
  Human,			// Human-friendly
  CSV,				// Comma-separated values for spreadsheet
  Math,				// Mathematica list or C/C++ struct
  JSON,				// JSON object
};

// Note: Human format implies prompting for N value(s); the other formats do
// not prompt since they are intended for redirecting output to a file or app.

class PrintFormat {
public:
  PrintFormatID id;
  const char* name;
  const char* desc;
};


// The supported print formats and default are defined in printformats.cpp
extern std::vector<PrintFormat> printFormatList;
extern const PrintFormatID defaultPrintFormat;

#endif
