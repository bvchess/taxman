package org.taxman.h6.search;

import org.taxman.h6.util.TxSet;
import org.taxman.h6.util.TxUnmodifiableSet;

import java.util.Arrays;


public class Branch {
    public final FrameSet gameState;
    private final long[] alreadyPromoted;
    private final long[] candidates;


    public Branch(FrameSet gameState, TxSet alreadyPromoted, TxSet candidates) {
        this.gameState = gameState;
        this.alreadyPromoted = alreadyPromoted.bits.toLongArray();
        this.candidates = candidates.bits.toLongArray();
    }

    public String toString() {
        var prom = getAlreadyPromoted();
        var cand = getCandidates();
        return String.format("%,d=%s with %d remaining selections and %d candidates %s", getAlreadyPromoted().sum(),
                prom, gameState.computeMaxNumberOfPromotions(cand),  cand.size(), cand);
    }

    public boolean equals(Object obj) {
        if (!(obj instanceof Branch)) return false;
        if (this == obj) return true;
        var other = (Branch) obj;
        return gameState.equals(other.gameState) && Arrays.equals(alreadyPromoted, other.alreadyPromoted)
                && Arrays.equals(candidates, other.candidates);
    }

    public TxUnmodifiableSet getAlreadyPromoted() {
        return TxUnmodifiableSet.of(alreadyPromoted);
    }

    public TxUnmodifiableSet getCandidates() {
        return TxUnmodifiableSet.of(candidates);
    }

}