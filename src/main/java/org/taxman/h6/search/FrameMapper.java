package org.taxman.h6.search;

import org.taxman.h6.util.TxSet;

import java.util.Arrays;
import java.util.concurrent.atomic.AtomicLong;

public class FrameMapper {
    public final AtomicLong promotionCounter;  // doesn't particularly belong here, just a convenient place to put it
    private final int[] homeFrameForNumber;
    private final int[][] slotWithinFrame;

    public FrameMapper(Frame[] frames) {
        this.promotionCounter = new AtomicLong(0);
        int maxNumber = Arrays.stream(frames)
                .map(Frame::allNumbers)
                .flatMapToInt(TxSet::stream)
                .max()
                .orElse(0);
        this.slotWithinFrame = new int[frames.length][maxNumber+1];
        int[] slotCount = new int[frames.length];
        this.homeFrameForNumber = new int[maxNumber + 1];
        for (int i = 0; i < frames.length; i++) {
            for (int n : frames[i].getFactors().toArray()) {
                homeFrameForNumber[n] = i;
                slotWithinFrame[i][n] = slotCount[i]++;
            }
            if (i > 0) {
                for (int m : frames[i].getFactors().toArray()) {
                    slotWithinFrame[i-1][m] = slotCount[i-1]++;
                }
            }
        }
    }

    public int homeFrame(int n) {
        return homeFrameForNumber[n];
    }

    public int slotForNumberWithinFrame(int frameNumber, int n) {
        return this.slotWithinFrame[frameNumber][n];
    }
}