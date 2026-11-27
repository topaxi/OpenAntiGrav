#!/usr/bin/env python3
"""Nothing in a test, a script or a recipe writes under the OS temp directory.

On Linux `/tmp` is often a `tmpfs` with a per-user quota; test leftovers once
filled it and every shell and gate on the machine failed with `EDQUOT`. Tests
write through `oag_testdata::scratch` (guard-removed, under `target/tmp`),
Python through `scripts/oag_tmp.py`, shell through `scripts/oag-tmp.sh`.

What fails, outside comments and docstrings:

- Rust: `env::temp_dir`, the `tempfile` crate, a `"/tmp` literal, and
  `CARGO_TARGET_TMPDIR` (a fixed path nothing removes: use `scratch`).
- Python: a string holding `/tmp`, `tempfile.gettempdir`, `TMPDIR`, and a
  `tempfile` constructor without `dir=`.
- Shell, `justfile`, nextest and Cargo config, Java: `/tmp`, `TMPDIR`, and
  `mktemp` without `-p`.

A use that is right goes in `ALLOWED` with its reason; an entry nothing matches
fails too, so the list only shrinks on its own.

Run: `just check-temp-use` (no build). `--selftest` breaks each rule on made-up
sources.
"""

import ast
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# (path, text the flagged line holds) -> why it stays.
ALLOWED = {
    ("crates/mod/src/tests.rs", "/tmp/.mount_x"): "a made-up AppImage mount point; nothing is written",
    ("crates/source/src/source/tests.rs", "/tmp/.mount_"): "asserts no AppImage mount path leaks into a message",
    ("crates/schema/src/tests.rs", "file:///tmp/anywhere"): "a URL that is only parsed",
    ("crates/game/src/shot_meta/tests.rs", '"/tmp/a.wav"'): "a command-line string that is only compared",
    ("crates/game/tests/settings_schema.rs", '"/tmp/oag.log"'): "a settings value that is only serialised",
    ("scripts/pcsx2_pine.py", "/tmp"): "PCSX2 itself puts its PINE socket there without a runtime directory",
    ("scripts/xvfb-own.sh", "/tmp"): "the X server's own socket and lock directories",
}

SCANNED = ("crates", "scripts", "justfile", ".config", ".cargo", "Cargo.toml")
SKIPPED = {"scripts/check-temp-use.py"}

TMP = re.compile(r"(?<![\w])/tmp(?![\w-])")
RUST = (
    (re.compile(r"\benv::temp_dir\b|\buse\s+std::env::\{[^}]*\btemp_dir\b"), "std::env::temp_dir"),
    (re.compile(r"\btempfile::|\buse\s+tempfile\b"), "the tempfile crate"),
    (re.compile(r'"(?:file://)?/tmp(?![\w-])'), "a /tmp literal"),
    (re.compile(r"\bCARGO_TARGET_TMPDIR\b"), "CARGO_TARGET_TMPDIR"),
)
CONSTRUCTORS = {
    "TemporaryDirectory",
    "TemporaryFile",
    "NamedTemporaryFile",
    "SpooledTemporaryFile",
    "mkdtemp",
    "mkstemp",
}
MKTEMP = re.compile(r"\bmktemp\b(?![^\n)`]*\s-p\b)")


def rust(text):
    for number, line in enumerate(text.split("\n"), 1):
        if line.lstrip().startswith("//"):
            continue
        for pattern, what in RUST:
            if pattern.search(line):
                yield number, what, line


def python(text):
    tree = ast.parse(text)
    docstrings = {
        id(node.value)
        for node in ast.walk(tree)
        if isinstance(node, ast.Expr) and isinstance(node.value, ast.Constant)
    }
    lines = text.split("\n")
    found = []
    for node in ast.walk(tree):
        what = None
        if isinstance(node, ast.Constant) and isinstance(node.value, str) and id(node) not in docstrings:
            if TMP.search(node.value):
                what = "a /tmp string"
            elif node.value == "TMPDIR":
                what = "TMPDIR"
        elif isinstance(node, ast.Attribute) and node.attr == "gettempdir":
            what = "tempfile.gettempdir"
        elif isinstance(node, ast.ImportFrom) and node.module == "tempfile":
            what = "from tempfile import (write tempfile.X(dir=...) so it can be checked)"
        elif (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Attribute)
            and isinstance(node.func.value, ast.Name)
            and node.func.value.id == "tempfile"
            and node.func.attr in CONSTRUCTORS
            and not any(keyword.arg == "dir" for keyword in node.keywords)
        ):
            what = f"tempfile.{node.func.attr} without dir="
        if what:
            found.append((node.lineno, what, lines[node.lineno - 1]))
    return sorted(found)


def plain(text, comment):
    for number, line in enumerate(text.split("\n"), 1):
        if line.lstrip().startswith(comment):
            continue
        if TMP.search(line):
            yield number, "/tmp", line
        elif "TMPDIR" in line and "CARGO_TARGET_TMPDIR" not in line:
            yield number, "TMPDIR", line
        elif MKTEMP.search(line):
            yield number, "mktemp without -p", line


def check(path, text):
    if path.endswith(".rs"):
        return rust(text)
    if path.endswith(".py"):
        return python(text)
    if path.endswith(".java"):
        return plain(text, "//")
    if path.endswith((".sh", ".toml")) or path == "justfile":
        return plain(text, "#")
    return ()


def tracked():
    listed = subprocess.run(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", *SCANNED],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return sorted(p for p in listed.split("\0") if p and p not in SKIPPED and (ROOT / p).is_file())


def selftest():
    broken = {
        "a.rs": [
            "let d = std::env::temp_dir();",
            "use std::env::{self, temp_dir};",
            'let p = "/tmp/x";',
            "let d = tempfile::tempdir();",
            'env!("CARGO_TARGET_TMPDIR")',
        ],
        "a.py": [
            'open("/tmp/x")',
            "import tempfile\ntempfile.mkdtemp()",
            "import tempfile\nwith tempfile.TemporaryDirectory(prefix='x') as t: pass",
            "import tempfile\ntempfile.gettempdir()",
            "from tempfile import mkstemp",
        ],
        "a.sh": ["d=$(mktemp -d)", "echo > /tmp/x", "d=${TMPDIR:-x}", "f=$(mktemp -u)"],
        "justfile": ['out="/tmp":'],
    }
    fine = {
        "a.rs": ["// std::env::temp_dir()", 'let d = oag_testdata::scratch::dir("x");', 'let p = "/var/tmp";'],
        "a.py": ['"""Run with --out /tmp/x."""', "# /tmp\nimport tempfile\ntempfile.mkdtemp(dir=here)"],
        "a.sh": ['d=$(mktemp -d -p "$(oag_tmp_root)")', "# mktemp writes /tmp", "adb push x /data/local/tmp-x"],
    }
    for path, cases in broken.items():
        for case in cases:
            assert list(check(path, case)), f"not caught: {path}: {case!r}"
    for path, cases in fine.items():
        for case in cases:
            assert not list(check(path, case)), f"wrongly caught: {path}: {case!r}"
    print("check-temp-use: selftest ok")


def main():
    if "--selftest" in sys.argv[1:]:
        selftest()
        return 0
    used, failures, paths = set(), [], tracked()
    for path in paths:
        text = (ROOT / path).read_text(errors="replace")
        for number, what, line in check(path, text):
            allowed = [key for key in ALLOWED if key[0] == path and key[1] in line]
            used.update(allowed)
            if not allowed:
                failures.append(f"{path}:{number}: {what}: {line.strip()}")
    failures += [f"stale ALLOWED entry: {path}: {text}" for path, text in sorted(set(ALLOWED) - used)]
    if failures:
        print("\n".join(failures))
        print(
            "\ncheck-temp-use: write through oag_testdata::scratch (Rust), scripts/oag_tmp.py"
            " or scripts/oag-tmp.sh, or allow the line with its reason in this script."
        )
        return 1
    print(f"check-temp-use: {len(paths)} files, {len(ALLOWED)} allowed uses")
    return 0


if __name__ == "__main__":
    sys.exit(main())
