#!/usr/bin/env python3
"""Verify that every relative link in the Markdown tree resolves.

Documentation is a deliverable here, and a docs tree quietly rots into broken
cross-references faster than code does, because nothing compiles it. This runs
in CI for the same reason clippy does.

Both halves of a link are checked: the file path, and the `#fragment` after it.
A fragment has to name a heading that actually exists in the target file, which
is the half that used to rot silently - a heading gets reworded, every link into
it keeps pointing at a slug nothing generates any more, and the build stays
green.

Links inside fenced code blocks are skipped: those are examples, not
navigation.

Also enforced, two rules from CLAUDE.md:

1. A `docs/` page must never reference `handover/` at all, link or plain text.
   A thread file under `handover/` is deleted the moment its work lands, so a
   link from a permanent page into one is a dead link waiting to happen with
   nothing else to catch it once the anchor check above stops seeing it (the
   target file is just gone, not malformed).
2. Nothing else outside `handover/` and `HANDOVER.md` - a doc comment, a
   script, a test, an asset file - may cite a *specific* thread file either,
   for the same reason. This one is path-shaped (`handover/<name>.md`), not a
   substring match like rule 1: the project keeps a few prose mentions of the
   `handover/` directory itself (no filename), and those describe a
   convention, not a citation that can go dangling.

Both are one-directional: a `handover/` thread citing a `docs/` page, or one
thread citing another, is how every thread here is written, and is unchecked.

3. No tracked file, `handover/` included, may cite a path under a `data/`
   directory on `DISPOSABLE_DATA_DIRS`. `data/` is gitignored, and its
   throwaway parts (`scratch/`, `shots/`, a lane's probe directory) are wiped
   wholesale once their lane merges, so a citation into one dangles the same way
   a deleted thread does (maintainer rule, 2026-10-09). The durable directories
   (disc images and what is extracted or cached from them, reference frames,
   save states, traces) stay citable: tests and tools read them by path. A
   script that *writes* scratch output names the directory without a literal
   `data/<dir>/<name>` path.

`docs/ghidra/captures/*.tsv` is exempt from rule 2: it is a verbatim capture
of what a Ghidra database held on a date, governed by its own check
(`check-ghidra-captures.py`'s `KNOWN_DANGLING`), where a dangling reference is
a finding to record, not a defect to fix. `scripts/check-ghidra-captures.py`
and `scripts/patches/*.patch` are exempt too - the first stores that same
known-dangling path as data, the second is a diff hunk body nobody here wrote
and editing it risks breaking `git apply`.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

LINK = re.compile(r"\[([^\]]*)\]\(([^)]+)\)")
FENCE = re.compile(r"^\s*(```|~~~)")
HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*$")
# A citation of a *specific* thread file, not a bare mention of the directory:
# `handover/foo.md` and `handover/rendering/foo.md` both match, `handover/`
# alone does not. Deliberately looser than a full path grammar - this only
# has to be right about where the `.md` sits, not validate every character a
# filename could legally contain.
HANDOVER_CITATION = re.compile(r"handover/[A-Za-z0-9_./-]+\.md")
# Rule 2's exemptions - see the module docstring for why each exists. This
# script is its own fourth exemption, undocumented there because the reason
# is purely mechanical: its own comments above illustrate the pattern with a
# real-shaped example, which the pattern then matches.
HANDOVER_CITATION_EXEMPT = {
    "scripts/check-doc-links.py",
    "scripts/check-ghidra-captures.py",
    "scripts/patches/ghidra-allegrex-psp-elf-extension-priority.patch",
}
# Rule 3: a path into a `data/` subdirectory. It must start the path (nothing
# path-like before `data/`), because 2048's and Omega's own archive paths begin
# with `data/` too (`data/art/...`, `data/audio/...`) and are not this checkout's
# `data/`. The character after the second slash must start a name, so a mention
# of the directory itself (`data/scratch/` in prose, `data/scratch/<lane>/` in a
# template) is not a citation.
DATA_CITATION = re.compile(r"(?<![A-Za-z0-9_./:\\-])data/([A-Za-z0-9_.-]+)/[A-Za-z0-9_.-]")
# The disposable `data/` subdirectories: lane scratch, screenshots and one-off
# probes, wiped once their lane merges. A deny-list rather than an allowlist of
# the durable ones (images, extracted, reference, saves, traces, ...) because
# of the archive-path collision above: an allowlist flags every in-game path.
DISPOSABLE_DATA_DIRS = {
    "ghidra-reloc-experiment",
    "lod-measure",
    "perf",
    "pulse-absorb-probe",
    "pulse-bloom",
    "scratch",
    "shots",
    "wine",
}
# This script names a disposable path in its own docstring.
DATA_CITATION_EXEMPT = {"scripts/check-doc-links.py"}
# Directories whose Markdown is not this project's to check.
#
# **`.claude` is the one that is easy to leave out and bites.** It holds this
# repository's own git worktrees, so a session working in one has a *second*
# checkout of the tree underneath the first - and, worse, whatever third-party
# source that session cloned into its own `.build`. On 2026-08-26 a vendored
# `VitaLoaderRedux/README.md` in one worktree failed this check for every other
# worktree at once, with three broken links in a file nobody here wrote.
#
# All four are gitignored, which is the rule this list is approximating: a file
# git does not track is not a file this project is answerable for.
SKIP_DIRS = {"target", ".git", ".claude", "data"}

MD_IMAGE = re.compile(r"!\[([^\]]*)\]\([^)]*\)")
MD_LINK = re.compile(r"\[([^\]]*)\]\([^)]*\)")
CODE_SPAN = re.compile(r"(`+)(.+?)\1")
HTML_TAG = re.compile(r"<[^>]+>")
HTML_ID = re.compile(r"""<[^>]*\b(?:id|name)\s*=\s*["']([^"']+)["'][^>]*>""")
TRAILING_HASHES = re.compile(r"\s+#+$")
# What GitHub's slugger keeps: letters, digits, `_` (all of `\w`), plus `-` and
# the spaces it is about to turn into hyphens. Everything else - `+`, `:`, `=`,
# backticks, commas, brackets - is dropped, not replaced, so `body+0x160`
# slugs to `body0x160` rather than `body-0x160`.
SLUG_DROP = re.compile(r"[^\w\- ]", re.UNICODE)


def strip_code_fences(text: str) -> list[tuple[int, str]]:
    """Returns (line number, line) for lines outside fenced code blocks."""
    out = []
    in_fence = False
    for number, line in enumerate(text.splitlines(), start=1):
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        if not in_fence:
            out.append((number, line))
    return out


def render(heading: str) -> str:
    """The text a heading displays, with its inline markup resolved.

    An image becomes its alt text, a link becomes its label, and inline HTML
    contributes nothing. A code span is the exception on both counts: its
    contents are literal, so `` `<Misc>` `` displays the angle brackets rather
    than being mistaken for a tag and dropped.
    """
    out = []
    position = 0

    for span in CODE_SPAN.finditer(heading):
        out.append(_render_markup(heading[position : span.start()]))
        out.append(span.group(2))
        position = span.end()

    out.append(_render_markup(heading[position:]))
    return "".join(out)


def _render_markup(text: str) -> str:
    text = MD_IMAGE.sub(r"\1", text)
    text = MD_LINK.sub(r"\1", text)
    return HTML_TAG.sub("", text)


def slug(heading: str) -> str:
    """The anchor GitHub generates for a heading, before de-duplication."""
    return SLUG_DROP.sub("", render(heading).lower()).replace(" ", "-")


def anchors(text: str) -> set[str]:
    """Every fragment the given Markdown file can be linked to.

    Two sources: the slug of each heading, and any explicit `id=`/`name=` on
    inline HTML, which docs use to keep a short stable anchor across rewordings.
    Repeated heading slugs get GitHub's `-1`, `-2`, ... suffixes, in document
    order.
    """
    found: set[str] = set()
    seen: dict[str, int] = {}

    for _, line in strip_code_fences(text):
        for match in HTML_ID.finditer(line):
            found.add(match.group(1))

        heading = HEADING.match(line)
        if not heading:
            continue

        base = slug(TRAILING_HASHES.sub("", heading.group(2)))
        if not base:
            continue

        count = seen.get(base, 0)
        seen[base] = count + 1
        found.add(base if count == 0 else f"{base}-{count}")

    return found


def is_external(link: str) -> bool:
    return link.startswith(("http://", "https://", "mailto:"))


def check(root: Path) -> list[str]:
    problems = []
    anchor_cache: dict[Path, set[str]] = {}

    def anchors_of(path: Path) -> set[str]:
        if path not in anchor_cache:
            anchor_cache[path] = anchors(path.read_text(encoding="utf-8"))
        return anchor_cache[path]

    for md in sorted(root.rglob("*.md")):
        rel = md.relative_to(root)
        # Relative to `root`, not absolute: `root` itself sits under `.claude`
        # whenever this runs from inside a worktree (`.claude/worktrees/...`
        # is where `EnterWorktree` puts one), and `md.parts` on the absolute
        # path matched `.claude` there unconditionally - which skipped every
        # file, in every worktree session, silently. `just check-docs` has
        # been reporting a pass without scanning anything since worktree
        # sessions started existing; caught while adding the handover check
        # below and re-verified nothing else was hiding behind it.
        if SKIP_DIRS & set(rel.parts):
            continue

        under_docs = rel.parts[0] == "docs"

        for number, line in strip_code_fences(md.read_text(encoding="utf-8")):
            if under_docs and "handover/" in line:
                problems.append(
                    f"{rel}:{number}: docs/ must not reference handover/ - "
                    f"{line.strip()}"
                )

            for match in LINK.finditer(line):
                raw = match.group(2).strip()
                if is_external(raw):
                    continue

                path, _, fragment = raw.partition("#")
                target = md if path == "" else (md.parent / path).resolve()

                if not target.exists():
                    problems.append(f"{rel}:{number}: broken link -> {raw}")
                    continue

                # Only Markdown has headings to point at; a link into an image
                # or a source file carries no fragment this can resolve.
                if not fragment or target.suffix != ".md":
                    continue

                if fragment not in anchors_of(target):
                    problems.append(f"{rel}:{number}: broken anchor -> {raw}")

    return problems


def tracked_files(root: Path) -> list[str]:
    result = subprocess.run(
        ["git", "ls-files"], cwd=root, capture_output=True, text=True, check=True
    )
    return result.stdout.splitlines()


def handover_citations(root: Path) -> list[str]:
    """Rule 2: nothing outside `handover/` and `HANDOVER.md` cites a thread by
    name. See the module docstring for the exemptions and why each exists.
    """
    problems = []

    for rel in tracked_files(root):
        parts = Path(rel).parts
        if parts[0] == "handover" or rel == "HANDOVER.md":
            continue
        if parts[:3] == ("docs", "ghidra", "captures"):
            continue
        if rel in HANDOVER_CITATION_EXEMPT:
            continue

        path = root / rel
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue

        for number, line in strip_code_fences(text):
            for match in HANDOVER_CITATION.finditer(line):
                problems.append(
                    f"{rel}:{number}: cites {match.group(0)} from outside "
                    f"handover/ - {line.strip()}"
                )

    return problems


def data_citations(root: Path) -> list[str]:
    """Rule 3: no tracked file cites a disposable `data/` directory."""
    problems = []

    for rel in tracked_files(root):
        if rel in DATA_CITATION_EXEMPT:
            continue

        try:
            text = (root / rel).read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue

        for number, line in enumerate(text.splitlines(), start=1):
            for match in DATA_CITATION.finditer(line):
                if match.group(1) not in DISPOSABLE_DATA_DIRS:
                    continue
                problems.append(
                    f"{rel}:{number}: cites disposable data/{match.group(1)}/ - "
                    f"{line.strip()}"
                )

    return problems


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    problems = check(root) + handover_citations(root) + data_citations(root)

    if problems:
        print(f"{len(problems)} problem(s):\n", file=sys.stderr)
        for p in problems:
            print(f"  {p}", file=sys.stderr)
        return 1

    print("all documentation links resolve")
    return 0


if __name__ == "__main__":
    sys.exit(main())
