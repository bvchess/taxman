import math

_maximum = 0
_primes = set()
_factors = dict()
_abbreviated_factors = dict()  # factors of a number that are not also factors of the numbers other factors
_composites = dict()
_abbreviated_composites = dict()


def initialize(max_num):
    import math
    global _maximum, _primes, _factors, _abbreviated_factors, _composites, _abbreviated_composites
    _maximum = max_num
    _factors = {x: set() for x in range(1, _maximum + 1)}
    _abbreviated_factors = {x: set() for x in range(1, _maximum + 1)}
    _composites = {x: set() for x in range(1, _maximum + 1)}
    _abbreviated_composites = {x: set() for x in range(1, _maximum + 1)}
    for n in range(1, _maximum + 1):
        for x in range(1, int(math.sqrt(n))+1):
            if n % x == 0:
                f1 = x
                f2 = n/x
                _factors[n].add(f1)
                _factors[n].add(f2)
                if f1 > 1:
                    _composites[f1].add(n)
                if f2 != n:
                    _composites[f2].add(n)

    _primes = frozenset(k for k in _factors.keys() if k > 1 and len(_factors[k]) == 2)

    for n in range(1, _maximum + 1):
        abbrev = _factors[n] - {y for x in _factors[n] - {n} for y in _factors[x] - {x}} - {n}
        _abbreviated_factors[n] = abbrev
        for x in abbrev:
            _abbreviated_composites[x].add(n)


def _format_sequence(seq):
    return ", ".join(map(str, sorted(seq)))


def find_gcd(number_sequence):
    if len(number_sequence) < 2:
        return 0
    else:
        numbers = sorted(list(number_sequence))
        first = numbers[0]
        candidates = set(_factors[first])
        candidates.add(first)
        for n in numbers[1:]:
            candidates.intersection_update(_factors[n])
            if len(candidates) == 0:
                break
        if len(candidates) == 0:
            return 0
        else:
            return max(candidates)


def find_primes(number_sequence):
    return frozenset(number_sequence) & _primes


def get_factors(n):
    return _factors[n]


def get_abbreviated_factors(n):
    return _abbreviated_factors[n]


def get_composites(n):
    """returns the set of composite numbers for n that are less than the maximum"""
    return _composites[n]


def get_abbreviated_composites(n):
    return _abbreviated_composites[n]


def is_prime(n):
    return n in _primes


def get_primes():
    return _primes


def find_partitions(numbers):
    """Find partitions.  Uses abbreviated factors, so does not include 1, which means won't treat primes properly."""
    already_seen = set()
    partitions = list()
    for n in numbers:
        if n not in already_seen:
            partition = [n]
            already_seen.add(n)
            partitions.append(partition)
            queue = list(get_abbreviated_factors(n) & numbers) + list(get_abbreviated_composites(n) & numbers)
            while len(queue):
                m = queue.pop()
                if m not in already_seen:
                    already_seen.add(m)
                    partition.append(m)
                    queue += get_abbreviated_factors(m) & numbers
                    queue += get_abbreviated_composites(m) & numbers
    result = [frozenset(p) for p in partitions]
    # print("{} partitions in {}: {}".format(len(result), numbers, result))
    return result


def ideal_score(numbers):
    """returns the number of points the player will win by in the ideal game."""
    size = len(numbers)
    s = sorted(list(numbers))
    top_half = int(math.ceil(size/2.0))
    small = s[:top_half]
    large = s[top_half:]
    # print("ideal score for {} is {} - {} = {}".format(s, large, small, sum(large)-sum(small)))
    return sum(large)-sum(small)


def ideal_score_accounting_for_primes(highest):
    board = frozenset(range(1, highest+1))
    primes = find_primes(board)
    board_after_first_move = board
    if len(primes):
        board_after_first_move = (board - primes) | {max(primes)}
    return ideal_score(board_after_first_move)


def remove_factors(number, number_sequence):
    """Remove number and its factors from number_sequence.  Return the removed numbers and the remaining sequence."""
    full_set = frozenset(number_sequence)
    to_remove = get_factors(number)
    to_remove.add(number)
    intersect = full_set & to_remove
    remainder = full_set - intersect
    return intersect, remainder


def debug_dump():
    print("maximum number: {}".format(_maximum))
    print("{} factors".format(sum([len(x) for x in _factors.values()])))
    for n in range(1, _maximum + 1):
        print("  {}: {}".format(n, _format_sequence(get_factors(n))))
    print("")
    print("{} abbreviated factors".format(sum([len(x) for x in _abbreviated_factors.values()])))
    for n in range(1, _maximum + 1):
        if len(_abbreviated_factors[n]):
            print("  {}: {}".format(n, _format_sequence(_abbreviated_factors[n])))
    print("")
    print("composites")
    for n in range(1, _maximum + 1):
        if len(_composites[n]):
            print("  {}: {}".format(n, _format_sequence(_composites[n])))
    print("")
    print("abbreviated composites")
    for n in range(1, _maximum + 1):
        if len(_composites[n]):
            print("  {}: {}".format(n, _format_sequence(_abbreviated_composites[n])))
    print("")
    print("primes")
    print("  {}".format(_format_sequence(_primes)))


def _gcd_test(seq):
    print("gcd of {} is {}".format(_format_sequence(seq), find_gcd(seq)))


def find_primes_test(seq):
    print("primes in {} are {}".format(_format_sequence(seq), _format_sequence(find_primes(seq))))


def main():
    print("testing factormath")
    top = 122
    initialize(top)
    debug_dump()
    print("")
    _gcd_test([4, 8, 10])
    _gcd_test([5, 10])
    _gcd_test([2, 4, 8])
    _gcd_test([2, 4, 5, 8])
    _gcd_test([4, 8])
    print("")
    find_primes_test(range(1, top+1))
    print("")
    initialize(64)
    _gcd_test([64, 32, 4, 16, 8])


if __name__ == "__main__":
    main()
