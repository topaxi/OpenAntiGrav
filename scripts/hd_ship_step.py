#!/usr/bin/env python3
"""HD/Fury's `Team Selection` screen object in a running RPCS3: find it, read the craft fit's state, count its frames.

    hd_ship_step.py selftest        # the frame counter, the forward law and the hull stepping on constructed rows; needs no emulator

`TeamSelectionScreen_Update` (`0x00254348`, `docs/ghidra/functions/ps3-hdfury-eu/team-selection-model.md`) keeps the
craft's fit on the screen object `st`: the depth target `st+0x1014`, the drawn depth `st+0x1020`, the slide targets
`st+0x100c`, `st+0x1010`, the drawn slide `st+0x1018`, `st+0x101c`, the selection `st+0x100` and the selected
`PI_TeamModel` `st+0x194` (whose `+0x1cc` is its `<FE Length>`). The update runs once a frame and rewrites the target
and the drawn depth every time, so **a change of the (target, depth, slide) tuple in RAM is one game frame**: a reader
polling in under a millisecond sees every frame, and the count of them since the selection changed is the age of a
step in game frames, whatever the emulator's speed.

`step_to_hull` drives the screen to a named hull by reading the selected one after every press, never by a press
count: which row a save opens on, and which rows are locked, differ between profiles
(`docs/reverse-engineering/rpcs3-capture.md`, "Picking the hull on Ship Select").

The object has no static address and no vtable word that was needed: `find` scans the heap for the word pattern
(target on the 0.1 grid at or above the floor, drawn depth near it, a model pointer whose length is one of the eight
the disc authors) and refuses anything but exactly one hit.
"""
import re
import struct
import sys

GUEST_BASE = 0x300000000
HEAP = (0x30000000, 0x40000000)
BLOCK = 0x100000
SPAN = 0x1040
#: The eight `<FE Length>` values the disc authors (`team-selection-model.md`, "The fit").
LENGTHS = (3.2, 3.8, 3.9, 4.0, 4.1, 4.2, 4.4, 4.5)
SEL, MODEL = 0x100, 0x194
SLIDE_TARGET, TARGET, SLIDE, DEPTH = 0x100C, 0x1014, 0x1018, 0x1020
#: The player's own turn, push, depth, pan and zoom, the idle counter: all zero at rest (`team-selection-model.md`).
INPUTS = (0xFE8, 0xFEC, 0xFF0, 0xFF4, 0xFF8, 0x1024)
IDLE = 0xFFC
FLOOR = 11.0


class Screen:
    """The live screen. `mem` has `seek` and `read` over the emulator's memory (`rpcs3-drive.py` `RemoteMem`)."""

    def __init__(self, mem, address=None):
        self.mem = mem
        self.address = address if address is not None else self.find(mem)[0]
        self.names = {}

    @staticmethod
    def _read(mem, address, size):
        mem.seek(GUEST_BASE + address)
        return mem.read(size)

    def read(self, address, size):
        return self._read(self.mem, address, size)

    @classmethod
    def find(cls, mem):
        """Every address that reads as the screen. Raises unless there is exactly one."""
        import numpy as np  # the scan alone needs it; the selftest runs without it

        hits = []
        for base in range(HEAP[0], HEAP[1], BLOCK):
            try:
                data = cls._read(mem, base, BLOCK + SPAN)
            except OSError:
                try:
                    data = cls._read(mem, base, BLOCK)
                except OSError:
                    continue
            words = len(data) // 4
            f = np.frombuffer(data[:words * 4], dtype=">f4")
            u = np.frombuffer(data[:words * 4], dtype=">u4")
            count = min(BLOCK // 4, words - SPAN // 4)
            if count <= 0:
                continue
            i = np.arange(count)
            t, d = f[i + TARGET // 4], f[i + DEPTH // 4]
            with np.errstate(all="ignore"):
                grid = np.abs(t * 10 - np.round(t * 10)) < 0.01
                ok = (t >= FLOOR - 0.01) & (t < 15) & grid & (np.abs(d - t) < 3)
                ok &= (np.abs(f[i + SLIDE // 4]) < 600) & (np.abs(f[i + (SLIDE + 4) // 4]) < 600)
            for at in i[ok]:
                model = int(u[at + MODEL // 4])
                if not HEAP[0] <= model < HEAP[1]:
                    continue
                try:
                    length = struct.unpack(">f", cls._read(mem, model + 0x1CC, 4))[0]
                except OSError:
                    continue
                if min(abs(length - v) for v in LENGTHS) < 1e-3 and u[at + SEL // 4] < 40:
                    hits.append(base + int(at) * 4)
        if len(hits) != 1:
            raise LookupError("expected one `Team Selection` screen object, found %s (is Ship Select up?)"
                              % [hex(h) for h in hits])
        return hits

    def snap(self):
        """The screen's words from `+0` to `+0x1040`, one read."""
        return struct.unpack(">%dI" % (SPAN // 4), self.read(self.address, SPAN))

    def model_name(self, model):
        """The `PI_TeamModel`'s hull entry name (`Feisar_c1`), cached by pointer."""
        if model not in self.names:
            blob = self.read(model, 0x400)
            hit = re.search(rb"[A-Za-z0-9_]+_[cn]\d", blob)
            self.names[model] = hit.group(0).decode() if hit else None
        return self.names[model]

    def state(self, words=None):
        """The fit's state: a dict of the words a step is judged by."""
        w = words or self.snap()
        f = lambda off: struct.unpack(">f", struct.pack(">I", w[off // 4]))[0]
        model = w[MODEL // 4]
        return {"sel": w[SEL // 4], "model": model, "target": f(TARGET), "depth": f(DEPTH),
                "slide_target": [f(SLIDE_TARGET), f(SLIDE_TARGET + 4)], "slide": [f(SLIDE), f(SLIDE + 4)],
                "inputs": [w[o // 4] for o in INPUTS], "idle": w[IDLE // 4]}

    def length(self, model):
        return struct.unpack(">f", self.read(model + 0x1CC, 4))[0]


def key(state):
    """What changes once a frame: `(target, depth, slide)`, as exact words."""
    return (state["target"], state["depth"], tuple(state["slide"]))


class Counter:
    """Counts the game frames since the selection moved, fed one `state` per poll."""

    def __init__(self):
        self.sel = None
        self.last = None
        self.frame = None      # None until the selection has changed
        self.rows = []

    def feed(self, state):
        if self.sel is None:
            self.sel, self.last = state["sel"], key(state)
            return None
        if self.frame is None:
            if state["sel"] != self.sel:
                self.frame = 0
                self.last = key(state)
                self.rows.append((0, state))
            return self.frame
        if key(state) != self.last:
            self.frame += 1
            self.last = key(state)
            self.rows.append((self.frame, state))
        return self.frame


def forward(depth, target, frames, step=0.1, ease=0.125):
    """The drawn depth and target `frames` frames on while the target keeps walking nearer by `step` (the walking
    phase of a step to a box that fits; the law, not a measurement of ours)."""
    for _ in range(frames):
        target = max(FLOOR, target - step)
        depth += (target - depth) * ease
    return depth, target


#: Presses in one direction that leave the selection where it was before the list is taken to end there. The emulator
#: drops about one press in three, so one unchanged press says nothing.
CLAMP_PRESSES = 6


def team_part(hull):
    """The team of a hull directory: `ag_systems` for `AG_Systems_c1` and for `AG_Systems`."""
    return re.sub(r"_[cn]\d$", "", hull.lower())


def step_to_hull(read, press, want, max_presses=80):
    """Press Ship Select until the selected hull is `want` (case-insensitive); returns the buttons pressed.

    `read()` returns `(model pointer, hull directory)` and `press(button)` presses and lets the screen settle. The team
    column first (`right`, then `left`), then that column's rows (`down`, then `up`): the screen opens on an unlocked
    row and a locked row can carry the same directory name, so the rows below are searched before the rows above. A
    direction ends when `CLAMP_PRESSES` presses leave the model pointer where it was, or when it comes back to one already
    passed (the team list wraps). Raises `LookupError` when neither
    direction reaches the hull. A locked row that matches is returned like any other: `cross` does nothing on it, which
    the caller checks."""
    presses = []

    def search(buttons, found):
        model, hull = read()
        for button in buttons:
            stuck, seen = 0, {model}
            while not found(hull) and stuck < CLAMP_PRESSES:
                if len(presses) >= max_presses:
                    raise LookupError("%d presses on Ship Select without reaching %s (on %s)" % (max_presses, want, hull))
                press(button)
                presses.append(button)
                after, hull = read()
                if after != model and after in seen:
                    model = after
                    break
                stuck = stuck + 1 if after == model else 0
                model = after
                seen.add(after)
            if found(hull):
                return
        raise LookupError("Ship Select has no %s: neither %s reaches it (on %s)" % (want, " nor ".join(buttons), hull))

    search(("right", "left"), lambda hull: team_part(hull) == team_part(want))
    search(("down", "up"), lambda hull: hull.lower() == want.lower())
    return presses


class FakeGrid:
    """Ship Select as read on the original: a column per team and the team list wraps, each column a list of rows that
    clamps at both ends, with the locked rows repeating a directory name, opening on the third row. Every third press is
    dropped."""

    ROWS = ("%s_c1", "%s_n1", "%s_c1", "%s", "%s", "%s", "%s")
    TEAMS = ("Feisar", "Qirex", "piranha", "AG_Systems", "Assegai", "Harimau")

    def __init__(self, row=2):
        self.team, self.row, self.count = 0, row, 0

    def read(self):
        return (self.team * 16 + self.row, self.ROWS[self.row] % self.TEAMS[self.team])

    def press(self, button):
        self.count += 1
        if self.count % 3 == 0:
            return
        dt, dr = {"right": (1, 0), "left": (-1, 0), "down": (0, 1), "up": (0, -1)}[button]
        self.team = (self.team + dt) % len(self.TEAMS)
        self.row = min(max(self.row + dr, 0), len(self.ROWS) - 1)


def selftest_step_to_hull():
    """The stepper lands on the unlocked row of the named team from where the screen opens, whatever was dropped, and
    refuses a hull the grid does not hold."""
    for want, team, row in (("harimau", "Harimau", 3), ("Harimau_c1", "Harimau", 2), ("Feisar_c1", "Feisar", 2),
                            ("Assegai_c1", "Assegai", 2), ("ag_systems", "AG_Systems", 3), ("Piranha", "piranha", 3)):
        grid = FakeGrid()
        step_to_hull(grid.read, grid.press, want)
        assert (grid.TEAMS[grid.team], grid.row) == (team, row), (want, grid.TEAMS[grid.team], grid.row)
    # a screen that opens lower finds the unlocked `_c1` row above, not the locked one over it
    grid = FakeGrid(row=3)
    step_to_hull(grid.read, grid.press, "feisar_c1")
    assert grid.row == 2, grid.row
    for want in ("Nobody", "Harimau_n9"):
        grid = FakeGrid()
        try:
            step_to_hull(grid.read, grid.press, want)
        except LookupError:
            pass
        else:
            raise AssertionError("%s is not on the grid and must be refused (on %s)" % (want, grid.read()[1]))
        assert grid.count < 50, "%s refused only after %d presses" % (want, grid.count)


def selftest():
    selftest_step_to_hull()
    c = Counter()
    base = {"sel": 1, "model": 1, "target": 13.6, "depth": 13.647, "slide_target": [1.0, 2.0], "slide": [1.0, 2.0]}
    assert c.feed(base) is None
    assert c.feed(dict(base)) is None
    assert c.feed(dict(base, sel=2, target=13.7)) == 0
    assert c.feed(dict(base, sel=2, target=13.7)) == 0
    assert c.feed(dict(base, sel=2, target=13.6, depth=13.64)) == 1
    d, t = forward(13.653, 13.7, 2)
    assert abs(t - 13.5) < 1e-9 and abs(d - 13.628) < 1e-3, (d, t)
    print("ok")


if __name__ == "__main__":
    if sys.argv[1:] == ["selftest"]:
        selftest()
    else:
        sys.exit(__doc__)
