#!/usr/bin/env python3
"""The Fury menu backdrop widget (`BackgroundAnimFury`) in a running RPCS3: find it, read its row, hide it for a grab.

    hd_fury_backdrop.py selftest        # a constructed widget in a fake memory; needs no emulator

A front-end pair cannot reproduce the backdrop: which of twelve clouds, which path and how far along it are a new roll on
every boot, two thirds of the rolls are modes ours does not draw, the music pulses it, and stock RPCS3 draws its quads
wrong (`docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md`, "Hidden for a compare shot"). So a pair script hides it
on both sides: `Widget.hide` writes zero over the red, green and blue of the tint of the `<ScreenSetting>` row the screen
showing uses (`row + 0x40`; the blend pass multiplies the fresh particles by it), `Widget.wait_frames` lets the trail
already on screen decay, the grab is taken, and `Widget.restore` puts the authored tint back. Ours is drawn with
`oag-game --fury-tint 0`. `record` writes what was done into the pair.

The widget is found by its own row table (0x60-byte rows, `default` then `Main Menu`, the names `skin.xml` authors) and
the item that points at it: `item + 0xb8` the table, `+ 0xc4` the current row, `+ 0xc8` the `default` row, `+ 0x1640` the
clip, whose frame counter is its third word. No static word points at the item; the scan takes under two seconds.
"""
import struct
import sys
import time

GUEST_BASE = 0x300000000
ROW = 0x60
ROW_TINT = 0x40
ITEM_TABLE, ITEM_CURRENT, ITEM_DEFAULT, ITEM_CLIP = 0xB8, 0xC4, 0xC8, 0x1640
CLIP_FRAME = 0x08
#: `FunkLayer + 0x254`, the adapted luma the front end's exposure follows
#: (`docs/ghidra/functions/ps3-hdfury-eu/front-end-model-lighting.md`).
ADAPTED_LUMA = 0x00C50EE0 + 0x254
HEAP = (0x30000000, 0x40000000)
BLOCK = 0x100000
#: Widget frames between the write and the grab. The accumulation keeps 0.647 of itself a frame and the wave pair
#: 0.94 (0.955 in blue) less 0.0003, so a saturated trail is under half an 8-bit step after 135 frames.
FRAMES_TO_DECAY = 180
#: What ours is drawn with when the original's row was written to zero.
OURS_FLAGS = ("--fury-tint", "0")


class Widget:
    """The live widget. `mem` has `seek`, `read` and `write` over the emulator's memory (`rpcs3-drive.py` `RemoteMem`)."""

    def __init__(self, mem):
        self.mem = mem
        self.table, self.item = self._find()
        self.hidden = None

    def read(self, address, size):
        self.mem.seek(GUEST_BASE + address)
        return self.mem.read(size)

    def write(self, address, data):
        self.mem.seek(GUEST_BASE + address)
        return self.mem.write(data)

    def u32(self, address):
        return struct.unpack(">I", self.read(address, 4))[0]

    def blocks(self):
        for base in range(HEAP[0], HEAP[1], BLOCK):
            try:
                yield base, self.read(base, BLOCK)
            except OSError:
                continue

    def _find(self):
        table = None
        for base, block in self.blocks():
            at = block.find(b"default\0")
            while at >= 0 and table is None:
                if at % 4 == 0 and row_name(self.read(base + at + ROW, ROW)) == "Main Menu":
                    table = base + at
                at = block.find(b"default\0", at + 4)
            if table is not None:
                break
        if table is None:
            raise LookupError("no BackgroundAnimFury row table: is the Fury style's front end up?")
        word = struct.pack(">I", table)
        for base, block in self.blocks():
            at = block.find(word)
            while at >= 0:
                # `+0xb8` and `+0xc8` both hold the table: its first row is `default`.
                if at % 4 == 0 and self.u32(base + at + ITEM_DEFAULT - ITEM_TABLE) == table:
                    return table, base + at - ITEM_TABLE
                at = block.find(word, at + 4)
        raise LookupError("found the rows at %#x but no item holds them" % table)

    def current_row(self):
        """`(address, name, [r, g, b, a])` of the row the screen showing uses."""
        address = self.u32(self.item + ITEM_CURRENT)
        row = self.read(address, ROW)
        return address, row_name(row), list(struct.unpack(">4f", row[ROW_TINT:ROW_TINT + 16]))

    def frame(self):
        """The clip's frame counter: one a drawn frame, back to zero when a new path is picked."""
        return self.u32(self.u32(self.item + ITEM_CLIP) + CLIP_FRAME)

    def adapted_luma(self):
        return struct.unpack(">f", self.read(ADAPTED_LUMA, 4))[0]

    def hide(self):
        """Zero the current row's red, green and blue. Returns the record `restore` and `record` use."""
        address, name, authored = self.current_row()
        self.write(address + ROW_TINT, struct.pack(">3f", 0.0, 0.0, 0.0))
        self.hidden = {"row": name, "row_address": address, "tint_authored": [round(v, 5) for v in authored[:3]],
                       "tint_written": [0.0, 0.0, 0.0], "frames_waited": 0}
        return self.hidden

    def wait_frames(self, frames=FRAMES_TO_DECAY, timeout=60.0, poll=0.1, sleep=time.sleep, clock=time.monotonic):
        """Block until the widget drew `frames` more frames, so the trail from before `hide` is gone. Returns the count,
        short of `frames` when `timeout` passed first (a paused game draws none)."""
        seen, last, end = 0, self.frame(), clock() + timeout
        while seen < frames and clock() < end:
            sleep(poll)
            now = self.frame()
            # A new clip starts its counter again: the frames since then are what is known to have been drawn.
            seen += now - last if now >= last else now
            last = now
        if self.hidden is not None:
            self.hidden["frames_waited"] = seen
        return seen

    def still_hidden(self):
        """Whether the row written is still the one in use and still reads zero: checked just before the grab."""
        if self.hidden is None:
            return False
        address, _, tint = self.current_row()
        return address == self.hidden["row_address"] and tint[:3] == [0.0, 0.0, 0.0]

    def restore(self):
        """Put the authored tint back over the row `hide` wrote."""
        if self.hidden is None:
            return
        self.write(self.hidden["row_address"] + ROW_TINT, struct.pack(">3f", *self.hidden["tint_authored"]))
        self.hidden = None


def row_name(row):
    return row[:ROW_TINT].split(b"\0")[0].decode(errors="replace")


def record(pair_rec, hidden, adapted_luma=None):
    """Write a hidden backdrop into a pair's record; returns the flags ours is drawn with. `hidden` is `Widget.hide`'s
    record after the wait."""
    target = "BackgroundAnimFury row `%s` tint (row + 0x40, three floats)" % hidden["row"]
    pair_rec["original"]["backdrop"] = {
        "state": "hidden", "row": hidden["row"], "tint_authored": hidden["tint_authored"], "tint_written": hidden["tint_written"],
        "frames_waited": hidden["frames_waited"], "adapted_luma": None if adapted_luma is None else round(adapted_luma, 6),
        "basis": "the pair script wrote zero over the row's tint in the emulator's RAM, waited that many of the widget's own frames for the "
                 "trail to decay, grabbed and wrote the authored tint back; ours is drawn with `--fury-tint 0`. The cloud, its path and its "
                 "music pulse are a new roll on every boot, so the pair compares the page without them"}
    pair_rec["original"]["provenance"]["pair_script_ram_writes"].append({
        "target": target, "value": "0 0 0", "authored": " ".join("%g" % v for v in hidden["tint_authored"]),
        "why": "hide the Fury backdrop for the grab", "when": "before the grab, %d widget frames" % hidden["frames_waited"],
        "restored": "after the grab"})
    return list(OURS_FLAGS)


class FakeMem:
    """A guest heap with one widget in it, for `selftest`: rows at `table`, the item before them."""

    def __init__(self):
        self.base, self.table, self.item, self.clip = HEAP[0], HEAP[0] + 0x2000, HEAP[0] + 0x400, HEAP[0] + 0x3000
        self.bytes = bytearray(0x4000)
        self.pos = 0
        for i, (name, tint) in enumerate((("default", 0x20 / 255), ("Main Menu", 1.0))):
            row = name.encode().ljust(ROW_TINT, b"\0") + struct.pack(">4f", tint, tint, tint, 1.0)
            self.put(self.table + i * ROW, row.ljust(ROW, b"\0"))
        # A decoy: the name alone, with no `Main Menu` row after it.
        self.put(self.base + 0x100, b"default\0")
        for off in (ITEM_TABLE, ITEM_CURRENT, ITEM_DEFAULT):
            self.put(self.item + off, struct.pack(">I", self.table))
        self.put(self.item + ITEM_CLIP, struct.pack(">I", self.clip))
        self.put(ADAPTED_LUMA, struct.pack(">f", 0.01451))

    def put(self, address, data):
        if address >= self.base:
            self.bytes[address - self.base:address - self.base + len(data)] = data
        else:
            self.low = (address, bytes(data))

    def seek(self, offset, whence=0):
        self.pos = offset - GUEST_BASE

    def read(self, n):
        if self.pos < self.base:
            address, data = self.low
            if self.pos != address:
                raise OSError("unmapped")
            return data[:n]
        at = self.pos - self.base
        if at >= len(self.bytes):
            raise OSError("unmapped")
        return bytes(self.bytes[at:at + n]).ljust(n, b"\0") if n > len(self.bytes) else bytes(self.bytes[at:at + n])

    def write(self, data):
        self.put(self.pos, data)
        return len(data)


def selftest():
    mem = FakeMem()
    widget = Widget(mem)
    assert (widget.table, widget.item) == (mem.table, mem.item), "the decoy name is not the table"
    grey = round(0x20 / 255, 5)
    address, name, tint = widget.current_row()
    assert (address, name) == (mem.table, "default") and [round(v, 5) for v in tint] == [grey, grey, grey, 1.0]
    assert not widget.still_hidden(), "nothing is hidden before the write"
    hidden = widget.hide()
    assert widget.current_row()[2] == [0.0, 0.0, 0.0, 1.0], "three floats are written and the alpha is left"
    assert hidden["row"] == "default" and hidden["tint_authored"] == [grey] * 3
    assert widget.still_hidden()
    # The clip counts 40 frames a poll and starts again once on the way: 40, 80, 120, then 10 of a new clip, 50, 90, 130.
    counts = iter([0, 40, 80, 120, 10, 50, 90, 130, 170])
    widget.frame = lambda: next(counts)
    assert widget.wait_frames(sleep=lambda s: None) == 210 and hidden["frames_waited"] == 210
    # A paused game draws nothing: the wait gives up and says how far it got.
    ticks = iter(range(100))
    widget.frame = lambda: 7
    assert widget.wait_frames(timeout=3, sleep=lambda s: None, clock=lambda: next(ticks)) == 0
    hidden["frames_waited"] = 210
    # The screen moved to another row: the write no longer hides what is on screen.
    mem.put(mem.item + ITEM_CURRENT, struct.pack(">I", mem.table + ROW))
    assert not widget.still_hidden()
    mem.put(mem.item + ITEM_CURRENT, struct.pack(">I", mem.table))
    rec = {"original": {"provenance": {"pair_script_ram_writes": []}}}
    assert record(rec, hidden, widget.adapted_luma()) == ["--fury-tint", "0"]
    backdrop = rec["original"]["backdrop"]
    assert backdrop["state"] == "hidden" and backdrop["tint_written"] == [0.0, 0.0, 0.0] and backdrop["frames_waited"] == 210
    assert backdrop["adapted_luma"] == 0.01451
    (write,) = rec["original"]["provenance"]["pair_script_ram_writes"]
    assert write["value"] == "0 0 0" and "default" in write["target"] and write["restored"] == "after the grab"
    widget.restore()
    assert [round(v, 5) for v in widget.current_row()[2]] == [grey, grey, grey, 1.0], "the authored tint is back"
    assert not widget.still_hidden()
    print("hd_fury_backdrop selftest ok")


if __name__ == "__main__":
    if sys.argv[1:] != ["selftest"]:
        sys.exit(__doc__)
    selftest()
