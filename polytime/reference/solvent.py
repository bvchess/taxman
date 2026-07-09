"""The solvent Taxman strategy, written to be read.

This is the executable version of the pseudocode in README.md
("Solvent, final form"): correct, minimal, and unhurried.  The fast,
bit-identical implementation (incremental matching + Kuhn augmenting
paths) lives in strategies/solvent.py's solvent().

The game, in one breath: the pot holds 1..n; picking a number c keeps
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
  scratch and pays roughly a factor of n for the privilege (~n^3.3
  measured) - it exists to be read, not raced.
* Rejections are proofs, not judgment calls.  When solve_mini fails,
  no reservation scheme exists for that set at all - and since a
  bigger set is only harder to pay for, a rejected number is rejected
  forever.  One consequence: the set this program picks is canonical,
  a deterministic function of the game, not of tie-breaking.
* The picks above n/2 are not merely good - they are exactly the
  >n/2 selections of an optimal game (verified against the known
  optima for every n <= 1000).  Numbers above n/2 never divide one
  another, so up there selection is a pure matching question, and
  the matchable upper sets form a matroid - the structure in which
  a greedy descending pass is provably unbeatable.  Because the loop
  runs from n downward, its first n/2 iterations are literally that
  optimal upper-half computation; whatever solvent loses to the true
  optimum, it loses among the small numbers below n/2.
* The one leap of faith is deferred, not hidden.  Accepting a set
  whenever the reservations exist assumes they can also be scheduled
  (the "schedulability conjecture" - never violated in any game ever
  run).  If it ever fails, ordered() halts the program rather than
  play an illegal game.

Run it with the game size as the only argument:

    $ python3 -m reference.solvent 21
    picks, in playing order: [19, 9, 15, 21, 14, 18, 12, 16, 20]
    score (sum of picks):    144

(144 is the known optimum for n=21.)  Sizes up to a few hundred
answer in seconds; beyond that, use strategies/solvent.py's fast solvent().
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
      and is sequenced FIRST, before any other pick could sweep that
      factor away.
    * A factor that only ONE member can use must go to that member -
      which is sequenced LAST, giving earlier picks time to consume
      its other factors, so this one is what it actually pays.

    When neither rule applies, no assignment of distinct payments
    exists and the set is unplayable - failure here is a theorem
    about the set, not a dead end in a search.

    Returns (sequence, pay) where pay maps each member to its
    reserved payment.
    """
    if not members:
        return [], {}

    for c in members:
        remaining = factors_of[c] & factors
        if len(remaining) == 1:
            (f,) = remaining
            seq, pay = solve_mini(members - {c}, factors - {f}, factors_of)
            pay[c] = f
            return [c] + seq, pay

    for f in factors:
        payers = [c for c in members if f in factors_of[c]]
        if len(payers) == 1:
            c = payers[0]
            seq, pay = solve_mini(members - {c}, factors - {f}, factors_of)
            pay[c] = f
            return seq + [c], pay

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
        _, pay = solve_mini(s_set, factors, factors_of)
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
    cycle and no order exists - the schedulability conjecture says
    solve_mini's reservations never do this, and this loud failure
    is where that trust is checked.
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
                f"schedulability conjecture violated: no valid ordering "
                f"exists for {remaining}"
            )
    return placed


def solvent(n):
    """Play Taxman game n: choose the set first, then order it.

    Descending order is the greedy heart: offer the biggest numbers
    first, keep each one whose addition leaves the set payable.  This
    is the wiki's optimize_mini promoted from the upper half to the
    whole game - restricted to the numbers above n/2 it collapses
    back into optimize_mini exactly.
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
