package org.taxman.h6.util;

import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;
import java.util.function.Supplier;

public class Memo {
    public static boolean showCacheClearing = true;

    private static final String LIMIT_PROPERTY_NAME = "taxman.memoLimit";
    private static final int LIMIT_DEFAULT = 10000000;

    private static final int MAX_SIZE = getMemoLimit();
    private static final int patrolRate = 100000;

    private volatile ConcurrentHashMap<TxSet, Boolean> map = makeMap();
    private final AtomicLong lookupCounter = new AtomicLong(0);
    private final AtomicLong hitCounter = new AtomicLong(0);
    private final AtomicInteger computeCounter = new AtomicInteger(0);
    private final String name;

    public Memo(String name) {
        this.name = name;
    }

    private static ConcurrentHashMap<TxSet, Boolean> makeMap() {
        return new ConcurrentHashMap<>(100,  0.75f, Runtime.getRuntime().availableProcessors());
    }

    private static int getMemoLimit() {
        int result = LIMIT_DEFAULT;
        var sys = System.getProperty(LIMIT_PROPERTY_NAME, null);
        if (sys != null) {
            try {
                result = Integer.parseInt(sys);
                System.out.printf("memoization size limit set to %,d\n", result);
            } catch (NumberFormatException e) {
                System.out.printf("ERROR: cannot parse %s value %s, using default %,d\n",
                        LIMIT_PROPERTY_NAME, sys, LIMIT_DEFAULT);
            }
        }
        return result;
    }

    public boolean test(TxSet set, Supplier<Boolean> compute) {
        lookupCounter.incrementAndGet();
        var result = map.getOrDefault(set, null);
        if (result == null) {
            if (computeCounter.incrementAndGet() % patrolRate == 0) cleanCache();
            result = compute.get();
            map.put(set, result);
        } else {
            hitCounter.incrementAndGet();
        }

        //if (lookupCounter.get() % patrolRate*100 == 0) {
        //    System.out.printf("!> hit rate for %s is %.1f with %,d lookups and size %,d\n",
        //    this, hitCounter.get()*100.0/lookupCounter.get(), lookupCounter.get(), map.size());
        //}

        return result;
    }

    private void cleanCache() {
        boolean showClear = false;
        int originalSize = 0;
        if (map.size() > MAX_SIZE) {
            synchronized (this) {
                originalSize = map.size();
                if (originalSize > MAX_SIZE) {
                    map.clear();
                    showClear = showCacheClearing;
                }
            }
        }
        if (showClear) {
            System.out.printf("      cleared memoization cache %s of size %,d\n", name, originalSize);
        }
    }

}
