#!/usr/bin/env python3
"""Make the three Wipeout 2048 race-load comparison pairs re-renderable: `pair.json` for each, then ours drawn.

    python3 scripts/vita-2048-race-load-pair.py [--pairs DIR] [--theirs DIR] [--oag-game BIN]

The originals (`theirs.png`, Vita3K frames of a campaign event's race load, `docs/formats/2048-loading.md`) are
captured by `scripts/vita-2048-race-load.py` and kept in `--theirs` (default `data/reference/compare-page/2048`, entries
`race-load-tip|mode|late`). For each, this writes `<pairs>/2048/race-load-<phase>/` holding a copy of `theirs.png` and a
`pair.json` (`kind = "loading"`, the exact `oag-game` flags, the profile's own settings) and renders ours with
`compare-rerender-ours.py`'s `rerender`, so the page's re-render path finds the pair afterwards. `--pairs` must not be
the shared `data/reference/compare-page` from a worktree (`docs/tools/compare-page.md`, "Re-rendering ours").

Ours is `--loading-screen 0/1 --loading-step race --loading-card '2048 - Event 2' --loading-stage S --ticks T`: `T` is the
screen's age at 60 Hz, `S` the load stage (4 turns the card to its mode page). Each phase's `T` is the tick whose meter
reads the original's percentage.
"""
import argparse
import importlib.util
import json
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402

#: `(directory, title, percent, stage, ticks)`: `ticks` is the first age at which the meter reads `percent` (the card's
#: tip page is stage 0, its mode page stage 4; past about 3,000 ticks the meter holds 98).
PHASES = (
    ("race-load-tip", "Race load, game tip phase (8 %)", 8, 0, 232),
    ("race-load-mode", "Race load, mode page (61 %)", 61, 4, 1780),
    ("race-load-late", "Race load, held at 98 %", 98, 4, 6000),
)
CARD = "2048 - Event 2"
NOTE = ("Ours draws no blurred campaign map behind the card (a launch has none in this build), the header's second line is "
        "the front end's own card text (RACE, where the original reads OFFENSIVE WEAPONS ONLY), the tip is a seeded draw "
        "(the original's was the missile tip), and the meter's breathing phase differs.")


def load_rerender():
    spec = importlib.util.spec_from_file_location("compare_rerender_ours", HERE / "compare-rerender-ours.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def pair_record(plat, title, stage, ticks):
    flags = ce.ours_flags(plat) + ["--loading-screen", "0/1", "--loading-step", "race", "--loading-card", CARD,
                                   "--loading-stage", str(stage), "--ticks", str(ticks)]
    return {
        "profile": "2048", "profile_sha": plat["profile_sha"], "output": list(plat["output"]), "kind": "loading",
        "original": {
            "emulator": {"name": "Vita3K", "capture": "vita-2048-race-load.py"},
            "capture_path": "walked from a fresh save to the event card's Play tap (no savestate)",
            "provenance": {
                "schema": 1, "pair_kind": "loading", "basis": "recorded by the pair script",
                "capture_path": {"kind": "cold_boot", "state_file": None},
                "presented_lag": {"value": None, "unit": None, "basis": "not applicable: a loading screen is a still"},
                "autopilot": {"armed": False, "timer_written": None, "timer_authored": None, "source": "no race running"},
                "ram_writes": [], "pair_script_ram_writes": [],
                "emulator": {"name": "Vita3K", "version": None, "settings_sha": None},
                "twin": {"agreed": None, "rmse_vs_previous": None, "basis": "no second capture of this pair"},
                "overrides": [],
            },
            "screen": {"seen": "Loading", "wanted": "Loading", "source": "read by eye from the frame (Vita3K has no screen log)"},
            "language": "English", "front_end_style": "the disc's own",
        },
        "ours": {"language": "English", "front_end_style": "the disc's own", "flags": flags},
        "meta": {"id": None, "title": title, "track": "Queens Mall (%s)" % CARD, "craft": "none (a menu screen)",
                 "state": "the load screen at the same phase and percentage; the original is Vita3K, a fresh frame of the maintainer's save",
                 "note": NOTE},
        "caveats": [], "refused": False,
        "argv": ["vita-2048-race-load-pair.py"],
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--pairs", type=Path, default=ROOT / "data/reference/compare-page")
    ap.add_argument("--theirs", type=Path, default=ROOT / "data/reference/compare-page/2048")
    ap.add_argument("--oag-game", type=Path, default=ROOT / "target/debug/oag-game")
    args = ap.parse_args()
    rr = load_rerender()
    plat = ce.platform("2048")
    bad = 0
    for name, title, percent, stage, ticks in PHASES:
        out = args.pairs / "2048" / name
        out.mkdir(parents=True, exist_ok=True)
        if (args.theirs / name / "theirs.png").resolve() != (out / "theirs.png").resolve():
            shutil.copyfile(args.theirs / name / "theirs.png", out / "theirs.png")
        rec = pair_record(plat, title, stage, ticks)
        rec["meta"]["id"] = "2048-" + name
        pj = out / "pair.json"
        pj.write_text(json.dumps(rec, indent=2) + "\n")
        result = rr.rerender(pj, args.oag_game)
        print("%-24s %s" % (name, result))
        bad += not result.startswith("ok")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
