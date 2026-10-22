#!/usr/bin/env python3
"""One original-vs-ours pair of Wipeout Omega Collection (PS4) on the start grid before GO, environment-checked.

    # once: the game folder shadPS4 boots (hard links, nothing copied; needs the .pkg for sce_sys)
    python3 scripts/shadps4-omega-pair.py prepare-game --game data/scratch/<lane>/omega-game

    # per pair: boots its own sway + shadPS4 + one virtual pad, walks the menus, grabs, renders ours, writes pair.json
    uv run --with evdev python3 scripts/shadps4-omega-pair.py pair \\
        --scratch data/scratch/<lane>/omega --game data/scratch/<lane>/omega-game \\
        --out data/scratch/<lane>/compare-page/omega/vineta-k \\
        --track 'Data\\Environments\\01_vineta_k\\track.vex' --track-steps 0

Run it under `scripts/emu-run.py --child-status <log-dir>/status.json -- ...` (member-rules.md, "Waiting on an emulator capture").

The original runs on shadPS4 0.20 inside a headless `sway` on the render node (Xvfb cannot present, docs/networking/online-lobby.md
part 3), with its own `HOME`, a muted SDL audio driver and one uinput pad (`scripts/uinput-pads.py`) that no other instance owns
unless another member runs pads on the same host: the pad is system-wide, so a second member's pads would also drive this game.

Two shadPS4 settings carry the picture (2026-10-22): `readback_linear_images_enabled = true` with `readbacks_mode = 1` makes the
race scene draw (with them off the 3D world is black and only the HUD and the lens flare draw); `copy_gpu_buffers = true` crashes
the emulator at the race load. `full_screen` with a 1920x1080 internal resolution and RCAS off gives the profile's 1920x1080 frame
with no window chrome and no sharpening.

The walk (MAIN MENU -> a Single Race) is RACEBOX, then the Race Box's own defaults on a fresh profile (WipEout HD mode, Single Race,
Venom class, one player), a confirm into Track Select (`--track-steps` presses of Right choose the circuit; the first is Vineta K),
a confirm into Ship Select (a fresh profile has most craft locked: two Downs reach the first Feisar craft), a confirm, the loading
screen, and START RACE. The grid is then on screen with the countdown board blank; the grab is the first frames after the HUD shows
and before the board's red "3". There is no pose readout: the pose is the spawn, and ours spawns the same way.
"""

import argparse
import json
import os
import shutil
import signal
import struct
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402
import compare_provenance as cp  # noqa: E402

#: The shadPS4 config keys the capture sets; everything else is the emulator's default (docs/tools/compare-page.md).
GPU_CONFIG = {
    "internal_screen_width": 1920, "internal_screen_height": 1080, "window_width": 1920, "window_height": 1080,
    "full_screen": True, "full_screen_mode": "Fullscreen", "rcas_enabled": False,
    "readbacks_mode": 1, "readback_linear_images_enabled": True,
}
#: The Omega craft the walk picks (first unlocked Feisar craft) and ours' team id for it.
DEFAULT_TEAM = "FEISAR_C1"
#: How many frames after the HUD first shows the grab is taken (the countdown board is still blank for about five).
GRAB_AFTER_HUD = 2
#: The hull's region of the 960x540 half-size frame (x, y, w, h) and the search, in half-size px.
CRAFT_REGION = (280, 250, 400, 290)
CRAFT_SEARCH = 14
#: Pixel probes in 1920x1080 frame coordinates.
MENU_RED = (260, 170)            # MAIN MENU's selected CAMPAIGN tab
HUD_REGION = "140x150+60+50"     # the lap hexagon: yellow once the HUD is up
BOARD_REGION = "90x40+800+258"   # where the countdown board's first digit lights
TITLE_BAR_COLOUR = (44, 85, 125)  # shadPS4's window title bar, absent in full screen


def log(msg):
    print("[omega-pair] " + msg, file=sys.stderr, flush=True)


def run(cmd, **kw):
    log("+ " + " ".join(str(c) for c in cmd))
    return subprocess.run([str(c) for c in cmd], check=True, **kw)


def pkg_entries(pkg):
    """`{name: bytes}` of the PS4 package's plaintext `sce_sys` entries (ids 0x1000 to 0x12ff): `param.sfo`, `playgo-chunk.dat`
    and the pictures. They are not in the extracted tree, and without `playgo-chunk.dat` the game parks after `scePlayGoOpen`."""
    out = {}
    with open(pkg, "rb") as f:
        head = f.read(0x20)
        count, table = struct.unpack(">I", head[0x10:0x14])[0], struct.unpack(">I", head[0x18:0x1C])[0]
        f.seek(table)
        raw = f.read(count * 32)
        ents = [struct.unpack(">IIIIII", raw[i * 32:i * 32 + 24]) for i in range(count)]
        names = b""
        for e in ents:
            if e[0] == 0x200:
                f.seek(e[4])
                names = f.read(e[5])
        for e in ents:
            if 0x1000 <= e[0] < 0x1300:
                f.seek(e[4])
                out[names[e[1]:names.index(b"\0", e[1])].decode()] = f.read(e[5])
    return out


def prepare_game(args):
    """The game folder: the base package's extraction hard-linked, the patch's files over it, `sce_sys` from the base `.pkg`.
    Hard links, because a symlink that leaves the folder is resolved inside the guest and the executable "does not exist"."""
    game = Path(args.game)
    game.mkdir(parents=True, exist_ok=True)
    for tree in ("omega-eu", "omega-eu-patch"):
        run(["cp", "-alf", "%s/." % (ce.MAIN / "data/extracted/ps4" / tree / "uroot"), game])
    sys_dir = game / "sce_sys"
    sys_dir.mkdir(exist_ok=True)
    for name, data in pkg_entries(ce.MAIN / "data/images/omega-ps4-eu.pkg").items():
        (sys_dir / name).write_bytes(data)
    log("game folder %s: %d archives, sce_sys %d files" % (game, len(list(game.glob("data*.psarc"))), len(list(sys_dir.iterdir()))))


class Rig:
    """The processes this run started, by PID, and everything it needs to drive them."""

    def __init__(self, scratch, game):
        self.dir = Path(scratch).resolve()
        self.game = Path(game).resolve()
        self.dir.mkdir(parents=True, exist_ok=True)
        self.runtime = Path("/tmp/oag-omega-%s" % self.dir.parent.name[:24])
        self.pids = {}
        self.home = self.dir / "home"
        self.env = {**os.environ, "XDG_RUNTIME_DIR": str(self.runtime), "WAYLAND_DISPLAY": "wayland-1"}
        self.pad = None

    def spawn(self, name, cmd, **kw):
        p = subprocess.Popen([str(c) for c in cmd], start_new_session=True, **kw)
        self.pids[name] = p.pid
        (self.dir / "pids.json").write_text(json.dumps(self.pids))
        log("started %s pid %d" % (name, p.pid))
        return p

    def start_sway(self):
        self.runtime.mkdir(mode=0o700, exist_ok=True)
        conf = self.dir / "sway.conf"
        conf.write_text("output HEADLESS-1 resolution 1920x1080\n")
        env = {k: v for k, v in self.env.items() if k not in ("DISPLAY", "WAYLAND_DISPLAY")}
        env.update(WLR_BACKENDS="headless", WLR_RENDERER="vulkan", WLR_RENDER_DRM_DEVICE="/dev/dri/renderD128",
                   WLR_LIBINPUT_NO_DEVICES="1")
        self.spawn("sway", ["sway", "-c", conf, "--unsupported-gpu"], env=env, stdout=open(self.dir / "sway.log", "w"),
                   stderr=subprocess.STDOUT)
        for _ in range(40):
            if (self.runtime / "wayland-1").exists():
                return
            time.sleep(0.25)
        sys.exit("sway made no wayland-1 socket in %s" % self.runtime)

    def write_config(self):
        cfg_dir = self.home / ".local/share/shadPS4"
        cfg_dir.mkdir(parents=True, exist_ok=True)
        cfg = cfg_dir / "config.json"
        doc = json.loads(cfg.read_text()) if cfg.exists() else {}
        doc.setdefault("GPU", {}).update(GPU_CONFIG)
        doc.setdefault("General", {}).setdefault("console_language", 1)
        cfg.write_text(json.dumps(doc, indent=2))
        # a zenity shim: shadPS4 asks (a dialog) to migrate saves from the real profile; "Do nothing" answers it
        shim = self.dir / "bin"
        shim.mkdir(exist_ok=True)
        (shim / "zenity").write_text('#!/bin/sh\ncase "$*" in *Save*) echo "Do nothing";; *Config*) echo "Defaults";; esac\nexit 1\n')
        (shim / "zenity").chmod(0o755)
        return doc

    def start_shadps4(self):
        h = str(self.home)
        env = {k: v for k, v in self.env.items() if k != "DISPLAY"}
        env.update(SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS="1", SDL_VIDEODRIVER="wayland", SDL_AUDIODRIVER="dummy", HOME=h,
                   XDG_DATA_HOME=h + "/.local/share", XDG_CONFIG_HOME=h + "/.config", XDG_CACHE_HOME=h + "/.cache",
                   PATH="%s:%s" % (self.dir / "bin", os.environ["PATH"]))
        self.spawn("shadps4", ["timeout", "1500", "shadps4", "-g", self.game / "eboot.bin"], env=env,
                   stdout=open(self.dir / "shadps4.log", "w"), stderr=subprocess.STDOUT)

    def start_pad(self):
        fifo = self.dir / "pad.cmd"
        if fifo.exists():
            fifo.unlink()
        os.mkfifo(fifo)
        self.hold = open(fifo, "r+b", buffering=0)  # keeps the fifo open for the pad's reader
        env = {**os.environ, "OAG_PADS": "1", "UV_CACHE_DIR": os.path.expanduser("~/.cache/uv"), "UV_OFFLINE": "1"}
        self.spawn("pad", ["uv", "run", "--with", "evdev", "python3", HERE / "uinput-pads.py"], env=env,
                   stdin=open(fifo, "rb"), stdout=open(self.dir / "pad.log", "w"), stderr=subprocess.STDOUT)
        self.fifo = fifo

    def press(self, button, wait=0.9):
        self.hold.write(b"a press %s\n" % button.encode())
        time.sleep(wait)

    def grab(self, path):
        subprocess.run(["grim", str(path)], check=True, env=self.env)
        return path

    def stop(self):
        """By PID, children first; a pad's `uv` leaves the python child behind unless its process group goes."""
        for name in ("pad", "shadps4", "sway"):
            pid = self.pids.get(name)
            if pid:
                try:
                    os.killpg(pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
        time.sleep(2)
        for pid in self.pids.values():
            try:
                os.killpg(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        log("stopped pids %s" % self.pids)


def pixel(path, x, y):
    fx = ",".join("%[fx:int(255*p{" + "%d,%d" % (x, y) + "}." + c + ")]" for c in "rgb")
    out = subprocess.run(["magick", str(path), "-format", fx, "info:"], capture_output=True, text=True, check=True).stdout
    return tuple(int(v) for v in out.split(","))


def region_mean(path, geometry, channel_fx):
    """Mean of an `fx` expression over a crop of the frame."""
    return float(subprocess.run(["magick", str(path), "-crop", geometry, "+repage", "-format", "%[fx:" + channel_fx + "]", "info:"],
                                capture_output=True, text=True, check=True).stdout)


def hud_up(path):
    """The lap hexagon's yellow ring: red and green over blue across the crop (0.05 up, under 0 before the HUD)."""
    return region_mean(path, HUD_REGION, "(mean.r+mean.g)/2-mean.b") > 0.03


def board_red(path):
    """The countdown board's first digit: red over blue across `BOARD_REGION` (a blank board reads -0.19, the red 3 from +0.02)."""
    return region_mean(path, BOARD_REGION, "mean.r-mean.b")


def edge_frame(path):
    return subprocess.run(["magick", str(path), "-resize", "960x540", "-colorspace", "Gray", "-blur", "0x1.2", "-edge", "1", "-depth", "8",
                           "gray:-"], capture_output=True, check=True).stdout


def craft_offset(theirs, ours):
    """Where the original's hull sits against ours', in half-size px: the best normalised cross-correlation of the two frames' edge
    maps over `CRAFT_REGION`. `dx`, `dy`: how far the original's picture is shifted right and down of ours'; (0, 0) is the same place."""
    a, b = edge_frame(ours), edge_frame(theirs)
    x, y, w, h = CRAFT_REGION

    def patch(buf, dx, dy):
        return [v for j in range(h) for v in buf[(y + dy + j) * 960 + x + dx:(y + dy + j) * 960 + x + dx + w]]

    def centred(v):
        m = sum(v) / len(v)
        return [e - m for e in v]

    def ncc(p, q):
        den = (sum(e * e for e in p) * sum(e * e for e in q)) ** 0.5
        return sum(e * f for e, f in zip(p, q)) / den if den else 0.0

    ref = centred(patch(a, 0, 0))
    best = (-2.0, 0, 0)
    for dy in range(-CRAFT_SEARCH, CRAFT_SEARCH + 1):
        for dx in range(-CRAFT_SEARCH, CRAFT_SEARCH + 1):
            s = ncc(ref, centred(patch(b, dx, dy)))
            if s > best[0]:
                best = (s, dx, dy)
    return {"dx": best[1], "dy": best[2], "ncc_at_best": round(best[0], 4), "ncc_at_zero": round(ncc(ref, centred(patch(b, 0, 0))), 4),
            "unit": "px at 960x540 (the 1920x1080 frame's half)"}


def capture_original(rig, args, out):
    """Boot, walk to the grid, grab. Returns the chosen frame's path and what was seen."""
    rig.write_config()
    rig.start_sway()
    rig.start_shadps4()
    rig.start_pad()
    probe = rig.dir / "probe.png"
    for waited in range(0, 150, 3):
        time.sleep(3)
        if (rig.dir / "shadps4.log").read_text(errors="replace").count("dumped core"):
            sys.exit("STALLED at boot: shadPS4 dumped core")
        try:
            rig.grab(probe)
            r, g, b = pixel(probe, *MENU_RED)
        except subprocess.CalledProcessError:
            continue
        if r > 200 and g < 40 and b < 40:
            log("MAIN MENU after about %d s" % (waited + 3))
            break
    else:
        sys.exit("STALLED at boot: no MAIN MENU in 150 s (see shadps4.log)")
    rig.press("right")
    rig.press("cross", 3)
    rig.press("cross", 3)  # Race Box -> Track Select
    for _ in range(args.track_steps):
        rig.press("right", 1.5)
    rig.grab(out / "track-select.png")
    rig.press("cross", 3)  # Track Select -> Ship Select
    rig.press("down")
    rig.press("down")
    rig.grab(out / "ship-select.png")
    rig.press("cross", 3)  # Ship Select -> loading
    time.sleep(12)
    rig.grab(out / "loading.png")
    time.sleep(14)  # the START RACE prompt waits for the confirm
    rig.press("cross", 0.2)
    frames, hud_at, t0 = [], None, time.time()
    while time.time() - t0 < 20:
        path = rig.grab(out / ("frame%02d.png" % len(frames)))
        frames.append(path)
        if hud_at is None and hud_up(path):
            hud_at = len(frames) - 1
        if hud_at is not None and len(frames) > hud_at + GRAB_AFTER_HUD + 3:
            break
        time.sleep(0.3)
    if hud_at is None:
        sys.exit("STALLED at grid: the HUD never showed in %d frames (see frame*.png)" % len(frames))
    pick = frames[hud_at + GRAB_AFTER_HUD]
    return pick, {"frames": len(frames), "hud_first_frame": hud_at, "picked_frame": hud_at + GRAB_AFTER_HUD,
                  "frames_seen": [str(f.name) for f in frames], "board_red": [round(board_red(f), 3) for f in frames[hud_at:]]}


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    pg = sub.add_parser("prepare-game")
    pg.add_argument("--game", required=True)
    pr = sub.add_parser("pair")
    pr.add_argument("--scratch", required=True, help="the instance's own directory under data/scratch/<lane>/ (HOME, logs, pids)")
    pr.add_argument("--game", required=True)
    pr.add_argument("--out", type=Path, required=True)
    pr.add_argument("--track", required=True, help="ours' track .vex entry name")
    pr.add_argument("--track-steps", type=int, default=0, help="Right presses on Track Select from Vineta K")
    pr.add_argument("--team", default=DEFAULT_TEAM)
    pr.add_argument("--nose-offset", help="DX,DY of the original's hull nose tip against ours', read by eye (the edge metric is unreliable)")
    pr.add_argument("--ours-ticks", type=int, default=100, help="a tick inside the countdown (the grid is at rest throughout)")
    pr.add_argument("--oag-game", default=str(ce.MAIN / "target/debug/oag-game"))
    pr.add_argument("--platform", default="omega")
    pr.add_argument("--meta", default="", help="JSON object for the manifest: id, title, track, craft, note")
    cp.add_arguments(pr)
    args = ap.parse_args()
    if args.cmd == "prepare-game":
        return prepare_game(args)
    if not os.environ.get("XDG_CONFIG_HOME", "").startswith(str(ce.MAIN / "data" / "scratch")):
        sys.exit("XDG_CONFIG_HOME must be a directory under data/scratch/<lane>/: oag-game persists settings there")
    pair = ce.Pair(args.platform, args.out)
    plat = pair.plat
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    pair.out = out
    pair.rec["original"]["emulator"] = {**plat["emulator"], "gpu_config": GPU_CONFIG}
    pair.rec["ours"]["binary"] = ce.binary_record(args.oag_game)
    rig = Rig(args.scratch, args.game)
    try:
        shot, seen = capture_original(rig, args, out)
    finally:
        rig.stop()
    shutil.copy(shot, out / "theirs.png")
    pair.stamp("original")
    pair.rec["original"]["grid_capture"] = seen
    pair.rec["original"]["state"] = {"circuit": args.track, "mode": "single_race (Race Box, WipEout HD mode)", "class": "venom",
                                     "team": args.team, "source": "a cold boot walked from MAIN MENU on a fresh profile"}
    cp.build(pair, args, "scenery", "cold_boot", None,
             {"value": 0, "unit": "ticks", "basis": "none needed: the grid before GO is a still (craft at rest, camera static), "
                                                    "no pose readout exists on this title's emulator", "what": "picture"})
    pair.check_size("original", out / "theirs.png")
    r0, g0, b0 = TITLE_BAR_COLOUR
    fx = "abs(mean.r*255-%d)+abs(mean.g*255-%d)+abs(mean.b*255-%d)" % (r0, g0, b0)
    bar = float(subprocess.run(["magick", str(out / "theirs.png"), "-crop", "400x20+0+0", "+repage", "-format", "%[fx:" + fx + "]", "info:"],
                               capture_output=True, text=True, check=True).stdout)
    pair.check("no emulator window chrome over the original's frame", bar > 20, "title-bar colour distance %.0f" % bar)
    red = board_red(out / "theirs.png")
    pair.check("the countdown board is still blank (before its red 3)", red < -0.1, "board red minus blue %.3f (blank -0.19, a lit digit above 0)" % red)
    profile_dir = Path(os.environ["XDG_CONFIG_HOME"]).parent
    ce.write_ours_settings(plat, profile_dir)
    source = ce.MAIN / "data/extracted/ps4"
    flags = ce.ours_flags(plat) + ["--mode", "single_race", "--class", "venom", "--team", args.team]
    base = [args.oag_game, source, "--race", "--track", args.track, "--ticks", args.ours_ticks] + flags
    pair.rec["ours"]["flags"] = [str(c) for c in base[2:]]
    pair.rec["ours"].update({"mode": "single_race", "class": "venom", "team": args.team, "ticks": args.ours_ticks,
                             "camera": "the title's default chase view; the grid is at rest, no pose is fed"})
    env = {**os.environ, **ce.ours_env(profile_dir)}
    run(base + ["--screenshot", out / "ours.png"], stdout=subprocess.DEVNULL, env=env)
    got, bad = ce.read_back(plat, profile_dir)
    pair.rec["ours"]["settings_readback"] = got
    for b in bad:
        pair.check("ours settings", False, b)
    pair.check("ours settings read back", not bad, "%d keys equal the profile" % (len(got) - len(bad)))
    pair.check_size("ours", out / "ours.png")
    off = craft_offset(out / "theirs.png", out / "ours.png")
    off["note"] = ("informational: the hulls are lit and coloured differently (the original's scenery lighting is not read), so edge "
                   "correlation stays under 0.1 and its peak is not a measurement")
    if args.nose_offset:
        dx, dy = (int(v) for v in args.nose_offset.split(","))
        off["nose_tip_by_eye"] = {"dx": dx, "dy": dy, "unit": "px at 1920x1080, original minus ours", "basis": "read off 2x crops of both frames"}
        pair.check("ours' hull is on the original's (nose tip, by eye)", abs(dx) <= 8 and abs(dy) <= 8,
                   "original is (%+d, %+d) px of ours at 1920x1080" % (dx, dy))
    pair.rec["craft_offset_px"] = off
    still = [out / n for n in seen["frames_seen"][seen["hud_first_frame"]:seen["picked_frame"] + 4] if (out / n).exists()]
    diffs = []
    for f in still[1:]:
        r = subprocess.run(["compare", "-metric", "RMSE", "-crop", "700x500+610+520", str(still[0]), str(f), "null:"], capture_output=True, text=True)
        diffs.append(float(r.stderr.split("(")[1].split(")")[0]))
    pair.rec["original"]["hull_shake_on_grid"] = {"frames": len(still), "hull_region_rmse_vs_first": diffs,
                                                  "note": "normalised RMSE over the hull's 700x500 region; 0 is a still hull"}
    pair.caveat("ours' gantry board shows the grid position 8 and never changes; the original's is blank before the red 3 and counts 3, 2, 1, GO")
    pair.caveat("scenery lighting is not read on this title: the original's grid is lit differently (sun, shadows, sky)")
    run(["magick", "montage", out / "theirs.png", out / "ours.png", "-tile", "2x1", "-geometry", "+2+2", out / "side.png"])
    if args.meta:
        pair.rec["meta"] = json.loads(args.meta)
    pair.finish()
    print("wrote", out / "side.png")


if __name__ == "__main__":
    main()
