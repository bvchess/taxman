package org.taxman.h6.util;

public class TxUnmodifiableList extends TxList {

    public static TxUnmodifiableList of(TxList other) {
        return new TxUnmodifiableList(other);
    }

    private TxUnmodifiableList(TxList other) {
        super(other.toArray(), other.size());
    }

    @Override
    public void append(int n) {
        throw new UnsupportedOperationException();
    }

    @Override
    public void append(TxCollection c) {
        throw new UnsupportedOperationException();
    }
}
