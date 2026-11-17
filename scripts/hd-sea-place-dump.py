"""Place the player craft at a pose, dump one frame's draws, and grab the window while the game is paused (RPCS3).

    OAG_RPCS3_ATTACH=1 HOOK_LIGHT=1 HOOK_NO_HEADS=1 HOOK_FP_LEN=0x900 \\
    uv run --with evdev python3 scripts/emu-run.py -- \\
        python3 scripts/hd-sea-place-dump.py x y z fx fy fz <out-dir> <stem>

Attached to a `serve` sitting in a running Time Trial (restored state, then three `cross` at `START RACE`). A placed
craft falls and the game resets it to the track within a few seconds, so the placement, the settle (`SETTLE`, default
0.8 s of game time), the dump and the window grab are one attach. Writes the dump (`hd-sea-reflect-read.py` reads it),
`body.json` (the craft's pose at the pause) and `theirs.png` (the 1920x1080 window at the origin after `FLUSH`, default
3.5 s, so the RSX has presented the paused frame; size the window first with `rpcs3-hd-pair.py`'s `set_window`).
`HOOK_FP_LEN=0x900` because the sea program is longer than the default 0x600 cut.
"""
import importlib.util, os, subprocess, sys, time, types, json, struct
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE + "/rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec); spec.loader.exec_module(drive)
import rpcs3_place as place
from pathlib import Path
from rpcs3_debugger import Debugger
x, y, z, fx, fy, fz = [float(v) for v in sys.argv[1:7]]
out = Path(sys.argv[7]); out.mkdir(parents=True, exist_ok=True); stem = sys.argv[8]
host, port = os.environ["OAG_RPCS3_GDB"].split(":")
# a restored state's command stream runs past the default 0x20000 seed
ring = os.environ.get("RING_DUMP", "0x40000000:0x120000")
args = types.SimpleNamespace(dump=[ring], dump_attempts=4, dump_gap=0.3, hook=HERE + "/rpcs3_draw_hook.py")
env = dict(os.environ)
with Debugger(port=int(port), host=host, timeout=120.0) as g:
    g.pause()
    ship, body = place.find_player(g)
    place.write_pose(g, body, (x, y, z), place.basis_rows((fx, fy, fz), (0, 1, 0)), 0.0)
    g.resume()
    time.sleep(float(os.environ.get("SETTLE", "0.8")))
    g.pause()
    drive.dump_frame(g, out, stem, args)
    pose = place.read_pose(g, body)
    print("pose after dump", pose["pos"])
    json.dump({"pos": pose["pos"], "rows": pose["rows"]}, open(out / "body.json", "w"))
    time.sleep(float(os.environ.get("FLUSH", "3.5")))
    subprocess.run(["import", "-display", "127.0.0.1:%s" % os.environ["OAG_RPCS3_DISPLAY"], "-window", "root", "-crop", "1920x1080+0+0", str(out / "theirs.png")], check=True)
    g.resume()
