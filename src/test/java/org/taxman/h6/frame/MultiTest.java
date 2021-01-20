package org.taxman.h6.frame;

import org.junit.jupiter.api.Test;
import org.taxman.h6.game.Board;
import org.taxman.h6.bombus.Apiary;
import org.taxman.h6.bombus.Namer;
import org.taxman.h6.game.VerificationException;
import org.taxman.h6.util.TxSet;

import java.util.stream.IntStream;

public class MultiTest {

    void runRange(int top, int expectedAccelerated) throws VerificationException {
        Multi multi = new Multi();
        IntStream.rangeClosed(1, top).forEach(n -> {
            var sln = multi.solve(n);
            sln.verify(n);
        });
        var count = multi.getCountOfAccelerated();
        assert count == expectedAccelerated : "expected "+expectedAccelerated+" accelerated games, got " + count;
    }

    @Test
    void medium() throws VerificationException {
        runRange(200, 161);
    }

    //@Test
    void showPromotions() throws VerificationException {
        Multi multi = new Multi();
        var start = 450; //2;
        var last = 471;
        var prev = multi.loadPreviouslyComputed(start-1);
        for (int i=start; i <= last; i++) {
            var sln = multi.loadPreviouslyComputed(i);
            var basedOnPrev = multi.solveBasedOnPrevOnly(i) != null;
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
        Multi multi = new Multi();
        System.out.println("playing n=" + targetGame);
        var sln = multi.solve(targetGame);
        sln.verify(targetGame);
    }

}
