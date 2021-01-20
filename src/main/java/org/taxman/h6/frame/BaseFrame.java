package org.taxman.h6.frame;

import org.taxman.h6.bombus.Hive;
import org.taxman.h6.util.TxSet;
import static org.taxman.h6.util.TxUnmodifiableSet.EmptySet;

import java.io.PrintStream;
import java.util.List;

public class BaseFrame {
    public final BaseFrame downstream;
    public final int myLevel;

    protected BaseFrame(int level) {
        this.myLevel = level;
        this.downstream = null;
    }

    public BaseFrame(BaseFrame downstream) {
        this.downstream = downstream;
        this.myLevel = downstream.myLevel - 1;
    }

    public BaseFrame addFrame(List<Hive> hives) {
        return new WorkingFrame(hives, this);
    }

    public void debugDump(PrintStream ps) {
        System.out.println();
    }

    boolean fitsInHive(TxSet promoteIntoHive, TxSet promoteOutOfHive) {
        return false;
    }

    public String getName() {
        return "(root)";
    }

    public int freeFactorCount() {
        return 0;
    }

    public TxSet factors() {
        return EmptySet;
    }

    public int estimateMaxPromotions(int upstreamOffer) {
        return 0;
    }

    public TxSet allCandidateNumbersIncludingDownstream() {
        return EmptySet;
    }

    public TxSet getImpossible() {
        return EmptySet;
    }

    public boolean fits(TxSet toPromote, TxSet allPromotions) {
        return true;
    }

    int fitsCallCountRecursive() {
        return 0;
    }
}