package org.taxman.h6.util;

import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.Predicate;

public class TxPredicate<T> {
    private final Predicate<T> p;
    private final AtomicInteger evalCount;

    public TxPredicate(Predicate<T> p) {
        this.p = p;
        evalCount = new AtomicInteger(0);
    }

    public int getEvalCount() {
        return evalCount.get();
    }

    public void resetEvalCount() {
        evalCount.set(0);
    }

    public boolean test(T t) {
        evalCount.incrementAndGet();
        return p.test(t);
    }
}
