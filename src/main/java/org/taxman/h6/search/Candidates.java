package org.taxman.h6.search;

import org.taxman.h6.util.TxSet;

class Candidates {
    private final TxSet set;
    private final int[] descendingArray;

    public Candidates(TxSet set, int numToKeep) {
        this.set = set.smallest(numToKeep);
        this.descendingArray = this.set.descendingArray();
    }

    public int[] getDescendingArray() {
        return descendingArray;
    }

    public boolean contains(int n) {
        return set.contains(n);
    }

    public int sumOfLargest(int maxSize) {
        return this.set.sumOfLargest(maxSize);
    }

    public int nextHighest(int target) {
        return this.set.nextHighest(target);
    }

    public String toString() {
        return set.toString();
    }
}
