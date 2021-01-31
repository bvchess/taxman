package org.taxman.h6.frame;

import org.taxman.h6.game.Board;
import org.taxman.h6.game.OptimalResult;
import org.taxman.h6.game.Solution;
import org.taxman.h6.bombus.Apiary;
import org.taxman.h6.bombus.BombusSolution;
import org.taxman.h6.bombus.Namer;
import org.taxman.h6.game.Solver;
import org.taxman.h6.search.Search;
import org.taxman.h6.util.TxList;
import org.taxman.h6.util.TxPredicate;
import org.taxman.h6.util.TxSet;
import static org.taxman.h6.util.TxUnmodifiableSet.EmptySet;

import java.io.PrintStream;
import java.util.HashMap;
import java.util.Map;


public class FrameSolver implements Solver {
    // debug output flags
    public static boolean printFrames = false;
    public static boolean printAccelerations = false;
    public static boolean printAccelerationFailures = false;
    public static boolean printSearch = false;

    // some debugging modes
    public static boolean verifyAccelerations = false;
    public static boolean cheatIfNoAcceleration = false;


    private final Map<Integer, BombusSolution> solutionMap = new HashMap<>();
    private int countOfAccelerated = 0;

    public BombusSolution solve(int n) {
        var sln = new Turbo(n).solve();
        sln.verify(n);
        solutionMap.put(n, sln);
        return sln;
    }

    public void printInternalsReport(PrintStream ps) {
        int accCount = getCountOfAccelerated();
        String g2 = accCount == 1 ? "game" : "games";
        System.out.printf("accelerated %d %s\n", accCount, g2);
    }

    public int getCountOfAccelerated() {
        return countOfAccelerated;
    }

    BombusSolution solveBasedOnPrevOnly(int n) {
        return new Turbo(n).solveBasedOnPrev();
    }

    BombusSolution solveTheHardWay(int n) {
        return new Turbo(n).solveTheHardWay();
    }

    BombusSolution loadPreviouslyComputed(int n) {
        var or = OptimalResult.get(n);
        BombusSolution result = null;
        if (or != null && or.moves != null) {
            return BombusSolution.upgrade(new Solution(Board.of(n), TxList.of(or.moves)));
        }
        return result;
    }

    private class Turbo {
        final private int n;
        final private Board board;
        final private BaseFrame frame;

        private Turbo(int n) {
            this.n = n;
            this.board = Board.of(n);
            this.frame = FrameBuilder.build(board);
            if (printFrames) {
                frame.debugDump(System.out);
            }
        }

        BombusSolution solve() {
            BombusSolution sln = solveBasedOnPrev();
            if (cheatIfNoAcceleration && sln == null) {
                System.out.println("BIG CHEAT for " + n);
                var cheat = loadPreviouslyComputed(n);
                sln = BombusSolution.upgrade(cheat);
            } else if (sln == null) {
                sln = solveTheHardWay();
            }
            return sln;
        }

        private void verifyAcceleration(BombusSolution sln) {
            if (n < 2) return;
            System.out.println("verifying acceleration for " + n);
            var sln2 = solveTheHardWay();
            assert sln.score() == sln2.score() : "accelerated solution for " + n + " did not verify";
        }

        private BombusSolution solveBasedOnPrev() {
            BombusSolution result = board.fm.isPrime(n) ? solutionForPrime() : reusePrev();
            if (result != null) {
                ++countOfAccelerated;
                if (printAccelerations) {
                    System.out.println("  accelerated " + n);
                }
            }
            return result;
        }

        private BombusSolution solutionForPrime() {
            if (printSearch) {
                System.out.println("  n is prime, so solution is built from solution to n-1");
            }
            var prev = getPrevious();
            var newMoves = prev.moves.toArray();
            if (newMoves.length == 0) newMoves = new int[1];  // the solution to 1 has no moves, so need this for 2
            newMoves[0] = n;
            return new BombusSolution(board, TxList.of(newMoves), prev.promotions);
        }

        private BombusSolution solveTheHardWay() {
            return spinDown(getMaxPromotionSum());
        }

        private int getMaxPromotionSum() {
            var a = new Apiary(board, new Namer());
            var inTheBag = a.getSolution().sum();
            var result = getPrevious().score() + n - inTheBag;
            var hint = Hint.get(n);
            if (hint != null && hint.maxPromotionSum > 0) result = hint.maxPromotionSum;
            return result;
        }

        private BombusSolution getPrevious() {
            var result = solutionMap.getOrDefault(n-1, null);
            if (result == null) result = loadPreviouslyComputed(n-1);
            if (result == null) throw new RuntimeException("cannot find solution to " + (n-1));
            //if (result == null) result = Multi.this.solve(n-1);
            return result;
        }

        private BombusSolution spinDown(int maxPromotionSum) {
            var maxPromotions = frame.estimateMaxPromotions(0);
            var candidates = frame.allCandidateNumbersIncludingDownstream();
            if (candidates.largest(maxPromotions).sum() < maxPromotionSum) {
                //System.out.println(n + ": lowering the max by " + (promotionSumMax - candidates.largest(maxPromotions).sum()));
                maxPromotionSum = candidates.largest(maxPromotions).sum();
            }

            if (printSearch) {
                System.out.printf("  searching for %d promotions totaling as much as %d among %d numbers: %s\n",
                        maxPromotions, maxPromotionSum, candidates.size(), candidates);
            }

            var p = new TxPredicate<TxSet>(c -> frame.fits(EmptySet, c));
            var promotions = Search.findLargest(candidates, maxPromotions, maxPromotionSum, maxPromotionSum - n, p);
            //var promotions = OldSearch.findLargest(candidates, maxPromotions, maxPromotionSum, p);


            if (printSearch) {
                System.out.printf("  found %d promotions totaling %d: %s\n",
                        promotions.size(), promotions.sum(), promotions);
            }

            Apiary a = new Apiary(board, promotions, new Namer());
            return new BombusSolution(board, a.getSolution(), promotions);
        }


        private BombusSolution reusePrev() {
            var a1 = new Apiary(board, new Namer());
            var a1Candidates = a1.getPromotionCandidateNumbers();
            if (a1Candidates.size() == 0) return new BombusSolution(board, a1.getSolution(), EmptySet);

            var prevMoves = TxSet.of(getPrevious().moves);
            var maxPromotions = frame.estimateMaxPromotions(0);
            var recycledPromotions = TxSet.and(a1Candidates, prevMoves);
            var prudentPromotions = recycledPromotions.largest(maxPromotions);
            var newlyImpossibleMoves = TxSet.and(a1.getImpossible(), prevMoves);
            var idealMoves = TxSet.or(prevMoves, n);
            idealMoves = TxSet.subtract(idealMoves, newlyImpossibleMoves);
            Apiary a2 = new Apiary(board, prudentPromotions, new Namer());
            var newMoves = a2.getSolution();
            //System.out.printf("%d: re-use new move sum: %d, ideal is %d, prudent promotions %s\n", n, newMoves.sum(), idealMoves.sum(), prudentPromotions);
            BombusSolution result = null;
            if (newMoves.sum() == idealMoves.sum()) {
                result = new BombusSolution(board, newMoves, prudentPromotions);
                if (printSearch) {
                    System.out.printf("  absolutely ideal score: %,d\n", TxSet.or(prevMoves, n).sum());
                    System.out.printf("  newly impossible moves: %,d\n", newlyImpossibleMoves.sum());
                    System.out.printf("           we settle for: %,d\n", idealMoves.sum());
                    System.out.printf("   using %d of %d possible promotions\n", prudentPromotions.size(), maxPromotions);
                    System.out.printf(
                            "  found optimal promotions using previous solution: %d promotions totaling %d: %s\n",
                            prudentPromotions.size(), prudentPromotions.sum(), prudentPromotions
                    );
                }
            } else if (printAccelerationFailures) {
                int newSum = newMoves.sum();
                int idealSum = idealMoves.sum();
                int delta = idealSum - newSum;
                var achievedPromotions = TxSet.and(prudentPromotions, TxSet.of(newMoves));
                int promoteSum = achievedPromotions.sum();
                System.out.printf("  reusePrev: scored %,d, %,d less than the ideal score of %,d\n",
                        newSum, delta, idealSum);
                System.out.printf("  reusePrev: %d promotions totaled %d\n", achievedPromotions.size(), promoteSum);
            }

            if (verifyAccelerations && result != null) verifyAcceleration(result);
            return result;
        }
    }
}