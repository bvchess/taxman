package org.taxman.h6.frame;

import org.taxman.h6.bombus.Hive;
import org.taxman.h6.util.Memo;
import org.taxman.h6.util.TxSet;

import java.io.PrintStream;
import java.util.List;
import java.util.Map;
import java.util.concurrent.atomic.AtomicInteger;


public class WorkingFrame extends BaseFrame {
    final Hive hive;
    final TxSet allPromotionCandidateNumbers = TxSet.empty();
    //private final ConcurrentHashMap<TxSet, Boolean> fitsMemo = new ConcurrentHashMap<>();
    //private final Map<TxSet, Boolean> fitsMemo = LRUCache.make(1000000);
    private final Memo fitsMemo;

    private final AtomicInteger fitsCallCount = new AtomicInteger(0);


    public WorkingFrame(List<Hive> hives, BaseFrame downstream) {
        super(downstream);
        hives.forEach(h -> {
            var candidates = TxSet.of(h.getPromotionCandidateNumbers());
            allPromotionCandidateNumbers.appendAll(candidates);
        });
        this.hive = (hives.size() == 1) ? hives.get(0) : hives.get(0).apiary.merge(hives);
        fitsMemo = new Memo(this.hive.getName());
    }

    @Override
    boolean fitsInHive(TxSet promoteIntoHive, TxSet promoteOutOfHive) {
        fitsCallCount.incrementAndGet();
        return hive.worksWithMods(promoteIntoHive, promoteOutOfHive);
    }

    @Override
    int fitsCallCountRecursive() {
        return fitsCallCount.get() + downstream.fitsCallCountRecursive();
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
    public boolean fits(TxSet promoteIntoThisHive, TxSet allPromotions) {
        var myPromotions = TxSet.and(factors(), allPromotions);
        var lump = TxSet.or(promoteIntoThisHive, myPromotions);
        var fitsInThisHive = fitsMemo.test(lump, () -> fitsInHive(promoteIntoThisHive, myPromotions));
        return fitsInThisHive && downstream.fits(myPromotions, allPromotions);
    }
}