#!/usr/bin/env python3
"""`just clean-scratch`: delete stale lane directories under data/scratch.

A directory is stale when nothing under it was modified in the last --days days
(default 7) and no git worktree or branch carries its name. `wipeout-pc`,
`mod-examples` and `xdg` are never touched. Without --yes it lists the
candidates with sizes and asks before deleting.

Hardlinked disc data (a copy made with `ln`) frees nothing until every link is
gone, so the sizes are an upper bound.
"""

import argparse
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

KEEP = {"wipeout-pc", "mod-examples", "xdg"}
ROOT = Path(__file__).resolve().parent.parent
SCRATCH = ROOT / "data" / "scratch"


def lane_names():
    """Worktree directory names and branch names: lanes that may still be open."""
    names = set()
    out = subprocess.run(
        ["git", "-C", str(ROOT), "worktree", "list", "--porcelain"],
        capture_output=True, text=True, check=True,
    ).stdout
    for line in out.splitlines():
        if line.startswith("worktree "):
            names.add(Path(line.split(" ", 1)[1]).name)
        elif line.startswith("branch "):
            names.add(line.split("/", 2)[2])
    out = subprocess.run(
        ["git", "-C", str(ROOT), "for-each-ref", "--format=%(refname:short)", "refs/heads"],
        capture_output=True, text=True, check=True,
    ).stdout
    names.update(out.split())
    return names


def newest_mtime_after(path, cutoff):
    """True when anything under path was modified after cutoff."""
    if os.lstat(path).st_mtime > cutoff:
        return True
    for dirpath, dirnames, filenames in os.walk(path):
        for name in dirnames + filenames:
            try:
                if os.lstat(os.path.join(dirpath, name)).st_mtime > cutoff:
                    return True
            except OSError:
                continue
    return False


def size_bytes(path):
    total = 0
    seen = set()
    for dirpath, _, filenames in os.walk(path):
        for name in filenames:
            try:
                st = os.lstat(os.path.join(dirpath, name))
            except OSError:
                continue
            if (st.st_dev, st.st_ino) not in seen:
                seen.add((st.st_dev, st.st_ino))
                total += st.st_size
    return total


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--days", type=int, default=7)
    parser.add_argument("--yes", action="store_true", help="delete without asking")
    args = parser.parse_args()

    if not SCRATCH.is_dir():
        print(f"no {SCRATCH}")
        return 0

    cutoff = time.time() - args.days * 86400
    open_lanes = lane_names()
    stale = []
    for entry in sorted(SCRATCH.iterdir()):
        if entry.name in KEEP or entry.name in open_lanes:
            continue
        if entry.is_symlink() or not entry.is_dir():
            if entry.lstat().st_mtime <= cutoff:
                stale.append((entry, entry.lstat().st_size))
            continue
        if not newest_mtime_after(entry, cutoff):
            stale.append((entry, size_bytes(entry)))

    if not stale:
        print(f"nothing under data/scratch is older than {args.days} days")
        return 0

    for entry, size in sorted(stale, key=lambda item: -item[1])[:30]:
        print(f"{size / 2**30:8.2f} GiB  {entry.name}")
    total = sum(size for _, size in stale)
    print(f"{len(stale)} entries, up to {total / 2**30:.1f} GiB")

    if not args.yes and input("delete these? [y/N] ").strip().lower() != "y":
        print("kept")
        return 0

    for entry, _ in stale:
        if entry.is_dir() and not entry.is_symlink():
            shutil.rmtree(entry)
        else:
            entry.unlink()
    print("deleted")
    return 0


if __name__ == "__main__":
    sys.exit(main())
