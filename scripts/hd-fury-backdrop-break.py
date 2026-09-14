#!/usr/bin/env python3
"""Read the Fury menu backdrop's camera matrices out of a live RPCS3 frame.

`BackgroundAnimFury_Render` (`0x00183c88`) builds the clip's `worldView` and
`proj` on its stack: the path camera writes a view matrix at `sp+0x430`, the
perspective helper writes a projection at `sp+0x470`, and `0x00182358` is
then handed both in place. This build draws the arithmetic as read and comes
out too white and too close (menu-backdrop.md's census), with a reverted
diagnostic saying a `0.4` on the eye-to-point distance closes most of the
gap; `0x00182358` is 587 lines of VMX permutes that the decompiler does not
make readable, and it calls two getters that return `0.5` and `0.4` off
`0x008c3520`. So: stop before and after it and read the matrices, which
settles what it does without reading it.

Two breakpoints, both in `Render`:

- `BEFORE` `0x0018438c` - the `bl` to the perspective helper. `sp+0x430` is
  the camera's own matrix, untouched; `f1..f4` are its arguments (fovy in
  radians, aspect, near, far).
- `AFTER` `0x001843a0` - the instruction after `0x00182358` returns. Both
  matrices are final; the clip block (`item+0x1640`) still holds last
  frame's constants.

At each hit the script also reads `0x008c3520` (the getters' object), the
current cloud's header (`item+0x224`, whose `+0x3c` 4x4 the render multiplies
the view by), and takes a screenshot so the numbers pair with a picture.

**Needs `PPU Decoder: Interpreter (static)` in `config.yml`** - `Z0`
breakpoints are silently dead under the default `Recompiler (LLVM)`
([rpcs3-debugger.md](../docs/reverse-engineering/rpcs3-debugger.md#z0-breakpoints-fire-but-only-under-the-interpreter)).
The script edits that line itself for the run and puts it back on exit.

    uv run --with evdev python3 scripts/hd-fury-backdrop-break.py [out_dir]
"""

import importlib.util
import json
import re
import struct
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
spec = importlib.util.spec_from_file_location(
    "rpcs3_drive", ROOT / "scripts" / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger, REG_FPR, REG_PC  # noqa: E402

IMAGE = ROOT / "data/images/hdfury-ps3-eu-dec.iso"
CONFIG = Path.home() / ".config/rpcs3/config.yml"

BEFORE = 0x0018438C
AFTER = 0x001843A0
# `RadioHead2_Upload`: `r5` is the constant block `RadioHead2_Update` filled.
UPLOAD = 0x0026E598
TARGETS = {BEFORE: "before", AFTER: "after"}
GETTERS_OBJECT = 0x008C3520
FURY_SETTINGS = 0x0099F8A0
FRAMES = 12
INTERVAL = 4.0


def read_pc_hit(dbg, targets):
    for tid in dbg.threads():
        regs = dbg.registers(tid)
        if regs is None:
            continue
        pc = int.from_bytes(regs[REG_PC:REG_PC + 8], "big")
        if pc in targets:
            return tid, pc, regs
    return None, None, None


def stop_at(dbg, address, tries=20, slice_seconds=0.3):
    """Arm one breakpoint, run until a thread parks on it, disarm, and return
    that thread's `(tid, registers)` with the target stopped.

    One breakpoint at a time, so a parked thread is never resumed onto an
    armed address (the stub does not step over one) and there is never a
    second hit's stop reply queued behind the first. The stub may or may not
    announce the stop, so a short `wait_for_stop` is tried before the
    interrupt rather than instead of it.
    """
    dbg.add_breakpoint(address)
    try:
        for _ in range(tries):
            dbg.resume()
            if dbg.wait_for_stop(timeout=slice_seconds) is None:
                dbg.pause()
            dbg.drain()
            hit = read_pc_hit(dbg, {address: True})
            if hit[0] is not None:
                return hit[0], hit[2]
    finally:
        dbg.remove_breakpoint(address)
    return None, None


def gpr(regs, index):
    return int.from_bytes(regs[index * 8:index * 8 + 8], "big")


def fpr(regs, index):
    return struct.unpack(">d", regs[REG_FPR + index * 8:REG_FPR + index * 8 + 8])[0]


def matrix(blob):
    return [list(struct.unpack(">4f", blob[i:i + 16])) for i in range(0, 64, 16)]


def floats(blob):
    return list(struct.unpack(">%df" % (len(blob) // 4), blob))


def set_decoder(value):
    text = CONFIG.read_text()
    new, n = re.subn(r"^(  PPU Decoder: ).*$", r"\g<1>" + value, text, count=1, flags=re.M)
    if n != 1:
        raise SystemExit("config.yml: no `PPU Decoder:` line to edit")
    CONFIG.write_text(new)


def current_decoder():
    m = re.search(r"^  PPU Decoder: (.*)$", CONFIG.read_text(), re.M)
    return m.group(1) if m else None


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/hd-fury-backdrop-break")
    out.mkdir(parents=True, exist_ok=True)
    previous = current_decoder()
    set_decoder("Interpreter (static)")
    try:
        return run(out)
    finally:
        if previous is not None:
            set_decoder(previous)


def run(out):
    frames = []
    with drive.Session(str(IMAGE), str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen_pressing("Main Menu", 300.0):
            print("never reached the Main Menu (last screen: %s)"
                  % drive.current_screen(), file=sys.stderr)
            return 1
        print("Main Menu; letting the backdrop settle", flush=True)
        time.sleep(15.0)

        dbg = Debugger()
        try:
            dbg.pause()
            # The debug `Force Path` type: `PickPath` reads `g_FurySettings+0x418`
            # and, when it is 1 or more, takes it as the kind instead of
            # rolling one - 1 morph, 2 dynamic, anything else static. Set to
            # 3 so every clip from the next pick on is a static path, the
            # mode this build draws; the index still rolls.
            dbg.write(FURY_SETTINGS + 0x418, (3).to_bytes(4, "big"))
            print("forced the picker to static paths", flush=True)
            deadline = time.time() + 1000.0
            while time.time() < deadline and len(frames) < FRAMES:
                if frames:
                    dbg.run_for(INTERVAL)
                tid, regs = stop_at(dbg, BEFORE)
                if tid is None:
                    print("no thread reached BEFORE", flush=True)
                    break
                sp = gpr(regs, 1)
                item = gpr(regs, 30)
                frame = {
                    "tid": tid,
                    "sp": "%#x" % sp,
                    "item": "%#x" % item,
                    "fovy_rad": fpr(regs, 1),
                    "aspect": fpr(regs, 2),
                    "near": fpr(regs, 3),
                    "far": fpr(regs, 4),
                    "camera_before": matrix(dbg.read(sp + 0x430, 64)),
                }
                print("before: fovy %.5f aspect %.4f near %.4f far %.1f"
                      % (frame["fovy_rad"], frame["aspect"], frame["near"], frame["far"]),
                      flush=True)
                for row in frame["camera_before"]:
                    print("   ", ["%9.5f" % v for v in row], flush=True)
                tid, regs = stop_at(dbg, AFTER)
                if tid is None:
                    print("no thread reached AFTER", flush=True)
                    break
                sp = gpr(regs, 1)
                item = gpr(regs, 30)
                frame["world_view_after"] = matrix(dbg.read(sp + 0x430, 64))
                frame["proj_after"] = matrix(dbg.read(sp + 0x470, 64))
                frame["getters_object"] = floats(dbg.read(GETTERS_OBJECT, 0x40))
                pick = dbg.read(FURY_SETTINGS + 0x414, 0x14)
                frame["pick_force"] = int.from_bytes(pick[0:4], "big", signed=True)
                frame["pick_index"] = int.from_bytes(pick[0xc:0x10], "big")
                frame["pick_type"] = int.from_bytes(pick[0x10:0x14], "big")
                cloud = int.from_bytes(dbg.read(item + 0x224, 4), "big")
                frame["cloud"] = "%#x" % cloud
                if cloud:
                    head = dbg.read(cloud, 0x80)
                    frame["cloud_count"] = int.from_bytes(head[4:8], "big")
                    frame["cloud_bbox"] = floats(head[0x10:0x28])
                    frame["cloud_offset"] = floats(head[0x30:0x3c])
                    frame["cloud_matrix"] = matrix(head[0x3c:0x7c])
                # `AllocateTargets`' three targets and three views at `+0xfc..`:
                # dumped raw, to find their sizes without reading the texture
                # object's layout first.
                slots = dbg.read(item + 0xFC, 0x1C)
                frame["targets"] = {}
                for n in range(6):
                    at = int.from_bytes(slots[4 * n:4 * n + 4], "big")
                    if at:
                        frame["targets"]["%#x" % at] = dbg.read(at, 0x80).hex()
                clip = int.from_bytes(dbg.read(item + 0x1640, 4), "big")
                frame["clip"] = "%#x" % clip
                if clip:
                    block = dbg.read(clip, 0x150)
                    frame["clip_mode"] = int.from_bytes(block[0:4], "big")
                    frame["clip_frame"] = int.from_bytes(block[8:12], "big")
                    frame["clip_duration"] = floats(block[0x10:0x14])[0]
                    frame["clip_fps"] = floats(block[0x14:0x18])[0]
                    frame["clip_frames"] = int.from_bytes(block[0x18:0x1c], "big")
                    frame["clip_phase"] = floats(block[0x1c:0x20])[0]
                    frame["clip_seconds"] = floats(block[0x20:0x24])[0]
                    frame["clip_head"] = block[:0x50].hex()
                    frame["clip_tail"] = block[0x130:0x150].hex()
                    frame["clip_world_view_last"] = matrix(block[0x50:0x90])
                    frame["clip_proj_last"] = matrix(block[0x90:0xd0])
                    frame["clip_extra_last"] = matrix(block[0xd0:0x110])
                    frame["clip_particle_colour"] = floats(block[0x110:0x120])
                    frame["clip_colour_ramp"] = floats(block[0x120:0x130])
                    frame["clip_sprite_size"] = floats(block[0x24:0x28])[0]
                if frame.get("clip_mode") == 2:
                    tid, regs = stop_at(dbg, UPLOAD, tries=6)
                    if tid is not None:
                        block = dbg.read(gpr(regs, 5), 0x160)
                        frame["constants"] = {
                            "particle_colour": floats(block[0x50:0x60]),
                            "world_view": matrix(block[0x60:0xa0]),
                            "proj": matrix(block[0xa0:0xe0]),
                            "sprite_size": floats(block[0xe0:0xe4])[0],
                            "colour_ramp_factors": floats(block[0xf0:0x100]),
                            "colour_ramp_factors2": floats(block[0x100:0x110]),
                            "colour_ramp": floats(block[0x110:0x120]),
                            "depth_fade_factors": floats(block[0x120:0x130]),
                            "dof_factors": floats(block[0x130:0x140]),
                            "fog_factors": floats(block[0x140:0x150]),
                            "music_multiplier": floats(block[0x150:0x160]),
                        }
                        for k, v in frame["constants"].items():
                            if k not in ("world_view", "proj"):
                                print("  %s = %s" % (k, v), flush=True)
                stem = "%02d" % len(frames)
                drive.screenshot(out / ("%s.png" % stem))
                frame["screenshot"] = "%s.png" % stem
                frames.append(frame)
                print("after: worldView", flush=True)
                for row in frame["world_view_after"]:
                    print("   ", ["%9.5f" % v for v in row], flush=True)
                print("after: proj", flush=True)
                for row in frame["proj_after"]:
                    print("   ", ["%9.5f" % v for v in row], flush=True)
                print("clip: mode %s type %s index %s frame %s duration %s colour %s"
                      % (frame.get("clip_mode"), frame["pick_type"], frame["pick_index"],
                         frame.get("clip_frame"), frame.get("clip_duration"),
                         frame.get("clip_particle_colour")), flush=True)
                (out / "frames.json").write_text(json.dumps(frames, indent=1))
            dbg.resume()
        except (TimeoutError, OSError) as exc:
            print("stub desync after %d frame(s): %r" % (len(frames), exc),
                  file=sys.stderr, flush=True)
        finally:
            try:
                dbg.resume()
            except Exception:
                pass
            dbg.close()
    (out / "frames.json").write_text(json.dumps(frames, indent=1))
    print("done; %d frame(s) in %s" % (len(frames), out), flush=True)
    return 0 if frames else 1


if __name__ == "__main__":
    sys.exit(main())
