package org.taxman.h6.game;

import java.io.PrintStream;

public interface Solver {
    public Solution solve(int n);
    public void printInternalsReport(PrintStream ps);
}
