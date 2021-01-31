package org.taxman.h6.search;

import org.taxman.h6.util.Stopwatch;
import org.taxman.h6.util.TxSet;

import java.util.*;
import java.util.concurrent.ForkJoinPool;
import java.util.concurrent.ForkJoinTask;
import java.util.concurrent.RecursiveTask;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.Predicate;
import java.util.stream.Collectors;
import java.util.stream.IntStream;
import java.util.stream.Stream;

import static org.taxman.h6.util.TxUnmodifiableSet.EmptySet;

public class Search {
    public static boolean printStatsPerTarget = false;
    public static boolean printSummary = false;


    private final Candidates[] candidatesArray;
    private final Predicate<TxSet> predicate;
    private final int searchMaxSize;
    private final int searchMaxSum;
    private final int searchMinSum;
    private final SearchQueueManager sqm;

    private final ForkJoinPool pool = ForkJoinPool.commonPool();
    private AtomicLong taskCounter;
    private int currentTopLevelTarget;


    public Search(TxSet candidateNumbers, Predicate<TxSet> predicate, int maxSize, int maxSum, int minSum) {
        this.candidatesArray = IntStream.rangeClosed(0, candidateNumbers.size())
                .mapToObj(i-> new Candidates(candidateNumbers, i))
                .toArray(Candidates[]::new);
        this.predicate = predicate;
        this.searchMaxSize = maxSize;
        this.searchMaxSum = maxSum;
        this.searchMinSum = minSum;
        this.sqm = new SearchQueueManager(maxSum, minSum);
    }

    public static TxSet findLargest(TxSet numbers, int maxSize, int maxSum, int minSum, Predicate<TxSet> predicate) {
        return new Search(numbers, predicate, maxSize, maxSum, minSum).findLargest();
    }

    private Candidates getCandidates(int size) {
        return candidatesArray[size];
    }

    private TxSet findLargest() {
        try {
            long taskTotal = 0;
            int levelsExplored = 0;
            var stopwatch = new Stopwatch().start();

            TxSet result = null;
            for (currentTopLevelTarget = searchMaxSum; result == null; currentTopLevelTarget--) {
                taskCounter = new AtomicLong(0);
                result = findTarget(currentTopLevelTarget);
                taskTotal += taskCounter.get();
                ++levelsExplored;
            }
            stopwatch.stop();

            if (printSummary) {
                System.out.printf("  explored %d levels with %,d tasks in %.1f seconds\n", levelsExplored, taskTotal, stopwatch.seconds());
                System.out.printf("  parallelism: %d\n", pool.getParallelism());
            }

            return result;
        }
        finally {
            sqm.shutdownAll();
        }
    }

    private TxSet findTarget(int target) {
        Stopwatch stopwatch = (printStatsPerTarget) ? new Stopwatch().start() : null;

        var targetFinders = streamTargetFinders(target)
                .map(ForkJoinTask::fork)
                .collect(Collectors.toList());
        if (target < this.searchMaxSum) sqm.shutdown(target);
        var result = targetFinders.stream()
                .map(ForkJoinTask::join)
                .filter(Objects::nonNull)
                .findFirst()
                .orElse(null);
        if (stopwatch != null) {
            stopwatch.stop();
            System.out.printf("    searching for %d took %.2f seconds with %,d tasks\n",
                    target, stopwatch.seconds(), taskCounter.get());
        }
        //if (queueStatusReport) sqm.statusReport(System.out);
        return result;
    }

    private Stream<TargetFinder> streamTargetFinders(int target) {
        Stream<TargetFinder> result;
        if (target == searchMaxSum) {
            result = Stream.of(new TargetFinder(candidatesArray.length-1, EmptySet, target));
        } else {
            result = sqm.stream(target)
                    .map(TargetFinder::new);
        }
        return result;
    }

    private class TargetFinder extends RecursiveTask<TxSet> {
        private final int candidatesRef;
        private final TxSet base;
        private final int target;

        private TargetFinder(int candidatesRef, TxSet base, int target) {
            assert target >= 0;
            this.candidatesRef = candidatesRef;
            this.base = base;
            this.target = target;
        }

        private TargetFinder(TaskData data) {
            this(data.candidatesRef, data.base, data.target);
        }

        private void wakeAt(int candidatesRef, TxSet base, int target, int maxPotential) {
            if (maxPotential < 0) maxPotential = 0;
            int wakeAt = currentTopLevelTarget - target + maxPotential;
            if (wakeAt >= searchMinSum) {
                //System.out.printf("waking at %d: %d=%s\n", wakeAt, base.sum(), base);
                sqm.add(wakeAt, new TaskData(candidatesRef, base, maxPotential));
            }
        }

        @Override
        protected TxSet compute() {
            TxSet result;
            taskCounter.incrementAndGet();
            var candidates = getCandidates(candidatesRef);
            int maxSize = searchMaxSize - base.size();

            //if System.out.printf("      base: %s, target %d, maxSize: %d\n", base, target, maxSize);

            if (target == 0) {
                result = (predicate.test(base)) ? base : null;
            } else if (candidates.contains(target)) {
                TxSet assembled = TxSet.or(base, target);
                result = (predicate.test(assembled)) ? assembled : null;
                if (result == null) wakeAt(candidatesRef, base, target, candidates.nextHighest(target));
            } else if (maxSize == 1) {
                wakeAt(candidatesRef, base, target, candidates.nextHighest(target));
                result = null;
            } else {
                result = ForkJoinTask.invokeAll(createBranches()).stream()
                        .map(ForkJoinTask::join)
                        .filter(Objects::nonNull)
                        .findFirst()
                        .orElse(null);
            }

            return result;
        }

        private Collection<TargetFinder> createBranches() {
            List<TargetFinder> result = new ArrayList<>();
            int maxSize = searchMaxSize - base.size();

            int currentCandidatesRef = candidatesRef;
            for (int n: getCandidates(candidatesRef).getDescendingArray()) {
                int sum = getCandidates(currentCandidatesRef).sumOfLargest(maxSize);
                if (sum < target) {
                    wakeAt(currentCandidatesRef, base, target, sum);
                    break;
                }
                int newTarget = target - n;
                --currentCandidatesRef;
                TxSet newBase = TxSet.or(base, n);
                if (predicate.test(newBase)) {
                    result.add(new TargetFinder(currentCandidatesRef, newBase, newTarget));
                }
            }
            return result;
        }
    }

}