import factormath
from cache import Cache
from result import Result


# 15M is a good limit for my mac with 16GB RAM.  40M seems to be a better number for an AWS linux box with 32GB RAM
_result_cache = Cache(15000000)


def print_extended_cache_report():
    if _result_cache and _result_cache.size():
        print("extended cache stats:")
        print(_result_cache.extended_stats())


def clear_cache():
    _result_cache.clear()


class SetSolver(object):
    def __init__(self, top_number):
        self._board = frozenset(range(1, top_number+1))
        self._highest = top_number
        self._result = Result(0, [])

        # For small boards you can play a perfect game: only take numbers in the top half of the choices.
        # As boards get larger, it seems you sometimes have to dig deeper into the numbers.  I haven't seen a case
        # where you have to go into the bottom quarter, but without proof of that, it's a risky optimization.
        self._lowest_move = top_number/4
        self._debug_print_level = 0


    @staticmethod
    def best_move(board):
        # Look for sinks that have a composite that only has one factor (the sink) remaining and is the largest
        # composite for the sink.  Take that composite as the best move.  This is a generalization of the
        # "take the largest prime" opening move.  Since the factor has no factors of its own, it must be paid as
        # tax.  If the largest composite doesn't have any composites (for example if it is the largest prime), then
        # if something else pays the factor as tax, then the composite will be abandoned.  Since the composite is
        # the largest associated with the factor, it will yield the most points.
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

    def _make_candidate_list(self, board, level):
        """Return numbers that can should be used for branching in an exhaustive search"""
        # Reserved numbers are numbers we're holding back to use as tax. If a number has no composites
        # and only a single factor, then reserve it.
        reserved = dict()
        no_composites = [x for x in board if len(factormath.get_abbreviated_composites(x) & board) is 0]
        r_size = -1
        while len(reserved) is not r_size:
            r_size = len(reserved)
            for n in sorted(no_composites, reverse=True):
                factors = factormath.get_abbreviated_factors(n) & board - set(reserved.values())
                if len(factors) == 1:
                    reserved[n] = list(factors)[0]
        # print("reservations: {}".format(reserved))

        # A candidate move must pay at least one but not more than two factors of tax,
        # must not be a reserved factor, and must not take a reserved factor not assigned to it.
        reserved_values = set(reserved.values())
        candidates = [x for x in board if x >= self._lowest_move]  # you must be this tall to ride
        candidates = [x for x in candidates if len(factormath.get_abbreviated_factors(x) & board)]  # must pay tax
        candidates = set(candidates) - reserved_values
        c2 = list()
        for n in sorted(candidates):
            factors = factormath.get_abbreviated_factors(n) & board
            if len(factors) == 1:
                removed, remaining = factormath.remove_factors(n, board)
                if len(removed) > 2:
                    continue  # don't allow taking a composite
            elif len(factors) == 2:
                removed, remaining = factormath.remove_factors(n, board)
                if len(removed) > 3:
                    continue  # don't allow taking a composite
            else:
                continue  # don't allow taking more than 2 factors

            reserved_for_n = reserved.get(n, 0)
            reserved_intersect = removed.intersection(reserved_values)
            if len(reserved_intersect) > 0:
                if len(reserved_intersect) > 1:
                    # print("skipping {} because it took reserved values {}".format(n, reserved_intersect))
                    continue
                else:  # took exactly 1 reserved value
                    reserved_taken = list(reserved_intersect)[0]
                    if reserved_for_n is not reserved_taken:
                        # print("skipping {} because it took {}, a value not reserved for it".format(n, reserved_taken))
                        continue
            c2.append(n)

        return c2

    def _play_exhaustive(self, board, level):
        # print("\n_play_exhaustive beginning for {}".format(sorted(board, reverse=True)))
        best_result = Result(float('-inf'), [])
        candidate_moves = self._make_candidate_list(board, level)
        if self._debug_print_level and self._debug_print_level >= level and len(candidate_moves) > 1:
            fmt = "{}level {} candidates are: {} with board size {}"
            print(fmt.format(" "*level, level, candidate_moves, len(board)))

        next_level = level + 1 if len(candidate_moves) > 1 else level
        for n in candidate_moves:
            removed, remaining = factormath.remove_factors(n, board)
            remaining_result = self._play(remaining, do_partition=True, level=next_level)
            result = Result(2 * n - sum(removed), [n]).extend(remaining_result)
            if self._debug_print_level and self._debug_print_level >= level and len(candidate_moves) > 1:
                print("\t{}score for {} is {}".format("  "*level, n, result.get_score()))
            if result.get_score() > best_result.get_score():
                best_result = result

        return best_result

    @staticmethod
    def _get_from_cache(board):
        cached_result = None
        if _result_cache:
            cached_result = _result_cache.get(board)
        return cached_result

    @staticmethod
    def _write_to_cache(board, result):
        if _result_cache:
            _result_cache.set(board, result)

    def _play(self, board, do_partition, level):
        """returns a Result object with an optimal set of moves"""

        if len(board) < 2:
            return Result(-sum(board), [])

        # print("playing board {}".format(board))
        cached = self._get_from_cache(board)
        if cached:
            return cached

        best = self.best_move(board)
        if best:
            # print("best move is {} ".format(best))
            # print("best move is {} for board {}".format(best, board))
            removed, remaining = factormath.remove_factors(best, board)
            remaining_result = self._play(remaining, do_partition=True, level=level)
            result = Result(best * 2 - sum(removed), [best]).extend(remaining_result)
        else:
            partitions = factormath.find_partitions(board) if do_partition else []
            if len(partitions) > 1:
                # print("found {} partitions: {}".format(len(partitions), partitions))
                result = Result(0, [])
                for p in partitions:
                    result = result.extend(self._play(p, do_partition=False, level=level))
            else:
                result = self._play_exhaustive(board, level)

        self._write_to_cache(board, result)
        return result

    def play(self):
        _result_cache.reset_tripmeter()
        self._result = self._play(self._board, do_partition=True, level=1)
        return self._result.get_moves()

    def print_internals_report(self):
        from optimal_deltas import optimal_win_by
        known_answer = optimal_win_by.get(self._highest, None)
        if known_answer is not None and not known_answer == self._result.get_score():
            fmt = "PROBLEM for board {}: score should be {} but came out {}"
            print(fmt.format(self._highest, known_answer, self._result.get_score()))
        if _result_cache:
            print(_result_cache.stats())
