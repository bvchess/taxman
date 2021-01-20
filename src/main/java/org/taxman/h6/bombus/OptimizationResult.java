package org.taxman.h6.bombus;

import org.taxman.h6.util.TxSet;

public class OptimizationResult {
    public final TxSet promoteThese;
    public final int score;

    public OptimizationResult(TxSet promoteThese, int score) {
        this.promoteThese = promoteThese.unmodifiable();
        this.score = score;
    }

    public OptimizationResult(Apiary a) {
        this.promoteThese = a.treatAsSources;
        this.score = a.sumOfSources();
    }

    public String toString() {
        return score + " with " + promoteThese;
    }
}
