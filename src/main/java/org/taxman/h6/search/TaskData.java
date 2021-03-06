package org.taxman.h6.search;

import org.taxman.h6.util.TxSet;

import java.io.*;
import java.nio.ByteBuffer;
import java.util.Objects;

class TaskData {
    public final TxSet candidates;
    public final TxSet base;
    public final int target;

    TaskData(TxSet candidates, TxSet base, int target) {
        assert target <= Character.MAX_VALUE;
        this.candidates = candidates;
        this.base = base;
        this.target = target;
    }

    public static TaskData fromByteArray(byte[] bytes) throws IOException {
        return read(new ByteArrayInputStream(bytes));
    }

    private static int readShort(InputStream is) throws IOException {
        int first = is.read();
        if (first == -1) return -1;
        int second = is.read();
        return (first << 8) + second;
    }

    public static TaskData read(InputStream is) throws IOException {

        int target = readShort(is);
        if (target == -1) return null;

        int candidatesSize = readShort(is);
        int[] candidatesArr = new int[candidatesSize];
        for (int i = 0; i < candidatesArr.length; i++) candidatesArr[i] = readShort(is);

        int baseSize = readShort(is);
        int[] baseArr = new int[baseSize];
        for (int i = 0; i < baseArr.length; i++) baseArr[i] = readShort(is);

        return new TaskData(TxSet.of(candidatesArr), TxSet.of(baseArr), target);
    }

    public byte[] toByteArray() {
        int countOfNumbersToWrite = 3 + base.size() + candidates.size();
        ByteBuffer bb = ByteBuffer.allocate(countOfNumbersToWrite*2);

        bb.putChar((char) target);

        var candidatesArray = candidates.descendingArray();
        bb.putChar((char) candidatesArray.length);
        for (int n: candidatesArray) bb.putChar((char) n);

        var baseArray = base.descendingArray();
        bb.putChar((char) baseArray.length);
        for (int n: baseArray) bb.putChar((char) n);

        return bb.array();
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (!(o instanceof TaskData)) return false;
        TaskData taskData = (TaskData) o;
        return target == taskData.target && base.equals(taskData.base) && candidates.equals(taskData.candidates);
    }

    @Override
    public int hashCode() {
        return Objects.hash(candidates, base, target);
    }
}
