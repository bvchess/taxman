package org.taxman.h6.frame;

import org.taxman.h6.bombus.Hive;
import org.taxman.h6.util.TxSet;
import static org.taxman.h6.util.TxUnmodifiableSet.EmptySet;

import java.io.PrintStream;
import java.util.List;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicInteger;


public class WorkingFrame extends BaseFrame {
    final Hive hive;
    final TxSet allPromotionCandidateNumbers = TxSet.empty();
    private final ConcurrentHashMap<TxSet, Boolean> fitsMemo = new ConcurrentHashMap<>();
    private final AtomicInteger fitsCallCount = new AtomicInteger(0);


    public WorkingFrame(List<Hive> hives, BaseFrame downstream) {
        super(downstream);
        hives.forEach(h -> {
            var candidates = TxSet.of(h.getPromotionCandidateNumbers());
            allPromotionCandidateNumbers.appendAll(candidates);
        });
        this.hive = (hives.size() == 1) ? hives.get(0) : hives.get(0).apiary.merge(hives);
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
    public boolean fits(TxSet toPromote, TxSet allPromotions) {
        var myPromotions = TxSet.and(factors(), allPromotions);
        var allRemaining = TxSet.subtract(allPromotions, toPromote);
        var lump = TxSet.or(toPromote, myPromotions);
        var result = fitsMemo.getOrDefault(lump, null);
        if (result == null) {
            result = fitsInHive(toPromote, myPromotions);
            fitsMemo.put(lump, result);
        }
        return result && downstream.fits(myPromotions, allRemaining);
    }
}