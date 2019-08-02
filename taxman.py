import sys
import factormath
import time
from set_solver import SetSolver
from set_solver import print_extended_cache_report
from greedy import GreedySolver
from g2 import G2Solver


def print_game(board, moves):
    def make_str(seq):
        return ", ".join(map(str, seq))

    print("  game:")
    remaining = board
    for m in moves:
        removed, remaining = factormath.remove_factors(m, remaining)
        removed = removed - {m}
        points = m - sum(removed)
        fmt = "\ttake {:2}, tax man takes {}\t({} points)"
        print(fmt.format(m, make_str(removed), points))
    print("\ttax man takes remainder: {}\t({} points)".format(make_str(remaining), -sum(remaining)))


def check_moves(top, moves):
    remaining = range(1, top+1)
    for m in moves:
        if m not in remaining:
            print("ERROR: illegal move, {} is not on the board".format(m))
            return
        removed, remaining = factormath.remove_factors(m, remaining)
        if len(removed) < 2:
            print("ERROR: illegal move, {} cannot be taken because it has no factors on the board".format(m))
            return


def play_one_game(n, solver_name):
    print("playing {} with {} solver".format(n, solver_name))

    start_time = time.time()
    solver = solvers[solver_name](n)
    solution = solver.play()
    solve_time = time.time() - start_time

    board = range(1, n+1)
    score = sum(solution)
    taxman_score = sum(x for x in board) - score
    win_by = score - taxman_score
    pot_total = sum(board)
    percent_of_total = 100.0*score/pot_total
    fmt = "player score: {:7,d}\n     tax man: {:7,d}\nwin by {:,d} with {:,.1f}% of the pot"
    print(fmt.format(score, taxman_score, win_by, percent_of_total))
    print("solution is {}".format(solution))

    if len(solution):
        lowest_play = min(solution)
        lowest_as_percent = 100.0*lowest_play/n
        print("lowest number taken: {} ({:,.2f}%)".format(lowest_play, lowest_as_percent))
    check_moves(n, solution)
    # print_game(board, solution)
    solver.print_internals_report()
    if solvers[solver_name] is SetSolver:
        greedy_score = sum(GreedySolver(n).play())
        taxman_for_greedy = sum(x for x in range(1, n + 1)) - greedy_score
        if score < greedy_score:
            print("PROBLEM: won by {} but g2 won by {}".format(score-taxman_score, greedy_score-taxman_for_greedy))

    print("solved in {:.2f} seconds".format(solve_time))
    sys.stdout.flush()


solvers = {
    "set": SetSolver,   # the default
    "g2": G2Solver,
    "greedy": GreedySolver
}


def main():
    show_usage_and_exit = False
    top, bottom = 0, 0
    solver_name = "set"
    if len(sys.argv) > 1:
        arg = sys.argv[1]
        bottom_top = arg.split("-")
        if len(bottom_top) is 1:
            bottom = int(bottom_top[0])
            top = bottom
        elif len(bottom_top) is 2:
            bottom = int(bottom_top[0])
            top = int(bottom_top[1])
        else:
            show_usage_and_exit = True
        if len(sys.argv) > 2:
            solver_name = sys.argv[2]

    if solver_name not in solvers:
        print("'{}' is not a supported solver.  Supported solvers are {}".format(solver_name, solvers.keys()))
        return

    if show_usage_and_exit:
        print("Specify the top number on the command line, or give a range such as 2-10 to solve multiple boards.")
        return

    if bottom > top:
        top, bottom = bottom, top
    factormath.initialize(top)

    for n in range(bottom, top+1):
        play_one_game(n, solver_name)
        if bottom != top:
            print("")

    if bottom != top:
        print_extended_cache_report()


if __name__ == "__main__":
    main()
