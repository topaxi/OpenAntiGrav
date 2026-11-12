#!/usr/bin/env python3
"""One original-vs-ours pair of Wipeout Omega Collection (PS4) on the start grid before GO, the original being the
`wipeout_pc` native port, environment-checked.

    # once: the game folder (hard links, nothing copied; needs the .pkg for sce_sys, which the port also reads)
    python3 scripts/wipeout-pc-omega-pair.py prepare-game --game data/scratch/<lane>/game

    # per pair: its own headless `sway`, the port on a fresh save, one virtual pad, muted; walks the menus, grabs, renders ours
    uv run --with evdev python3 scripts/wipeout-pc-omega-pair.py pair \\
        --scratch data/scratch/<lane>/port --game data/scratch/<lane>/game \\
        --out data/scratch/<lane>/pairs/vineta-k \\
        --track 'Data\\Environments\\01_vineta_k\\track.vex' --track-steps 0 --circuit 'VINETA K' \\
        --oag-game data/scratch/<lane>/oag-game-bin

Run it under `scripts/emu-run.py --child-status <log-dir>/status.json -- ...` (member-rules.md, "Waiting on an emulator capture").
`docs/tools/compare-page.md`, "Omega", records what the port is and how it differs from shadPS4.

The walk is `scripts/shadps4-omega-pair.py`'s (MAIN MENU, RACEBOX, Track Select with `--track-steps` presses of Right, Ship Select
with two Downs to the first Feisar craft, loading, START RACE) and so are the grab (the first frames after the HUD shows, before the
countdown board's red 3) and ours' render; its helpers are imported from that script. What differs is the original:

* The port runs in a headless `sway` on the render node with `SDL_VIDEODRIVER=wayland`, `SDL_AUDIODRIVER=dummy` and
  `SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=1` (without it the window never sees the virtual pad). A `for_window` rule makes the
  window full screen: without it the frame carries the port's title bar and a border (a 1920x1080 frame, but not the game's).
* **A fresh save every boot** (`GOW3_USER_DIR` under the instance directory, deleted first), so Track Select opens on Vineta K and
  Ship Select on its first craft; the GPU caches stay in one persistent `GOW3_GPU_USER_DIR`, so a boot is not a cold shader build.
* The pad is system-wide (`scripts/uinput-pads.py`): the driver refuses to start when an Xbox-360-shaped pad it did not make exists.
* **WipEout 2048 mode** (`--game-mode 2048`: RACEBOX, GAME MODE one Right; `--class-steps N` Rights on SPEED CLASS, C, B, A, A+).
  Ship Select opens on the yellow Feisar2048, so there are no Downs. The race load ends in an intro that loops a fly-over of the
  circuit under a START RACE prompt: the first tap of cross starts a craft presentation, which returns to the fly-over, and a second tap
  about 2.5 s later starts the race, so the driver taps until the 2048 HUD shows (and stops tapping then). That HUD is told from the
  fly-over by reference crops of the "LAP 1/4, TOTAL, CURRENT" labels (`--hud-labels-ref`) and the TOTAL timer reading 0.00.00
  (`--hud-timer-ref`), both thresholded to a white mask against the frame's own (a fade to white scores as no match); the grab is the
  first HUD frame plus `GRAB_AFTER_HUD`, and the timer must still read 0.00.00 there (the grid before GO). HD's yellow-ring detector
  and countdown-board check do not apply to this HUD and are not run.
* **Proof of the state, not only the walk**: `track-select.png`'s circuit-name crop is compared with `--name-ref` (a crop of the
  same screen made once from the original, kept in scratch because it is a game frame), and the pair is refused when the name is
  not the one asked for; `--ship-ref` does the same for the Feisar logo on Ship Select. The same frames are kept in the pair directory.
"""

import argparse
import importlib.util
import json
import os
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402
import compare_provenance as cp  # noqa: E402

_spec = importlib.util.spec_from_file_location("shadps4_omega_pair", HERE / "shadps4-omega-pair.py")
sp = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(sp)

#: The port's settings the capture sets (gow3.ini); everything else is the port's default. `native` is 1920x1080.
#: `engine_fps=60` only leaves the frame rate patches out; the port's Omega profile (`scripts/game_profiles.py`) sets `GOW3_ENGINE_FPS=240`
#: (the engine steps by the time between frames), which the port log reports as "Engine frame rate: 240 FPS" and `pair.json` records.
GOW3_INI = {"engine_fps": "60", "render_resolution": "native"}
#: The circuit name's crop on Track Select, 1920x1080 frame (width x height + x + y): the name under the hexagon.
NAME_CROP = "700x60+70+360"
#: Largest normalised RMSE between the name crop and its reference for "the same name".
NAME_RMSE_MAX = 0.08
#: How far `board_red` may rise from the first HUD frame to the grab before the countdown board counts as lit.
BOARD_LIT_RISE = 0.08
#: How far the lap ring reading rises from the first grid frame (no HUD yet) when the HUD shows.
HUD_RISE = 0.1
#: The team logo's crop on Ship Select (the cursor's team, which the model preview beside it does not move).
SHIP_CROP = "420x140+220+170"
#: 2048 mode's HUD, 1920x1080 crops thresholded to a white mask (gray over 97 %): the "LAP / 1/4 / TOTAL / CURRENT" labels, which
#: stay put on every circuit, and the TOTAL reading. Against a reference mask made once from a HUD frame of the original, the labels
#: read 0.00 to 0.07 on 2048-mode HUD frames (0.17 to 0.22 when white scenery sits behind them) and 0.34 or more on every fly-over,
#: presentation, fade and menu frame seen; the timer reads 0.00 and 0.04 on two frames showing 0.00.00 and 0.35 on 0.02.16.
HUD2048_LABELS_CROP = "270x230+0+0"
HUD2048_TIMER_CROP = "200x50+280+138"
HUD2048_LABELS_MAX = 0.20
HUD2048_TIMER_MAX = 0.15
#: Seconds between taps of cross on the START RACE intro (a tap closer than the craft presentation's start is swallowed).
START_TAP_EVERY = 2.5
#: Give up on the HUD after this many seconds of tapping.
START_TAP_LIMIT = 70
#: The Xbox-360-shaped pad `uinput-pads.py` makes; a second one on the machine is somebody else's.
PAD_NAME = "Microsoft X-Box 360 pad"


def log(msg):
    print("[omega-port-pair] " + msg, file=sys.stderr, flush=True)


def foreign_pads():
    """Names of virtual pads already on the machine: another run's pad would answer to the same presses."""
    text = Path("/proc/bus/input/devices").read_text(errors="replace")
    return [l for l in text.splitlines() if l.startswith("N:") and "X-Box 360" in l]


def port_commit(port):
    return subprocess.run(["git", "-C", str(port), "rev-parse", "--short=12", "HEAD"], capture_output=True, text=True).stdout.strip()


class Rig:
    """The processes this run started, by PID, and everything it needs to drive them."""

    def __init__(self, scratch, game, port, lane):
        self.dir = Path(scratch).resolve()
        self.game = Path(game).resolve()
        self.port = Path(port).resolve()
        self.dir.mkdir(parents=True, exist_ok=True)
        # a short path: the sway IPC socket lives here and a unix socket path is limited to 107 bytes
        self.runtime = Path("/tmp/oag-omega-port-%s" % lane[:24])
        self.pids = {}
        self.env = {**os.environ, "XDG_RUNTIME_DIR": str(self.runtime), "WAYLAND_DISPLAY": "wayland-1"}
        self.hold = None

    def spawn(self, name, cmd, **kw):
        p = subprocess.Popen([str(c) for c in cmd], start_new_session=True, **kw)
        self.pids[name] = p.pid
        (self.dir / "pids.json").write_text(json.dumps(self.pids))
        log("started %s pid %d" % (name, p.pid))
        return p

    def start_sway(self):
        self.runtime.mkdir(mode=0o700, exist_ok=True)
        for stale in [*self.runtime.glob("wayland-1*"), *self.runtime.glob("sway-ipc.*")]:  # a killed sway leaves its socket, and "exists" would mean "not yet"
            stale.unlink()
        conf = self.dir / "sway.conf"
        conf.write_text('output HEADLESS-1 resolution 1920x1080\nfor_window [app_id="gow3"] fullscreen enable\n')
        env = {k: v for k, v in self.env.items() if k not in ("DISPLAY", "WAYLAND_DISPLAY")}
        env.update(WLR_BACKENDS="headless", WLR_RENDERER="vulkan", WLR_RENDER_DRM_DEVICE="/dev/dri/renderD128", WLR_LIBINPUT_NO_DEVICES="1")
        self.spawn("sway", ["sway", "-c", conf, "--unsupported-gpu"], env=env, stdout=open(self.dir / "sway.log", "w"), stderr=subprocess.STDOUT)
        for _ in range(40):
            if (self.runtime / "wayland-1").exists():
                return
            time.sleep(0.25)
        sys.exit("sway made no wayland-1 socket in %s" % self.runtime)

    def write_ini(self):
        data = self.dir / "data"
        data.mkdir(exist_ok=True)
        (data / "gow3.ini").write_text("# gow3 settings\n" + "".join("%s=%s\n" % kv for kv in GOW3_INI.items()))

    def fresh_user(self):
        """A save-less profile: the directory is deleted (guarded: only ever a `user` directory inside the instance's own)."""
        user = self.dir / "user"
        if user.exists():
            assert user.resolve().parent == self.dir and user.name == "user", user
            shutil.rmtree(user)
        user.mkdir()
        return user

    def start_port(self):
        h = self.dir / "home"
        for sub in (".local/share/shadPS4", ".config", ".cache"):
            (h / sub).mkdir(parents=True, exist_ok=True)
        user = self.fresh_user()
        gpu = self.dir / "gpu-cache"
        gpu.mkdir(exist_ok=True)
        env = {k: v for k, v in self.env.items() if k != "DISPLAY"}
        env.update(SDL_VIDEODRIVER="wayland", SDL_AUDIODRIVER="dummy", SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS="1", HOME=str(h),
                   XDG_DATA_HOME=str(h / ".local/share"), XDG_CONFIG_HOME=str(h / ".config"), XDG_CACHE_HOME=str(h / ".cache"),
                   GOW3_GAME_DIR=str(self.game), GOW3_DATA_DIR=str(self.dir / "data"), GOW3_USER_DIR=str(user),
                   GOW3_GPU_USER_DIR=str(gpu), GOW3_TIMEOUT="0")
        self.spawn("port", ["timeout", "1500", sys.executable, self.port / "run.py"], env=env,
                   stdout=open(self.dir / "port.log", "w"), stderr=subprocess.STDOUT)

    def start_pad(self):
        fifo = self.dir / "pad.cmd"
        if fifo.exists():
            fifo.unlink()
        os.mkfifo(fifo)
        self.hold = open(fifo, "r+b", buffering=0)  # keeps the fifo open for the pad's reader
        env = {**os.environ, "OAG_PADS": "1", "UV_CACHE_DIR": os.path.expanduser("~/.cache/uv"), "UV_OFFLINE": "1"}
        self.spawn("pad", ["uv", "run", "--with", "evdev", "python3", HERE / "uinput-pads.py"], env=env,
                   stdin=open(fifo, "rb"), stdout=open(self.dir / "pad.log", "w"), stderr=subprocess.STDOUT)

    def press(self, button, wait=0.9):
        self.hold.write(b"a press %s\n" % button.encode())
        time.sleep(wait)

    def grab(self, path):
        subprocess.run(["grim", str(path)], check=True, env=self.env)
        return path

    def stop(self):
        """By PID, children first; a pad's `uv` leaves the python child behind unless its process group goes."""
        for name in ("pad", "port", "sway"):
            pid = self.pids.get(name)
            if pid:
                try:
                    os.killpg(pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
        time.sleep(3)
        for pid in self.pids.values():
            try:
                os.killpg(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        log("stopped pids %s" % self.pids)


def name_crop(frame, out):
    subprocess.run(["magick", str(frame), "-crop", NAME_CROP, "+repage", str(out)], check=True)
    return out


def ship_crop(frame, out):
    subprocess.run(["magick", str(frame), "-crop", SHIP_CROP, "+repage", str(out)], check=True)
    return out


def rmse(a, b):
    r = subprocess.run(["compare", "-metric", "RMSE", str(a), str(b), "null:"], capture_output=True, text=True)
    return float(r.stderr.split("(")[1].split(")")[0])


def white_mask_rmse(frame, geometry, ref, scratch):
    """Normalised RMSE between the frame's crop thresholded to a white mask and the reference mask."""
    subprocess.run(["magick", str(frame), "-crop", geometry, "+repage", "-colorspace", "Gray", "-threshold", "97%", str(scratch)], check=True)
    return rmse(ref, scratch)


def hud2048(rig, args, frame):
    """(labels, timer) mask RMSE of a frame against the 2048 HUD references."""
    scratch = rig.dir / "mask.png"
    return (white_mask_rmse(frame, HUD2048_LABELS_CROP, args.hud_labels_ref, scratch),
            white_mask_rmse(frame, HUD2048_TIMER_CROP, args.hud_timer_ref, scratch))


def grid_frames_2048(rig, args, out):
    """Tap cross on START RACE until the 2048 HUD shows, then grab. Returns the frames from the first HUD frame on."""
    scan, frames, scores, t0, last_tap, hud_at, taps = out / "scan.png", [], [], time.time(), None, None, 0
    while time.time() - t0 < START_TAP_LIMIT:
        rig.grab(scan)
        labels, timer = hud2048(rig, args, scan)
        scores.append((round(labels, 3), round(timer, 3)))
        if hud_at is None and labels < HUD2048_LABELS_MAX:
            hud_at = 0
            log("2048 HUD up after %.1f s and %d taps (labels %.3f, timer %.3f)" % (time.time() - t0, taps, labels, timer))
        if hud_at is not None:
            path = out / ("frame%02d.png" % len(frames))
            shutil.copy(scan, path)
            frames.append(path)
            if len(frames) > sp.GRAB_AFTER_HUD + 3:
                break
        elif last_tap is None or time.time() - last_tap >= START_TAP_EVERY:
            rig.press("cross", 0.2)
            last_tap, taps = time.time(), taps + 1
        time.sleep(0.2)
    scan.unlink()
    if hud_at is None:
        sys.exit("STALLED at grid: no 2048 HUD in %d s and %d taps (best labels %.3f; see loading.png)" % (START_TAP_LIMIT, taps, min(s[0] for s in scores)))
    return frames, {"taps": taps, "scan_scores_labels_timer": scores, "hud_up_after_s": round(time.time() - t0, 1)}


def capture_original(rig, args, out):
    """Boot, walk to the grid, grab. Returns the chosen frame's path and what was seen."""
    rig.write_ini()
    rig.start_sway()
    rig.start_port()
    rig.start_pad()
    probe = rig.dir / "probe.png"
    for waited in range(0, 180, 3):
        time.sleep(3)
        if "Game process exited" in (rig.dir / "port.log").read_text(errors="replace"):
            sys.exit("STALLED at boot: the port exited (see port.log)")
        try:
            rig.grab(probe)
            r, g, b = sp.pixel(probe, *sp.MENU_RED)
        except subprocess.CalledProcessError:
            continue
        if r > 200 and g < 40 and b < 40:
            log("MAIN MENU after about %d s" % (waited + 3))
            break
    else:
        sys.exit("STALLED at boot: no MAIN MENU in 180 s (see port.log)")
    rig.press("right")
    rig.press("cross", 3)  # MAIN MENU -> Race Box
    if args.game_mode == "2048":
        rig.press("right", 1.5)  # GAME MODE: WipEout HD -> WipEout 2048
        if args.class_steps:
            rig.press("down")
            rig.press("down")  # RACE TYPE, SPEED CLASS
        for _ in range(args.class_steps):
            rig.press("right", 1.5)  # SPEED CLASS: C -> B -> A -> A+
        time.sleep(1)
        rig.grab(out / "racebox.png")
    rig.press("cross", 3)  # Race Box -> Track Select
    for _ in range(args.track_steps):
        rig.press("right", 1.5)
    time.sleep(1.5)
    rig.grab(out / "track-select.png")
    name_crop(out / "track-select.png", out / "circuit-name.png")
    rig.press("cross", 3)  # Track Select -> Ship Select
    if args.game_mode != "2048":  # 2048 mode's Ship Select opens on the Feisar2048 itself
        rig.press("down")
        rig.press("down")
    time.sleep(1)
    rig.grab(out / "ship-select.png")
    rig.press("cross", 3)  # Ship Select -> loading
    time.sleep(args.load_wait)
    rig.grab(out / "loading.png")
    time.sleep(args.load_wait2)  # the START RACE prompt waits for the confirm
    if args.game_mode == "2048":
        frames, seen = grid_frames_2048(rig, args, out)
        pick = frames[sp.GRAB_AFTER_HUD]
        seen.update({"frames": len(frames), "hud_first_frame": 0, "picked_frame": sp.GRAB_AFTER_HUD, "frames_seen": [f.name for f in frames]})
        return pick, seen
    rig.press("cross", 0.2)
    frames, hud_at, t0, ring0 = [], None, time.time(), None
    while time.time() - t0 < 20:
        path = rig.grab(out / ("frame%02d.png" % len(frames)))
        frames.append(path)
        ring = sp.region_mean(path, sp.HUD_REGION, "(mean.r+mean.g)/2-mean.b")
        ring0 = ring if ring0 is None else ring0
        # the lap hexagon's yellow ring: over the shipped threshold, or well above the first frame's (a pale scene, Tech De Ra,
        # keeps the ring under the threshold). A yellow sign in the loading shot's corner can pass it, so the board check and
        # the eye (`side.png`) decide whether the frame is the grid.
        if hud_at is None and (ring > 0.03 or ring - ring0 > HUD_RISE):
            hud_at = len(frames) - 1
        if hud_at is not None and len(frames) > hud_at + sp.GRAB_AFTER_HUD + 3:
            break
        time.sleep(0.3)
    if hud_at is None:
        sys.exit("STALLED at grid: the HUD never showed in %d frames (see frame*.png)" % len(frames))
    pick = frames[hud_at + sp.GRAB_AFTER_HUD]
    return pick, {"frames": len(frames), "hud_first_frame": hud_at, "picked_frame": hud_at + sp.GRAB_AFTER_HUD,
                  "frames_seen": [f.name for f in frames], "board_red": [round(sp.board_red(f), 3) for f in frames[hud_at:]]}


def prepare_game(args):
    sp.prepare_game(args)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    pg = sub.add_parser("prepare-game")
    pg.add_argument("--game", required=True)
    pr = sub.add_parser("pair")
    pr.add_argument("--scratch", required=True, help="the instance's own directory under data/scratch/<lane>/ (HOME, save, caches, logs)")
    pr.add_argument("--game", required=True)
    pr.add_argument("--port", default=str(ce.MAIN / "data/tools/wipeout_pc"), help="the port's checkout (built: out/gow3-probe)")
    pr.add_argument("--out", type=Path, required=True)
    pr.add_argument("--track", required=True, help="ours' track .vex entry name")
    pr.add_argument("--track-steps", type=int, default=0, help="Right presses on Track Select from Vineta K")
    pr.add_argument("--circuit", required=True, help="the name Track Select shows, for the record and the name check")
    pr.add_argument("--name-ref", type=Path, help="a crop (NAME_CROP) of Track Select showing this circuit's name, from the original")
    pr.add_argument("--ship-ref", type=Path, help="a crop (SHIP_CROP) of Ship Select with the Feisar logo, from the original")
    pr.add_argument("--twin-of", type=Path, help="an earlier theirs.png of the same pair: its normalised RMSE against this boot is the twin")
    pr.add_argument("--game-mode", choices=("hd", "2048"), default="hd", help="Race Box's GAME MODE: WipEout HD (default) or WipEout 2048 (see the docstring)")
    pr.add_argument("--class-steps", type=int, default=0, help="2048 mode: Rights on SPEED CLASS from C CLASS (2 reaches A CLASS)")
    pr.add_argument("--class-label", default="", help="2048 mode: the class the Race Box shows after --class-steps, for the record")
    pr.add_argument("--ours-class", default="venom", help="ours' --class (the mapping of 2048's C, B, A, A+ to ours' rungs is assumed, not measured)")
    pr.add_argument("--hud-labels-ref", type=Path, help="2048 mode: white mask of the LAP/TOTAL/CURRENT label crop of a 2048 HUD frame of the original")
    pr.add_argument("--hud-timer-ref", type=Path, help="2048 mode: white mask of the TOTAL reading crop showing 0.00.00")
    pr.add_argument("--team", default=sp.DEFAULT_TEAM)
    pr.add_argument("--load-wait", type=float, default=12, help="seconds after the Ship Select confirm before the loading frame")
    pr.add_argument("--load-wait2", type=float, default=14, help="then seconds before START RACE is confirmed")
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
    if args.game_mode == "2048" and not (args.hud_labels_ref and args.hud_timer_ref):
        sys.exit("2048 mode needs --hud-labels-ref and --hud-timer-ref (the HUD detector's references)")
    others = foreign_pads()
    if others:
        sys.exit("refusing to start: a virtual pad already exists (%s); the pad is system-wide, so it would answer to this run's presses" % others)
    pair = ce.Pair(args.platform, args.out)
    plat = pair.plat
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    pair.out = out
    commit = port_commit(args.port)
    pair.rec["original"]["emulator"] = {"name": "wipeout_pc native port", "commit": commit, "resolution": "1920x1080", "upscaler": "none",
                                        "engine_fps_ini": int(GOW3_INI["engine_fps"]), "capture": "grim on a headless sway, full screen",
                                        "fresh_save": True}
    pair.rec["ours"]["binary"] = ce.binary_record(args.oag_game)
    rig = Rig(args.scratch, args.game, args.port, Path(args.scratch).resolve().parent.name)
    try:
        shot, seen = capture_original(rig, args, out)
    finally:
        rig.stop()
    shutil.copy(shot, out / "theirs.png")
    pair.stamp("original")
    pair.rec["original"]["grid_capture"] = seen
    fps = [l for l in (rig.dir / "port.log").read_text(errors="replace").splitlines() if l.startswith("Engine frame rate")]
    pair.rec["original"]["emulator"]["engine_frame_rate_line"] = fps[-1] if fps else None
    status = rig.dir / "data/out/patch_status.json"
    if status.exists():
        pair.rec["original"]["emulator"]["patch_status"] = json.loads(status.read_text())
    mode_2048 = args.game_mode == "2048"
    pair.rec["original"]["state"] = {"circuit": args.track, "circuit_name": args.circuit,
                                     "mode": "single_race (Race Box, %s)" % ("WipEout 2048 mode" if mode_2048 else "WipEout HD mode"),
                                     "class": (args.class_label or "C CLASS") if mode_2048 else "venom", "team": args.team,
                                     "track_steps": args.track_steps, "class_steps": args.class_steps, "game_mode": args.game_mode,
                                     "source": "a cold boot on a fresh save, walked from MAIN MENU"}
    if mode_2048:
        pair.rec["ours"]["class_mapping"] = "assumed, not measured: 2048's C, B, A, A+ to ours' %s" % args.ours_class
    cp.build(pair, args, "scenery", "cold_boot", None,
             {"value": 0, "unit": "ticks", "basis": "none needed: the grid before GO is a still (craft at rest, camera static), "
                                                    "no pose readout exists on this title's port", "what": "picture"})
    if args.twin_of and args.twin_of.exists():
        d = rmse(args.twin_of, out / "theirs.png")
        pair.rec["original"]["provenance"]["twin"] = {"agreed": None, "rmse_vs_previous": round(d, 4),
                                                      "basis": "a second cold boot of the same state; no agreement threshold is documented"}
    pair.check_size("original", out / "theirs.png")
    for f in ("track-select.png", "ship-select.png", "theirs.png"):
        pair.check("%s is a full-screen frame (no window title bar)" % f, bool(_no_chrome(out / f)), "frame top rows read as the scene, not the port's bar")
    if args.name_ref:
        d = rmse(args.name_ref, out / "circuit-name.png")
        pair.check("Track Select names %s" % args.circuit, d < NAME_RMSE_MAX, "name crop RMSE %.4f against the reference (limit %.2f; another circuit's name reads 0.15 or more)" % (d, NAME_RMSE_MAX))
    if args.ship_ref:
        d = rmse(args.ship_ref, ship_crop(out / "ship-select.png", out / "ship-logo.png"))
        pair.check("Ship Select is on the %s" % ("Feisar2048" if mode_2048 else "Feisar"), d < NAME_RMSE_MAX, "team logo crop RMSE %.4f against the reference (limit %.2f)" % (d, NAME_RMSE_MAX))
    if mode_2048:
        labels, timer = hud2048(rig, args, out / "theirs.png")
        first_timer = hud2048(rig, args, out / "frame00.png")[1]
        seen["picked_labels_timer"] = [round(labels, 3), round(timer, 3)]
        pair.check("the 2048 HUD is up", labels < HUD2048_LABELS_MAX, "label mask RMSE %.3f against the reference (limit %.2f; fly-over, presentation and fade frames read 0.34 or more)" % (labels, HUD2048_LABELS_MAX))
        pair.check("the grab is before GO (TOTAL reads 0.00.00)", timer < HUD2048_TIMER_MAX and first_timer < HUD2048_TIMER_MAX,
                   "TOTAL mask RMSE %.3f at the grab and %.3f at the first HUD frame against a 0.00.00 reference (limit %.2f; 0.02.16 reads 0.35)" % (timer, first_timer, HUD2048_TIMER_MAX))
    else:
        red = sp.board_red(out / "theirs.png")
        base = seen["board_red"][0]  # the first frame with the HUD up: the board is blank there on every circuit
        pair.check("the countdown board is still blank (before its red 3)", red - base < BOARD_LIT_RISE,
                   "board red minus blue %.3f at the grab, %.3f at the first HUD frame (a lit digit raises it by 0.2 or more; the value itself depends on the scenery behind the board)" % (red, base))
    profile_dir = Path(os.environ["XDG_CONFIG_HOME"]).parent
    ce.write_ours_settings(plat, profile_dir)
    source = ce.MAIN / "data/extracted/ps4"
    flags = ce.ours_flags(plat) + ["--mode", "single_race", "--class", args.ours_class, "--team", args.team]
    base = [args.oag_game, source, "--race", "--track", args.track, "--ticks", args.ours_ticks] + flags
    pair.rec["ours"]["flags"] = [str(c) for c in base[2:]]
    pair.rec["ours"].update({"mode": "single_race", "class": args.ours_class, "team": args.team, "ticks": args.ours_ticks,
                             "camera": "the title's default chase view; the grid is at rest, no pose is fed"})
    env = {**os.environ, **ce.ours_env(profile_dir)}
    sp.run(base + ["--screenshot", out / "ours.png"], stdout=subprocess.DEVNULL, env=env)
    got, bad = ce.read_back(plat, profile_dir)
    pair.rec["ours"]["settings_readback"] = got
    for b in bad:
        pair.check("ours settings", False, b)
    pair.check("ours settings read back", not bad, "%d keys equal the profile" % (len(got) - len(bad)))
    pair.check_size("ours", out / "ours.png")
    pair.rec["craft_offset_px"] = sp.craft_offset(out / "theirs.png", out / "ours.png")
    pair.rec["craft_offset_px"]["note"] = "informational: the hulls are lit and coloured differently; not a measurement"
    if mode_2048:
        pair.caveat("ours races its %s rung for 2048's %s, an assumed mapping, and its lap count differs from the original's (the original: 4 laps at C CLASS, 5 at A CLASS on Sol; ours on Sol: venom 3, flash 4, rapier 4, phantom 5)"
                    % (args.ours_class, args.class_label or "C CLASS"))
        pair.caveat("ours draws HD's HUD (lap hexagon, position disc, BEST and time bar, speed readout) in Omega's 2048 mode; the original draws 2048's (LAP 1/4, TOTAL, CURRENT, POS 8/8, temperature gauge)")
    else:
        pair.caveat("ours' gantry board shows the grid position 8 and never changes; the original's is blank before the red 3 and counts 3, 2, 1, GO")
    if mode_2048:
        pair.caveat("the original's Feisar2048 hull is lit and graded by the scene (orange at a sunset circuit, yellow and white at a grey one) and stands in a white start ring; ours lights it flatter and draws no ring")
    else:
        pair.caveat("the original's hull livery varies from boot to boot; ours' is fixed")
    sp.run(["magick", "montage", out / "theirs.png", out / "ours.png", "-tile", "2x1", "-geometry", "+2+2", out / "side.png"])
    if args.meta:
        pair.rec["meta"] = json.loads(args.meta)
    pair.finish()
    print("wrote", out / "side.png")


def _no_chrome(path):
    """The port's title bar (a 26 px band of its own colour across the top, a 2 px border at the sides) is absent: the top-left
    pixel is not the bar's blue-grey and the first column is not the border's black."""
    r, g, b = sp.pixel(path, 400, 8)
    return not (abs(r - 44) < 12 and abs(g - 85) < 12 and abs(b - 125) < 12)


if __name__ == "__main__":
    main()
