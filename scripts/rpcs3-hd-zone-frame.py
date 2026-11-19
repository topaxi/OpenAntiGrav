#!/usr/bin/env python3
"""HD Zone countdown frame on RPCS3: draw state, render targets, post-chain state and the sky dome's vertices.

Cold boots into a Racebox Zone race (the walk of `rpcs3-hd-hud-gate.py`), taps START RACE, waits for the
countdown (race manager phase 1) to run `--at` seconds, pauses and writes into `--out`:

- `frame-*.bin`: the frame's RSX command stream and fragment programs (`rpcs3_draw_hook.py`, light stages;
  read with `scripts/rsx-draw-list.py <out> frame`);
- `frame.png`: the presented frame of the same pause;
- `target-<offset>.bin` and `targets.json`: every colour target a draw of the frame names, `pitch * height`
  bytes each, with its surface format, pitch and clip size (the scene target before the resolve, the bloom
  ladder, the screen buffer);
- `mem.json` and `mem-<address>.bin`: the `FunkLayer` global (`0x00c50ee0`, 0x300 bytes: `+0x240` the ladder's
  mean colour, `+0x254` the adapted luma, `+0x25c` the frame's luma), the tone block (`0x008c3520`), the Zone
  parameter block (`0x00c81200..0x00c81600`), `g_ZoneEffectsActive` (`0x00d45f84`) and the race manager's phase;
- `small-draws.json` and `va-<draw>-<attribute>.bin`: the vertex arrays of every draw on the scene target with
  at most `--small` vertices (the gradient dome is one of them).

Run from the repository root under `scripts/emu-run.py` with a private env (`scripts/emu-env.sh`) and
`OAG_COMPARE_PLATFORM=hd-fury`. `docs/ghidra/functions/ps3-hdfury-eu/zone-frame.md`.
"""

import argparse
import importlib.util
import json
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_spec = importlib.util.spec_from_file_location("rpcs3_trace", HERE / "rpcs3-trace.py")
tr = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(tr)
drive = tr.drive
import rpcs3_draw_hook as hook  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

RACE_MANAGER_SLOT = 0x0095AE78
RING = (0x40000000, 0x20000)
MEMORY = {
    "funk_layer": (0x00C50EE0, 0x300),
    "tone_block": (0x008C3520, 0x80),
    "zone_params": (0x00C81200, 0x400),
    "zone_active": (0x00D45F84, 4),
    # The settings block the resolve reads: `+0x540` tone boost, `+0x544` clamp, `+0x548` maximum, `+0x54c` feedback.
    "settings_block": (0x00C489F0, 0x600),
}
FUNK_LAYER = 0x00C50EE0
SETTINGS = 0x00C489F0


def tone(guest):
    """`(adapted, luma, feedback float, boost, clamp, maximum, feedback, clamped product)` read with no pause."""
    f = struct.unpack(">8f", guest.read(FUNK_LAYER + 0x240, 0x20))
    t = struct.unpack(">4f", guest.read(SETTINGS + 0x540, 0x10))
    held = struct.unpack(">f", guest.read(0x008C3520 + 0x28, 4))[0]
    return {"adapted": f[5], "luma": f[7], "feedback_float": f[4], "boost": t[0], "clamp": t[1], "maximum": t[2],
            "feedback": t[3], "clamped_product": held, "exposure": t[2] - held}


def u32(b, o=0):
    return struct.unpack(">I", b[o:o + 4])[0]


def phase(guest):
    rm = u32(guest.read(RACE_MANAGER_SLOT, 4))
    if not rm:
        return None, 0.0
    blk = guest.read(rm + 0x1950, 0x28)
    return u32(blk, 0x20), struct.unpack(">f", blk[0x18:0x1C])[0]


def dump_frame(gdb, out, stem, attempts):
    for attempt in range(attempts):
        for old in out.glob("%s-*.bin" % stem):
            old.unlink()
        (out / ("%s-%08x.bin" % (stem, RING[0]))).write_bytes(gdb.read(*RING))
        hook.ST.update(stage=0, asked=set())
        retry = False
        for round_no in range(24):
            extra = hook.regions(str(out), stem, round_no)
            if extra is None:
                retry = True
                break
            if not extra:
                break
            for addr, size in extra:
                try:
                    (out / ("%s-%08x.bin" % (stem, addr))).write_bytes(gdb.read(addr, size))
                except Exception as error:  # noqa: BLE001
                    print("  dump %#x+%#x failed: %s" % (addr, size, error), flush=True)
        if not retry:
            return attempt
        print("  incomplete frame, retry %d" % attempt, flush=True)
        gdb.resume()
        time.sleep(0.5)
        gdb.pause()
    return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--rights", type=int, default=4,
                    help="RIGHT presses on RACE TYPE from Single Race: Time Trial, Speed Lap, Eliminator, Zone")
    ap.add_argument("--at", type=float, default=1.5, help="countdown seconds elapsed at the pause")
    ap.add_argument("--small", type=int, default=400)
    ap.add_argument("--after-go", type=float, default=0.0,
                    help="also read the tone state this many wall seconds after the release, with a frame")
    ap.add_argument("--boot-secs", type=float, default=400.0)
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    hook.LIGHT = True
    plan = {"Main Menu": ["right"], "Single Player": ["right"] * args.rights + ["cross"]}
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        (out / "rpcs3.pid").write_text("%d\n" % session.proc.pid)
        if session.guard is not None:
            session.guard.stage("boot", limit=args.boot_secs)
        if not session.wait_for_screen_pressing("Main Menu", 240):
            sys.exit("never reached the Main Menu")
        session.settle_menu(20)
        (out / "nav").mkdir(exist_ok=True)
        navs = [0]

        def race_type_crop():
            # The RACE TYPE value box on the Racebox screen (1280x720 trimmed frame).
            from PIL import Image
            probe = out / "nav" / "probe.png"
            drive.screenshot(probe, trim=True)
            return Image.open(probe).convert("L").resize((1280, 720)).crop((470, 125, 680, 147)).tobytes()

        def navigate(plan, session=session):
            # `rpcs3-hd-zone-palette.py`'s walk: a RIGHT on the race type is repeated until the box changes,
            # since a press on a loaded machine is dropped about one time in four.
            screen = drive.current_screen()
            for button in plan.get(screen, []):
                for _attempt in range(3):
                    before = race_type_crop() if screen == "Single Player" else None
                    if button in ("cross", "circle"):
                        session.press_once(button)
                    else:
                        session.tap(button, settle=1.5)
                    time.sleep(1.5)
                    navs[0] += 1
                    drive.screenshot(out / "nav" / ("nav-%02d.png" % navs[0]), trim=True)
                    print("    nav %-8s at %s" % (button, screen), flush=True)
                    if before is None or button != "right" or race_type_crop() != before:
                        break
                    print("    (race type unchanged, tapping again)", flush=True)
            plan.pop(screen, None)

        session.navigate = navigate
        if session.walk_to_race(plan=plan) not in drive.RACE_ARRIVED:
            sys.exit("no race")
        # The load prints its `GetMode()` line some seconds after the switch to `InGame`.
        modes = []
        for _ in range(90):
            modes = [line for line in drive.tty_text().splitlines() if "GetMode()" in line]
            if modes:
                break
            time.sleep(1.0)
        print("track: %s; mode lines: %s" % (drive.track_name(), modes[-2:]), flush=True)
        if not any("GetMode()==6" in line for line in modes[-1:]):
            sys.exit("not a Zone race: %s" % modes[-1:])
        session.wait_for_load(120)
        guest = tr.Guest(session.proc.pid, session.open_mem())
        time.sleep(4.0)
        session.tap("cross", settle=0.0)
        t = time.time()
        said = tapped = t
        first = None
        timeline = []
        while True:
            p, elapsed = phase(guest)
            now = time.time()
            if not timeline or now - timeline[-1]["wall"] >= 0.5:
                timeline.append({"wall": now, "phase": p, "timer": elapsed, **tone(guest)})
            if p == 1 and first is None:
                first = now
                print("countdown began, timer %.3f" % elapsed, flush=True)
            # The pause lands `--at` timer seconds in, or 3 wall seconds if the timer runs on another scale.
            if p == 1 and (elapsed >= args.at or now - first >= 3.0):
                break
            if p is not None and p >= 2:
                sys.exit("the countdown ended before the pause (phase %s)" % p)
            if now - t > 200:
                sys.exit("no countdown (phase %s, timer %.2f)" % (p, elapsed))
            if now - said >= 5.0:
                said = now
                print("  waiting: phase %s, timer %.3f" % (p, elapsed), flush=True)
            # A press on a loaded machine is dropped about one time in four: ask again while the grid waits.
            if p == 0 and now - tapped >= 8.0:
                tapped = now
                session.tap("cross", settle=0.0)
            time.sleep(0.02)
        gdb = Debugger(port=port)
        gdb.pause()
        p, elapsed = phase(guest)
        print("paused: phase %s, countdown %.3f s, tone %s" % (p, elapsed, tone(guest)), flush=True)
        used = dump_frame(gdb, out, "frame", 10)
        print("frame dump attempt %s" % used, flush=True)
        drive.screenshot(out / "frame.png", trim=True)

        def read(addr, size):
            try:
                blob = guest.read(addr, size)
                if len(blob) == size:
                    return blob
            except Exception as error:  # noqa: BLE001
                print("  /proc read %#x failed (%s), using gdb" % (addr, error), flush=True)
            return gdb.read(addr, size)

        mem = {"phase": p, "countdown_s": elapsed, "track": drive.track_name(), "modes": modes[-2:]}
        for name, (addr, size) in MEMORY.items():
            blob = read(addr, size)
            (out / ("mem-%08x.bin" % addr)).write_bytes(blob)
            mem[name] = "%#x+%#x" % (addr, size)
        (out / "mem.json").write_text(json.dumps(mem, indent=1))

        _m, _wk, draws = hook.draws_of(str(out), "frame")
        targets = {}
        for d in draws:
            off = d.get(0x210)
            if off is None or off in targets:
                continue
            pitch = d.get(0x20C, 0)
            height = (d.get(0x204, 0) >> 16) & 0xFFFF
            width = (d.get(0x200, 0) >> 16) & 0xFFFF
            targets[off] = {"offset": "%#x" % off, "pitch": pitch, "width": width, "height": height,
                            "format": "%#x" % d.get(0x208, 0), "context": "%#x" % d.get(0x194, 0)}
        for off, t in targets.items():
            size = min(t["pitch"] * max(t["height"], 1), 0x4000000)
            base = 0x40000000 if t["context"] == "0xfeed0001" else 0xC0000000
            try:
                (out / ("target-%08x.bin" % off)).write_bytes(read(base + off, size))
                t["bytes"] = size
            except Exception as error:  # noqa: BLE001
                t["error"] = str(error)
            print("target %s" % t, flush=True)
        (out / "targets.json").write_text(json.dumps(list(targets.values()), indent=1))

        scene = max(targets, key=lambda o: sum(1 for d in draws if d.get(0x210) == o))
        small = []
        for index, d in enumerate(draws):
            if d.get(0x210) != scene:
                continue
            arrays = d.get(0x1814)
            indexed = d.get(0x1824)
            spans = []
            if indexed is not None and d.get("batches"):
                ixoff = d.get(0x181C, 0)
                ix = []
                for first, count in d["batches"]:
                    blob = read(0x40000000 + ixoff + first * 2, count * 2)
                    ix += struct.unpack(">%dH" % count, blob)
                if not ix or len(ix) > args.small:
                    continue
                lo, n = min(ix), max(ix) - min(ix) + 1
                kind = {"indexed": ix}
            elif arrays is not None:
                lo, n = arrays & 0xFFFFFF, ((arrays >> 24) & 0xFF) + 1
                if n > args.small:
                    continue
                kind = {"first": lo, "count": n}
            else:
                continue
            for a in range(16):
                off, fmt = d.get(0x1680 + 4 * a), d.get(0x1740 + 4 * a)
                if not off or not fmt or (fmt & 0xF) == 0 or ((fmt >> 4) & 0xF) == 0:
                    continue
                stride = (fmt >> 8) & 0xFF
                if not stride:
                    continue
                base = 0x40000000 if off >> 31 else 0xC0000000
                o = off & 0x7FFFFFFF
                try:
                    (out / ("va-%04d-%02d.bin" % (index, a))).write_bytes(read(base + o + lo * stride, n * stride))
                    spans.append({"attribute": a, "format": "%#x" % fmt, "stride": stride, "offset": "%#x" % off})
                except Exception as error:  # noqa: BLE001
                    spans.append({"attribute": a, "error": str(error)})
            small.append({"draw": index, "mode": d.get(0x1808), "program": "%#x" % d.get(0x8E4, 0),
                          "low": lo, "vertices": n, "arrays": spans, **kind})
        (out / "small-draws.json").write_text(json.dumps(small, indent=1))
        print("scene target %#x, %d small draws" % (scene, len(small)), flush=True)
        gdb.resume()
        # The same readings through the release and `--after-go` seconds of the race, with a frame at the end
        # (the Zone craft flies itself), to tell a countdown state from a Zone-race one.
        t = time.time()
        released = None
        while args.after_go > 0 and time.time() - t < 240:
            p, elapsed = phase(guest)
            now = time.time()
            if now - timeline[-1]["wall"] >= 0.5:
                timeline.append({"wall": now, "phase": p, "timer": elapsed, **tone(guest)})
            if p is not None and p >= 2 and released is None:
                released = now
                print("released, tone %s" % tone(guest), flush=True)
            if released is not None and now - released >= args.after_go:
                break
            if now - said >= 5.0:
                said = now
                print("  racing wait: phase %s, tone %s" % (p, tone(guest)), flush=True)
            time.sleep(0.05)
        if released is not None:
            drive.screenshot(out / "after-go.png", trim=True)
            print("after GO: tone %s" % tone(guest), flush=True)
            for name in ("funk_layer", "settings_block", "tone_block"):
                addr, size = MEMORY[name]
                (out / ("after-go-mem-%08x.bin" % addr)).write_bytes(read(addr, size))
        (out / "timeline.json").write_text(json.dumps(timeline, indent=1))
    print("done", flush=True)


if __name__ == "__main__":
    main()
