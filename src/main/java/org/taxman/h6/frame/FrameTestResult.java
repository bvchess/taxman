package org.taxman.h6.frame;

import org.taxman.h6.util.TxSet;
import org.taxman.h6.util.TxUnmodifiableSet;

public class FrameTestResult {
    public final boolean success;
    public final TxUnmodifiableSet newRemovals;

    private FrameTestResult(boolean success, TxUnmodifiableSet newRemovals) {
        this.success = success;
        this.newRemovals = newRemovals;
    }

    public static FrameTestResult FAIL = new FrameTestResult(false, TxUnmodifiableSet.EmptySet);

    public static FrameTestResult reportSuccess(TxSet newRemovals) {
        TxUnmodifiableSet removals = (newRemovals == null) ? TxUnmodifiableSet.EmptySet : newRemovals.unmodifiable();
        return new FrameTestResult(true, removals);
    }

    @Override
    public String toString() {
        String result;
        if (success) {
            result = "success" + ((newRemovals.size() > 0) ? " and removed " + newRemovals : "");
        } else {
            result = "failure";
        }
        return result;
    }
}