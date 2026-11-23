#!/usr/bin/env python3
"""Original-vs-ours pairs of the PSP titles' front-end screens (PPSSPP software renderer), environment-checked.

    source <scratch>/pp/env.sh    # DISPLAY, OAG_PPSSPP_PORT, PP_HOME of a `scripts/ppsspp-start.sh` instance
    uv run --with websocket-client python3 scripts/psp-menu-pair.py walk --out D --plan "boot,shot:a,key:down,wait:1,shot:b"   # explore
    ... pairs --platform pulse-psp --out D --oag-game target/debug/oag-game                                                     # `WALK`

The PPSSPP counterpart of `rpcs3-hd-menu-pair.py`, `shadps4-omega-menu-pair.py` and `vita3k-2048-menu-pair.py`, on
`compare_menu.py`'s route. A front-end screen is a still: the pair is the same screen, size, language and highlighted row, with ours
drawn from the disc's own front-end data (`--menu-page track-select`, `grid-select`, `cell-select`, `pure-chain:...`), never from
`menu.toml`'s stand-in pages, which are **gaps** (the original's frame kept, `compare_menu.write_gap`).

The screen of each Pulse frame is read from the game's own state machine (`Debugger.state_name`, `0x08b31784 + 0x18c`, Pulse USA);
Pure has no recovered counterpart, so its screen is declared in `WALK` and read by eye (`original.screen_basis` says which). The
frame is the emulator window at 480x272 (`InternalResolution = 1`), grabbed off the Xvfb root and cropped at the window origin.
"""

import argparse
import datetime
import importlib.util
import json
import os
import struct
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402
import compare_menu as cm  # noqa: E402
import compare_provenance as cp  # noqa: E402
from ppsspp_debugger import Debugger  # noqa: E402
import psp_ticker as pt  # noqa: E402

_spec = importlib.util.spec_from_file_location("psp_drive", HERE / "psp-drive.py")
psp_drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(psp_drive)
FIRST_BOOT = psp_drive.FIRST_BOOT
named = psp_drive.named


def xdo(*args):
    return subprocess.run(["xdotool", *args], capture_output=True, text=True, check=True).stdout


def window():
    """`(id, x, y, w, h)` of PPSSPP's own window (960x544 or 480x272)."""
    best = None
    for wid in xdo("search", "--onlyvisible", "--name", "").split():
        geo = xdo("getwindowgeometry", "--shell", wid)
        v = dict(line.split("=") for line in geo.split() if "=" in line)
        x, y, w, h = (int(v[k]) for k in ("X", "Y", "WIDTH", "HEIGHT"))
        if (w, h) in ((960, 544), (480, 272)):
            best = (wid, x, y, w, h)
    if best is None:
        sys.exit("no PPSSPP window of 960x544 or 480x272 on DISPLAY")
    return best


def set_window(w, h):
    wid, x, y, cw, ch = window()
    if (cw, ch) != (w, h):
        xdo("windowsize", wid, str(w), str(h))
        time.sleep(2.0)
    return window()


def orange_share(png):
    """Share of strongly orange pixels: Pure's title (the yellow-orange WipEout logo on white)."""
    from PIL import Image
    px = Image.open(png).convert("RGB").getdata()
    return sum(1 for r, g, b in px if r > 230 and 150 < g < 215 and b < 90) / len(px)


#: Frame predicates for `pu:`; each takes the grabbed png.
#: Seconds between state reads while waiting for a screen: the screen-entry clock (`original.seconds_since_entry`) is good to half of it.
ENTRY_POLL = 0.03

#: The `WALK` ids whose ours is drawn at the original's seconds since entry (`--menu-picker-seconds`): the pages with a measured clock law.
CLOCKED = ("grid", "cell", "team", "track")

#: The front end's animation clock: `*(0x08b317b0) + 0x40`, seconds of emulated time (`AnimTransform_Update`'s fallback clock while
#: `g_ingame` is null; `docs/ghidra/functions/psp-pulse-usa/anim-transform.md`). It stops with the CPU, so unlike a wall clock it does not
#: depend on how fast the software renderer runs. `team` and `track` read their seconds since entry off it.
FE_CLOCK_PTR = 0x08B317B0
FE_CLOCK_FIELD = 0x40
#: A shot of these ids is grabbed with the CPU stopped, so the frame and the clock read are one moment.
STOPPED = ("grid", "cell", "team", "track", "main", "racebox", "single", "options")
#: The shots whose footer ticker is pinned (`tick:N` before the `shot`): every Pulse screen with the footer's ticker, so the message ours draws is the one grabbed.
TICKERED = ("grid", "cell", "team", "main", "racebox", "single", "options")

#: Pure has no recovered front-end clock or state name, so its clock is the emulator's own tick counter (`cpu.status.ticks`, the PSP's
#: 222 MHz CPU clock, which stops with the CPU and ran at 1.0000 of wall time over two 52 s recordings), read at the start of the pad
#: press that entered the screen and again with the CPU stopped for the grab. Error: the press-to-screen latency (a frame or two) and
#: the one frame the presented picture lags the stop. The `PURE_WALK` ids whose description types in (`oag_ui_screens::campaign::footer::typein`).
PURE_CLOCKED = ("main", "mode", "sr-class", "sr-league", "sr-track", "team", "t-class", "t-sel")
PSP_CPU_HZ = 222e6
#: The disc id and version a Pure save state's file name carries (`psp-state.py --serial`).
PURE_STATE_SERIAL = "UCUS98612_2.00"
#: Where `CURRENT` reads 0 in ours' drive of `scripts/pure-pause.drive`: the tick after `until stage=race` at which the HUD clock starts (1,480 ticks read 20.1 s).
PAUSE_CLOCK_ZERO_TICKS = {"pure": 274, "pulse": 276}
#: A Time Trial on Vineta K, Feisar, Vector, just restarted: the pause menu's own frame is grabbed from it.
PURE_PAUSE_STATE = "data/saves/pure-psp/time-trial-vineta-vector-feisar-restart.ppst"
#: Pulse's: a Time Trial on Moa Therma White, Venom, at the grid's GO (`pulse-psp/moa-therma-white`'s state; ours is `scripts/pulse-pause.drive`).
PULSE_PAUSE_STATE = "data/saves/pulse-psp/time-trial-moa-therma-white-venom-grid.ppst"
#: Pulse's pause walk: 4 s after START the header bar has finished its slide (`Animation delay="1"`, a 1 s `Key` run), which ours draws at rest.
PULSE_PAUSE_PLAN = ",".join(["state:" + PULSE_PAUSE_STATE, "wait:4", "key:start", "wait:4", "shot:pause"])
#: The pause screen's walk, run apart from `PURE_PLAN` (a state load leaves the front end): `--plan "$PURE_PAUSE_PLAN" --ids pause`.
PURE_PAUSE_PLAN = ",".join(["state:" + PURE_PAUSE_STATE, "wait:4", "key:start", "wait:2", "shot:pause"])
#: `team`: the ship's own `Anim Transform` node (loop `LoopEnd` 5.9833336 s on Assegai's `ship_FE.vex`) holds its own time at `+0x40`.
SHIP_NODE_LOOP = 5.9833336

#: Where the `Selector`'s quad is on `grid` and `cell` (a GE dump: 42x43 at (60,138) and (120,90)): its centre pixel's alpha in the
#: displayed EDRAM buffer is the glow-mask byte the quad stamps, `80 + 150 * t` of the pulse (`docs/ui/campaign-screens.md`, "The front
#: end's bloom, wired for the cursor"), which gives the pulse phase exactly where the wall clock does not.
SELECTOR_CENTRE = {"grid": (81, 159), "cell": (141, 111)}
EDRAM = 0x04000000
#: The card title's glyph box on `grid` and `cell` (x0, y0, x1, y1): `Text_Draw` stamps `220 - s` over its glyph quads, `s` a ladder 0, 15 .. 150 and back
#: that steps every 1/20 s of a counter of its own (`docs/ui/campaign-screens.md`, "The front end's bloom, wired from the widgets' own attributes").
TITLE_BOX = {"grid": (262, 88, 340, 116), "cell": (262, 30, 420, 60)}

PREDICATES = {"title": lambda png: orange_share(png) > 0.03}


class Rig:
    def __init__(self, port, shots, title):
        self.port = port
        self.dbg = Debugger(port)
        self.shots = shots
        self.title = title
        self.seen = {}
        self.pressed_clock = None  # the front end's animation clock at the press straight before an `st:` step
        self.entered_clock = None  # the front end's animation clock at the poll that saw the state name change
        self.entered = None  # (state, time.monotonic() when `expect` saw it replace another): the screen-entry clock
        self.pressed_at = None  # time.monotonic() of the press straight before an `st:` step: the entry cannot be earlier
        self.press_ticks = None  # Pure: `cpu.status.ticks` at the start of the latest pad press
        self.watch = pt.Watch()
        self.ticker_frozen = None  # `wait_ticker`'s state, taken with the CPU stopped, for the `shot` straight after
        if self.dbg.is_stepping():
            self.dbg.call("cpu.resume")
            time.sleep(0.5)

    def state(self):
        """Pulse only: the front end's own state name. Pure has none recovered."""
        return self.dbg.state_name() if self.title == "pulse" else None

    def tap(self, button, times=1, wait=0.7):
        for _ in range(times):
            if self.title == "pure":
                self.press_ticks = self.dbg.call("cpu.status")["ticks"]
            self.dbg.press(button, duration=6)
            time.sleep(wait)

    def expect(self, wanted, timeout=20.0):
        end = time.time() + timeout
        seen = None
        last_other = None
        while time.time() < end:
            seen = self.state()
            if named(seen, wanted):
                # entry lies between the last poll that saw another screen and this one: take the midpoint (+-ENTRY_POLL/2)
                # Seen already on the first poll: the entry was not watched, so there is no clock (`None`), never "now".
                now = time.monotonic()
                # The press that causes the entry returns after the screen has already changed on a settled emulator, so the first
                # poll can see the new name at once: the entry then lies between the press and that poll, a wider window than a
                # watched change (which is `ENTRY_POLL`).
                before = last_other if last_other is not None else self.pressed_at
                self.pressed_at = None
                self.entered = (seen, None if before is None else (before + now) / 2)
                self.entered_clock = (self.pressed_clock if last_other is None and self.pressed_clock is not None else self.fe_clock()) if self.title == "pulse" else None
                self.pressed_clock = None
                return
            last_other = time.monotonic()
            time.sleep(ENTRY_POLL)
        sys.exit("STALLED: front end is in %r, not %r" % (seen, wanted))

    def back_to(self, wanted, timeout=30.0):
        """Press circle until the front end announces `wanted` (Pulse only)."""
        end = time.time() + timeout
        while time.time() < end:
            if named(self.state(), wanted):
                return
            self.tap("circle", wait=1.2)
        sys.exit("STALLED backing out to %r, in %r" % (wanted, self.state()))

    def press_until(self, button, predicate, timeout=90.0):
        """Press `button` every 3 s until the grabbed frame satisfies `predicate` (`PREDICATES`): Pure has no recovered state name, and
        a fixed-sleep boot walk runs a screen behind or ahead depending on the load."""
        end = time.time() + timeout
        while time.time() < end:
            if PREDICATES[predicate](self.shot("pu")):
                return
            self.tap(button, wait=3.0)
        sys.exit("STALLED pressing %s until %s" % (button, predicate))

    def fe_clock(self):
        """The front end's animation clock, seconds (emulated time), or `None` when its object is not there."""
        obj = self.dbg.read_u32(FE_CLOCK_PTR)
        return self.dbg.read_f32(obj + FE_CLOCK_FIELD) if obj else None

    def ship_node_time(self):
        """`team`: the live ship `Anim Transform`'s own time (`node+0x40`) - the one node with this loop whose time moves with the clock
        (`node+0x80` equals the front end's clock), or `None`. The CPU must be stopped."""
        clock = self.fe_clock()
        key = struct.pack("<f", SHIP_NODE_LOOP)
        best = None
        for base in range(0x08800000, 0x0A000000, 0x400000):
            raw = self.dbg.read(base, 0x400000)
            at = raw.find(key)
            while at >= 0:
                node = base + at - 0x54
                if node >= 0x08800000 and node % 4 == 0 and clock is not None and abs(self.dbg.read_f32(node + 0x80) - clock) < 0.1 and self.dbg.read_u32(node + 0x84) == 0:
                    best = self.dbg.read_f32(node + 0x40)
                at = raw.find(key, at + 1)
        return best

    def title_stamp(self, name):
        """The seconds that put ours' card-title glow ladder where the original's is: the displayed buffer's most common glow-mask byte over the
        title's glyph box (`220 - 15 k`, `k` 0 to 10) read as step `k` of the rising half, at the middle of its 1/20 s. `None` when no byte of the ladder
        is there. The CPU must be stopped."""
        x0, y0, x1, y1 = TITLE_BOX[name]
        for offset in (0, 0x88000):
            raw = self.dbg.read(EDRAM + offset, 512 * 272 * 4)
            if raw[(5 * 512 + 5) * 4 + 3] != 4:
                continue
            counts = {}
            for y in range(y0, y1):
                row = raw[(y * 512 + x0) * 4 + 3:(y * 512 + x1) * 4 + 3:4]
                for a in row:
                    if a != 4:
                        counts[a] = counts.get(a, 0) + 1
            if not counts:
                continue
            value = max(counts, key=counts.get)
            if value in range(70, 221, 15):
                return {"value": value, "seconds": ((220 - value) // 15 + 0.5) / 20, "pixels": counts[value]}
        return None

    def selector_stamp(self, name):
        """The pulse level `t` (0 cyan, 1 white) read off the displayed buffer's `Selector` stamp, or `None`. The CPU must be stopped."""
        cx, cy = SELECTOR_CENTRE[name]
        for offset in (0, 0x88000):
            raw = self.dbg.read(EDRAM + offset, 512 * 272 * 4)
            ref = raw[(cy * 512 + cx) * 4 + 3]
            if raw[(5 * 512 + 5) * 4 + 3] == 4 and ref != 4:
                return max(0.0, min(1.0, (ref - 80) / 150.0))
        return None

    def wait_ticker(self, timeout=480.0):
        """Pulse's footer ticker draws its next message by `rand()` and the grab cannot choose it, so this waits for a frame ours can show:
        exactly one message on screen, its item one `records/lifetime.rs` counts, its head well inside the viewport. It returns with the CPU
        **stopped**, the state kept in `ticker_frozen`, and the `shot` that follows grabs that very frame (docs/tools/compare-page.md,
        "`menu-team`: the footer is the original's random ticker draw")."""
        ticker = pt.Ticker(self.dbg, self.news)
        if not ticker.find():
            sys.exit("STALLED: no footer ticker widget in RAM (is the front end on a screen with a footer?)")
        end = time.time() + timeout
        polls = 0
        while time.time() < end:
            self.dbg.call("cpu.stepping")
            sample = ticker.sample()
            self.watch.observe(sample)
            seen = self.watch.visible(sample)
            polls += 1
            if seen and 40 <= seen[1] <= 240:
                item, head_x, text = seen
                self.ticker_frozen = {"item": item, "head_x": head_x, "text": text, "sample": sample, "polls": polls}
                print("ticker", item["id"], "head_x", head_x, repr(text), "after", polls, "polls", flush=True)
                return
            self.dbg.call("cpu.resume")
            if polls % 40 == 0:
                print("ticker: %d polls, last message %s" % (polls, sample["item"] and sample["item"]["id"]), flush=True)
            time.sleep(0.25)
        sys.exit("STALLED: no pinnable footer message in %.0f s (%d polls)" % (timeout, polls))

    def shot(self, name):
        wid, x, y, w, h = set_window(480, 272)
        dst = self.shots / (name + ".png")
        stopped = name.split("@")[0] in (STOPPED if self.title == "pulse" else PURE_CLOCKED)
        if stopped:
            # Frozen for the grab and the read, so the frame and the stamp are one pulse phase.
            if not self.dbg.is_stepping():
                self.dbg.call("cpu.stepping")
            time.sleep(0.25)
        before = time.monotonic()
        subprocess.run(["import", "-window", "root", "-crop", "%dx%d+%d+%d" % (w, h, x, y), "+repage", str(dst)], check=True)
        after = time.monotonic()
        level = None
        title = tag = None
        since_ticks = None
        clock = ship_t = None
        if stopped:
            if name in SELECTOR_CENTRE:
                level = self.selector_stamp(name)
            if name in TITLE_BOX:
                title = self.title_stamp(name)
                if title is None:
                    print("title glow: no ladder byte over the title box of %s; ours keeps the Selector's phase for it" % name, flush=True)
            if self.title == "pulse":
                tag = pt.profile_tag(self.dbg)
            if self.title == "pure":
                ticks = self.dbg.call("cpu.status")["ticks"]
                if self.press_ticks is not None:
                    since_ticks = (ticks - self.press_ticks) / PSP_CPU_HZ
            clock = self.fe_clock() if self.title == "pulse" else None
            if self.title == "pulse" and name.split("@")[0] == "team":
                ship_t = self.ship_node_time()
            self.dbg.call("cpu.resume")
        state = self.state()
        # The grab takes `after - before`; the frame is read about the middle of it. Only when the entry was watched and the screen is still it.
        since = (before + after) / 2 - self.entered[1] if self.entered and self.entered[0] == state and self.entered[1] is not None else None
        phase = None if level is None else level / 2
        if phase is None and clock is not None and self.entered_clock is not None and self.entered and self.entered[0] == state:
            since = clock - self.entered_clock
        if since_ticks is not None:
            since = since_ticks
        self.seen[name] = {"png": dst, "at": datetime.datetime.now().astimezone().isoformat(timespec="seconds"), "state": state,
                           "since_entry": phase if phase is not None else since, "from_stamp": phase is not None, "level": level, "from_clock": phase is None and clock is not None, "ship_t": ship_t, "from_ticks": since_ticks is not None, "title": title, "tag": tag,
                           "ticker": self.ticker_frozen if stopped and name.split("@")[0] in TICKERED else None}
        if self.seen[name]["ticker"] is not None:
            self.ticker_frozen = None
        print("shot", name, "since_entry", self.seen[name]["since_entry"], "ship_node_time", ship_t, "clock", clock, flush=True)
        return dst

    def boot(self, timeout=240.0):
        """Answer the first-boot dialogs until `Main Menu`. Pulse reads the state; Pure is walked by `start`/`cross` until it settles."""
        end = time.time() + timeout
        while time.time() < end:
            st = self.state()
            if self.title == "pulse":
                if named(st, psp_drive.MAIN_MENU):
                    return
                keys = next((k for name, k in FIRST_BOOT if st == name), None)
                if st and st.startswith(psp_drive.DEMO_PREFIX):
                    self.tap("cross", wait=1.2)
                elif keys is not None:
                    for button, times in keys:
                        self.tap(button, times)
                    time.sleep(1.5)
                elif st:
                    self.tap("circle", wait=1.5)
                else:
                    time.sleep(0.5)
            else:
                time.sleep(2.0)
                return
        sys.exit("STALLED: never reached Main Menu, last state %r" % self.state())

    def load_state(self, path):
        """Load a save state (`psp-state.py`, by the emulator's own pause menu: a 960x544 window) and let the game run on from it."""
        set_window(960, 544)
        if self.dbg.is_stepping():
            self.dbg.call("cpu.resume")
        serial = ["--serial", PURE_STATE_SERIAL] if self.title == "pure" else []
        subprocess.run([sys.executable, str(HERE / "psp-state.py"), "--port", str(self.port), "--home", os.environ["PP_HOME"], "load", "2",
                        "--file", str(path)] + serial, check=True)
        self.dbg = Debugger(self.port)

    def run(self, plan):
        """Plan steps: boot, wait:S, key:BUTTON[*N], pu:BUTTON:PREDICATE, st:STATE, back:STATE (Pulse state wait), state:PATH (a save
        state), shot:NAME."""
        steps = list(filter(None, plan.split(",")))
        for at, item in enumerate(steps):
            kind, _, rest = item.partition(":")
            if kind == "boot":
                self.boot()
            elif kind == "wait":
                time.sleep(float(rest))
            elif kind == "key":
                button, _, n = rest.partition("*")
                # A key straight before `st:` skips the tap's 0.7 s settle so `expect` sees the screen change itself (the entry clock).
                watched = steps[at + 1:at + 2] and steps[at + 1].startswith("st:") and int(n or 1) == 1
                self.pressed_at = time.monotonic() if watched else None
                # The press call returns only after the button's six frames, by which time the screen has long changed: the entry clock is
                # the front end's clock as the press starts (the screen changes a frame or two after it), not the clock at the poll.
                self.pressed_clock = self.fe_clock() if watched and self.title == "pulse" else None
                self.tap(button, 1, wait=0.0) if watched else self.tap(button, int(n or 1))
            elif kind == "pu":
                button, _, pred = rest.partition(":")
                self.press_until(button, pred)
            elif kind == "back":
                self.back_to(rest)
            elif kind == "st":
                self.expect(rest)
            elif kind == "state":
                self.load_state(rest)
            elif kind == "tick":
                self.wait_ticker(float(rest or 480))
            elif kind == "shot":
                self.shot(rest)
            else:
                sys.exit("unknown plan step %r" % item)
            print("step", item, "state", self.state(), flush=True)


#: One walk per title, from `Main Menu` (Pulse; Pure is walked from its own main page): `shot:ID` photographs the screen `WALK` names.
PULSE_PLAN = ",".join([
    "back:Main Menu", "wait:2", "tick:480", "shot:main",
    "key:cross", "st:Grid Selection", "wait:3", "tick:480", "shot:grid",
    "key:cross", "st:Cell Selection", "wait:3", "tick:480", "shot:cell", "back:Main Menu", "wait:1",
    "key:down", "key:cross", "wait:2", "st:Racebox", "wait:2", "tick:480", "shot:racebox",
    "key:cross", "wait:2", "st:Single Player", "wait:2", "tick:480", "shot:single",
    "key:cross", "st:Track Creation", "wait:3", "shot:track",
    "key:cross", "st:Team Selection", "wait:3", "tick:480", "shot:team", "back:Main Menu", "wait:1",
    "key:down*5", "wait:1", "key:cross", "wait:2", "st:Additional", "wait:2", "tick:480", "shot:options", "back:Main Menu", "wait:1",
])

#: `screen` is the state name the game reports (Pulse) or the name read by eye (Pure); `page` is ours' `--menu-page`, `xml` where ours
#: draws it from, `None` for a gap (ours draws it from `menu.toml`, or not at all).
PULSE_WALK = [
    {"id": "title", "title": "Title: Press Start", "screen": "Show Logo", "page": "show-logo", "xml": "the title screen's own XML (`--until \"Show Logo\" --press cross --press-until-reached --ticks 200`: the sequence draws the backdrop `--screen` leaves out)",
     "route": ["--until", "Show Logo", "--press", "cross", "--press-until-reached", "--ticks", "200"],
     "selection": {"original": "settled, the prompt", "ours": "settled, the prompt"}},
    {"id": "grid", "title": "Race Campaign: Grid Selection", "screen": "Grid Selection", "page": "grid-select",
     "xml": "CellMode_Definition.xml (Grid Selection, the tier hexes) and grid_00.xml (the card)",
     "selection": {"original": "Grid 1, the first tier", "ours": "Grid 1, the first tier"}},
    {"id": "cell", "title": "Race Campaign: Cell Selection (campaign cell)", "screen": "Cell Selection", "page": "cell-select",
     "xml": "CellMode_Definition.xml (Cell Selection) and grid_00.xml",
     "selection": {"original": "Single Race, Moa Therma White, AI difficulty Medium (the footer)", "ours": "Single Race, Moa Therma White, AI difficulty Medium (the footer)"}},
    {"id": "track", "title": "Track Select", "screen": "Track Creation", "page": "track-select", "xml": "Selection_Definition.xml (Track Creation)",
     "selection": {"original": "first circuit of the fresh profile's list", "ours": "first circuit of the fresh profile's list"}},
    {"id": "team", "title": "Ship Select", "screen": "Team Selection", "page": "ship-select", "xml": "Selection_Definition.xml (Team Selection)",
     "selection": {"original": "Assegai, first unlocked craft", "ours": "Assegai, first unlocked craft"}},
    {"id": "main", "title": "Main Menu", "screen": "Main Menu", "page": "pulse-main", "xml": "MainMenu_Definition.xml (`Main Menu`: the title, the seven rows, `helptext0`)",
     "selection": {"original": "RACE CAMPAIGN (first row)", "ours": "RACE CAMPAIGN (first row)"}},
    {"id": "racebox", "title": "Racebox", "screen": "Racebox", "page": "pulse-racebox", "xml": "RaceBox_Definition.xml (`Racebox`: the three rows)",
     "selection": {"original": "CUSTOM RACE (first row)", "ours": "CUSTOM RACE (first row)"}},
    {"id": "single", "title": "Racebox setup (Single Player)", "screen": "Single Player", "page": "pulse-single", "xml": "RaceBox_Definition.xml (`Single Player`: the five lists, the rules)",
     "selection": {"original": "RACE TYPE (first row), SINGLE RACE, VENOM, ON, EASY", "ours": "RACE TYPE (first row), SINGLE RACE, VENOM, ON, EASY"}},
    {"id": "options", "title": "Options+", "screen": "Additional", "page": "pulse-options", "xml": "Additional_Definition.xml (`Additional`: the three rows)",
     "selection": {"original": "GAME OPTIONS (first row)", "ours": "GAME OPTIONS (first row)"}},
    {"id": "pause", "title": "Pause menu (Time Trial)", "screen": "InGame Pause SP Time Trial", "page": "pause", "xml": "InGame_Definition.xml (`InGame Pause SP Time Trial`: the six rows, `Music List`)",
     "route": ["--drive-file", str(HERE / "pulse-pause.drive"), "--original-menu", "--state-log", "/dev/null"],
     "selection": {"original": "CONTINUE (first row)", "ours": "CONTINUE (first row)"}},
]


#: Pure from a cold boot: the language and memory stick screens are answered by pressing cross until the title is up (a boot with no
#: recovered state name), then START, Profile > Load (three crosses: the list, the dialog, the completion), and the Single Player chain.
#: Triangle is Pure's BACK.
PURE_PLAN = ",".join([
    "pu:cross:title", "wait:3", "shot:title", "key:start", "wait:5", "key:cross", "wait:5", "key:cross", "wait:5", "key:cross", "wait:6",
    "key:cross", "wait:4", "shot:mode", "key:triangle", "wait:3", "shot:main",
    "key:cross", "wait:4", "key:cross", "wait:4", "shot:sr-class", "key:cross", "wait:5", "shot:sr-league", "key:cross", "wait:5", "shot:sr-track",
    "key:cross", "wait:5", "shot:team", "key:triangle", "wait:3", "key:triangle", "wait:3", "key:triangle", "wait:3", "key:triangle", "wait:3",
    "key:down", "wait:2", "key:cross", "wait:4", "shot:t-class", "key:cross", "wait:5", "shot:t-sel",
])

PURE_WALK = [
    {"id": "title", "title": "Title: Press Start", "screen": "Title Screen", "page": "title-screen", "xml": "Intro Screen->Title Screen (`--screen`)",
     "route": ["--screen", "Intro Screen->Title Screen", "--ticks", "200"],
     "selection": {"original": "the settled title, PRESS START BUTTON", "ours": "the settled title, PRESS START BUTTON"}},
    {"id": "mode", "title": "Single Player: mode select", "screen": "Single Player", "page": "pure-chain", "xml": "Skin.xml Single Player",
     "selection": {"original": "SINGLE RACE (first row)", "ours": "SINGLE RACE (first row)"}},
    {"id": "sr-class", "title": "Single Race: Class Selection", "screen": "Class Selection", "page": "pure-chain:arcade", "xml": "Selection_Definition.xml Class Selection",
     "selection": {"original": "VECTOR (first row)", "ours": "VECTOR (first row)"}},
    {"id": "sr-league", "title": "Single Race: League Selection", "screen": "League Selection", "page": "pure-chain:arcade:0", "xml": "Selection_Definition.xml League Selection",
     "selection": {"original": "ALPHA (first row)", "ours": "ALPHA (first row)"}},
    {"id": "sr-track", "title": "Single Race: Track Selection", "screen": "Track Selection", "page": "pure-chain:arcade:0:0", "xml": "Selection_Definition.xml Track Selection",
     "selection": {"original": "VINETA K (first row)", "ours": "VINETA K (first row)"}},
    {"id": "team", "title": "Team Selection", "screen": "Team Selection", "page": "ship-select", "xml": "Selection_Definition.xml Team Selection",
     "selection": {"original": "FEISAR (first row)", "ours": "FEISAR (first row)"}},
    {"id": "t-class", "title": "Tournament: Class Selection", "screen": "Class Selection", "page": "pure-chain:tournament", "xml": "Selection_Definition.xml Class Selection",
     "selection": {"original": "VECTOR (first row)", "ours": "VECTOR (first row)"}},
    {"id": "t-sel", "title": "Tournament: Tournament Selection (the campaign)", "screen": "Tournament Selection", "page": "pure-chain:tournament:0",
     "xml": "Selection_Definition.xml Tournament Selection",
     "selection": {"original": "ALPHA (the only tournament open), class VECTOR", "ours": "ALPHA (the only tournament open), class VECTOR"}},
    {"id": "pause", "title": "Pause menu (Time Trial)", "screen": "InGame Main Menu", "page": "pause", "xml": "InGame_Definition.xml InGame Main Menu",
     "route": ["--drive-file", str(HERE / "pure-pause.drive"), "--original-menu", "--state-log", "/dev/null"],
     "selection": {"original": "CONTINUE (first row)", "ours": "CONTINUE (first row)"}},
    {"id": "main", "title": "Main Menu", "screen": "Main Menu", "page": "pure-main", "xml": "Skin.xml Main Menu",
     "selection": {"original": "SINGLE PLAYER (first row)", "ours": "SINGLE PLAYER (first row)"}},
]


def prepare_profile(plat, profile, title_name, difficulty, race=None, player=None):
    """Ours' private profile: the compare settings, `[dev] unlock_everything = false` (a debug build unlocks everything, the original's
    fresh profile does not), `race` (the pickers' cursor: the team the original's fresh list rests on, where it is not ours' default) and
    the campaign difficulty row of `records.toml` (none for a title with no such control), written and read back by the run."""
    ce.write_ours_settings(plat, profile)
    cm.extra_settings(profile, race=race, locked=True, player=player)
    if difficulty:
        (ce.settings_path(profile).parent / "records.toml").write_text('[[campaign_difficulty]]\ntitle = "%s"\ndifficulty = "%s"\n' % (title_name.lower(), difficulty))


def ticker_box_grey(theirs, ours):
    """Mean grey difference of the ticker's own box (`oag_ui_screens::campaign::footer::TickerLayout::viewport`, 85, 235, 370 x 32) between two frames."""
    from PIL import Image
    import numpy as np
    box = (85, 235, 455, 267)
    a = np.asarray(Image.open(theirs).convert("L").crop(box), dtype=float)
    b = np.asarray(Image.open(ours).convert("L").crop(box), dtype=float)
    return float(abs(a - b).mean())


def fit_ticker_lag(args, plat, profile, item, theirs, clocked, pin, pdir):
    """0 or 1: the frames the presented picture lags the CPU stop the footer grab was taken at, read off which render of the pinned message matches."""
    diffs = []
    for lag in (0, 1):
        png = pdir / ("ticker-lag%d.png" % lag)
        cm.render_ours(args.oag_game, args.source, plat, item["page"], png, profile, ce.size_text(plat), clocked + ("--menu-ticker", "%s@%s" % (pin["item"]["id"], pin["head_x"] + lag)))
        diffs.append(round(ticker_box_grey(theirs, png), 3))
        png.unlink()
    return (0 if diffs[0] <= diffs[1] else 1), diffs


def build_pairs(args, seen, out, plat, walk):
    """One pair or gap per `WALK` entry from the grabbed frames; returns the ids `Pair.finish` refused."""
    refused = []
    profile = Path(os.environ["XDG_CONFIG_HOME"]).parent
    for item in walk:
        if args.ids and item["id"] not in args.ids.split(","):
            continue
        if item["id"] not in seen:
            # A walk shoots only what its plan names: the pause screen is its own plan (`PURE_PAUSE_PLAN`, a state load leaves the front end).
            if args.ids:
                sys.exit("--ids names %r but the plan never shot it (the pause screen needs --plan PURE_PAUSE_PLAN)" % item["id"])
            print("skip", item["id"], "(not shot by this plan)")
            continue
        shot = seen[item["id"]]
        if "why" in item:
            cm.write_gap(out / ("gap-" + item["id"]), shot["png"], item["page"], "menu.toml",
                         "ours draws %s only from menu.toml or not at all: %s (the disc's own screen is %s)" % (item["screen"], item["why"], item["xml"]))
            print("gap", item["id"])
            continue
        pdir = out / ("menu-" + item["id"])
        pdir.mkdir(parents=True, exist_ok=True)
        pair = ce.Pair(args.platform, pdir)
        pair.rec["meta"] = {"id": "%s-menu-%s" % (args.platform, item["id"]), "title": item["title"], "track": "front end", "craft": "none (a menu)",
                            "note": "Ours is `oag-game --menu-page %s`, drawn from %s. Both sides English, a fresh profile, campaign difficulty Medium." % (item["page"], item["xml"]),
                            "state": ("PPSSPP software renderer, a Time Trial restored from a save state, START pressed" if item["id"] == "pause" else "PPSSPP software renderer, walked by pad from Main Menu on a fresh memory stick")}
        pair.rec["original"]["emulator"] = dict(plat["emulator"])
        pair.rec["original"]["captured_at"] = shot["at"]
        pair.rec["original"]["capture_path"] = ("restored from a save state, then START (the state is not committed: data/saves/)" if item["id"] == "pause" else "walked by pad from Main Menu (no savestate)")
        pair.rec["original"]["selection_basis"] = "read by eye from the frame; the cursor is where the walk left it"
        if shot.get("from_stamp"):
            pair.rec["original"]["seconds_since_entry"] = {"value": round(shot["since_entry"], 3), "basis": "not a wall clock: the pulse level %.3f the Selector's own glow-mask stamp reads in EDRAM (80 + 150 t, CPU stopped for the grab), as the seconds at which ours shows that level (t / 2 on the rising half; the original has no entry hold, `0x088a3e8c`); good to one game frame" % shot["level"]}
        elif shot.get("from_ticks"):
            pair.rec["original"]["seconds_since_entry"] = {"value": round(shot["since_entry"], 3), "basis": "not a wall clock: the emulator's own tick counter (cpu.status.ticks / 222e6, CPU stopped for the grab) at the grab minus its value at the start of the pad press that entered the screen; the press-to-screen latency (a frame or two) and the one frame the picture lags the stop are inside it. Pure has no recovered front-end clock"}
        elif shot.get("from_clock") and shot.get("since_entry") is not None:
            pair.rec["original"]["seconds_since_entry"] = {"value": round(shot["since_entry"], 3), "basis": "not a wall clock: the front end's own animation clock (*(0x08b317b0)+0x40, emulated seconds, CPU stopped for the grab) at the grab minus its value at the poll that saw the state name change; good to one poll (a frame or two)" + ("; the ship's Anim Transform node time read %.3f s (loop 5.983 s)" % shot["ship_t"] if shot.get("ship_t") is not None else "")}
        elif shot.get("since_entry") is not None:
            pair.rec["original"]["seconds_since_entry"] = {"value": round(shot["since_entry"], 3), "basis": "measured: wall clock from the poll that saw the state name change (+-%.2f s; when the first poll already saw the new name, from the midpoint between the press and that poll, wider) to the middle of the frame grab; PPSSPP runs at 60 fps, its pulse period reads 1 s on this clock" % (ENTRY_POLL / 2)}
        pair.rec["original"]["screen_basis"] = ("the game's own state machine (0x08b31784+0x18c): %r" % shot["state"]) if shot["state"] else "read by eye from the frame (no recovered state name for this title)"
        prepare_profile(plat, profile, args.campaign_title, args.difficulty, item.get("race"), {"tag": shot["tag"]} if shot.get("tag") else None)
        if shot.get("tag"):
            pair.rec["original"]["player"] = {"tag": shot["tag"], "basis": "the profile record's footer tag (profile + 0x457, three characters) read from RAM with the CPU stopped (scripts/psp_ticker.py `profile_tag`); ours stores it as `player.tag`, which the footer's tag box draws"}
        clocked = (("--menu-picker-seconds", "%.3f" % shot["since_entry"]) + (("--menu-model-seconds", "%.3f" % shot["ship_t"]) if shot.get("ship_t") is not None else ())) if item["id"] in (CLOCKED if args.title == "pulse" else PURE_CLOCKED) and shot.get("since_entry") is not None else ()
        if shot.get("title"):
            clocked += ("--menu-title-seconds", "%.3f" % shot["title"]["seconds"])
            pair.rec["original"]["title_glow"] = {"value": shot["title"]["value"], "pixels": shot["title"]["pixels"], "seconds": round(shot["title"]["seconds"], 3),
                                                  "basis": "the displayed buffer's glow-mask byte over the card title's glyph box (CPU stopped for the grab), a step of the title's own ladder (220 - 15 k): ours pins its title ladder to that step (--menu-title-seconds), apart from the Selector's phase"}
        pin = shot.get("ticker")
        lag = 0
        if pin:
            # Ours' counters are put where the original's table is, so ours draws the message the grab caught from its own statistics.
            records = ce.settings_path(profile).parent / "records.toml"
            records.write_text((records.read_text() if records.exists() else "") + "\n" + pt.ours_lifetime(pin["sample"], args.campaign_title.lower()))
            pair.rec["original"]["footer_ticker"] = {
                "basis": "read from RAM with the CPU stopped, the frame grabbed in that stop (scripts/psp_ticker.py): the message is a rand() draw, so the "
                         "grab waits for one ours can show and records it",
                "item": pin["item"]["id"], "head_x": pin["head_x"], "text": pin["text"], "polls": pin["polls"],
                "widget": pin["sample"]["widget"], "records": pin["sample"]["buffers"], "next_starts_at": pin["sample"]["next_starts_at"],
                "stats_u32": pin["sample"]["stats_u32"], "favourites": pin["sample"]["favourites"]}
            lag, diffs = fit_ticker_lag(args, plat, profile, item, shot["png"], clocked, pin, pdir)
            pair.rec["original"]["footer_ticker"]["lag_frames"] = {
                "value": lag, "ticker_box_mean_grey": diffs,
                "basis": "the presented picture lags the CPU stop by 0 or 1 frame (a message moves one pixel a frame), so ours is drawn at the head read and at the head "
                         "plus one and the one that matches the grabbed ticker box (85, 235, 370 x 32) better is kept"}
        cp.build(pair, args, "menu", "cold_boot", None, {"value": None, "unit": None, "basis": "not applicable: a menu is a still"})
        route = item.get("route")
        if item["id"] == "pause":
            # The drive plays to the clock the original's HUD shows: `PAUSE_GO_TICKS` sim ticks to GO's board, then the clock's seconds.
            if args.pause_clock is None:
                sys.exit("the pause pair needs --pause-clock: the race clock (`CURRENT`) the original's frame shows, read by eye")
            zero = PAUSE_CLOCK_ZERO_TICKS[args.title]
            ticks = round(zero + args.pause_clock * 60)
            (pdir / "ours.drive").write_text(Path(route[1]).read_text().replace("@TICKS@", str(ticks)))
            route = [route[0], str(pdir / "ours.drive")] + route[2:]
            pair.rec["original"]["race_clock_read"] = {"value": args.pause_clock, "basis": "the HUD's CURRENT in the grabbed frame, read by eye; ours is played %d sim ticks (clock zero at tick %d, 60 a second)" % (ticks, zero)}
        try:
            cm.menu_pair(pair, theirs_png=shot["png"], oag_game=args.oag_game, image=args.source, page=item["page"], drawn_from="xml",
                         xml_file=item["xml"], screen_seen=shot["state"] or item["screen"], screen_wanted=item["screen"], profile_dir=profile,
                         size=ce.size_text(plat), front_end_style="the disc's own", language="English", original_language="English",
                         selection=item["selection"], route=route, race=item.get("race"), player={"tag": shot["tag"]} if shot.get("tag") else None,
                         extra_args=clocked + (("--menu-ticker", "%s@%s" % (pin["item"]["id"], pin["head_x"] + lag)) if pin else ()), screen_source="PPSSPP, the front end's state name" if shot["state"] else "read by eye")
            pair.finish()
        except SystemExit as exc:
            if exc.code == 3:
                refused.append(item["id"])
                continue
            raise
        print("pair", item["id"], "ok")
    return refused


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("cmd", choices=("walk", "pairs"))
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--plan", default="")
    ap.add_argument("--title", choices=("pulse", "pure"), default="pulse")
    ap.add_argument("--port", type=int, default=int(os.environ.get("OAG_PPSSPP_PORT", 0)) or None)
    ap.add_argument("--platform", default=None, help="compare-environment.toml platform (default: <title>-psp)")
    ap.add_argument("--source", default=None, help="the disc image ours reads (default: the platform's image)")
    ap.add_argument("--oag-game", default=str(HERE.parent / "target/debug/oag-game"))
    ap.add_argument("--campaign-title", default=None, help="records.toml campaign_difficulty row title (default: the game's name)")
    ap.add_argument("--difficulty", default="medium")
    ap.add_argument("--ids", default="", help="pairs: only these WALK ids (comma separated)")
    ap.add_argument("--from-png", action="append", default=[], metavar="ID=PNG",
                    help="pairs: build the pair of WALK id ID from an original frame already grabbed, with no emulator (a re-make of ours' side)")
    ap.add_argument("--pause-clock", type=float, default=None,
                    help="pairs, id pause: the race clock the original's HUD `CURRENT` reads in the frame (read by eye), in seconds; ours is played to it")
    cp.add_arguments(ap)
    args = ap.parse_args()
    args.platform = args.platform or args.title + "-psp"
    out = args.out.resolve()
    shots = out / "shots"
    shots.mkdir(parents=True, exist_ok=True)
    rig = None if args.from_png else Rig(args.port, shots, args.title)
    if args.cmd == "walk":
        rig.run(args.plan)
        return 0
    if not os.environ.get("XDG_CONFIG_HOME", "").startswith(str(ce.MAIN / "data" / "scratch")):
        sys.exit("XDG_CONFIG_HOME must be a directory under data/scratch/<lane>/: oag-game persists settings there")
    plat = ce.platform(args.platform)
    args.source = args.source or str(ce.MAIN / plat["image"])
    args.campaign_title = args.campaign_title or plat["title"]
    if args.title == "pulse" and rig is not None:
        rig.news = pt.load_rows(Path(args.oag_game).with_name("oag-wad"), args.source + ":" + "PSP_GAME/USRDIR/Data.wad") if "tick:" in (args.plan or PULSE_PLAN) else None
    if rig is None:
        seen = {}
        for spec in args.from_png:
            wid, _, png = spec.partition("=")
            seen[wid] = {"png": Path(png), "at": datetime.datetime.fromtimestamp(Path(png).stat().st_mtime).astimezone().isoformat(timespec="seconds"),
                         "state": None, "since_entry": None, "from_stamp": False, "level": None, "from_clock": False, "ship_t": None,
                         "from_ticks": False, "title": None, "tag": None, "ticker": None}
    else:
        rig.run(args.plan or (PULSE_PLAN if args.title == "pulse" else PURE_PLAN))
        seen = rig.seen
    refused = build_pairs(args, seen, out, plat, PULSE_WALK if args.title == "pulse" else PURE_WALK)
    json.dump({"refused": refused}, open(out / "refused.json", "w"))
    return 3 if refused else 0


if __name__ == "__main__":
    sys.exit(main())
