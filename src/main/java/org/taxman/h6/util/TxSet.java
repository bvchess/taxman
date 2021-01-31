package org.taxman.h6.util;

import java.util.Arrays;
import java.util.BitSet;
import java.util.Collection;
import java.util.Comparator;
import java.util.stream.Collector;
import java.util.stream.Collectors;
import java.util.stream.IntStream;
import java.util.stream.Stream;

public class TxSet extends TxCollection {
    public static final int MAX_VALUE = 1023;

    final BitSet bits;

    static BitSet cloneBits(BitSet bits) {
        return (BitSet) bits.clone();
    }

    public static TxSet of(TxCollection c) {
        return TxSet.of(c.stream());
    }

    public static TxSet of(TxSet s) {
        return new TxSet(cloneBits(s.bits));
    }

    public static TxSet of(IntStream stream) {
        return TxSet.of(stream.toArray());
    }

    public static TxSet of(int... arr) {
        int max = 0;
        for (int i : arr) max = Math.max(i, max);
        BitSet bits = new BitSet(max+1);
        for (int i : arr) {
            assert i > 0 && i <= MAX_VALUE : "cannot insert " + i + " with max value of " + MAX_VALUE;
            bits.set(i);
        }
        return new TxSet(bits);
    }

    public static TxSet of(Stream<TxSet> s) {
        return TxSet.or(s.toArray(TxSet[]::new));
    }

    public static TxSet of(Collection<Integer> c) {
        return TxSet.of(c.stream().mapToInt(Integer::intValue));
    }

    public static TxSet of(BitSet bits) {
        return new TxSet(cloneBits(bits));
    }

    public static TxSet empty() {
        return new TxSet(new BitSet(0));
    }

    TxSet(BitSet b) {
        bits = b;
    }

    @Override
    public int[] toArray() {
        int sz = size();
        var result = new int[sz];
        int prev = 0;
        for (int i=0; i < sz; i++) {
            prev = bits.nextSetBit(prev+1);
            result[i] = prev;
        }
        return result;
    }

    public int[] descendingArray() {
        var arr = new int[size()];
        int i=0;
        for (int j = bits.length(); (j = bits.previousSetBit(j-1)) >= 0; ) {
            arr[i++] = j;
        }
        return arr;
    }

    @Override
    public IntStream streamDescending() {
        return Arrays.stream(descendingArray());
    }

    @Override
    public int sum() {
        int result = 0;
        for (int i = bits.nextSetBit(1); i >= 0; i = bits.nextSetBit(i+1))
            result += i;
        return result;
    }

    @Override
    public IntStream stream() {
        return bits.stream();
    }

    @Override
    public int size() {
        return bits.cardinality();
    }

    @Override
    public int max() {
        return bits.previousSetBit(bits.length()-1);
    }

    @Override
    public int min() {
        return bits.nextSetBit(0);
    }

    @Override
    public boolean contains(int n) {
        return bits.get(n);
    }

    public boolean contains(TxSet other) {
        return TxSet.subtract(other, this).isEmpty();
    }

    public static TxSet and(TxSet first, TxSet second) {
        BitSet newBits = cloneBits(first.bits);
        newBits.and(second.bits);
        return new TxSet(newBits);
    }

    public static TxSet and(TxSet first, int n) {
        if (first.bits.get(n)) {
            return TxSet.of(n);
        } else {
            return TxSet.empty();
        }
    }

    public static TxSet or(TxSet... sets) {
        TxSet result = TxSet.empty();
        for (TxSet s: sets) {
            result.bits.or(s.bits);
        }
        return result;
    }

    public static TxSet or(TxSet first, int n) {
        BitSet newBits = cloneBits(first.bits);
        newBits.set(n);
        return new TxSet(newBits);
    }

    public static TxSet subtract(TxSet first, TxSet second) {
        BitSet newBits = cloneBits(first.bits);
        newBits.andNot(second.bits);
        return new TxSet(newBits);
    }

    public static TxSet subtract(TxSet first, int n) {
        BitSet newBits = cloneBits(first.bits);
        newBits.set(n, false);
        return new TxSet(newBits);
    }

    public static Comparator<TxSet> BySumDescending() {
        return Comparator.comparing(TxSet::sum).reversed();
    }

    @Override
    public void append(int n) {
        assert n >= 0 && n <= MAX_VALUE;
        bits.set(n);
    }

    @Override
    public int compareTo(TxCollection o) {
        return toString().compareTo(o.toString());
    }

    public boolean isEmpty() {
        return bits.isEmpty();
    }

    public TxUnmodifiableSet unmodifiable() {
        return TxUnmodifiableSet.of(this);
    }

    @Override
    public String toString() {
        return toString("{", "}");
    }

    public String toString(String start, String end) {
        String meat = stream()
                .mapToObj(Integer::toString)
                .collect(Collectors.joining(", "));
        return start + meat + end;
    }

    public String toStringNoBrackets() {
        return toString("", "");
    }

    public boolean equals(Object obj) {
        if (obj instanceof TxSet) {
            var other = (TxSet) obj;
            return this == other || bits.equals(other.bits);
        } else {
            return false;
        }
    }

    public int hashCode() {
        return bits.hashCode();
    }

    @Override
    public IntStream sorted() {
        return stream();
    }

    public static Collector<TxCollection, TxSet, TxSet> collector() {
        return Collector.of(TxSet::empty,
                TxSet::appendAll,
                TxSet::or,
                Collector.Characteristics.IDENTITY_FINISH);
    }

    public TxSet largest(int sz) {
        if (sz >= size()) return TxSet.of(this);
        int j = bits.length();
        var newBits = new BitSet(j);
        for (int i = 0; i < sz; i++) {
            j = bits.previousSetBit(j-1);
            newBits.set(j);
        }
        return new TxSet(newBits);
    }

    public TxSet smallest(int sz) {
        int fullSize = size();
        if (sz >= fullSize) return TxSet.of(this);
        int j = 0;
        var newBits = new BitSet(fullSize);
        for (int i=0; i < sz; i++) {
            j = bits.nextSetBit(j+1);
            newBits.set(j);
        }
        return new TxSet(newBits);
    }

    public int sumOfLargest(int sz) {
        if (sz >= size()) return sum();
        int j = bits.length();
        int result = 0;
        for (int i = 0; i < sz; i++) {
            j = bits.previousSetBit(j-1);
            result += j;
        }
        return result;
    }

    public void remove(int n) {
        assert n >= 0 && n <= MAX_VALUE;
        bits.set(n, false);
    }

    public int nextHighest(int n) {
        int result = bits.previousSetBit(n-1);
        assert result < n;
        return result;
    }
}
