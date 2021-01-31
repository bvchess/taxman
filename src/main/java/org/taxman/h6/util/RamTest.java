package org.taxman.h6.util;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.stream.IntStream;

public class RamTest {
    public static void main(String[] args) {
        String msg = "out of memory after just getting started";
        List<byte[]> bytes = Collections.synchronizedList(new ArrayList<>());
        try {
            while (true) {
                IntStream.range(0, 100)
                        .parallel()
                        .mapToObj(i -> new byte[1073741824])
                        .forEach(bytes::add);
            }
        } catch (OutOfMemoryError oome) {
            System.out.printf("out of memory after %d gigs allocated\n", bytes.size());
        }
    }
}