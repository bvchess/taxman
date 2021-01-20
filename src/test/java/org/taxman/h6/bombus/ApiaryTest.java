package org.taxman.h6.bombus;

import org.junit.jupiter.api.Test;
import org.taxman.h6.game.Board;
import org.taxman.h6.game.FactorTable;
import org.taxman.h6.game.Solution;
import org.taxman.h6.util.TxList;
import org.taxman.h6.util.TxSet;

import java.util.ArrayList;
import java.util.List;
import java.util.stream.IntStream;

public class ApiaryTest {

    @Test
    void theMysteryOf528() {
        int n = 528;
        var b = Board.of(n);
        var promote = TxSet.of(128, 154, 168, 171, 174, 175, 180, 189, 195, 196, 198, 207, 208, 228, 230, 231, 234, 238, 240, 243, 244, 245, 248, 250, 252, 255, 258, 260, 261, 264);
        var a = new Apiary(b, promote, new Namer());
        var sln = new Solution(b, a.getSolution());
        sln.verify(n);
    }
}
