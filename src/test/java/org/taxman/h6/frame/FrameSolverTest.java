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
import org.taxman.h6.game.Board;
import org.taxman.h6.bombus.Apiary;
import org.taxman.h6.bombus.Namer;
import org.taxman.h6.game.VerificationException;
import org.taxman.h6.oldsearch.OldSearch;
import org.taxman.h6.util.Memo;
import org.taxman.h6.util.TxSet;
import org.taxman.h6.util.TxWorkerThread;

import java.util.stream.IntStream;

public class FrameSolverTest {

    void runRange(int top, int expectedAccelerated) throws VerificationException {
        FrameSolver frameSolver = new FrameSolver();
        IntStream.rangeClosed(1, top).forEach(n -> {
            //System.out.println("running n=" + n);
            var sln = frameSolver.solve(n);
            sln.verify(n);
        });
        var count = frameSolver.getCountOfAccelerated();
        assert count == expectedAccelerated : "expected "+expectedAccelerated+" accelerated games, got " + count;
    }

    @Test
    void medium() throws VerificationException {
        runRange(200, 146);
    }

    //@Test
    void accel() {
        IntStream.rangeClosed(2, 683).forEach(n -> {
            FrameSolver frameSolver = new FrameSolver();
            var sln = frameSolver.solveBasedOnPrevOnly(n);
            if (sln != null) {
                var sln2 = frameSolver.solveTheHardWay(n);
                assert sln.moves.sum() == sln2.moves.sum();
                System.out.println("verified " + n);
            }
        });


    }

    //@Test
    void showPromotions() throws VerificationException {
        FrameSolver frameSolver = new FrameSolver();
        var start = 450; //2;
        var last = 471;
        var prev = frameSolver.loadPreviouslyComputed(start-1);
        for (int i=start; i <= last; i++) {
            var sln = frameSolver.loadPreviouslyComputed(i);
            var basedOnPrev = frameSolver.solveBasedOnPrevOnly(i) != null;
            if (!basedOnPrev) {
                var added = TxSet.subtract(TxSet.of(sln.promotions), TxSet.of(prev.promotions));
                var subtracted = TxSet.subtract(TxSet.of(prev.promotions), TxSet.of(sln.promotions));

                var board = Board.of(i);
                var a1 = new Apiary(board, new Namer());
                var a1Candidates = a1.getPromotionCandidateNumbers();
                var noLongerAnOption = TxSet.subtract(subtracted, a1Candidates);
                var inTheBag = a1.getSolution().sum();
                var promotionSumMax = prev.score() + i - inTheBag;
                var frame = FrameBuilder.build(board);
                var maxPromotions = frame.estimateMaxPromotions(0);

                System.out.printf("%d\n       added: %s=%d\n  subtracted: %s=%d\n  no longer an option: %s\n  all optimal promotions: %s\n",
                        i, added, added.sum(), subtracted, subtracted.sum(), noLongerAnOption, sln.promotions);
                System.out.printf("  max number of promotions: %d, all candidates: %s\n", maxPromotions, a1Candidates);
                System.out.printf("  max feasible promotion sum: %d, achieved: %d, diff: %d\n",
                        promotionSumMax, sln.promotions.sum(), promotionSumMax-sln.promotions.sum());
            }

            prev = sln;
        }
    }

    void setDebugFlags(boolean value) {
        FrameSolver.printFrames = value;
        FrameSolver.printSearch = value;
        FrameSolver.printAccelerationFailures = value;
        OldSearch.printStatsPerTarget = value;
        OldSearch.printSummary = value;
        FrameSolver.printCacheStats = value;
        Memo.trackHits = value;
    }

    @Test
    void justOne() throws VerificationException {
        int targetGame = 85;
        TxWorkerThread.maxThreads = 1;
        setDebugFlags(true);
        FrameSolver frameSolver = new FrameSolver();
        System.out.println("playing n=" + targetGame);
        var sln = frameSolver.solve(targetGame);
        sln.verify(targetGame);
        setDebugFlags(false);
    }

}
