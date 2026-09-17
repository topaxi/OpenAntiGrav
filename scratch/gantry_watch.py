#!/usr/bin/env python3
"""One-shot: boot HD under the patched watchpoint RPCS3, walk to a race on
Talon's Junction, arm Z2 write watchpoints on slot 8's (321Go_StartFinish)
per-submesh uvOffset/uvScale instance fields, let the countdown play, and
report which addresses (if any) got written.

Lane: lane/hd-gantry-glyph-walk. Uses:
  - the patched RPCS3 at /home/topaxi/build/rpcs3-oag/build/bin/rpcs3
    (Z2/Z3 GDB watchpoints, verified working - see Z3-NOTES.md there and
    docs/reverse-engineering/rpcs3-debugger.md in this repo)
  - this repo's own scripts/rpcs3-drive.py (menu walk, TTY.log, config) and
    scripts/rpcs3_debugger.py (the GDB client) for everything else.

Own port 2345, own display :91 (per this lane's brief) - checked empty at
start; this script aborts rather than touching a display or process it
did not start itself.
"""
import importlib.util
import os
import re
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SCRIPTS = REPO / "scripts"
sys.path.insert(0, str(SCRIPTS))

PATCHED_RPCS3 = "/home/topaxi/build/rpcs3-oag/build/bin/rpcs3"
DISPLAY_NUMBER = 91
# Own port, distinct from the other lane's 2345/2346 - RPCS3 is a genuine
# single-instance singleton regardless (confirmed live: their AppRun.wrapped
# held 2345 while this port sat free), so a different port avoids a stale
# bind but does not itself allow running concurrently.
GDB_PORT = 2350
# 6s authored loop-close, generously padded for interpreter-mode slowdown
# (measured to run noticeably under real-time) plus whatever lead-in the
# grid/camera takes before the countdown itself starts.
WINDOW_SECONDS = 45.0
IMAGE = str(REPO / "data" / "images" / "hdfury-ps3-eu-dec.iso")
SCRATCH = REPO / "scratch" / "gantry-watch"
SCRATCH.mkdir(parents=True, exist_ok=True)

# --- import the hyphenated driver module by path ---------------------------
spec = importlib.util.spec_from_file_location("rpcs3_drive", SCRIPTS / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)

import rpcs3_pad
import rpcs3_debugger

# Retarget the driver module at our own, unclaimed display.
drive.DISPLAY_NUMBER = DISPLAY_NUMBER
drive.DISPLAY = "127.0.0.1:%d" % DISPLAY_NUMBER
drive.DISPLAY_MARKER = str(SCRATCH / "xvfb.owner.json")

# g_BillboardSlots's own pointer slot (see docs/ghidra/functions/ps3-hdfury-eu/billboards.md,
# "The name of the table base was wrong on this page"). Holds the per-slot
# table's own base address; NOT itself the table.
G_BILLBOARD_SLOTS_PTR = 0x008b6f38

TTY = drive.TTY


def tty_lines():
    try:
        with open(TTY, errors="replace") as fh:
            return fh.read().splitlines()
    except FileNotFoundError:
        return []


def wait_for_substring(sub, after_index=0, timeout=180.0, poll=1.0):
    """Index (in tty_lines()) of the first line containing `sub` at or after
    `after_index`, waiting for it to appear. Returns (index, line) or (None, None).
    """
    deadline = time.time() + timeout
    while time.time() < deadline:
        lines = tty_lines()
        for i in range(after_index, len(lines)):
            if sub in lines[i]:
                return i, lines[i]
        time.sleep(poll)
    return None, None


def _set_gdb_port(config_path, port):
    """Rewrite `Miscellaneous: GDB Server` in a generated config copy.

    `scratch_config()` only edits Audio Renderer and PPU Decoder; this is a
    second, narrower pass for the one extra key this lane's own port needs.
    """
    text = config_path.read_text()
    new_text, count = re.subn(
        r"(?m)^(\s*GDB Server:\s*127\.0\.0\.1:)\d+\s*$",
        r"\g<1>%d" % port,
        text,
    )
    if count != 1:
        raise SystemExit("%s: expected exactly one GDB Server line, found %d"
                          % (config_path, count))
    config_path.write_text(new_text)


def main():
    if drive.emulator_running():
        print("ABORT: an rpcs3 process is already running - not touching it",
              file=sys.stderr)
        return 1
    if drive.display_running():
        print("NOTE: Xvfb :%d already listening (reusing)" % DISPLAY_NUMBER)

    # Prepend a directory that resolves "rpcs3" to our patched binary, since
    # Session.__enter__ launches literally ["rpcs3", ...] via PATH lookup.
    shim_dir = SCRATCH / "bin"
    shim_dir.mkdir(exist_ok=True)
    shim = shim_dir / "rpcs3"
    if shim.exists() or shim.is_symlink():
        shim.unlink()
    shim.symlink_to(PATCHED_RPCS3)
    os.environ["PATH"] = str(shim_dir) + os.pathsep + os.environ["PATH"]

    my_config = SCRATCH / "config.yml"
    drive.scratch_config(path=str(my_config), interpreter=True)
    _set_gdb_port(my_config, GDB_PORT)
    print("config: %s (Interpreter (static), Null audio, GDB %d)"
          % (my_config, GDB_PORT))

    time.sleep(3.0)  # let a previous run's virtual pad device fully clear
    session = drive.Session(image=IMAGE, log_dir=SCRATCH, config=str(my_config))
    open(TTY, "w").close()  # fresh TTY.log so line-indices below are ours alone
    with session:
        print("rpcs3 pid %d (patched, watchpoints) on DISPLAY=%s"
              % (session.proc.pid, drive.DISPLAY), flush=True)
        if not session.wait_for_screen_pressing("Main Menu", timeout=180.0):
            print("never reached Main Menu (last: %s)" % drive.current_screen(),
                  file=sys.stderr)
            return 1
        print("Main Menu; settling", flush=True)
        time.sleep(12.0)

        print("walking to race", flush=True)
        screen = session.walk_to_race()
        if screen not in drive.RACE_ARRIVED:
            print("ended on %r, not a race" % screen, file=sys.stderr)
            return 1

        print("in race; waiting for track load", flush=True)
        idx, line = wait_for_substring("Loading track model", timeout=180.0)
        if idx is None:
            print("never saw 'Loading track model'", file=sys.stderr)
            return 1
        print("  %s" % line, flush=True)
        idx2, line2 = wait_for_substring("Loading Screen Finished", after_index=idx,
                                          timeout=180.0)
        if idx2 is None:
            print("never saw the post-track 'Loading Screen Finished'",
                  file=sys.stderr)
            return 1
        print("  %s" % line2, flush=True)

        # Connect immediately - this pauses the CPU, so no game-time elapses
        # while we compute addresses and arm watches below.
        print("connecting GDB stub (this pauses the target)", flush=True)
        dbg = rpcs3_debugger.Debugger(port=GDB_PORT)
        try:
            addr = _find_and_arm(dbg)
            print("armed %d write watchpoints; resuming for the countdown"
                  % len(addr), flush=True)
            dbg.resume()
            # Interpreter mode runs well under real-time (docs: "runs
            # perceptibly slower"), so the authored 6.000s loop-close can
            # take much longer than 6 wall-clock seconds. Poll RPCS3.log in
            # slices rather than one long sleep, so hits are timestamped
            # against wall clock as they arrive instead of only at the end.
            deadline = time.time() + WINDOW_SECONDS
            last_count = 0
            while time.time() < deadline:
                time.sleep(3.0)
                hits = [l for l in drive.rpcs3_log_text().splitlines()
                        if "watchpoint hit" in l.lower()]
                if len(hits) != last_count:
                    print("  [t=%.0fs] %d hit(s) so far" %
                          (time.time() - (deadline - WINDOW_SECONDS), len(hits)),
                          flush=True)
                    last_count = len(hits)
            dbg.pause()
            print("paused after countdown window", flush=True)
            _report_hits(SCRATCH / "rpcs3.log")
            print("--- post-window instance state ---")
            for inst in addr:  # addr entries are inst+0x50 (uvOffset base)
                off = [rf32(dbg, inst + 4 * i) for i in range(4)]
                scale = [rf32(dbg, inst + 0x10 + 4 * i) for i in range(4)]
                print("  inst+0x50=0x%08x uvOffset=%s uvScale=%s"
                      % (inst, off, scale))
            print("--- end ---")
            # Leave the game running rather than frozen-while-paused (a known
            # trap: disconnecting while paused freezes the emulator).
            dbg.resume()
        finally:
            dbg.close()
        time.sleep(2.0)
    return 0


def r32(dbg, addr):
    return int.from_bytes(dbg.read(addr, 4), "big")


def rf32(dbg, addr):
    import struct
    return struct.unpack(">f", dbg.read(addr, 4))[0]


def _find_and_arm(dbg):
    slots_base = r32(dbg, G_BILLBOARD_SLOTS_PTR)
    print("  g_BillboardSlots = 0x%08x" % slots_base, flush=True)
    slot_index = 7  # Num=8, 0-based -> 321Go_StartFinish, per billboards.md
    piVar29 = slots_base + slot_index * 0x100 + 0x10
    resource = r32(dbg, piVar29)  # *piVar29
    print("  slot8 struct @0x%08x, resource handle=0x%08x" % (piVar29, resource),
          flush=True)
    count = r32(dbg, resource + 0x1c)
    instance_base = r32(dbg, piVar29 + 0x3e * 4)
    print("  submesh count=%d, instance array base=0x%08x" % (count, instance_base),
          flush=True)
    if not (0 < count <= 64) or instance_base == 0:
        raise RuntimeError(
            "sanity check failed (count=%d, base=0x%x) - offsets need "
            "re-deriving against the live decompile, see billboards.md" %
            (count, instance_base))
    armed = []
    for i in range(count):
        inst = instance_base + i * 0x90
        uv_off = r32(dbg, inst + 0x50)
        uv_scale = r32(dbg, inst + 0x60)
        node = r32(dbg, inst + 0x70)
        print("    [%2d] inst=0x%08x uvOffset.x=%08x uvScale.x=%08x node=0x%08x"
              % (i, inst, uv_off, uv_scale, node), flush=True)
        reply = dbg.cmd("Z2,%x,%x" % (inst + 0x50, 0x20))
        if reply != "OK":
            raise RuntimeError("Z2 arm failed at 0x%x: %r" % (inst + 0x50, reply))
        armed.append(inst + 0x50)
    return armed


def _report_hits(rpcs3_log_path):
    text = drive.rpcs3_log_text()
    hits = [l for l in text.splitlines() if "watchpoint hit" in l.lower()]
    print("\n--- watchpoint hits in RPCS3.log ---")
    if not hits:
        print("  none")
    for h in hits:
        print("  " + h)
    print("--- end ---\n")


if __name__ == "__main__":
    raise SystemExit(main())
