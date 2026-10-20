#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Runs a command and adds its CPU seconds to a ledger.  Standard library only.

usage: python3 -I ledger.py LEDGER.jsonl LABEL COMMAND [ARGUMENT]...

The command runs as given, with its output on this one's, and its exit status is this one's.  The ledger gets a line
`{"label", "cpu_s", "wall_s", "status"}`: the user and system seconds of the command and every process it started
(`getrusage` of the children), for the work whose CPU no result file records: building, testing, `tune units`, the
landscape's gaps.  The runs of the entry's own files (checkpoints, probes, runs) record their CPU in their lines, and
`tables.py cpu` adds the two together.
"""
import json
import resource
import subprocess
import sys
import time


def main():
    if len(sys.argv) < 4:
        sys.exit(__doc__)
    ledger, label, command = sys.argv[1], sys.argv[2], sys.argv[3:]
    before, began = resource.getrusage(resource.RUSAGE_CHILDREN), time.time()
    status = subprocess.run(command, check=False).returncode
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime
    with open(ledger, "a") as out:
        out.write(json.dumps({"label": label, "cpu_s": round(cpu, 1), "wall_s": round(time.time() - began, 1),
                              "status": status}) + "\n")
    sys.exit(status)


if __name__ == "__main__":
    main()
