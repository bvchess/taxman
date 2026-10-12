package org.taxman.h6.bombus;

import org.taxman.h6.util.TxList;

/**
 * Exposes package-private members of Hive to the golden-data exporter (which lives in another package).
 * Lives in the org.taxman.h6.bombus package on the classpath next to bin/taxman.jar; the jar is not sealed.
 */
public final class GoldenHiveAccess {
    private GoldenHiveAccess() {
    }

    /** Hive.getSolution(): the moves for this hive, in the order the hive plays them. */
    public static TxList solution(Hive hive) {
        return hive.getSolution();
    }
}
