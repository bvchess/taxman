package org.taxman.h6.frame;

import org.taxman.h6.util.TxSet;
import org.taxman.h6.util.TxUnmodifiableSet;

public class FrameTestInput {
    public final TxUnmodifiableSet promotions;
    public final TxUnmodifiableSet removals;

    public FrameTestInput(TxUnmodifiableSet promotions, TxUnmodifiableSet removals) {
        this.promotions = promotions;
        this.removals = removals;
    }

    public FrameTestInput(TxSet promotions, TxSet removals) {
        this(promotions.unmodifiable(), removals.unmodifiable());
    }
}
