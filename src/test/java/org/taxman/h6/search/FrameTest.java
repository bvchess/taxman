package org.taxman.h6.search;

import org.junit.jupiter.api.Test;
import org.taxman.h6.game.Board;
import org.taxman.h6.util.TxSet;
import org.taxman.h6.util.TxUnmodifiableSet;

public class FrameTest {

    @Test
    void cantPromoteIfItsNotThere() {
        var b = Board.of(200);
        var math = FactorProvider.create(b, TxSet.of(108, 72, 54, 36, 10));
        var frame = new Frame(0, TxUnmodifiableSet.of(108, 72), TxUnmodifiableSet.of(54, 36));
        boolean caughtException = false;
        try {
            frame.promoteOutOf(10, math);
        } catch (AssertionError e) {
            caughtException = true;
        }

        assert caughtException;
    }

    @Test
    void tiny() {
        var b = Board.of(200);
        var moves = TxUnmodifiableSet.of(108, 72);
        var factors = TxUnmodifiableSet.of(54, 36, 10);
        var math = FactorProvider.create(b, TxSet.or(moves, factors));
        var frame = new Frame(0, moves, factors);
        var result = frame.promoteOutOf(10, math);
        assert result.getFactors().size() == 0;
        assert result.getMoves().size() == 0;
    }

    @Test
    void derivedFrom120() {
        var b = Board.of(200);
        var moves = TxUnmodifiableSet.of(69, 74, 77, 82, 85, 86, 87, 91, 93, 94, 95, 106, 111, 115, 118, 11);
        var factors = TxUnmodifiableSet.of(2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59);
        var math = FactorProvider.create(b, TxSet.or(moves, factors, TxSet.of(39)));
        var frame = new Frame(0, moves, factors);
        var result = frame.promoteInto(9, math);
        assert result == null;
    }
}
