#!/usr/bin/env python3
"""Screenshot HD's boost pulse at chosen values of `E` (and a damage frame at a chosen `P`) from an attached RPCS3.

    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-zoom-frames.py --out DIR \\
        --rest --rise 0.5 --hold --fall 0.6 --fall 0.2 --damage 0.6

Each target fires its own Turbo (state 4 into the pickup slot, triangle), polls `FunkLayer` `E` (`+0x64`) and `P`
(`+0x58`) through `/proc/<pid>/mem` and pauses the game through the GDB stub the moment the target is met, so the
screenshot is one frozen frame whose `E` and `P` are read at the same pause. `--rise T` waits for `E >= T` on the
way up, `--hold` for `E == 1`, `--fall T` for `E <= T` on the way down after the hold, `--damage T` for `P >= T`.
Files are `<out>/E<value>-P<value>-<kind>.png` plus `<out>/frames.txt`.
"""
import argparse
import importlib.util
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


poll = load("rpcs3_mem_poll", "rpcs3-mem-poll.py")
drive, place = poll.drive, poll.place
from rpcs3_debugger import Debugger  # noqa: E402

GUEST_BASE = 0x300000000
FUNK_LAYER = 0x00C50EE0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--rise", type=float, action="append", default=[])
    ap.add_argument("--hold", action="store_true")
    ap.add_argument("--fall", type=float, action="append", default=[])
    ap.add_argument("--damage", type=float, action="append", default=[])
    ap.add_argument("--rest", action="store_true", help="also one frame with no pulse")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    session = drive.read_session()
    mem = drive.RemoteMem(session["socket"])
    pad = drive.RemotePad(session["socket"])
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])

    def inputs():
        mem.seek(GUEST_BASE + FUNK_LAYER + 0x58)
        raw = struct.unpack(">13f", mem.read(52))
        return raw[0], raw[(0x64 - 0x58) // 4]

    targets = [("rise", t) for t in args.rise] + ([("hold", 1.0)] if args.hold else [])
    targets += [("fall", t) for t in args.fall] + [("damage", t) for t in args.damage]
    log = []
    with Debugger(port=port) as gdb:
        gdb.pause()
        ship, _body = place.find_player(gdb)
        slot = struct.unpack(">I", gdb.read(ship + poll.SLOT_OFFSET, 4))[0]
        gdb.resume()
        if args.rest:
            gdb.pause()
            p, e = inputs()
            drive.screenshot(out / "E0.000-P0.000-rest.png", trim=True)
            log.append("rest E=%.4f P=%.4f" % (e, p))
            gdb.resume()
        try:
            for kind, target in targets:
                for _attempt in range(8):
                    gdb.pause()
                    gdb.write(slot + poll.STATE, struct.pack(">ii", 4, 4))
                    gdb.resume()
                    time.sleep(0.5)
                    pad.set("triangle", True)
                    time.sleep(0.12)
                    pad.set("triangle", False)
                    t0 = time.time()
                    peaked = False
                    hit = False
                    while time.time() - t0 < 3.0:
                        p, e = inputs()
                        peaked = peaked or e >= 0.999
                        if kind == "rise":
                            hit = e >= target and not peaked
                        elif kind == "hold":
                            hit = e >= 0.999
                        elif kind == "fall":
                            hit = peaked and 0 < e <= target
                        else:
                            hit = p >= target
                        if hit:
                            gdb.pause()
                            p, e = inputs()
                            name = "E%.3f-P%.3f-%s.png" % (e, p, kind)
                            drive.screenshot(out / name, trim=True)
                            gdb.resume()
                            log.append("%s target=%.2f -> %s" % (kind, target, name))
                            print(log[-1], flush=True)
                            break
                        time.sleep(0.004)
                    time.sleep(2.5)
                    if hit:
                        break
                else:
                    print("%s %.2f: not reached in 4 tries" % (kind, target), flush=True)
        finally:
            try:
                gdb.resume()
            except Exception:
                pass
    (out / "frames.txt").write_text("\n".join(log) + "\n")


if __name__ == "__main__":
    main()
