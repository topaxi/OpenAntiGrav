# ADR-0085: Comparison originals live in a private repository

## Status

**Accepted** (2026-11-05). Extends [ADR-0074](0074-curated-screenshots-may-be-published-no-content-may-be-shipped.md).
ADR-0074 is not edited: nothing here is published, and the public repository and its builds
do not change.

## Context

The comparison page's pairs live under `data/reference/compare-page/`, gitignored, one copy per
workstation, moved by hand. On 2026-11-05 the two workstations differed in 23 files by size, and
one pair existed on each side only. A `theirs.png` can only be remade by running the original in
an emulator, and it had no copy off those two machines.

## Decision

1. **`OpenAntiGrav/private` is a separate, private repository, checked out at `data/reference`.**
   The public repository does not know it: `data/` stays gitignored, `scripts/check-leakage.py`
   does not change, and no public file, script or CI job needs it or links into it. A checkout
   without it works as one without `data/` always has.
2. **It tracks an allowlist, kept in its own `.gitignore`:** the manifests, `theirs*.png`,
   `pair.json`, `gap.json`, and capture scripts. Nothing else, by default. **Every `*.png` is a
   Git LFS object from the first commit** (`.gitattributes`), so a re-captured frame replaces a
   pointer and git history stays small.
3. **`ours*.png` and `side*.png` stay out.** They regenerate with `just compare-rerender-ours`,
   and each re-render would add hundreds of MB to a history that never shrinks.
4. **Memory dumps, disc images, extracted or decoded assets and keys never enter it.** A dump of
   the running game's memory can hold its assets. These are the content the project does not
   redistribute, and being private does not change that.
5. **A push is a manual step by the maintainer's say-so.** Agents read and pull; no drive member
   and no CI job pushes to it, and it stays private, with no collaborator beyond the maintainer.
6. **The limits of ADR-0074 still bind what leaves it.** A frame in this repository is not
   published by being here; publishing stays the manual, curated step ADR-0074 describes.

## Consequences

- Both workstations reach the same originals and manifests with a pull, and the originals have a
  copy off them.
- This is the first time frames of the original games sit in a git history, on GitHub, under the
  same organisation as the public project. A rights holder's request under
  [`legal.md`](../../overview/legal.md) would mean rewriting that history, not deleting a file,
  and an LFS object goes only with the repository or through GitHub support.
- A checkout needs `git-lfs` installed, or it holds pointer files where the frames should be.
- A fresh checkout has no `ours.png`: run `just compare-rerender-ours` before `just compare-page`.
- `pair.json` carries build stamps, so a re-render shows as a change to commit or discard.
- Older captures outside `compare-page/` stay on the workstations. Their disk use and their
  dumps do not belong here.
