# The golden data

`n<NNNN>.json` is what the Java solver produced for the board of `n`, stage by stage, for checking the port: the frames
as the setup forms them, the numbered and merged frames, the candidates, the reduced frame set, the greedy incumbent,
the prime shortcut and the result of the solve.  There are 318 files, for the games 1 to 760, and
[`../golden_wide`](../golden_wide) holds the structure alone of eleven wider games, 511 to 2048.  The test
`tests/golden_frames.rs` and the unit tests of `src/` (which `src/model/test_support.rs` loads the files for) read them;
the item "Golden tests" of [the design's catalog of tests](../../../docs/design/tests.md) says what they check.

The files are the Java's answers, and nothing makes them again.  On eight small boards setup departs from them, where
the Java's claims are not those of the theory's root problem: the test lists the departures (`DEPARTURES`).
`../games_java.txt` is the same kind of record, for the games of the board's frames beyond 760.
