package org.taxman.h6.util;

import java.time.Duration;
import java.time.Instant;

public class Stopwatch {
    private Instant start;
    private Instant finish;

    public Stopwatch start() {
        start = Instant.now();
        return this;
    }

    public void stop() {
        finish = Instant.now();
    }

    public Duration duration() {
        return Duration.between(start, finish);
    }

    public float seconds() {
        return (float) duration().toMillis()/1000;
    }
}
