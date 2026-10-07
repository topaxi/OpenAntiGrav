#!/bin/sh
# Linker for `cargo build --target x86_64-pc-windows-gnu` on a machine with zig
# but no MinGW-w64 gcc: zig ships the MinGW runtime and headers. rustc passes
# -nodefaultlibs and names msvcrt and libgcc itself; zig supplies its own
# runtime (ucrt) and compiler-rt, so those arguments are dropped.
for a in "$@"; do
    shift
    case "$a" in
        -nodefaultlibs|-l:libpthread.a|-lmsvcrt|-lgcc|-lgcc_eh|-Wl,-O1) ;;
        *) set -- "$@" "$a" ;;
    esac
done
exec zig cc -target x86_64-windows-gnu "$@" -lunwind
