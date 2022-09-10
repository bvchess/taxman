package org.taxman.h6.util;

import java.util.concurrent.ForkJoinPool;
import java.util.concurrent.ForkJoinWorkerThread;

// All the code required to set the priority of the worker threads a little lower.
public class TxWorkerThread extends ForkJoinWorkerThread {
    private final static int PRIORITY = Thread.NORM_PRIORITY - 1;
    private final static ForkJoinPool.ForkJoinWorkerThreadFactory factory = TxWorkerThread::new;
    public static int maxThreads = Runtime.getRuntime().availableProcessors() - 1;  // set this to 1 for debugging
    private static ForkJoinPool pool = null;

    public static ForkJoinPool getWorkerPool() {
        if (pool == null) {
            pool = new ForkJoinPool(maxThreads, factory, null, false);
        }
        return pool;
    }

    public TxWorkerThread(ForkJoinPool pool) {
        super(pool);
        this.setPriority(PRIORITY);
    }

}
