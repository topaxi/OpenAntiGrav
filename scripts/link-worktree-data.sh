#!/usr/bin/env bash
# Links the main checkout's `data/` subdirectories into the current worktree.
#
# `data/` is gitignored, so it does not travel into a `git worktree add` the way
# tracked files do. A worktree therefore starts with an empty `data/` holding
# only its tracked `README.md`, and every disc-backed test skips - or, under
# `OAG_REQUIRE_GAME_DATA=1`, fails with "is missing" on a file that is sitting
# in the main checkout the whole time. See CLAUDE.md's sandbox note.
#
# Symlinks rather than copies: `data/images` alone is about 8.5 GB, the disc
# images are read-only in practice, and derived output written through a link
# lands in the main checkout where the next worktree can see it too.
#
# Idempotent. Safe to run from the main checkout, where it does nothing.
set -euo pipefail

# `--git-common-dir` is the main checkout's `.git` from anywhere in the
# repository, worktrees included, where `--git-dir` would give this worktree's
# own `.git/worktrees/<name>`. That is the whole trick this script turns on.
common_dir=$(git rev-parse --git-common-dir)
main_root=$(cd "$(dirname "$common_dir")" && pwd)
this_root=$(git rev-parse --show-toplevel)

if [ "$main_root" = "$this_root" ]; then
    echo "this is the main checkout; nothing to link"
    exit 0
fi

source_data="$main_root/data"
if [ ! -d "$source_data" ]; then
    echo "no data/ in the main checkout at $main_root - nothing to link" >&2
    exit 0
fi

mkdir -p "$this_root/data"
linked=0
skipped=0

for path in "$source_data"/*/; do
    [ -d "$path" ] || continue
    name=$(basename "$path")
    target="$this_root/data/$name"

    # A real directory here is somebody's own work, not a stale link. Left
    # alone and reported, because silently replacing it with a symlink would
    # lose whatever is in it.
    if [ -d "$target" ] && [ ! -L "$target" ]; then
        echo "  skip $name (a real directory is already there)"
        skipped=$((skipped + 1))
        continue
    fi

    ln -sfn "${path%/}" "$target"
    linked=$((linked + 1))
done

echo "linked $linked director(ies) from $source_data${skipped:+, skipped $skipped}"
echo "disc-backed tests can run here now: just test-data"
