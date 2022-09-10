package org.taxman.h6.util;

import java.io.DataInputStream;
import java.io.IOException;

public class TxUnmodifiableSetPickler extends TxSetPickler {

    public TxUnmodifiableSetPickler() {
        super();
    }

    public TxUnmodifiableSetPickler(int bufferSize) {
        super(bufferSize);
    }

    @Override
    public TxUnmodifiableSet fromStream(DataInputStream in) throws IOException {
        return new TxUnmodifiableSet(readBitSet(in));
    }
}