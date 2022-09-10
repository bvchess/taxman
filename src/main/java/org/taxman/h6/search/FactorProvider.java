package org.taxman.h6.search;

import org.taxman.h6.game.Board;
import org.taxman.h6.util.TxSet;

public class FactorProvider {
    private final int[][] factorTable;
    private final int[][] compositeTable;

    private FactorProvider(int[][] factorTable, int[][] compositeTable) {
        this.factorTable = factorTable;
        this.compositeTable = compositeTable;
    }

    static FactorProvider create(Board b, TxSet allRemainingNumbers) {
        int max = allRemainingNumbers.max();
        int[][] factors = new int[max+1][];
        int[][] composites = new int[max+1][];
        allRemainingNumbers.stream().forEach(n -> {
            factors[n] = TxSet.and(b.factors(n), allRemainingNumbers).toArray();
            composites[n] = TxSet.and(b.composites(n), allRemainingNumbers).toArray();
        });
        return new FactorProvider(factors, composites);
    }

    public int[] getFactors(int n) {
        return factorTable[n];
    }

    public int[] getComposites(int n) {
        return compositeTable[n];
    }
}
