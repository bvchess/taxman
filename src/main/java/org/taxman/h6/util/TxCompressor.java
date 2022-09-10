package org.taxman.h6.util;

import java.util.BitSet;

public class TxCompressor {
    private int[] mapIn;
    private int[] mapOut;
    private int numMapped;

    public static final int DEFAULT_SIZE = 10;

    public TxCompressor() {
        this.mapIn = new int[DEFAULT_SIZE];
        this.mapOut = new int[DEFAULT_SIZE];
        this.numMapped = 0;
    }

    public TxSet compress(TxSet toCompress) {
        BitSet bits = new BitSet();
        toCompress.stream().map(this::compressNumber).forEach(bits::set);
        return TxSet.of(bits);
    }

    public TxSet decompress(TxSet toDecompress) {
        return TxSet.of(toDecompress.stream().map(this::decompressNumber));
    }

    private int compressNumber(int n) {
        if (n >= mapIn.length) {
            synchronized (this) {
                if (n >= mapIn.length) {
                    var newIn = new int[n * 2];
                    System.arraycopy(mapIn, 0, newIn, 0, mapIn.length);
                    mapIn = newIn;
                }
            }
        }
        if (mapIn[n] == 0) {
            synchronized (this) {
                if (mapIn[n] == 0) {
                    var mapTo = ++numMapped;
                    mapIn[n] = mapTo;
                    if (mapTo >= mapOut.length) {
                        var newOut = new int[mapTo * 2];
                        System.arraycopy(mapOut, 0, newOut, 0, mapOut.length);
                        mapOut = newOut;
                    }
                    mapOut[mapTo] = n;
                }
            }
        }
        //System.out.printf("%d compresses to %d\n", n, mapIn[n]);
        return mapIn[n];
    }

    private int decompressNumber(int x) {
        assert mapOut.length > x && mapOut[x] != 0 : "compressor hasn't mapped " + x;
        //System.out.printf("%d decompresses to %d\n", x, mapOut[x]);
        return mapOut[x];
    }
}