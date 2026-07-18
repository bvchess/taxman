# Copyright (c) Brian Chess 2026
# SPDX-License-Identifier: MIT

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
of the set, not of a specific move order: a set of picks can be
played if and only if every member can reserve its own personal tax
payment (a divisor outside the set, no two members sharing) and the
reservations can be scheduled without stepping on each other.
peel finds the reservations; ordered() does the scheduling.

What makes it special:
* No search, no lookahead, no scoring heuristics - one feasibility
  question ("can everyone still be paid?") asked n times, landing
  within ~0.15% of the known optimal scores for n <= 1000.  Answered
  incrementally (strategies/solvent.py), the strategy runs in
  O(n^3) time.
* The picks above n/2 are the >n/2 selections of an optimal game,
  at least for every n <= 1000. Whatever solvent loses to the true
  optimum, it loses among the numbers below n/2 where some numbers
  have the potential to be either a selection or tax.
* The scheduling step is guaranteed: the selection procedure cannot
  emit an assignment whose precedence is cyclic. ordered() still
  checks and halts rather than play an illegal game, but this is a
  safety precaution only.

Run it with the game size as the only argument:

    $ python3 -m reference.solvent 21
    picks, in playing order: [19, 9, 15, 21, 14, 18, 12, 16, 20]
    score (sum of picks):    144

(144 is the known optimum for n=21.)  Sizes up to a few hundred
answer in seconds.
"""


class Infeasible(Exception):
    """Raised when peel cannot pay every member from the factor pool."""


# Populated once by solvent() at startup: all_maximal_factors[c] is the set of
# maximal factors of c, for every c in 0..n.  Global so the playability test can
# read it without threading it through every call.
all_maximal_factors = []


def build_maximal_factor_table(n):
    """Precompute the maximal factors of every number in 1..n.

    The maximal factors of c are the numbers f with c / f prime - c with
    a single prime stripped out.  A prime's only maximal factor is 1; 1
    itself has none.  One sieve pass fills the whole table: walk the
    primes p in increasing order (p is prime exactly when nothing has
    recorded a factor for it yet), and for every multiple m of p add the
    maximal factor m // p, since m / (m // p) == p is prime.

    Args:
        n: The game size; the table covers indices 0..n inclusive.

    Returns:
        A list `all_maximal_factors` of length n + 1 where all_maximal_factors[c] is the set of
        maximal factors of c (all_maximal_factors[0] and all_maximal_factors[1] are empty).
    """
    global all_maximal_factors

    all_maximal_factors = [set() for _ in range(n + 1)]
    for p in range(2, n + 1):
        if all_maximal_factors[p]:
            continue  # composite: a smaller prime already recorded a factor
        for m in range(p, n + 1, p):
            all_maximal_factors[m].add(m // p)  # m / (m // p) == p, prime


def peel(members, available_as_tax):
    """Reserve a distinct payment ("pay") for every member, or fail.

    A peeling procedure that only ever makes forced moves, never
    guesses, and never backtracks.
    Two moves are forced:
    * A member with exactly ONE usable factor left must reserve it -
      no other payment can save that member, and every valid
      assignment makes this exact reservation anyway.
    * A factor that only ONE member can use must go to that member -
      granting it takes nothing away from anyone else.

    When neither rule applies, the set is refused as unplayable.
    Note what this refusal is NOT: it is not always "no assignment of
    distinct payments exists" - some refused sets have perfect
    payment assignments, every one of which is impossible to
    schedule.  The forced-move discipline makes peel reject
    those too: it is a playability oracle, provably exact (THEORY.md),
    strictly stronger than a matching test.

    Returns pay, mapping each member to its reserved payment.  No
    move order is returned: the peel's discovery order is meaningful
    only in the factor game, where a move consumes exactly its
    reserved factor.  Real sweeps take every divisor, maximal or not,
    so a real play order must be built from the full divisibility
    precedence - ordered()'s job, guaranteed to succeed by the
    schedulability theorem.  A member's usable factors are read from
    the module-level all_maximal_factors table (its maximal factors
    that still lie in available_as_tax).

    Args:
        members: The picks that each need a distinct reserved payment.
        available_as_tax: The pool of available payments - maximal factors
            outside the pick set - not yet reserved.

    Returns:
        A dict `pay` mapping each member to its reserved payment.

    Raises:
        Infeasible: when a state is reached with no forced move - i.e.
            the set is unplayable.
    """
    if not members:
        return {}

    for c in members:
        remaining = all_maximal_factors[c] & available_as_tax
        if len(remaining) == 1:
            (f,) = remaining
            pay = peel(members - {c}, available_as_tax - {f})
            pay[c] = f
            return pay

    for f in available_as_tax:
        payers = [c for c in members if f in all_maximal_factors[c]]
        if len(payers) == 1:
            c = payers[0]
            pay = peel(members - {c}, available_as_tax - {f})
            pay[c] = f
            return pay

    raise Infeasible(f"cannot select every member of {members} using {available_as_tax}")


def playable(pick_set):
    """Can this set of picks be a legal game?  The heart of the strategy.

    Reduce the question to a factor game: the candidate picks are the
    members, and the payment pool holds every number OUTSIDE the set
    that is a MAXIMAL factor of a member (an f with member/f prime; a
    number inside the set is a pick, so it cannot double as anyone's tax
    payment).  Maximal factors are the right pool because any surviving
    proper divisor lifts to a surviving maximal factor outside the set,
    so the matching question is unchanged.  The set is accepted exactly
    when peel can reserve a distinct payment for every member.

    Returns the pay reservations on success (evidence, reused by
    ordered()), or None.

    Args:
        pick_set: The candidate set of picks to test.

    Returns:
        The `pay` reservations (a dict) when the set is playable, else
        None.
    """
    available_as_tax = set()
    for s in pick_set:
        available_as_tax |= all_maximal_factors[s]
    available_as_tax -= pick_set

    try:
        pay = peel(pick_set, available_as_tax)
    except Infeasible:
        return None
    return pay


def ordered(pay):
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
    peel's reservations never do this, so the loud failure
    below is an assertion of a theorem, reachable only by a bug.

    Args:
        pay: The payment reservations from playable / peel, mapping
            each member to its reserved maximal factor.

    Returns:
        A list of the picks in a legal playing order.

    Raises:
        RuntimeError: if no legal order exists - impossible unless a bug
            leads to the set of picks being unplayable
    """
    remaining = set(pay.keys())
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

    Args:
        n: The game size; the pot holds 1..n.

    Returns:
        The chosen picks as a list in legal playing order.
    """
    build_maximal_factor_table(n)
    pick_set = set()  # collect the numbers we pick
    for c in range(n, 1, -1):
        if playable(pick_set | {c}) is not None:
            pick_set.add(c)
    return ordered(playable(pick_set))


if __name__ == "__main__":
    import sys

    if len(sys.argv) != 2 or not sys.argv[1].isdigit():
        sys.exit(f"usage: python3 {sys.argv[0]} N   (the game size, e.g. 21)")

    game_number = int(sys.argv[1])
    # peel makes one selection per recursive call, so the deepest
    # recursion needs one stack frame per selection - at most n.  Grant
    # that (plus headroom), never going below Python's default of 1000.
    sys.setrecursionlimit(max(1000, game_number + 100))
    result = solvent(game_number)
    print("picks, in playing order:", result)
    print("score (sum of picks):   ", sum(result))
