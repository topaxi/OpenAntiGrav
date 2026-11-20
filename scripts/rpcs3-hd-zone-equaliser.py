#!/usr/bin/env python3
"""HD Zone race on RPCS3: the Environment stage's 512 x 512 equaliser target and the band state behind it.

Cold boots into a Racebox Zone race (the walk of `rpcs3-hd-zone-frame.py`), taps START RACE, waits for the
release, then at each `--at` wall-clock offset after GO reads through /proc, with no pause, into `--out`:

- `target-<k>.bin`: the 512 x 512 `A8R8G8B8` target (`*(0x00c7dfb0 + 0x3378)`, 1 MiB);
- `block-<k>.bin`: the stage block `0x00c7dfb0 + 0x32b0 .. + 0x3380` (raw bands `+0x32b8`, held bands `+0x32f8`,
  the two floats `+0x3354`/`+0x3374`, the target pointers);
- `entries-<k>.bin`: the per-player stage entries `0x008c2cb8`, 0x78 bytes (`+0x00` stage, `+0x04` pending stage,
  `+0x18` blend weight, `+0x34` gain, `+0x74` the equaliser style);
- `vis-<k>.bin`: the `zoneTexVis` texels (`*(*(block + 0x32b0) + 0x10)`, 176 words);
- `eq-tints.bin`: the fifteen stages' `EQ colour tint` / `EQ analogue colour tint` words (`+0x1000 + n*0x250 + 0x150`);
- `frame-<k>.png`: the presented frame.

Run from the repository root under `scripts/emu-run.py` with a private env (`scripts/emu-env.sh`).
`docs/ghidra/functions/ps3-hdfury-eu/zone-equaliser.md`.
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
from rpcs3_debugger import Debugger  # noqa: E402

RACE_MANAGER_SLOT = 0x0095AE78
STAGE_BLOCK = 0x00C7DFB0
ENTRIES = 0x008C2CB8
SIDE = 512


def u32(b, o=0):
    return struct.unpack(">I", b[o:o + 4])[0]


def phase(guest):
    rm = u32(guest.read(RACE_MANAGER_SLOT, 4))
    if not rm:
        return None, 0.0
    blk = guest.read(rm + 0x1950, 0x28)
    return u32(blk, 0), struct.unpack(">f", blk[0x24:0x28])[0]


def grid_like(blob):
    """The target's own signature: a 2-pixel black line on each 16-pixel cell's right edge."""
    if len(blob) != SIDE * SIDE * 4:
        return False
    texel = lambda x, y: u32(blob, (y * SIDE + x) * 4)  # noqa: E731
    return texel(14, 3) == 0xFF000000 and texel(15, 3) == 0xFF000000 and texel(3, 14) == 0xFF000000


def find_data(guest, header):
    """The texel address in a texture header: the first word that addresses a grid-like 1 MiB."""
    for o in range(0, len(header), 4):
        w = u32(header, o)
        for addr in (w, 0xC0000000 + w):
            if not (0xC0000000 <= addr < 0xD0000000):
                continue
            try:
                if grid_like(guest.read(addr, SIDE * SIDE * 4)):
                    return addr, o
            except OSError:
                continue
    return None, None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--rights", type=int, default=4,
                    help="RIGHT presses on RACE TYPE from Single Race: Time Trial, Speed Lap, Eliminator, Zone")
    ap.add_argument("--at", default="0,1,2", help="wall seconds after the music starts to read at")
    ap.add_argument("--music-wait", type=float, default=150.0)
    ap.add_argument("--boot-secs", type=float, default=400.0)
    args = ap.parse_args()
    at = [float(s) for s in args.at.split(",")]
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    plan = {"Main Menu": ["right"], "Single Player": ["right"] * args.rights + ["cross"]}
    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        (out / "rpcs3.pid").write_text("%d\n" % session.proc.pid)
        if session.guard is not None:
            session.guard.stage("boot", limit=args.boot_secs)
        if not session.wait_for_screen_pressing("Main Menu", 240):
            sys.exit("never reached the Main Menu")
        session.settle_menu(20)

        def race_type_crop():
            from PIL import Image
            probe = out / "probe.png"
            drive.screenshot(probe, trim=True)
            return Image.open(probe).convert("L").resize((1280, 720)).crop((470, 125, 680, 147)).tobytes()

        def settled_crop():
            last = race_type_crop()
            for _ in range(8):
                time.sleep(0.6)
                now = race_type_crop()
                if now == last:
                    break
                last = now
            return last

        def navigate(plan, session=session):
            screen = drive.current_screen()
            for button in plan.get(screen, []):
                for _attempt in range(4):
                    before = settled_crop() if screen == "Single Player" and button == "right" else None
                    if button in ("cross", "circle"):
                        session.press_once(button)
                    else:
                        session.tap(button, settle=1.5)
                    time.sleep(1.5)
                    print("    nav %-8s at %s" % (button, screen), flush=True)
                    if before is None or settled_crop() != before:
                        break
                    print("    (race type unchanged, tapping again)", flush=True)
            plan.pop(screen, None)

        session.navigate = navigate
        if session.walk_to_race(plan=plan) not in drive.RACE_ARRIVED:
            sys.exit("no race")
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
        t = tapped = said = time.time()
        released = None
        while released is None:
            p, elapsed = phase(guest)
            now = time.time()
            if p is not None and p >= 2:
                released = now
                print("released (phase %s)" % p, flush=True)
                break
            if now - t > 300:
                sys.exit("no release (phase %s, timer %.2f)" % (p, elapsed))
            if now - said >= 5.0:
                said = now
                print("  waiting: phase %s, timer %.3f" % (p, elapsed), flush=True)
            if p == 0 and now - tapped >= 8.0:
                tapped = now
                session.tap("cross", settle=0.0)
            time.sleep(0.05)

        tints = b"".join(guest.read(STAGE_BLOCK + 0x1000 + n * 0x250 + 0x150, 8) for n in range(15))
        (out / "eq-tints.bin").write_bytes(tints)
        target_obj = u32(guest.read(STAGE_BLOCK + 0x3378, 4))
        view_obj = u32(guest.read(STAGE_BLOCK + 0x32B4, 4))
        header = guest.read(target_obj, 0x80)
        (out / "target-header.bin").write_bytes(header)
        (out / "view-header.bin").write_bytes(guest.read(view_obj, 0x80))
        data, field = find_data(guest, header)
        if data is None:
            data, field = find_data(guest, guest.read(view_obj, 0x80))
        meta = {"track": drive.track_name(), "modes": modes[-2:], "target_obj": "%#x" % target_obj,
                "view_obj": "%#x" % view_obj, "data": None if data is None else "%#x" % data,
                "data_field": field, "samples": []}
        print("target object %#x, view %#x, texels %s" % (target_obj, view_obj, meta["data"]), flush=True)
        # The header's third word is the texel address (`0xc5420000` on the first read).
        texels = u32(header, 8) if 0xC0000000 <= u32(header, 8) < 0xD0000000 else None
        meta["texels"] = None if texels is None else "%#x" % texels
        # The stub halts the emulator when it attaches (`rpcs3_debugger`): connected only at the first
        # target read, after the music wait, never during it.
        gdb = None
        # Music first: wait for a held band above 0.3 (up to `--music-wait` seconds), then read at the offsets.
        while time.time() - released < args.music_wait:
            held = struct.unpack(">16f", guest.read(STAGE_BLOCK + 0x32F8, 0x40))
            if max(held) > 0.3:
                break
            time.sleep(0.5)
        music = time.time() - released
        print("music at %.1f s after the release" % music, flush=True)
        for k, offset in enumerate(at):
            while time.time() - released < music + offset:
                time.sleep(0.02)
            stamp = time.time() - released
            if data is not None:
                (out / ("target-%d.bin" % k)).write_bytes(guest.read(data, SIDE * SIDE * 4))
            elif texels:
                # Not grid-like through /proc (a render target RPCS3 keeps on the GPU): the stub, paused.
                (out / ("target-proc-%d.bin" % k)).write_bytes(guest.read(texels, SIDE * SIDE * 4))
                if gdb is None:
                    gdb = Debugger(port=int(drive.GDB_SERVER.rsplit(":", 1)[1]))
                gdb.pause()
                try:
                    (out / ("target-gdb-%d.bin" % k)).write_bytes(gdb.read(texels, SIDE * SIDE * 4))
                except Exception as error:  # noqa: BLE001
                    print("  gdb read %#x failed: %s" % (texels, error), flush=True)
                gdb.resume()
            block = guest.read(STAGE_BLOCK + 0x32B0, 0xD0)
            (out / ("block-%d.bin" % k)).write_bytes(block)
            entries = guest.read(ENTRIES, 0x78)
            (out / ("entries-%d.bin" % k)).write_bytes(entries)
            vis = u32(guest.read(u32(block, 0) + 0x10, 4))
            (out / ("vis-%d.bin" % k)).write_bytes(guest.read(vis, 176 * 4) if vis else b"")
            drive.screenshot(out / ("frame-%d.png" % k), trim=True)
            sample = {"k": k, "wall_after_go": round(stamp, 3), "stage": u32(entries, 0),
                      "pending": u32(entries, 4), "style": u32(entries, 0x74),
                      "f3354": struct.unpack(">f", block[0x3354 - 0x32B0:][:4])[0],
                      "f3374": struct.unpack(">f", block[0x3374 - 0x32B0:][:4])[0],
                      "held": list(struct.unpack(">16f", block[0x48:0x88]))}
            meta["samples"].append(sample)
            print("sample %s" % sample, flush=True)
        (out / "meta.json").write_text(json.dumps(meta, indent=1))
    print("done", flush=True)


if __name__ == "__main__":
    main()
