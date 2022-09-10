package org.taxman.h6.util;

import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.util.BitSet;

public class TxSetPickler extends TxPickler<TxSet> {

    public TxSetPickler() {
        super();
    }

    public TxSetPickler(int bufferSize) {
        super(bufferSize);
    }

    @Override
    public void toStream(TxSet set, DataOutputStream out) throws IOException {
        out.writeShort(set.size());
        for (int i = set.max(); i > 0;  i = set.nextHighest(i)) {
            out.writeShort(i);
        }
    }

    @Override
    public TxSet fromStream(DataInputStream in) throws IOException {
        return new TxSet(readBitSet(in));
    }

    protected BitSet readBitSet(DataInputStream in) throws IOException {
        var bitCount = in.readShort();
        var bits = new BitSet();
        for (int i = 0; i < bitCount; i++) {
            bits.set(in.readShort(), true);
        }
        return bits;
    }
}
