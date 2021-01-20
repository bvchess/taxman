package org.taxman.h6.game;

import org.junit.jupiter.api.Test;
import org.taxman.h6.game.Board;
import org.taxman.h6.game.Solution;
import org.taxman.h6.util.TxList;

public class SolutionTest {

    @Test
    public void display() {
        Solution s = new Solution(Board.of(10), TxList.of(7, 9, 6, 10, 8));
        s.display(System.out);
    }
}
