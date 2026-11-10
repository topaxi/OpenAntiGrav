#!/usr/bin/env python3
"""Which tests a branch's change can affect, for `just gate-affected`.

A member lane changes one or two crates, and the full suite spends most of its
time on tests the lane cannot reach. This script reads what changed against the
merge base with `main` and turns it into the tests that link the change. The
measurements behind every rule, and the case each one could miss, are in
`docs/architecture/affected-gate.md`.

It prints on stdout, for the recipe to act on:

- `full`  - run the whole workspace, as `just test` does;
- `none`  - nothing a test can observe changed;
- otherwise the arguments for `cargo nextest run`, one per line: a `-p` pair
  per selected package (so only those are built) and one `-E` filterset that
  narrows a package to the test binaries and test modules the change reaches.

and, on stderr, every changed file with what it selected and why, so a
reviewer can check the selection rather than trust it.

# What a package's tests link

A test binary of package `P` links `P`, what `P` depends on through normal and
build dependencies, transitively, and `P`'s dev-dependencies with what those
depend on. A dev-dependency edge ends at `P`: a package that depends on `P`
links `P`'s library, never `P`'s dev-dependencies. Optional dependencies count
when a feature switches them on, resolved as `cargo test -p P` resolves them:
`P`'s default features, and the features each edge asks for.

# Narrowing inside a package

A selected package runs in full unless one of these holds:

- the change is only in files under its `tests/`: the test modules that own
  those files, and the modules that name them through `super::` or `crate::`.
  No dependent links a test file, so none is selected.
- the change is only in a `#[cfg(test)] mod tests` under `src/`: the unit
  tests, and no dependent.
- the change is in its `src/`, or reaches it through a dependency: only the
  test modules whose code names a crate that links the change. A module that
  names the package's own library reaches whatever the library links; one
  that never names it, and reads a file through another crate alone, does not.
  A module that spawns the package's binary or reads a file out of `src/`
  counts as naming the package.

# Escalation to the full suite

- `Cargo.toml` at the root, `rust-toolchain.toml`, `.cargo/`, and any file in
  `.config/` other than `nextest.toml`.
- a `Cargo.lock` change that touches a package this script cannot place.
- a file this script cannot place: under `crates/` but in no package, or a
  non-crate path that no crate names and that is not known to be inert.

`oag-core` and the simulation crates are not escalated: everything that links
one is selected by the rule above, which is every test that can see a
determinism slip, and `check-determinism` runs in full on every gate. Such a
change is named on stderr, and the lead gates that merge at once.

# Changes that select nothing

- a `.rs` change that leaves the code unchanged once comments are removed.
- `.config/nextest.toml`, which schedules tests and never changes one.
- the `justfile`: no test runs a recipe or reads one (`oag-source` only asks
  whether the file exists), and every check recipe runs in full.
- `docs/`, `handover/`, `HANDOVER.md`, `.claude/`, `.github/`, `licences/`,
  `packaging/`, `scratch/` and the root's Markdown and licence files.

# Files outside crates

A file outside `crates/` matters only if some crate reads it, and every crate
that does names it by path: `include_str!("../../../assets/ui/menu.toml")`,
`repo_root().join("verification/scenarios/...")`. So the file's path and each
of its parent directories down to two components are searched for, as plain
text, in every package. A package that names one under `src/` is treated as
changed; one that names it only under `tests/` runs the test modules naming
it. A comment naming the path selects too, on purpose. A data file inside a
crate (a fixture another crate's test reads) is searched for the same way.

# Changed files

`git diff --name-only <merge-base>` covers committed, staged and unstaged
changes alike; `git ls-files --others --exclude-standard` adds new files not
yet added. The base is `main` unless `OAG_AFFECTED_BASE` names another ref.
"""

import json
import os
import re
import subprocess
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# A change here selects by the same rule as any other, and is also named on
# stderr, because the lead gates such a merge at once rather than per batch.
SIMULATION_PACKAGES = {
    "oag-core",
    "oag-physics",
    "oag-gameplay",
    "oag-ai",
    "oag-race",
    "oag-formats",
}

ESCALATING_FILES = {
    "Cargo.toml",
    "rust-toolchain.toml",
    "rust-toolchain",
}

ESCALATING_DIRS = (".cargo/", ".config/")

SCHEDULING_FILES = {".config/nextest.toml"}

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

# The unit tests of a package's library and binaries, as one unit.
UNIT = ("unit", "")

MOD_DECL = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;", re.M)
CFG_TEST_MOD = re.compile(
    r"#\[cfg\(test\)\]\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+tests\s*;"
)
# A test module that spawns its package's binary, or reads a file of its `src/`.
OWN_PACKAGE = re.compile(r"CARGO_BIN_EXE|\bsrc/")
SIBLING = re.compile(r"\b(?:super|crate)::(\w+|\*|\{)")
# A line comment, or a literal a comment marker may sit inside: a raw string, a
# string, a character.
TOKEN = re.compile(
    r"""//[^\n]*|\b[bc]?r(#*)".*?"\1|"(?:\\.|[^"\\])*"|'(?:\\.[^']*|[^\\'])'""", re.S
)


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout


def changed_files(base: str) -> tuple[str, list[str]]:
    merge_base = git("merge-base", "HEAD", base).strip()
    files = set(git("diff", "--name-only", merge_base).split())
    files |= set(git("ls-files", "--others", "--exclude-standard").split())
    return merge_base, sorted(files)


def content(path: str, rev: str | None) -> str | None:
    """A file's text at a revision, or in the working tree for `None`."""
    if rev is None:
        file = ROOT / path
        return file.read_text(errors="replace") if file.is_file() else None
    shown = subprocess.run(
        ["git", "show", f"{rev}:{path}"], cwd=ROOT, capture_output=True, text=True
    )
    return shown.stdout if shown.returncode == 0 else None


def strip_comments(source: str) -> str:
    """Rust source without its comments, blank lines and trailing spaces."""
    if "/*" in source:
        kept = _strip_nested(source)
    else:
        kept = TOKEN.sub(lambda m: "" if m.group(0).startswith("//") else m.group(0), source)
    lines = (line.rstrip() for line in kept.splitlines())
    return "\n".join(line for line in lines if line)


def _strip_nested(source: str) -> str:
    """The same for a file with block comments, which nest."""
    out, i, n = [], 0, len(source)
    while i < n:
        if source.startswith("/*", i):
            depth, i = 1, i + 2
            while i < n and depth:
                if source.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif source.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    i += 1
        elif token := TOKEN.match(source, i):
            if not token.group(0).startswith("//"):
                out.append(token.group(0))
            i = token.end()
        else:
            out.append(source[i])
            i += 1
    return "".join(out)


def only_comments_changed(path: str, base: str, head: str | None) -> bool:
    before, after = content(path, base), content(path, head)
    if before is None or after is None:
        return False
    return strip_comments(before) == strip_comments(after)


@dataclass
class Target:
    """One integration test binary and the test modules its root declares."""

    name: str
    root: str
    modules: dict[str, list[str]] = field(default_factory=dict)


@dataclass
class Package:
    name: str
    directory: str
    features: dict[str, list[str]]
    dependencies: list[dict]
    lib_crate: str | None = None
    targets: list[Target] = field(default_factory=list)


class Workspace:
    def __init__(self) -> None:
        meta = json.loads(
            subprocess.run(
                ["cargo", "metadata", "--format-version=1", "--no-deps"],
                cwd=ROOT,
                check=True,
                capture_output=True,
                text=True,
            ).stdout
        )
        self.packages: dict[str, Package] = {}
        for p in meta["packages"]:
            directory = Path(p["manifest_path"]).parent.relative_to(ROOT).as_posix()
            package = Package(p["name"], directory, p["features"], p["dependencies"])
            for target in p["targets"]:
                if "lib" in target["kind"] or "proc-macro" in target["kind"]:
                    package.lib_crate = target["name"].replace("-", "_")
                if target["kind"] == ["test"]:
                    root = Path(target["src_path"]).relative_to(ROOT).as_posix()
                    package.targets.append(Target(target["name"], root))
            self.packages[p["name"]] = package
        self.crates = {
            p.lib_crate: name for name, p in self.packages.items() if p.lib_crate
        }
        self._texts: dict[str, str] = {}
        self._files: dict[str, list[str]] = {}
        self._resolved: dict[str, tuple[dict[str, set[str]], set[str]]] = {}
        self._units: dict[str, dict[tuple[str, str], set[str]]] = {}
        for package in self.packages.values():
            for target in package.targets:
                self._read_modules(target)

    def text(self, path: str) -> str:
        if path not in self._texts:
            file = ROOT / path
            self._texts[path] = (
                file.read_text(errors="replace") if file.is_file() else ""
            )
        return self._texts[path]

    def files(self, directory: str) -> list[str]:
        if directory not in self._files:
            base = ROOT / directory
            self._files[directory] = sorted(
                f.relative_to(ROOT).as_posix()
                for f in (base.rglob("*") if base.is_dir() else ())
                if f.suffix in SOURCE_SUFFIXES and f.is_file()
            )
        return self._files[directory]

    def _read_modules(self, target: Target) -> None:
        """The `mod x;` lines of a test binary's root, each with its files.

        A root that redirects a module with `#[path]` is left as one unit.
        """
        root = self.text(target.root)
        if "#[path" in root:
            return
        directory = Path(target.root).parent
        for name in MOD_DECL.findall(root):
            files = [
                f
                for f in (
                    (directory / f"{name}.rs").as_posix(),
                    *self.files((directory / name).as_posix()),
                )
                if (ROOT / f).is_file()
            ]
            if files:
                target.modules[name] = files

    def owner(self, path: str) -> str | None:
        best = None
        for name, package in self.packages.items():
            prefix = f"{package.directory}/"
            if path.startswith(prefix) and (
                best is None or len(prefix) > len(self.packages[best].directory) + 1
            ):
                best = name
        return best

    def resolve(self, root: str) -> tuple[dict[str, set[str]], set[str]]:
        """What `cargo test -p root` links, among the workspace's packages.

        Returns each reached package's normal and build dependencies that are
        switched on, and `root`'s dev-dependencies.
        """
        if root in self._resolved:
            return self._resolved[root]
        features: dict[str, set[str]] = {root: set()}
        optional: dict[str, set[str]] = {}
        edges: dict[str, set[str]] = {}
        dev: set[str] = set()

        def enable(package: str, feature: str) -> bool:
            have = features.setdefault(package, set())
            if feature in have:
                return False
            have.add(feature)
            declared = self.packages[package].features
            if feature not in declared:
                optional.setdefault(package, set()).add(feature)
                return True
            for item in declared[feature]:
                if item.startswith("dep:"):
                    optional.setdefault(package, set()).add(item[4:])
                elif "/" in item:
                    dep = item.split("/")[0]
                    if not dep.endswith("?"):
                        optional.setdefault(package, set()).add(dep)
                else:
                    enable(package, item)
            return True

        enable(root, "default")
        changed = True
        while changed:
            changed = False
            for package in list(features):
                on = optional.get(package, set())
                for dep in self.packages[package].dependencies:
                    name = dep["name"]
                    key = dep.get("rename") or name
                    if name not in self.packages:
                        continue
                    if dep["kind"] == "dev" and package != root:
                        continue
                    if dep["optional"] and key not in on:
                        continue
                    bucket = (
                        dev if dep["kind"] == "dev" else edges.setdefault(package, set())
                    )
                    if name not in bucket:
                        bucket.add(name)
                        changed = True
                    wanted = list(dep["features"])
                    if dep["uses_default_features"]:
                        wanted.append("default")
                    for have in list(features[package]):
                        for item in self.packages[package].features.get(have, []):
                            if "/" in item and item.split("/")[0].rstrip("?") == key:
                                wanted.append(item.split("/")[1])
                    features.setdefault(name, set())
                    for feature in wanted:
                        changed |= enable(name, feature)
        self._resolved[root] = (edges, dev)
        return edges, dev

    def reach(self, root: str, start: str) -> set[str]:
        """`start` and what it links, inside `root`'s resolution.

        A package `root` does not link is resolved on its own.
        """
        edges, dev = self.resolve(root)
        if start != root and start not in dev and not any(start in d for d in edges.values()):
            edges, _ = self.resolve(start)
        out, todo = {start}, [start]
        while todo:
            for dep in edges.get(todo.pop(), ()):
                if dep not in out:
                    out.add(dep)
                    todo.append(dep)
        return out

    def links(self, package: str) -> set[str]:
        """Every workspace package a test binary of `package` links."""
        _, dev = self.resolve(package)
        out = self.reach(package, package)
        for dep in dev:
            out |= self.reach(package, dep)
        return out

    def named(self, text: str) -> set[str]:
        """The workspace packages a source's code names, as a crate or a package."""
        text = strip_comments(text)
        found = {
            self.crates[c] for c in re.findall(r"\boag_\w+", text) if c in self.crates
        }
        found |= {
            p for p in re.findall(r"\boag-[a-z0-9-]+", text) if p in self.packages
        }
        return found

    def units(self, package: str) -> dict[tuple[str, str], set[str]]:
        """Each test unit of a package and the packages its source names.

        A unit is the unit tests (`UNIT`), a test module `(binary, module)`, or
        a whole test binary `(binary, "")` when its root declares no modules.
        A module also carries what the root and the sibling modules it refers
        to name; a glob over the root carries every sibling.
        """
        if package in self._units:
            return self._units[package]
        p = self.packages[package]
        edges, dev = self.resolve(package)
        normal = edges.get(package, set())
        src = "\n".join(self.text(f) for f in self.files(f"{p.directory}/src"))
        units = {UNIT: {package} | normal | (self.named(src) & dev)}
        for target in p.targets:
            root = self.named(self.text(target.root))
            if OWN_PACKAGE.search(self.text(target.root)):
                root.add(package)
            if not target.modules:
                units[(target.name, "")] = root | {package}
                continue
            own, refers = {}, {}
            for module, files in target.modules.items():
                text = "\n".join(self.text(f) for f in files)
                own[module] = self.named(text)
                if OWN_PACKAGE.search(text):
                    own[module].add(package)
                refers[module] = self._refers(target, module)
            for module in target.modules:
                names, todo, done = set(root), [module], set()
                while todo:
                    m = todo.pop()
                    if m in done:
                        continue
                    done.add(m)
                    names |= own[m]
                    todo.extend(refers[m])
                units[(target.name, module)] = names
        self._units[package] = units
        return units

    def _refers(self, target: Target, module: str) -> set[str]:
        """The sibling modules a test module's source refers to."""
        text = "\n".join(self.text(f) for f in target.modules[module])
        seen = set(SIBLING.findall(text))
        if seen & {"*", "{"}:
            return set(target.modules) - {module}
        return seen & set(target.modules) - {module}

    def users(self, target: Target, module: str) -> set[str]:
        """`module` and every module of its binary that refers to it."""
        out, grew = {module}, True
        while grew:
            grew = False
            for other in target.modules:
                if other not in out and self._refers(target, other) & out:
                    out.add(other)
                    grew = True
        return out


@dataclass
class Selection:
    """Per package, every test (`None`) or a set of units."""

    packages: dict[str, set[tuple[str, str]] | None] = field(default_factory=dict)

    def add(self, package: str, units: set[tuple[str, str]] | None) -> None:
        if units is None or self.packages.get(package, set()) is None:
            self.packages[package] = None
        elif units:
            self.packages.setdefault(package, set()).update(units)

    def matches(self, package: str, kind: str, binary: str, test: str) -> bool:
        """Whether one test is selected; `kind` is `unit` or `test`."""
        if package not in self.packages:
            return False
        units = self.packages[package]
        if units is None:
            return True
        if kind == "unit":
            return UNIT in units
        if (binary, "") in units:
            return True
        modules = {m for b, m in units if b == binary}
        return bool(modules) and ("::" not in test or test.split("::")[0] in modules)

    def filterset(self) -> str:
        parts = []
        for package in sorted(self.packages):
            units = self.packages[package]
            if units is None:
                parts.append(f"package(={package})")
                continue
            inner = ["kind(lib) | kind(bin)"] if UNIT in units else []
            for binary in sorted({b for b, _ in units if (b, _) != UNIT}):
                if (binary, "") in units:
                    inner.append(f"(kind(test) & binary(={binary}))")
                    continue
                modules = "|".join(sorted(m for b, m in units if b == binary))
                inner.append(
                    f"(kind(test) & binary(={binary})"
                    f" & test(/^(({modules})::|[^:]+$)/))"
                )
            parts.append(f"(package(={package}) & ({' | '.join(inner)}))")
        return " | ".join(parts)

    def arguments(self) -> list[str]:
        out = []
        for package in sorted(self.packages):
            out += ["-p", package]
        return [*out, "-E", self.filterset()]


def needles(path: str, inside: str | None = None) -> list[str]:
    """The texts that name a file: its path, then each parent directory.

    Outside a crate, down to two components. Inside the crate at `inside`,
    relative to the crate's parent (another crate reads it as
    `crates/game/tests/fixtures/x` or `../game/tests/fixtures/x`) and down to
    the directory below the crate's `tests/` or `src/`, because the crate's
    own name alone is in every other comment.
    """
    parts = path.split("/")
    if inside is None:
        return ["/".join(parts[:n]) for n in range(len(parts), 1, -1)] or [path]
    skip = len(inside.split("/")) - 1
    return ["/".join(parts[skip:n]) for n in range(len(parts), skip + 2, -1)]


def lock_packages(text: str | None) -> dict[str, str]:
    if text is None:
        return {}
    entries: dict[str, list[str]] = {}
    for package in tomllib.loads(text).get("package", []):
        entries.setdefault(package["name"], []).append(
            json.dumps(package, sort_keys=True)
        )
    return {name: "\n".join(sorted(rows)) for name, rows in entries.items()}


def lock_dependents(names: set[str]) -> set[str] | None:
    """The workspace packages that link any of `names`; `None` if one is unknown."""
    meta = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--format-version=1", "--locked", "--offline"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    )
    name_of = {p["id"]: p["name"] for p in meta["packages"]}
    if not names <= set(name_of.values()):
        return None
    deps = {n["id"]: n["dependencies"] for n in meta["resolve"]["nodes"]}
    found = set()
    for member in meta["workspace_members"]:
        seen, todo = {member}, [member]
        while todo:
            for dep in deps[todo.pop()]:
                if dep not in seen:
                    seen.add(dep)
                    todo.append(dep)
        if names & {name_of[i] for i in seen - {member}}:
            found.add(name_of[member])
    return found


class Classifier:
    def __init__(self, ws: Workspace, base: str, head: str | None) -> None:
        self.ws, self.base, self.head = ws, base, head
        self.changed: set[str] = set()
        # The changed packages with a change outside `src/`: a manifest, a
        # build script, a data file, a lock entry. Those run in full.
        self.whole: set[str] = set()
        self.selection = Selection()
        self.why: list[str] = []
        self.full = False

    def escalate(self, path: str, reason: str) -> None:
        self.why.append(f"FULL   {path}: {reason}")
        self.full = True

    def readers(self, path: str, skip: str | None = None) -> bool:
        """Select what names `path`; whether anything did.

        `skip` is the package that owns the file, already selected.
        """
        found = []
        wanted = needles(path, self.ws.packages[skip].directory if skip else None)
        for name, package in self.ws.packages.items():
            if name == skip:
                continue
            tests = {t.root for t in package.targets}
            tests |= {f for t in package.targets for fs in t.modules.values() for f in fs}
            rest = [f for f in self.ws.files(package.directory) if f not in tests]
            if any(n in self.ws.text(f) for f in rest for n in wanted):
                self.changed.add(name)
                self.whole.add(name)
                found.append(name)
                continue
            units = set()
            for target in package.targets:
                if any(n in self.ws.text(target.root) for n in wanted):
                    units.add((target.name, ""))
                for module, files in target.modules.items():
                    if any(n in self.ws.text(f) for f in files for n in wanted):
                        units |= {(target.name, m) for m in self.ws.users(target, module)}
            if units:
                self.selection.add(name, units)
                found.append(f"{name} (tests)")
        if found:
            self.why.append(f"reader {path}: named by {', '.join(sorted(found))}")
        return bool(found)

    def test_file(self, path: str, package: str) -> bool:
        """A file under a package's `tests/`: the modules that own it."""
        p = self.ws.packages[package]
        if not path.startswith(f"{p.directory}/tests/"):
            return False
        units: set[tuple[str, str]] | None = set()
        for target in p.targets:
            if path == target.root:
                units.add((target.name, ""))
            for module, files in target.modules.items():
                if path in files:
                    units |= {(target.name, m) for m in self.ws.users(target, module)}
        if not units or not path.endswith(".rs"):
            units = None
        self.selection.add(package, units)
        what = "every test" if units is None else f"{len(units)} test unit(s)"
        self.why.append(f"tests  {path}: {package}, {what}, no dependent")
        if not path.endswith(".rs"):
            self.readers(path, skip=package)
        return True

    def unit_test_file(self, path: str, package: str) -> bool:
        """A file of a `#[cfg(test)] mod tests` under `src/`: the unit tests."""
        src = f"{self.ws.packages[package].directory}/src"
        if not (path.startswith(f"{src}/") and path.endswith(".rs")):
            return False
        parts = Path(path).relative_to(src).parts
        at = next(
            (i for i, part in enumerate(parts) if part in ("tests", "tests.rs")), None
        )
        if at is None:
            return False
        parent = Path(src, *parts[:at])
        candidates = (
            [parent / "lib.rs", parent / "main.rs"]
            if at == 0
            else [parent.with_suffix(".rs"), parent / "mod.rs"]
        )
        if not any(CFG_TEST_MOD.search(self.ws.text(c.as_posix())) for c in candidates):
            return False
        self.selection.add(package, {UNIT})
        self.why.append(f"tests  {path}: {package}, unit tests, no dependent")
        return True

    def lock(self, path: str) -> None:
        before = lock_packages(content(path, self.base))
        after = lock_packages(content(path, self.head))
        names = {
            n for n in before.keys() | after.keys() if before.get(n) != after.get(n)
        }
        members = names & self.ws.packages.keys()
        found = lock_dependents(names - members - (names - after.keys()))
        if found is None:
            self.escalate(path, "a locked package this workspace does not resolve")
            return
        self.changed |= found | members
        self.whole |= found | members
        self.why.append(
            f"lock   {path}: {', '.join(sorted(names)) or 'nothing'}"
            f" -> {', '.join(sorted(found | members)) or 'no package'}"
        )

    def file(self, path: str) -> None:
        if path in SCHEDULING_FILES:
            self.why.append(f"inert  {path}: schedules tests, changes none")
            return
        if path in ESCALATING_FILES or path.startswith(ESCALATING_DIRS):
            self.escalate(path, "a workspace build input")
            return
        if path == "Cargo.lock":
            self.lock(path)
            return
        if path == "justfile":
            self.why.append(f"inert  {path}: no test reads a recipe; checks run in full")
            return
        package = self.ws.owner(path)
        if package is not None:
            if path.endswith(".rs") and only_comments_changed(path, self.base, self.head):
                self.why.append(f"inert  {path}: comments only")
            elif not (self.test_file(path, package) or self.unit_test_file(path, package)):
                self.changed.add(package)
                if not path.startswith(f"{self.ws.packages[package].directory}/src/"):
                    self.whole.add(package)
                self.why.append(f"crate  {path}: {package}")
                if Path(path).suffix not in (".rs", ".wgsl", ".wesl") and not path.endswith(
                    "Cargo.toml"
                ):
                    self.readers(path, skip=package)
            return
        if path.startswith("crates/"):
            self.escalate(path, "under crates/ but in no package")
        elif path.startswith(INERT_DIRS) or (
            "/" not in path
            and (path.endswith(INERT_ROOT_SUFFIXES) or path in INERT_ROOT_FILES)
        ):
            self.why.append(f"inert  {path}")
        elif self.readers(path):
            pass
        elif path.startswith("scripts/"):
            self.why.append(f"inert  {path}: no crate names it; checks run in full")
        else:
            self.escalate(path, "no crate names it and it is not known inert")

    def dependents(self) -> None:
        """Every package that links a changed one, narrowed to the units that do."""
        for name in self.ws.packages:
            if name in self.whole:
                self.selection.add(name, None)
                continue
            if not self.ws.links(name) & self.changed:
                continue
            every = self.ws.units(name)
            units = {
                unit
                for unit, names in every.items()
                if any(self.ws.reach(name, n) & self.changed for n in names)
            }
            self.selection.add(name, None if len(units) == len(every) else units)


def classify(
    files: list[str], base: str, head: str | None = None, ws: Workspace | None = None
) -> tuple[str, Selection, list[str], set[str]]:
    """`(verdict, selection, one explanation line per file, changed packages)`."""
    classifier = Classifier(ws or Workspace(), base, head)
    for path in files:
        classifier.file(path)
    classifier.dependents()
    selection = classifier.selection
    verdict = "full" if classifier.full else ("some" if selection.packages else "none")
    return verdict, selection, classifier.why, classifier.changed


def main() -> int:
    base = os.environ.get("OAG_AFFECTED_BASE", "main")
    merge_base, files = changed_files(base)
    ws = Workspace()
    verdict, selection, why, changed = classify(files, merge_base, None, ws)
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
    if simulation := sorted(changed & SIMULATION_PACKAGES):
        print(
            f"affected: core or simulation changed ({' '.join(simulation)});"
            " the lead runs the full gate at once after this merge",
            file=log,
        )
    whole = sorted(p for p, u in selection.packages.items() if u is None)
    print(
        f"affected: {len(changed)} changed package(s), "
        f"{len(selection.packages)} selected:",
        file=log,
    )
    print(f"  changed:  {' '.join(sorted(changed)) or '(tests only)'}", file=log)
    print(f"  in full:  {' '.join(whole) or '(none)'}", file=log)
    for package in sorted(selection.packages):
        units = selection.packages[package]
        if units is not None:
            total = len(ws.units(package))
            print(f"  in part:  {package}, {len(units)} of {total} test unit(s)", file=log)
    print("\n".join(selection.arguments()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
