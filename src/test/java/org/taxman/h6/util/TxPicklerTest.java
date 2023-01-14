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

import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.util.List;
import java.util.stream.Collectors;


public class TxPicklerTest {

    public static class IntegerDBL extends TxPickler<Integer> {
        public IntegerDBL(int bufferSize) {
            super(bufferSize);
        }

        @Override
        public void toStream(Integer i, DataOutputStream out) throws IOException {
            out.writeInt(i);
        }
        @Override
        public Integer fromStream(DataInputStream in) throws IOException {
            return in.readInt();
        }
    }


    @Test
    public void integerTest() {
        var dbl = new IntegerDBL(3);
        var list = List.of(1, 2, 3, 4, 5, 6, 7, 8, 9);
        dbl.addAll(list);
        List<Integer> readBack = dbl.stream().collect(Collectors.toList());
        assert list.equals(readBack);
    }

    @Test
    public void setTest() {
        var dbl = new TxSetPickler(2);
        var list = List.of(TxSet.of(1, 2, 3), TxSet.of(4, 5, 6), TxSet.of(7, 8, 9));
        dbl.addAll(list);
        var readBack = dbl.readAll();  // probably not enough time for anything to have gone to disk
        assert list.equals(readBack);
    }

    @Test
    public void waitForDiskWrite() throws InterruptedException {
        var dbl = new IntegerDBL(3);
        var list = List.of(1, 2, 3, 4, 5, 6, 7, 8, 9);
        dbl.addAll(list);
        Thread.sleep(100);  // need to yield to give time for the disk write
        assert dbl.sizeOnDisk() == 9;
        var readBack = dbl.stream().collect(Collectors.toList());
        assert list.equals(readBack);
    }

    @Test
    public void sizeTest() throws InterruptedException {
        var dbl = new IntegerDBL(3);
        var list = List.of(1, 2, 3, 4, 5, 6, 7, 8, 9);
        dbl.addAll(list);
        assert dbl.sizeOnDisk() == 0; // haven't had time to write to disk yet
        Thread.sleep(100);  // need to yield so write to disk will happen
        dbl.add(19);
        assert dbl.sizeOnDisk() == 9;
        assert dbl.size() == 10;
        var readBack = dbl.stream().collect(Collectors.toList());
        assert List.of(1, 2, 3, 4, 5, 6, 7, 8, 9, 19).equals(readBack) : "got back " + readBack;
    }
}