package org.taxman.h6.util;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.util.Arrays;
import java.util.Hashtable;
import java.util.stream.IntStream;

public class TxListTest {
    private final static int[] threeFourFiveArray = {3, 4, 5};
    private TxList oneTwo;
    private TxList oneTwoThree;
    private TxList threeTwoOne;

    @BeforeEach
    public void init() {
        oneTwo = TxList.of(1, 2);
        oneTwoThree = TxList.of(1, 2, 3);
        threeTwoOne = TxList.of(3, 2, 1);
    }

    @Test
    public void create() {
        assert(TxList.of(1).length() == 1);
        assert(TxList.of(1, 2, 3).length() == 3);
        assert(TxList.of(threeFourFiveArray).length() == 3);
        assert(TxList.of().length() == 0);
        assert(TxList.of(IntStream.range(1, 10)).length() == 9);
        TxList copy = TxList.of(threeTwoOne);
        assert(copy.equals(threeTwoOne));
    }

    @Test
    public void append() {
        TxList list =  TxList.of(1, 2, 3);
        list.append(TxList.of(4, 5, 6));
        assert(list.sum() == 21);
    }

    @Test
    public void concat() {
        TxList list =  TxList.concat(TxList.of(1, 2, 3), TxList.of(4, 5, 6));
        assert(list.sum() == 21);
        assert(TxList.concat(list, 9).sum() == 30);
        assert(TxList.concat(list, threeFourFiveArray).sum() == 33);
        assert(TxList.concat(TxList.of(IntStream.range(1, 5)), IntStream.range(5, 10)).sum() == 45);
    }

    @Test
    public void reverse() {
        assert(oneTwoThree.equals(TxList.of(threeTwoOne.reverse())));
    }

    @Test
    public void toArray() {
        assert(Arrays.equals(oneTwoThree.toArray(), new int[] {1, 2, 3}));
    }

    @Test
    public void hash() {
        Hashtable<TxList, String> hashtable = new Hashtable<>();
        hashtable.put(oneTwoThree, "1 2 3");
        hashtable.put(threeTwoOne, "3 2 1");
        assert(hashtable.get(oneTwoThree).equals("1 2 3"));
    }

    @Test
    public void pop() {
        TxList list = TxList.of(oneTwoThree);
        int popVal = list.pop();
        assert(popVal == 3);
        assert(list.equals(oneTwo));
    }

    @Test
    public void peek() {
        assert oneTwoThree.peek() == 3;
        int popped = oneTwoThree.pop();
        assert popped == 3;
        assert oneTwoThree.peek() == 2;
    }
}