package org.taxman.h6.util;

import org.junit.jupiter.api.Test;

import java.util.Map;

public class LRUCacheTest {

    @Test
    public void t1() {
        Map<Integer, Boolean> c = LRUCache.make(2);
        c.put(1, true);
        c.put(2, false);
        c.put(3, false);
        System.out.println(c);
        assert !c.containsKey(1);
        assert c.containsKey(2);
        assert c.containsKey(3);
    }

    @Test
    public void t2() {
        Map<Integer, Boolean> c = LRUCache.make(2);
        c.put(1, true);
        c.put(2, false);
        c.get(1);
        c.put(3, false);
        System.out.println(c);
        assert c.containsKey(1);
        assert !c.containsKey(2);
        assert c.containsKey(3);
    }

    @Test
    public void t3() {
        Map<Integer, Boolean> c = LRUCache.make(2);
        c.put(1, true);
        c.put(2, false);
        c.get(1);
        c.put(3, false);
        c.get(1);
        c.put(4, true);
        System.out.println(c);
        assert c.containsKey(1);
        assert !c.containsKey(2);
        assert !c.containsKey(3);
        assert c.containsKey(4);
    }
}
