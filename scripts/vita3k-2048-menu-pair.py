#!/usr/bin/env python3
"""Original-vs-ours pairs of Wipeout 2048's front-end screens (Vita3K), environment-checked.

    source <scratch>/env.sh    # HOME, OAG_VITA3K_CACHE/CONFIG/DISPLAY, ours' XDG profile (see vita-2048-pair.py)
    uv run --with evdev python3 scripts/emu-run.py --child-status <log-dir>/status.json -- \\
        python3 scripts/vita3k-2048-menu-pair.py walk --out <dir> --plan "wait:20,shot:a,key:Return,wait:4,shot:b"   # explore
    ... pairs --out <dir> --oag-game target/release/oag-game                                                       # `WALK`

The 2048 counterpart of `rpcs3-hd-menu-pair.py` and `shadps4-omega-menu-pair.py`, on `vita-2048-pair.py`'s Session (Vita3K on its
own weston and Xwayland, xdotool input) and `compare_menu.py`'s route. Vita3K has no screen log: the screen a frame shows is read by
eye and declared in `WALK`, and the record says so (`original.screen_basis`). 2048's front end is a touch front end (ADR-0054):
ours draws it through `--screen`, `--until` (the sequence, with `--press`) and not `--menu-page`, so each `WALK` entry carries
`route`, the oag-game arguments that draw it, and the pages ours draws from `menu.toml` or not at all are **gaps**.
"""

import argparse
import datetime
import importlib.util
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


def load_pair_module():
    spec = importlib.util.spec_from_file_location("vita_2048_pair", HERE / "vita-2048-pair.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


vp = load_pair_module()


def boot(s, args):
    """Weston and Xwayland, Vita3K on the private config, its game window; no hull patch (a menu has no hull)."""
    d = s.drive
    d.bring_up_display(args.display, args.socket)
    pid = d.vita3k_alive()
    if pid is None:
        pid = d.launch_vita3k(vp.TITLE, args.display)
    elif not args.attach:
        sys.exit("a Vita3K of this cache is already up (pid %d): --attach to drive it, or `stop`" % pid)
    s.window(args.boot_timeout)
    return pid


def run_plan(s, plan, shots):
    """Run `plan`: 'wait:S', 'key:NAME', 'tap:XxY', 'shot:NAME'. Returns `{name: {png, at}}`."""
    seen = {}
    for item in filter(None, plan.split(",")):
        kind, _, rest = item.partition(":")
        if kind == "wait":
            time.sleep(float(rest))
        elif kind == "key":
            s.key(rest)
        elif kind == "pu":
            key, _, pred = rest.partition(":")
            press_until(s, key, PREDICATES[pred], shots)
        elif kind == "tap":
            x, y = rest.split("x")
            s.tap((int(x), int(y)))
        elif kind == "shot":
            png = s.shot(rest + ".png")
            dst = shots / (rest + ".png")
            dst.write_bytes(Path(png).read_bytes())
            seen[rest] = {"png": dst, "at": datetime.datetime.now().astimezone().isoformat(timespec="seconds")}
        else:
            sys.exit("unknown plan step %r" % item)
        print("step", item, flush=True)
    return seen


def mean_luma(path):
    return vp.mean_luma(path)


#: Screen predicates on a grabbed frame, by mean luma (Vita3K has no screen log; the intro movie is dark to mid, every menu is light, a
#: dialog dims the menu under it). The walk checks its frames by eye afterwards; these only keep it from running ahead of the game.
def logo_up(frame):
    """The title screen: a light frame with the navy WipEout 2048 logo across the middle (the intro movie and the loading screens have none)."""
    return mean_luma(frame) >= 0.85 and sum(1 for v in vp.crop_bytes(frame, "560x150+200+140") if v < 110) > 20000


PREDICATES = {"light": lambda f: mean_luma(f) >= 0.7, "title": logo_up, "dark": lambda f: mean_luma(f) < 0.55, "hud": vp.hud_up}


def press_until(s, key, predicate, shots, timeout=90.0):
    """Press `key` every 4 s until the grabbed frame satisfies `predicate`: the first presses are swallowed while the game is
    still compiling shaders at boot, so a fixed 'wait, press' walk runs a screen behind (seen 2026-10-27, both boots)."""
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        frame = s.shot("pu.png")
        if predicate(frame):
            return
        if key.startswith("tap@"):
            x, y = key[4:].split("x")
            s.tap((int(x), int(y)))
        else:
            s.key(key)
        time.sleep(4.0)
    sys.exit("STALLED pressing %s (%.0f s); last frame %s" % (key, timeout, s.frames / "pu.png"))


#: The walk's shared buttons, in the window's own 960x544 coordinates. `CHECK` is the bottom-right check mark (confirm, and back on Home's
#: children), `HOME` the map's house, the five Home tiles sit on one row at y 256.
CHECK = "882x480"
HOME = "38x38"
HOME_TILES = {"team": "156x256", "community": "318x256", "profile": "480x256", "options": "642x256", "extras": "804x256"}
#: Ours' way into a screen: `--until STATE --press BUTTONS` (`--press` is one mask, see docs/formats/2048-frontend.md), the buttons that
#: leave the intro (start) and walk the fresh profile's Game Mode, tutorial and map.
TO_MAP = "start,cross,cross,cross"
TO_HOME = "start,cross,cross,triangle,cross"
EVENT = "2048 - Event 1"

#: The original's cold-boot walk, one plan, shots named by `WALK` ids. The original is a FRESH save (no campaign progress), ours a fresh profile.
PLAN = ",".join([
    "wait:8", "pu:Return:title", "wait:6", "shot:title",
    "key:x", "wait:3", "shot:psn", "pu:x:light", "wait:4", "shot:game-mode",
    "tap:295x162", "wait:4", "shot:sp-tutorial", "tap:" + CHECK, "wait:14", "shot:map",
    "tap:490x270", "wait:3", "shot:card-0", "tap:940x240", "wait:3", "shot:card-1", "tap:752x480", "wait:3",
    "tap:" + HOME, "wait:4", "shot:home",
    "tap:" + HOME_TILES["team"], "wait:4", "shot:team", "tap:" + CHECK, "wait:3",
    "tap:" + HOME_TILES["options"], "wait:4", "shot:options", "tap:" + CHECK, "wait:3",
    "tap:" + HOME_TILES["profile"], "wait:4", "shot:profile", "tap:" + CHECK, "wait:3",
    "tap:" + HOME_TILES["extras"], "wait:4", "shot:extras",
])

#: The same walk on to the race and its pause menu, for the one screen of the front end ours draws from `menu.toml` and the original
#: shows only in a race: tap Play twice, wait out the (light) loading screen, tap the flyby's check mark until the HUD is up, pause.
PAUSE_PLAN = PLAN.split(",tap:940x240")[0] + ",tap:" + CHECK + ",wait:3,tap:" + CHECK + ",wait:10,pu:tap@" + CHECK + ":dark,pu:tap@" + CHECK + ":hud,wait:2,key:Return,wait:2,shot:pause"

#: One entry per screen the walk photographs. `screen` is read by eye off the frame (Vita3K has no screen log) and is the name the
#: XML gives it; `route` is ours' oag-game arguments, `None` where ours has no XML-driven still (a gap, with `why`).
WALK = [
    {"id": "title", "title": "Title screen", "screen": "TitleScreen", "route": ["--until", "TitleScreen", "--press", "start"],
     "xml": "Title_Definition.xml / Skin.xml TitleScreen",
     "selection": {"original": "PRESS ANY BUTTON TO START, settled", "ours": "entry tick of TitleScreen", "matched": False,
                   "why": "ours stops on the tick TitleScreen is entered (`--ticks` after `--until` does not advance it), before the logo and the prompt appear"}},
    {"id": "psn", "title": "PSN prompt", "screen": "Boot Connect", "route": None, "xml": "the Vita system's own dialog",
     "why": "the dialog is the system's, not a screen of the game's XML"},
    {"id": "game-mode", "title": "Game Mode", "screen": "GameModeChoice", "route": ["--until", "GameModeChoice", "--press", TO_MAP],
     "xml": "Definition.xml GameModeChoice",
     "selection": {"original": "no tile highlighted (fresh entry)", "ours": "Single Player Campaign ringed", "matched": False,
                   "why": "ours rings Single Player Campaign on entry; the original rings nothing until a tile is touched"}},
    {"id": "sp-tutorial", "title": "Campaign welcome panel", "screen": "SPTutorial", "route": None, "xml": "newFEshell->SPTutorial",
     "why": "ours draws SPTutorial only through `--screen`, on black without the shell, and its live flow never opens it"},
    {"id": "map", "title": "Campaign map", "screen": "newFEshell", "route": ["--until", "map:" + EVENT, "--press", TO_MAP],
     "xml": "SP.xml events on newFEshell", "selection": {"original": "Empire Climb, the first event (TOUCH TO START)", "ours": "Empire Climb, the first event (TOUCH TO START)"}},
    {"id": "card-0", "title": "Event card, page 0", "screen": "newFEshell", "route": ["--until", "card:0:" + EVENT, "--press", TO_MAP],
     "xml": "SP.xml / newFEshell event card", "selection": {"original": "Empire Climb, page 0", "ours": "Empire Climb, page 0"}},
    {"id": "card-1", "title": "Event card, page 1", "screen": "newFEshell", "route": ["--until", "card:1:" + EVENT, "--press", TO_MAP],
     "xml": "SP.xml / newFEshell event card", "selection": {"original": "Empire Climb, page 1 (leaderboard)", "ours": "Empire Climb, page 1 (leaderboard)"}},
    {"id": "home", "title": "Home", "screen": "Home", "route": ["--until", "Home", "--press", TO_HOME, "--ticks", "40"],
     "xml": "Definition.xml Home", "selection": {"original": "no tile highlighted", "ours": "first tile highlighted", "matched": False,
                                               "why": "ours leaves the pad cursor on a tile"}},
    {"id": "team", "title": "Select Team", "screen": "team", "route": ["--until", "team", "--press", TO_HOME, "--ticks", "40"],
     "xml": "Team_Definition.xml", "selection": {"original": "Feisar Speed, top-left tile of the 5x4 grid", "ours": "AG Systems and Speed ringed in two button rows", "matched": False,
                   "why": "ours draws the team and craft as two rows of labelled buttons (a chosen stand-in, docs/formats/2048-frontend.md) beside the skin list, not the original's grid"}},
    {"id": "options", "title": "Options", "screen": "options", "route": None, "xml": "Options_Definition.xml",
     "why": "ours has no headless still route to `options`: `--press` is one mask, so Home's fourth tile cannot be chosen, `--screen` draws only the header on black, and a windowed `--script` run reaches Home's tiles but in a 1440x816 window at llvmpipe speed, not a 960x544 frame (tried 2026-10-27)"},
    {"id": "profile", "title": "Profile", "screen": "profile", "route": None, "xml": "Profile_Definition.xml",
     "why": "the same: no headless still route past Home's first tile"},
    {"id": "extras", "title": "Extras", "screen": "2048extras", "route": None, "xml": "Extras_Definition_EU.xml",
     "why": "the same: no headless still route past Home's first tile"},
]


def build_pairs(args, seen, out, plat):
    """One pair or gap per `WALK` entry from the grabbed frames; returns the ids `Pair.finish` refused."""
    refused = []
    profile = Path(os.environ["XDG_CONFIG_HOME"]).parent
    for item in WALK:
        shot = seen[item["id"]]
        if item["route"] is None:
            cm.write_gap(out / ("gap-" + item["id"]), shot["png"], item["screen"], "none",
                         "ours draws no XML-driven still of %s: %s (%s)" % (item["screen"], item["why"], item["xml"]))
            print("gap", item["id"])
            continue
        pdir = out / ("menu-" + item["id"])
        pdir.mkdir(parents=True, exist_ok=True)
        pair = ce.Pair(args.platform, pdir)
        pair.rec["meta"] = {"id": "2048-menu-" + item["id"], "title": item["title"], "track": "front end", "craft": "none (a menu)",
                            "note": "Ours is `oag-game %s`, drawn from %s. Both sides English, a fresh campaign save." % (" ".join(item["route"]), item["xml"]),
                            "state": "cold boot on a fresh Vita3K save, walked from the intro with keys and touch taps"}
        pair.rec["original"]["emulator"] = dict(plat["emulator"])
        pair.rec["original"]["captured_at"] = shot["at"]
        pair.rec["original"]["capture_path"] = "cold boot, walked (no savestate)"
        pair.rec["original"]["selection_basis"] = "read by eye from the frame; the cursor is where the walk's taps left it"
        pair.rec["original"]["screen_basis"] = "read by eye from the frame (Vita3K has no screen log)"
        ce.write_ours_settings(plat, profile)
        cp.build(pair, args, "menu", "cold_boot", None, {"value": None, "unit": None, "basis": "not applicable: a menu is a still"})
        sel = item["selection"]
        try:
            cm.menu_pair(pair, theirs_png=shot["png"], oag_game=args.oag_game, image=args.source, page=item["id"], drawn_from="xml",
                         xml_file=item["xml"], screen_seen=item["screen"], screen_wanted=item["screen"], profile_dir=profile,
                         size=ce.size_text(plat), front_end_style="the disc's own", language="English", original_language="English",
                         selection=sel, route=item["route"], screen_source="read by eye (Vita3K has no screen log)")
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
    ap.add_argument("cmd", choices=("walk", "pairs", "stop"))
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--plan", default="")
    ap.add_argument("--display", type=int, default=int(os.environ.get("OAG_VITA3K_DISPLAY", 0)) or None)
    ap.add_argument("--socket", default=None)
    ap.add_argument("--boot-timeout", type=float, default=90.0)
    ap.add_argument("--attach", action="store_true", help="drive the Vita3K already up (explore without a reboot)")
    ap.add_argument("--keep", action="store_true", help="leave Vita3K running after the walk")
    ap.add_argument("--pause-png", type=Path, help="pairs: the original's pause menu frame (a `walk` with PAUSE_PLAN), kept as a gap")
    ap.add_argument("--platform", default="2048")
    ap.add_argument("--source", default=str(vp.ROOT / vp.SOURCE))
    ap.add_argument("--oag-game", default=str(vp.ROOT / "target/release/oag-game"))
    cp.add_arguments(ap)
    args = ap.parse_args()
    if args.display is None or "OAG_VITA3K_CACHE" not in os.environ:
        sys.exit("OAG_VITA3K_DISPLAY and OAG_VITA3K_CACHE must name this session's own display and cache (see vita-2048-pair.py)")
    if not os.environ.get("XDG_CONFIG_HOME", "").startswith(str(ce.MAIN / "data" / "scratch")):
        sys.exit("XDG_CONFIG_HOME must be a directory under data/scratch/<lane>/: oag-game persists settings there")
    args.socket = args.socket or "wayland-oag%d" % args.display
    out = args.out.resolve()
    shots = out / "shots"
    shots.mkdir(parents=True, exist_ok=True)
    s = vp.Session(args.display, out / "frames")
    if args.cmd == "stop":
        s.drive.cmd_stop(args)
        return 0
    boot(s, args)
    try:
        seen = run_plan(s, args.plan if args.cmd == "walk" else (args.plan or PLAN), shots)
    finally:
        if not args.keep:
            s.drive.cmd_stop(args)
    if args.cmd == "walk":
        return 0
    plat = ce.platform(args.platform)
    refused = build_pairs(args, seen, out, plat)
    if args.pause_png:
        cm.write_gap(out / "gap-pause", args.pause_png, "pause menu", "menu.toml",
                     "ours draws the pause menu as a `Stage::Menu` of menu.toml's pages over the held frame and has no still route; the original "
                     "(a race walked from this save's first event, Return pressed on the HUD) shows GAME PAUSED with Options and Photo Mode tiles "
                     "beside the event card and quit, retry and resume buttons (ingame Definition XML)")
    json.dump({"refused": refused}, open(out / "refused.json", "w"))
    return 3 if refused else 0


if __name__ == "__main__":
    sys.exit(main())
