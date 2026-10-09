#!/usr/bin/env python3
"""Count Wipeout HD's SCREAM master ticks per wall second at the Main Menu.

The evidence reproducer for `docs/ghidra/functions/ps3-hdfury-eu/sound.md`,
"The master tick is 240 Hz": boots the HD disc in RPCS3 on the virtual display,
waits for the Main Menu (the audio thread runs there too), attaches the GDB
stub and, per window, resumes for a stretch of wall clock, interrupts, and reads
the counters. One debugger session per launch (rpcs3_debugger.py, trap 2).

    OAG_RPCS3_DISPLAY=94 python3 scripts/rpcs3-drive.py display
    env -u WAYLAND_DISPLAY OAG_RPCS3_DISPLAY=94 \\
        uv run --with evdev python3 scripts/rpcs3-hd-sound-tick.py [window_s] [windows] [out_dir]

Reads (module B, TOC 0x008bd3c4): tick counter 0x013bc500 (u32, what
`*PTR_DAT_008c0148` points at), accumulator 0x013bc5cc (f32), pending count
0x013bc5b8, the tick step 0x0091ea60 (2.56f) and threshold 0x008c023c (1.0f),
the sound system flag +0xf90 through the slot at 0x008b4c5c, and the cellAudio
read index through the slot at 0x008bf958. The wall clock includes the pause
round trip, so the figure is a slight under-read; the default 10 s windows keep
that near 0.3 %.
"""
import importlib.util, struct, sys, time, json
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
spec = importlib.util.spec_from_file_location("rpcs3_drive", ROOT / "scripts" / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec); spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger

def u32(b, o=0): return struct.unpack_from(">I", b, o)[0]
def f32(b, o=0): return struct.unpack_from(">f", b, o)[0]

WINDOW = float(sys.argv[1]) if len(sys.argv) > 1 else 10.0
N = int(sys.argv[2]) if len(sys.argv) > 2 else 4
out = Path(sys.argv[3]) if len(sys.argv) > 3 else ROOT / "data" / "scratch" / "hd-sound-tick"
out.mkdir(parents=True, exist_ok=True)

def snap(d):
    tick = u32(d.read(0x013bc500, 4))
    pend = u32(d.read(0x013bc5b8, 4))
    acc = f32(d.read(0x013bc5cc, 4))
    ra = u32(d.read(0x008bf958, 4)); rp = u32(d.read(ra, 4)) if ra else 0
    ridx = d.read(rp, 8) if rp else b"\0" * 8
    return dict(tick=tick, pend=pend, acc=acc, ra=ra, rp=rp, ridx=ridx.hex())

with drive.Session(str(ROOT / "data/images/hdfury-ps3-eu-dec.iso"), str(out / "logs")) as s:
    print("pid", s.proc.pid, flush=True)
    if not s.wait_for_screen_pressing("Main Menu", 240):
        sys.exit("no main menu")
    print("main menu; settling", flush=True)
    time.sleep(20)
    d = Debugger()
    try:
        d.pause()
        step = d.read(0x0091ea60, 4); thr = d.read(0x008c023c, 4)
        print("step", f32(step), "thr", f32(thr))
        ss = u32(d.read(0x008b4c5c, 4)); print("g_sound_system slot->%08x" % ss)
        try:
            print("flag+0xf90 (via ptr):", d.read(ss + 0xf90, 1).hex())
        except Exception as e: print("flag read", e)
        for lbl, a in (("grain", 0x008bf980), ("grainflag", 0x008bf984)):
            p = u32(d.read(a, 4))
            try: print(lbl, "%08x" % p, u32(d.read(p, 4)))
            except Exception as e: print(lbl, e)
        rows = []
        prev = snap(d); print("s0", prev, flush=True)
        for i in range(N):
            t0 = time.time(); d.resume(); time.sleep(WINDOW); d.pause(); t1 = time.time()
            cur = snap(d)
            dt = t1 - t0; dn = (cur["tick"] - prev["tick"]) & 0xffffffff
            rows.append(dict(wall=dt, ticks=dn, hz=dn / dt, prev=prev, cur=cur))
            print("win %d: %d ticks in %.3f s wall = %.2f Hz  %s" % (i, dn, dt, dn / dt, cur), flush=True)
            prev = cur
        (out / "tick_windows.json").write_text(json.dumps(rows, indent=1))
    finally:
        try: d.resume()
        except Exception: pass
