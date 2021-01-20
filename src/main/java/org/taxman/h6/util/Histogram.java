package org.taxman.h6.util;

import java.io.PrintStream;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicLong;

public class Histogram<K extends Comparable<K>> {
    private final ConcurrentHashMap<K, AtomicLong> map = new ConcurrentHashMap<>();

    public Histogram() {
    }

    public void add(K bucket) {
        map.computeIfAbsent(bucket, x -> new AtomicLong(0)).incrementAndGet();
    }

    public void display(PrintStream ps, double percentThreshold) {
        long total = map.values().stream()
                .mapToLong(AtomicLong::get)
                .sum();
        var runningTotal = new AtomicLong(0);

        map.keySet().stream()
                .sorted()
                .forEach(k -> {
                    long val = map.get(k).get();
                    double pct = (double) 100 * val / total;
                    runningTotal.addAndGet(val);
                    double runningTotalPct = (double) 100 * runningTotal.get() / total;
                    if (pct > percentThreshold) ps.printf("%s: %,d, %2.0f%%, %2.0f%%\n", k, val, pct, runningTotalPct);
                });
    }
}
