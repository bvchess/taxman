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

import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.util.Arrays;
import java.util.stream.IntStream;

public class TxSetTest {

    private final int[] threeFourFiveArray = {3, 4, 5};
    private final int[] oneThroughFiveArray = {1, 2, 3, 4, 5};
    private TxSet oneTwoSet;
    private TxSet oneTwoThreeSet;
    private TxSet threeFourFiveSet;
    private TxList ninetyNineOneHundred;

    @BeforeEach
    public void init() {
        oneTwoSet = TxSet.of(1, 2);
        oneTwoThreeSet = TxSet.of(1, 2, 3);
        threeFourFiveSet = TxSet.of(3, 4, 5);
        ninetyNineOneHundred = TxList.of(99, 100);
    }

    @Test
    public void create() {
        TxSet.of(1);
        TxSet.of(1, 2, 3);
        TxSet.of(threeFourFiveArray);
        TxSet.empty();
        TxSet.of(ninetyNineOneHundred);
        TxSet.of(IntStream.range(1, 10));
        Assertions.assertThrows(AssertionError.class, () -> TxSet.of(0));
    }

    @Test
    public void add() {
        TxSet set = TxSet.of(17, 19);
        set.append(21);
        set.append(29);
        assert (set.equals(TxSet.of(29, 21, 19, 17)));
    }

    @Test
    public void subtract() {
        assert (TxSet.subtract(oneTwoThreeSet, threeFourFiveSet).equals(oneTwoSet));
        assert (TxSet.subtract(oneTwoThreeSet, 3).equals(oneTwoSet));
    }

    @Test
    public void members() {
        oneTwoThreeSet.appendAll(threeFourFiveSet);
        assert (Arrays.equals(oneTwoThreeSet.toArray(), oneThroughFiveArray));
    }

    @Test
    public void contains() {
        TxSet set = TxSet.of(1);
        set.append(2);
        set.append(10);
        assert (set.contains(2));
        assert (TxSet.of(ninetyNineOneHundred).contains(99));
        assert (!TxSet.of(1).contains(101));
    }

    @Test
    public void cardinality() {
        assert (oneTwoSet.size() == 2);
        assert (oneTwoThreeSet.size() == 3);
        assert (TxSet.subtract(oneTwoSet, oneTwoThreeSet).isEmpty());
    }

    @Test
    public void iterate() {
        int sum = 0;
        for (int i : oneTwoThreeSet.toArray()) sum += i;
        assert (sum == 6);
    }

    @Test
    public void stream() {
        TxSet set = TxSet.empty();
        set.append(5);
        set.append(10);
        set.appendAll(TxList.of(15, 20));
        assert (set.sum() == 50);
    }

    @Test
    public void and() {
        assert (TxSet.and(oneTwoThreeSet, oneTwoSet).equals(oneTwoSet));
    }

    @Test
    public void unmodifiable() {
        assert (oneTwoThreeSet.unmodifiable().equals(oneTwoThreeSet));
    }

    @Test
    public void unmodifiable2() {
        assert (TxSet.or(TxSet.of(1, 2), 3).unmodifiable().equals(oneTwoThreeSet));
    }

    @Test
    public void largest1() {
        var lg = TxSet.of(1, 2, 5, 7).largest(2);
        var expected = TxSet.of(5, 7);
        assert lg.equals(expected) : "oops, got " + lg + " rather than " + expected;
    }

    @Test
    void testReadWrite() throws IOException {
        var x = TxSet.of(2, 3, 5, 7, 9, 11, 13);
        var bytes = x.toBytes();
        assert bytes.length == x.size() * 2 + 2;
        var y = TxSet.read(new ByteArrayInputStream(bytes));
        assert x.equals(y);
        var z = TxUnmodifiableSet.read(new ByteArrayInputStream(bytes));
        assert x.equals(z);
    }

    @Test
    void testBigArray() {
        var arr = new byte[1000];
        var x = TxSet.of(80, 11, 9, 7, 6, 5, 3);
        var positions = new int[3];
        var head = 0;
        for (int i = 0; i < positions.length; i++) {
            var bytes = x.toBytes();
            System.arraycopy(bytes, 0, arr, head, bytes.length);
            positions[i] = head;
            head += bytes.length;
        }
        for (int position : positions) {
            var y = TxSet.readFromBigArray(arr, position);
            assert x.equals(y);
        }
    }

    @Test
    void combos() {
        var set = TxSet.of(1, 2, 3, 4, 5);
        assert TxSet.combinations(set, 2).count() == 10;
        assert TxSet.combinationsUpToSize(set, 2).count() == 15;
    }
}