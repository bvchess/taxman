package org.taxman.h6.frame;

import org.taxman.h6.game.Board;
import org.taxman.h6.game.OptimalResult;
import org.taxman.h6.game.Solution;
import org.taxman.h6.bombus.Apiary;
import org.taxman.h6.bombus.BombusSolution;
import org.taxman.h6.bombus.Namer;
import org.taxman.h6.util.TxList;
import org.taxman.h6.util.TxPredicate;
import org.taxman.h6.util.TxSet;
import static org.taxman.h6.util.TxUnmodifiableSet.EmptySet;

import java.util.HashMap;
import java.util.Map;


public class Multi {
    public static boolean printFrames = false;
    public static boolean printAccelerations = false;
    public static boolean printAccelerationFailures = false;
    public static boolean printSearch = false;

    private final Map<Integer, BombusSolution> solutionMap = new HashMap<>();
    private int countOfAccelerated = 0;

    public BombusSolution solve(int n) {
        var sln = new Turbo(n).solve();
        solutionMap.put(n, sln);
        //System.out.println("!! " + n + " " + sln.promotions.size() + " " + sln.promotions.sum());
        return sln;
    }

    public int getCountOfAccelerated() {
        return countOfAccelerated;
    }

    BombusSolution solveBasedOnPrevOnly(int n) {
        return new Turbo(n).solveBasedOnPrev();
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
            if (sln == null) sln = solveTheHardWay();
            return sln;
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
            var prev = getPrevious();
            var a = new Apiary(board, new Namer());
            var inTheBag = a.getSolution().sum();
            var promotionSumMax = getPrevious().score() + n - inTheBag;
            return spinDown(promotionSumMax);
        }

        private BombusSolution getPrevious() {
            var result = solutionMap.getOrDefault(n-1, null);
            if (result == null) result = loadPreviouslyComputed(n-1);
            if (result == null) throw new RuntimeException("cannot find solution to " + (n-1));
            //if (result == null) result = Multi.this.solve(n-1);
            return result;
        }

        private BombusSolution spinDown(int promotionSumMax) {
            var maxPromotions = frame.estimateMaxPromotions(0);
            var candidates = frame.allCandidateNumbersIncludingDownstream();
            if (candidates.largest(maxPromotions).sum() < promotionSumMax) {
                //System.out.println(n + ": lowering the max by " + (promotionSumMax - candidates.largest(maxPromotions).sum()));
                promotionSumMax = candidates.largest(maxPromotions).sum();
            }

            if (printSearch) {
                System.out.printf("  searching for %d promotions totalling as much as %d among %d: %s\n",
                        maxPromotions, promotionSumMax, candidates.size(), candidates);
            }

            var p = new TxPredicate<TxSet>(c -> frame.fits(EmptySet, c));
            var promotions = Search.findLargest(candidates, maxPromotions, promotionSumMax, p);
            //var promotions =  Greedy.find(candidates, maxPromotions, p);

            if (printSearch) {
                System.out.printf("  found %d promotions totalling %d: %s\n",
                        promotions.size(), promotions.sum(), promotions);
            }

            Apiary a = new Apiary(board, promotions, new Namer());
            //System.out.println("promotions are: " + promotions);
            //System.out.println(a.debugDump("!!"));
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
            var droppedPromotions = TxSet.subtract(recycledPromotions, prudentPromotions);
            var idealMoves = TxSet.or(prevMoves, n);
            idealMoves = TxSet.subtract(idealMoves, newlyImpossibleMoves);
            idealMoves = TxSet.subtract(idealMoves, droppedPromotions);
            Apiary a2 = new Apiary(board, prudentPromotions, new Namer());
            var newMoves = a2.getSolution();
            //System.out.printf("%d: re-use new move sum: %d, ideal is %d, prudent promotions %s\n", n, newMoves.sum(), idealMoves.sum(), prudentPromotions);
            BombusSolution result = null;
            if (newMoves.sum() == idealMoves.sum()) {
                result = new BombusSolution(board, newMoves, prudentPromotions);
                if (printSearch) {
                    System.out.printf(
                            "  found optimal promotions using previous solution: %d promotions totalling %d: %s\n",
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
                System.out.printf("  reusePrev: %d promotions totalled %d\n", achievedPromotions.size(), promoteSum);
            }
            return result;
        }
    }
}