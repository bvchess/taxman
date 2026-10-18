# The golden data

`n<NNNN>.json` is what the Java solver produced for the board of `n`, stage by stage, for checking the port: the frames
as the setup forms them, the numbered and merged frames, the candidates, the reduced frame set, the greedy incumbent,
the prime shortcut and the result of the solve.  There are 318 files, for boards from 1 to 760, and
[`../golden_wide`](../golden_wide) holds the structure only of eleven wider boards, 511 to 2048.  The tests
`tests/golden_frames.rs` and `tests/solver.rs` read them; DESIGN.md, section 9 ("Golden tests"), says what they check.

They were made by `tools/golden` (`export.sh` and the Java classes `GoldenExport`, `GameProbe` and the others beside it)
at the git tag `java-final`, where the script is `rust/tools/golden/export.sh` (the crate was `rust/` then) and the Java
program it compiles against is in the tag too.  The exporter and the Java program are no longer in the tree;
`java-final` keeps them: `git show java-final:rust/tools/golden/export.sh`.  The files are the Java's answers and are not
made again; `../games_java.txt` is the same kind of record, from `GameProbe`, for the games of the board's frames
beyond 760.
