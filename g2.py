import factormath


class G2Solver(object):
    def __init__(self, x):
        if isinstance(x, int):
            self.board = frozenset(range(1, x + 1))
        else:
            self.board = frozenset(x)  # assume this is a sequence
        self.movecounter = 0
        self._highest = max(self.board)
        self._opt_cut_off_low_moves = True
        self._lowest_move = max(self.board)/4  # is this ok?  can we set a better bound?  set to zero to disable.
        self._opt_limitnumberofremoves = True  # don't allow a move to take out more than 2 factors
        self._opt_bestmove = True  # use set_solver best move
        self._opt_count_abandoned = True  # count abandoned numbers against the score when considering a move
        self._opt_2tax_penalty = True  # reduce the score given to a move if 2 factors are paid as tax
        self._2tax_penalty = max(self.board)/2
        self.opt_reserved = True

    @staticmethod
    def _remove_abandoned(board):
        """returns abandoned numbers and the board without the abandoned numbers"""
        abandoned = {list(p)[0] for p in factormath.find_partitions(board) if len(p) == 1}
        return abandoned, board - abandoned

    @staticmethod
    def best_move(board):
        # Look for sinks that have a composite that only has one factor (the sink) remaining and is the largest
        # composite for the sink.  Take that composite as the best move.
        for n in board:
            if len(factormath.get_abbreviated_factors(n) & board) == 0:  # sinks
                all_comps = factormath.get_abbreviated_composites(n) & board
                if len(all_comps):
                    comp = max(all_comps)
                    num_factors = len(factormath.get_abbreviated_factors(comp) & board)
                    num_composites = len(factormath.get_abbreviated_composites(comp) & board)
                    if num_factors == 1 and num_composites == 0:
                        # print("last chance: {} is the only factor of {}".format(n, comp))
                        return comp
        return 0

    def _score_move(self, move, board):
        score = float('-inf')
        if self._opt_cut_off_low_moves and move < self._lowest_move:
            return score
        else:
            removed, new_board = self._remove(move, board)
            tax = set(removed) - {move}
            if len(tax) > 0:
                score = move - sum(tax)
                if self._opt_2tax_penalty and len(tax) > 1:
                    score -= self._2tax_penalty
            if self._opt_count_abandoned:
                abandoned, new_board = self._remove_abandoned(new_board)
                score -= sum(abandoned)
                # print("score for {} is {}, tax is {}, abandoned is {}".format(move, score, removed, abandoned))
            return score

    def _remove(self, n, board):
        """returns the numbers removed and the board with the numbers removed"""
        removed, new_board = factormath.remove_factors(n, board)
        # print("removing {}, took out factors {}".format(n, removed))
        if len(removed) < 2:
            return [], board # tax man did not get paid
        if len(removed) > 3 and self._opt_limitnumberofremoves:
            return [], board  # too many numbers removed for this move to be optimal
        return removed, new_board

    def _find_reserved_numbers(self, board):
        reserved = set()
        if self.opt_reserved:
            """
            Reserved numbers are numbers we're holding back to use as tax. If a number has no composites
            and only a single factor, then reserve that factor.
            """
            targets = [x for x in board if len(factormath.get_abbreviated_composites(x) & board) is 0]
            r_size = -1
            while len(reserved) is not r_size:
                r_size = len(reserved)
                for n in targets:
                    factors = (factormath.get_abbreviated_factors(n) & board) - reserved
                    if len(factors) == 1:
                        reserved |= factors
            # print("reserved numbers are {}".format(reserved))
        return reserved

    def rank_moves(self, board):
        if self._opt_bestmove:
            best = G2Solver.best_move(board)
            if best > 0:
                # print("best move: {}".format(best))
                return [best]

        reserved = self._find_reserved_numbers(board)

        scores = [(n, self._score_move(n, board)) for n in board - reserved]
        scores = sorted(scores, key=lambda x: x[1], reverse=True)
        # print("scores: {}".format([s for s in scores if s[1] > -float('inf')]))
        # print("scores: {}".format(scores))
        return [i for i, j in scores if j > float('-inf')]

    def _choose_moves(self):
        """Yields a  move based on a greedy heuristic until no more moves can be made."""
        board = self.board
        while len(board):
            self.movecounter += len(board)
            ranked_moves = self.rank_moves(board)
            if len(ranked_moves):
                best_move = ranked_moves[0]
                removed, board = self._remove(best_move, board)
                if self._opt_count_abandoned:
                    abandoned, board = self._remove_abandoned(board)
                # print("best move is {}, factors are {}, new board is {}".format(best_move, removed, board))
                # print("best move is {}, factors are {}".format(best_move, removed))
                yield best_move
            else:
                break

    def print_internals_report(self):
        print("considered {:,d} moves".format(self.movecounter))

    def play(self):
        """returns a list of moves"""
        return [m for m in self._choose_moves()]


def main():
    print("testing greedy solver")
    print("100: {}".format(GreedySolver(100).play()))


if __name__ == "__main__":
    main()
