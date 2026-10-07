#!/usr/bin/env python3
"""Disassemble a fragment program dumped from RPCS3's RAM.

    ps3-fp-live.py dump.bin [offset]

`dump.bin` starts at the program (the `SET_SHADER_PROGRAM` offset); the stream
runs until the instruction with the END bit. Constants are the patched values.
"""
import importlib.util
import sys
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "ps3_microcode", Path(__file__).with_name("ps3-microcode.py"))
mc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mc)

raw = Path(sys.argv[1]).read_bytes()
start = int(sys.argv[2], 0) if len(sys.argv) > 2 else 0
mc.fp_code(raw, start, len(raw) - start, {})
