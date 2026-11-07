#!/usr/bin/env python3
"""Walks Wipeout Omega Collection (shadPS4) to a race and photographs its pause screen.

    uv run --with evdev python3 scripts/emu-run.py --bg --total 2400 --status F -- \\
        python3 scripts/shadps4-omega-pause-capture.py SCRATCH GAME OUT "pause:p1,hold-down:30,shot:p2"

`SCRATCH` and `GAME` are `shadps4-omega-pair.py`'s `--scratch` and `--game` (its `prepare-game` makes the folder), `OUT` the
directory the frames go to. The last argument is the plan after the race has started: `pause:NAME` (START, then polls the
frame until it darkens), `hold-BUTTON:SECONDS`, `BUTTON:SECONDS`, `shot:NAME`, `wait:SECONDS`, `burst:N`.

Environment: `RACE_TYPE_ROW=1` moves to the Racebox RACE TYPE row; `TARGET_TYPE=time_trial|speed_lap|eliminator|zone` steps it
until the icon matches the crop in `$OMEGA_TYPE_REFS/modeN.png` (N is 1 to 4, a 1920x1080 frame of the Racebox page on that
type with RACE TYPE focused); `SHIP_DOWNS` is the Downs on Ship Select (default 2, the first unlocked Feisar craft; 0 for Zone).
`MODE_RIGHTS=N` with `STOP_AT_MODE=1` photographs N steps of a row and stops.

shadPS4 runs about a frame a second under a loaded machine and drops short presses, so every step polls what is on screen.
Zone killed shadPS4 0.20 at the race start (`docs/ui/pause-menu.md`, "Omega").
"""
import importlib
import os
import subprocess
import sys
import time
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
op = importlib.import_module("shadps4-omega-pair")
scratch, game, out = (Path(a) for a in sys.argv[1:4])
plan = sys.argv[4] if len(sys.argv) > 4 else ""
out.mkdir(parents=True, exist_ok=True)
rig = op.Rig(scratch, game)
try:
    rig.write_config(); rig.start_sway(); rig.start_shadps4(); rig.start_pad()
    probe = rig.dir / "probe.png"
    for waited in range(0, 420, 3):
        time.sleep(3); print("boot wait", waited, flush=True)
        try:
            rig.grab(probe); r, g, b = op.pixel(probe, *op.MENU_RED)
        except Exception:
            continue
        if r > 200 and g < 40 and b < 40:
            op.log("MAIN MENU after about %d s" % (waited + 3)); break
    else:
        sys.exit("STALLED at boot")
    time.sleep(8)
    for n in range(12):
        rig.press("right", 4); rig.grab(out / "w0.png")
        r, g, b = op.pixel(out / "w0.png", 700, 150)
        print("right try", n, (r, g, b), flush=True)
        if r > 200 and g < 70:
            break
    else:
        sys.exit("STALLED: the pad does not reach the game")
    rig.press("cross", 9); rig.grab(out / "w1.png")
    if os.environ.get("RACE_TYPE_ROW"):
        for n in range(8):
            rig.hold.write(b"a hold down 0.25\n"); time.sleep(10)
            rig.grab(out / "row.png")
            r, g, b = op.pixel(out / "row.png", 400, 240)
            print("row probe", n, (r, g, b), flush=True)
            if r > 200 and g < 70:
                break
            if n > 0 and False:
                pass
    target = os.environ.get("TARGET_TYPE")
    if target:
        import subprocess
        ref = {"time_trial": 1, "speed_lap": 2, "eliminator": 3, "zone": 4}[target]
        refpng = os.path.join(os.environ["OMEGA_TYPE_REFS"], "mode%d.png" % ref)
        crop = ["-crop", "500x320+1180+160", "+repage"]
        def dist(png):
            a_ = subprocess.run(["magick", str(png), *crop, "-resize", "50%", "gray:-"], capture_output=True).stdout
            b_ = subprocess.run(["magick", refpng, *crop, "-resize", "50%", "gray:-"], capture_output=True).stdout
            return sum(abs(x - y) for x, y in zip(a_, b_)) / max(len(a_), 1)
        for n in range(14):
            rig.grab(out / "mode_probe.png"); d = dist(out / "mode_probe.png")
            print("type probe", n, round(d, 2), flush=True)
            if d < 6:
                break
            rig.hold.write(b"a hold right 0.25\n"); time.sleep(14)
        else:
            sys.exit("STALLED: race type never matched")
        rig.grab(out / "mode_chosen.png")
        rig.hold.write(b"a hold up 0.25\n"); time.sleep(10)
    for n in range(0 if target else int(os.environ.get("MODE_RIGHTS", "0"))):
        rig.hold.write(b"a hold right 0.25\n"); time.sleep(8); rig.grab(out / ("mode%d.png" % (n + 1)))
    if os.environ.get("STOP_AT_MODE"):
        sys.exit(0)

    rig.press("cross", 9); rig.grab(out / "w2.png")
    rig.press("cross", 9); rig.grab(out / "w3.png")
    for _ in range(int(os.environ.get("SHIP_DOWNS", "2"))):
        rig.press("down", 3)
    rig.grab(out / "w4.png")
    rig.press("cross", 3)
    def mean(png):
        import subprocess
        return float(subprocess.run(["magick", str(png), "-colorspace", "Gray", "-format", "%[fx:mean]", "info:"], capture_output=True, text=True).stdout)
    t0 = time.time()
    while time.time() - t0 < 500:
        time.sleep(4)
        rig.grab(out / "poll.png"); m = mean(out / "poll.png"); print("poll", int(time.time() - t0), round(m, 3), flush=True)
        if m < 0.85 and time.time() - t0 > 10:
            break
    else:
        sys.exit("STALLED: no prompt")
    op.log("prompt at %ds" % (time.time() - t0))
    def line(png):
        import subprocess
        v = float(subprocess.run(["magick", str(png), "-crop", "1100x1+200+1031", "+repage", "-colorspace", "Gray", "-format", "%[fx:mean]", "info:"], capture_output=True, text=True).stdout)
        return 0.5 < v < 0.92
    clear = 0
    for n in range(40):
        rig.grab(out / "poll.png")
        if line(out / "poll.png"):
            clear = 0
            rig.press("cross", 0.2)
        else:
            clear += 1
        print("start try", n, clear, flush=True)
        if clear >= 3:
            break
        time.sleep(8)
    for n in range(6):
        time.sleep(10)
        rig.grab(out / ("pre%02d.png" % n)); print("pre", n, flush=True)
    rig.grab(out / "race.png")
    for i, item in enumerate(filter(None, plan.split(","))):
        kind, _, rest = item.partition(":")
        if kind == "pause":
            rig.grab(out / "before-pause.png"); m0 = mean(out / "before-pause.png")
            rig.hold.write(b"a hold start 0.25\n")
            for n in range(14):
                time.sleep(10); rig.grab(out / "pausing.png"); m1 = mean(out / "pausing.png")
                print("pause probe", n, round(m0, 3), round(m1, 3), flush=True)
                if m1 < 0.8 * m0:
                    break
            time.sleep(4); rig.grab(out / (rest + ".png"))
        elif kind.startswith("hold-"):
            rig.hold.write(("a hold %s 0.25\n" % kind[5:]).encode()); time.sleep(float(rest) if rest else 3.0)
        elif kind == "burst":
            for n in range(int(rest)):
                time.sleep(5); rig.grab(out / ("burst%02d.png" % n)); print("burst", n, flush=True)
        elif kind == "shot":
            time.sleep(1.5); rig.grab(out / (rest + ".png"))
        elif kind == "wait":
            time.sleep(float(rest))
        else:
            rig.press(kind, float(rest) if rest else 1.5)
        print("step", item, flush=True)
finally:
    rig.stop()
