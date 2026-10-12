package org.taxman.h6.frame;

import org.taxman.h6.bombus.BombusSolution;
import org.taxman.h6.game.Board;
import org.taxman.h6.util.TxList;
import org.taxman.h6.util.TxSet;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.lang.reflect.Field;
import java.util.Map;

/**
 * Reads lines "n m1 m2 ..." where the m's are the moves of the solution to n-1; injects that solution into a
 * FrameSolver and runs the acceleration step only.  Prints "n none" or "n accel m1 m2 ...".
 */
public class AccelProbe {
    @SuppressWarnings("unchecked")
    public static void main(String[] args) throws Exception {
        BufferedReader in = new BufferedReader(new InputStreamReader(System.in));
        String line;
        Field f = FrameSolver.class.getDeclaredField("solutionMap");
        f.setAccessible(true);
        while ((line = in.readLine()) != null) {
            line = line.trim();
            if (line.isEmpty()) continue;
            String[] parts = line.split("\\s+");
            int n = Integer.parseInt(parts[0]);
            int[] prev = new int[parts.length - 1];
            for (int i = 1; i < parts.length; i++) prev[i - 1] = Integer.parseInt(parts[i]);
            FrameSolver solver = new FrameSolver();
            Map<Integer, BombusSolution> map = (Map<Integer, BombusSolution>) f.get(solver);
            map.put(n - 1, new BombusSolution(Board.of(n - 1), TxList.of(prev), TxSet.empty()));
            BombusSolution s = GoldenFrameAccess.solveBasedOnPrevOnly(solver, n);
            StringBuilder sb = new StringBuilder();
            sb.append(n).append(s == null ? " none" : " accel");
            if (s != null) for (int m : s.moves.toArray()) sb.append(' ').append(m);
            System.out.println(sb);
        }
    }
}
