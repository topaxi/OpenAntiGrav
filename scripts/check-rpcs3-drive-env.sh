#!/usr/bin/env bash
# `just check-rpcs3-drive-env`: scripts/rpcs3-drive.py refuses to run without a
# private XDG_CACHE_HOME and XDG_CONFIG_HOME (scripts/emu-env.sh), and touches
# nothing when it refuses. No RPCS3 and no display are started.
#
# Every case that could touch a profile runs with `HOME` in a temporary directory,
# so a broken guard reaches only that. The one case about the password database's
# home calls the guard function alone. `RPCS3_DRIVE_UNDER_TEST` points at another
# copy of rpcs3-drive.py (a mutant); it must sit beside its sibling modules.
set -uo pipefail
[ -d /proc/self ] || { echo "check-rpcs3-drive-env: skipped (no /proc)"; exit 0; }
cd "$(dirname "$0")/.." || exit 1
drive=${RPCS3_DRIVE_UNDER_TEST:-$PWD/scripts/rpcs3-drive.py}
export PYTHONDONTWRITEBYTECODE=1
tmp=$(mktemp -d)
trap 'rm -rf -- "$tmp"' EXIT
home=$tmp/home
mine=$tmp/lane
mkdir -p "$home/.cache/rpcs3" "$home/.config/rpcs3" "$mine/cache/rpcs3" "$mine/config" "$tmp/linked"
ln -s "$home/.cache/rpcs3" "$tmp/linked/rpcs3"
# A dead run's instance lock in the "user's own" cache: what an unguarded `stop` deletes.
lock=$home/.cache/rpcs3/RPCS3.buf
touch "$lock"
# A broken guard must not reach a real display server either: `Xvfb` is a stand-in
# that exits at once.
mkdir -p "$tmp/bin"
printf '#!/bin/sh\nexit 1\n' >"$tmp/bin/Xvfb"
chmod +x "$tmp/bin/Xvfb"
export PATH="$tmp/bin:$PATH"

fail() { echo "check-rpcs3-drive-env: $*" >&2; [ -z "${out:-}" ] || echo "last output: $out" >&2; exit 1; }
# run "VAR=value ..." command... : a clean profile environment, `HOME` private.
run() {
    local vars=$1
    shift
    # shellcheck disable=SC2086 # `vars` is a list of assignments
    out=$(env -u XDG_CACHE_HOME -u XDG_CONFIG_HOME -u OAG_RPCS3_OWN_HOME -u OAG_RPCS3_ATTACH \
        HOME="$home" OAG_RPCS3_DISPLAY=59987 $vars "$@" 2>&1)
    rc=$?
}
refused() {
    local what=$1
    [ "$rc" -eq 78 ] || fail "$what: expected exit 78, got $rc"
    case "$out" in *env.sh*) ;; *) fail "$what: the message names no env.sh" ;; esac
    [ -e "$lock" ] || fail "$what: the lock in the user's cache was removed"
    [ ! -e "$home/.cache/oag-rpcs3-drive" ] || fail "$what: wrote into the user's cache"
}
private="XDG_CACHE_HOME=$mine/cache XDG_CONFIG_HOME=$mine/config"

stop() { run "$1" python3 "$drive" stop; refused "$2"; }
stop "" "XDG unset"
stop "XDG_CACHE_HOME=$home/.cache XDG_CONFIG_HOME=$home/.config" "the defaults, exported"
stop "XDG_CACHE_HOME=$mine/cache" "config unset"
stop "XDG_CONFIG_HOME=$mine/config" "cache unset"
stop "XDG_CACHE_HOME=$home/.cache XDG_CONFIG_HOME=$mine/config" "own cache"
stop "XDG_CACHE_HOME=$mine/cache XDG_CONFIG_HOME=$home/.config" "own config"
stop "XDG_CACHE_HOME=$home XDG_CONFIG_HOME=$mine/config" "the home itself"
stop "XDG_CACHE_HOME=$tmp/linked XDG_CONFIG_HOME=$mine/config" "rpcs3 symlinked to the own one"
stop "XDG_CACHE_HOME=relative XDG_CONFIG_HOME=$mine/config" "relative path"

# Every subcommand is guarded, not only `stop`.
for sub in display preflight status; do
    run "" python3 "$drive" "$sub"
    refused "$sub"
done

# `--help` needs no environment.
run "" python3 "$drive" --help
[ "$rc" -eq 0 ] || fail "--help failed (rc=$rc)"

# A script that imports the module gets the same refusal before it touches anything.
load='import importlib.util, sys
sys.path.insert(0, sys.argv[1].rsplit("/", 1)[0])
spec = importlib.util.spec_from_file_location("drive", sys.argv[1])
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)'
for call in 'drive.clear_stale_lock()' 'drive.stop_emulator()' 'drive.stop_display()' \
    'drive.start_display()' 'drive.Session("no-image", sys.argv[2])'; do
    run "" python3 -c "$load
$call" "$drive" "$tmp/session-log"
    refused "importer: $call"
done
[ ! -e "$tmp/session-log" ] || fail "a refused Session made its log directory"

# The password database's home counts even when `HOME` says otherwise. Only the
# guard runs here: this is the real user's path.
real=$(python3 -c 'import os, pwd; print(pwd.getpwuid(os.getuid()).pw_dir)')
run "XDG_CACHE_HOME=$real/.cache XDG_CONFIG_HOME=$mine/config" python3 -c "$load
drive.require_private_env()" "$drive"
refused "the real home under another HOME"

# A private profile runs, and clears its own stale lock.
touch "$mine/cache/rpcs3/RPCS3.buf"
run "$private" python3 "$drive" stop
[ "$rc" -eq 0 ] || fail "a private profile was refused (rc=$rc)"
[ ! -e "$mine/cache/rpcs3/RPCS3.buf" ] || fail "the private stale lock survived stop"
[ -e "$lock" ] || fail "a private run removed the user's lock"

# The explicit opt-out for driving one's own profile (the temporary one here).
run "OAG_RPCS3_OWN_HOME=1" python3 "$drive" stop
[ "$rc" -eq 0 ] || fail "OAG_RPCS3_OWN_HOME=1 was refused (rc=$rc)"

echo "check-rpcs3-drive-env: ok"
