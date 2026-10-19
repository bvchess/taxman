#!/usr/bin/env python3
"""The cache of release binaries that setup.sh uses (RUNBOOK.md, section 11).  Standard library only.

A machine that needs the release binaries of a commit looks here before it builds them, and the one release machine
that has built and tested them puts them here.  Nothing is stored that has not passed the release suite.

  build_cache.py key       the cache key of this checkout if the cache can be used here, or why it cannot
  build_cache.py fetch     download the binaries of this key that cover the set asked for, check them, unpack them
  build_cache.py pack      pack the binaries just built into a stage directory, with the manifest (nothing is sent)
  build_cache.py put       send a packed stage to the cache, with the result of the release suite beside it
  build_cache.py times-get / times-put     the test durations the release suite orders its tests by (setup.sh calls
                           them only where the key exists: they look at no checkout)

Common options (setup.sh passes them):
  --root DIR      the checkout (the directory with solver/ in it)
  --cache URL     where the cache is: s3://bucket/prefix, file:///directory (a stand-in for tests), or off.  Default
                  $BUILD_CACHE, else s3://taxman-013764204840-us-east-1/builds
  --solver ARGS   what was (or is to be) built in solver/frame/: cargo arguments, "--bins --examples" or
                  "--bin taxman";
                  `--features studies` builds the study examples too, and is a part of the set (see below)
  --prover ARGS   what was built in solver/noframe/ (default none): the same
  --tmp DIR       where files are staged and downloaded (default: the system's temporary directory)

The key is COMMIT-ARCH-CPU: the checkout must be clean and at a commit, and RUSTFLAGS must be empty or a single
`-C target-cpu=...` (`native` is resolved to the CPU name the compiler reports), because `target-cpu=native` makes a
binary for the CPU that built it.  Under it, one directory a set of binaries:

  builds/KEY/SET/binaries.tar.gz   the files, paths relative to the checkout (solver/frame/target/release/..., and for
                                   the prover solver/noframe/target/release/...); every executable of a plural selector
  builds/KEY/SET/manifest          key=value lines: the commit, architecture, CPU, RUSTFLAGS, rustc, the host, the time,
                                   the sha256 of the tarball, `set=` lines (what the tarball holds) and the result of
                                   the release suite.  Sent last: an entry without one does not exist.
  builds/test-times.json           seconds a test took in the last release suite (any commit): a hint for ordering

An entry is never changed by a later run except by a release run of the same set, which replaces it.  A request is
covered by an entry that holds each of its parts: `--bin X` by `--bin X` or `--bins`, `--example X` by `--example X` or
`--examples`, `--features F` by `--features F`.  The entry of a larger set serves a smaller request; there is no
half-built entry to complete.  A feature is a part like the others: the entry of `--bins --examples --features studies`
(which holds the study examples that `--examples` alone does not build) serves a request for `--bins --examples`, and
an entry without the feature never serves a request with it, so a default build is not taken for one with the studies.

Exit status: 0 done; 1 a miss, or a transfer that failed; 2 a wrong request; 3 the cache is not used (the reason is
printed): a dirty checkout, no commit, other RUSTFLAGS, the cache off or out of reach.
"""
import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import socket
import subprocess
import sys
import tarfile
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, FrozenSet, List, Optional, Tuple

DEFAULT_CACHE = "s3://taxman-013764204840-us-east-1/builds"
CRATES = {"solver": "solver/frame", "prover": "solver/noframe"}  # the crates, relative to the checkout
TARBALL = "binaries.tar.gz"
MANIFEST = "manifest"
TIMES = "test-times.json"
NAME = re.compile(r"[A-Za-z0-9_-]+")
HASHED = re.compile(r".*-[0-9a-f]{16}")  # cargo's copies of a binary with its metadata hash in the name
S3_TIMEOUT = 900  # seconds for one transfer

Selector = Tuple[str, str, str]  # (crate, "bin" | "bins" | "example" | "examples", name or "")


class Refused(Exception):
    """The cache is not used for this run; the message says why."""


class Failed(Exception):
    """What was asked cannot be done (a build lacks a file, a tarball holds a stranger, a suite did not pass)."""


class Usage(Failed):
    """The request itself is wrong."""


def say(message: str) -> None:
    print(f"{datetime.now(timezone.utc):%Y-%m-%dT%H:%M:%SZ} build cache: {message}", file=sys.stderr, flush=True)


# --- what is asked for ----------------------------------------------------------------------------------------------
def parse_selectors(crate: str, text: str) -> FrozenSet[Selector]:
    """The parts of a cargo build that `text` (`--bins --examples`, `--bin taxman --example ledger`) asks for.  A
    feature (`--features studies`) is a part of its own: it makes the build another one, whose entry serves a request
    that does not name the feature but not the other way round."""
    found = set()
    words = text.split()
    i = 0
    while i < len(words):
        word = words[i]
        if word in ("--bins", "--examples"):
            found.add((crate, word[2:], ""))
            i += 1
            continue
        if word in ("--bin", "--example", "--features") and i + 1 < len(words):
            kind, name = word[2:], words[i + 1]
            i += 2
        elif word.startswith(("--bin=", "--example=", "--features=")):
            kind, name = word[2:].split("=", 1)
            i += 1
        else:
            raise Usage(f"unsupported build argument {word!r} for the {crate}: --bins, --examples, --bin NAME, "
                        "--example NAME and --features NAME are what BUILD and BUILD_PROVER take")
        if not NAME.fullmatch(name):
            raise Usage(f"{name!r} is not a name of a binary, an example or a feature")
        found.add((crate, kind, name))
    return frozenset(found)


def wanted(args: argparse.Namespace) -> FrozenSet[Selector]:
    return parse_selectors("solver", args.solver) | parse_selectors("prover", args.prover)


def text_of(selector: Selector) -> str:
    return " ".join(part for part in selector if part)


def slug(selectors: FrozenSet[Selector]) -> str:
    """The name of the entry that holds exactly these parts."""
    return "+".join(text_of(s).replace(" ", "-") for s in sorted(selectors))


def covers(have: FrozenSet[Selector], want: FrozenSet[Selector]) -> bool:
    """Whether a build of `have` holds everything of `want`."""
    for crate, kind, name in want:
        if (crate, kind, name) in have or (name and (crate, kind + "s", "") in have):
            continue
        return False
    return True


def artifact_paths(root: Path, selectors: FrozenSet[Selector]) -> List[str]:
    """The files (paths relative to `root`) that a build of `selectors` made: a named binary or example is one file, a
    plural selector is every executable of its directory (not the hash-named copies cargo makes, dependency files or
    objects)."""
    paths = set()
    for crate, kind, name in sorted(selectors):
        if kind == "features":
            continue  # an option of the build, not a file
        directory = Path(CRATES[crate]) / "target" / "release"
        if kind.startswith("example"):
            directory = directory / "examples"
        if name:
            candidates = [directory / name]
        else:
            listing = sorted((root / directory).iterdir()) if (root / directory).is_dir() else []
            candidates = [directory / p.name for p in listing if "." not in p.name and not HASHED.fullmatch(p.name)
                          and p.is_file() and os.access(p, os.X_OK)]
            if not candidates:
                raise Failed(f"nothing was built in {directory} for {text_of((crate, kind, name))}")
        for candidate in candidates:
            if not os.access(root / candidate, os.X_OK):
                raise Failed(f"the build did not produce {candidate}")
            paths.add(candidate.as_posix())
    return sorted(paths)


# --- which checkout and which build ---------------------------------------------------------------------------------
def git(root: Path, *arguments: str) -> str:
    done = subprocess.run(["git", "-C", str(root), *arguments], capture_output=True, text=True)
    if done.returncode != 0:
        raise Refused(f"git {' '.join(arguments)}: {done.stderr.strip() or 'failed'}")
    return done.stdout


def native_cpu() -> str:
    try:
        listing = subprocess.run(["rustc", "--print", "target-cpus"], capture_output=True, text=True).stdout
    except OSError as error:
        raise Refused(f"cannot ask rustc for the native CPU: {error}")
    found = re.search(r"\(currently ([^)]+)\)", listing)
    if not found:
        raise Refused("rustc does not say which CPU `native` is")
    return found.group(1)


def target_cpu(rustflags: str) -> str:
    """The CPU the build targets, from RUSTFLAGS, which must be empty or a single `-C target-cpu=...`."""
    flags = " ".join(rustflags.split())
    if not flags:
        return "generic"
    found = re.fullmatch(r"-C ?target-cpu=([A-Za-z0-9._-]+)", flags)
    if not found:
        raise Refused(f"RUSTFLAGS={flags!r}: the key records only a target-cpu")
    return native_cpu() if found.group(1) == "native" else found.group(1)


def architecture() -> str:
    machine = platform.machine()
    return "aarch64" if machine == "arm64" else machine


def cache_key(root: Path) -> Tuple[str, Dict[str, str]]:
    """The key of the checkout, and the facts it is made of; Refused if the checkout is dirty or has no commit."""
    rustflags = os.environ.get("RUSTFLAGS", "")
    commit = git(root, "rev-parse", "HEAD").strip()
    if git(root, "status", "--porcelain").strip():
        raise Refused("the checkout has uncommitted changes")
    facts = {"commit": commit, "arch": architecture(), "cpu": target_cpu(rustflags),
             "rustflags": " ".join(rustflags.split())}
    return f"{commit}-{facts['arch']}-{facts['cpu']}", facts


# --- the store: an S3 prefix, or a directory standing in for one in tests -------------------------------------------
class FileStore:
    def __init__(self, base: Path):
        self.base = base

    def entries(self, prefix: str) -> List[str]:
        directory = self.base / prefix
        return sorted(p.name for p in directory.iterdir() if p.is_dir()) if directory.is_dir() else []

    def read_text(self, key: str) -> Optional[str]:
        path = self.base / key
        return path.read_text() if path.is_file() else None

    def get(self, key: str, destination: Path) -> bool:
        path = self.base / key
        if not path.is_file():
            return False
        shutil.copyfile(path, destination)
        return True

    def put(self, source: Path, key: str) -> bool:
        path = self.base / key
        path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, path)
        return True


class S3Store:
    def __init__(self, base: str):
        self.base = base.rstrip("/")
        self.aws = os.environ.get("AWS_CLI", "aws")
        if shutil.which(self.aws) is None:
            raise Refused(f"the AWS CLI ({self.aws}) is not installed")

    def run(self, arguments: List[str]) -> subprocess.CompletedProcess:
        try:
            done = subprocess.run([self.aws, "s3", *arguments], capture_output=True, text=True, timeout=S3_TIMEOUT)
        except subprocess.TimeoutExpired:
            say(f"aws s3 {arguments[0]}: no answer in {S3_TIMEOUT} s")
            return subprocess.CompletedProcess(arguments, 124, "", "")
        if done.returncode != 0 and done.stderr.strip():
            say(f"aws s3 {arguments[0]}: {done.stderr.strip().splitlines()[-1]}")
        return done

    def entries(self, prefix: str) -> List[str]:
        done = self.run(["ls", f"{self.base}/{prefix}/"])
        return sorted(line.split()[-1].rstrip("/") for line in done.stdout.splitlines() if line.strip())

    def read_text(self, key: str) -> Optional[str]:
        done = self.run(["cp", f"{self.base}/{key}", "-", "--only-show-errors"])
        return done.stdout if done.returncode == 0 else None

    def get(self, key: str, destination: Path) -> bool:
        return self.run(["cp", f"{self.base}/{key}", str(destination), "--only-show-errors"]).returncode == 0

    def put(self, source: Path, key: str) -> bool:
        return self.run(["cp", str(source), f"{self.base}/{key}", "--only-show-errors"]).returncode == 0


def open_store(url: str):
    if url in ("", "off", "none"):
        raise Refused("the cache is off (BUILD_CACHE)")
    if url.startswith("file://"):
        return FileStore(Path(url[len("file://"):]))
    if url.startswith("s3://"):
        return S3Store(url)
    raise Usage(f"BUILD_CACHE={url!r}: s3://bucket/prefix, file:///directory or off")


def open_cache(args: argparse.Namespace) -> Tuple[object, str, Dict[str, str]]:
    root = Path(args.root)
    key, facts = cache_key(root)
    return open_store(args.cache), key, facts


# --- manifests and checksums ----------------------------------------------------------------------------------------
def parse_manifest(text: str) -> Dict[str, List[str]]:
    found: Dict[str, List[str]] = {}
    for line in text.splitlines():
        if "=" in line:
            name, value = line.split("=", 1)
            found.setdefault(name, []).append(value)
    return found


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def entry_set(manifest: Dict[str, List[str]]) -> FrozenSet[Selector]:
    found = set()
    for line in manifest.get("set", []):
        crate, kind, *name = line.split()
        found.add((crate, kind, name[0] if name else ""))
    return frozenset(found)


def choose_entry(store, key: str, want: FrozenSet[Selector]) -> Optional[Tuple[str, Dict[str, List[str]]]]:
    """The entry under `key` that covers `want` and whose suite passed: the one that holds the fewest parts."""
    best = None
    for name in store.entries(key):
        text = store.read_text(f"{key}/{name}/{MANIFEST}")
        if text is None:
            continue
        manifest = parse_manifest(text)
        have = entry_set(manifest)
        if manifest.get("tests", [""])[0] != "passed" or not covers(have, want):
            continue
        if best is None or len(have) < len(entry_set(best[1])):
            best = (name, manifest)
    return best


def checked_members(archive: tarfile.TarFile) -> List[tarfile.TarInfo]:
    """The members of the tarball, if they are regular files under a crate's target/release and nothing else."""
    allowed = tuple(f"{crate}/target/release/" for crate in CRATES.values())
    members = archive.getmembers()
    for member in members:
        if not (member.isreg() and member.name.startswith(allowed) and ".." not in Path(member.name).parts):
            raise Failed(f"the tarball holds {member.name!r}, which is not a file under target/release")
    return members


# --- the commands ---------------------------------------------------------------------------------------------------
def command_key(args: argparse.Namespace) -> int:
    wanted(args)  # a wrong request is an error even where the cache is not used
    _, key, _ = open_cache(args)  # the cache can be used: the checkout is clean, the flags are known, the store opens
    print(key)
    return 0


def command_fetch(args: argparse.Namespace) -> int:
    want = wanted(args)
    store, key, _ = open_cache(args)
    chosen = choose_entry(store, key, want)
    if chosen is None:
        say(f"no entry under {key} holds {' + '.join(text_of(s) for s in sorted(want))}")
        return 1
    name, manifest = chosen
    say(f"entry {key}/{name} (built {manifest.get('built', ['?'])[0]}, tests {manifest.get('tests', ['?'])[0]})")
    with tempfile.TemporaryDirectory(dir=args.tmp) as scratch:
        tarball = Path(scratch) / TARBALL
        if not store.get(f"{key}/{name}/{TARBALL}", tarball):
            say("the tarball could not be downloaded")
            return 1
        expected = manifest.get("sha256", [""])[0]
        actual = sha256(tarball)
        if actual != expected:
            say(f"the tarball's checksum is {actual}, the manifest's is {expected}: not used")
            return 1
        root = Path(args.root)
        with tarfile.open(tarball) as archive:
            members = checked_members(archive)
            options = {"filter": "data"} if hasattr(tarfile, "data_filter") else {}
            archive.extractall(root, members=members, **options)
    try:
        unpacked = artifact_paths(root, want)
    except Failed as why:
        say(f"the unpacked entry lacks something that was asked for: {why}")
        return 1
    say(f"{len(unpacked)} files unpacked from {key}/{name}")
    return 0


def command_pack(args: argparse.Namespace) -> int:
    want = wanted(args)
    key, facts = cache_key(Path(args.root))
    root = Path(args.root)
    paths = artifact_paths(root, want)
    stage = Path(args.stage)
    shutil.rmtree(stage, ignore_errors=True)
    stage.mkdir(parents=True)
    tarball = stage / TARBALL
    with tarfile.open(tarball, "w:gz", compresslevel=6) as archive:
        for path in paths:
            archive.add(root / path, arcname=path, filter=_plain)
    lines = [f"{name}={value}" for name, value in facts.items()]
    lines += [f"rustc={subprocess.run(['rustc', '--version'], capture_output=True, text=True).stdout.strip()}",
              f"host={socket.gethostname()}",
              f"built={datetime.now(timezone.utc):%Y-%m-%dT%H:%M:%SZ}",
              f"sha256={sha256(tarball)}",
              f"bytes={tarball.stat().st_size}"]
    lines += [f"set={text_of(s)}" for s in sorted(want)]
    lines += [f"file={path}" for path in paths]
    (stage / MANIFEST).write_text("\n".join(lines) + "\n")
    say(f"packed {len(paths)} files, {tarball.stat().st_size // (1 << 20)} MiB, in {stage}")
    return 0


def _plain(info: tarfile.TarInfo) -> tarfile.TarInfo:
    info.uid = info.gid = 0
    info.uname = info.gname = ""
    return info


def command_put(args: argparse.Namespace) -> int:
    stage = Path(args.stage)
    # The stage was packed from a clean checkout and is all there is to send: the checkout need not still be clean (the
    # tests may have left files in it), and its key is the one the manifest recorded.
    manifest = parse_manifest((stage / MANIFEST).read_text())
    store = open_store(args.cache)
    key = f"{manifest['commit'][0]}-{manifest['arch'][0]}-{manifest['cpu'][0]}"
    summary = json.loads(Path(args.summary).read_text())
    failed = sum(crate["failed"] for crate in summary["crates"].values())
    if failed or not summary["crates"]:
        raise Failed("refusing to store a build whose release suite did not pass")
    need = {crate for crate, _, _ in entry_set(manifest)}
    if not need <= set(summary["crates"]):
        raise Failed(f"the suite of {sorted(need - set(summary['crates']))} was not run: nothing to say it passed")
    lines = ["tests=passed", f"tests_seconds={summary['seconds']:.0f}", f"tests_jobs={summary['jobs']}"]
    for crate, result in sorted(summary["crates"].items()):
        lines.append(f"tests_{crate}={result['passed']} passed, {result['failed']} failed, {result['ignored']} ignored")
    entry = f"{key}/{slug(entry_set(manifest))}"
    final = stage / "manifest.final"
    final.write_text((stage / MANIFEST).read_text() + "\n".join(lines) + "\n")
    if not store.put(stage / TARBALL, f"{entry}/{TARBALL}") or not store.put(final, f"{entry}/{MANIFEST}"):
        say(f"could not send {entry}")
        return 1
    say(f"stored {entry}")
    return 0


def command_times_get(args: argparse.Namespace) -> int:
    return 0 if open_store(args.cache).get(TIMES, Path(args.file)) else 1


def command_times_put(args: argparse.Namespace) -> int:
    return 0 if open_store(args.cache).put(Path(args.file), TIMES) else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    for name, function, extra in (("key", command_key, ()), ("fetch", command_fetch, ()),
                                  ("pack", command_pack, ("stage",)), ("put", command_put, ("stage", "summary")),
                                  ("times-get", command_times_get, ("file",)),
                                  ("times-put", command_times_put, ("file",))):
        sub = commands.add_parser(name)
        sub.set_defaults(function=function)
        sub.add_argument("--root", required=True)
        sub.add_argument("--cache", default=os.environ.get("BUILD_CACHE", DEFAULT_CACHE))
        sub.add_argument("--solver", default="")
        sub.add_argument("--prover", default="")
        sub.add_argument("--tmp", default=None)
        for option in extra:
            sub.add_argument(f"--{option}", required=True)
    args = parser.parse_args()
    try:
        return args.function(args)
    except Refused as why:
        say(f"not used: {why}")
        return 3
    except Usage as why:
        say(f"error: {why}")
        return 2
    except Failed as why:
        say(f"error: {why}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
