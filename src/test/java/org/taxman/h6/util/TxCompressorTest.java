package org.taxman.h6.util;

import org.junit.jupiter.api.Test;

import java.util.stream.IntStream;

public class TxCompressorTest {


    @Test
    public void testBasics() {
        TxSet vocab = TxSet.of(10, 20, 30, 40, 50, 60);
        TxCompressor compressor = new TxCompressor();
        compressor.compress(vocab);
        var input = TxSet.of(20, 40, 60);
        var compressed = compressor.compress(input);
        assert TxSet.of(2, 4, 6).equals(compressed);
        var decompressed = compressor.decompress(compressed);
        assert input.equals(decompressed);
    }

    @Test
    public void testExtremes() {
        TxSet vocab = TxSet.of(10, 20, 30, 40, 50, 60);
        TxCompressor compressor = new TxCompressor();
        var compressed = compressor.compress(vocab);
        assert TxSet.of(IntStream.rangeClosed(1, 6)).equals(compressed);
        var decompressed = compressor.decompress(compressed);
        assert vocab.equals(decompressed);
    }

    @Test
    public void testBadDecompress() {
        TxCompressor compressor = new TxCompressor();
        var exceptionCaught = false;
        try {
            compressor.decompress(TxSet.of(15));
        } catch (AssertionError e) {
            exceptionCaught = true;
        }
        assert exceptionCaught;
    }
}
