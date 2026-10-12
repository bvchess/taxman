package org.taxman.h6.search;

import org.taxman.h6.bombus.Apiary;
import org.taxman.h6.bombus.Namer;
import org.taxman.h6.game.Board;
import org.taxman.h6.util.TxList;
import org.taxman.h6.util.TxSet;

import java.io.BufferedReader;
import java.io.InputStreamReader;

/** Reads lines "n p1 p2 ..." (n, then promotions) from stdin; prints "n m1 m2 ..." with the Java's apiary game. */
public class GameProbe {
    public static void main(String[] args) throws Exception {
        BufferedReader in = new BufferedReader(new InputStreamReader(System.in));
        String line;
        while ((line = in.readLine()) != null) {
            line = line.trim();
            if (line.isEmpty()) continue;
            String[] parts = line.split("\\s+");
            int n = Integer.parseInt(parts[0]);
            int[] promos = new int[parts.length - 1];
            for (int i = 1; i < parts.length; i++) promos[i - 1] = Integer.parseInt(parts[i]);
            Board board = Board.of(n);
            Apiary a = new Apiary(board, TxSet.of(promos), new Namer());
            TxList moves = a.getSolution();
            StringBuilder sb = new StringBuilder();
            sb.append(n);
            for (int m : moves.toArray()) sb.append(' ').append(m);
            System.out.println(sb);
        }
    }
}
