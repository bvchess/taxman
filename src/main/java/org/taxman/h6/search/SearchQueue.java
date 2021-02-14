package org.taxman.h6.search;


import java.util.LinkedList;
import java.util.List;
import java.util.concurrent.ConcurrentHashMap;
import java.util.stream.Collectors;
import java.util.stream.Stream;

public class SearchQueue {
    // allow a thread list to grow to this size before sending to disk helper
    private static final int MAX_LIST_LEN = 1000;

    private final DiskHelper diskHelper;
    private final ConcurrentHashMap<String, List<TaskData>> taskData;

    public SearchQueue(int name, SearchQueueManager sqm) {
        this.taskData = new ConcurrentHashMap<>(Runtime.getRuntime().availableProcessors()*2);
        this.diskHelper = new DiskHelper(sqm.game, name);
    }

    public void add(TaskData td) {
        var threadName = Thread.currentThread().getName();
        var list = taskData.computeIfAbsent(threadName, x -> new LinkedList<>());
        list.add(td);

        if (list.size() >= MAX_LIST_LEN) {
            diskHelper.sendToDisk(list);
            taskData.put(threadName, new LinkedList<>());
        }
    }

    public void onDeck() {
        diskHelper.switchToLoading();
    }

    public Stream<TaskData> stream() {
        return Stream.concat(streamList(), diskHelper.stream());
    }

    private Stream<TaskData> streamList() {
        var listCopy = taskData.values().stream()
                .flatMap(List::stream)
                .collect(Collectors.toList());
        taskData.clear();
        return listCopy.stream();
    }

    public void shutdown() {
        taskData.clear();
        diskHelper.shutdown();
    }

    public long getCountOfTasksInMemory() {
        long local = taskData.values().stream()
                .mapToLong(List::size)
                .sum();
        return local + diskHelper.getCountOfTasksInMemory();
    }

    public long getCountOfTasksOnDisk() {
        return diskHelper.getTaskCountOnDisk();
    }

    public long getTotalTaskCount() {
        return getCountOfTasksInMemory() + getCountOfTasksOnDisk();
    }
}
