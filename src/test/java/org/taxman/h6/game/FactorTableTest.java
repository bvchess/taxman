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

package org.taxman.h6.game;

import org.junit.jupiter.api.Assertions;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.taxman.h6.util.TxList;
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

    @Test
    public void primeFactors() {
        FactorTable fm100 = new FactorTable(100);
        assert TxList.of(2, 2, 2, 2).equals(fm100.primeFactors[16]);
        assert TxList.of(5, 5, 2, 2).equals(fm100.primeFactors[100]);
        assert TxList.of(5, 3, 2).equals(fm100.primeFactors[30]);
    }
}