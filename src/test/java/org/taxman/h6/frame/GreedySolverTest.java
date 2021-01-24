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
        var g = new GreedySolver(4);
        var sln = g.solve(n);
        var optimal = getExistingSolution(n);
        if (optimal.score() != sln.score()) {
            System.out.printf("%d: got %d, optimal is %d, difference is %d\n",
                    n, sln.score(), optimal.score(), optimal.score() - sln.score());
        }
    }

    @Test
    void doAFew() {
        IntStream.rangeClosed(1, 100).forEach(this::doOne);
    }

}
