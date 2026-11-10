#!/usr/bin/env python3
"""What `scripts/affected.py` would have selected for the last merges on main.

    python3 scripts/affected-replay.py target/test-data.log [-n 50] [--crates]

Each merge's diff against its first parent goes through the selector, and the
selection is priced with the per-test seconds of a full `just test-data` log:
the tests selected, their summed seconds, and an estimated wall clock. The
estimate is the largest of three floors: the summed seconds over the log's own
parallelism (its summed seconds over its wall clock), the `gpu-vram` group's
summed seconds over the group's thread cap, and the slowest selected test.

The diffs are historical; the workspace, the test files and the timings are
today's. A crate or test that has since moved is priced where it is now.

`--crates` prints the other table of `docs/architecture/affected-gate.md`: per
crate, the commits of the last three months, the packages whose tests link it,
and what a change to its `src/` selects.
"""

import argparse
import importlib.util
import re
import statistics
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

RESULT = re.compile(
    r"^\s+(?:PASS|FAIL|FLAKY)\s+\[\s*([0-9.]+)s\]\s+\(\s*\d+/\d+\)\s+(\S+)\s+(.*)$"
)
SUMMARY = re.compile(r"^\s+Summary \[\s*([0-9.]+)s\]")
GROUP = re.compile(r"package\(([\w-]+)\) & test\(/(.*)/\)$")


def selector():
    spec = importlib.util.spec_from_file_location("affected", ROOT / "scripts/affected.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules["affected"] = module
    spec.loader.exec_module(module)
    return module


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout


def gpu_group() -> tuple[int, list[tuple[str, re.Pattern]]]:
    config = tomllib.loads((ROOT / ".config/nextest.toml").read_text())
    cap = config["test-groups"]["gpu-vram"]["max-threads"]
    members = []
    for override in config["profile"]["default"]["overrides"]:
        if override.get("test-group") == "gpu-vram":
            package, pattern = GROUP.match(override["filter"]).groups()
            members.append((package, re.compile(pattern)))
    return cap, members


def read_log(path: Path) -> tuple[list[tuple[str, str, str, str, float, bool]], float]:
    """Every test as `(package, kind, binary, name, seconds, in gpu group)`."""
    _, members = gpu_group()
    tests, wall = [], 0.0
    for line in path.read_text(errors="replace").splitlines():
        if summary := SUMMARY.match(line):
            wall = float(summary.group(1))
        if not (result := RESULT.match(line)):
            continue
        seconds, binary_id, name = float(result.group(1)), result.group(2), result.group(3)
        package, _, binary = binary_id.partition("::")
        kind = "unit" if not binary or binary.startswith("bin/") else "test"
        gpu = any(p == package and r.search(name) for p, r in members)
        tests.append((package, kind, binary, name, seconds, gpu))
    return tests, wall


class Pricer:
    def __init__(self, log: Path) -> None:
        self.tests, wall = read_log(log)
        if not self.tests or not wall:
            sys.exit(f"{log}: no test results or no Summary line; is it a full test-data log?")
        self.total = sum(t[4] for t in self.tests)
        self.parallel = self.total / wall
        self.cap, _ = gpu_group()

    def price(self, verdict: str, selection) -> tuple[int, float, float]:
        """`(tests, summed seconds, estimated wall seconds)` of one selection."""
        if verdict == "none":
            return 0, 0.0, 0.0
        chosen = [
            t
            for t in self.tests
            if verdict == "full" or selection.matches(t[0], t[1], t[2], t[3])
        ]
        if not chosen:
            return 0, 0.0, 0.0
        summed = sum(t[4] for t in chosen)
        gpu = sum(t[4] for t in chosen if t[5])
        wall = max(summed / self.parallel, gpu / self.cap, max(t[4] for t in chosen))
        return len(chosen), summed, wall


def replay(pricer: Pricer, count: int) -> None:
    affected = selector()
    ws = affected.Workspace()
    rows = []
    for line in git("log", "--merges", f"-{count}", "--format=%h %s", "main").splitlines():
        commit, subject = line.split(" ", 1)
        files = sorted(set(git("diff", "--name-only", f"{commit}^1", commit).split()))
        verdict, selection, why, changed = affected.classify(
            files, f"{commit}^1", commit, ws
        )
        tests, summed, wall = pricer.price(verdict, selection)
        rows.append((commit, len(files), verdict, tests, summed, wall, changed, subject))
    print("merge\tfiles\tverdict\ttests\tsummed_s\twall_s\tchanged\tsubject")
    for commit, files, verdict, tests, summed, wall, changed, subject in rows:
        names = ",".join(sorted(n.removeprefix("oag-") for n in changed))
        print(
            f"{commit}\t{files}\t{verdict}\t{tests}\t{summed:.0f}\t{wall:.0f}"
            f"\t{names[:48]}\t{subject[:44]}"
        )
    for label, column in (("tests", 3), ("summed_s", 4), ("wall_s", 5)):
        values = [row[column] for row in rows]
        print(
            f"# {label}: median {statistics.median(values):.0f}"
            f" mean {statistics.mean(values):.0f} worst {max(values):.0f}"
        )
    verdicts = [row[2] for row in rows]
    print(
        f"# {len(rows)} merges: {verdicts.count('full')} full,"
        f" {verdicts.count('none')} none; log {pricer.total:.0f} summed s,"
        f" parallelism {pricer.parallel:.1f}"
    )


def crates(pricer: Pricer) -> None:
    affected = selector()
    ws = affected.Workspace()
    rows = []
    for name, package in ws.packages.items():
        commits = int(
            git(
                "rev-list", "--count", "--no-merges", "--since=3 months ago",
                "HEAD", "--", package.directory,
            )
        )
        classifier = affected.Classifier(ws, "HEAD", None)
        classifier.changed.add(name)
        classifier.dependents()
        linked = sum(name in ws.links(other) for other in ws.packages if other != name)
        own = [t for t in pricer.tests if t[0] == name]
        tests, summed, wall = pricer.price("some", classifier.selection)
        rows.append(
            (commits * summed, name, commits, linked, len(own),
             sum(t[4] for t in own), tests, summed, wall)
        )
    print("crate\tcommits_3m\tlinked_by\town_tests\town_s\tsel_tests\tsel_s\tsel_wall_s\tcommits_x_sel_s")
    for weight, name, commits, linked, own, own_s, tests, summed, wall in sorted(rows, reverse=True):
        print(
            f"{name}\t{commits}\t{linked}\t{own}\t{own_s:.0f}\t{tests}"
            f"\t{summed:.0f}\t{wall:.0f}\t{weight:.0f}"
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("log", type=Path, help="a full `just test-data` log")
    parser.add_argument("-n", type=int, default=50, help="how many merges")
    parser.add_argument("--crates", action="store_true", help="the per-crate table")
    args = parser.parse_args()
    pricer = Pricer(args.log)
    if args.crates:
        crates(pricer)
    else:
        replay(pricer, args.n)
    return 0


if __name__ == "__main__":
    sys.exit(main())
