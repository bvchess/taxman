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