#!/usr/bin/env python3
"""Makes the web module's page thread report a lock wait instead of blocking on it.

    python3 scripts/web-page-wait-guard.py MODULE.wasm --stack 8388608 --names OUT.txt

A contended `std::sync::Mutex`, a `OnceLock` another thread is initialising, a
`Condvar` or `thread::park` all end in `memory.atomic.wait32/64`. On the page's
thread Chromium traps on it, but Firefox blocks there: the page freezes and
Firefox offers to stop "a page that is slowing down Firefox", with nothing in the
console (docs/tools/web.md, "The page's thread may not wait"). This rewrites the
module in place so that, before every wait instruction, the page's thread calls
the module's own `oag_page_would_block(function)` export, which panics into the
page's one error report. Workers still wait as before.

The page's thread is told apart by its stack: the module is linked stack-first
(`-zstack-size`, scripts/build-web.sh), so the page's stack is the bottom
`--stack` bytes of memory and every worker's stack is allocated above it. The
guard is `global.get $__stack_pointer; i32.const STACK; i32.le_u; if; i32.const
FUNCTION; call $oag_page_would_block; unreachable; end`, stack-neutral, placed
before the wait. `FUNCTION` is the patched function's index in this module,
which `--names` maps to its name (written beside the module: the published
module carries no name section).

Runs after `wasm-bindgen` and before `wasm-opt`. Refuses a module whose first
global does not start at `--stack` or that has no `oag_page_would_block` export.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


def uleb(data: bytes, i: int) -> tuple[int, int]:
    result = shift = 0
    while True:
        byte = data[i]
        i += 1
        result |= (byte & 0x7F) << shift
        shift += 7
        if byte < 0x80:
            return result, i


def sleb(data: bytes, i: int) -> tuple[int, int]:
    result = shift = 0
    while True:
        byte = data[i]
        i += 1
        result |= (byte & 0x7F) << shift
        shift += 7
        if byte < 0x80:
            if byte & 0x40:
                result -= 1 << shift
            return result, i


def enc_uleb(value: int) -> bytes:
    out = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        if value:
            out.append(byte | 0x80)
        else:
            out.append(byte)
            return bytes(out)


def enc_sleb(value: int) -> bytes:
    out = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        if (value == 0 and not byte & 0x40) or (value == -1 and byte & 0x40):
            out.append(byte)
            return bytes(out)
        out.append(byte | 0x80)


def skip_memarg(code: bytes, i: int) -> int:
    align, i = uleb(code, i)
    if align & 0x40:  # multi-memory: a memory index follows
        _, i = uleb(code, i)
    _, i = uleb(code, i)
    return i


def wait_offsets(code: bytes, start: int, end: int) -> list[int]:
    """Offsets of every memory.atomic.wait32/64 in one function body's instructions."""
    found = []
    i = start
    while i < end:
        op = code[i]
        at = i
        i += 1
        if op in (0x02, 0x03, 0x04):  # block, loop, if: a block type
            if code[i] == 0x40 or code[i] in (0x7F, 0x7E, 0x7D, 0x7C, 0x7B, 0x70, 0x6F):
                i += 1
            else:
                _, i = sleb(code, i)
        elif op in (0x0C, 0x0D, 0x10, 0x12, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0xD2):
            _, i = uleb(code, i)
        elif op == 0x0E:  # br_table
            n, i = uleb(code, i)
            for _ in range(n + 1):
                _, i = uleb(code, i)
        elif op in (0x11, 0x13):  # call_indirect, return_call_indirect
            _, i = uleb(code, i)
            _, i = uleb(code, i)
        elif op == 0x1C:  # select t*
            n, i = uleb(code, i)
            i += n
        elif 0x28 <= op <= 0x3E:
            i = skip_memarg(code, i)
        elif op in (0x3F, 0x40):
            _, i = uleb(code, i)
        elif op == 0x41:
            _, i = sleb(code, i)
        elif op == 0x42:
            _, i = sleb(code, i)
        elif op == 0x43:
            i += 4
        elif op == 0x44:
            i += 8
        elif op == 0xD0:
            i += 1
        elif op == 0xFC:
            sub, i = uleb(code, i)
            if sub in (8, 10, 12, 14):
                _, i = uleb(code, i)
                _, i = uleb(code, i)
            elif sub in (9, 11, 13, 15, 16, 17):
                _, i = uleb(code, i)
            elif sub > 17:
                raise ValueError(f"unknown 0xFC {sub} at {at:#x}")
        elif op == 0xFE:
            sub, i = uleb(code, i)
            if sub == 0x03:  # atomic.fence
                i += 1
            elif sub <= 0x4E:
                i = skip_memarg(code, i)
                if sub in (0x01, 0x02):
                    found.append(at)
            else:
                raise ValueError(f"unknown 0xFE {sub} at {at:#x}")
        elif op == 0xFD:
            raise ValueError(f"SIMD instruction at {at:#x}: the module is built without simd128")
        elif op in (0x00, 0x01, 0x05, 0x0B, 0x0F, 0x1A, 0x1B, 0xD1) or 0x45 <= op <= 0xC4:
            pass
        else:
            raise ValueError(f"unknown opcode {op:#x} at {at:#x}")
    if i != end:
        raise ValueError(f"function body overran its end ({i:#x} past {end:#x})")
    return found


def sections(data: bytes) -> list[tuple[int, int, int, int]]:
    """(id, header start, payload start, payload end) for every section."""
    out = []
    i = 8
    while i < len(data):
        sid = data[i]
        size, j = uleb(data, i + 1)
        out.append((sid, i, j, j + size))
        i = j + size
    return out


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("module", type=Path)
    parser.add_argument("--stack", type=int, required=True, help="the page's stack size (-zstack-size)")
    parser.add_argument("--names", type=Path, required=True, help="where to write `index<TAB>name` per guarded function")
    args = parser.parse_args()

    data = args.module.read_bytes()
    secs = sections(data)
    imported_funcs = 0
    exports: dict[str, int] = {}
    names: dict[int, str] = {}
    first_global = None
    code = None
    for sid, _, start, end in secs:
        if sid == 2:
            n, i = uleb(data, start)
            for _ in range(n):
                for _ in range(2):
                    length, i = uleb(data, i)
                    i += length
                kind = data[i]
                i += 1
                if kind == 0:
                    _, i = uleb(data, i)
                    imported_funcs += 1
                elif kind in (1, 2):
                    if kind == 1:
                        i += 1
                    flags, i = uleb(data, i)
                    _, i = uleb(data, i)
                    if flags & 1:
                        _, i = uleb(data, i)
                elif kind == 3:
                    i += 2
                elif kind == 4:
                    i += 1
                    _, i = uleb(data, i)
        elif sid == 6:
            _, i = uleb(data, start)
            if data[i + 2] == 0x41:
                first_global, _ = sleb(data, i + 3)
        elif sid == 7:
            n, i = uleb(data, start)
            for _ in range(n):
                length, i = uleb(data, i)
                name = data[i:i + length].decode()
                i += length
                kind = data[i]
                index, i = uleb(data, i + 1)
                if kind == 0:
                    exports[name] = index
        elif sid == 10:
            code = (start, end)
        elif sid == 0:
            length, i = uleb(data, start)
            if data[i:i + length] == b"name":
                i += length
                while i < end:
                    sub = data[i]
                    size, i = uleb(data, i + 1)
                    if sub == 1:
                        count, q = uleb(data, i)
                        for _ in range(count):
                            index, q = uleb(data, q)
                            length, q = uleb(data, q)
                            names[index] = data[q:q + length].decode("utf-8", "replace")
                            q += length
                    i += size

    if first_global != args.stack:
        sys.exit(f"{args.module}: the first global starts at {first_global}, not the page's stack top {args.stack}")
    guard = exports.get("oag_page_would_block")
    if guard is None:
        sys.exit(f"{args.module}: no oag_page_would_block export to call")
    if code is None:
        sys.exit(f"{args.module}: no code section")
    # The wait in these is wasm-bindgen's and the linker's own per-thread
    # start-up and teardown, not a lock of the program's.
    runtime = ("__wasm_init_memory", "__wbindgen_start", "__wbindgen_thread_destroy")
    skip = {index for index, name in names.items() if name in runtime}
    skip |= {exports[name] for name in runtime if name in exports}
    skip.add(guard)

    start, end = code
    count, i = uleb(data, start)
    bodies = bytearray(enc_uleb(count))
    guarded: list[tuple[int, int]] = []
    for k in range(count):
        index = imported_funcs + k
        size, body_start = uleb(data, i)
        body_end = body_start + size
        j = body_start
        groups, j = uleb(data, j)
        for _ in range(groups):
            _, j = uleb(data, j)
            j += 1
        waits = [] if index in skip else wait_offsets(data, j, body_end)
        if waits:
            check = (b"\x23\x00" + b"\x41" + enc_sleb(args.stack) + b"\x4d" + b"\x04\x40"
                     + b"\x41" + enc_sleb(index) + b"\x10" + enc_uleb(guard) + b"\x00" + b"\x0b")
            body = bytearray()
            at = body_start
            for wait in waits:
                body += data[at:wait] + check
                at = wait
            body += data[at:body_end]
            bodies += enc_uleb(len(body)) + body
            guarded.append((index, len(waits)))
        else:
            bodies += data[i:body_end]
        i = body_end
    if i != end:
        sys.exit(f"{args.module}: the code section did not end where its size says")

    # Rebuilt without any `.debug_*` section: its code offsets no longer hold.
    out = bytearray(data[:8])
    for sid, header, s, e in secs:
        if sid == 10:
            out += b"\x0a" + enc_uleb(len(bodies)) + bodies
            continue
        if sid == 0:
            length, q = uleb(data, s)
            if data[q:q + length].startswith(b".debug_"):
                continue
        out += data[header:e]
    args.module.write_bytes(bytes(out))
    plain = re.compile(r"\[[0-9a-f]+\]")
    args.names.write_text("".join(
        f"{index}\t{plain.sub('', names.get(index, f'function {index}'))}\n" for index, _ in guarded))
    print(f"page wait guard: {sum(n for _, n in guarded)} wait(s) in {len(guarded)} function(s) guarded "
          f"(names: {args.names})")


if __name__ == "__main__":
    main()
