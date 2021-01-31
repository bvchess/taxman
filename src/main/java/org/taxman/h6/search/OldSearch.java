package org.taxman.h6.search;

import org.taxman.h6.util.Stopwatch;
import org.taxman.h6.util.TxSet;

import java.util.*;
import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.Predicate;
import java.util.stream.IntStream;

import static org.taxman.h6.util.TxUnmodifiableSet.EmptySet;

public class OldSearch {
    public static boolean printStatsPerTarget = false;
    public static boolean printSummary = false;

    private final Predicate<TxSet> predicate;
    private final int searchMaxSize;
    private final ForkJoinPool pool = ForkJoinPool.commonPool();
    private int levelsExplored = 0;
    private AtomicLong taskCounter;
    private long taskTotal = 0;


    private static int getEnvOrDefault(String name, int dflt) {
        String str = System.getenv(name);
        if (str != null) {
            try {
                return Integer.parseInt(str);
            }
            catch (final NumberFormatException e) {
                System.err.printf("WARNING: could not parse environment variable %s value as an integer: %s",
                        name, str);
            }
        }
        return dflt;
    }

    public OldSearch(Predicate<TxSet> predicate, int maxSize) {
        this.predicate = predicate;
        this.searchMaxSize = maxSize;
    }

    public static TxSet findLargest(TxSet numbers, int maxSize, int maxSum, Predicate<TxSet> predicate) {
        return new OldSearch(predicate, maxSize).findLargest(numbers, maxSum);
    }

    private TxSet findLargest(TxSet numbers, int maxSum) {
        var stopwatch = new Stopwatch().start();
        var result = IntStreamDescendingClosed(maxSum, 0)
                .peek(i -> ++levelsExplored)
                .mapToObj(i -> findTarget(numbers, i))
                .filter(Objects::nonNull)
                .findFirst()
                .orElse(null);
        stopwatch.stop();

        assert result != null;

        if (printSummary) {
            System.out.printf("  explored %d levels with %,d tasks in %.1f seconds\n",
                    levelsExplored, taskTotal, stopwatch.seconds());
            System.out.printf("  parallelism: %d\n", pool.getParallelism());
        }

        return result;
    }

    private void recordNewTask(int target, int maxSize) {
        taskCounter.incrementAndGet();
        //bigHist.add(maxSize);
    }

    public static IntStream IntStreamDescendingClosed(int highest, int lowest) {
        return IntStream.rangeClosed(lowest, highest)
                .map(i -> highest - (i-lowest));
    }

    private TxSet findTarget(TxSet numbers, int target) {
        Stopwatch stopwatch = null;

        if (printStatsPerTarget) {
            stopwatch = new Stopwatch().start();
        }
        taskCounter = new AtomicLong(0);
        var task = new TargetFinder(numbers, EmptySet, searchMaxSize, target);
        pool.execute(task);
        var result = task.join();
        taskTotal += taskCounter.get();
        if (printStatsPerTarget && stopwatch != null) {
            stopwatch.stop();
            System.out.printf("    searching for %d took %.2f seconds with %,d tasks\n",
                    target, stopwatch.seconds(), taskCounter.get());
        }
        return result;
    }

    private class TargetFinder extends RecursiveTask<TxSet> {
        private final TxSet numbers;
        private final TxSet base;
        private final int maxSize;
        private final int target;

        private TargetFinder(TxSet numbers, TxSet base, int maxSize, int target) {
            this.numbers = numbers;
            this.base = base;
            this.maxSize = maxSize;
            this.target = target;
            recordNewTask(target, maxSize);
        }

        @Override
        protected TxSet compute() {
            TxSet result;
            if (numbers.contains(target)) {
                TxSet assembled = TxSet.or(base, target);
                result = (predicate.test(assembled)) ? assembled : null;
            } else if (maxSize == 1) {
                result = null;
            } else if (maxSize < searchMaxSize/2) {  // unclear when (or even if) we should switch to recursive
                result = recursiveSearch(numbers, TxSet.of(base), maxSize, target);
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
            TxSet newNumbers = TxSet.of(numbers);
            for (int n: numbers.descendingArray()) {
                int newTarget = target - n;
                newNumbers = TxSet.subtract(newNumbers, n);
                if (newNumbers.sumOfLargest(maxSize-1) < newTarget) break;
                if (n < target) {
                    TxSet newBase = TxSet.or(base, n);
                    if (predicate.test(newBase)) {
                        result.add(new TargetFinder(newNumbers, newBase, maxSize - 1, newTarget));
                    }
                }
            }
            return result;
        }

        // Recursive doesn't use any parallelism, but it does get to reuse TxSets rather than creating new
        // ones all over the place.  So we get the parallelism in the top half of the search tree
        // from the Fork/Join tasks, and get some efficiency in the bottom half of the tree from the
        // recursive search.
        private TxSet recursiveSearch(TxSet numbers, TxSet base, int maxSize, int target) {
            TxSet result;
            if (numbers.contains(target)) {
                TxSet assembled = TxSet.or(base, target);
                result = (predicate.test(assembled)) ? assembled : null;
            } else if (maxSize == 1) {
                result = null;
            } else {
                result = null;
                TxSet newNumbers = TxSet.of(numbers);
                for (int n: numbers.descendingArray()) {
                    int newTarget = target - n;
                    newNumbers.remove(n);
                    if (newNumbers.sumOfLargest(maxSize-1) < newTarget) break;
                    if (n < target) {
                        base.append(n);
                        if (predicate.test(base)) {
                            result = recursiveSearch(newNumbers, base, maxSize - 1, newTarget);
                        }
                        base.remove(n);
                        if (result != null) break;
                    }
                }
            }

            return result;
        }

    }

}
