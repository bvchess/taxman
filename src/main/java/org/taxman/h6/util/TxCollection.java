package org.taxman.h6.util;

import java.util.PrimitiveIterator;
import java.util.function.IntConsumer;
import java.util.function.IntPredicate;
import java.util.function.IntUnaryOperator;
import java.util.stream.IntStream;

public abstract class TxCollection implements Comparable<TxCollection> {

    public IntStream sorted() {
        return stream().sorted();
    }

    public IntStream streamDescending() {
        int[] arr = sorted().toArray();
        return IntStream.rangeClosed(1, arr.length)
                .map(i -> arr[arr.length - i]);
    }

    public PrimitiveIterator.OfInt iterator() {
        return stream().iterator();
    }

    public int[] toArray() {
        return stream().toArray();
    }

    public void appendAll(TxCollection c) {
        c.forEach(this::append);
    }

    public void appendAll(int... c) {
        for (int i: c)
            append(i);
    }

    public void appendAll(IntStream s) {
        s.forEach(this::append);
    }

    public void forEach(IntConsumer func) {
        stream().forEach(func);
    }

    public IntStream map(IntUnaryOperator func) {
        return stream().map(func);
    }

    public IntStream filter(IntPredicate p) {
        return stream().filter(p);
    }

    public int sum() {
        return stream().sum();
    }

    public abstract IntStream stream();

    public abstract int size();

    public abstract int max();

    public abstract int min();

    public abstract boolean contains(int n);

    public abstract void append(int n);

}

