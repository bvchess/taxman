package org.taxman.h6.util;

import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.function.Function;
import java.util.stream.Collectors;

public class LRUCache {

    public static <K, V> Map<K, V> make(int capacity) {
        return Collections.synchronizedMap(
                new LinkedHashMap<K, V>(capacity *2, 0.5f, true) {
                    @Override
                    protected boolean removeEldestEntry(Map.Entry eldest) {
                        return size() > capacity;
                    }

                    @Override
                    public String toString() {
                        return "{" +
                                entrySet().stream()
                                        .map(entry -> entry.getKey() + ": " + entry.getValue())
                                        .collect(Collectors.joining(", ")) +
                                "}";
                    }
                }
        );
    }

}
