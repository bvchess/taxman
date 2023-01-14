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
import org.taxman.h6.frame.BaseFrame;
import org.taxman.h6.frame.FrameBuilder;
import org.taxman.h6.game.Board;
import org.taxman.h6.game.OptimalResult;
import org.taxman.h6.util.TxSet;
import org.taxman.h6.util.TxUnmodifiableSet;
import org.taxman.h6.util.TxWorkerThread;


public class SearchManagerTest {

    public static class SearchManagerTestRig {
        public final int n;
        public final SearchManager manager;
        public final BaseFrame oldSchool;
        public final TxUnmodifiableSet correctAnswer;

        private SearchManagerTestRig(int n, SearchManager manager, BaseFrame oldSchool, TxUnmodifiableSet correctAnswer) {
            this.n = n;
            this.manager = manager;
            this.oldSchool = oldSchool;
            this.correctAnswer = correctAnswer;
        }

        public static SearchManagerTestRig create(int n) {
            SearchManager.resetDebugFlags();
            TxWorkerThread.maxThreads = 1;
            var board = Board.of(n);
            var oldSchool = FrameBuilder.build(board);
            var manager = SearchManager.create(board, oldSchool);
            var optimalSolution = TxSet.of(OptimalResult.get(n).moves);
            var allCandidates = oldSchool.allCandidateNumbersIncludingDownstream();
            var optimalPromotions = TxUnmodifiableSet.and(optimalSolution, allCandidates);
            return new SearchManagerTestRig(n, manager, oldSchool, optimalPromotions);
        }

        public void setAllDebugFlags() {
            SearchManager.debugPrintSummary = true;
            SearchManager.debugPrintDetail = true;
            SearchManager.debugPrintFineGrainDetail = true;
        }

        public void runAndCheck() {
            var result = manager.findOptimalPromotions();
            var compare = correctAnswer.equals(result);
            assert compare : diagnosis(result);
        }

        private String diagnosis(TxSet wrong) {
            return String.format("for n=%d expected %,d=%s but got %,d=%s",
                    n, correctAnswer.sum(), correctAnswer, wrong.sum(), wrong);
        }
    }

    @Test
    public void testOneGame() {
        var rig = SearchManagerTestRig.create(120);
        //rig.manager.startingPoint.debugDumpFactors();
        rig.setAllDebugFlags();
        rig.runAndCheck();
    }

    @Test
    public void testFirst200() {
        for (int n = 1; n <= 200; n++) {
            var rig = SearchManagerTestRig.create(n);
            //rig.setAllDebugFlags();
            rig.runAndCheck();
        }
    }
}