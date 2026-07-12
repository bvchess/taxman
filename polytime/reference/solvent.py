"""
This is implementation of the Solvent Taxman strategy.  The code is
written for readability rather than performance. There are many ways
to make it run faster at the cost of making it more complicated.

The game: the pot holds 1..n; picking a number c keeps
c for you and surrenders every divisor of c still in the pot to the
taxman; every pick must surrender at least one divisor, and when no
legal pick remains the taxman sweeps the leftovers.  Highest total
wins.

What the program illustrates: an excellent full game can be played
without ever simulating a turn.  The strategy decides *membership*
first - walk c from n down to 2 and keep c whenever the kept set
stays playable - and only at the very end arranges the chosen set
into a legal sequence.  This works because playability is a property
of the set, not of any particular move order: a set of picks can be
played if and only if every member can reserve its own personal tax
payment (a divisor outside the set, no two members sharing) and the
reservations can be scheduled without stepping on each other.
solve_mini finds the reservations; ordered() does the scheduling.

What makes it special:
* No search, no lookahead, no scoring heuristics - one feasibility
  question ("can everyone still be paid?") asked n times, landing
  within ~0.15% of the known optimal scores for n <= 1000.  Answered
  incrementally (strategies/solvent.py), the whole strategy runs in
  O(n^2 log n); this readable version re-derives each answer from
  scratch and pays roughly a factor of n for the privilege O(~n^3).
* The picks above n/2 are the >n/2 selections of an optimal game,
  at least for every n <= 1000. Whatever solvent loses to the true
  optimum, it loses among the numbers below n/2 where some numbers
  have the potential to be either a selection or tax.
* The scheduling step is guaranteed: the selection proceedure cannot
  emit an assignment whose precedence is cyclic. ordered() still
  checks and halts rather than play an illegal game, but this is a
  safety precaution only.

Run it with the game size as the only argument:

    $ python3 -m reference.solvent 21
    picks, in playing order: [19, 9, 15, 21, 14, 18, 12, 16, 20]
    score (sum of picks):    144

(144 is the known optimum for n=21.)  Sizes up to a few hundred
answer in seconds using the pypy implementation of python.
"""


class Infeasible(Exception):
    """Raised when solve_mini cannot pay every member from the factor pool."""


def maximal_factors(c):
    # The maximal factors of c: every f such that c / f is prime.  A prime's
    # only maximal factor is 1; 1 itself has none.  Found by dividing c by
    # each of its distinct prime factors.
    primes = set()
    r = c
    p = 2
    while p * p <= r:
        while r % p == 0:
            primes.add(p)
            r //= p
        p += 1
    if r > 1:
        primes.add(r)
    return {c // p for p in primes}


def solve_mini(members, factors, factors_of):
    """Reserve a distinct payment ("pay") for every member, or fail.

    This is the wiki's solve_mini: a peeling procedure that only ever
    makes forced moves, never guesses, and never backtracks.  Two
    moves are forced:

    * A member with exactly ONE usable factor left must reserve it -
      no other payment can save that member, and every valid
      assignment makes this exact reservation anyway.
    * A factor that only ONE member can use must go to that member -
      granting it takes nothing away from anyone else.

    When neither rule applies, the set is refused as unplayable.
    Note what this refusal is NOT: it is not always "no assignment of
    distinct payments exists" - some refused sets have perfect
    payment assignments, every one of which is impossible to
    schedule.  The forced-move discipline makes solve_mini reject
    those too: it is a playability oracle, provably exact (THEORY.md),
    strictly stronger than a matching test.

    Returns pay, mapping each member to its reserved payment.  No
    move order is returned: the peel's discovery order is meaningful
    only in the factor game, where a move consumes exactly its
    reserved factor.  Real sweeps take every divisor, maximal or not,
    so a real play order must be built from the full divisibility
    precedence - ordered()'s job, guaranteed to succeed by the
    schedulability theorem.
    """
    if not members:
        return {}

    for c in members:
        remaining = factors_of[c] & factors
        if len(remaining) == 1:
            (f,) = remaining
            pay = solve_mini(members - {c}, factors - {f}, factors_of)
            pay[c] = f
            return pay

    for f in factors:
        payers = [c for c in members if f in factors_of[c]]
        if len(payers) == 1:
            c = payers[0]
            pay = solve_mini(members - {c}, factors - {f}, factors_of)
            pay[c] = f
            return pay

    raise Infeasible(f"cannot select every member of {members} using {factors}")


def playable(s_set):
    """Can this set of picks be a legal game?  The heart of the strategy.

    Reduce the question to a factor game: the candidate picks are the
    members, and the payment pool holds every number OUTSIDE the set
    that is a MAXIMAL factor of a member (an f with member/f prime; a
    number inside the set is a pick, so it cannot double as anyone's tax
    payment).  Maximal factors are the right pool because any surviving
    proper divisor lifts to a surviving maximal factor outside the set,
    so the matching question is unchanged.  The set is accepted exactly
    when solve_mini can reserve a distinct payment for every member.

    Returns the pay reservations on success (evidence, reused by
    ordered()), or None.
    """
    factors = set()
    for s in s_set:
        factors |= maximal_factors(s)
    factors -= s_set

    factors_of = {s: maximal_factors(s) & factors for s in s_set}

    try:
        pay = solve_mini(s_set, factors, factors_of)
    except Infeasible:
        return None
    return pay


def ordered(s_set, pay):
    """Arrange the accepted set into a legal playing order.

    A pick sweeps ALL of its remaining divisors, so the order must
    respect two precedence rules - s comes before t whenever:

    * pay(s) divides t: otherwise t's sweep would destroy s's
      reserved payment before s could pay with it;
    * s divides t: s is itself a divisor of t, so s must be picked
      before t's sweep removes s from the pot entirely.

    Any order respecting both is legal.  A repeated scan is all it
    takes: place a member a as soon as no still-waiting member b has
    to precede it (no b with pay(b) dividing a, no b dividing a).
    If every remaining member is blocked, the constraints form a
    cycle and no order exists - the schedulability theorem says
    solve_mini's reservations never do this, so the loud failure
    below is an assertion of a theorem, reachable only by a bug.
    """
    remaining = set(s_set)
    placed = []
    while remaining:
        for a in remaining:
            blocked = any(
                b != a
                and ((pay.get(b) is not None and a % pay[b] == 0) or a % b == 0)
                for b in remaining
            )
            if not blocked:
                placed.append(a)
                remaining.discard(a)
                break
        else:
            raise RuntimeError(
                f"schedulability theorem violated (impossible unless "
                f"there is a bug): no valid ordering exists for {remaining}"
            )
    return placed


def solvent(n):
    """Play Taxman game n: choose the set of picks first, then order
    them.

    Select the biggest numbers first, keep each one whose addition
    leaves the set playable.
    """
    s = set()
    for c in range(n, 1, -1):
        if playable(s | {c}) is not None:
            s.add(c)
    pay = playable(s)
    return ordered(s, pay)


if __name__ == "__main__":
    import sys

    if len(sys.argv) != 2 or not sys.argv[1].isdigit():
        sys.exit(f"usage: python3 {sys.argv[0]} N   (the game size, e.g. 21)")

    n = int(sys.argv[1])
    # solve_mini peels one selection per recursive call, so the deepest
    # recursion needs one stack frame per selection - at most n.  Grant
    # that (plus headroom), never going below Python's default of 1000.
    sys.setrecursionlimit(max(1000, n + 100))
    result = solvent(n)
    print("picks, in playing order:", result)
    print("score (sum of picks):   ", sum(result))
