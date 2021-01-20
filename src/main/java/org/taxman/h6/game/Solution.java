package org.taxman.h6.game;

import org.taxman.h6.util.TxList;
import org.taxman.h6.util.TxSet;

import java.io.PrintStream;
import java.util.stream.Collectors;

public class Solution {
    public final Board board;
    public final TxList moves;


    public Solution(Board b, TxList m) {
        board = b;
        moves = m.unmodifiable();
    }

    public int score() {
        return moves.sum();
    }

    private String makeTaxString(TxSet tax) {
        return tax.stream()
                .mapToObj(Integer::toString)
                .collect(Collectors.joining(", "));
    }

    public void display(PrintStream ps) throws VerificationException {
        int allPoints = board.set.sum();
        int playerScore = score();
        int taxScore = allPoints - playerScore;
        float percentOfPot = (float) 100.0 * playerScore/allPoints;
        int lowest = moves.min();
        float lowestPercent = (float) 100.0 * lowest / board.set.max();
        ps.printf("player score: %,d\n", playerScore);
        ps.printf("     tax man: %,d\n", taxScore);
        ps.printf("win by %,d with %.1f%% of the pot\n", playerScore - taxScore, percentOfPot);
        ps.printf("lowest number taken: %,d (%.2f%%)\n", lowest, lowestPercent);
        //ps.println("moves: " + moves);
        ps.println("  game:");
        Board remainder = board;
        for(int m: moves.toArray()) {
            Move move = remainder.makeMove(m);
            ps.println(String.format("      take %2d", m) + ", tax man takes " + makeTaxString(move.tax));
            remainder = move.remainder;
        }
        if (remainder.size() > 0) {
            ps.println("      tax man takes remainder: " + makeTaxString(remainder.set));
        } else {
            ps.println("      no remainder");
        }
    }

    public void verify(int boardSize) throws VerificationException {
        Board remainder = board;

        try {
            for (int m : moves.toArray()) {
                Move move = remainder.makeMove(m);
                remainder = move.remainder;
            }

            var opt = OptimalResult.get(boardSize);
            if (opt != null) {
                if (opt.moves != null) {
                    TxSet thisMoves = TxSet.of(moves);
                    TxSet optimalMoves = TxSet.of(opt.moves);
                    if (!optimalMoves.equals(thisMoves)) {
                        var missing = TxSet.subtract(optimalMoves, thisMoves);
                        var extra = TxSet.subtract(thisMoves, optimalMoves);
                        System.out.println();
                        System.out.println("in optimal moves but not in this solution: " + missing);
                        System.out.println("in this solution but not in optimal moves: " + extra);
                    }
                }
                int thisScore = moves.sum();
                if (opt.score != thisScore) {
                    throw new VerificationException(
                            String.format("for %d the sum of moves should be %,d but came out %,d",
                            boardSize, opt.score, this.score())
                    );
                }
            }
        } catch (VerificationException ve) {
            throw ve.setSolution(this);
        }
    }
}