#!/usr/bin/env python3
"""Which packages a branch's change can affect, for `just gate-affected`.

A member lane changes one or two crates, and the full suite spends most of its
time on crates the lane never touched. This script reads what changed against
the merge base with `main` and turns it into the packages whose tests can see
the change: every package a changed file belongs to, plus everything that
depends on one of those, transitively, through normal, build and dev
dependencies alike (`oag-testdata` is a dev-dependency of nearly everything,
and a test binary links its dev-dependencies).

It prints one line on stdout for the recipe to act on:

- `full`  - run the whole workspace, as `just test` does;
- `none`  - nothing a test can observe changed;
- `-p a -p b ...` - those packages, ready to hand to `cargo nextest run`.

and, on stderr, every changed file with what it selected and why, so a
reviewer can check the selection rather than trust it.

# Escalation to the full suite

Some changes reach every test, or reach state every test hashes:

- `oag-core` and the simulation crates (`oag-physics`, `oag-gameplay`,
  `oag-ai`, `oag-race`, `oag-formats`). Their reverse dependencies are most of
  the workspace anyway, and a determinism slip is the regression a partial run
  is worst at missing.
- the workspace's build inputs: `Cargo.toml`, `Cargo.lock`,
  `rust-toolchain.toml`, `.cargo/`, `.config/` (nextest's own profile), the
  `justfile`, and any `build.rs`.
- a file this script cannot place: under `crates/` but in no package, or a
  non-crate path that no crate names and that is not known to be inert.

# Files outside crates

A file outside `crates/` matters only if some crate reads it, and every crate
that does names it by path: `include_str!("../../../assets/ui/menu.toml")`,
`repo_root().join("verification/scenarios/...")`. So the file's path and each
of its parent directories down to two components (`assets/ui/strings/x.toml`,
`assets/ui/strings`, `assets/ui`) are searched for, as plain text, in every
package's sources, and every package that names one is selected. This
over-selects on purpose: a comment naming the path selects its crate too.

Paths no crate can read are inert and select nothing: `docs/`, `handover/`,
`HANDOVER.md`, `.claude/`, `.github/`, `licences/`, `packaging/`, `scratch/`
and the root's Markdown and
licence files. Nothing under `crates/` reads them at run time (checked
2026-10-07: every `docs/` string in a test is a message, not a path read).
`scripts/` is searched like any other path (`oag-trace` includes
`scripts/psp_trace_fields.py`); a script nothing includes is a `check-*`
script, which the affected gate still runs in full.

# Changed files

`git diff --name-only <merge-base>` covers committed, staged and unstaged
changes alike; `git ls-files --others --exclude-standard` adds new files not
yet added. The base is `main` unless `OAG_AFFECTED_BASE` names another ref.
"""

import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

ESCALATING_PACKAGES = {
    "oag-core",
    "oag-physics",
    "oag-gameplay",
    "oag-ai",
    "oag-race",
    "oag-formats",
}

ESCALATING_FILES = {
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "rust-toolchain",
    "justfile",
}

ESCALATING_DIRS = (".cargo/", ".config/")

INERT_DIRS = (
    "docs/",
    "handover/",
    ".claude/",
    ".github/",
    "licences/",
    "packaging/",
    "scratch/",
)

INERT_ROOT_SUFFIXES = (".md",)

INERT_ROOT_FILES = {
    "LICENSE-APACHE",
    "LICENSE-MIT",
    "info",
    ".mcp.json",
    ".gitignore",
    "rustfmt.toml",
}

SOURCE_SUFFIXES = {".rs", ".toml", ".wgsl", ".wesl"}


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout


def changed_files(base: str) -> list[str]:
    merge_base = git("merge-base", "HEAD", base).strip()
    files = set(git("diff", "--name-only", merge_base).split())
    files |= set(git("ls-files", "--others", "--exclude-standard").split())
    return sorted(files)


def workspace() -> tuple[dict[str, Path], dict[str, set[str]]]:
    """Each package's directory, relative to the root, and who depends on it."""
    meta = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--format-version=1", "--no-deps"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    )
    dirs = {
        p["name"]: Path(p["manifest_path"]).parent.relative_to(ROOT)
        for p in meta["packages"]
    }
    dependents: dict[str, set[str]] = {name: set() for name in dirs}
    for package in meta["packages"]:
        for dep in package["dependencies"]:
            if dep["name"] in dirs:
                dependents[dep["name"]].add(package["name"])
    return dirs, dependents


def owner(path: str, dirs: dict[str, Path]) -> str | None:
    best = None
    for name, directory in dirs.items():
        prefix = f"{directory.as_posix()}/"
        if path.startswith(prefix) and (
            best is None or len(prefix) > len(f"{dirs[best].as_posix()}/")
        ):
            best = name
    return best


def needles(path: str) -> list[str]:
    parts = path.split("/")
    return ["/".join(parts[:n]) for n in range(len(parts), 1, -1)]


def readers(path: str, dirs: dict[str, Path], sources: dict[str, str]) -> set[str]:
    found = set()
    for needle in needles(path):
        for name, text in sources.items():
            if needle in text:
                found.add(name)
    return found


def package_sources(dirs: dict[str, Path]) -> dict[str, str]:
    texts = {}
    for name, directory in dirs.items():
        chunks = []
        for file in (ROOT / directory).rglob("*"):
            if file.suffix in SOURCE_SUFFIXES and file.is_file():
                chunks.append(file.read_text(errors="replace"))
        texts[name] = "\n".join(chunks)
    return texts


def closure(seeds: set[str], dependents: dict[str, set[str]]) -> set[str]:
    out, todo = set(seeds), list(seeds)
    while todo:
        for user in dependents[todo.pop()]:
            if user not in out:
                out.add(user)
                todo.append(user)
    return out


def classify(files: list[str]) -> tuple[str, set[str], list[str]]:
    """`(verdict, seed packages, one explanation line per file)`."""
    dirs, _ = workspace()
    sources = None
    seeds: set[str] = set()
    why: list[str] = []
    full = False
    for path in files:
        name = Path(path).name
        if path in ESCALATING_FILES or path.startswith(ESCALATING_DIRS):
            why.append(f"FULL   {path}: a workspace build input")
            full = True
            continue
        if name == "build.rs":
            why.append(f"FULL   {path}: a build script")
            full = True
            continue
        package = owner(path, dirs)
        if package in ESCALATING_PACKAGES:
            why.append(f"FULL   {path}: {package} is core or simulation")
            full = True
            continue
        if package is not None:
            seeds.add(package)
            why.append(f"crate  {path}: {package}")
            continue
        if path.startswith("crates/"):
            why.append(f"FULL   {path}: under crates/ but in no package")
            full = True
            continue
        if path.startswith(INERT_DIRS) or (
            "/" not in path
            and (path.endswith(INERT_ROOT_SUFFIXES) or path in INERT_ROOT_FILES)
        ):
            why.append(f"inert  {path}")
            continue
        if sources is None:
            sources = package_sources(dirs)
        found = readers(path, dirs, sources)
        if found & ESCALATING_PACKAGES:
            why.append(f"FULL   {path}: read by {', '.join(sorted(found))}")
            full = True
        elif found:
            seeds |= found
            why.append(f"reader {path}: named by {', '.join(sorted(found))}")
        elif path.startswith("scripts/"):
            why.append(f"inert  {path}: no crate names it; checks run in full")
        else:
            why.append(f"FULL   {path}: no crate names it and it is not known inert")
            full = True
    verdict = "full" if full else ("none" if not seeds else "some")
    return verdict, seeds, why


def main() -> int:
    base = os.environ.get("OAG_AFFECTED_BASE", "main")
    files = changed_files(base)
    verdict, seeds, why = classify(files)
    log = sys.stderr
    print(f"affected: {len(files)} file(s) changed against merge-base with {base}", file=log)
    for line in why:
        print(f"  {line}", file=log)
    if verdict == "full":
        print("affected: FULL suite (an escalating change, marked FULL above)", file=log)
        print("full")
        return 0
    if verdict == "none":
        print("affected: no package selected, no tests to run", file=log)
        print("none")
        return 0
    _, dependents = workspace()
    selected = closure(seeds, dependents)
    print(
        f"affected: {len(seeds)} changed package(s), "
        f"{len(selected)} with reverse dependencies:",
        file=log,
    )
    print(f"  changed:  {' '.join(sorted(seeds))}", file=log)
    print(f"  plus:     {' '.join(sorted(selected - seeds)) or '(none)'}", file=log)
    print(" ".join(f"-p {name}" for name in sorted(selected)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
