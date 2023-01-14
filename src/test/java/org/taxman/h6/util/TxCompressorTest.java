/*
 * Copyright (c) Brian Chess 2019-2023.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of
 * this software and associated documentation files (the "Software"), to deal in
 * the Software without restriction, including without limitation the rights to
 * use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
 * of the Software, and to permit persons to whom the Software is furnished to do
 * so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

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
