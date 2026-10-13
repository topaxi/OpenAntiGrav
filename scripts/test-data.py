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


def green_binaries(log: str) -> tuple[dict[str, int], set[str]]:
    """Per binary, how many tests printed PASS, and which printed anything else."""
    passed: dict[str, int] = {}
    other: set[str] = set()
    for line in log.splitlines():
        if found := RESULT.match(line):
            status, binary, _ = found.groups()
            if status.strip() == "PASS":
                passed[binary] = passed.get(binary, 0) + 1
            else:
                other.add(binary)
    return passed, other


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


# The line `check-test-budget.py` looks for: a log carrying it is a partial run,
# and its wall clock says nothing about the whole suite's.
PARTIAL_MARKER = "test-data: PARTIAL RUN"


class Tee:
    """Everything said goes to the terminal and to `target/test-data.log`."""

    def __init__(self) -> None:
        LOG.parent.mkdir(parents=True, exist_ok=True)
        self.log = LOG.open("w")

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


def nextest_run(skipped: list[str]) -> list[str]:
    cmd = [*NEXTEST[:2], "run", *NEXTEST[2:], "--no-fail-fast", "--status-level", "pass"]
    if skipped:
        excluded = " | ".join(f"binary_id(={b})" for b in skipped)
        cmd += ["-E", f"not ({excluded})"]
    return cmd


def main() -> int:
    args = sys.argv[1:]
    full = "--full" in args or os.environ.get("OAG_TEST_DATA_FULL", "") not in ("", "0")
    unknown = [a for a in args if a != "--full"]
    if unknown:
        print(f"test-data: unknown argument(s) {unknown}; only --full", file=sys.stderr)
        return 2

    # `oag-game`'s build script embeds the commit hash, which would make every
    # `oag-game` binary differ after every commit whether or not its code did.
    os.environ.setdefault("OAG_GIT_HASH", "test-data")

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
    skipped = []
    if not full:
        skipped = sorted(
            b
            for b, f in current.items()
            if f["fingerprint"] in recorded.get(b, {})
        )
    skipped_tests = sum(current[b]["tests"] for b in skipped)
    total_tests = sum(f["tests"] for f in current.values())

    out = Tee()
    out.say(
        f"test-data: fingerprinted {len(current)} test binaries in "
        f"{time.time() - started:.1f}s"
        + ("; --full, running every one" if full else "")
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
                when = recorded[b][current[b]["fingerprint"]]
                out.say(f"    SKIPPED  {b}  ({current[b]['tests']} tests, green {when})")

    report_skips(detail=True)
    out.say()
    if len(skipped) == len(current):
        out.say("test-data: every test binary is skipped, nothing to run")
        tests = 0
    else:
        tests = out.run(nextest_run(skipped))

    passed, other = green_binaries(LOG.read_text(errors="replace"))
    stamp = time.strftime("%Y-%m-%d %H:%M")
    ran = [b for b in current if b not in skipped]
    newly, failed = 0, []
    for b in ran:
        fingerprint = current[b]["fingerprint"]
        greens = recorded.get(b, {})
        greens.pop(fingerprint, None)
        if passed.get(b, 0) == current[b]["tests"] and b not in other:
            greens = {fingerprint: stamp, **dict(list(greens.items())[: KEEP - 1])}
            newly += 1
        else:
            failed.append(b)
        if greens:
            recorded[b] = greens
        else:
            recorded.pop(b, None)
    for b in list(recorded):
        if b not in current:
            del recorded[b]  # a binary that no longer exists
    record["hash-cache"] = {k: v for k, v in cache.items() if Path(k).is_file()}
    save_record(record)

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
        f"test-data: {newly} of {len(ran)} test binaries that ran passed in full and "
        f"are recorded; {len(failed)} did not"
        + (f": {', '.join(failed)}" if failed else "")
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
            [sys.executable, str(ROOT / "scripts/check-test-budget.py"), str(LOG)],
            cwd=ROOT,
        ).returncode
    return tests if tests else budget


if __name__ == "__main__":
    sys.exit(main())
