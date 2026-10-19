# The golden data

`n<NNNN>.json` is what the Java solver produced for the board of `n`, stage by stage, for checking the port: the frames
as the setup forms them, the numbered and merged frames, the candidates, the reduced frame set, the greedy incumbent,
the prime shortcut and the result of the solve.  There are 318 files, for the games 1 to 760, and
[`../golden_wide`](../golden_wide) holds the structure alone of eleven wider games, 511 to 2048.  The tests
`tests/golden_frames.rs` and `tests/solver.rs` read them; DESIGN.md, section 9 ("Golden tests"), says what they check.

The files are the Java's answers, and nothing makes them again.  The exporter `tools/golden` made them (`export.sh`,
and the Java classes `GoldenExport`, `GameProbe` and the others beside it).  Neither the exporter nor the Java program
is in the tree: the git tag `java-final` keeps both, the script as `rust/tools/golden/export.sh` (the crate was `rust/`
then), so `git show java-final:rust/tools/golden/export.sh` prints it.  `../games_java.txt` is the same kind of record,
from `GameProbe`, for the games of the board's frames beyond 760.
