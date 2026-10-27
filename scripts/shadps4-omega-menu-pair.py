#!/usr/bin/env python3
"""Original-vs-ours pairs of Wipeout Omega Collection's front-end screens (shadPS4), environment-checked.

    # once: the game folder (hard links) - see scripts/shadps4-omega-pair.py
    python3 scripts/shadps4-omega-pair.py prepare-game --game data/scratch/<lane>/omega-game

    export XDG_CONFIG_HOME=data/scratch/<lane>/omega-profile/cfg ...        # ours' own profile
    uv run --with evdev python3 scripts/emu-run.py --child-status <log-dir>/status.json -- \\
        python3 scripts/shadps4-omega-menu-pair.py walk --scratch data/scratch/<lane>/omega --game data/scratch/<lane>/omega-game \\
        --out data/scratch/<lane>/compare-page/omega --plan "shot:main,cross:3,shot:next"      # explore: photographs only
    ... pairs  --scratch ... --game ... --out ... --oag-game target/release/oag-game        # the WALK below, pairs and gaps

Boots shadPS4 in its own headless `sway` with one uinput pad (the machinery of `shadps4-omega-pair.py`, whose `Rig` this
imports), waits for MAIN MENU, then taps the plan. shadPS4 has no `TTY.log`: the screen a frame shows is **read by eye** from
its SCREEN TITLE and declared in `WALK` (`screen`), so the pair's screen check compares two declarations, not a log line, and
the record says so (`original.screen.source`). A page ours draws from `menu.toml` is a gap, as in `rpcs3-hd-menu-pair.py`.
"""

import argparse
import datetime
import importlib
import json
import os
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402
import compare_menu as cm  # noqa: E402
import compare_provenance as cp  # noqa: E402

op = importlib.import_module("shadps4-omega-pair")

#: See `rpcs3-hd-menu-pair.py`'s `WALK`; the same fields, with `screen` the title read off the frame.
WALK = [
    {"id": "main", "title": "Main Menu", "screen": "MAIN MENU", "page": "main", "drawn_from": "menu.toml",
     "xml": "MainMenu_Definition.xml (rows only: the page itself is menu.toml's `main`)",
     "selection": {"original": "CAMPAIGN", "ours": "RACE CAMPAIGN"}, "plan": []},
    {"id": "campaign-select", "title": "Campaign Select", "screen": "CAMPAIGN SELECT", "page": "campaign-select", "drawn_from": "none",
     "xml": "ours: `--menu-page campaign-select` refuses on Omega, no Campaign Selection screen read (DATA06's CellMode_Definition.xml copy)",
     "selection": {"original": "2048 card", "ours": "-"}, "plan": ["cross:4"]},
    {"id": "grid-select", "title": "Campaign grid (Fury)", "screen": "CAMPAIGN", "page": "grid-select", "drawn_from": "xml",
     "xml": "grid_00.xml (Fury grid 1, Omega's DATA09 copy)",
     "selection": {"original": "Fury campaign grid 1 (HD's 21 points, the blitzed flyer card)", "ours": "Omega's own first grid (18 points, no flyer card)",
                   "matched": False, "why": "ours' `grid-select` and `grid-select-hd` both draw Omega's own list on this source and ours has no selector for the Fury or HD campaign"},
     "plan": ["right:2", "right:2", "cross:4"]},
    {"id": "cell-select", "title": "Campaign cell (Fury)", "screen": "CAMPAIGN", "page": "cell-select", "drawn_from": "xml",
     "xml": "CellMode_Definition.xml",
     "selection": {"original": "Fury grid 1, first cell, TALON'S JUNCTION", "ours": "Omega's own first grid, first cell, `01_TRACK` (name unresolved)",
                   "matched": False, "why": "the same as the grid page: ours has no selector for the original's campaign"}, "plan": ["cross:4"]},
    {"id": "racebox", "title": "Racebox setup", "screen": "RACEBOX", "page": "race", "drawn_from": "menu.toml",
     "xml": "racebox_definition.xml (rows only: the page is menu.toml's `race`)",
     "selection": {"original": "GAME MODE", "ours": "GAME MODE"}, "plan": ["circle:3", "circle:3", "circle:3", "right:1.5", "cross:4"]},
    {"id": "track-select", "title": "Track Select, Vineta K", "screen": "TRACK SELECT", "page": "track-select", "drawn_from": "xml",
     "xml": "Track_Selection_Definition.xml", "selection": {"original": "Vineta K", "ours": "Vineta K"},
     "race": {"track": "01_vineta_k"}, "plan": ["cross:4"]},
    {"id": "ship-select", "title": "Ship Select, Feisar", "screen": "SHIP SELECT", "page": "ship-select", "drawn_from": "xml",
     "xml": "Team_Selection_Definition.xml",
     "selection": {"original": "Feisar, third row (the first unlocked craft)", "ours": "Feisar, top row", "matched": False,
                   "why": "ours has no way to put the cursor on the craft the original's fresh profile unlocks (a release build's locks follow the profile)"},
     "race": {"team": "FEISAR_C1"}, "plan": ["cross:4", "down:1.5", "down:1.5"]},
    {"id": "options", "title": "Options", "screen": "OPTIONS", "page": "options", "drawn_from": "menu.toml",
     "xml": "options pages are menu.toml's", "selection": {"original": "first row", "ours": "first row"},
     "plan": ["circle:3", "circle:3", "circle:3", "right:1.5", "right:1.5", "cross:4"]},
]


def boot(rig, args):
    """Start sway, shadPS4 and the pad; return when MAIN MENU is up (the CAMPAIGN tab is red)."""
    rig.write_config()
    rig.start_sway()
    rig.start_shadps4()
    rig.start_pad()
    probe = rig.dir / "probe.png"
    for waited in range(0, 180, 3):
        time.sleep(3)
        if (rig.dir / "shadps4.log").read_text(errors="replace").count("dumped core"):
            sys.exit("STALLED at boot: shadPS4 dumped core")
        try:
            rig.grab(probe)
            r, g, b = op.pixel(probe, *op.MENU_RED)
        except Exception:
            continue
        if r > 200 and g < 40 and b < 40:
            op.log("MAIN MENU after about %d s" % (waited + 3))
            return
    sys.exit("STALLED at boot: no MAIN MENU in 180 s (see shadps4.log)")


def run_plan(rig, plan, shots, w, h):
    """Tap `plan` ('button[:settle]', 'wait:S', 'shot:NAME'); returns `{name: {png, at}}`."""
    seen = {}
    for item in filter(None, plan.split(",")):
        kind, _, rest = item.partition(":")
        if kind == "shot":
            time.sleep(1.5)
            png = shots / (rest + ".png")
            rig.grab(png)
            seen[rest] = {"png": png, "at": datetime.datetime.now().astimezone().isoformat(timespec="seconds")}
        elif kind == "wait":
            time.sleep(float(rest))
        else:
            rig.press(kind, float(rest) if rest else 2.0)
        print("step", item, flush=True)
    return seen


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("cmd", choices=("walk", "pairs"))
    ap.add_argument("--scratch", required=True)
    ap.add_argument("--game", required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--plan", default="", help="walk: the plan; pairs: defaults to the plan WALK_PLAN derives")
    ap.add_argument("--oag-game", default=str(Path(__file__).resolve().parent.parent / "target/release/oag-game"))
    ap.add_argument("--platform", default="omega")
    ap.add_argument("--front-end-style", default="")
    cp.add_arguments(ap)
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    shots = out / "shots"
    shots.mkdir(exist_ok=True)
    plat = ce.platform(args.platform)
    w, h = plat["output"]
    rig = op.Rig(args.scratch, args.game)
    try:
        boot(rig, args)
        if args.cmd == "walk":
            run_plan(rig, args.plan, shots, w, h)
            return 0
        plan = args.plan or ",".join(",".join(i["plan"] + ["shot:" + i["id"]]) for i in WALK)
        seen = run_plan(rig, plan, shots, w, h)
    finally:
        rig.stop()
    refused = []
    for item in (i for i in WALK if isinstance(i, dict)):
        shot = seen[item["id"]]
        pdir = out / ("menu-" + item["id"])
        if cm.is_gap(item["drawn_from"]):
            cm.write_gap(out / ("gap-" + item["id"]), shot["png"], item["page"], item["drawn_from"],
                         "ours draws %s from %s, not the disc's XML: %s" % (item["page"], "this project's `menu.toml` page tree" if item["drawn_from"] == "menu.toml" else "nothing (no such screen)", item["xml"]))
            print("gap", item["id"])
            continue
        pdir.mkdir(parents=True, exist_ok=True)
        pair = ce.Pair(args.platform, pdir)
        pair.rec["meta"] = {"id": "omega-menu-" + item["id"], "title": item["title"], "track": "front end", "craft": "none (a menu)",
                            "note": "Ours is `--menu-page %s`, drawn from %s. Both sides English, Omega's own white front end." % (item["page"], item["xml"]),
                            "state": "cold boot on a fresh shadPS4 profile, walked from MAIN MENU with the pad"}
        pair.rec["original"]["emulator"] = {**plat["emulator"], "gpu_config": op.GPU_CONFIG}
        pair.rec["original"]["captured_at"] = shot["at"]
        pair.rec["original"]["capture_path"] = "cold boot, walked (no savestate)"
        pair.rec["original"]["selection_basis"] = "read by eye from the frame; the cursor is where the walk's taps left it"
        pair.rec["original"]["screen_basis"] = "read by eye from the frame's SCREEN TITLE (shadPS4 has no TTY.log)"
        profile = Path(os.environ["XDG_CONFIG_HOME"]).parent
        ce.write_ours_settings(plat, profile)
        cm.extra_settings(profile, args.front_end_style or None, item.get("race"))
        cp.build(pair, args, "menu", "cold_boot", None, {"value": None, "unit": None, "basis": "not applicable: a menu is a still"})
        try:
            cm.menu_pair(pair, theirs_png=shot["png"], oag_game=args.oag_game, image=ce.MAIN / "data/extracted/ps4", page=item["page"],
                         drawn_from=item["drawn_from"], xml_file=item["xml"], screen_seen=item["screen"], screen_wanted=item["screen"],
                         profile_dir=profile, size=ce.size_text(plat), front_end_style=args.front_end_style or "the disc's own",
                         language="English", original_language="English", selection=item["selection"], extra_args=item.get("args", []), race=item.get("race"))
            pair.finish()
        except SystemExit as exc:
            if exc.code == 3:
                refused.append(item["id"])
                continue
            raise
        print("pair", item["id"], "ok")
    json.dump({"refused": refused}, open(out / "refused.json", "w"))
    return 3 if refused else 0


if __name__ == "__main__":
    sys.exit(main())
