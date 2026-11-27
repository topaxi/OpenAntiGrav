#!/usr/bin/env python3
"""Original-vs-ours pairs of Wipeout HD / Fury's front-end screens, one RPCS3 walk, every pair environment-checked.

    source data/scratch/<lane>/env.sh ; export XDG_CACHE_HOME=$PWD/data/scratch/<lane>/cache
    OAG_COMPARE_PLATFORM=hd-fury python3 scripts/rpcs3-drive.py --image data/images/hdfury-ps3-eu.iso serve   # to Main Menu
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/emu-run.py -- python3 scripts/rpcs3-hd-menu-pair.py \\
        --out data/scratch/<lane>/compare-page/hd --oag-game target/release/oag-game

Attached to a fresh cold-booted `serve` sitting on `Main Menu`, this walks `WALK` below: it taps the buttons, photographs
each screen settled at 1920x1080 and, for a pair, renders ours' page with `oag-game --menu-page` under the profile's
settings, then `Pair.finish` writes `pair.json` (exit 3 on a failed check, every mismatch named). A screen ours draws
from `menu.toml` (`drawn_from` `menu.toml`) is a **gap**: the original's frame goes to `<out>/gap-<id>/` with `gap.json`
and no pair is made. Each pair is `<out>/menu-<id>/`; `compare_env.py manifest` turns them into a page manifest.

Which XML ours draws a page from is **declared** per step in `WALK` (read off `crates/game/src/capture/` and
`docs/ui/campaign-screens.md`); it is not discovered at run time. The highlighted row is read by eye from both frames and
declared too. The original's screen name is read from RPCS3's `TTY.log` (`Switching Screen`), so a walk that took a wrong
turn refuses.
"""

import argparse
import datetime
import importlib
import json
import math
import os
import re
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
MAIN = Path("/home/topaxi/projects/OpenAntiGrav")
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402
import compare_menu as cm  # noqa: E402
import compare_provenance as cp  # noqa: E402
import hd_flyer_clock as fc  # noqa: E402
import hd_fury_backdrop as fb  # noqa: E402
import hd_track_hex as hth  # noqa: E402

drive = importlib.import_module("rpcs3-drive")
walk = importlib.import_module("rpcs3-hd-menu-walk")


def prepare_profile(plat, profile, safe_area, front_end_style, race):
    """Ours' private profile: the compare settings, the front-end styling, the race selection and `[dev] unlock_everything = false`
    (a debug build unlocks everything; the walk starts from a profile with its locks, and Ship Select's grid shows them)."""
    ce.write_ours_settings(plat, profile, safe_area)
    cm.extra_settings(profile, front_end_style, race, locked=True)

#: The `WALK` ids whose ours is drawn at the turntable angle the original's RAM held at the grab (`--menu-picker-seconds`, the seconds that
#: turn ours' table that far). `Track Creation`'s update turns the circuit model by a global angle, `*(*(TOC + 0x49ec) + 0x1c4)` with
#: TOC `0x008ad4d8` (`docs/ghidra/functions/ps3-hdfury-eu/track-selection-screen.md`, "The circuit model's update"); it persists across
#: visits to the screen, so the wall time since the tap says nothing about it.
CLOCKED = ("track-select",)
GUEST_BASE = 0x300000000
ANGLE_SLOT = 0x008AD4D8 + 0x49EC
ANGLE_FIELD = 0x1C4
#: Radians the angle moves a frame (`TOC + 0x4b4c`) and frames a second: ours reads `--menu-picker-seconds` at 60 fps.
ANGLE_STEP, FRAMES_PER_SECOND = 0.01, 60.0

#: The `CLOCKED` ids whose grab is taken with the game stopped, the turntable angle and the ring's pulse phase read in the stop
#: (`hd_track_hex.py`): the grab takes 0.1 to 0.7 s, in which both move by tens of frames. The picture is `hth.PICTURE_LAG_FRAMES` older than the read.
STOPPED_GRAB = ("track-select",)

#: The `WALK` ids photographed with the Fury backdrop hidden on both sides: the screens whose page is mostly backdrop. The cloud, its
#: path and how far along it are a new roll on every boot, two paths in three are modes ours does not draw and the music pulses it,
#: so no grab can be reproduced; the row's tint is written to zero in the emulator's RAM before the settle and put back after the grab
#: (`hd_fury_backdrop.py`), and ours is drawn with `--fury-tint 0` and the adapted luma read at the grab (`docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md`, "Hidden
#: for a compare shot").
HIDDEN_BACKDROP = ("track-select", "ship-select", "campaign-select", "grid-select", "cell-select")

#: The `WALK` ids whose ours is drawn at the flyer card's own clock as the original's RAM held it at the grab (`--menu-model-seconds`,
#: `hd_flyer_clock.py`): the card's arrows and layers play a 3 s loop of `Flyer_Item`'s own clock, so a grab's phase is the time of the tap
#: and the boot, nothing a settle fixes.
CARD_CLOCKED = ("grid-select",)
GDB_PORT = int(os.environ.get("OAG_RPCS3_GDB", "127.0.0.1:2345").rsplit(":", 1)[1])

#: The walk. A string is a button (`name` or `name:SETTLE_SECONDS`); a dict is a photograph. `pair` fields: `id`, `title`,
#: `screen` (HD's own name, from TTY.log), `page` (ours' `--menu-page`), `drawn_from` (`xml` or `menu.toml`), `xml` (the
#: disc file that drives ours' page), `selection` (the highlighted row on each side), `race` (the selection ours' picker
#: starts on: team, circuit), `args` (more `oag-game` flags).
WALK = [
    {"id": "main", "title": "Main Menu", "screen": "Main Menu", "page": "main", "drawn_from": "menu.toml",
     "xml": "MainMenu_Definition.xml (rows only: the page itself is menu.toml's `main`)",
     "selection": {"original": "CAMPAIGN", "ours": "RACE CAMPAIGN"}},
    "cross:5",
    {"id": "campaign-select", "title": "Campaign Select", "screen": "Campaign Selection", "page": "campaign-select",
     "drawn_from": "xml", "xml": "DATA06 Campaign Selection screen (two flyer cards)",
     "selection": {"original": "Fury card", "ours": "Fury card"}},
    "cross:5",
    {"id": "grid-select", "title": "Campaign grid", "screen": "Grid Selection Fury", "page": "grid-select",
     "drawn_from": "xml", "xml": "grid_00.xml (Fury grid 1, DATA06 Grid Selection Fury)",
     "selection": {"original": "grid 1, event 01/08 blitzed", "ours": "grid 1, event 01/08 blitzed"}},
    "cross:5",
    {"id": "cell-select", "title": "Campaign cell", "screen": "Cell Selection", "page": "cell-select",
     "drawn_from": "xml", "xml": "CellMode_Definition.xml",
     "selection": {"original": "first cell", "ours": "first cell"}},
    "circle:5", "circle:5", "circle:5",
    "right",
    "cross:5",
    {"id": "racebox", "title": "Racebox setup", "screen": "Single Player", "page": "race", "drawn_from": "menu.toml",
     "xml": "racebox_definition.xml (rows only: the page is menu.toml's `racebox`)",
     "selection": {"original": "RACE TYPE", "ours": "RACE TYPE"}},
    "cross:5",
    {"id": "track-select", "title": "Track Select, Vineta K", "screen": "Track Creation", "page": "track-select",
     "drawn_from": "xml", "xml": "Track_Selection_Definition.xml",
     "selection": {"original": "Vineta K", "ours": "Vineta K"}, "race": {"track": "01_Track"}},
    "cross:6",
    {"id": "ship-select", "title": "Ship Select, Feisar", "screen": "Team Selection", "page": "ship-select",
     "drawn_from": "xml", "xml": "Team_Selection_Definition.xml",
     "selection": {"original": "Feisar", "ours": "Feisar"},
     # the original shows the classic hull (070/100/080/080): `variant_chosen` stops ours opening on concept1, the
     # default of a profile that never picked a model
     "race": {"team": "Feisar", "variant_chosen": True}},
    "circle:5", "circle:5", "circle:5",
    "right", "right",
    "cross:5",
    {"id": "options", "title": "Options", "screen": "Additional", "page": "options", "drawn_from": "menu.toml",
     "xml": "options pages are menu.toml's",
     "selection": {"original": "first row", "ours": "first row"}},
]

#: The loading walk: Main Menu to a Racebox single race on Vineta K with Feisar, then CROSS on Team Selection starts the
#: race load, and the one photograph is taken `settle` seconds after it (the old hand-grabbed frame l3 was 4 s after the
#: CROSS, `Launch Game` then `InGame` on TTY.log; the bar then stood at 32 of its 166 columns, which `compare_menu.loading_pair`
#: counts off every frame and states to ours as a tick count). **The walk is one-way**: the race load ends in a
#: race, so a later menu walk needs a fresh cold boot. The screen name is declared (`TTY.log` names none for this screen),
#: the feature is read from the constructor's `Feature type == N` line.
LOADING_WALK = [
    "cross:5", "cross:5", "cross:5", "circle:5", "circle:5", "circle:5", "right", "cross:5", "cross:5", "cross:6",
    "cross:0.2",
    {"id": "loading", "kind": "loading", "title": "Race loading, Vineta K, Feisar", "screen": "Loading Screen (declared)",
     "settle": 4.0, "mode": "single_race", "track": "Data\\Environments\\01_Vineta_K\\track.vex",
     "race": {"team": "Feisar", "variant_chosen": True, "track": "01_Track"}},
]

FEATURE_LINE = re.compile(r"Feature type == (\d+)")


def feature_read():
    """The feature the last constructed loading screen drew (`Feature type == N`), or None."""
    hits = FEATURE_LINE.findall(drive.tty_text())
    return int(hits[-1]) if hits else None


#: The language of the original: RPCS3's `System: Language` in the private config.
def original_language(config_dir):
    for line in (Path(config_dir) / "config.yml").read_text().splitlines():
        if line.strip().startswith("Language:"):
            return line.split(":", 1)[1].strip()
    return "unknown"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--oag-game", default=str(ROOT / "target/release/oag-game"))
    ap.add_argument("--image", default=str(MAIN / "data/images/hdfury-ps3-eu.iso"))
    ap.add_argument("--platform", default="hd-fury")
    ap.add_argument("--walk", choices=("menus", "loading"), default="menus",
                    help="`menus` walks WALK (one pair per screen); `loading` walks LOADING_WALK to the race load (ends in a race)")
    ap.add_argument("--only", help="comma list of walk ids to pair (the walk still runs through all)")
    ap.add_argument("--settle", type=float, default=3.0, help="seconds after the last tap before a photograph")
    ap.add_argument("--taps-before", action="append", default=[], metavar="ID=BUTTON[,BUTTON...]",
                    help="extra taps on a screen before its photograph, repeatable: for a save whose cursor rests elsewhere than "
                         "WALK declares (`ship-select=down` steps a save that opens on the Fury hull to the classic one)")
    ap.add_argument("--show-backdrop", action="store_true",
                    help="photograph every screen with the Fury backdrop as the boot rolled it (no RAM write, no `--fury-tint`)")
    ap.add_argument("--front-end-style", default="FURY",
                    help="the styling both sides run: HD's red-and-black Fury front end is this profile's (see the docs)")
    cp.add_arguments(ap)
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    only = set(filter(None, (args.only or "").split(",")))
    info = drive.live_session()
    if info is None:
        sys.exit("no live `rpcs3-drive.py serve` (OAG_RPCS3_ATTACH=1, the lane's XDG_CACHE_HOME)")
    pad = drive.RemotePad(info["socket"])
    plat = ce.platform(args.platform)
    w, h = plat["output"]
    walk.window(w, h)
    cfg = Path(os.environ["XDG_CONFIG_HOME"]) / "rpcs3"
    language = original_language(cfg)
    shots = out / "shots"
    shots.mkdir(exist_ok=True)
    seen = {}
    mem = drive.RemoteMem(info["socket"])

    def turntable_angle():
        """The circuit model's global angle, radians, read out of the emulator's RAM."""
        mem.seek(GUEST_BASE + ANGLE_SLOT)
        (obj,) = struct.unpack(">I", mem.read(4))
        mem.seek(GUEST_BASE + obj + ANGLE_FIELD)
        return struct.unpack(">f", mem.read(4))[0]
    taps_before = dict(item.split("=", 1) for item in args.taps_before)
    card_clock = None
    widget = None
    hex_widget = None
    steps = LOADING_WALK if args.walk == "loading" else WALK
    for item in steps:
        if isinstance(item, str):
            button, _, settle = item.partition(":")
            before = drive.current_screen()
            pad.press(button, 0.15)
            time.sleep(float(settle) if settle else 3.0)
            print("%-10s %-22s -> %s" % (item, before, drive.current_screen()), flush=True)
            continue
        for button in filter(None, taps_before.get(item["id"], "").split(",")):
            pad.press(button, 0.15)
            time.sleep(1.0)
        hidden = None
        if item["id"] in HIDDEN_BACKDROP and not args.show_backdrop:
            widget = widget or fb.Widget(mem)
            hidden = widget.hide()
        settled = time.monotonic() + item.get("settle", args.settle)
        if hidden is not None:
            # The settle counted in the widget's own frames too: a slow emulator draws fewer in the same seconds.
            widget.wait_frames(fb.FRAMES_TO_DECAY)
        time.sleep(max(0.0, settled - time.monotonic()))
        if hidden is not None:
            hidden = dict(hidden, still_hidden=widget.still_hidden(), adapted_luma=widget.adapted_luma())
        screen = drive.current_screen()
        png = shots / (item["id"] + ".png")
        # Read just before the grab: the picture the grab returns is within one step of it (a burst of ten grabs against ours' yaw sweep:
        # the angle read before a grab runs 0.04 rad ahead of the frame, the one read after it 0.12).
        angle = turntable_angle() if item["id"] in CLOCKED else None
        card = None
        ring = None
        if item["id"] in STOPPED_GRAB:
            hex_widget = hex_widget or hth.Widget(mem)

            class StoppedClock:
                """What the stopped game holds: the angle and the pulse phase."""
                def seconds(self):
                    state = hex_widget.state()
                    return (turntable_angle(), state["phase"], tuple(sorted(state.items())))
            (angle, phase, state), after = fc.grab_paused(StoppedClock(), lambda: walk.grab(png, w, h), GDB_PORT)
            ring = {"read": phase, "stopped": after == (angle, phase, state), "state": dict(state)}
        elif item["id"] in CARD_CLOCKED:
            card_clock = card_clock or fc.Clock(mem)
            before, after = fc.grab_paused(card_clock, lambda: walk.grab(png, w, h), GDB_PORT)
            card = {"before": before, "after": after, "loop": card_clock.loop()}
        else:
            walk.grab(png, w, h)
        if hidden is not None:
            widget.restore()
        seen[item["id"]] = {"screen": screen, "feature": feature_read(), "png": png, "at": datetime.datetime.now().astimezone().isoformat(timespec="seconds"),
                            "angle": angle, "hidden": hidden, "card": card, "ring": ring}
        print("shot %-16s screen %r" % (item["id"], screen), flush=True)
    refused = []
    for item in (i for i in steps if isinstance(i, dict)):
        if only and item["id"] not in only:
            continue
        shot = seen[item["id"]]
        pdir = out / ("menu-" + item["id"])
        if item.get("kind") == "loading":
            pdir = out / ("loading-" + item["id"])
            pdir.mkdir(parents=True, exist_ok=True)
            pair = ce.Pair(args.platform, pdir)
            pair.rec["meta"] = {"id": "hd-loading", "title": item["title"], "track": "Vineta K", "craft": "Feisar",
                                "note": "Racebox single race, Vineta K, Feisar; ours states the bar as the original's lit-column count (no `--loading-live`), drawn from LoadingScreen_HD.toml. "
                                "Both sides English, front-end styling %s." % args.front_end_style,
                                "state": "cold boot, walked from Main Menu with the pad; photographed %.0f s after CROSS on Team Selection"
                                % item["settle"]}
            pair.rec["original"]["emulator"] = dict(plat["emulator"])
            pair.rec["original"]["captured_at"] = shot["at"]
            pair.rec["original"]["capture_path"] = "cold boot, walked (no savestate)"
            profile = pdir / "ours-profile" / "profile"
            safe_area = ce.record_safe_area(pair)
            prepare_profile(plat, profile, safe_area, args.front_end_style, item.get("race"))
            args.accept_provenance = list(args.accept_provenance)
            cp.build(pair, args, "loading", "cold_boot", None,
                     {"value": None, "unit": None, "basis": "not applicable: a loading screen is a still, the bar's fill is not locked"})
            try:
                cm.loading_pair(pair, theirs_png=shot["png"], oag_game=args.oag_game, image=args.image, profile_dir=profile,
                                size=ce.size_text(plat), front_end_style=args.front_end_style, language="English",
                                original_language=language.split(" ")[0], screen_seen=shot["screen"], screen_wanted=item["screen"],
                                feature_seen=shot["feature"], mode=item["mode"], track=item["track"], safe_area=safe_area)
                pair.finish()
            except SystemExit as exc:
                if exc.code == 3:
                    refused.append(item["id"])
                    continue
                raise
            print("pair %s ok (feature %s)" % (item["id"], shot["feature"]))
            continue
        if cm.is_gap(item["drawn_from"]):
            cm.write_gap(out / ("gap-" + item["id"]), shot["png"], item["page"], item["drawn_from"],
                         "ours draws %s from %s, not the disc's XML: %s"
                         % (item["page"], "this project's `menu.toml` page tree" if item["drawn_from"] == "menu.toml" else "nothing (no such screen)", item["xml"]))
            print("gap  %s (original screen %r)" % (item["id"], shot["screen"]))
            continue
        pdir.mkdir(parents=True, exist_ok=True)
        pair = ce.Pair(args.platform, pdir)
        pair.rec["meta"] = {"id": "hd-menu-" + item["id"], "title": item["title"], "track": "front end",
                            "craft": "none (a menu)",
                            "note": "Ours is `--menu-page %s`, drawn from %s. Both sides English, front-end styling %s." % (
                                item["page"], item["xml"], args.front_end_style),
                            "state": "cold boot, walked from Main Menu with the pad; settled %.0f s after the last tap"
                            % args.settle}
        pair.rec["original"]["emulator"] = dict(plat["emulator"])
        pair.rec["original"]["captured_at"] = shot["at"]
        pair.rec["original"]["capture_path"] = "cold boot, walked (no savestate)"
        profile = pdir / "ours-profile" / "profile"
        safe_area = ce.record_safe_area(pair)
        prepare_profile(plat, profile, safe_area, args.front_end_style, item.get("race"))
        pair.rec["original"]["selection_basis"] = "read by eye from the frame; the cursor is where the walk's taps left it"
        args.accept_provenance = list(args.accept_provenance)
        cp.build(pair, args, "menu", "cold_boot", None,
                 {"value": None, "unit": None, "basis": "not applicable: a menu is a still"})
        extra = list(item.get("args", []))
        if item["id"] in CLOCKED and shot["angle"] is not None:
            # Ours' table starts at zero and its angle is the picture's yaw modulo a turn.
            lag = hth.PICTURE_LAG_FRAMES * ANGLE_STEP if shot["ring"] is not None else 0.0
            since_entry = ((shot["angle"] - lag) % (2 * math.pi)) / (ANGLE_STEP * FRAMES_PER_SECOND)
            extra += ["--menu-picker-seconds", "%.3f" % since_entry]
            pair.rec["original"]["turntable_angle"] = {
                "radians": round(shot["angle"], 4),
                "ours_seconds": round(since_entry, 3),
                "basis": "measured: the circuit model's global angle read out of the emulator's RAM %s the grab (`*(*(0x008ad4d8 + 0x49ec) + 0x1c4)`, "
                         "the `Track Creation` update's own RotY, stepped 0.01 rad a frame)%s; ours is drawn at the seconds that turn its own table that far "
                         "at 60 frames a second" % (
                             "in the stop of" if shot["ring"] is not None else "just before",
                             ", less the picture's %.1f frames of lag (a forward circuit)" % hth.PICTURE_LAG_FRAMES if shot["ring"] is not None else "")}
        if shot.get("ring") is not None:
            ring = shot["ring"]
            shown = hth.shown_phase(ring["read"])
            extra += ["--menu-ring-seconds", "%.4f" % (shown / 3.0)]
            pair.check("the game stood still through the grab", ring["stopped"], "angle and ring phase read in the stop agree with the read after the grab")
            pair.rec["original"]["ring_phase"] = {
                "read": round(ring["read"], 4),
                "shown": round(shown, 4),
                "ours_seconds": round(shown / 3.0, 4),
                "picture_lag_frames": hth.PICTURE_LAG_FRAMES,
                "widget": {key: ring["state"][key] for key in ("parity", "selected", "row", "built", "slide_timer")},
                "basis": "measured: the `TrackHexSelection` widget's pulse counter (`+0xe0`, found by its vtable word `0x00868090`, `hd_track_hex.py`) read out of the "
                         "emulator's RAM with the game stopped, less the picture's lag (ten stopped grabs: the ring's colour ran 0.118 to 0.228 of a phase "
                         "behind the read, mean 0.176). The counter is the widget's own update count since the boot's first visit and is never reset, so "
                         "ours is pinned to it (`--menu-ring-seconds`)"}
        if shot.get("card") is not None:
            extra += fc.record(pair.rec, shot["card"]["before"], shot["card"]["after"], shot["card"]["loop"])
            pair.check("the card's clock stood still through the grab", shot["card"]["after"] == shot["card"]["before"],
                       "read %.4f before the grab, %.4f after it" % (shot["card"]["before"], shot["card"]["after"]))
            pair.rec["meta"]["note"] += " The flyer card is drawn at the clock the original's RAM held at the grab (`--menu-model-seconds`)."
        hidden = shot.get("hidden")
        if hidden is not None:
            pair.check("the Fury backdrop is hidden at the grab", hidden["still_hidden"] and hidden["frames_waited"] >= fb.FRAMES_TO_DECAY,
                       "row %s zero at the grab: %s, %d widget frames since the write (%d let the trail decay)"
                       % (hidden["row"], hidden["still_hidden"], hidden["frames_waited"], fb.FRAMES_TO_DECAY))
            extra += fb.record(pair.rec, hidden, hidden["adapted_luma"])
            pair.rec["meta"]["note"] += " The Fury backdrop is hidden on both sides (the original's row tint written to zero for the grab, ours `--fury-tint 0 --fury-adapted-luma` at the luma read at the grab)."
        taps = taps_before.get(item["id"])
        if taps:
            pair.rec["meta"]["state"] += "; then %s on this screen (--taps-before)" % taps
        try:
            cm.menu_pair(pair, theirs_png=shot["png"], oag_game=args.oag_game, image=args.image, page=item["page"],
                         drawn_from=item["drawn_from"], xml_file=item["xml"], screen_seen=shot["screen"],
                         screen_wanted=item["screen"], profile_dir=profile, size=ce.size_text(plat),
                         front_end_style=args.front_end_style, language="English", original_language=language.split(" ")[0],
                         selection=item["selection"], extra_args=extra,
                         race=item.get("race"), safe_area=safe_area)
            pair.finish()
        except SystemExit as exc:
            if exc.code == 3:
                refused.append(item["id"])
                continue
            raise
        print("pair %s ok" % item["id"])
    json.dump({"refused": refused}, open(out / "refused.json", "w"))
    return 3 if refused else 0


if __name__ == "__main__":
    sys.exit(main())
