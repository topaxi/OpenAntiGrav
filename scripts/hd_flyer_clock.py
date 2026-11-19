#!/usr/bin/env python3
"""The clock of HD/Fury's `Grid Selection` flyer card, read out of a running RPCS3.

    hd_flyer_clock.py selftest        # a constructed widget in a fake memory; needs no emulator

The card is a `Flyer_Item` (vtable word `TOC[-0x16f]`, TOC `0x008ad4d8`) whose front model, `+0x224`, is a tree of
`MeshImporter` nodes. Each frame the widget's render (`0x001a2510`) reads the first of them through `AnimNode_GetTime`
(`+0xc0`, `0x002c0d78`) and, once the time reaches `+0x234` (6.0), hands `AnimNode_UpdateTransformTree` the time less 6.0
plus `+0x238` (3.0): the card plays `[3, 6)` seconds for ever, a loop of 3 s, so two grabs of the original show different
phases of the arrows and layers however alike their taps. A node samples its keys at the time modulo its own `LoopEnd`
(4.0 on Fury), so the raw value is what ours is given (`oag-game --menu-model-seconds`). Read series on Grid Selection Fury
(RPCS3, 40 reads 0.15 s apart): the value rises 1.0 a second of wall time and drops 6.01 to 3.03 (`docs/ui/campaign-screens.md`,
"The card's clock").

The instance is found by its vtable word and a non-null front model (the selection's own card; the other `Flyer_Item`s the
screen keeps hold none). The node is the first `MeshImporter` (vtable `0x00869fd0`) in the tree, walked through the child
link `+0x14` and the sibling link `+0x10`, the order `AnimNode_GetTime` searches. Only that one word is read at the grab: a
read of every node walks the tree over the control socket and the frames go by while it does.
"""
import struct
import sys
import time

GUEST_BASE = 0x300000000
TOC = 0x008AD4D8
VTABLE_SLOT = TOC - 0x16F * 4
MESH_IMPORTER_VTABLE = 0x00869FD0
FRONT, BACK = 0x224, 0x228
LOOP_END, LOOP_FLOOR = 0x234, 0x238
CHILD, SIBLING = 0x14, 0x10
TIME = 0xC0
HEAP = (0x30000000, 0x40000000)
BLOCK = 0x100000
#: Seconds the grab's picture is older than the clock read in the pause: the RSX is a few frames behind the PPU. **Fitted**: four
#: paused grabs at clocks 5.54, 5.52, 5.59 and 5.70, ours re-rendered at the clock plus -0.14 to -0.02 in 0.02 steps; the whole-frame
#: mean grey bottoms at -0.08, -0.06, -0.08 and -0.06, so 0.07 s, 4 frames at 60 Hz. Unpaused grabs (0.1 to 0.35 s long, clock read
#: before) bottomed at -0.03 to -0.05 and one at +0.04, which is why the script pauses. A property of the capture, not of ours.
PRESENTED_LAG_S = 0.07
#: Seconds the window is given to show the frame the stopped game last handed the RSX.
PAUSE_FLUSH_S = 0.6
#: Heap addresses a node can live at (the models sit above `HEAP`'s first blocks).
NODES = (0x30000000, 0x40000000)


class Clock:
    """The live card. `mem` has `seek` and `read` over the emulator's memory (`rpcs3-drive.py` `RemoteMem`)."""

    def __init__(self, mem):
        self.mem = mem
        self.item = self._find_item()
        self.node = self._find_node()

    def read(self, address, size):
        self.mem.seek(GUEST_BASE + address)
        return self.mem.read(size)

    def u32(self, address):
        return struct.unpack(">I", self.read(address, 4))[0]

    def f32(self, address):
        return struct.unpack(">f", self.read(address, 4))[0]

    def _find_item(self):
        word = struct.pack(">I", self.u32(VTABLE_SLOT))
        for base in range(HEAP[0], HEAP[1], BLOCK):
            try:
                block = self.read(base, BLOCK)
            except OSError:
                continue
            at = block.find(word)
            while at >= 0:
                if at % 4 == 0 and at + BACK + 4 <= len(block):
                    front = struct.unpack(">I", block[at + FRONT:at + FRONT + 4])[0]
                    if front:
                        return base + at
                at = block.find(word, at + 4)
        raise LookupError("no `Flyer_Item` with a front model: is a grid screen up?")

    def _find_node(self):
        stack, seen = [self.u32(self.item + FRONT)], set()
        while stack and len(seen) < 4000:
            node = stack.pop()
            if node in seen or not NODES[0] <= node < NODES[1]:
                continue
            seen.add(node)
            if self.u32(node) == MESH_IMPORTER_VTABLE:
                return node
            stack.append(self.u32(node + SIBLING))
            stack.append(self.u32(node + CHILD))
        raise LookupError("the card's front model holds no `MeshImporter`")

    def seconds(self):
        """The card's clock now: a value in `[3, 6)` once the widget has wrapped it."""
        return self.f32(self.node + TIME)

    def loop(self):
        """`(end, floor)`: the card's clock goes back by `end - floor` on reaching `end`."""
        return self.f32(self.item + LOOP_END), self.f32(self.item + LOOP_FLOOR)


def grab_paused(clock, grab, port, flush=PAUSE_FLUSH_S):
    """Run `grab()` with the game stopped, the clock read before and after it: `(before, after)`.

    Unpaused, the grab takes 0.1 to 0.35 s and the card's clock moves 1.0 a second through it, and on a stretch of the loop
    (a flash near 5.6) one frame decides the arrows. The stub stops every thread; the RSX still presents what it was handed,
    so `flush` seconds let the window catch up before the read."""
    from rpcs3_debugger import Debugger
    with Debugger(port=port) as dbg:
        dbg.resume()
        time.sleep(0.05)
        dbg.pause()
        try:
            time.sleep(flush)
            before = clock.seconds()
            grab()
            after = clock.seconds()
        finally:
            dbg.resume()
    return before, after


def shown(before, loop, lag=PRESENTED_LAG_S):
    """The clock the grab's picture shows: the read before the grab less the presented lag, back inside the loop."""
    end, floor = loop
    at = before - lag
    return at + (end - floor) if at < floor else at


def record(pair_rec, before, after, loop):
    """Write the clock into a pair's record; returns the flags ours is drawn with."""
    at = shown(before, loop)
    pair_rec["original"]["card_seconds"] = {
        "before_grab": round(before, 4),
        "after_grab": round(after, 4),
        "presented_lag_s": PRESENTED_LAG_S,
        "value": round(at, 4),
        "loop": [loop[1], loop[0]],
        "basis": "read: the `Grid Selection` card's clock, the time of the first `MeshImporter` of the front model's tree "
                 "(`AnimNode_GetTime`, node `+0xc0`) under the `Flyer_Item` at `*(front model, +0x224)`, out of the emulator's RAM just "
                 "before the grab (`before_grab`) and just after it (`after_grab`), the game stopped through the grab so the two agree. The widget keeps the clock inside [%g, %g) (`0x001a2510`). `value` is `before_grab` less "
                 "`presented_lag_s`, a **fitted** constant (`hd_flyer_clock.PRESENTED_LAG_S`: the RSX's frame is a few frames older "
                 "than RAM); ours is drawn with `--menu-model-seconds` at `value`" % loop[::-1]}
    return ["--menu-model-seconds", "%.4f" % at]


class FakeMem:
    """A guest heap with one card in it, for `selftest`: an item, a front model, a tree of two nodes."""

    def __init__(self, time):
        self.base = HEAP[0]
        self.bytes = bytearray(0x4000)
        self.pos = 0
        self.vtable = 0x00865CB8
        self.item, self.model, self.child, self.node = (self.base + o for o in (0x400, 0x800, 0xC00, 0x1000))
        self.low = {VTABLE_SLOT: struct.pack(">I", self.vtable)}
        self.put(self.base + 0x100, struct.pack(">I", self.vtable))
        self.put(self.item, struct.pack(">I", self.vtable))
        self.put(self.item + FRONT, struct.pack(">I", self.model))
        self.put(self.item + LOOP_END, struct.pack(">ff", 6.0, 3.0))
        self.put(self.model + CHILD, struct.pack(">I", self.child))
        self.put(self.child + SIBLING, struct.pack(">I", self.node))
        self.put(self.child, struct.pack(">I", 0x00873B38))
        self.put(self.node, struct.pack(">I", MESH_IMPORTER_VTABLE))
        self.put(self.node + TIME, struct.pack(">f", time))

    def put(self, address, data):
        self.bytes[address - self.base:address - self.base + len(data)] = data

    def seek(self, offset, whence=0):
        self.pos = offset - GUEST_BASE

    def read(self, n):
        if self.pos in self.low:
            return self.low[self.pos][:n]
        at = self.pos - self.base
        if not 0 <= at < len(self.bytes):
            raise OSError("unmapped")
        return bytes(self.bytes[at:at + n])


def selftest():
    mem = FakeMem(4.25)
    clock = Clock(mem)
    assert clock.item == mem.item, "the decoy vtable word with no front model is not the card"
    assert clock.node == mem.node, "the tree is walked child then sibling to the first MeshImporter"
    assert clock.seconds() == 4.25 and clock.loop() == (6.0, 3.0)
    mem.put(mem.node + TIME, struct.pack(">f", 5.5))
    assert clock.seconds() == 5.5, "the node's word is read afresh"
    rec = {"original": {}}
    assert record(rec, 5.5, 5.5, clock.loop()) == ["--menu-model-seconds", "5.4300"]
    card = rec["original"]["card_seconds"]
    assert (card["before_grab"], card["after_grab"], card["value"], card["loop"]) == (5.5, 5.5, 5.43, [3.0, 6.0])
    assert abs(shown(3.01, (6.0, 3.0)) - 5.94) < 1e-6, "a read just after the wrap shows the end of the loop"
    print("hd_flyer_clock selftest ok")


if __name__ == "__main__":
    if sys.argv[1:] != ["selftest"]:
        sys.exit(__doc__)
    selftest()
