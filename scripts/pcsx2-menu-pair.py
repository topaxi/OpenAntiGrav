#!/usr/bin/env python3
"""Original-vs-ours pairs of Wipeout Pulse (PS2) front-end screens from PCSX2 savestates, environment-checked.

    source <scratch>/env.sh    # OAG_PCSX2_DISPLAY, OAG_PCSX2_DATA, OAG_COMPARE_PLATFORM=pulse-ps2, DISPLAY of a `pcsx2-drive.py boot` instance
    python3 scripts/pcsx2-menu-pair.py pairs --slot 28092 --out D --oag-game target/debug/oag-game

The PS2 counterpart of `psp-menu-pair.py`, on `compare_menu.py`'s route: the same screen, size, language and highlighted row, ours
drawn from the disc's own front-end XML (`--menu-page grid-select`, `cell-select`). The original is a PCSX2 savestate of the screen
(`data/saves/pulse-ps2/menu/`, made by the walk below): the state is loaded into slot 1 of the instance's own savestate folder, the
VM runs on, and PCSX2's own GS screenshot is taken and resized ONCE (Lanczos) to the profile's output, as `pcsx2-pulse-pair.py` does.

The walk that makes a state, from `pcsx2-drive.py boot` (each key through `pcsx2-drive.py press`, a settle of 5-8 s between): the
"continue with this screen mode?" prompt (down, cross answers YES), PRESS START (cross), ENTER YOUR NAME (cross to the fourth letter, right to
CONFIRM, cross), ENTER YOUR TAG (cross, cross, right, cross), the format-memory-card prompt (cross on NO) and Main Menu; then cross for Grid
Selection, cross again for Cell Selection (Single Race, Moa Therma White, AI difficulty Medium), `pcsx2-drive.py state save N` on each.

The screen is read by eye from the frame: PS2 Pulse's front-end state name has no recovered address, so `original.screen_basis` says so.
The selector pulses between cyan and white on its own clock and a loaded state keeps the state's phase, so ours is drawn at the pulse's
rest phase; compare placement by gradient (`menu_hex_offsets.py`), never by brightness.
"""

import argparse
import datetime
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402
import compare_menu as cm  # noqa: E402
import compare_provenance as cp  # noqa: E402

_spec = importlib.util.spec_from_file_location("pcsx2_drive", HERE / "pcsx2-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)

SERIAL = "SCES-54748 (F8AE6FF2)"

#: One pair per entry. `state` is the savestate file under `data/saves/pulse-ps2/menu/`; `page` is ours' `--menu-page`.
WALK = [
    {"id": "grid", "title": "Race Campaign: Grid Selection", "screen": "Grid Selection", "page": "grid-select", "state": "grid-selection.p2s",
     "xml": "CellMode_Definition.xml (Grid Selection, the tier hexes)",
     "selection": {"original": "Grid 1, the first tier", "ours": "Grid 1, the first tier"}},
    {"id": "cell", "title": "Race Campaign: Cell Selection (campaign cell)", "screen": "Cell Selection", "page": "cell-select", "state": "cell-selection.p2s",
     "xml": "CellMode_Definition.xml (Cell Selection)",
     "selection": {"original": "Single Race, Moa Therma White, AI difficulty Medium (the footer)", "ours": "Single Race, Moa Therma White, AI difficulty Medium (the footer)"}},
    {"id": "team", "title": "Ship Select", "screen": "Team Selection", "page": "ship-select", "state": "team-selection.p2s",
     "xml": "Selection_Definition.xml (Team Selection)",
     "selection": {"original": "Assegai, Classic (first craft)", "ours": "Assegai, Classic (first craft)"}},
]
#: `--menu-model-seconds` for ours' hull, per WALK id. PS2 Pulse has no recovered address for the ship node's clock (the PSP's is `node+0x40`),
#: and a loaded state keeps its phase while the VM runs on, so the hull's sway phase is matched by eye/box against the frame: pass `--model-seconds`.


def run(cmd, **kw):
    print("+", " ".join(str(c) for c in cmd), file=sys.stderr)
    return subprocess.run([str(c) for c in cmd], check=True, **kw)


def grab(slot, state_file, dest, settle):
    """Load `state_file` into the instance's slot 1 and take PCSX2's GS screenshot of the running frame into `dest`."""
    states = Path(os.environ["OAG_PCSX2_DATA"]).expanduser() / "PCSX2" / "sstates"
    states.mkdir(parents=True, exist_ok=True)
    shutil.copy(state_file, states / ("%s.01.p2s" % SERIAL))
    pine = drive.Pine(slot=slot)
    keyboard = drive.Keyboard()
    drive.set_paused(pine, keyboard, False)
    pine.loadstate(1)
    time.sleep(settle)
    path = drive.gs_screenshot(keyboard, str(dest))
    if not path:
        sys.exit("no GS screenshot (is the VM paused?)")


def prepare_profile(plat, profile):
    """Ours' private profile: the compare settings and `[dev] unlock_everything = false` (the original's fresh profile has the locks)."""
    ce.write_ours_settings(plat, profile)
    cm.extra_settings(profile, locked=True)


def build_pair(args, item, out, plat, profile, raw, captured_at):
    pdir = out / ("menu-" + item["id"])
    pdir.mkdir(parents=True, exist_ok=True)
    pair = ce.Pair(args.platform, pdir)
    pair.rec["meta"] = {"id": "%s-menu-%s" % (args.platform, item["id"]), "title": item["title"], "track": "front end", "craft": "none (a menu)",
                        "note": "Ours is `oag-game --menu-page %s`, drawn from %s. Both sides English, a fresh profile, campaign difficulty Medium." % (item["page"], item["xml"]),
                        "state": "PCSX2 savestate %s, GS screenshot" % item["state"]}
    w, h = plat["output"]
    shutil.copyfile(raw, pdir / "theirs-gs.png")
    size = ce.image_size(pdir / "theirs-gs.png")
    run(["magick", pdir / "theirs-gs.png", "-filter", "Lanczos", "-resize", "%dx%d!" % (w, h), pdir / "resized.png"])
    pair.rec["original"]["emulator"] = dict(plat["emulator"])
    pair.rec["original"]["gs_screenshot_size"] = size
    pair.rec["original"]["resize"] = "one Lanczos resize from %dx%d to %dx%d" % (*size, w, h)
    pair.rec["original"]["captured_at"] = captured_at
    pair.rec["original"]["capture_path"] = "savestate %s loaded, VM running, GS screenshot" % item["state"]
    pair.rec["original"]["selection_basis"] = "read by eye from the frame"
    pair.rec["original"]["screen_basis"] = "read by eye from the frame (no recovered state name for PS2 Pulse)"
    pair.check("GS screenshot aspect equals the output aspect", abs(size[0] / size[1] - w / h) < 0.01, "%dx%d against %dx%d" % (size[0], size[1], w, h))
    prepare_profile(plat, profile)
    cp.build(pair, args, "menu", "savestate_restore", item["state"], {"value": None, "unit": None, "basis": "not applicable: a menu is a still"})
    try:
        cm.menu_pair(pair, theirs_png=pdir / "resized.png", oag_game=args.oag_game, image=args.source, page=item["page"], drawn_from="xml",
                     xml_file=item["xml"], screen_seen=item["screen"], screen_wanted=item["screen"], profile_dir=profile,
                     size=ce.size_text(plat), front_end_style="the disc's own", language="English", original_language="English",
                     selection=item["selection"], screen_source="read by eye",
                     extra_args=("--menu-model-seconds", "%.3f" % args.model_seconds) if item["id"] == "team" and args.model_seconds is not None else ())
        pair.finish()
        (pdir / "resized.png").unlink()
    except SystemExit as exc:
        if exc.code == 3:
            return False
        raise
    return True


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("cmd", choices=("pairs",))
    ap.add_argument("--slot", type=int, required=True, help="the instance's PINE slot")
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--platform", default="pulse-ps2")
    ap.add_argument("--source", default=None, help="the disc image ours reads (default: the platform's image)")
    ap.add_argument("--oag-game", default=str(HERE.parent / "target/debug/oag-game"))
    ap.add_argument("--states", type=Path, default=ce.MAIN / "data" / "saves" / "pulse-ps2" / "menu")
    ap.add_argument("--ids", default="", help="only these WALK ids (comma separated)")
    ap.add_argument("--settle", type=float, default=4.0, help="seconds the VM runs after the state loads, before the grab")
    ap.add_argument("--model-seconds", type=float, default=None, help="ours' ship hull's own animation clock for the `team` pair (sway phase)")
    cp.add_arguments(ap)
    args = ap.parse_args()
    if not os.environ.get("OAG_PCSX2_DATA") or "OAG_PCSX2_DISPLAY" not in os.environ:
        sys.exit("source the lane's own env: OAG_PCSX2_DATA and OAG_PCSX2_DISPLAY must be set")
    if os.environ.get("OAG_COMPARE_PLATFORM") != args.platform:
        sys.exit("start PCSX2 with OAG_COMPARE_PLATFORM=%s so its ini carries the profile's settings" % args.platform)
    if not os.environ.get("XDG_CONFIG_HOME", "").startswith(str(ce.MAIN / "data" / "scratch")):
        sys.exit("XDG_CONFIG_HOME must be a directory under data/scratch/<lane>/: oag-game persists settings there")
    plat = ce.platform(args.platform)
    args.source = args.source or str(ce.MAIN / plat["image"])
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    profile = Path(os.environ["XDG_CONFIG_HOME"]).parent
    refused = []
    for item in WALK:
        if args.ids and item["id"] not in args.ids.split(","):
            continue
        raw = out / ("raw-%s.png" % item["id"])
        grab(args.slot, args.states / item["state"], raw, args.settle)
        at = datetime.datetime.now().astimezone().isoformat(timespec="seconds")
        if not build_pair(args, item, out, plat, profile, raw, at):
            refused.append(item["id"])
        else:
            print("pair", item["id"], "ok")
    json.dump({"refused": refused}, open(out / "refused.json", "w"))
    return 3 if refused else 0


if __name__ == "__main__":
    sys.exit(main())
