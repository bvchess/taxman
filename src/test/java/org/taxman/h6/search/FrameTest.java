/*
 * Copyright (c) Brian Chess 2019-2023.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
 * of the Software, and to permit persons to whom the Software is furnished to do
 * so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

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
