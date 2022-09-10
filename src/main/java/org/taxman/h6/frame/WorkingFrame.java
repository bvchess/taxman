package org.taxman.h6.frame;

import org.taxman.h6.bombus.Hive;
import org.taxman.h6.util.Memo;
import org.taxman.h6.util.TxSet;
import org.taxman.h6.util.TxUnmodifiableSet;

import java.io.PrintStream;
import java.util.List;
import java.util.function.Supplier;


public class WorkingFrame extends BaseFrame {
    final Hive hive;
    final TxSet allPromotionCandidateNumbers = TxSet.empty();
    private Memo fitsMemo;

    public WorkingFrame(List<Hive> hives, BaseFrame downstream) {
        super(downstream);
        hives.forEach(h -> {
            var candidates = TxSet.of(h.getPromotionCandidateNumbers());
            allPromotionCandidateNumbers.appendAll(candidates);
            //h.debugDump(System.out);  // prints out hives before they've been merged into frames
        });
        this.hive = (hives.size() == 1) ? hives.get(0) : hives.get(0).apiary.merge(hives);
    }

    @Override
    public void debugDump(PrintStream ps) {
        downstream.debugDump(ps);
        hive.debugDump(System.out);
    }

    @Override
    public String getName() {
        return hive.getName();
    }

    @Override
    public int freeFactorCount() {
        return hive.freeFactorCount();
    }

    @Override
    TxSet promotionCandidates() {
        return allPromotionCandidateNumbers;
    }

    @Override
    public int estimateMaxPromotions(int upstreamOffer) {
        var freedoms = freeFactorCount();
        var moves = Math.min(freedoms, upstreamOffer);
        return moves + downstream.estimateMaxPromotions(freedoms - moves);
    }

    @Override
    public TxSet allCandidateNumbersIncludingDownstream() {
        return TxSet.or(allPromotionCandidateNumbers, downstream.allCandidateNumbersIncludingDownstream());
    }

    @Override
    public List<TxSet> allCandidateNumbersByFrame() {
        var lst = downstream.allCandidateNumbersByFrame();
        lst.add(allPromotionCandidateNumbers);
        return lst;
    }

    @Override
    public BaseFrame addFrame(List<Hive> hives) {
        return new WorkingFrame(hives, this);
    }

    @Override
    public TxSet factors() {
        return hive.factors();
    }

    @Override
    public TxSet getImpossible() {
        return TxSet.or(hive.getImpossible(), downstream.getImpossible());
    }

    @Override
    public boolean fits(TxSet promoteIntoThisHive, TxSet allPromotions, TxUnmodifiableSet allRemovals) {
        return fancyFit(promoteIntoThisHive, allPromotions, allRemovals).success;
    }

    @Override
    public FrameTestResult fancyFit(TxSet promoteIntoThisHive, TxSet allPromotions, TxSet allRemovals) {
        var myPromotions = TxSet.and(factors(), allPromotions);
        var lump = TxSet.or(myPromotions, promoteIntoThisHive);
        var downstreamResult = downstream.fancyFit(myPromotions, allPromotions, allRemovals);
        if (downstreamResult.success) {
            Supplier<FrameTestResult> sftr = () -> hive.worksWithMods(promoteIntoThisHive, myPromotions, allRemovals);
            var localResult = fitsMemo.test(lump, sftr);
            if (localResult.success) {
                //return FrameTestResult.reportSuccess(TxSet.or(localResult.newRemovals, downstreamResult.newRemovals));
                return FrameTestResult.reportSuccess(null);
            }
        }
        return FrameTestResult.FAIL;
    }

    public List<BaseFrame> getAllFrames() {
        var frames = downstream.getAllFrames();
        frames.add(this);
        return frames;
    }

    @Override
    protected void finishFrameSetup(BaseFrame upstream) {
        super.finishFrameSetup(upstream);
        var vocabulary = promotionCandidates();
        if (upstream != null) {
            vocabulary.appendAll(upstream.promotionCandidates());
        }
        //System.out.printf("vocabulary size for %s is %d\n", getName(), vocabulary.size());
        fitsMemo = new Memo(this.hive.getName(), vocabulary);
    }

    @Override
    public String getCacheStats() {
        return downstream.getCacheStats() + fitsMemo.getCacheStats();
    }

    @Override
    public TxSet remainingSources() {
        return hive.remainingSources();
    }

    @Override
    public TxSet remainingFactors() {
        return hive.remainingFactors();
    }
}