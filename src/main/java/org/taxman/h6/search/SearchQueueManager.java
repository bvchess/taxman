package org.taxman.h6.search;

import java.io.PrintStream;
import java.util.ArrayList;
import java.util.Collections;
import java.util.stream.Collectors;
import java.util.stream.IntStream;
import java.util.stream.Stream;

public class SearchQueueManager {
    private final int max;
    private final int min;
    private final ArrayList<SearchQueue> searchQueues;


    public SearchQueueManager(int max, int min) {
        this.max = max;
        this.min = min;
        this.searchQueues = IntStream.rangeClosed(min, max)
                .mapToObj(SearchQueue::new)
                .collect(Collectors.toCollection(ArrayList::new));
    }

    Stream<TaskData> stream(int target) {
        return searchQueues.get(target-min).stream();
    }

    void add(int target, TaskData td) {
        searchQueues.get(target-min).add(td);
    }

    public void shutdown(int target) {
        searchQueues.get(target-min).shutdown();
    }

    public void shutdownAll() {
        searchQueues.stream()
                .parallel()
                .forEach(SearchQueue::shutdown);
    }

    public void statusReport(PrintStream ps) {
        var q2 = new ArrayList<>(searchQueues);
        Collections.reverse(q2);
        q2.forEach(sq -> sq.statusReport(ps));
    }
}
