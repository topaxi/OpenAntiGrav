#!/usr/bin/env python3
"""HD/Fury's `TrackHexSelection` widget in a running RPCS3: find it, read its pulse phase and step state.

    hd_track_hex.py selftest        # the decode and the picture-lag arithmetic on constructed bytes; needs no emulator

`TrackHexSelection` is the `HexSelection` class (`docs/ghidra/functions/ps3-hdfury-eu/ship-select-ring.md`) with its own
update (`0x002096f0`). The object is built once and kept: leaving `Track Creation` clears its built byte (`+0x10c`), so its
update stops, and re-entering resumes the counter where it stopped. Its `+0xe0` is therefore a count of the widget's own
updates since the first visit of the boot (`p += dt * 3`, wrapping past 2), not a time since the page opened.

The object has no static address; `find` scans the heap for its vtable word (`0x00868090`) and checks the authored shape
(`Columns` 9 at `+0xb8`, 10 at `+0xbc`, `Rows` 2 at `+0xc0`). It refuses anything but exactly one hit.
"""
import struct
import sys

import numpy as np

GUEST_BASE = 0x300000000
HEAP = (0x30000000, 0x40000000)
BLOCK = 0x100000
SPAN = 0x150
VTABLE = 0x00868090
COLUMNS, COLUMNS_PLUS_ONE, ROWS = 9, 10, 2
#: Game frames a grab's picture is older than the RAM read in a stopped game: the ring's colour in ten paused grabs ran 0.118 to
#: 0.228 of a phase behind `+0xe0` (mean 0.176, a phase step is 0.05 a frame), so 3.5 frames. The RSX presents a frame or so late and the
#: stub answers a frame or so after the stop; the turntable's angle (`track-selection-screen.md`) takes the same lag.
PICTURE_LAG_FRAMES = 3.5
PHASE_PER_FRAME = 0.05
PHASE_PERIOD = 2.0


def shown_phase(read, lag_frames=PICTURE_LAG_FRAMES):
    """The pulse phase the grab's picture shows, given `+0xe0` read in the stop: the read less the picture lag, inside `[0, 2)`."""
    return (read - lag_frames * PHASE_PER_FRAME) % PHASE_PERIOD


def decode(data):
    """The fields of the object out of `SPAN` bytes from its start."""
    u = struct.unpack(">%dI" % (SPAN // 4), data[:SPAN])
    f = struct.unpack(">%df" % (SPAN // 4), data[:SPAN])
    return {
        "vtable": u[0],
        "columns": u[0xB8 // 4],
        "parity": (u[0xB4 // 4] >> 24) & 1,
        "phase": f[0xE0 // 4],
        "slide_timer": f[0xEC // 4],
        "built": (u[0x10C // 4] >> 24) & 1,
        "previous": u[0x118 // 4],
        "selected": u[0x11C // 4],
        "row": u[0x110 // 4],
    }


def _read(mem, address, size):
    mem.seek(GUEST_BASE + address)
    return mem.read(size)


def find(mem):
    """Every address that reads as the widget. Raises unless there is exactly one."""
    hits = []
    for base in range(HEAP[0], HEAP[1], BLOCK):
        try:
            data = _read(mem, base, BLOCK + SPAN)
        except OSError:
            try:
                data = _read(mem, base, BLOCK)
            except OSError:
                continue
        words = len(data) // 4
        u = np.frombuffer(data[:words * 4], dtype=">u4")
        for i in np.nonzero(u[:min(BLOCK // 4, words - SPAN // 4)] == VTABLE)[0]:
            if (u[i + 0xB8 // 4], u[i + 0xBC // 4], u[i + 0xC0 // 4]) == (COLUMNS, COLUMNS_PLUS_ONE, ROWS):
                hits.append(base + int(i) * 4)
    if len(hits) != 1:
        raise RuntimeError("expected one TrackHexSelection, found %s" % [hex(h) for h in hits])
    return hits


class Widget:
    """The live widget. `mem` has `seek` and `read` over the emulator's memory (`rpcs3-drive.py` `RemoteMem`)."""

    def __init__(self, mem, address=None):
        self.mem = mem
        self.address = address if address is not None else find(mem)[0]

    def state(self):
        return decode(_read(self.mem, self.address, SPAN))

    def phase(self):
        """The pulse counter, `+0xe0`."""
        return struct.unpack(">f", _read(self.mem, self.address + 0xE0, 4))[0]


def _selftest():
    data = bytearray(SPAN)
    struct.pack_into(">I", data, 0, VTABLE)
    struct.pack_into(">I", data, 0xB8, COLUMNS)
    struct.pack_into(">f", data, 0xE0, 1.425)
    struct.pack_into(">I", data, 0x10C, 1 << 24)
    struct.pack_into(">I", data, 0x11C, 3)
    state = decode(bytes(data))
    assert (state["columns"], state["built"], state["selected"]) == (9, 1, 3)
    assert abs(state["phase"] - 1.425) < 1e-6
    assert abs(shown_phase(1.425) - 1.25) < 1e-6
    assert abs(shown_phase(0.1) - 1.925) < 1e-6, "the read just after a wrap shows the end of the last cycle"
    print("selftest ok")


if __name__ == "__main__":
    if sys.argv[1:] == ["selftest"]:
        _selftest()
    else:
        sys.exit(__doc__)
