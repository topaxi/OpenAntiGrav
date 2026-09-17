#!/usr/bin/env python3
"""One-shot, read-only: boot HD under the patched watchpoint RPCS3, walk to a
race on Talon's Junction, and dump slot 8's (321Go_StartFinish) per-node
`+0xe4` static UV override table for every one of its 19 instances - twice,
once right after load and once again mid-countdown, to see whether the
table's own *content* ever changes rather than only whether it gets applied.

No watchpoints armed here at all - this is strictly the "read `node+0xe4`
once" step `Billboard_UpdateInstanceUvs`'s own first pass (decompiled
2026-09-17, lane/hd-gantry-wire) calls for before chasing a writer:

    iVar2 = *(int *)(*(int *)(instance + 0x70) + 0xe4);      // node -> pointer
    if (iVar2 != 0) {
        // per-component override, -1 (0xffffffff) sentinel = "leave alone"
        uvScale.z  = *(float *)(iVar2 + 0x18);
        uvScale.w  = *(float *)(iVar2 + 0x1c);
        uvOffset.x = *(float *)(iVar2 + 0x20);
        uvOffset.y = *(float *)(iVar2 + 0x24);
    }

So `node+0xe4` is a *pointer*, not an inline table - this dumps both the
pointer and what it points at.

Lane: lane/hd-gantry-wire. Reuses scripts/rpcs3-drive.py and
scripts/rpcs3_debugger.py exactly as scratch/gantry_watch.py (the previous
lane's own driver) does; only the on-target logic after connecting differs.
Own display/port, distinct from any other lane's, checked empty at start.
"""
import importlib.util
import os
import struct
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SCRIPTS = REPO / "scripts"
sys.path.insert(0, str(SCRIPTS))

PATCHED_RPCS3 = "/home/topaxi/build/rpcs3-oag/build/bin/rpcs3"
DISPLAY_NUMBER = 91
GDB_PORT = 2351
IMAGE = str(REPO / "data" / "images" / "hdfury-ps3-eu-dec.iso")
SCRATCH = REPO / "scratch" / "gantry-e4-dump"
SCRATCH.mkdir(parents=True, exist_ok=True)

spec = importlib.util.spec_from_file_location("rpcs3_drive", SCRIPTS / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)

import rpcs3_debugger

drive.DISPLAY_NUMBER = DISPLAY_NUMBER
drive.DISPLAY = "127.0.0.1:%d" % DISPLAY_NUMBER
drive.DISPLAY_MARKER = str(SCRATCH / "xvfb.owner.json")

G_BILLBOARD_SLOTS_PTR = 0x008b6f38

TTY = drive.TTY


def tty_lines():
    try:
        with open(TTY, errors="replace") as fh:
            return fh.read().splitlines()
    except FileNotFoundError:
        return []


def wait_for_substring(sub, after_index=0, timeout=180.0, poll=1.0):
    deadline = time.time() + timeout
    while time.time() < deadline:
        lines = tty_lines()
        for i in range(after_index, len(lines)):
            if sub in lines[i]:
                return i, lines[i]
        time.sleep(poll)
    return None, None


def r32(dbg, addr):
    return int.from_bytes(dbg.read(addr, 4), "big")


def rf32(dbg, addr):
    return struct.unpack(">f", dbg.read(addr, 4))[0]


def dump_once(dbg, label):
    print("=== %s ===" % label, flush=True)
    slots_base = r32(dbg, G_BILLBOARD_SLOTS_PTR)
    slot_index = 7  # Num=8 -> 321Go_StartFinish
    piVar29 = slots_base + slot_index * 0x100 + 0x10
    resource = r32(dbg, piVar29)
    count = r32(dbg, resource + 0x1c)
    instance_base = r32(dbg, piVar29 + 0x3e * 4)
    print("  g_BillboardSlots=0x%08x resource=0x%08x count=%d instance_base=0x%08x"
          % (slots_base, resource, count, instance_base), flush=True)
    if not (0 < count <= 64) or instance_base == 0:
        raise RuntimeError("sanity check failed (count=%d, base=0x%x)" % (count, instance_base))

    for i in range(count):
        inst = instance_base + i * 0x90
        node = r32(dbg, inst + 0x70)
        # uvOffset/uvScale live at inst+0x50/+0x60 (billboards.md's own
        # "Billboard_LoadModelAndBind" section) - NOT inst+0x00, which is the
        # instance's identity matrix and reads as a plausible-looking but
        # wrong (1,0,0,0)/(0,1,0,0) pair if this offset is dropped, as a first
        # version of this script did.
        uv_off = [rf32(dbg, inst + 0x50 + 4 * k) for k in range(4)]
        uv_scale = [rf32(dbg, inst + 0x60 + 4 * k) for k in range(4)]
        if node == 0:
            print("  [%2d] inst=0x%08x node=0 (no bound .vex node) uvOffset=%s uvScale=%s"
                  % (i, inst, uv_off, uv_scale), flush=True)
            continue
        e4ptr = r32(dbg, node + 0xe4)
        if e4ptr == 0:
            print("  [%2d] inst=0x%08x node=0x%08x +0xe4=NULL (no override) "
                  "uvOffset=%s uvScale=%s"
                  % (i, inst, node, uv_off, uv_scale), flush=True)
            continue
        raw = [r32(dbg, e4ptr + 0x18 + 4 * k) for k in range(4)]
        as_f = [struct.unpack(">f", v.to_bytes(4, "big"))[0] for v in raw]
        sentinel = [v == 0xFFFFFFFF for v in raw]
        print("  [%2d] inst=0x%08x node=0x%08x +0xe4=0x%08x "
              "override(scale.z,scale.w,offset.x,offset.y)=%s sentinel=%s "
              "raw=%s | live uvOffset=%s uvScale=%s"
              % (i, inst, node, e4ptr, as_f, sentinel, [hex(v) for v in raw],
                 uv_off, uv_scale), flush=True)
    print("=== end %s ===\n" % label, flush=True)


def main():
    if drive.emulator_running():
        print("ABORT: an rpcs3 process is already running", file=sys.stderr)
        return 1

    shim_dir = SCRATCH / "bin"
    shim_dir.mkdir(exist_ok=True)
    shim = shim_dir / "rpcs3"
    if shim.exists() or shim.is_symlink():
        shim.unlink()
    shim.symlink_to(PATCHED_RPCS3)
    os.environ["PATH"] = str(shim_dir) + os.pathsep + os.environ["PATH"]

    my_config = SCRATCH / "config.yml"
    drive.scratch_config(path=str(my_config), interpreter=True)
    text = my_config.read_text()
    import re
    text, n = re.subn(r"(?m)^(\s*GDB Server:\s*127\.0\.0\.1:)\d+\s*$",
                       r"\g<1>%d" % GDB_PORT, text)
    assert n == 1
    my_config.write_text(text)

    time.sleep(3.0)
    session = drive.Session(image=IMAGE, log_dir=SCRATCH, config=str(my_config))
    open(TTY, "w").close()
    with session:
        print("rpcs3 pid %d on DISPLAY=%s" % (session.proc.pid, drive.DISPLAY), flush=True)
        if not session.wait_for_screen_pressing("Main Menu", timeout=180.0):
            print("never reached Main Menu", file=sys.stderr)
            return 1
        time.sleep(12.0)
        screen = session.walk_to_race()
        if screen not in drive.RACE_ARRIVED:
            print("ended on %r, not a race" % screen, file=sys.stderr)
            return 1

        idx, line = wait_for_substring("Loading track model", timeout=180.0)
        if idx is None:
            print("never saw 'Loading track model'", file=sys.stderr)
            return 1
        idx2, line2 = wait_for_substring("Loading Screen Finished", after_index=idx, timeout=180.0)
        if idx2 is None:
            print("never saw the post-track 'Loading Screen Finished'", file=sys.stderr)
            return 1
        print("track loaded: %s / %s" % (line, line2), flush=True)

        dbg = rpcs3_debugger.Debugger(port=GDB_PORT)
        try:
            dump_once(dbg, "immediately after load (pre-countdown)")
            dbg.resume()
            time.sleep(3.0)
            dbg.pause()
            dump_once(dbg, "t+3s into the countdown")
            dbg.resume()
            time.sleep(3.0)
            dbg.pause()
            dump_once(dbg, "t+6s into the countdown (around GO)")
            dbg.resume()
        finally:
            dbg.close()
        time.sleep(2.0)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
