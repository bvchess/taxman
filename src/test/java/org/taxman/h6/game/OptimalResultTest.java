package org.taxman.h6.game;

import org.junit.jupiter.api.Test;
import org.taxman.h6.game.OptimalResult;

import java.io.IOException;
import java.util.List;
import java.util.stream.IntStream;

public class OptimalResultTest {

    private final static String json =
            "[{'n': 2, 'score': 2, 'moves': [2]}, {'n': 3, 'score': 3}, {'n': 4, 'score': 7, 'moves': [3, 4]}]"
                    .replaceAll("'","\"");  // to make it legal json
    private final static List<OptimalResult> expected = List.of(
            new OptimalResult(2, 2, new int[]{2}),
            new OptimalResult(3, 3),
            new OptimalResult(4, 7, new int[]{3, 4})
    );

    @Test
    public void loadFromJson() throws IOException {
        var result = OptimalResult.fromJson(json);
        assert result.equals(expected) : "expected " + expected + ", got " + result;
    }

    @Test
    public void writeToJson() throws IOException {
        String str = OptimalResult.toJson(expected);
        System.out.println(str);
        var result = OptimalResult.fromJson(str);
        assert result.equals(expected) : "expected " + expected + ", got " + result;
    }

    @Test
    public void get() {
        var result = OptimalResult.get(4);
        var expect = expected.get(2);
        System.out.println(result);
        assert result.equals(expect) : "expected " + expect + ", got " + result;
    }


    //@Test
    public void dumpInOeisFormat() {
        OptimalResult.getAll()
                .forEach(optRes -> System.out.println(optRes.oeisFormat()));
        System.out.println();
        OptimalResult.getAll()
                .forEach(optRes -> System.out.println(optRes.hoeySolutionFormat()));
    }
}