#!/usr/bin/env python3
"""usage: run.py GAMEDIR SETFILE IDFILE NODELIMIT [P]

Solves each unit of IDFILE under each setting of SETFILE (lines "NAME ARGS..."; the token STORE is replaced by
GAMEDIR/store-GAME.txt), P at once (default 6) at nice 19, by tune solve.  The settings are interleaved unit by unit,
the units in the order of IDFILE.  Resumable: a finished GAMEDIR/out/NAME-ID.json is skipped.  Environment variables of
a setting go before its name's arguments as ENV=VALUE tokens (e.g. "nl600 TAXMAN_CH_NOLEARN=1 --gate 600 ...")."""
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

TUNE_DEFAULT = "/home/brian/taxman_work/lab/work/gate-2026-10-28/tune"
KNOWN = "/home/brian/taxman_work/research/results/strict.jsonl"


def main():
    gamedir, setfile, idfile, limit = sys.argv[1:5]
    par = int(sys.argv[5]) if len(sys.argv) > 5 else 6
    game = os.path.basename(gamedir.rstrip("/"))
    store = f"{gamedir}/store-{game}.txt"
    units = f"{gamedir}/units-{game}.jsonl"
    out = f"{gamedir}/out"
    os.makedirs(out, exist_ok=True)
    sets = []
    for line in open(setfile):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        name, *args = line.split()
        tune = TUNE_DEFAULT
        env = {}
        rest = []
        for a in args:
            if a.startswith("BIN="):
                tune = a[4:]
            elif "=" in a and a.split("=")[0].isupper() and not a.startswith("--"):
                env[a.split("=")[0]] = a.split("=", 1)[1]
            else:
                rest.append(store if a == "STORE" else a)
        sets.append((name, tune, env, rest))
    ids = open(idfile).read().split()
    jobs = [(i, s) for i in ids for s in sets]

    def one(job):
        i, (name, tune, env, rest) = job
        res = f"{out}/{name}-{i}.json"
        if os.path.exists(res) and os.path.getsize(res) > 0:
            return
        cmd = ["nice", "-n", "19", tune, "solve", game, "--known", KNOWN, "--units", units, "--ids", i, *rest,
               "--node-limit", limit]
        with open(f"{out}/{name}-{i}.tmp", "w") as fo, open(f"{out}/{name}-{i}.err", "w") as fe:
            rc = subprocess.call(cmd, stdout=fo, stderr=fe, env={**os.environ, **env})
        if rc == 0:
            os.replace(f"{out}/{name}-{i}.tmp", res)

    with ThreadPoolExecutor(par) as ex:
        list(ex.map(one, jobs))


if __name__ == "__main__":
    main()
