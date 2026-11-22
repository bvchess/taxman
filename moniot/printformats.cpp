// Copyright (C) Robert K. Moniot
// November 2026
// SPDX-License-Identifier: MIT

// Definitions of supported print formats

#include "printformats.h"

std::vector<PrintFormat> printFormatList = {
  {PrintFormatID::Human, "Human", "Descriptive format"},
  {PrintFormatID::CSV, "CSV", "Comma-separated values for spreadsheet input"},
  {PrintFormatID::Math, "Math", "Initialization expressions in Mathematica or C++"},
  {PrintFormatID::JSON, "JSON", "JavaScript Object Notation"},
};

const PrintFormatID defaultPrintFormat = PrintFormatID::Human; // Default print format


