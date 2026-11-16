#!/usr/bin/env python3
"""Checks the web module's stack: its size, and that no frame takes a large share of it.

    python3 scripts/check-wasm-stack.py target/web/dist/pkg/<hash>/oag_web_bg.wasm --stack 8388608

The page's thread runs on the stack the linker lays out below the module's data
(`__stack_pointer` starts at its top). An overflow runs below address 0 and traps
as "memory access out of bounds", which reads like a GPU or memory bug and
leaves every later callback trapping too (docs/tools/web.md, "The page's
stack"). `build-web.sh` runs this on every build, so the Pages job fails rather
than publish a module that cannot boot.

A frame is read off each function's prologue (`global.get $sp; i32.const N;
i32.sub`), which is how LLVM reserves one. Two checks: the stack pointer's
initial value is `--stack` (the link argument took), and no single frame is
over `--stack / --share` bytes. A call chain's depth is not computed, so the
share is a margin, not a proof. `--top N` lists the largest.
"""

from __future__ import annotations

import argparse
import sys
from collections import Counter
from pathlib import Path


def uleb(data: bytes, at: int) -> tuple[int, int]:
    value = shift = 0
    while True:
        byte = data[at]
        at += 1
        value |= (byte & 0x7F) << shift
        shift += 7
        if byte < 0x80:
            return value, at


def sleb(data: bytes, at: int) -> tuple[int, int]:
    value = shift = 0
    while True:
        byte = data[at]
        at += 1
        value |= (byte & 0x7F) << shift
        shift += 7
        if byte < 0x80:
            if byte & 0x40:
                value -= 1 << shift
            return value, at


def sections(data: bytes):
    if data[:4] != b"\0asm":
        sys.exit("not a wasm module")
    at = 8
    while at < len(data):
        size, body = uleb(data, at + 1)
        yield data[at], body, body + size
        at = body + size


def imported(data: bytes, at: int) -> tuple[int, int]:
    """How many functions and globals the module imports: both index spaces start with them."""
    count, at = uleb(data, at)
    functions = globals_ = 0
    for _ in range(count):
        for _ in range(2):
            length, at = uleb(data, at)
            at += length
        kind = data[at]
        at += 1
        if kind == 0:
            _, at = uleb(data, at)
            functions += 1
        elif kind == 1:
            at += 1
            flags, at = uleb(data, at)
            _, at = uleb(data, at)
            if flags & 1:
                _, at = uleb(data, at)
        elif kind == 2:
            flags, at = uleb(data, at)
            _, at = uleb(data, at)
            if flags & 1:
                _, at = uleb(data, at)
        elif kind == 3:
            at += 2
            globals_ += 1
        elif kind == 4:
            at += 1
            _, at = uleb(data, at)
        else:
            sys.exit(f"unknown import kind {kind}")
    return functions, globals_


def global_inits(data: bytes, at: int) -> dict[int, int]:
    """The `i32.const` initial value of each module-defined global, by index."""
    count, at = uleb(data, at)
    inits = {}
    for index in range(count):
        at += 2
        if data[at] != 0x41:
            break
        value, at = sleb(data, at + 1)
        if data[at] != 0x0B:
            break
        inits[index] = value
        at += 1
    return inits


def frames(data: bytes, at: int, first: int) -> list[tuple[int, int, int]]:
    """(frame bytes, function index, stack-pointer global) per function with a prologue."""
    count, at = uleb(data, at)
    found = []
    for offset in range(count):
        size, body = uleb(data, at)
        end = body + size
        locals_, cursor = uleb(data, body)
        for _ in range(locals_):
            _, cursor = uleb(data, cursor)
            cursor += 1
        if data[cursor] == 0x23:
            sp, cursor = uleb(data, cursor + 1)
            if data[cursor] == 0x41:
                value, cursor = sleb(data, cursor + 1)
                if data[cursor] == 0x6B and value > 0:
                    found.append((value, first + offset, sp))
        at = end
    return found


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("wasm", type=Path)
    parser.add_argument("--stack", type=int, required=True, help="the -zstack-size the module was linked with")
    parser.add_argument("--share", type=int, default=8, help="a frame may take at most stack/share")
    parser.add_argument("--top", type=int, default=5)
    args = parser.parse_args()

    data = args.wasm.read_bytes()
    imports = 0
    inits: dict[int, int] = {}
    found: list[tuple[int, int, int]] = []
    imported_globals = 0
    for kind, body, _end in sections(data):
        if kind == 2:
            imports, imported_globals = imported(data, body)
        elif kind == 6:
            inits = global_inits(data, body)
        elif kind == 10:
            found = frames(data, body, imports)
    if not found:
        sys.exit(f"{args.wasm}: no function reserves a stack frame; is this the web module?")
    sp = Counter(g for _, _, g in found).most_common(1)[0][0]
    top = inits.get(sp - imported_globals)
    if top != args.stack:
        sys.exit(
            f"{args.wasm}: the stack pointer starts at {top}, not {args.stack}: "
            "the -zstack-size link argument did not take"
        )
    limit = args.stack // args.share
    found.sort(reverse=True)
    print(f"wasm stack: {args.stack} bytes; largest frames (limit {limit}):")
    for size, index, _ in found[: args.top]:
        print(f"  function {index}: {size} bytes")
    over = [(size, index) for size, index, _ in found if size > limit]
    if over:
        sys.exit(
            f"{len(over)} frame(s) over {limit} bytes, the largest {over[0][0]} (function {over[0][1]}): "
            "box the large value it holds, or mark the callee that builds it #[inline(never)]"
        )


if __name__ == "__main__":
    main()
