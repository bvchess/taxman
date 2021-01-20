package org.taxman.h6.util;

public class TxUnmodifiableSet extends TxSet {
    public static final TxSet EmptySet = TxSet.empty().unmodifiable();

    public static TxUnmodifiableSet of(TxSet other) {
        return new TxUnmodifiableSet(other);
    }

    private TxUnmodifiableSet(TxSet other) {
        super(TxSet.cloneBits(other.bits));
    }

    @Override
    public void append(int n) {
        throw new UnsupportedOperationException();
    }

    @Override
    public void remove(int n) {
        throw new UnsupportedOperationException();
    }
}
