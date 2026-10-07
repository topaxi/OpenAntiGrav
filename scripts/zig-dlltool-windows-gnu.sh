#!/bin/sh
# rustc's windows-gnu target calls `x86_64-w64-mingw32-dlltool` for raw-dylib
# import libraries; zig bundles llvm-dlltool, which takes the same arguments.
exec zig dlltool "$@"
