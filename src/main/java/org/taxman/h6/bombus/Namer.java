package org.taxman.h6.bombus;

import org.taxman.h6.util.TxSet;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;

public class Namer {
    private final Map<Integer, String> numberToName = new HashMap<>();
    private int nextNumber = 0;

    String getNameForSources(TxSet sources) {
        String result;
        List<String> alreadyAssigned = sources.filter(numberToName::containsKey)
                .mapToObj(numberToName::get)
                .distinct()
                .collect(Collectors.toList());
        if (alreadyAssigned.size() > 0) {
            result = String.join("+", alreadyAssigned);
        } else {
            result = nextName();
            sources.forEach(n -> numberToName.put(n, result));
        }
        return result;
    }

    private String nextName() {
        return intToName(nextNumber++);
    }

    // Generate names like Excel column names A, B, C ... AA, AB, ...
    private static String intToName(int i) {
        final int base = 26;
        final char remainderLetter = (char) (i % base + 'A');
        final int divided = i/base;
        return ((divided > 0) ? intToName(divided - 1) : "") + remainderLetter;
    }
}
