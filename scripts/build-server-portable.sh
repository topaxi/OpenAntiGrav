#!/bin/sh
# Builds a release `oag-server` that runs on a host other than this machine.
#
#   scripts/build-server-portable.sh musl [OUT]
#   scripts/build-server-portable.sh [GLIBC_VERSION] [OUT]
#
# `musl`: a fully static binary (`ldd` says "statically linked") for any x86-64
# Linux, whatever its libc. Needs the `x86_64-unknown-linux-musl` Rust target
# (`rustup target add x86_64-unknown-linux-musl`) and `zig` on PATH, which
# compiles `ring`'s C for musl; Rust links the rest itself. Possible because the
# server builds `oag-raceplay` without `view` and so links no ALSA, X11 or
# Vulkan library at all (rule 6 of `just check-deps`).
#
# Otherwise: a dynamic binary linked through `zig cc` against a named glibc
# (default 2.35, Ubuntu 22.04), needing libc and libm only (see
# `crates/net/tests/server_libs.rs`).
#
# The build string the lobby compares (`oag_net::BUILD_VERSION`) is the checkout's short commit hash,
# read by `crates/net/build.rs`; `OAG_GIT_HASH=<hash>` in the environment overrides it, and a
# client of any other build is refused by name at the hello.
#
# Either way the result is stripped into OUT (default `target/portable/oag-server`).
# Nothing is installed anywhere.
set -eu
mode=${1:-2.35}
out=${2:-target/portable/oag-server}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
if [ "$mode" = musl ]; then
    target=x86_64-unknown-linux-musl
    # cc-rs passes its own --target, which zig spells differently.
    cat >"$tmp/cc" <<'EOF'
#!/bin/sh
for a in "$@"; do case "$a" in --target=*) ;; *) set -- "$@" "$a";; esac; shift; done
exec zig cc -target x86_64-linux-musl "$@"
EOF
    chmod +x "$tmp/cc"
    CC_x86_64_unknown_linux_musl="$tmp/cc" \
        cargo build --release --target "$target" \
        --target-dir target/portable-build -p oag-net --bin oag-server
    label=musl
else
    glibc=$mode
    target=x86_64-unknown-linux-gnu
    # rustc passes its own --target and -lgcc_s; zig takes the target from here and
    # supplies unwinding through libunwind. Undefined symbols in a shared library
    # the linker finds on this machine are not this binary's business: it never
    # calls them and the library is not in its DT_NEEDED list.
    cat >"$tmp/cc" <<EOF
#!/bin/sh
for a in "\$@"; do case "\$a" in --target=*) ;; -lgcc_s) set -- "\$@" -lunwind;; *) set -- "\$@" "\$a";; esac; shift; done
exec zig cc -Wl,--allow-shlib-undefined -target x86_64-linux-gnu.$glibc "\$@"
EOF
    chmod +x "$tmp/cc"
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$tmp/cc" \
        cargo build --release --target "$target" \
        --target-dir target/portable-build -p oag-net --bin oag-server
    label="glibc $glibc"
fi
mkdir -p "$(dirname "$out")"
strip -o "$out" "target/portable-build/$target/release/oag-server"
echo "wrote $out ($label)"
