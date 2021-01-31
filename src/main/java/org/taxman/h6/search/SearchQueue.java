package org.taxman.h6.search;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.PrintStream;
import java.nio.ByteBuffer;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;
import java.util.stream.Stream;

public class SearchQueue {
    private static final int MAX_LIST_LEN = 1000000;  // allow list to grow to this size before writing to file
    public static boolean showFileAccess = false;

    private final int name;
    private int count = 0;
    private final List<byte[]> taskData;
    private Path tmpFile = null;


    public SearchQueue(int name) {
        this.name = name;
        this.taskData = new ArrayList<>();
    }

    public void add(TaskData td) {
        var bytes = td.toByteArray();
        synchronized (this) {
            if (taskData.size() >= MAX_LIST_LEN) taskDataToFile();
            taskData.add(bytes);
            ++count;
        }
    }

    private void taskDataToFile() {
        try {
            if (tmpFile == null) tmpFile = Files.createTempFile("taxman", ".tmp");
            var baos = new ByteArrayOutputStream();
            for (var td: taskData) baos.write(td);
            Files.write(tmpFile, baos.toByteArray(), StandardOpenOption.APPEND);
            if (showFileAccess) System.out.printf("      wrote %,d tasks to disk for target %d\n", taskData.size(), name);
            taskData.clear();
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
    }

    public Stream<TaskData> stream() {
        return Stream.concat(streamList(), streamFile());
    }

    private Stream<TaskData> streamFile() {
        Stream<TaskData> result;
        if (tmpFile == null) {
            result = Stream.empty();
        } else {
            var iter = iterateFile();
            result = Stream.generate(() -> null)
                .takeWhile(x -> iter.hasNext())
                .map(x -> iter.next());
        }
        return result;
    }

    private Iterator<TaskData> iterateFile() {
        try {
            byte[] bytes = Files.readAllBytes(tmpFile);
            Files.delete(tmpFile);
            tmpFile = null;
            ByteBuffer bb = ByteBuffer.wrap(bytes);

            return new Iterator<>() {
                int count = 0;

                @Override
                public boolean hasNext() {
                    boolean result = bb.hasRemaining();
                    if (!result && showFileAccess) {
                        System.out.printf("      read %,d tasks from disk for %d\n", count, name);
                    }
                    return result;
                }

                @Override
                public TaskData next() {
                    ++count;
                    return TaskData.readFromBuffer(bb);
                }
            };
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
    }

    private Stream<TaskData> streamList() {
        var listCopy = List.copyOf(taskData);
        taskData.clear();
        return listCopy.stream()
                .parallel()
                .map(TaskData::fromByteArray);
    }

    public void shutdown() {
        //System.out.println("shutting down " + name + " that had a count of " + count);
        count = 0;
        taskData.clear();
        if (tmpFile != null) {
            try {
                Files.delete(tmpFile);
                tmpFile = null;
            }
            catch (IOException e) {
                throw new RuntimeException(e);
            }
        }
    }

    public void statusReport(PrintStream ps) {
        if (count > 0) {
            ps.printf("      %d: %,7d tasks\n", name, count);
        }
    }
}
