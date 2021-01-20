package org.taxman.h6.util;

import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.Predicate;

public class TxPredicate<T> implements Predicate<T> {
    private final Predicate<T> p;
    private final AtomicInteger evalCount;

    public TxPredicate(Predicate<T> p) {
        this.p = p;
        evalCount = new AtomicInteger(0);
    }

    public int getEvalCount() {
        return evalCount.get();
    }

    @Override
    public boolean test(T t) {
        evalCount.incrementAndGet();
        return p.test(t);
    }
}
