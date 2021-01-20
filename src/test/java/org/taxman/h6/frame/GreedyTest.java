package org.taxman.h6.frame;

import org.junit.jupiter.api.Test;
import org.taxman.h6.bombus.Apiary;
import org.taxman.h6.bombus.BombusSolution;
import org.taxman.h6.bombus.Namer;
import org.taxman.h6.game.Board;
import org.taxman.h6.game.OptimalResult;
import org.taxman.h6.game.Solution;
import org.taxman.h6.util.TxList;
import org.taxman.h6.util.TxPredicate;
import org.taxman.h6.util.TxSet;

import static org.taxman.h6.util.TxUnmodifiableSet.EmptySet;

public class GreedyTest {

    BombusSolution getSolution(int n) {
        var or = OptimalResult.get(n);
        BombusSolution result = null;
        if (or != null && or.moves != null) {
            return BombusSolution.upgrade(new Solution(Board.of(n), TxList.of(or.moves)));
        }
        return result;
    }

    @Test
    void letsDoOne() {
        int n = 81;
        var board = Board.of(n);
        var a = new Apiary(board, new Namer());
        var inTheBag = a.getSolution().sum();
        var frame = FrameBuilder.build(board);
        var maxPromotions = frame.estimateMaxPromotions(0);
        var candidates = frame.allCandidateNumbersIncludingDownstream();
        var prev = getSolution(n-1);
        var promotionSumMax = prev.score() + n - inTheBag;

        if (candidates.largest(maxPromotions).sum() < promotionSumMax) {
            //System.out.println(n + ": lowering the max by " + (promotionSumMax - candidates.largest(maxPromotions).sum()));
            promotionSumMax = candidates.largest(maxPromotions).sum();
        }


        var p = new TxPredicate<TxSet>(c -> frame.fits(EmptySet, c));
        var searchResult = Search.findLargest(candidates, maxPromotions, promotionSumMax, p);
        var greedyResult =  Greedy.find(candidates, maxPromotions, p);

        System.out.println("greedy result: " + greedyResult.sum() + " = " + greedyResult);
        System.out.println("search result: " + searchResult.sum() + " = " + searchResult);
    }

    @Test
    void comboTest() {
        var s = TxSet.of(1, 2 , 3, 4, 5);
        var combos = Greedy.combinations(s, 3);
        combos.forEach(combo -> System.out.println(combo));
    }
}
