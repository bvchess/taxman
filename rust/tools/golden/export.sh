#!/bin/bash
#
# Export golden data from the Java Taxman solver, for checking the Rust port stage by stage.
#
#   export.sh [--structure-only] [--skip-existing] OUTDIR N-or-range [N-or-range ...]
#
# A range is A-B, or A-B/STEP.  Writes OUTDIR/nNNNN.json for each n.  See GoldenExport.java for the schema.
# Compiles the exporter against bin/taxman.jar into tools/golden/build, then runs it.

set -euo pipefail

if [ $# -lt 2 ]; then
    echo "usage: export.sh [--structure-only] [--skip-existing] OUTDIR N-or-A-B[/STEP] [...]" >&2
    exit 2
fi

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
REPO="$( cd "$DIR/../../.." >/dev/null 2>&1 && pwd )"
JAR="$REPO/bin/taxman.jar"
CLASSES="$DIR/build/classes"

mkdir -p "$CLASSES"
find "$DIR/org" -name '*.java' -print0 | xargs -0 javac -cp "$JAR" -d "$CLASSES"

exec java -ea -XX:MaxRAMPercentage=60 -cp "$CLASSES:$JAR" org.taxman.h6.search.GoldenExport "$@"
