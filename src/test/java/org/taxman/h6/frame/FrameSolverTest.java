package org.taxman.h6.frame;

import org.junit.jupiter.api.Test;
import org.taxman.h6.game.Board;
import org.taxman.h6.bombus.Apiary;
import org.taxman.h6.bombus.Namer;
import org.taxman.h6.game.VerificationException;
import org.taxman.h6.util.TxSet;

import java.util.stream.IntStream;

public class FrameSolverTest {

    void runRange(int top, int expectedAccelerated) throws VerificationException {
        FrameSolver frameSolver = new FrameSolver();
        IntStream.rangeClosed(1, top).forEach(n -> {
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

    //@Test
    void justOne() throws VerificationException {
        int targetGame = 180; //464;
        FrameSolver frameSolver = new FrameSolver();
        System.out.println("playing n=" + targetGame);
        var sln = frameSolver.solve(targetGame);
        sln.verify(targetGame);
    }

}
