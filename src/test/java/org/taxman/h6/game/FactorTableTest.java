package org.taxman.h6.game;

import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.taxman.h6.game.FactorTable;
import org.taxman.h6.util.TxSet;

public class FactorTableTest {
    int[] compositesOf2to20 = {4, 6, 8, 10, 12, 14, 16, 18, 20};
    int[] primesTo100 = {2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97};

    @BeforeEach
    public void init() {

    }

    public void assertSetEquality(TxSet set, int... values) {
        assert(set.equals(TxSet.of(values)));
    }

    @Test
    public void empty() {
        new FactorTable(TxSet.empty());
    }

    @Test
    public void main() {
        String[] args = {"10"};
        FactorTable.main(args, false);
    }

    @Test
    public void primes() {
        FactorTable fm100 = new FactorTable(100);
        assertSetEquality(fm100.primes, primesTo100);
    }

    @Test
    public void factors() {
        FactorTable fm20 = new FactorTable(20);
        FactorTable fm100 = new FactorTable(100);
        assertSetEquality(fm20.getFactors(20), 1, 2, 4, 5, 10, 20);
        assertSetEquality(fm100.getFactors(83), 1, 83);
        Assertions.assertThrows(AssertionError.class, () -> fm100.getFactors(101));
    }

    @Test
    public void composites() {
        FactorTable fm20 = new FactorTable(20);
        FactorTable fm100 = new FactorTable(100);
        assertSetEquality(fm20.getComposites(2), compositesOf2to20);
        assert(fm20.getComposites(20).equals(TxSet.empty()));
        assertSetEquality(fm100.getComposites(50), 100);
    }

    @Test
    public void abbreviatedFactors() {
        FactorTable fm20 = new FactorTable(20);
        FactorTable fm100 = new FactorTable(100);
        assert(fm20.getAbbreviatedFactors(1).isEmpty());
        assertSetEquality(fm100.getAbbreviatedFactors(66), 6, 22, 33);
        assertSetEquality(fm100.getAbbreviatedFactors(50), 10, 25);
    }

    @Test
    public void abbreviatedComposites() {
        FactorTable fm100 = new FactorTable(100);
        assertSetEquality(fm100.getAbbreviatedComposites(49), 98);
        assertSetEquality(fm100.getAbbreviatedComposites(20), 40, 60, 100);
    }

    @Test
    public void selectNumbers() {
        FactorTable fm = new FactorTable(TxSet.of(2, 3, 4, 5, 30));
        //fm.printTables(System.out);
        assertSetEquality(fm.getAbbreviatedFactors(30), 2, 3, 5);
    }
}