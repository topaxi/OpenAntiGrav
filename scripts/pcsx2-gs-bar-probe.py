#!/usr/bin/env python3
"""Where PCSX2 puts a GS frame row and column: a synthetic dump of white bars at known coordinates, replayed and measured.

    pcsx2-gs-bar-probe.py make SRC.gs OUT.gs [--dy N] [--dx N] [--frac]
    pcsx2-gs-bar-probe.py replay DUMP.gs OUT.png [--gs KEY=VALUE ...]     # own pcsx2-qt, own display (OAG_PCSX2_DISPLAY, OAG_PCSX2_DATA)
    pcsx2-gs-bar-probe.py measure SHOT.png [--frac]

`make` keeps SRC's header, VRAM and privileged registers (the game's own `DISPLAY1/2`, `PMODE`, `SMODE2`) and replaces its packets
with one frame, drawn into the buffer `DISPFB1` reads: flat grey `(64, 64, 64)` over the whole 640x448 frame and white bars 8 rows tall at rows 20, 120, 220, 320, 410 (x 100
to 540) and 8 columns wide at columns 40, 320, 600 (y 60 to 100). `--frac` puts the bars at fractional 12.4 coordinates instead
(rows 120.25, 220.5, 320.75, 410.125; columns 200.25, 400.5, 600.75). `--dy`/`--dx` rewrite `DISPLAY1/2` before the replay.

`replay` is a fresh `pcsx2-qt -batch -nogui -- DUMP.gs` under `$OAG_PCSX2_DATA/../replay/` (the ini is the live instance's, PINE off,
`--gs KEY=VALUE` lines written into `[EmuCore/GS]`), one GS screenshot (F8), the process stopped by the pid it started. Run it under
`uv run --with python-xlib` and `scripts/emu-run.py`; `docs/reverse-engineering/emulator-recipes.md` has the display and data-path rules.

`measure` prints, per bar, the centroid of the bar's brightness in screenshot pixels against where a frame row or column `r` would
land at the screenshot's own scale with no display offset (`scale * (r + 4)`, the bar's centre), and the difference. A difference of
`+4.00` rows on a 2x screenshot is a picture 2.000 frame rows low. Result and reading: `docs/ps2/aspect-ratio.md`, "Measured against
PCSX2's frame".
"""

import argparse
import glob
import importlib.util
import os
import re
import shutil
import struct
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROWS_PLAIN, COLS_PLAIN = [20, 120, 220, 320, 410], [40, 320, 600]
ROWS_FRAC, COLS_FRAC = [20, 120.25, 220.5, 320.75, 410.125], [40, 200.25, 400.5, 600.75]
GREY = 64.0
FRAME_1, ZBUF_1, TEST_1, COLCLAMP, DTHE, TEX0_1, PRIM, RGBAQ, XYZ2, XYOFFSET_1, SCISSOR_1 = 0x4C, 0x4E, 0x47, 0x46, 0x45, 0x06, 0x00, 0x01, 0x05, 0x18, 0x40


def make(args):
    data = bytearray(open(args.src, "rb").read())
    header = struct.unpack_from("<I", data, 4)[0]
    state = struct.unpack_from("<I", data, 12)[0]
    priv = 8 + header + state
    start = priv + 8192
    for offset in (0x80, 0xA0):
        value = struct.unpack_from("<Q", data, priv + offset)[0]
        if args.dy is not None:
            value = (value & ~(0x7FF << 12)) | (args.dy << 12)
        if args.dx is not None:
            value = (value & ~0xFFF) | args.dx
        struct.pack_into("<Q", data, priv + offset, value)
    last_regs, i = None, start
    while i < len(data):
        kind = data[i]
        if kind == 0:
            i += 6 + struct.unpack_from("<I", data, i + 2)[0]
        elif kind == 1:
            i += 2
        elif kind == 2:
            i += 5
        else:
            last_regs = bytearray(data[i:i + 8193])
            i += 8193
    for offset in (0x70, 0x80, 0x90, 0xA0):  # the register packet carries the (patched) display registers too
        last_regs[1 + offset:9 + offset] = data[priv + offset:priv + offset + 8]
    fbp = struct.unpack_from("<Q", data, priv + 0x70)[0] & 0x1FF  # the frame buffer the display reads
    regs = [(FRAME_1, fbp | 10 << 16), (ZBUF_1, 0x100000118), (TEST_1, 0x30000), (COLCLAMP, 1), (DTHE, 0), (TEX0_1, 0), (XYOFFSET_1, 0),
            (SCISSOR_1, 639 << 16 | 447 << 48), (PRIM, 6)]

    def sprite(x0, y0, x1, y1, grey):
        regs.append((RGBAQ, grey | grey << 8 | grey << 16 | 0x80 << 24))
        regs.append((XYZ2, round(x0 * 16) | round(y0 * 16) << 16))
        regs.append((XYZ2, round(x1 * 16) | round(y1 * 16) << 16))

    sprite(0, 0, 640, 448, 64)
    rows, cols = (ROWS_FRAC, COLS_FRAC) if args.frac else (ROWS_PLAIN, COLS_PLAIN)
    for y in rows:
        sprite(100, y, 540, y + 8, 255)
    for x in cols:
        sprite(x, 60, x + 8, 100, 255)
    body = b"".join(struct.pack("<QQ", value, address) for address, value in regs)
    gif = struct.pack("<QQ", len(regs) | 1 << 15 | 1 << 60, 0xE) + body  # NLOOP, EOP, PACKED, NREG 1, REGS A+D
    transfer = bytes([0, 3]) + struct.pack("<I", len(gif)) + gif
    out = bytes(data[:start]) + transfer + bytes(last_regs) + bytes([1, 0]) + transfer + bytes(last_regs) + bytes([1, 1])
    Path(args.out).write_bytes(out)
    print("wrote %s (%d registers per frame)" % (args.out, len(regs)))


def replay(args):
    spec = importlib.util.spec_from_file_location("pcsx2_drive", HERE / "pcsx2-drive.py")
    drive = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(drive)
    live = Path(os.environ["OAG_PCSX2_DATA"])
    root = live.parent / "replay"
    shutil.rmtree(root, ignore_errors=True)
    (root / "PCSX2" / "inis").mkdir(parents=True)
    (root / "PCSX2" / "snaps").mkdir(parents=True)
    text = (live / "PCSX2" / "inis" / "PCSX2.ini").read_text().replace("EnablePINE = true", "EnablePINE = false")
    for item in args.gs:
        key, value = item.split("=", 1)
        pattern = r"^%s\s*=.*$" % re.escape(key)
        text = re.sub(pattern, "%s = %s" % (key, value), text, flags=re.M) if re.search(pattern, text, re.M) \
            else text.replace("[EmuCore/GS]\n", "[EmuCore/GS]\n%s = %s\n" % (key, value), 1)
    (root / "PCSX2" / "inis" / "PCSX2.ini").write_text(text)
    env = dict(os.environ, DISPLAY=drive.DISPLAY, QT_QPA_PLATFORM="xcb")
    env.pop("WAYLAND_DISPLAY", None)
    env.update(drive.pad_seat.sdl_blind_env())
    command = ["pcsx2-qt", "-datapath", str(root), "-batch", "-nogui", "-logfile", str(root / "log.txt"), "--", os.path.abspath(args.dump)]
    child = subprocess.Popen(command, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
    try:
        keyboard = None
        for _ in range(60):
            try:
                keyboard = drive.Keyboard()
                if keyboard.find_render_window():
                    break
            except Exception:
                pass
            time.sleep(0.5)
        if keyboard is None or keyboard.window is None:
            sys.exit("STALLED: no render window on %s" % drive.DISPLAY)
        time.sleep(args.wait)
        keyboard.focus()
        shots = []
        for _ in range(5):
            keyboard.tap("F8", after=0.5)
            for _ in range(60):
                shots = glob.glob(str(root / "PCSX2" / "snaps" / "*.png"))
                if shots and os.path.getsize(shots[0]) > 0:
                    break
                time.sleep(0.1)
            if shots:
                break
        if not shots:
            sys.exit("STALLED: no GS screenshot")
        time.sleep(0.5)
        shutil.copy(sorted(shots)[-1], args.out)
        print("wrote", args.out)
    finally:
        child.terminate()
        try:
            child.wait(10)
        except subprocess.TimeoutExpired:
            child.kill()


def measure(args):
    import numpy
    from PIL import Image
    image = numpy.asarray(Image.open(args.shot).convert("L"), dtype=float)
    height, width = image.shape
    sy, sx = height / 448.0, width / 640.0
    rows, cols = (ROWS_FRAC, COLS_FRAC) if args.frac else (ROWS_PLAIN, COLS_PLAIN)
    profile = image[:, int(150 * sx):int(500 * sx)].mean(axis=1)
    print("%s %dx%d" % (args.shot, width, height))
    for y in rows:
        centre = (y + 4) * sy
        lo, hi = int(centre - 20), int(centre + 20)
        seg = numpy.clip(profile[lo:hi] - GREY, 0, None)
        got = (numpy.arange(lo, hi) + 0.5) @ seg / seg.sum()
        print("  row %-8s expect %8.2f got %8.2f diff %+6.2f rows (%+.3f frame rows) width %.2f" % (y, centre, got, got - centre, (got - centre) / sy, seg.sum() / (255 - GREY)))
    strip = image[int(65 * sy):int(95 * sy), :].mean(axis=0)
    for x in cols:
        centre = (x + 4) * sx
        lo, hi = int(centre - 30), int(centre + 30)
        seg = numpy.clip(strip[lo:hi] - GREY, 0, None)
        got = (numpy.arange(lo, hi) + 0.5) @ seg / seg.sum()
        print("  col %-8s expect %8.2f got %8.2f diff %+6.2f px (%+.3f frame cols)" % (x, centre, got, got - centre, (got - centre) / sx))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("make")
    p.add_argument("src"), p.add_argument("out")
    p.add_argument("--dy", type=int), p.add_argument("--dx", type=int)
    p.add_argument("--frac", action="store_true")
    p.set_defaults(func=make)
    p = sub.add_parser("replay")
    p.add_argument("dump"), p.add_argument("out")
    p.add_argument("--gs", action="append", default=[], metavar="KEY=VALUE")
    p.add_argument("--wait", type=float, default=6.0)
    p.set_defaults(func=replay)
    p = sub.add_parser("measure")
    p.add_argument("shot")
    p.add_argument("--frac", action="store_true")
    p.set_defaults(func=measure)
    args = ap.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
