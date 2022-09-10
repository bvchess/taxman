package org.taxman.h6.oldsearch;


import java.util.ArrayList;
import java.util.List;
import java.util.stream.Collectors;
import java.util.stream.Stream;

public class SearchQueue {
    private static final int MAX_LIST_LEN = 10000;   // allow a thread list to grow to this size before sending to disk

    private final String name;
    private final DiskHelper diskHelper;
    private List<List<TaskData>> taskData;
    private boolean hasBeenShutDown = false;
    private final ThreadLocal<List<TaskData>> taskDataForThread;


    public SearchQueue(int name, SearchQueueManager sqm) {
        this.name = "search queue " + name;
        this.diskHelper = new DiskHelper(sqm.game, name);
        this.taskData = new ArrayList<>();
        this.taskDataForThread = ThreadLocal.withInitial(this::newTaskDataList);
    }

    public String getName() {
        return name;
    }

    private List<TaskData> newTaskDataList() {
        ArrayList<TaskData> list = new ArrayList<>();
        synchronized (this) {
            taskData.add(list);
        }
        return list;
    }

    public void add(TaskData td) {
        assert !hasBeenShutDown;
        var list = taskDataForThread.get();
        list.add(td);

        if (list.size() >= MAX_LIST_LEN) {
            synchronized (this) {
                taskDataForThread.set(newTaskDataList());
                // Can't call taskData.remove() here because it compares elements using the equals() method.
                // Because these lists are being modified by other threads, calling equals() can land us with
                // a ConcurrentModificationException.  So we filter instead in order to remove by comparing pointers.
                taskData = taskData.stream().filter(lst -> lst != list).collect(Collectors.toList());
            }
            diskHelper.sendToDisk(list);
        }
    }

    public void onDeck() {
        if (!hasBeenShutDown) diskHelper.switchToLoading();
    }

    public Stream<TaskData> stream() {
        return Stream.concat(streamList(), diskHelper.stream())
                .onClose(this::shutdown);
    }

    private Stream<TaskData> streamList() {
        return taskData.stream().flatMap(List::stream);
    }

    public void shutdown() {
        // System.out.printf("    shutting down %s\n", diskHelper.getName());
        synchronized (this) {
            taskData.clear(); // The threadlocal still has a pointer to each list, so this won't GC all the lists
        }
        diskHelper.shutdown();
        hasBeenShutDown = true;
    }

    public long getCountOfTasksInMemory() {
        long local = taskData.stream()
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
