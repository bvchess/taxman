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

package org.taxman.h6.frame;

import org.junit.jupiter.api.Test;
import org.taxman.h6.bombus.BombusSolution;
import org.taxman.h6.game.Board;
import org.taxman.h6.game.OptimalResult;
import org.taxman.h6.game.Solution;
import org.taxman.h6.util.TxList;

import java.util.stream.IntStream;

public class GreedySolverTest {

    BombusSolution getExistingSolution(int n) {
        var or = OptimalResult.get(n);
        BombusSolution result = null;
        if (or != null && or.moves != null) {
            return BombusSolution.upgrade(new Solution(Board.of(n), TxList.of(or.moves)));
        }
        return result;
    }

    void doOne(int n) {
        var printSuboptimal = false;
        var g = new GreedySolver(4);
        g.warnOnImperfectScore = false;
        var sln = g.solve(n);
        var optimal = getExistingSolution(n);
        if (printSuboptimal && optimal.score() != sln.score()) {
            System.out.printf("%d: got %d, optimal is %d, difference is %d\n",
                    n, sln.score(), optimal.score(), optimal.score() - sln.score());
        }
    }

    @Test
    void doAFew() {
        IntStream.rangeClosed(1, 100).forEach(this::doOne);
    }

}
