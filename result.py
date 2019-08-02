class Result(object):
    __slots__ = "_score", "_moves"

    def __init__(self, score, moves):
        self._score = score
        self._moves = tuple(moves)

    def extend(self, other):
        # assert not len(set(self._moves).intersection(set(other.get_moves())))
        return Result(self.get_score() + other.get_score(),
                       self.get_moves() + other.get_moves())

    def get_moves(self):
        return self._moves

    def get_score(self):
        return self._score

    def __str__(self):
        return str(list(self._moves))