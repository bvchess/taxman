import factormath


class GreedySolver(object):
    def __init__(self, x):
        if isinstance(x, int):
            self.board = frozenset(range(1, x + 1))
        else:
            self.board = frozenset(x)  # assume this is a sequence
        self.movecounter = 0
        self._highest = max(self.board)
        self._opt_count_abandoned = True  # count abandoned numbers against the score when considering a move

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
        removed, new_board = self._remove(move, board)
        tax = set(removed) - {move}
        if len(tax) > 0:
            score = move - sum(tax)
        if self._opt_count_abandoned:
            abandoned, new_board = self._remove_abandoned(new_board)
            score -= sum(abandoned)
            # print("score for {} is {}, tax is {}, abandoned is {}".format(move, score, removed, abandoned))
        return score

    @staticmethod
    def _remove(n, board):
        """returns the numbers removed and the board with the numbers removed"""
        removed, new_board = factormath.remove_factors(n, board)
        # print("removing {}, took out factors {}".format(n, removed))
        if len(removed) < 2:
            return [], board  # tax man did not get paid
        return removed, new_board

    def rank_moves(self, board):
        scores = [(n, self._score_move(n, board)) for n in board]
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
