# shellcheck shell=bash
# Sourced, never run: where a shell script puts temporary files, so that nothing
# lands in the OS temp directory (`oag_tmp.py`, the same rule for Python).
#
#   . scripts/oag-tmp.sh
#   tmp=$(mktemp -d -p "$(oag_tmp_root)")

# Prints <target>/tmp/scripts, made if it is not there.
oag_tmp_root() {
    local root=${CARGO_TARGET_DIR:-}
    [ -n "$root" ] || root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/target
    mkdir -p "$root/tmp/scripts" && printf '%s\n' "$root/tmp/scripts"
}
