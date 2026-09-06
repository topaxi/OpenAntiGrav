#!/usr/bin/env bash
# Links the main checkout's `data/` subdirectories into the current worktree.
#
# `data/` is gitignored, so it does not travel into a `git worktree add` the way
# tracked files do. A worktree therefore starts with an empty `data/` holding
# only its tracked files, and every disc-backed test skips - or, under
# `OAG_REQUIRE_GAME_DATA=1`, fails with "is missing" on a file that is sitting
# in the main checkout the whole time. See CLAUDE.md's sandbox note.
#
# Symlinks rather than copies: `data/images` alone is about 8.5 GB, the disc
# images are read-only in practice, and derived output written through a link
# lands in the main checkout where the next worktree can see it too.
#
# `data/traces` and `data/keys` are no longer all-or-nothing: ADR-0046 put a
# handful of reference captures under `data/traces` into git, so a worktree
# materialises that directory for real (via the ordinary checkout, not this
# script) with only the tracked files in it. A whole-directory symlink cannot
# land on top of a real directory, so a naive "skip if it's already a real
# directory" check - what this script used to do - left every *untracked*
# capture sitting beside the tracked ones in the main checkout unreachable
# from any worktree. `link_into` below descends into a real directory instead
# of skipping it, and links only the entries missing from the worktree side,
# so a partially-tracked directory still ends up complete.
#
# Idempotent. Safe to run from the main checkout, where it does nothing.
set -euo pipefail
shopt -s nullglob

# Mirrors the entries of $2 (source) into $1 (destination) as symlinks, one
# per entry missing on the destination side. A destination entry that
# already exists for real (not a symlink) is never touched:
#
#   - a real file: left alone. This is what keeps a tracked file (a
#     `data/traces` reference capture, `data/keys/README.md`) intact - the
#     worktree's checkout put it there, and it is not this script's to
#     replace.
#   - a real directory: descended into, rather than skipped outright, so a
#     directory that is only partly tracked (some entries checked out for
#     real, others existing only in the main checkout) still gets its
#     missing entries linked instead of the whole directory being given up
#     on.
#
# A destination entry that is already a symlink is refreshed unconditionally
# (`ln -sfn`), which is what makes re-running this idempotent and lets a
# stale/dangling link from an older layout heal itself.
link_into() {
    local dst="$1" src="$2"
    local entry name target
    for entry in "$src"/*; do
        name=$(basename "$entry")
        target="$dst/$name"

        if [ -e "$target" ] && [ ! -L "$target" ]; then
            if [ -d "$target" ] && [ -d "$entry" ]; then
                link_into "$target" "$entry"
            else
                echo "  skip ${target#"$dest_root"/} (a real file or directory is already there)"
                skipped=$((skipped + 1))
            fi
            continue
        fi

        ln -sfn "$entry" "$target"
        linked=$((linked + 1))
    done
}

# `--self-check` proves the three invariants `link_into` promises, entirely
# inside a temp directory - no worktree, no real `data/`, safe to run from
# anywhere (including the main checkout, and CI). See `just check-link-data`.
self_check() {
    local tmp
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' RETURN

    mkdir -p "$tmp/src/traces" "$tmp/dst/traces"
    mkdir -p "$tmp/src/images"
    echo tracked >"$tmp/src/traces/tracked.csv"
    echo tracked >"$tmp/dst/traces/tracked.csv" # already checked out for real
    echo untracked >"$tmp/src/traces/untracked.csv"
    mkdir -p "$tmp/src/traces/nested"
    echo deep >"$tmp/src/traces/nested/deep.csv"
    echo iso >"$tmp/src/images/pulse.iso"

    linked=0
    skipped=0
    dest_root="$tmp/dst"
    link_into "$tmp/dst" "$tmp/src"

    local failures=0

    if [ ! -L "$tmp/dst/traces/tracked.csv" ] && [ "$(cat "$tmp/dst/traces/tracked.csv")" = tracked ]; then
        echo "PASS: a real (tracked) file was left alone, not clobbered"
    else
        echo "FAIL: the tracked file was replaced or removed"
        failures=$((failures + 1))
    fi

    if [ -L "$tmp/dst/traces/untracked.csv" ] && [ "$(cat "$tmp/dst/traces/untracked.csv")" = untracked ]; then
        echo "PASS: an untracked file beside a tracked one was linked"
    else
        echo "FAIL: the untracked sibling of a tracked file was not linked"
        failures=$((failures + 1))
    fi

    if [ -L "$tmp/dst/traces/nested" ] && [ "$(cat "$tmp/dst/traces/nested/deep.csv")" = deep ]; then
        echo "PASS: an untracked subdirectory was linked whole"
    else
        echo "FAIL: the untracked nested directory was not linked"
        failures=$((failures + 1))
    fi

    if [ -L "$tmp/dst/images" ] && [ "$(cat "$tmp/dst/images/pulse.iso")" = iso ]; then
        echo "PASS: a directory absent on the destination side is still linked whole"
    else
        echo "FAIL: an untracked top-level directory was not linked"
        failures=$((failures + 1))
    fi

    if [ ! -d "$tmp/dst/traces" ] || [ -L "$tmp/dst/traces" ]; then
        echo "FAIL: the real 'traces' directory itself was replaced by a symlink"
        failures=$((failures + 1))
    else
        echo "PASS: the real 'traces' directory itself was never replaced"
    fi

    # Re-running must not error and must not disturb the tracked file - the
    # idempotency guarantee.
    link_into "$tmp/dst" "$tmp/src"
    if [ "$(cat "$tmp/dst/traces/tracked.csv")" = tracked ]; then
        echo "PASS: re-running is idempotent and still leaves the tracked file alone"
    else
        echo "FAIL: re-running disturbed the tracked file"
        failures=$((failures + 1))
    fi

    if [ "$failures" -eq 0 ]; then
        echo "self-check: all checks passed"
        return 0
    fi
    echo "self-check: $failures check(s) failed"
    return 1
}

if [ "${1:-}" = "--self-check" ]; then
    self_check
    exit $?
fi

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
dest_root="$this_root/data"

link_into "$this_root/data" "$source_data"

echo "linked $linked item(s) from $source_data${skipped:+, skipped $skipped}"
echo "disc-backed tests can run here now: just test-data"
