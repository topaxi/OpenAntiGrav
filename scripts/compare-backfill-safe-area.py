#!/usr/bin/env python3
"""Backfill what a pair's original ran with, for pairs made before the pair scripts recorded it.

    python3 scripts/compare-backfill-safe-area.py DIR [--apply]      # dry run unless --apply

Every pair under DIR whose profile pins a `hud_safe_area` (HD/Fury) and records no `original.safe_area` gets
`{"percent": 96, "source": "RAM read 2026-10-21 (docs/formats/hd-hud.md)"}`: all the HD captures then on disk came from
one save, whose `Safe Area Setting` was read live as 96 (and, 2026-10-27, off nine distinct private saves' `DATA.BIN`).
A cold-booted HD front-end pair (`kind` menu) also gets the `ours.race` selection its picker was drawn with, read off the
walk in `rpcs3-hd-menu-pair.py`: `compare-rerender-ours.py` puts `ours.front_end_style` and `ours.race` back, and a pair
without them draws the title's first team (Assegai) and circuit instead of the one the original shows.
"""
import argparse
import importlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402

SOURCE = "RAM read 2026-10-21 (docs/formats/hd-hud.md)"


def walk_races():
    """`{walk id: race dict}` of `rpcs3-hd-menu-pair.py`'s WALK (the module needs the emulator drivers importable)."""
    sys.argv = sys.argv[:1]
    mod = importlib.import_module("rpcs3-hd-menu-pair")
    return {i["id"]: i["race"] for i in mod.WALK if isinstance(i, dict) and i.get("race")}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("dir", type=Path)
    ap.add_argument("--apply", action="store_true")
    args = ap.parse_args()
    races = walk_races()
    changed = 0
    for pj in sorted(args.dir.rglob("pair.json")):
        rec = json.loads(pj.read_text())
        if "hud_safe_area" not in ce.platform(rec["profile"])["ours"]:
            continue
        todo = []
        if ce.recorded_safe_area(rec) is None:
            rec["original"]["safe_area"] = {"percent": 96, "source": SOURCE}
            todo.append("original.safe_area = 96")
        wid = (rec.get("meta") or {}).get("id", "")
        race = races.get(wid.removeprefix("hd-menu-")) if rec.get("kind") == "menu" else None
        if race and rec["ours"].get("race") != race:
            rec["ours"]["race"] = race
            todo.append("ours.race = %s" % race)
        if not todo:
            continue
        changed += 1
        print("%s: %s" % (pj, "; ".join(todo)))
        if args.apply:
            rec["environment"] = ce.environment_text(ce.platform(rec["profile"]), rec)
            pj.write_text(json.dumps(rec, indent=2) + "\n")
    print("%d pair(s) %s" % (changed, "updated" if args.apply else "would change (dry run)"))


if __name__ == "__main__":
    main()
