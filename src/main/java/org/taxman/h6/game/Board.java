package org.taxman.h6.game;

import org.taxman.h6.util.TxSet;
import org.taxman.h6.util.TxUnmodifiableSet;

import java.io.PrintStream;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import java.util.stream.Collectors;
import java.util.stream.IntStream;

public class Board implements Comparable<Board> {
    public final TxUnmodifiableSet set;
    public final FactorTable fm;

    public static Board of(int highest) {
        FactorTable fm = new FactorTable(highest);
        TxUnmodifiableSet set = TxSet.of(IntStream.rangeClosed(1, highest)).unmodifiable();
        return new Board(set, fm);
    }

    public Board(TxSet set, FactorTable fm) {
        this.set = set.unmodifiable();
        this.fm = fm;
    }

    public TxSet factors(int n) {
        return TxSet.and(fm.getAbbreviatedFactors(n),set);
    }

    public TxSet allFactors(int n) {
        return TxSet.and(fm.getFactors(n),set);
    }

    public TxSet composites(int n) {
        return TxSet.and(fm.getAbbreviatedComposites(n),set);
    }

    public IntStream sources() {
        return set.filter(n -> composites(n).isEmpty());
    }

    public IntStream sinks() {
        return set.filter(n -> factors(n).isEmpty());
    }

    public int size() {
        return set.size();
    }

    public Board subtract(Board other) {
        return subtract(other.set);
    }

    public Board subtract(TxSet other) {
        return new Board(TxSet.subtract(set, other), fm);
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        Board board = (Board) o;
        return set.equals(board.set);
    }

    @Override
    public int hashCode() {
        return Objects.hash(set);
    }

    public String toString() {
        return "[" + set.toStringNoBrackets() + "]";
    }

    @Override
    public int compareTo(Board o) {
        return set.compareTo(o.set);
    }

    public Move makeMove(int n) throws VerificationException {
        return new Move(n, this);
    }

    public List<TxSet> makeLeveling() {
        List<TxSet> levels = new ArrayList<>();
        TxSet alreadyLeveled = TxSet.empty();
        TxSet thisLevel = TxSet.of(sinks());
        while (!thisLevel.isEmpty()) {
            levels.add(thisLevel);
            alreadyLeveled.appendAll(thisLevel);
            thisLevel = thisLevel.stream()
                    .mapToObj(this::composites)
                    .flatMapToInt(TxSet::stream)
                    .filter(n -> !alreadyLeveled.contains(n))
                    .collect(TxSet::empty, TxSet::append, TxSet::appendAll);
        }
        return levels;
    }

    public void printByLevel(PrintStream ps, String prefix) {
        TxSet[] levels = makeLeveling().toArray(new TxSet[0]);
        for (int i = 0; i < levels.length; i++) {
            String levelString = levels[i].stream()
                    .mapToObj(n -> n + ((factors(n).size() > 0) ? " -> " + factors(n) : ""))
                    .collect(Collectors.joining(", "));
            ps.println(prefix + (i+1) + ": " + levelString);
        }
    }
}
