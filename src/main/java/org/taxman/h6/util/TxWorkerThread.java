package org.taxman.h6.util;

import java.util.concurrent.ForkJoinPool;
import java.util.concurrent.ForkJoinWorkerThread;

// All the code required to set the priority of the worker threads a little lower.
public class TxWorkerThread extends ForkJoinWorkerThread {
    private final static int PRIORITY = Thread.NORM_PRIORITY - 1;
    private final static int MAX_THREADS = Runtime.getRuntime().availableProcessors() - 1;

    private final static ForkJoinPool.ForkJoinWorkerThreadFactory factory = TxWorkerThread::new;

    public final static ForkJoinPool pool = new ForkJoinPool(MAX_THREADS, factory, null, false);

    public TxWorkerThread(ForkJoinPool pool) {
        super(pool);
        this.setPriority(PRIORITY);
    }

}
