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

import java.util.List;

import static org.taxman.h6.util.TxUnmodifiableSet.EmptySet;


public class FrameSetTest {

    final static Board board85 = Board.of(85);
    private final static List<TxUnmodifiableSet> columnsFor85 = List.of(
            TxUnmodifiableSet.of(4, 6, 9, 10, 14, 15, 21, 25),
            TxUnmodifiableSet.of(45, 50, 63, 75),
            TxUnmodifiableSet.of(8, 12, 20, 28, 30, 42),
            TxUnmodifiableSet.of(56, 60, 84),
            TxUnmodifiableSet.of(16, 24, 36, 40),
            TxUnmodifiableSet.of(48, 72, 80)
    );
    private final static List<TxUnmodifiableSet> reducedColumnsFor85 = List.of(
            TxUnmodifiableSet.of(4, 6, 9, 10, 14, 15, 21, 25),
            TxUnmodifiableSet.of(28, 42, 45, 50, 63, 75),
            EmptySet,
            EmptySet,
            EmptySet,
            EmptySet
    );

    private final static List<TxUnmodifiableSet> framesFrom120 = List.of(
            TxUnmodifiableSet.of(2, 3, 5, 7, 17, 23, 37),
            TxUnmodifiableSet.of(69, 74, 85, 111, 115, 119),
            TxUnmodifiableSet.of(4, 6, 9, 10, 14, 15, 21, 22, 25, 26, 33, 35, 39, 49),
            TxUnmodifiableSet.of(63, 66, 70, 75, 78, 98, 99, 105, 117)
    );


    final static TxUnmodifiableSet candidates85 = TxUnmodifiableSet.of(8, 12, 16, 20, 24, 28, 30, 36, 40, 42);
    final static TxSet allNumbersFor85 = TxSet.of(columnsFor85.stream().flatMapToInt(TxSet::stream));
    final static FrameSet fs85 = create85(board85, columnsFor85);


    private static FrameSet create85(Board b, List<TxUnmodifiableSet> columns) {
        FactorProvider math = FactorProvider.create(b, allNumbersFor85);
        return FrameSet.create(math, columns);
    }

    @Test
    public void display() {
        fs85.debugDump();
    }

    @Test
    void promote() {
        var fs = create85(board85, columnsFor85);
        var newFs = fs.promote(42).promote(40).promote(28);
        var rightAnswer = create85(board85, reducedColumnsFor85);
        assert rightAnswer.equals(newFs);
    }

    @Test
    void candidates() {
        var cand = fs85.narrowCandidateSet(candidates85);
        assert cand.equals(candidates85) : "candidates for 85 should be " + candidates85 + " not " + cand;
    }

    @Test
    void maxMoves() {
        assert fs85.computeMaxNumberOfPromotions() == 3;
        fs85.debugPromotionEstimate();
        var newFs = fs85.promote(42).promote(40).promote(28);
        assert newFs.computeMaxNumberOfPromotions() == 0;
    }

    static void dumpFrame(Frame f, FactorProvider math) {
        f.getMoves().streamDescending().forEach(n -> {
            System.out.printf("%d: %s\n", n, TxSet.and(TxSet.of(math.getFactors(n)), f.getFactors()));
        });
    }

    //@Test
    void aTestFrom120() {
        var b = Board.of(120);
        var allNumbers = TxSet.of(framesFrom120.stream().flatMapToInt(TxSet::stream));
        var math = FactorProvider.create(b, allNumbers);
        var fs = FrameSet.create(math, framesFrom120);
        var nextState = fs.promote(49);
        System.out.println("after promoting 49:");
        var frame = nextState.getFrames().get(1);
        dumpFrame(frame, math);
        System.out.println("after promoting 25:");
        nextState = fs.promote(25);
        frame = nextState.getFrames().get(1);
        dumpFrame(frame, math);
    }

}