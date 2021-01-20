package org.taxman.h6.bombus;

import org.taxman.h6.game.Board;
import org.taxman.h6.game.Solution;
import org.taxman.h6.util.TxList;
import org.taxman.h6.util.TxSet;

public class BombusSolution extends Solution {
    public final TxSet promotions;

    public BombusSolution(Board b, TxList m, TxSet promotions) {
        super(b, m);
        this.promotions = promotions;
    }

    public static BombusSolution upgrade(Solution sln) {
        var moves = TxList.of(sln.moves);
        var a = new Apiary(sln.board, new Namer());
        var candidates = a.getPromotionCandidateNumbers();
        var promotions = TxSet.and(candidates, TxSet.of(moves));
        return new BombusSolution(sln.board, moves, promotions);
    }
}
