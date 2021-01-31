package org.taxman.h6.search;

import org.taxman.h6.util.TxSet;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.util.Objects;

class TaskData {
    public final int candidatesRef;
    public final TxSet base;
    public final int target;

    TaskData(int candidatesRef, TxSet base, int target) {
        assert candidatesRef <= Character.MAX_VALUE;
        assert target <= Character.MAX_VALUE;
        this.candidatesRef = candidatesRef;
        this.base = base;
        this.target = target;
    }

    public static TaskData fromByteArray(byte[] bytes) {
        ByteBuffer bb = ByteBuffer.wrap(bytes);
        return readFromBuffer(bb);
    }

    public static TaskData readFromBuffer(ByteBuffer bb) {
        int countOfNumbersToRead = bb.getChar();
        int candidatesRef = bb.getChar();
        int target = bb.getChar();
        int[] baseArr = new int[countOfNumbersToRead-3];
        for (int i = 0; i < baseArr.length; i++) baseArr[i] = bb.getChar();
        return new TaskData(candidatesRef, TxSet.of(baseArr), target);
    }

    public byte[] toByteArray() {
        int countOfNumbersToWrite = 3 + base.size();
        ByteBuffer bb = ByteBuffer.allocate(countOfNumbersToWrite*2);
        bb.putChar((char) countOfNumbersToWrite);
        bb.putChar((char) candidatesRef);
        bb.putChar((char) target);
        for (int n: base.toArray()) bb.putChar((char) n);
        return bb.array();
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (!(o instanceof TaskData)) return false;
        TaskData taskData = (TaskData) o;
        return candidatesRef == taskData.candidatesRef && target == taskData.target && base.equals(taskData.base);
    }

    @Override
    public int hashCode() {
        return Objects.hash(candidatesRef, base, target);
    }
}
