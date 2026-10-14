#!/bin/sh
# Builds a release `oag-server` that runs on an older glibc than this machine's,
# by linking through `zig cc` against a named glibc (default 2.35, Ubuntu 22.04).
#
#   scripts/build-server-portable.sh [GLIBC_VERSION] [OUT]
#
# Needs `zig` on PATH. The result needs libc and libm only (see
# `crates/net/tests/server_libs.rs`); it is stripped into OUT
# (default `target/portable/oag-server`). Nothing is installed anywhere.
set -eu
glibc=${1:-2.35}
out=${2:-target/portable/oag-server}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
# rustc passes its own --target and -lgcc_s; zig takes the target from here and
# supplies unwinding through libunwind. Undefined symbols in a shared library
# the linker finds on this machine (libasound, say) are not this binary's
# business: it never calls them and the library is not in its DT_NEEDED list.
cat >"$tmp/cc" <<EOF
#!/bin/sh
for a in "\$@"; do case "\$a" in --target=*) ;; -lgcc_s) set -- "\$@" -lunwind;; *) set -- "\$@" "\$a";; esac; shift; done
exec zig cc -Wl,--allow-shlib-undefined -target x86_64-linux-gnu.$glibc "\$@"
EOF
chmod +x "$tmp/cc"
CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$tmp/cc" \
    cargo build --release --target x86_64-unknown-linux-gnu \
    --target-dir target/portable-build -p oag-net --bin oag-server
mkdir -p "$(dirname "$out")"
strip -o "$out" target/portable-build/x86_64-unknown-linux-gnu/release/oag-server
echo "wrote $out (glibc $glibc)"
