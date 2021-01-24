package org.taxman.h6.frame;

import com.squareup.moshi.JsonAdapter;
import com.squareup.moshi.Moshi;
import com.squareup.moshi.Types;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.lang.reflect.Type;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;

public class Hint {
    public final int n;
    public final String comment;
    public final int maxPromotionSum;

    private final static String JSON_DATA_FILE = "/hint.json";
    private static Map<Integer, Hint> hintMap = null;

    public Hint(int n, String comment, int maxPromotionSum) {
        this.n = n;
        this.comment = comment;
        this.maxPromotionSum = maxPromotionSum;
    }

    public static Hint get(int n) {
        loadMap();
        return hintMap.getOrDefault(n, null);
    }

    private static synchronized void loadMap() {
        if (hintMap == null) {
            var is = Hint.class.getResourceAsStream(JSON_DATA_FILE);
            var r = new BufferedReader(new InputStreamReader(is));
            var json = r.lines().collect(Collectors.joining("\n"));
            try {
                hintMap = toMap(fromJson(json));
            } catch (IOException e) {
                throw new RuntimeException("failed loading json", e);
            }
        }
    }

    private static JsonAdapter<List<Hint>> makeAdapter() {
        Moshi moshi = new Moshi.Builder().build();
        Type resultList = Types.newParameterizedType(List.class, Hint.class);
        return moshi.adapter(resultList);
    }

    static List<Hint> fromJson(String json) throws IOException {
        var result = makeAdapter().fromJson(json);
        assert result != null;
        return result;
    }

    static Map<Integer, Hint> toMap(List<Hint> list) throws IOException {
        var seen = new HashSet<Integer>();
        return list.stream()
                .peek(r -> {
                    if (seen.contains(r.n)) throw new RuntimeException("multiple entries for " + r.n);
                    seen.add(r.n);
                })
                .collect(Collectors.toMap(r -> r.n, r -> r));
    }
}
