#!/usr/bin/env python3
"""Original-vs-ours pairs of Wipeout HD / Fury's front-end screens, one RPCS3 walk, every pair environment-checked.

    source data/scratch/<lane>/env.sh ; export XDG_CACHE_HOME=/tmp/oag-<lane>-cache
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
import os
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

drive = importlib.import_module("rpcs3-drive")
walk = importlib.import_module("rpcs3-hd-menu-walk")

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
     "selection": {"original": "Vineta K", "ours": "Vineta K"}, "race": {"track": "01_Vineta_K"}},
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
    ap.add_argument("--only", help="comma list of walk ids to pair (the walk still runs through all)")
    ap.add_argument("--settle", type=float, default=3.0, help="seconds after the last tap before a photograph")
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
    for item in WALK:
        if isinstance(item, str):
            button, _, settle = item.partition(":")
            before = drive.current_screen()
            pad.press(button, 0.15)
            time.sleep(float(settle) if settle else 3.0)
            print("%-10s %-22s -> %s" % (item, before, drive.current_screen()), flush=True)
            continue
        time.sleep(args.settle)
        screen = drive.current_screen()
        png = shots / (item["id"] + ".png")
        walk.grab(png, w, h)
        seen[item["id"]] = {"screen": screen, "png": png, "at": datetime.datetime.now().astimezone().isoformat(timespec="seconds")}
        print("shot %-16s screen %r" % (item["id"], screen), flush=True)
    refused = []
    for item in (i for i in WALK if isinstance(i, dict)):
        if only and item["id"] not in only:
            continue
        shot = seen[item["id"]]
        pdir = out / ("menu-" + item["id"])
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
        ce.write_ours_settings(plat, profile, safe_area)
        cm.extra_settings(profile, args.front_end_style, item.get("race"))
        pair.rec["original"]["selection_basis"] = "read by eye from the frame; the cursor is where the walk's taps left it"
        args.accept_provenance = list(args.accept_provenance)
        cp.build(pair, args, "menu", "cold_boot", None,
                 {"value": None, "unit": None, "basis": "not applicable: a menu is a still"})
        try:
            cm.menu_pair(pair, theirs_png=shot["png"], oag_game=args.oag_game, image=args.image, page=item["page"],
                         drawn_from=item["drawn_from"], xml_file=item["xml"], screen_seen=shot["screen"],
                         screen_wanted=item["screen"], profile_dir=profile, size=ce.size_text(plat),
                         front_end_style=args.front_end_style, language="English", original_language=language.split(" ")[0],
                         selection=item["selection"], extra_args=item.get("args", []),
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
