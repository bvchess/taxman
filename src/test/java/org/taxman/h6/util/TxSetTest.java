package org.taxman.h6.util;

import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

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
        assert(set.equals(TxSet.of(29, 21, 19, 17)));
    }

    @Test
    public void subtract() {
        assert(TxSet.subtract(oneTwoThreeSet, threeFourFiveSet).equals(oneTwoSet));
        assert(TxSet.subtract(oneTwoThreeSet, 3).equals(oneTwoSet));
    }

    @Test
    public void members() {
        oneTwoThreeSet.appendAll(threeFourFiveSet);
        assert(Arrays.equals(oneTwoThreeSet.toArray(), oneThroughFiveArray));
    }

    @Test
    public void contains() {
        TxSet set = TxSet.of(1);
        set.append(2);
        set.append(10);
        assert(set.contains(2));
        assert(TxSet.of(ninetyNineOneHundred).contains(99));
        assert(!TxSet.of(1).contains(101));
    }

    @Test
    public void cardinality() {
        assert(oneTwoSet.size() == 2);
        assert(oneTwoThreeSet.size() == 3);
        assert(TxSet.subtract(oneTwoSet, oneTwoThreeSet).isEmpty());
    }

    @Test
    public void iterate() {
        int sum = 0;
        for (int i: oneTwoThreeSet.toArray()) sum += i;
        assert(sum == 6);
    }

    @Test
    public void stream() {
        TxSet set = TxSet.empty();
        set.append(5);
        set.append(10);
        set.appendAll(TxList.of(15, 20));
        assert(set.sum() == 50);
    }

    @Test
    public void and() {
        assert(TxSet.and(oneTwoThreeSet, oneTwoSet).equals(oneTwoSet));
    }

    @Test
    public void unmodifiable() {
        assert(oneTwoThreeSet.unmodifiable().equals(oneTwoThreeSet));
    }

    @Test
    public void unmodifiable2() {
        assert(TxSet.or(TxSet.of(1, 2), 3).unmodifiable().equals(oneTwoThreeSet));
    }

    @Test
    public void largest1() {
        var lg = TxSet.of(1, 2, 5, 7).largest(2);
        var expected = TxSet.of(5, 7);
        assert lg.equals(expected) : "oops, got " + lg + " rather than " + expected;
    }
}