package org.taxman.h6.frame;

import org.taxman.h6.util.TxSet;
import static org.taxman.h6.util.TxUnmodifiableSet.EmptySet;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.Predicate;
import java.util.stream.IntStream;
import java.util.stream.Stream;


public class Greedy {
    private final TxSet allNumbers;
    private final Predicate<TxSet> predicate;
    private final int maxHammingDistance;
    private final ConcurrentHashMap<Integer, ConcurrentHashMap<TxSet, TxSet>> gMemo;

    public Greedy(TxSet allNumbers, Predicate<TxSet> predicate, int maxHammingDistance) {
        this.allNumbers = allNumbers;
        this.predicate = predicate;
        this.maxHammingDistance = maxHammingDistance;
        this.gMemo = new ConcurrentHashMap<>();
    }

    public static TxSet find(TxSet candidates, int maxPromotions, Predicate<TxSet> predicate) {
        return new Greedy(candidates, predicate, 2).search();
    }

    private TxSet baseCase(TxSet numbers) {
        var result = TxSet.empty();
        for (int n: numbers.descendingArray()) {
            TxSet newTry = TxSet.or(result, n);
            if (predicate.test(newTry)) result = newTry;
        }
        return result;
    }

    private TxSet search() {
        var best = baseCase(allNumbers);
        System.out.printf("  greedy search initial: %d=%s from %s\n", best.sum(), best, allNumbers);

        while (true) {
            var result = jiggle(best);
            if (result.sum() <= best.sum()) break;
            var dropped = TxSet.subtract(best, result);
            var added = TxSet.subtract(result, best);
            System.out.printf("         greedy upgrade: %d = %s: added %s, dropped %s\n",
                    result.sum(), result, added, dropped);
            best = result;
        }

        return best;
    }

    private TxSet jiggle(TxSet base) {
        return combinationsUpToSize(base, maxHammingDistance)
                .map(blacklist -> fill(base, blacklist))
                .reduce(base, (best, current) -> (best.sum() > current.sum() ? best : current));
    }


    private TxSet fill(TxSet base, TxSet blacklist) {

        /*
        var numbers = TxSet.subtract(allNumbers, TxSet.or(base, blacklist));
        var nakedBase = TxSet.subtract(base, blacklist);

        System.out.printf("    filling %s from %s without %s\n", base, numbers, blacklist);
        var bestSub = combinations(numbers, blacklist.size())
                .map(s -> TxSet.or(nakedBase, s))
                .filter(predicate)
                .peek(s -> System.out.println("      ! " + s))
                .reduce(blacklist, (best, current) -> (best.sum() > current.sum() ? best : current));

        TxSet result;
        if (bestSub.sum() > blacklist.sum()) {
            result = TxSet.or(TxSet.subtract(base, blacklist), bestSub);
        } else {
            result = base;
        }
        return result;

         */

        var numbers = TxSet.subtract(allNumbers, TxSet.or(base, blacklist));
        var result = TxSet.subtract(base, blacklist);
        for (int n: numbers.descendingArray()) {
            TxSet newTry = TxSet.or(result, n);
            if (predicate.test(newTry)) result = newTry;
        }
        System.out.printf("  base: %s, blacklist: %s -> %s\n", base, blacklist, result);
        return result;

    }

    static Stream<TxSet> combinationsUpToSize(TxSet numbers, int size) {
        return IntStream.rangeClosed(1, size)
                .mapToObj(x -> combinations(numbers, x))
                .flatMap(x -> x);
    }

    static Stream<TxSet> combinations(TxSet numbers, int size) {
        if (numbers.size() < size) {
            return Stream.of();
        } else if (numbers.size() == size) {
            return Stream.of(numbers);
        } else if (size == 1) {
            return numbers.stream()
                    .mapToObj(TxSet::of);
        } else {
            return numbers.stream()
                    .parallel()
                    .mapToObj(n -> combinations(TxSet.of(numbers.filter(x -> x < n)), size-1)
                            .map(set -> TxSet.or(set, n))
                    )
                    .flatMap(x -> x);
        }
    }

    private TxSet explore(TxSet candidates,int distanceToExplore) {
        var mem = gMemo.computeIfAbsent(distanceToExplore, k -> new ConcurrentHashMap<>());
        if (mem.containsKey(candidates)) return mem.get(candidates);

        var baseline = baseCase(candidates);
        TxSet result;

        //System.out.println(" ".repeat((2-distanceToExplore)*2) + " baseline: " + baseline.sum() + "=" + baseline);


        if (distanceToExplore == 0) {
            result = baseline;
        } else {
            result = baseline.largest(baseline.size()-1).stream()
                    //.parallel()
                    .peek(n -> System.out.println(" ".repeat((2-distanceToExplore)*2) + " dropping " + n))
                    .mapToObj(n -> TxSet.subtract(candidates, n))
                    .map(fewerNumbers -> explore(fewerNumbers, distanceToExplore-1))
                    .reduce(baseline, (best, current) -> (best.sum() > current.sum() ? best : current));
        }

        TxSet missing = TxSet.subtract(allNumbers, candidates);
        System.out.println("  best without " + missing + ": " + result.sum() + " = " + result);
        //System.out.println(" ".repeat((2-distanceToExplore)*2) + " result: " + result.sum() + "=" + result);

        mem.put(candidates, result);
        return result;
    }
}
