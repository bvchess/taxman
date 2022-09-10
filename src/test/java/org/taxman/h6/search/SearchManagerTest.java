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