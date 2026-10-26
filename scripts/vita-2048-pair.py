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
        d.launch_vita3k(TITLE, args.display)
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


def restart_and_burst(s, count, tag):
    """Pause, Retry, and grab `count` frames back to back; returns `[(t, path)]`, t seconds from the confirm tap."""
    s.key("Return")
    time.sleep(1.5)
    s.tap(PAUSE_RETRY)
    time.sleep(1.2)
    s.tap(RETRY_YES)
    t0 = time.monotonic()
    out = []
    for i in range(count):
        path = s.frames / ("%s%03d.png" % (tag, i))
        s.drive.screenshot(s.display, s.wid, path)
        out.append((time.monotonic() - t0, path))
    return out


def pick_grid_frame(burst, offset):
    """The first frame with the HUD up is tick 0 of the countdown; the frame `offset` seconds later is the grid.
    Returns `(index_of_hud, index_picked, t_hud, t_picked)` or None."""
    hud = None
    for i, (t, path) in enumerate(burst):
        if hud_up(path):
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

    s = Session(args.display, out / "frames")
    if args.walk:
        boot_and_walk(s, args)
    else:
        s.window(10.0)
    burst = restart_and_burst(s, args.burst, "burst")
    picked = pick_grid_frame(burst, args.grid_seconds)
    pair.check("the countdown's HUD came up after Retry", picked is not None,
               "first HUD frame at %s" % (("%.2f s" % picked[2]) if picked else "none of %d frames" % len(burst)))
    if picked is None:
        pair.finish()
    hud_i, pick_i, t_hud, t_pick = picked
    frame = burst[pick_i][1]
    (out / "theirs.png").write_bytes(Path(frame).read_bytes())
    pair.rec["original"]["emulator"] = dict(plat["emulator"])
    pair.rec["original"]["grab"] = {"hud_first_frame_s": t_hud, "grid_frame_s": t_pick, "burst_frames": len(burst),
                                    "burst_interval_s": (burst[-1][0] - burst[0][0]) / (len(burst) - 1),
                                    "grid_offset_s": t_pick - t_hud}
    # the grid is at rest: the frames after the camera settles (0.5 s after the HUD) agree to the animated sign boards
    settled = [p for t, p in burst[hud_i:] if 0.6 <= t - t_hud <= t_pick - t_hud + 0.01]
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
    which = ["--track", args.track, "--mode", args.mode] if args.track else ["--event", args.event]
    which += ["--team", args.team] if args.team else []
    base = [args.oag_game, args.source, "--race"] + which + ["--ticks", ticks] + ce.ours_flags(plat)
    pair.rec["ours"]["flags"] = [str(c) for c in base[2:]]
    env = {**os.environ, **ce.ours_env(profile_dir)}
    run(base + ["--screenshot", out / "ours.png"], stdout=subprocess.DEVNULL, env=env)
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
