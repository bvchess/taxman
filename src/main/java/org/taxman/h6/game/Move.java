package org.taxman.h6.game;

import org.taxman.h6.util.TxSet;

public class Move {
    public final int n;
    public final Board before;
    public final Board remainder;
    public final TxSet tax;

    public Move(int n, Board b) throws VerificationException {
        if (!b.set.contains(n))
            throw new VerificationException("illegal move: "+n+", is not on the board: " + b, n);
        this.n = n;
        before = b;
        TxSet toRemove = b.allFactors(n);
        tax = TxSet.subtract(toRemove, n).unmodifiable();
        if (tax.size() == 0)
            throw new VerificationException("illegal move: "+n+", no tax paid", n);
        remainder = b.subtract(toRemove);
    }
}
