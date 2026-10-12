package org.taxman.h6.frame;

import org.taxman.h6.bombus.BombusSolution;

/**
 * Exposes package-private members of FrameSolver to the golden-data exporter (which lives in another package).
 * Lives in the org.taxman.h6.frame package on the classpath next to bin/taxman.jar; the jar is not sealed.
 */
public final class GoldenFrameAccess {
    private GoldenFrameAccess() {
    }

    /** The acceleration step only: null if solving n from the solution to n-1 does not work. */
    public static BombusSolution solveBasedOnPrevOnly(FrameSolver solver, int n) {
        return solver.solveBasedOnPrevOnly(n);
    }
}
