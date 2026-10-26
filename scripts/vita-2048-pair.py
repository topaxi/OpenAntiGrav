#!/usr/bin/env python3
"""One original-vs-ours pair of Wipeout 2048 (Vita, EU) on the start grid before GO, environment-checked.

    export HOME=<scratch>/vhome SDL_AUDIODRIVER=dummy OAG_VITA3K_CACHE=<scratch>/vc OAG_VITA3K_CONFIG=<scratch>/vc/config.yml
    OAG_VITA3K_DISPLAY=91 XDG_CONFIG_HOME=<scratch>/profile/cfg \\
    python3 scripts/vita-2048-pair.py --out <scratch>/pairs/mall --event "2048 - Event 2" --walk

The 2048 counterpart of `psp-pure-pair.py`, built on `vita3k-drive.py`'s display, launcher and input. Vita3K gives no
per-tick state channel (see that script's docstring), so the walk waits on what is on screen instead of a fixed sleep:
each step grabs a frame, reduces it to a few numbers and goes on when the numbers say the next screen is up.

    --walk      boot Vita3K (if none of this session's is up), pass the title and the PSN prompt, open the Single Player
                Campaign and tap the event's node, then the play and confirm buttons, wait out the load on the loading
                bar, skip the ship flyby, and stop on the HUD. Without it Vita3K is assumed to be on a race already.
    always      pause, Retry, and grab a burst of frames every ~0.15 s while the countdown stands; the HUD's first
                frame stands for tick 0 of ours' countdown and the frame ~3 s later is the grid before GO.

The node tap is a screen coordinate on THIS SAVE'S campaign map (`--node`, default Queens Mall, "2048 - Event 2"): the
map's pan and which nodes are open depend on the profile's progress, so a different save needs its own `--node`.

What this pair pins down and what it does not, in the pair's own checks: the circuit and the craft are the event's (ours
runs `--event`), the grid slot is read off both HUDs' place by eye, the frame is the countdown's settled camera (the
original's camera sweeps in for ~0.5 s after the HUD appears). The phase inside the countdown is wall-clock: the
race clock reads zero for the whole countdown, so no clock can tell the tick, and `--grid-seconds` is the offset
after the HUD appears at the grab; ours renders `round(60 * offset)` ticks (the grab's own spacing, 0.15 s, is the
phase uncertainty). No pose is read: ours spawns the same grid slot the original does.

    --walk also patches the original's drawn-hull shake out, the way `rpcs3-hd-pair.py` does on RPCS3 (`rpcs3-hd-no-hull-shake.py`):
    2048's ship carries the same random B-spline shake as HD's (`HullShake_Update` at `0x811b51bc`, `HullShake_Apply` at
    `0x811b5500`, `docs/ghidra/functions/ps3-hdfury-eu/hull-shake.md`, "Cross-title"), ours is drawn with `--no-hull-shake`, and
    the shake moves the hull a few px between frames. Vita3K has no patch mechanism and its GDB stub halts the guest (see
    `patch_hull_shake`), so the two `bl HullShake_Apply` calls of the drawn hull are overwritten with `nop`s in the process's
    memory (`/proc/<pid>/mem`) right after the launch, which needs Vita3K to be this script's own child. `--no-patch-hull-shake`
    keeps the stock game; a Vita3K that was already running is not patched, and the pair records that.

    --read-pose (with --walk) reads the original's grid pose out of the same memory before the Retry (the drawn hull matrix nearest ours' slot),
    records it against ours' (`original.pose_read`, `pose_delta`) and renders a second ours on the original's pose, `ours-original-pose.png`
    (`--pose-ours-height` for an HD-derived hull, whose node sits above ours' craft position). `docs/tools/compare-page.md`, "The hull shake and
    the grid pose", has the method and the numbers.
"""

import argparse
import importlib.util
import json
import os
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402
import compare_provenance as cp  # noqa: E402

#: The EU v1.04 package ours races from: an extracted `PCSF00007` (`just` recipes in docs/formats/vita-package.md).
SOURCE = "data/extracted/vita/PCSF00007"
TITLE = "PCSF00007"

#: Touch coordinates in the 960x544 game window (`vita3k-capture.md`'s walk-in table, re-measured 2026-10-22).
GAME_MODE_SINGLE_PLAYER = (295, 162)
#: The HD Campaign tile beside it (the DLC's own campaign, `vita3k-capture.md`'s walk-in table).
GAME_MODE_HD_CAMPAIGN = (477, 162)
GAME_MODES = {"single": GAME_MODE_SINGLE_PLAYER, "hd": GAME_MODE_HD_CAMPAIGN}
CONFIRM = (882, 480)
#: The craft button left of the event card's Close and Play buttons (HD Campaign).
HD_SELECT_TEAM = (622, 480)
PAUSE_RETRY = (750, 480)
RETRY_YES = (402, 312)
#: The campaign map's first open node of a save that has finished "2048 - Event 1": Queens Mall.
DEFAULT_NODE = (270, 68)



#: The two `bl HullShake_Apply` of the drawn hull in the EU v1.04 `eboot.bin` (Thumb-2, `bl 0x811b5500`): `0x812cb010` in the drawn
#: hull matrix function (`0x812caf1c`, the only caller is `0x812cb4da`) and `0x811c15a0`; both load the hull scale (`DAT_8151fdd0`
#: / `DAT_8151fdf4`) right after. The third caller, `0x811be926`, shakes the ship's "internal tripod" (the camera node, `ship+0x5650`)
#: with `r2 = 1`, which also turns the node into the view matrix: a `nop` there blanks the camera (the HUD never came up, 2026-10-26),
#: so it is left alone. `HullShake_Update` keeps running, so `rand` is drawn exactly as before.
HULL_SHAKE_APPLY = 0x811B5500
HULL_SHAKE_SITES = (0x812CB010, 0x811C15A0)
NOP_NOP = bytes.fromhex("00bf00bf")
#: Where Vita3K maps the guest's 4 GiB: guest address `a` is at `GUEST_BASE + a` in the process (read off `/proc/<pid>/maps` and
#: confirmed by the bytes at the call sites; the scan below finds it again if a build moves it).
GUEST_BASE = 0x400000000
HOST_WINDOW = (0x400000000, 0x500000000)


def bl_bytes(site, target):
    """The Thumb-2 `bl target` at `site`, encoded (so a patch site is checked against what it must be, not trusted)."""
    off = target - (site + 4)
    s, i1, i2 = (off >> 24) & 1, (off >> 23) & 1, (off >> 22) & 1
    j1, j2 = (~(i1 ^ s)) & 1, (~(i2 ^ s)) & 1
    hw1 = 0xF000 | (s << 10) | ((off >> 12) & 0x3FF)
    hw2 = 0xD000 | (j1 << 13) | (j2 << 11) | ((off >> 1) & 0x7FF)
    return hw1.to_bytes(2, "little") + hw2.to_bytes(2, "little")


def quick_base(mem, probe_site, probe_bytes):
    """`GUEST_BASE` when the probe bytes are there (the guest's code is mapped), else None."""
    try:
        mem.seek(GUEST_BASE + probe_site)
        return GUEST_BASE if mem.read(len(probe_bytes)) == probe_bytes else None
    except OSError:
        return None


def scan_base(mem, pid, probe_site, probe_bytes):
    """The host address of guest address 0 found by scanning the readable mappings of the guest window for the probe bytes
    (they must be found exactly once): for a build that maps the guest elsewhere than `GUEST_BASE`."""
    found = []
    for line in open("/proc/%d/maps" % pid):
        span, perms = line.split()[:2]
        lo, hi = (int(x, 16) for x in span.split("-"))
        if not perms.startswith("r") or lo < HOST_WINDOW[0] or lo >= HOST_WINDOW[1]:
            continue
        try:
            mem.seek(lo)
            data = mem.read(hi - lo)
        except OSError:
            continue
        at = data.find(probe_bytes)
        while at >= 0:
            found.append(lo + at - probe_site)
            at = data.find(probe_bytes, at + 1)
    if len(found) != 1:
        sys.exit("the eboot's call site bytes were found %d times in Vita3K's memory (want exactly 1): nothing patched" % len(found))
    return found[0]


def patch_hull_shake(pid, timeout=30.0):
    """Overwrite the drawn hull's `bl HullShake_Apply` calls with `nop`s in the running Vita3K `pid` (a child of this process: Yama's
    `ptrace_scope` 1 refuses `/proc/<pid>/mem` to anything else), before the guest has run the code. Returns what was read back.

    Vita3K's GDB stub is not usable for this: it halts the guest at the connect, and the game then never got past the
    "Sony Computer Entertainment presents" screen or hung at 56 % of the first load (four boots, 2026-10-26), where the same
    config with `gdbstub: false` boots in about 30 s."""
    probe_site = HULL_SHAKE_SITES[0]
    probe = bl_bytes(probe_site, HULL_SHAKE_APPLY)
    sites = list(HULL_SHAKE_SITES)
    try:
        mem = open("/proc/%d/mem" % pid, "r+b", 0)
    except OSError as e:
        sys.exit("cannot open /proc/%d/mem (%s): Vita3K must be this script's own child" % (pid, e))
    end = time.monotonic() + timeout
    base = quick_base(mem, probe_site, probe)
    while base is None and time.monotonic() < end:
        time.sleep(0.5)
        base = quick_base(mem, probe_site, probe)
    if base is None:
        base = scan_base(mem, pid, probe_site, probe)
    record = {"method": "write to /proc/<pid>/mem of two Thumb nops over each `bl 0x811b5500`, before the guest ran the code", "host_base": hex(base),
              "sites": {}}
    for site in sites:
        want = bl_bytes(site, HULL_SHAKE_APPLY)
        mem.seek(base + site)
        before = mem.read(4)
        if before != want:
            sys.exit("%#x holds %s, not `bl %#x`: this is not the EU v1.04 eboot, nothing more patched" % (site, before.hex(), HULL_SHAKE_APPLY))
        mem.seek(base + site)
        mem.write(NOP_NOP)
        mem.seek(base + site)
        after = mem.read(4)
        if after != NOP_NOP:
            sys.exit("%#x reads %s after the write" % (site, after.hex()))
        record["sites"]["%#x" % site] = {"before": before.hex(), "after": after.hex()}
    mem.close()
    return record


def scan_original_pose(pid, near, tol=6.0):
    """The original's grid pose, read out of Vita3K's guest memory: the drawn hull's matrix (rows left, up, forward, scaled by the hull
    scale `0.75`, then the position) nearest `near`, the position of ours' craft at the same tick. The renderer keeps a copy per draw, so the
    best match is the one found most often. Returns `[(count, position, rows)]`, most frequent first; needs numpy and a Vita3K that is this
    process's child."""
    import numpy as np
    mem = open("/proc/%d/mem" % pid, "rb", 0)
    found = {}
    for line in open("/proc/%d/maps" % pid):
        span, perms = line.split()[:2]
        lo, hi = (int(v, 16) for v in span.split("-"))
        if not perms.startswith("rw") or not HOST_WINDOW[0] <= lo < HOST_WINDOW[1]:
            continue
        try:
            mem.seek(lo)
            data = mem.read(hi - lo)
        except OSError:
            continue
        f = np.frombuffer(data[: len(data) // 4 * 4], dtype="<f4")
        if len(f) < 16:
            continue
        with np.errstate(invalid="ignore"):
            hit = np.where((abs(f[12:-2] - near[0]) < tol) & (abs(f[13:-1] - near[1]) < tol) & (abs(f[14:] - near[2]) < tol))[0] + 12
        for i in hit:
            rows = f[i - 12:i].reshape(3, 4)
            lengths = np.linalg.norm(rows[:, :3], axis=1)
            if np.any(abs(rows[:, 3]) > 1e-6) or np.any(abs(lengths - 0.75) > 0.06) or np.any(abs(lengths - lengths[0]) > 1e-3):
                continue
            key = (tuple(round(float(v), 4) for v in f[i:i + 3]), tuple(round(float(v), 4) for v in rows[:, :3].ravel()))
            found[key] = found.get(key, 0) + 1
    return sorted(((n, pos, rows) for (pos, rows), n in found.items()), reverse=True)


def pose_delta(ours_pos, ours_rows, theirs_pos, theirs_rows):
    """Original minus ours: position (units), heading (degrees, + is left turn seen from above) and pitch (degrees, + nose up) of the
    forward row. Rows are (left, up, forward), any scale."""
    import math

    def heading_pitch(fwd):
        n = math.sqrt(sum(v * v for v in fwd))
        x, y, z = (v / n for v in fwd)
        return math.degrees(math.atan2(-x, -z)), math.degrees(math.asin(max(-1.0, min(1.0, y))))

    (h0, p0), (h1, p1) = heading_pitch(ours_rows[6:9]), heading_pitch(theirs_rows[6:9])
    wrap = (h1 - h0 + 180.0) % 360.0 - 180.0
    return {"position": [round(t - o, 4) for t, o in zip(theirs_pos, ours_pos)], "heading_deg": round(wrap, 3), "pitch_deg": round(p1 - p0, 3),
            "ours": {"heading_deg": round(h0, 3), "pitch_deg": round(p0, 3)}, "original": {"heading_deg": round(h1, 3), "pitch_deg": round(p1, 3)}}


def write_pose_csv(trace_csv, pose, path):
    """A one-row trace for `oag-game --pose-from`: ours' last row with the pose columns replaced by the original's. The trace's `right_*`
    columns hold row 0, the craft's LEFT (`Basis::LeftUpForward`)."""
    import csv
    rows = list(csv.DictReader(open(trace_csv)))
    row = rows[-1]
    pos, mat = pose
    for k, vec in (("right", mat[0:3]), ("up", mat[3:6]), ("fwd", mat[6:9]), ("pos", pos)):
        n = (sum(v * v for v in vec) ** 0.5) if k != "pos" else 1.0
        for axis, v in zip("xyz", vec):
            row["%s_%s" % (k, axis)] = repr(v / n)
    row["tick"] = "0"
    writer = csv.DictWriter(open(path, "w", newline=""), fieldnames=list(rows[0].keys()))
    writer.writeheader()
    writer.writerow(row)


def read_trace_pose(trace_csv):
    """Ours' position and (left, up, forward) rows from the last row of a `--trace-out` file; its `right_*` columns hold the true right here."""
    import csv
    row = list(csv.DictReader(open(trace_csv)))[-1]
    g = lambda k: [float(row[k + a]) for a in ("_x", "_y", "_z")]
    right, up, fwd = g("right"), g("up"), g("fwd")
    return g("pos"), [-v for v in right] + up + fwd


def load_drive():
    spec = importlib.util.spec_from_file_location("vita3k_drive", HERE / "vita3k-drive.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def small(path, size="96x54!"):
    """A frame reduced to bytes: 8-bit grey at `size`, the thing every screen predicate reads."""
    return subprocess.run(["magick", str(path), "-colorspace", "Gray", "-resize", size, "-depth", "8", "gray:-"],
                          capture_output=True, check=True).stdout


def crop_bytes(path, geometry):
    return subprocess.run(["magick", str(path), "-colorspace", "Gray", "-crop", geometry, "+repage", "-depth", "8", "gray:-"],
                          capture_output=True, check=True).stdout


def mean_luma(path):
    data = small(path)
    return sum(data) / len(data) / 255.0


def hud_up(path):
    """The race HUD is on screen: white "LAP" and "TOTAL" text at the top left of a dark frame. Loading screens and the
    pause menu are light, the ship flyby has a check mark and no text there."""
    if mean_luma(path) > 0.55:
        return False
    white = sum(1 for v in crop_bytes(path, "150x70+10+8") if v >= 235)
    return white >= 150


class Session:
    def __init__(self, display, frames):
        self.drive = load_drive()
        self.display = display
        self.frames = Path(frames)
        self.frames.mkdir(parents=True, exist_ok=True)
        self.wid = None

    def window(self, timeout=90.0):
        self.wid = self.drive.find_window(self.display, timeout=timeout)
        if self.wid is None:
            sys.exit("no Vita3K game window on :%d" % self.display)
        return self.wid

    def shot(self, name):
        return self.drive.screenshot(self.display, self.wid, self.frames / name)

    def tap(self, point, hold=0.3):
        self.drive.tap(self.display, point[0], point[1], hold_seconds=hold)

    def key(self, *keys, delay=400):
        self.drive.press(self.display, self.wid, *keys, delay=delay)

    def wait(self, what, predicate, timeout, every=1.5, name="wait.png"):
        """Grab frames until `predicate(frame)`; stop with the last frame kept when it never comes."""
        end = time.monotonic() + timeout
        while time.monotonic() < end:
            frame = self.shot(name)
            if predicate(frame):
                return frame
            time.sleep(every)
        sys.exit("STALLED waiting for %s (%.0f s); last frame %s" % (what, timeout, self.frames / name))


def boot_and_walk(s, args):
    d = s.drive
    d.bring_up_display(args.display, args.socket)
    if d.vita3k_alive() is None:
        pid = d.launch_vita3k(TITLE, args.display)
        args.vita_pid = pid
        if args.patch_hull_shake:
            args.hull_shake = patch_hull_shake(pid)
    elif args.patch_hull_shake:
        print("WARNING: Vita3K was already running, so it cannot be patched: the original's hull shake is ON", file=sys.stderr)
    s.window(args.boot_timeout)
    # The title: Return starts the intro, then three presses reach the Game Mode grid with the PSN prompt gone.
    time.sleep(8.0)
    s.key("Return")
    for _ in range(3):
        time.sleep(4.0)
        s.key("x")
    time.sleep(2.0)
    tile = GAME_MODES[args.game_mode]
    s.tap(tile)
    time.sleep(3.0)
    s.tap(tile)  # the first tap highlights the tile, the second opens it (measured)
    time.sleep(3.0)
    s.tap(args.node)
    time.sleep(3.0)
    if args.game_mode == "hd":
        # The HD Campaign's load stalls at 56 to 75 percent unless the craft was confirmed on Select Team first (four cold boots,
        # 2026-10-26): the wrench opens it on the default craft (Feisar HD), its check mark returns to the map, the node again.
        s.tap(HD_SELECT_TEAM)
        time.sleep(3.0)
        s.tap(CONFIRM)
        time.sleep(3.0)
        s.tap(args.node)
        time.sleep(3.0)
    s.tap(CONFIRM)  # Play
    time.sleep(3.0)
    s.tap(CONFIRM)  # the mode card
    # The menus are light (0.7-0.9 mean luma), the loading screen 0.9, the flyby and the race dark (0.2-0.4).
    # The HD Campaign's event card is a blurred picture of the circuit behind a white panel (mean luma under 0.55 long before
    # the load), so the load is not told by luma there: wait it out blind (about 6 s once the craft is confirmed), then the
    # tap loop below waits for the HUD itself.
    if args.game_mode == "single":
        s.wait("the load to finish (a dark frame after the light menus and loading screen)", lambda f: mean_luma(f) < 0.55,
               args.load_timeout, every=2.0, name="walk-load.png")
    else:
        time.sleep(20.0)
    # The ship flyby holds a check mark at the bottom right until it is tapped; the first tap may land on the load's tail.
    for _ in range(6):
        s.tap(CONFIRM)
        time.sleep(1.5)
        frame = s.shot("walk-hud.png")
        if hud_up(frame):
            return
    sys.exit("STALLED waiting for the HUD after the flyby; last frame %s" % (s.frames / "walk-hud.png"))


def parse_ram_reads(items):
    """`--ram-read NAME=ADDR:TYPE` into `[(name, address, type)]`. TYPE is `f32`, `u32`, `u8` or `hexN` (N bytes as hex); ADDR is a hex
    guest address, or `[PTR]+OFF` for OFF bytes past the u32 stored at PTR."""
    reads = []
    for item in items:
        name, _, rest = item.partition("=")
        addr, _, kind = rest.rpartition(":")
        if not name or not (kind in ("f32", "u32", "u8") or (kind.startswith("hex") and kind[3:].isdigit())):
            sys.exit("--ram-read wants NAME=ADDR:f32|u32|u8|hexN, got %r" % item)
        if addr.startswith("["):
            ptr, _, off = addr[1:].partition("]")
            reads.append((name, (int(ptr, 16), int(off.lstrip("+") or "0", 16)), kind))
        else:
            reads.append((name, int(addr, 16), kind))
    return reads


def read_guest(pid, reads):
    """One sample of every `--ram-read`, read out of Vita3K's guest memory (`/proc/<pid>/mem`, guest + `HOST_WINDOW[0]`); needs a Vita3K
    that is this process's child. Returns `{name: value}`."""
    import struct
    got = {}
    with open("/proc/%d/mem" % pid, "rb", 0) as mem:
        def raw(addr, n):
            mem.seek(HOST_WINDOW[0] + addr)
            return mem.read(n)
        for name, addr, kind in reads:
            if isinstance(addr, tuple):
                addr = struct.unpack("<I", raw(addr[0], 4))[0] + addr[1]
            if kind.startswith("hex"):
                got[name] = raw(addr, int(kind[3:])).hex()
                continue
            b = raw(addr, 1 if kind == "u8" else 4)
            got[name] = {"f32": lambda b: struct.unpack("<f", b)[0], "u32": lambda b: struct.unpack("<I", b)[0], "u8": lambda b: b[0]}[kind](b)
    return got


def parse_ram_pokes(items):
    """`--ram-poke ADDR=VALUE:f32[@FROM]` into `[(guest_address, value, from_grab)]`."""
    pokes = []
    for item in items:
        addr, _, rest = item.partition("=")
        rest, _, start = rest.partition("@")
        value, _, kind = rest.partition(":")
        if kind != "f32":
            sys.exit("--ram-poke wants ADDR=VALUE:f32[@FROM], got %r" % item)
        pokes.append((int(addr, 16), float(value), int(start or 0)))
    return pokes


def poke_guest(pid, pokes, grab):
    """Write every `--ram-poke` whose first grab is at or before `grab` into Vita3K's guest memory; needs a Vita3K that is this process's
    child. Returns `{address: value}` of what was written."""
    import struct
    done = {}
    with open("/proc/%d/mem" % pid, "r+b", 0) as mem:
        for addr, value, start in pokes:
            if grab >= start:
                mem.seek(HOST_WINDOW[0] + addr)
                mem.write(struct.pack("<f", value))
                done["%#x" % addr] = value
    return done


def restart_and_burst(s, count, tag, sample=None):
    """Pause, Retry, and grab `count` frames back to back; returns `[(t, path)]`, t seconds from the confirm tap. `sample`, when given, is
    called with the grab's index before each grab and its result is returned as a third element, `[(t, path, sample(i))]`."""
    s.key("Return")
    time.sleep(1.5)
    s.tap(PAUSE_RETRY)
    time.sleep(1.2)
    s.tap(RETRY_YES)
    t0 = time.monotonic()
    out = []
    for i in range(count):
        path = s.frames / ("%s%03d.png" % (tag, i))
        got = sample(i) if sample else None
        s.drive.screenshot(s.display, s.wid, path)
        out.append((time.monotonic() - t0, path) + ((got,) if sample else ()))
    return out


def pick_grid_frame(burst, offset):
    """The first frame with the HUD up is tick 0 of the countdown; the frame `offset` seconds later is the grid.
    Returns `(index_of_hud, index_picked, t_hud, t_picked)` or None."""
    hud = None
    for i, b in enumerate(burst):
        if hud_up(b[1]):
            hud = i
            break
    if hud is None:
        return None
    t_hud = burst[hud][0]
    picked = min(range(hud, len(burst)), key=lambda i: abs(burst[i][0] - (t_hud + offset)))
    return hud, picked, t_hud, burst[picked][0]


def yellow_hull(path, width=960, height=544):
    """The Feisar hull's yellow livery in the bottom 60% of the frame, columns 150 to 750: `(count, cx, cy, x0, x1, y0, y1)` over the pixels
    that are saturated yellow (r > 0.7, g > 0.55, b < 0.4). The hull is the only large yellow thing there; the HUD is white."""
    top, left, wide = int(height * 0.4), 150, 600
    raw = subprocess.run(["magick", str(path), "-crop", "%dx%d+%d+%d" % (wide, height - top, left, top), "+repage", "-depth", "8", "rgb:-"],
                         capture_output=True, check=True).stdout
    n = cx = cy = 0
    x0, x1, y0, y1 = width, 0, height, 0
    for i in range(0, len(raw), 3):
        r, g, b = raw[i], raw[i + 1], raw[i + 2]
        if r > 178 and g > 140 and b < 102:
            px, py = (i // 3) % wide + left, (i // 3) // wide + top
            n += 1
            cx += px
            cy += py
            x0, x1, y0, y1 = min(x0, px), max(x1, px), min(y0, py), max(y1, py)
    if not n:
        return None
    return {"count": n, "cx": cx / n, "cy": cy / n, "x0": x0, "x1": x1, "y0": y0, "y1": y1}


def craft_offset(theirs, ours):
    """Where the original's hull sits against ours': the offset of the two yellow-livery centroids in px (`dx`, `dy`:
    how far the original's is right of and below ours'), the width and height ratios of their bounding boxes (original
    over ours: above 1 means the original's hull is drawn larger, a nearer camera or a bigger field of view), and the
    pixel counts. Edge correlation is not used: the two hulls differ in shading and a few degrees of yaw, and a
    correlation over mismatched shapes reads near zero whether or not the hulls sit in the same place."""
    a, b = yellow_hull(theirs), yellow_hull(ours)
    if a is None or b is None:
        return {"error": "no yellow hull found in %s" % ("the original" if a is None else "ours")}
    return {"dx": a["cx"] - b["cx"], "dy": a["cy"] - b["cy"],
            "width_ratio": (a["x1"] - a["x0"]) / max(1, b["x1"] - b["x0"]),
            "height_ratio": (a["y1"] - a["y0"]) / max(1, b["y1"] - b["y0"]),
            "original": a, "ours": b}


def run(cmd, **kw):
    print("+", " ".join(str(c) for c in cmd), file=sys.stderr)
    return subprocess.run([str(c) for c in cmd], check=True, **kw)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--event", default="2048 - Event 2", help="our --event, the campaign event the original was walked into")
    ap.add_argument("--game-mode", choices=sorted(GAME_MODES), default="single",
                    help="the Game Mode tile to open: `single` (Single Player Campaign) or `hd` (the DLC's HD Campaign)")
    ap.add_argument("--track", default=None,
                    help="ours races this circuit (`--track <vex entry> --mode`) instead of `--event`: a circuit no campaign event of ours names, "
                         "such as a DLC one (`Data\\art\\published\\DLC1\\environments\\Vineta_K\\track.vex`)")
    ap.add_argument("--mode", default="single_race", help="ours' race mode with --track")
    ap.add_argument("--team", default=None,
                    help="ours' `--team` id; the DLC's HD Campaign flies the HD-derived roster (`Feisar`, under `hdships`), not 2048's own `feisar2048`")
    ap.add_argument("--node", type=int, nargs=2, default=DEFAULT_NODE, metavar=("X", "Y"),
                    help="the event's node on the campaign map, in the 960x544 window")
    ap.add_argument("--nose-original", type=float, nargs=2, metavar=("X", "Y"),
                    help="the hull's nose tip in the original's frame, read by eye off a 4x crop: for a craft with no yellow livery to find "
                         "(the HD Campaign's Feisar), where the centroid offset cannot be measured; needs --nose-ours")
    ap.add_argument("--nose-ours", type=float, nargs=2, metavar=("X", "Y"), help="the same in ours' frame")
    ap.add_argument("--no-patch-hull-shake", dest="patch_hull_shake", action="store_false",
                    help="keep the original's drawn-hull shake (default with --walk: patched out in the process's memory, see the docstring)")
    ap.add_argument("--read-pose", action="store_true",
                    help="with --walk (Vita3K this script launched): read the original's grid pose out of the guest's memory and record it against "
                         "ours' (`original.pose_read`, `pose_delta`), and render a second ours with the craft put on the original's pose "
                         "(`ours-original-pose.png`)")
    ap.add_argument("--pose-ours-height", action="store_true",
                    help="with --read-pose: keep ours' own height in the posed render. The drawn node of an HD-derived hull (the DLC's Feisar) "
                         "sits 1.85 units above ours' craft position (floor 33 px off if taken as is); 2048's own hulls agree to 0.05")
    ap.add_argument("--walk", action="store_true", help="boot and walk from the title to the HUD (else Vita3K is on a race already)")
    ap.add_argument("--display", type=int, default=int(os.environ.get("OAG_VITA3K_DISPLAY", 0)) or None)
    ap.add_argument("--socket", default=None, help="weston's socket name (with --walk, if the display is not up)")
    ap.add_argument("--boot-timeout", type=float, default=90.0)
    ap.add_argument("--load-timeout", type=float, default=240.0)
    ap.add_argument("--grid-seconds", type=float, default=3.0, help="seconds after the HUD's first frame the grid frame is taken")
    ap.add_argument("--burst", type=int, default=44, help="frames in the countdown burst")
    ap.add_argument("--ours-ticks", type=int, default=None, help="ticks ours renders (default round(60 * the grab's offset))")
    ap.add_argument("--platform", default="2048")
    ap.add_argument("--source", default=str(ROOT / SOURCE))
    ap.add_argument("--oag-game", default=str(ROOT / "target/debug/oag-game"))
    ap.add_argument("--ram-read", action="append", default=[], metavar="NAME=ADDR:TYPE",
                    help="with --walk: read a guest value (f32, u32, u8 or hexN bytes at a hex address, or at `[PTR]+OFF`) after every burst grab "
                         "and record the series")
    ap.add_argument("--ram-poke", action="append", default=[], metavar="ADDR=VALUE:f32[@FROM]",
                    help="with --walk: write an f32 into the guest before every burst grab from grab FROM on (default 0), recorded with the reads; "
                         "a pair made with one is a probe, not a reference")
    ap.add_argument("--meta", default="", help="JSON object for the manifest: id, title, track, craft, state, note")
    cp.add_arguments(ap)
    args = ap.parse_args()
    if args.display is None:
        sys.exit("--display (or OAG_VITA3K_DISPLAY) must name this session's own Xwayland display")
    if "OAG_VITA3K_CACHE" not in os.environ:
        sys.exit("OAG_VITA3K_CACHE must name a directory of this session's own (the drive script's pid file and config live in it)")
    if not os.environ.get("XDG_CONFIG_HOME", "").startswith(str(ce.MAIN / "data" / "scratch")):
        sys.exit("XDG_CONFIG_HOME must be a directory under data/scratch/<lane>/: oag-game persists settings there")
    args.socket = args.socket or "wayland-oag%d" % args.display
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    pair = ce.Pair(args.platform, out)
    plat = pair.plat
    profile_dir = Path(os.environ["XDG_CONFIG_HOME"]).parent
    ce.write_ours_settings(plat, profile_dir)

    args.hull_shake = None
    args.vita_pid = None
    which = ["--track", args.track, "--mode", args.mode] if args.track else ["--event", args.event]
    which += ["--team", args.team] if args.team else []
    env = {**os.environ, **ce.ours_env(profile_dir)}
    ours_trace = None
    if args.read_pose:
        if not args.walk:
            sys.exit("--read-pose needs --walk: the memory can only be read of a Vita3K this script launched")
        # ours' craft at the grid tick, to know where in the original's memory to look and what to compare with
        ours_trace = out / "ours-trace.csv"
        run([args.oag_game, args.source, "--race"] + which + ["--ticks", args.ours_ticks or 179, "--no-audio", "--no-hull-shake",
                                                              "--trace-out", ours_trace], stdout=subprocess.DEVNULL, env=env)
    s = Session(args.display, out / "frames")
    if args.walk:
        boot_and_walk(s, args)
    else:
        s.window(10.0)
    original_pose = None
    if args.read_pose:
        ours_pos, ours_rows = read_trace_pose(ours_trace)
        found = scan_original_pose(args.vita_pid, ours_pos)
        pair.check("the original's grid pose was found in its memory", bool(found),
                   "%d distinct poses within 6 units of ours' slot; the most frequent seen %d times" % (len(found), found[0][0]) if found else "none")
        if found:
            count, pos, rows = found[0]
            original_pose = {"position": list(pos), "rows_left_up_forward_scaled": list(rows), "seen": count, "others": len(found) - 1}
    reads = parse_ram_reads(args.ram_read)
    pokes = parse_ram_pokes(args.ram_poke)
    if (reads or pokes) and not args.walk:
        sys.exit("--ram-read and --ram-poke need --walk: the memory is only open to a Vita3K this script launched")

    def sample(i):
        poked = poke_guest(args.vita_pid, pokes, i) if pokes else {}
        return {**read_guest(args.vita_pid, reads), **({"poked": poked} if pokes else {})}
    burst = restart_and_burst(s, args.burst, "burst", sample if reads or pokes else None)
    picked = pick_grid_frame(burst, args.grid_seconds)
    pair.check("the countdown's HUD came up after Retry", picked is not None,
               "first HUD frame at %s" % (("%.2f s" % picked[2]) if picked else "none of %d frames" % len(burst)))
    if picked is None:
        pair.finish()
    hud_i, pick_i, t_hud, t_pick = picked
    frame = burst[pick_i][1]
    if reads or pokes:
        pair.rec["original"]["ram_read"] = {
            "method": "/proc/<pid>/mem at guest address + 0x400000000, written (--ram-poke) then read just before each burst grab",
            "addresses": {name: "%s:%s" % (("[%#x]+%#x" % addr) if isinstance(addr, tuple) else "%#x" % addr, kind) for name, addr, kind in reads},
            "at_grid_frame": burst[pick_i][2],
            "series": [{"t": round(b[0] - t_hud, 3), "frame": Path(b[1]).name, **b[2]} for b in burst[hud_i:]]}
        print("MEASURED ram at the grid frame: %s" % json.dumps(burst[pick_i][2]), file=sys.stderr)
    (out / "theirs.png").write_bytes(Path(frame).read_bytes())
    pair.rec["original"]["emulator"] = dict(plat["emulator"])
    pair.rec["original"]["hull_shake"] = (
        {"state": "patched out", "camera_tripod_shake": "on", **args.hull_shake} if args.hull_shake else "on")
    pair.rec["original"]["grab"] = {"hud_first_frame_s": t_hud, "grid_frame_s": t_pick, "burst_frames": len(burst),
                                    "burst_interval_s": (burst[-1][0] - burst[0][0]) / (len(burst) - 1),
                                    "grid_offset_s": t_pick - t_hud}
    # the grid is at rest: the frames after the camera settles (0.5 s after the HUD) agree to the animated sign boards
    settled = [b[1] for b in burst[hud_i:] if 0.6 <= b[0] - t_hud <= t_pick - t_hud + 0.01]
    first_settled = settled[0]
    drift = []
    for p in settled[1:]:
        r = subprocess.run(["compare", "-metric", "RMSE", str(first_settled), str(p), "null:"], capture_output=True, text=True)
        drift.append(float(r.stderr.split("(")[1].split(")")[0]))
    pair.rec["original"]["settled_rmse_vs_first_settled_max"] = max(drift) if drift else None
    # the original's chase camera and hull sway through the countdown: the hull's centroid over the settled frames is the
    # spread a single grab sits inside, which the offset to ours is to be read against
    cents = [h for h in (yellow_hull(p) for p in settled[::2]) if h]
    if cents:
        pair.rec["original"]["hull_centroid_range_px"] = {
            "frames": len(cents), "x": [min(h["cx"] for h in cents), max(h["cx"] for h in cents)],
            "y": [min(h["cy"] for h in cents), max(h["cy"] for h in cents)]}
    shot_size = ce.image_size(out / "theirs.png")
    pair.check_equal("original window size", shot_size, list(plat["output"]))
    pair.check("the original's picture is steady across the countdown, hull shake and sign animation only (frames from 0.6 s after the HUD to the grab)",
               bool(drift) and max(drift) < 0.15,
               "largest RMSE of %d later frames against the first settled one: %.3f" % (len(drift), max(drift) if drift else -1))
    cp.build(pair, args, "scenery", "cold_boot" if args.walk else "savestate_restore", None,
             {"value": 0, "unit": "ticks", "basis": "not measured on Vita3K; the grid is at rest, so a pose cannot lag the picture",
              "what": "picture"})

    ticks = args.ours_ticks if args.ours_ticks is not None else round(60 * (t_pick - t_hud))
    pair.rec["ours"]["ticks"] = ticks
    pair.rec["ours"]["event"] = args.track or args.event
    pair.check("ours renders before GO", ticks < ce.go_tick(plat), "tick %d, GO at tick %d" % (ticks, ce.go_tick(plat)))
    pair.rec["ours"]["binary"] = ce.binary_record(args.oag_game)
    base = [args.oag_game, args.source, "--race"] + which + ["--ticks", ticks] + ce.ours_flags(plat)
    pair.rec["ours"]["flags"] = [str(c) for c in base[2:]]
    run(base + ["--screenshot", out / "ours.png"], stdout=subprocess.DEVNULL, env=env)
    if original_pose:
        pair.rec["original"]["pose_read"] = {
            **original_pose, "method": "float-triple scan of the guest's memory (/proc/<pid>/mem) for the drawn hull's matrix nearest ours' "
            "slot, read on the first countdown of the walk, hull scale 0.75"}
        pair.rec["pose_delta"] = pose_delta(ours_pos, ours_rows, original_pose["position"], original_pose["rows_left_up_forward_scaled"])
        print("MEASURED original minus ours: %s" % json.dumps(pair.rec["pose_delta"]), file=sys.stderr)
        pose_csv = out / "original-pose.csv"
        posed = list(original_pose["position"])
        if args.pose_ours_height:
            posed[1] = ours_pos[1]
        write_pose_csv(ours_trace, (posed, original_pose["rows_left_up_forward_scaled"]), pose_csv)
        run(base + ["--pose-from", pose_csv, "--pose-hold", "--screenshot", out / "ours-original-pose.png"], stdout=subprocess.DEVNULL, env=env)
    got, bad = ce.read_back(plat, profile_dir)
    pair.rec["ours"]["settings_readback"] = got
    for b in bad:
        pair.check("ours settings", False, b)
    pair.check("ours settings read back", not bad, "%d keys equal the profile" % (len(got) - len(bad)))
    pair.check_size("ours", out / "ours.png")
    if args.nose_original and args.nose_ours:
        dx, dy = args.nose_original[0] - args.nose_ours[0], args.nose_original[1] - args.nose_ours[1]
        pair.rec["craft_offset_px"] = {"nose_tip": {"dx": dx, "dy": dy, "original": args.nose_original, "ours": args.nose_ours,
                                                     "read": "by eye, 4x crops, +-1 px"}}
        pair.check("the hull's nose tip is read in both frames", True, "nose tip offset (%+.1f, %+.1f) px, original minus ours" % (dx, dy))
        print("MEASURED nose tip offset (%+.1f, %+.1f) px" % (dx, dy), file=sys.stderr)
        if abs(dx) > 6 or abs(dy) > 6:
            pair.caveat("the grid pose or camera is not matched: nose tip offset (%+.1f, %+.1f) px" % (dx, dy))
    else:
        off = craft_offset(out / "theirs.png", out / "ours.png")
        pair.rec["craft_offset_px"] = off
        found = "error" not in off
        pair.check("the livery-yellow hull is found in both frames", found, off["error"] if not found else "original %d px, ours %d px" % (
            off["original"]["count"], off["ours"]["count"]))
        if found:
            text = ("hull centroid offset (%+.1f, %+.1f) px, bounding box %.2fx wide, %.2fx high (original over ours)"
                    % (off["dx"], off["dy"], off["width_ratio"], off["height_ratio"]))
            print("MEASURED %s" % text, file=sys.stderr)
            if abs(off["dx"]) > 6 or abs(off["dy"]) > 6 or not 0.9 <= off["width_ratio"] <= 1.1:
                pair.caveat("the grid pose or camera is not matched: " + text)
    run(["magick", "montage", out / "theirs.png", out / "ours.png", "-tile", "2x1", "-geometry", "+2+2", out / "side.png"])
    if args.meta:
        pair.rec["meta"] = json.loads(args.meta)
    pair.finish()
    print("wrote", out / "side.png")


if __name__ == "__main__":
    main()
