package org.taxman.h6.game;

public class VerificationException extends RuntimeException {
    public final int n;
    private Solution sln;

    public VerificationException(String msg, int n) {
        super(msg);
        this.n = n;
    }

    public VerificationException(String msg) {
        this(msg, -1);
    }

    public Solution getSolution() {
        return sln;
    }

    public VerificationException setSolution(Solution sln) {
        this.sln = sln;
        return this;
    }

    public String solutionString() {
        if (sln != null) {
            return sln.moves.toString();
        } else {
            return "(no solution provided)";
        }
    }
}
