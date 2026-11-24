#!/usr/bin/env python3
"""Pulse PS2's front-end pulse clocks, read from EE RAM over PINE (`docs/ghidra/functions/ps2-pulse-eu/frontend-pulse.md`).

Two counters run apart on Grid and Cell Selection, as on the PSP:

- the `Selector`'s phase, `float` at `controller + 0xa0`, advanced `2 * dt` and wrapped at `2.0` by `GridController_UpdateSelectorStamp`
  (`0x0018ee30`), which also writes the stamp `int(80 + 150 t)` at `*(controller + 0xc0) + 0xfc`; ours' `--menu-picker-seconds` is `phase / 2`;
- the `pulse="true"` title's ladder, `Text_Draw` (`0x001a7fe8`): the byte `s` at `text + 0xed` steps 15 per tick of the front-end clock
  (`*(0x002e3640) + 0x40`, seconds, times 20), up to 150 and back, the direction in bit 31 of the word at `text + 0xec`, whose low byte is
  the tick last stepped to. Ours' `--menu-title-seconds` puts ours' step `s / 15` mid-tick.

The objects are found by shape in one RAM read and kept only if they move between two reads.
"""

import struct
import time

import ps2_ticker as pt

CLOCK_POINTER = 0x002E3640
CLOCK_SECONDS = 0x40
PHASE = 0xA0
SELECTOR = 0xC0
SELECTOR_STAMP = 0xFC
ACTIVE = 0x2C
TEXT_VTABLE_AT = 0x38
#: `Text`'s vtable pointer as `Text_Construct` leaves it (read live on Grid and Cell Selection; the table holds `Text_Draw` and `Text_ParseValues`).
TEXT_VTABLE = 0x00298118
TEXT_FLAGS = 0xCC
PULSE_FLAG = 8
LADDER_WORD = 0xEC
LADDER_S = 0xED


def _f32(blob, at):
    return struct.unpack_from("<f", blob, at)[0]


def _u32(blob, at):
    return struct.unpack_from("<I", blob, at)[0]


def stamp(phase):
    """The `Selector` stamp `GridController_UpdateSelectorStamp` writes for `phase`."""
    t = phase if phase <= 1.0 else 2.0 - phase
    return int(t * 150.0 + 80.0)


def candidates(ram, base=pt.RAM_START):
    """`(controllers, texts)`: addresses whose fields have the shape, before the liveness check."""
    controllers, texts = [], []
    top = len(ram) - 0x100
    for o in range(0, top, 4):
        sel = _u32(ram, o + SELECTOR)
        if base <= sel < base + top and sel % 4 == 0 and _u32(ram, o + ACTIVE) & 0x200:
            phase = _f32(ram, o + PHASE)
            # Loose: a 32 MiB read of a running VM spans many frames, so the phase and the stamp may be read a few frames apart.
            if 0.0 <= phase < 2.0 and 80 <= _u32(ram, sel - base + SELECTOR_STAMP) <= 230:
                controllers.append(base + o)
        if _u32(ram, o + TEXT_VTABLE_AT) == TEXT_VTABLE and _u32(ram, o + TEXT_FLAGS) & PULSE_FLAG:
            s = ram[o + LADDER_S]
            if s <= 150 and s % 15 == 0:
                texts.append(base + o)
    return controllers, texts


def clock(pine):
    return struct.unpack("<f", struct.pack("<I", pine.read32(pine.read32(CLOCK_POINTER) + CLOCK_SECONDS)))[0]


def read_state(pine, controller, text):
    """One read of both counters and the clock."""
    state = {"clock": clock(pine)}
    if controller is not None:
        state["phase"] = struct.unpack("<f", struct.pack("<I", pine.read32(controller + PHASE)))[0]
    if text is not None:
        word = pine.read32(text + LADDER_WORD)
        state["s"] = (word >> 8) & 0xFF
        state["up"] = word >> 31
        state["last_tick"] = word & 0xFF
    return state


def find(pine, wait=0.25):
    """`(controller, text)`: the one `Selector` controller whose phase moves and the one pulse `Text` whose ladder ticks, `None` for none or several."""
    controllers, texts = candidates(pt.read_ram(pine))
    first = {a: (pine.read32(a + PHASE), pine.read8(a + LADDER_WORD)) for a in controllers + texts}
    time.sleep(wait)
    live_c = [a for a in controllers if pine.read32(a + PHASE) != first[a][0] and _stamp_agrees(pine, a)]
    live_t = [a for a in texts if pine.read8(a + LADDER_WORD) != first[a][1]]
    return (live_c[0] if len(live_c) == 1 else None), (live_t[0] if len(live_t) == 1 else None)


def _stamp_agrees(pine, controller):
    """The controller's phase and its `Selector`'s stamp, read back to back, agree as `GridController_UpdateSelectorStamp` writes them
    (within one frame's step of 5)."""
    phase = struct.unpack("<f", struct.pack("<I", pine.read32(controller + PHASE)))[0]
    if not 0.0 <= phase < 2.0:
        return False
    return abs(pine.read32(pine.read32(controller + SELECTOR) + SELECTOR_STAMP) - stamp(phase)) <= 6


def step_ladder(s, up, last_tick, tick):
    """`Text_Draw`'s ladder caught up from `last_tick` to `tick` (bytes): one step of 15 per tick, a tick spent turning at 0 and 150.
    A tick count that wrapped (`tick < last_tick`) steps nothing, as the original's signed difference does."""
    for _ in range(max(0, tick - last_tick)):
        if up:
            if s < 150:
                s += 15
            else:
                up = 0
        elif s == 0:
            up = 1
        else:
            s -= 15
    return s, up


def at_fraction(before, after, frac):
    """Both counters where the frame `frac` of the way from `before` to `after` (two `read_state`s) had them."""
    frac = min(1.0, max(0.0, frac))
    seconds = before["clock"] + frac * (after["clock"] - before["clock"])
    placed = {"clock": seconds}
    if "phase" in before:
        placed["phase"] = (before["phase"] + 2.0 * (seconds - before["clock"])) % 2.0
    if "s" in before:
        placed["s"], placed["up"] = step_ladder(before["s"], before["up"], before["last_tick"], int(seconds * 20.0) & 0xFF)
    return placed


def picker_seconds(phase):
    """Ours' `--menu-picker-seconds` for a `Selector` phase: ours advances `2 * seconds`."""
    return phase / 2.0


def title_seconds(s):
    """Ours' `--menu-title-seconds` for a ladder byte: ours' step `k` is `floor(20 * seconds) % 20` and draws `s = 15 k` on the rising half."""
    return (s / 15.0 + 0.5) / 20.0


def _selftest():
    # 22 ticks a cycle: 0..150 rising, a tick held at 150 while turning, 150..0, a tick held at 0.
    s, up, seen = 0, 1, []
    for tick in range(22):
        s, up = step_ladder(s, up, tick, tick + 1)
        seen.append(s)
    assert seen == [15, 30, 45, 60, 75, 90, 105, 120, 135, 150, 150, 135, 120, 105, 90, 75, 60, 45, 30, 15, 0, 0], seen
    assert step_ladder(60, 1, 250, 3) == (60, 1)
    assert stamp(0.0) == 80 and stamp(1.0) == 230 and stamp(1.5) == 155
    for k in range(11):
        assert int(title_seconds(15 * k) * 20.0) % 20 == k
    placed = at_fraction({"clock": 10.0, "phase": 1.9}, {"clock": 10.1, "phase": 0.1}, 0.5)
    assert abs(placed["phase"] - 0.0) < 1e-4 or abs(placed["phase"] - 2.0) < 1e-4, placed


if __name__ == "__main__":
    _selftest()
    print("ok")
