#!/usr/bin/env python3
"""HD Zone race on RPCS3: read the Zone palette textures' texels live.

Cold boot into Racebox Zone (which flies `Data\\Environments\\Zone_1`), hold
throttle, and sample through `/proc/<pid>/mem`, no pause:

- the stage state `H[0]` (`0x008c2cb8`, 0x38 bytes: `+0x00` current stage);
- the seven two-entry texture-handle arrays at `0x00c81330`..`0x00c81368`;
- the texel pointers the loader keeps beside them (`0x00c7dfb0 + 14964`);
- each handle's header and the bytes its texel pointer names: 3 x 8 bytes for
  a gradient, 17 x 8 bytes for an aniso palette (FP16 RGBA);
- `zoneAnisoPower` Scene (`0x00c81460`) and Track (`0x00c814e0`).

Run from the repository root under scripts/emu-run.py with a private env
(scripts/emu-env.sh). Evidence page:
docs/ghidra/functions/ps3-hdfury-eu/zone-palette-textures.md, section 5.
"""
import argparse
import importlib.util
import pickle
import struct
import sys
import time
from pathlib import Path

ROOT = Path.cwd()
sys.path.insert(0, str(ROOT / "scripts"))
_spec = importlib.util.spec_from_file_location("rpcs3_trace", ROOT / "scripts" / "rpcs3-trace.py")
tr = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(tr)
drive = tr.drive

STAGE_STATE = 0x008C2CB8
HANDLES = 0x00C81330
HANDLE_WORDS = 14
TEXEL_PTRS = 0x00C7DFB0 + 14964
TEXEL_PTR_WORDS = 14
ANISO_POWER = (0x00C81460, 0x00C814E0)


def sample(guest):
    row = {
        "state": guest.read(STAGE_STATE, 0x38),
        "handles": guest.read(HANDLES, 4 * HANDLE_WORDS),
        "texel_ptrs": guest.read(TEXEL_PTRS, 4 * TEXEL_PTR_WORDS),
        "power": [guest.read(a, 16) for a in ANISO_POWER],
        "tex": {},
    }
    handles = struct.unpack(">%dI" % HANDLE_WORDS, row["handles"])
    for i, h in enumerate(handles):
        if not h:
            continue
        try:
            head = guest.read(h, 0x50)
        except OSError as exc:
            row["tex"][i] = ("head-error", str(exc))
            continue
        data = struct.unpack(">I", head[0x10:0x14])[0]
        texels = 3 if HANDLES + 4 * i < 0x00C81348 else 17
        try:
            body = guest.read(data, 8 * texels)
        except OSError as exc:
            body = ("data-error", str(exc))
        row["tex"][i] = (head, data, body)
    return row


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--rights", type=int, default=5)
    ap.add_argument("--drive-secs", type=float, default=150.0)
    ap.add_argument("--step", type=float, default=1.0)
    args = ap.parse_args()
    out = Path(args.out)
    (out / "frames").mkdir(parents=True, exist_ok=True)
    plan = {"Main Menu": ["right"],
            "Single Player": ["right"] * args.rights + ["cross"]}
    rows = []
    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        (out / "rpcs3.pid").write_text("%d\n" % session.proc.pid)
        if not session.wait_for_screen_pressing("Main Menu", 240):
            sys.exit("never reached the Main Menu")
        session.settle_menu(20)

        navs = [0]

        def race_type_crop():
            # The RACE TYPE value box on the Racebox screen (1280x720 trimmed frame).
            from PIL import Image
            p = out / "frames" / "probe.png"
            drive.screenshot(p, trim=True)
            return Image.open(p).convert("L").resize((1280, 720)).crop((470, 125, 680, 147)).tobytes()

        def navigate(plan, session=session):
            screen = drive.current_screen()
            for button in plan.get(screen, []):
                for attempt in range(3):
                    before = race_type_crop() if screen == "Single Player" else None
                    if button in ("cross", "circle"):
                        session.press_once(button)
                    else:
                        session.tap(button, settle=1.5)
                    time.sleep(1.5)
                    navs[0] += 1
                    drive.screenshot(out / "frames" / ("nav-%02d.png" % navs[0]), trim=True)
                    print("    nav %-8s at %s" % (button, screen), flush=True)
                    if before is None or button != "right" or race_type_crop() != before:
                        break
                    print("    (race type unchanged, tapping again)", flush=True)
            plan.pop(screen, None)

        session.navigate = navigate
        if session.walk_to_race(plan=plan) not in drive.RACE_ARRIVED:
            sys.exit("no race")
        tty = drive.tty_text()
        modes = [line for line in tty.splitlines() if "GetMode()" in line]
        print("track: %s; mode lines: %s" % (drive.track_name(), modes[-2:]), flush=True)
        if not any("GetMode()==6" in line for line in modes[-1:]):
            sys.exit("not a Zone race: %s" % modes[-1:])
        session.wait_for_load(120)
        guest = tr.Guest(session.proc.pid, session.open_mem())
        t0 = time.time()

        def run(phase, secs, shot_every):
            t = time.time()
            k = 0
            last_stage = None
            while time.time() - t < secs:
                r = sample(guest)
                r["wall"] = time.time() - t0
                r["phase"] = phase
                stage = struct.unpack(">I", r["state"][:4])[0]
                if k % shot_every == 0 or stage != last_stage:
                    p = out / "frames" / ("%s-%04d-stage%d.png" % (phase, k, stage))
                    drive.screenshot(p, trim=True)
                    r["frame"] = str(p)
                    print("  %6.1fs %s stage %d" % (r["wall"], phase, stage), flush=True)
                last_stage = stage
                rows.append(r)
                k += 1
                time.sleep(args.step)

        run("flyby", 3.0, 3)
        session.tap("cross", settle=0.0)
        run("count", 10.0, 5)
        session.pad.set("cross", True)
        run("drive", args.drive_secs, 15)
        session.pad.set("cross", False)
        pickle.dump({"rows": rows, "track": drive.track_name(), "modes": modes},
                    open(out / "palette.pkl", "wb"))
    print("done: %d samples" % len(rows), flush=True)


if __name__ == "__main__":
    main()
