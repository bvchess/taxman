#!/usr/bin/env python3
"""Solves units under settings, one thread each, at nice 19, pinned to this study's logical CPUs.

Usage: run.py solo SETFILE IDS OUT NL    (each unit under each setting, one solve a CPU, CPUs 30-33 and 74-77, or
                                          the first SLOTS of them)
       run.py pair SETFILE IDS OUT NL    (the two settings of SETFILE on the two hyperthreads of one core, 4 cores at a
                                          time, the sides swapped on alternate units)

A line of SETFILE: NAME BINARY ENV ARGS..., ENV a comma-separated list of VAR=VALUE or "-"; ARGS are added to the
chain's tactics.  IDS holds unit ids (GAME-INDEX), one a line; the game picks the units file and the store.  One file a
solve, OUT/NAME-ID.json (stderr in .err); resumable (a finished solve is skipped)."""
import os
import queue
import subprocess
import sys
import threading

HERE = '/home/brian/taxman_work/lab/work/choice-2026-10-28'
KNOWN = f'{HERE}/known.jsonl'
GAMES = {
    '4984': ('/home/brian/taxman_work/lab/work/patch/units-4984.jsonl',
             '/home/brian/taxman_work/lab/work/carry/s1/store-4983-144.txt'),
    '4995': ('/home/brian/taxman_work/lab/work/patch4995/units-4995.jsonl', f'{HERE}/store-4992.txt'),
    '5052': (f'{HERE}/units-5052.jsonl', f'{HERE}/store-5052.txt'),
}
CHAIN = ['--tactic', 'conflict_branching=on', '--tactic', 'price_reuse=on', '--tactic', 'warm_start=on',
         '--tactic', 'carried_nogoods=on']
CORES = [30, 31, 32, 33]
SIBLING = 44


def settings(path):
    out = []
    for line in open(path):
        parts = line.split()
        if not parts or parts[0].startswith('#'):
            continue
        name, binary, env = parts[0], parts[1], parts[2]
        extra = {} if env == '-' else dict(kv.split('=', 1) for kv in env.split(','))
        out.append((name, binary, extra, parts[3:]))
    return out


def solve(cpu, setting, uid, out, nl):
    name, binary, extra, args = setting
    f = os.path.join(out, f'{name}-{uid}')
    if os.path.exists(f + '.json') and os.path.getsize(f + '.json') > 0:
        return
    game = uid.split('-')[0]
    units, store = GAMES[game]
    cmd = ['nice', '-n', '19', 'taskset', '-c', str(cpu), binary, 'solve', game, '--known', KNOWN, '--units', units,
           '--ids', uid, *CHAIN, '--carried', store, '--node-limit', str(nl), *args]
    env = dict(os.environ, **extra)
    with open(f + '.tmp', 'w') as o, open(f + '.err', 'w') as e:
        r = subprocess.run(cmd, stdout=o, stderr=e, env=env)
    if r.returncode == 0:
        os.replace(f + '.tmp', f + '.json')


def main():
    mode, setfile, ids, out, nl = sys.argv[1:6]
    os.makedirs(out, exist_ok=True)
    sets = settings(setfile)
    units = [line.split()[0] for line in open(ids) if line.strip()]
    jobs = queue.Queue()
    if mode == 'solo':
        for u in units:
            for s in sets:
                jobs.put((s, u))
        cpus = (CORES + [c + SIBLING for c in CORES])[:int(os.environ.get('SLOTS', '8'))]

        def worker(cpu):
            while True:
                try:
                    s, u = jobs.get_nowait()
                except queue.Empty:
                    return
                solve(cpu, s, u, out, nl)
        threads = [threading.Thread(target=worker, args=(c,)) for c in cpus]
    else:
        assert len(sets) == 2, 'pair takes two settings'
        for k, u in enumerate(units):
            jobs.put((k, u))

        def worker(core):
            while True:
                try:
                    k, u = jobs.get_nowait()
                except queue.Empty:
                    return
                a, b = (core, core + SIBLING) if k % 2 == 0 else (core + SIBLING, core)
                ta = threading.Thread(target=solve, args=(a, sets[0], u, out, nl))
                tb = threading.Thread(target=solve, args=(b, sets[1], u, out, nl))
                ta.start()
                tb.start()
                ta.join()
                tb.join()
        threads = [threading.Thread(target=worker, args=(c,)) for c in CORES]
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    open(os.path.join(out, 'DONE'), 'w').close()


main()
