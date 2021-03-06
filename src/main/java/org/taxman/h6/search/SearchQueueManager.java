package org.taxman.h6.search;

import java.io.PrintStream;
import java.time.ZonedDateTime;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.stream.Collectors;
import java.util.stream.IntStream;
import java.util.stream.Stream;

public class SearchQueueManager {

    private static final DateTimeFormatter timestampFormat = DateTimeFormatter.ofPattern("HH:mm a");

    private final int min;
    public final int game;
    private final ArrayList<SearchQueue> searchQueues;


    public SearchQueueManager(int max, int min, int game) {
        this.min = min;
        this.game = game;
        this.searchQueues = IntStream.rangeClosed(min, max)
                .mapToObj(i-> new SearchQueue(i, this))
                .collect(Collectors.toCollection(ArrayList::new));
    }

    Stream<TaskData> stream(int target) {
        if (target - 1 - min >= 0) searchQueues.get(target-1-min).onDeck();
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
        var timestamp = ZonedDateTime.now().format(timestampFormat);
        long total = getTotalTaskCount();
        long onDisk = getCountOfTasksOnDisk();
        double onDiskPercent = 100.0 * onDisk / total;

        ps.printf("    %s active targets with %,d tasks (%.1f%% on disk) at %s\n",
                getCountOfActiveSearchTargets(), total, onDiskPercent, timestamp
        );
    }

    private long getCountOfActiveSearchTargets() {
        return searchQueues.stream()
                .mapToLong(SearchQueue::getTotalTaskCount)
                .filter(i -> i > 0)
                .count();
    }

    private long getCountOfTasksInMemory() {
        return searchQueues.stream()
                .mapToLong(SearchQueue::getCountOfTasksInMemory)
                .sum();
    }

    private long getCountOfTasksOnDisk() {
        return searchQueues.stream()
                .mapToLong(SearchQueue::getCountOfTasksOnDisk)
                .sum();
    }

    private long getTotalTaskCount() {
        return getCountOfTasksInMemory() + getCountOfTasksOnDisk();
    }
}
