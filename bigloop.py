import set_solver
import greedy
import factormath
import sys
import time
from optimal_deltas import optimal_win_by


def _compute_delta(board, solver):
    """return the number of points the player wins by for the given solver"""
    moves = solver.play()
    points = sum(moves)
    return 2*points - sum(board)


def convert_delta_to_score(total, delta):
    return (total - delta) / 2 + delta


def as_percent(n1, n2):
    if n2 is 0:
        return 100.0
    else:
        return 100.0*n1/n2


def print_result(n, *numbers):
    strings = map(lambda x: "{:6.2f}%".format(x), numbers)
    print("{:3}, {}".format(n, ", ".join(strings)))
    sys.stdout.flush()


def get_optimal_scores():
    highest = 200
    factormath.initialize(highest)
    for n in range(1, highest + 1):
        board = range(1, n + 1)
        optimal_score = _compute_delta(board, set_solver.SetSolver(n))
        print_result(n, optimal_score)


def just_greedy():
    highest = 1000
    factormath.initialize(highest)
    print("n, greedy %, greedy time, g2 %, g2 time")
    for n in range(1, highest + 1):
        board = range(1, n + 1)

        start_time = time.time()
        basic_greedy_score = sum(greedy.GreedySolver(n, optimize=False).play())
        basic_greedy_time = time.time() - start_time
        basic_percent = as_percent(basic_greedy_score, sum(board))

        start_time = time.time()
        g2_score = sum(greedy.GreedySolver(n, optimize=True).play())
        g2_time = time.time() - start_time
        g2_percent = as_percent(g2_score, sum(board))

        print("{}, {:6.2f}%, {:6.2f}, {:6.2f}%, {:6.2f}".format(n, basic_percent, basic_greedy_time, g2_percent, g2_time))
        sys.stdout.flush()


def mean(numbers):
    return sum(numbers) / len(numbers)


def compare_solvers():
    highest = max(optimal_win_by.keys())
    factormath.initialize(highest)

    print("N, optimal/total, greedy/total")
    for n in range(1, highest + 1):
        board = range(1, n + 1)
        total_points_available = sum(board)
        optimal_delta = optimal_win_by[n] # score(board, set_solver.SetSolver(n))
        optimal_score = convert_delta_to_score(total_points_available, optimal_delta)
        basic_greedy_score = sum(greedy.GreedySolver(n, optimize=False).play())
        best_greedy_score = sum(greedy.GreedySolver(n, optimize=True).play())
        print_result(
            n,
            as_percent(optimal_score, total_points_available),
            as_percent(basic_greedy_score, total_points_available),
            as_percent(best_greedy_score, total_points_available)
        )
    # print("")
    # print("basic greedy average: {:,.1f}%".format(mean(basic_greedy_percentages)))
    # print("optimized greedy average: {:,.1f}%".format(mean(optimized_greedy_percentages)))


def main():
    # compare_solvers()
    just_greedy()


if __name__ == "__main__":
    main()
