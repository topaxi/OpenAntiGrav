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

Also enforced: a `docs/` page must never reference `handover/` at all, link or
plain text - CLAUDE.md's own rule. A thread file under `handover/` is deleted
the moment its work lands, so a link from a permanent page into one is a dead
link waiting to happen with nothing else to catch it once the anchor check
above stops seeing it (the target file is just gone, not malformed). The
reverse direction is fine and unchecked: a `handover/` thread citing a `docs/`
page as its evidence is how every thread here is written.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

LINK = re.compile(r"\[([^\]]*)\]\(([^)]+)\)")
FENCE = re.compile(r"^\s*(```|~~~)")
HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*$")
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


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    problems = check(root)

    if problems:
        print(f"{len(problems)} broken link(s):\n", file=sys.stderr)
        for p in problems:
            print(f"  {p}", file=sys.stderr)
        return 1

    print("all documentation links resolve")
    return 0


if __name__ == "__main__":
    sys.exit(main())
