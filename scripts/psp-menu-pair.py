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
CLOCKED = ("grid", "cell")

PREDICATES = {"title": lambda png: orange_share(png) > 0.03}


class Rig:
    def __init__(self, port, shots, title):
        self.dbg = Debugger(port)
        self.shots = shots
        self.title = title
        self.seen = {}
        self.entered = None  # (state, time.monotonic() when `expect` saw it replace another): the screen-entry clock
        self.pressed_at = None  # time.monotonic() of the press straight before an `st:` step: the entry cannot be earlier
        if self.dbg.is_stepping():
            self.dbg.call("cpu.resume")
            time.sleep(0.5)

    def state(self):
        """Pulse only: the front end's own state name. Pure has none recovered."""
        return self.dbg.state_name() if self.title == "pulse" else None

    def tap(self, button, times=1, wait=0.7):
        for _ in range(times):
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

    def shot(self, name):
        wid, x, y, w, h = set_window(480, 272)
        dst = self.shots / (name + ".png")
        before = time.monotonic()
        subprocess.run(["import", "-window", "root", "-crop", "%dx%d+%d+%d" % (w, h, x, y), "+repage", str(dst)], check=True)
        after = time.monotonic()
        state = self.state()
        # The grab takes `after - before`; the frame is read about the middle of it. Only when the entry was watched and the screen is still it.
        since = (before + after) / 2 - self.entered[1] if self.entered and self.entered[0] == state and self.entered[1] is not None else None
        self.seen[name] = {"png": dst, "at": datetime.datetime.now().astimezone().isoformat(timespec="seconds"), "state": state, "since_entry": since}
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

    def run(self, plan):
        """Plan steps: boot, wait:S, key:BUTTON[*N], pu:BUTTON:PREDICATE, st:STATE, back:STATE (Pulse state wait), shot:NAME."""
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
                self.tap(button, 1, wait=0.0) if watched else self.tap(button, int(n or 1))
            elif kind == "pu":
                button, _, pred = rest.partition(":")
                self.press_until(button, pred)
            elif kind == "back":
                self.back_to(rest)
            elif kind == "st":
                self.expect(rest)
            elif kind == "shot":
                self.shot(rest)
            else:
                sys.exit("unknown plan step %r" % item)
            print("step", item, "state", self.state(), flush=True)


#: One walk per title, from `Main Menu` (Pulse; Pure is walked from its own main page): `shot:ID` photographs the screen `WALK` names.
PULSE_PLAN = ",".join([
    "back:Main Menu", "wait:2", "shot:main",
    "key:cross", "st:Grid Selection", "wait:3", "shot:grid",
    "key:cross", "st:Cell Selection", "wait:3", "shot:cell", "back:Main Menu", "wait:1",
    "key:down", "key:cross", "wait:2", "st:Racebox", "wait:2", "shot:racebox",
    "key:cross", "wait:2", "st:Single Player", "wait:2", "shot:single",
    "key:cross", "wait:3", "st:Track Creation", "wait:2", "shot:track",
    "key:cross", "wait:3", "st:Team Selection", "wait:3", "shot:team", "back:Main Menu", "wait:1",
    "key:down*5", "wait:1", "key:cross", "wait:2", "st:Additional", "wait:2", "shot:options", "back:Main Menu", "wait:1",
])

#: `screen` is the state name the game reports (Pulse) or the name read by eye (Pure); `page` is ours' `--menu-page`, `xml` where ours
#: draws it from, `None` for a gap (ours draws it from `menu.toml`, or not at all).
PULSE_WALK = [
    {"id": "title", "title": "Title: Press Start", "screen": "Show Logo", "page": "show-logo", "xml": "the title screen's own XML (`--until \"Show Logo\" --press cross --ticks 200`: the sequence draws the backdrop `--screen` leaves out)",
     "route": ["--until", "Show Logo", "--press", "cross", "--ticks", "200"],
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
    {"id": "main", "title": "Main Menu", "screen": "Main Menu", "page": "main", "xml": "MainMenu_Definition.xml",
     "why": "ours' page is menu.toml's `main` (ADR-0066: the vanilla rows plus REMIX, RECORDS and MODS, title OPENANTIGRAV)"},
    {"id": "racebox", "title": "Racebox", "screen": "Racebox", "page": "racebox", "xml": "RaceBox_Definition.xml",
     "why": "ours has no page for it: RACEBOX opens menu.toml's `race` page directly"},
    {"id": "single", "title": "Racebox setup (Single Player)", "screen": "Single Player", "page": "race", "xml": "Racebox settings",
     "why": "ours' `race` page is menu.toml's (own rows MODE, SPEED CLASS, WEAPONS, AI DIFFICULTY, KILLS, START, BACK)"},
    {"id": "options", "title": "Options+", "screen": "Additional", "page": "options", "xml": "Additional_Definition.xml",
     "why": "ours' `options` pages are menu.toml's (Display, Graphics, Controls and the rest are ours)"},
]


#: Pure from a cold boot: the language and memory stick screens are answered by pressing cross until the title is up (a boot with no
#: recovered state name), then START, Profile > Load (three crosses: the list, the dialog, the completion), and the Single Player chain.
#: Triangle is Pure's BACK.
PURE_PLAN = ",".join([
    "pu:cross:title", "wait:3", "shot:title", "key:start", "wait:5", "key:cross", "wait:5", "key:cross", "wait:5", "key:cross", "wait:6", "shot:main",
    "key:cross", "wait:4", "shot:mode",
    "key:cross", "wait:4", "shot:sr-class", "key:cross", "wait:5", "shot:sr-league", "key:cross", "wait:5", "shot:sr-track",
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
     "race": {"team": "Feisar"}, "selection": {"original": "FEISAR (first row)", "ours": "FEISAR (first row)"}},
    {"id": "t-class", "title": "Tournament: Class Selection", "screen": "Class Selection", "page": "pure-chain:tournament", "xml": "Selection_Definition.xml Class Selection",
     "selection": {"original": "VECTOR (first row)", "ours": "VECTOR (first row)"}},
    {"id": "t-sel", "title": "Tournament: Tournament Selection (the campaign)", "screen": "Tournament Selection", "page": "pure-chain:tournament:0",
     "xml": "Selection_Definition.xml Tournament Selection",
     "selection": {"original": "ALPHA (the only tournament open), class VECTOR", "ours": "ALPHA (the only tournament open), class VECTOR"}},
    {"id": "main", "title": "Main Menu", "screen": "Main Menu", "page": "main", "xml": "Skin.xml Main Menu",
     "why": "ours' page is menu.toml's `main` (ADR-0066: the vanilla rows plus more, drawn dark teal on the Pulse-style frame); `--screen \"Top FE Screen->FE Screen->Main Menu\"` draws the heading alone on black"},
]


def prepare_profile(plat, profile, title_name, difficulty, race=None):
    """Ours' private profile: the compare settings, `[dev] unlock_everything = false` (a debug build unlocks everything, the original's
    fresh profile does not), `race` (the pickers' cursor: the team the original's fresh list rests on, where it is not ours' default) and
    the campaign difficulty row of `records.toml` (none for a title with no such control), written and read back by the run."""
    ce.write_ours_settings(plat, profile)
    cm.extra_settings(profile, race=race, locked=True)
    if difficulty:
        (ce.settings_path(profile).parent / "records.toml").write_text('[[campaign_difficulty]]\ntitle = "%s"\ndifficulty = "%s"\n' % (title_name.lower(), difficulty))


def build_pairs(args, seen, out, plat, walk):
    """One pair or gap per `WALK` entry from the grabbed frames; returns the ids `Pair.finish` refused."""
    refused = []
    profile = Path(os.environ["XDG_CONFIG_HOME"]).parent
    for item in walk:
        if args.ids and item["id"] not in args.ids.split(","):
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
                            "state": "PPSSPP software renderer, walked by pad from Main Menu on a fresh memory stick"}
        pair.rec["original"]["emulator"] = dict(plat["emulator"])
        pair.rec["original"]["captured_at"] = shot["at"]
        pair.rec["original"]["capture_path"] = "walked by pad from Main Menu (no savestate)"
        pair.rec["original"]["selection_basis"] = "read by eye from the frame; the cursor is where the walk left it"
        if shot.get("since_entry") is not None:
            pair.rec["original"]["seconds_since_entry"] = {"value": round(shot["since_entry"], 3), "basis": "measured: wall clock from the poll that saw the state name change (+-%.2f s; when the first poll already saw the new name, from the midpoint between the press and that poll, wider) to the middle of the frame grab; PPSSPP runs at 60 fps, its pulse period reads 1 s on this clock" % (ENTRY_POLL / 2)}
        pair.rec["original"]["screen_basis"] = ("the game's own state machine (0x08b31784+0x18c): %r" % shot["state"]) if shot["state"] else "read by eye from the frame (no recovered state name for this title)"
        prepare_profile(plat, profile, args.campaign_title, args.difficulty, item.get("race"))
        cp.build(pair, args, "menu", "cold_boot", None, {"value": None, "unit": None, "basis": "not applicable: a menu is a still"})
        try:
            cm.menu_pair(pair, theirs_png=shot["png"], oag_game=args.oag_game, image=args.source, page=item["page"], drawn_from="xml",
                         xml_file=item["xml"], screen_seen=shot["state"] or item["screen"], screen_wanted=item["screen"], profile_dir=profile,
                         size=ce.size_text(plat), front_end_style="the disc's own", language="English", original_language="English",
                         selection=item["selection"], route=item.get("route"),
                         extra_args=("--menu-picker-seconds", "%.3f" % shot["since_entry"]) if item["id"] in CLOCKED and shot.get("since_entry") is not None else (), screen_source="PPSSPP, the front end's state name" if shot["state"] else "read by eye")
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
    cp.add_arguments(ap)
    args = ap.parse_args()
    args.platform = args.platform or args.title + "-psp"
    out = args.out.resolve()
    shots = out / "shots"
    shots.mkdir(parents=True, exist_ok=True)
    rig = Rig(args.port, shots, args.title)
    if args.cmd == "walk":
        rig.run(args.plan)
        return 0
    if not os.environ.get("XDG_CONFIG_HOME", "").startswith(str(ce.MAIN / "data" / "scratch")):
        sys.exit("XDG_CONFIG_HOME must be a directory under data/scratch/<lane>/: oag-game persists settings there")
    plat = ce.platform(args.platform)
    args.source = args.source or str(ce.MAIN / plat["image"])
    args.campaign_title = args.campaign_title or plat["title"]
    rig.run(args.plan or (PULSE_PLAN if args.title == "pulse" else PURE_PLAN))
    refused = build_pairs(args, rig.seen, out, plat, PULSE_WALK if args.title == "pulse" else PURE_WALK)
    json.dump({"refused": refused}, open(out / "refused.json", "w"))
    return 3 if refused else 0


if __name__ == "__main__":
    sys.exit(main())
