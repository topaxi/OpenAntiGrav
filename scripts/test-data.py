#!/usr/bin/env python3
"""`just test-data`: the disc-backed suite, minus the test binaries that cannot
have changed since they last passed.

A test binary's result is a function of its own bytes and of what it reads at
run time. So after a run, every binary whose tests all passed is recorded in
`target/test-data-pass.json` with a fingerprint of both, and the next run skips
a binary whose fingerprint still matches. A merge batch that only touched
`oag-game` reruns `oag-game`'s binaries and nothing below it.

# The fingerprint

Per test binary, a hash of:

- **the binary's own bytes**. Measured on 2026-10-13: a rebuild with nothing
  changed, after a `touch`, and after `cargo clean -p oag-texture` is
  byte-identical, with kache on or off (kache and plain `rustc` differ from each
  other, as kache adds `--remap-path-prefix`, but each is stable). A change
  that compiles to the same code - a dead constant, a message the optimiser
  drops - relinks every dependent and still skips;
- **its package's executables** (`oag-game`'s tests spawn
  `CARGO_BIN_EXE_oag-game`), from nextest's own `non-test-binaries`;
- **the `.rs` files in its package's `tests/`**, by content, which each crate's
  `every_test_file_is_listed` reads at run time;
- **the shared inputs**, the same for every binary:
  - every file under `data/` by path, size and mtime (not content: tens of GB),
    following the `just link-data` symlinks, except the directories tests write
    their derived output to (`DATA_OUTPUTS`);
  - the content of every tracked or untracked-but-not-ignored file under
    `REPO_INPUTS`, except `.rs` files (compiled into the binary already) and
    Markdown that no `.rs` file names by its repository path (`docs/` pages
    are the evidence, not a test input - `web_picker_table` reads
    `docs/overview/status.md`, and that page stays in because it is named);
  - every environment variable a crate reads by name, every `OAG_*` one, and
    the libtest harness's own.

`oag-game`'s build script embeds the commit hash, so every commit would change
every `oag-game` binary. This script builds with `OAG_GIT_HASH=test-data`
unless it is already set. The cost: `oag-game` recompiles (about 45 s) when a
plain dev-profile build (`just play`, `cargo run -p oag-game`) and
`just test-data` alternate, as its build script reruns on that variable.

What it does not see: an external tool a test spawns (`ffmpeg`, `chdman`), the
GPU driver, and a test that reads a path it builds at run time from parts no
`.rs` file spells whole. `--full` is the answer to any doubt.

# Using it

    just test-data            # skip what is recorded green and unchanged
    just test-data --full     # run everything (also OAG_TEST_DATA_FULL=1)

Skipped binaries are listed by name before the run and counted again after
it, in the log as well as on the terminal; they are never counted as passed.
Each binary keeps its last `KEEP` green fingerprints, so a revert skips too. A
failed, cancelled or partial run records only the binaries every one of whose
tests printed `PASS`, and drops the fingerprint of any binary that ran and did
not.

`check-test-budget` judges the suite's wall clock only on a run that skipped
nothing: the log of one that did carries `PARTIAL_MARKER`, and the check holds
each test that ran to its own ceiling, as for `test-data-affected`.

# Slices

    just test-data --partition 2/4          # the second of four slices
    just test-data --partition hash:1/3     # nextest's own spellings also work

`N/M` is `count:N/M`: tests dealt out round-robin in nextest's order, which
spreads one binary's heavy tests over the slices (`hash:` is the same shape
but lands 20% uneven in tests on this suite). `scripts/lead-test-data.sh` runs
all M slices, each under its own `flock -o` on the shared gate lock, so a
member's gate waits for at most one slice. Never wrap that script in `flock`.

A binary's tests are spread over the slices, so a slice alone proves nothing
about it. The run's state lives in `target/test-data-run.json`, keyed by a hash
of the partition, the flags and every binary's fingerprint (a tree that changed
between slices is a new run and the old state is discarded):

- the skip set is fixed by the first slice of a run, because nextest deals
  tests out after the filter, and a filter that changed between slices would
  leave tests in no slice at all;
- a binary is recorded green once the tests that passed, over every slice run
  so far, are all of its tests and nothing in it printed anything but `PASS`;
- a binary that printed anything else loses its recorded fingerprint at once;
  one still missing tests loses it only when the last slice has run;
- re-running a slice replaces its earlier result.

Every slice's log (`target/test-data.slice-N-of-M.log`) carries
`PARTIAL_MARKER`: each test is held to its own ceiling, and the suite ceiling
is not judged, because a slice's wall clock is a part of the suite's and the
sum of the parts exceeds it (the tail no longer overlaps).
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RECORD = ROOT / "target" / "test-data-pass.json"
LOG = ROOT / "target" / "test-data.log"
RECORD_VERSION = 2

# Green fingerprints kept per binary, newest first, so switching back to a tree
# that already passed (a revert, a branch and back) skips again.
KEEP = 8

# Top-level `data/` directories that tests write into: derived caches,
# screenshots and scratch output. Fingerprinting them would make every run
# invalidate the next. `cache/` is derived from `images/` and `dlc/`, which are
# fingerprinted themselves.
DATA_OUTPUTS = {"cache", "derived", "scratch", "shots"}

# The repository paths outside `data/` a test reads at run time, found by
# searching `crates/**/*.rs` for every top-level name as a path (2026-10-13).
# `crates/` is its non-`.rs` files: fixtures beside the tests. Anything else -
# `scripts/`, `packaging/`, `Cargo.toml` - reaches a test only through
# `include_str!`/`include_bytes!` or the build, so it is in the binary already.
# A test that starts reading a new top-level directory adds it here.
REPO_INPUTS = (
    "crates/",
    "assets/",
    "docs/",
    "schemas/",
    "verification/",
    "web/",
    ".config/",
    ".gitignore",
)

# Read by the libtest harness or by nextest rather than by any crate.
HARNESS_ENV = ("RUST_MIN_STACK", "RUST_TEST_THREADS", "RUST_TEST_TIME_UNIT")

NEXTEST = [
    "cargo",
    "nextest",
    "--workspace",
    "--tests",
    "--run-ignored",
    "all",
]


def digest(*parts: bytes | str) -> str:
    h = hashlib.blake2b(digest_size=20)
    for part in parts:
        data = part.encode() if isinstance(part, str) else part
        h.update(len(data).to_bytes(8, "little"))
        h.update(data)
    return h.hexdigest()


def file_hash(path: Path, cache: dict[str, list]) -> str:
    """The content hash of `path`, memoised across runs on its inode, size,
    mtime and ctime. A build cache that restores a file can set its mtime but
    not its ctime, so a restored file is always hashed again."""
    st = path.stat()
    key = str(path)
    stamp = [st.st_ino, st.st_size, st.st_mtime_ns, st.st_ctime_ns]
    if (hit := cache.get(key)) and hit[:4] == stamp:
        return hit[4]
    h = hashlib.blake2b(digest_size=20)
    with path.open("rb") as f:
        while chunk := f.read(1 << 20):
            h.update(chunk)
    cache[key] = [*stamp, h.hexdigest()]
    return cache[key][4]


def git_files() -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        cwd=ROOT,
        capture_output=True,
        check=True,
    ).stdout
    return sorted({p for p in out.decode().split("\0") if p})


def repo_inputs(files: list[str]) -> str:
    """Every non-`.rs` file under `REPO_INPUTS`, by content."""
    sources = [p for p in files if p.endswith(".rs") and p.startswith("crates/")]
    named: set[str] = set()
    literal = re.compile(r"[A-Za-z0-9_./-]+\.md\b")
    for source in sources:
        try:
            text = (ROOT / source).read_text(errors="replace")
        except FileNotFoundError:
            continue
        named.update(re.sub(r"^(\.\./)+", "", m) for m in literal.findall(text))
    h = hashlib.blake2b(digest_size=20)
    for path in files:
        if path.endswith(".rs") or not path.startswith(REPO_INPUTS):
            continue
        if path.endswith(".md") and path not in named:
            continue
        full = ROOT / path
        if not full.is_file():
            continue  # deleted from the worktree but still in the index
        h.update(path.encode() + b"\0")
        h.update(hashlib.blake2b(full.read_bytes(), digest_size=20).digest())
    return h.hexdigest()


def data_inputs() -> dict[str, str]:
    """Per top-level `data/` entry, a hash of every file's path, size and mtime."""
    data = ROOT / "data"
    out: dict[str, str] = {}
    if not data.is_dir():
        return out
    for top in sorted(os.listdir(data)):
        if top in DATA_OUTPUTS:
            continue
        h = hashlib.blake2b(digest_size=20)
        base = data / top
        if base.is_file():
            st = base.stat()
            h.update(f"{top}\0{st.st_size}\0{st.st_mtime_ns}\n".encode())
        # The top-level entry is followed - in a worktree `just link-data` makes it
        # a symlink into the main checkout - but nothing below it is: a scratch
        # directory holding a linked `data/` is a cycle, and a Wine prefix's
        # `dosdevices/z:` is the whole filesystem. Either made this walk endless
        # (2026-10-14). A nested link counts by its own path, size and mtime, and a
        # symlinked directory below the top is hashed as such, not entered.
        real = os.path.realpath(base)
        for dirpath, dirnames, filenames in os.walk(real, followlinks=False):
            dirnames.sort()
            linked = [d for d in dirnames if os.path.islink(os.path.join(dirpath, d))]
            for name in sorted(filenames + linked):
                path = os.path.join(dirpath, name)
                try:
                    st = os.lstat(path)
                except OSError:
                    continue
                rel = os.path.join(top, os.path.relpath(path, real))
                h.update(f"{rel}\0{st.st_size}\0{st.st_mtime_ns}\n".encode())
        out[top] = h.hexdigest()
    return out


def env_inputs(files: list[str]) -> str:
    names = set(HARNESS_ENV)
    read = re.compile(r"env::var(?:_os)?\(\s*\"([A-Za-z_][A-Za-z0-9_]*)\"")
    for source in files:
        if source.endswith(".rs") and source.startswith("crates/"):
            try:
                names.update(read.findall((ROOT / source).read_text(errors="replace")))
            except FileNotFoundError:
                pass
    names.update(k for k in os.environ if k.startswith("OAG_"))
    names.discard("OAG_TEST_DATA_FULL")
    return digest(*(f"{k}={os.environ.get(k, '<unset>')}" for k in sorted(names)))


def list_suites() -> dict:
    """Build everything and list it; build output streams to the terminal."""
    proc = subprocess.run(
        [*NEXTEST[:2], "list", *NEXTEST[2:], "--message-format", "json"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
    )
    if proc.returncode != 0:
        sys.exit(proc.returncode)
    return json.loads(proc.stdout)


def fingerprints(listing: dict, shared: str, cache: dict) -> dict[str, dict]:
    meta = listing["rust-build-meta"]
    target = Path(meta["target-directory"])
    bins: dict[str, list[str]] = {}
    for package_id, entries in meta.get("non-test-binaries", {}).items():
        for entry in entries:
            path = target / entry["path"]
            if path.is_file():
                bins.setdefault(package_id, []).append(
                    f"{entry['name']}={file_hash(path, cache)}"
                )
    out = {}
    for binary_id, suite in listing["rust-suites"].items():
        tests_dir = Path(suite["cwd"]) / "tests"
        listed = (
            sorted(
                f"{p.relative_to(tests_dir)}={file_hash(p, cache)}"
                for p in tests_dir.rglob("*.rs")
            )
            if tests_dir.is_dir()
            else []
        )
        own = file_hash(Path(suite["binary-path"]), cache)
        out[binary_id] = {
            "fingerprint": digest(
                own,
                *sorted(bins.get(suite["package-id"], [])),
                *listed,
                shared,
            ),
            "tests": len(suite["testcases"]),
        }
    return out


# `PASS [   1.234s] (  12/4101) crate::binary test_name`, and every other
# status nextest prints for a finished test (FAIL, SIGSEGV, TIMEOUT, LEAK, ...).
RESULT = re.compile(r"^\s+([A-Z][A-Z -]*?\d*)\s+\[\s*[0-9.]+s\]\s+\(\s*[\d/ ]+\)\s+(\S+)\s+(\S+)")


def green_binaries(log: str) -> tuple[dict[str, set[str]], set[str]]:
    """Per binary, which tests printed PASS, and which binaries printed anything else."""
    passed: dict[str, set[str]] = {}
    other: set[str] = set()
    for line in log.splitlines():
        if found := RESULT.match(line):
            status, binary, name = found.groups()
            if status.strip() == "PASS":
                passed.setdefault(binary, set()).add(name)
            else:
                other.add(binary)
    return passed, other


class Partition:
    """One slice of a run: `index` of `total`, dealt out by nextest's `mode`."""

    MODES = ("count", "hash")

    def __init__(self, mode: str, index: int, total: int) -> None:
        self.mode, self.index, self.total = mode, index, total

    def __str__(self) -> str:
        return f"{self.mode}:{self.index}/{self.total}"


def parse_partition(text: str) -> Partition:
    """`N/M`, `count:N/M` or `hash:N/M` with 1 <= N <= M; `N/M` means `count`."""
    mode, _, rest = text.rpartition(":")
    mode = mode or "count"
    found = re.fullmatch(r"([0-9]+)/([0-9]+)", rest)
    if mode not in Partition.MODES or not found:
        raise ValueError(f"{text!r} is not [count:|hash:]N/M")
    index, total = int(found.group(1)), int(found.group(2))
    if not 1 <= index <= total:
        raise ValueError(f"{text!r}: the slice must satisfy 1 <= N <= M")
    return Partition(mode, index, total)


def run_id(part: Partition | None, full: bool, current: dict[str, dict]) -> str:
    """Names a run: the same for every slice of one tree and one set of flags."""
    return digest(
        f"{part.mode}:{part.total}" if part else "whole",
        f"full={full}",
        *(f"{b}={f['fingerprint']}" for b, f in sorted(current.items())),
    )


def new_run(rid: str, total: int, skipped: list[str]) -> dict:
    return {"id": rid, "m": total, "skipped": skipped, "slices": {}}


def absorb_slice(
    run: dict, index: int, passed: dict[str, set[str]], other: set[str]
) -> None:
    """Records what slice `index` printed, replacing an earlier run of it."""
    run["slices"][str(index)] = {
        "passed": {b: sorted(names) for b, names in sorted(passed.items())},
        "other": sorted(other),
    }


def settle(
    recorded: dict, current: dict[str, dict], run: dict, stamp: str
) -> tuple[list[str], list[str], list[str]]:
    """Updates the green record from every slice of `run` so far.

    Returns the binaries newly recorded, the ones that failed, and the ones
    still missing tests that a slice not yet run may hold."""
    final = len(run["slices"]) >= run["m"]
    passed: dict[str, set[str]] = {}
    other: set[str] = set()
    for done in run["slices"].values():
        for b, names in done["passed"].items():
            passed.setdefault(b, set()).update(names)
        other.update(done["other"])
    newly, failed, waiting = [], [], []
    for b, f in current.items():
        if b in run["skipped"]:
            continue
        fingerprint = f["fingerprint"]
        greens = recorded.get(b, {})
        if b not in other and len(passed.get(b, ())) == f["tests"]:
            greens.pop(fingerprint, None)
            greens = {fingerprint: stamp, **dict(list(greens.items())[: KEEP - 1])}
            newly.append(b)
        elif b in other or final:
            greens.pop(fingerprint, None)
            failed.append(b)
        else:
            waiting.append(b)
            continue
        if greens:
            recorded[b] = greens
        else:
            recorded.pop(b, None)
    return newly, failed, waiting


def load_record() -> dict:
    try:
        record = json.loads(RECORD.read_text())
        if record.get("version") == RECORD_VERSION:
            return record
    except (FileNotFoundError, json.JSONDecodeError):
        pass
    return {"version": RECORD_VERSION, "binaries": {}, "hash-cache": {}}


def save_record(record: dict) -> None:
    RECORD.parent.mkdir(parents=True, exist_ok=True)
    tmp = RECORD.with_suffix(".tmp")
    tmp.write_text(json.dumps(record, indent=1, sort_keys=True))
    tmp.replace(RECORD)


RUN_STATE = ROOT / "target" / "test-data-run.json"


def load_run(rid: str) -> dict | None:
    """The state of the run named `rid`, or None when it is another run's."""
    try:
        run = json.loads(RUN_STATE.read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return None
    return run if run.get("id") == rid else None


def save_run(run: dict) -> None:
    tmp = RUN_STATE.with_suffix(".tmp")
    tmp.write_text(json.dumps(run, sort_keys=True))
    tmp.replace(RUN_STATE)


# The line `check-test-budget.py` looks for: a log carrying it is a partial run,
# and its wall clock says nothing about the whole suite's.
PARTIAL_MARKER = "test-data: PARTIAL RUN"


class Tee:
    """Everything said goes to the terminal and to `target/test-data.log`."""

    def __init__(self, path: Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        self.path = path
        self.log = path.open("w")

    def say(self, line: str = "") -> None:
        print(line, flush=True)
        self.log.write(line + "\n")
        self.log.flush()

    def run(self, cmd: list[str]) -> int:
        proc = subprocess.Popen(
            cmd,
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            errors="replace",
        )
        assert proc.stdout is not None
        for line in proc.stdout:
            sys.stdout.write(line)
            self.log.write(line)
        self.log.flush()
        return proc.wait()


def nextest_run(skipped: list[str], part: Partition | None) -> list[str]:
    cmd = [*NEXTEST[:2], "run", *NEXTEST[2:], "--no-fail-fast", "--status-level", "pass"]
    if part:
        cmd += ["--partition", str(part), "--no-tests=pass"]
    if skipped:
        excluded = " | ".join(f"binary_id(={b})" for b in skipped)
        cmd += ["-E", f"not ({excluded})"]
    return cmd


def parse_args(args: list[str]) -> tuple[bool, Partition | None]:
    """`--full` and `--partition SPEC` (or `--partition=SPEC`); anything else is an error."""
    full = False
    part = None
    it = iter(args)
    for a in it:
        if a == "--full":
            full = True
        elif a == "--partition" or a.startswith("--partition="):
            spec = a.partition("=")[2] if "=" in a else next(it, "")
            part = parse_partition(spec)
        else:
            raise ValueError(f"unknown argument {a!r}; only --full, --partition [count:|hash:]N/M")
    return full, part


def main() -> int:
    try:
        full, part = parse_args(sys.argv[1:])
    except ValueError as e:
        print(f"test-data: {e}", file=sys.stderr)
        return 2
    full = full or os.environ.get("OAG_TEST_DATA_FULL", "") not in ("", "0")

    # `oag-game`'s build script embeds the commit hash, which would make every
    # `oag-game` binary differ after every commit whether or not its code did.
    os.environ.setdefault("OAG_GIT_HASH", "test-data")
    os.environ.setdefault("OAG_BUILD_DATE", "")

    record = load_record()
    cache = record.setdefault("hash-cache", {})
    listing = list_suites()
    started = time.time()
    files = git_files()
    data = data_inputs()
    shared = digest(
        repo_inputs(files), env_inputs(files), *(f"{k}={v}" for k, v in data.items())
    )
    current = fingerprints(listing, shared, cache)

    recorded = record["binaries"]
    rid = run_id(part, full, current)
    run = load_run(rid) if part else None
    if run is None:
        skipped = []
        if not full:
            skipped = sorted(
                b
                for b, f in current.items()
                if f["fingerprint"] in recorded.get(b, {})
            )
        run = new_run(rid, part.total if part else 1, skipped)
    else:
        # Fixed by the run's first slice: see "Slices" in the module docstring.
        skipped = [b for b in run["skipped"] if b in current]
        run["skipped"] = skipped
    skipped_tests = sum(current[b]["tests"] for b in skipped)
    total_tests = sum(f["tests"] for f in current.values())

    log_path = (
        LOG.with_name(f"test-data.slice-{part.index}-of-{part.total}.log")
        if part
        else LOG
    )
    out = Tee(log_path)
    out.say(
        f"test-data: fingerprinted {len(current)} test binaries in "
        f"{time.time() - started:.1f}s"
        + ("; --full, running every one" if full else "")
    )
    if part:
        out.say(
            f"{PARTIAL_MARKER}: slice {part.index} of {part.total} ({part}); a "
            f"slice's wall clock says nothing about the suite's, so only the "
            f"per-test ceilings are judged. {len(run['slices'])} slice(s) of this "
            f"run are already recorded."
        )

    def report_skips(detail: bool) -> None:
        if not skipped:
            return
        out.say()
        out.say(
            f"{PARTIAL_MARKER}: SKIPPED {len(skipped)} of {len(current)} test binaries, "
            f"{skipped_tests} of {total_tests} tests. Each passed in full on its "
            f"recorded run with a byte-identical binary and unchanged inputs; none "
            f"of them ran now. `just test-data --full` runs everything."
        )
        if detail:
            for b in skipped:
                when = recorded.get(b, {}).get(current[b]["fingerprint"], "earlier")
                out.say(f"    SKIPPED  {b}  ({current[b]['tests']} tests, green {when})")

    report_skips(detail=True)
    out.say()
    if len(skipped) == len(current):
        out.say("test-data: every test binary is skipped, nothing to run")
        tests = 0
    else:
        tests = out.run(nextest_run(skipped, part))

    passed, other = green_binaries(log_path.read_text(errors="replace"))
    stamp = time.strftime("%Y-%m-%d %H:%M")
    ran = [b for b in current if b not in skipped]
    absorb_slice(run, part.index if part else 1, passed, other)
    newly, failed, waiting = settle(recorded, current, run, stamp)
    for b in list(recorded):
        if b not in current:
            del recorded[b]  # a binary that no longer exists
    record["hash-cache"] = {k: v for k, v in cache.items() if Path(k).is_file()}
    save_record(record)
    if part and len(run["slices"]) < run["m"]:
        save_run(run)
    else:
        RUN_STATE.unlink(missing_ok=True)

    after = data_inputs()
    moved = sorted(k for k in set(data) | set(after) if data.get(k) != after.get(k))
    if moved:
        out.say()
        out.say(
            f"test-data: data/{{{','.join(moved)}}} changed during the run. It is "
            f"fingerprinted as an input, so the next run skips nothing; if a test "
            f"writes there, add it to DATA_OUTPUTS in scripts/test-data.py."
        )

    report_skips(detail=False)
    out.say(
        f"test-data: {len(newly)} of {len(ran)} test binaries that ran passed in "
        f"full and are recorded; {len(failed)} did not"
        + (f": {', '.join(failed)}" if failed else "")
        + (
            f"; {len(waiting)} wait on slices not yet run"
            if waiting
            else ""
        )
    )
    if part:
        out.say(
            f"test-data: slice {part.index}/{part.total} took "
            f"{time.time() - started:.0f}s; {len(run['slices'])} of {run['m']} "
            f"slices of this run are done"
        )
    if skipped:
        out.say(
            "test-data: the suite ceiling is judged only on a run that skipped "
            "nothing; every test that ran is still held to its own."
        )
    out.log.close()

    budget = 0
    if len(skipped) < len(current):
        budget = subprocess.run(
            [sys.executable, str(ROOT / "scripts/check-test-budget.py"), str(log_path)],
            cwd=ROOT,
        ).returncode
    return tests if tests else budget


def selftest() -> int:
    """The rules above, on made-up listings; needs no build and no data."""

    def current(**tests: int) -> dict[str, dict]:
        return {b: {"fingerprint": f"fp-{b}", "tests": n} for b, n in tests.items()}

    def names(binary: str, *idx: int) -> dict[str, set[str]]:
        return {binary: {f"t{i}" for i in idx}}

    def start(cur, total, skipped=()):
        return new_run("id", total, list(skipped))

    # Argument parsing.
    for text, want in {
        "2/4": "count:2/4",
        "count:2/4": "count:2/4",
        "hash:1/3": "hash:1/3",
        "4/4": "count:4/4",
    }.items():
        assert str(parse_partition(text)) == want, text
    for bad in ("0/4", "5/4", "1/0", "foo:1/2", "1", "1/2/3", "", "-1/4", "count:", "1/x"):
        try:
            parse_partition(bad)
        except ValueError:
            continue
        raise AssertionError(f"{bad!r} must be refused")
    assert parse_args(["--partition", "2/4", "--full"])[1].index == 2
    assert str(parse_args(["--partition=hash:1/3"])[1]) == "hash:1/3"
    assert parse_args([]) == (False, None)
    for bad_args in (["--partition"], ["--partition", "9/4"], ["--bogus"]):
        try:
            parse_args(bad_args)
        except ValueError:
            continue
        raise AssertionError(f"{bad_args} must be refused")

    # A binary that spans two slices is not recorded from one slice alone.
    cur = current(spans=4)
    run, rec = start(cur, 2), {}
    absorb_slice(run, 1, names("spans", 0, 2), set())
    newly, failed, waiting = settle(rec, cur, run, "now")
    assert (newly, failed, waiting) == ([], [], ["spans"]), (newly, failed, waiting)
    assert rec == {}, "recorded from one slice of two"
    absorb_slice(run, 2, names("spans", 1, 3), set())
    newly, failed, waiting = settle(rec, cur, run, "now")
    assert newly == ["spans"] and rec == {"spans": {"fp-spans": "now"}}

    # A binary wholly inside the slices run so far is recorded at once.
    cur = current(small=2, big=4)
    run, rec = start(cur, 2), {}
    absorb_slice(run, 1, {**names("small", 0, 1), **names("big", 0)}, set())
    newly, _, waiting = settle(rec, cur, run, "now")
    assert newly == ["small"] and waiting == ["big"] and "big" not in rec

    # A failure evicts the old green at once, from any slice, even a partial run.
    cur = current(spans=4)
    run, rec = start(cur, 2), {"spans": {"fp-spans": "old"}}
    absorb_slice(run, 1, names("spans", 0, 2), set())
    settle(rec, cur, run, "now")
    assert "fp-spans" in rec["spans"], "a pending binary keeps its older green"
    absorb_slice(run, 2, names("spans", 1), {"spans"})
    newly, failed, _ = settle(rec, cur, run, "now")
    assert failed == ["spans"] and "spans" not in rec

    # The last slice leaves a binary short of tests red, as an unsliced run does.
    cur = current(spans=4)
    run, rec = start(cur, 2), {"spans": {"fp-spans": "old"}}
    absorb_slice(run, 1, names("spans", 0, 2), set())
    absorb_slice(run, 2, names("spans", 1), set())
    _, failed, _ = settle(rec, cur, run, "now")
    assert failed == ["spans"] and "spans" not in rec

    # Re-running a slice replaces it; names are not counted twice.
    cur = current(spans=4)
    run, rec = start(cur, 2), {}
    absorb_slice(run, 1, names("spans", 0, 1), set())
    absorb_slice(run, 1, names("spans", 0, 1), set())
    _, _, waiting = settle(rec, cur, run, "now")
    assert waiting == ["spans"] and not rec

    # A skipped binary is neither recorded nor evicted.
    cur = current(skipped=3, ran=1)
    run, rec = start(cur, 1, ["skipped"]), {"skipped": {"fp-skipped": "old"}}
    absorb_slice(run, 1, names("ran", 0), set())
    newly, failed, _ = settle(rec, cur, run, "now")
    assert newly == ["ran"] and failed == [] and rec["skipped"] == {"fp-skipped": "old"}

    # The unsliced run is a run of one slice: all tests passed, or red.
    cur = current(whole=2)
    run, rec = start(cur, 1), {}
    absorb_slice(run, 1, names("whole", 0, 1), set())
    assert settle(rec, cur, run, "now")[0] == ["whole"]
    run, rec = start(cur, 1), {}
    absorb_slice(run, 1, names("whole", 0), set())
    assert settle(rec, cur, run, "now")[1] == ["whole"]

    # A tree that changed between slices is another run.
    a, b = current(x=1), current(x=1)
    b["x"]["fingerprint"] = "other"
    part = parse_partition("1/2")
    assert run_id(part, False, a) != run_id(part, False, b)
    assert run_id(part, False, a) != run_id(part, True, a)
    assert run_id(part, False, a) != run_id(parse_partition("1/3"), False, a)
    assert run_id(part, False, a) == run_id(parse_partition("2/2"), False, a)
    print("test-data selftest ok")
    return 0


if __name__ == "__main__":
    sys.exit(selftest() if sys.argv[1:] == ["selftest"] else main())
