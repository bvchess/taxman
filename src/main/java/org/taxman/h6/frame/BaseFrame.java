package org.taxman.h6.frame;

import org.taxman.h6.bombus.Hive;
import org.taxman.h6.util.TxSet;
import org.taxman.h6.util.TxUnmodifiableSet;

import static org.taxman.h6.util.TxUnmodifiableSet.EmptySet;

import java.io.PrintStream;
import java.util.ArrayList;
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

    public List<TxSet> allCandidateNumbersByFrame() {
        return new ArrayList<>();
    }

    public TxSet getImpossible() {
        return EmptySet;
    }

    public boolean fits(TxSet toPromote, TxSet allPromotions, TxUnmodifiableSet allRemovals) {
        return true;
    }

    public FrameTestResult fancyFit(TxSet toPromote, TxSet allPromotions, TxSet allRemovals) {
        return FrameTestResult.reportSuccess(EmptySet);
    }

    public String getCacheStats() {
        return "";
    }

    public List<BaseFrame> getAllFrames() {
        return new ArrayList<>();
    }

    public void finishFrameSetup() {
        finishFrameSetup(null);
    }

    protected void finishFrameSetup(BaseFrame upstream) {
        if (downstream != null) downstream.finishFrameSetup(this);
    }

    TxSet promotionCandidates() {
        return EmptySet;
    }

    public TxSet remainingSources() {
        return TxSet.empty();
    }

    public TxSet remainingFactors() {
        return TxSet.empty();
    }
}