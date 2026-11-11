#!/usr/bin/env python3
"""Walk a running Pulse PS2 PCSX2 from Racebox's Track Select to a race, and save the grid state a pair is made from.

    source <scratch>/ps2/env.sh                       # OAG_PCSX2_DISPLAY, OAG_PCSX2_DATA (own display and data path)
    uv run --with python-xlib python3 scripts/pcsx2-pulse-walk.py --slot S unlock
    uv run --with python-xlib python3 scripts/pcsx2-pulse-walk.py --slot S titles --out DIR
    uv run --with python-xlib python3 scripts/pcsx2-pulse-walk.py --slot S race --entry 3 \\
        --menu-slot 6 --name single-race-moa-therma-white-grid-widescreen --circuit "03_Track forward"

`unlock` writes 1 to the int at `g_settings + 0x46c`, the dev-unlock word `FUN_0017fa68` (PS2's
`Definition_IsUnlocked`) reads, so Track Select lists all 32 entries instead of 3
(`docs/ghidra/functions/ps2-pulse-eu/names.tsv`, `Definition_IsUnlocked`). Run it BEFORE entering Track Select: the
list is built on entry. `titles` steps through the list one `down` at a time, waits for the title crop to change
and settle, and writes `NN.png` crops plus `titles.json` (the list order read off the screen, never counted).
`race` loads the menu state (a savestate taken on Track Select, with the unlock on), presses `down` until the cursor word
in EE RAM (`TRACK_SELECT_CURSOR`) reads `--entry` minus one (a press is lost now and then, so it is read back), confirms
the track and the ship, waits for the race clock, sets the game's own `Aspect Ratio` option to 16:9
(`DAT_00284fe8 = 1`, `docs/ghidra/functions/ps2-pulse-eu/camera.md`), advances four frames and saves the state.
"""

import argparse
import hashlib
import json
import os
import shutil
import struct
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import pcsx2_state  # noqa: E402

G_SETTINGS_POINTER = 0x002DB350
UNLOCK_OFFSET = 0x46C
WIDESCREEN_OPTION = 0x00284FE8
CLOCK_ADDRESS = 0x70EE9C
#: Track Select's cursor, the 0-based entry (the screen's `n / 32` is this plus one): two words `0x48dfa0` and `0x48dfa4` held
#: 27, 28, 29 at three savestates the screen read 28, 29, 30 on (the only two words in EE RAM that did).
TRACK_SELECT_CURSOR = 0x48DFA0
TITLE_CROP = "700x200+1100+90"
COUNTER_CROP = "300x60+440+790"


def load_drive():
    import importlib.util
    spec = importlib.util.spec_from_file_location("pcsx2_drive", HERE / "pcsx2-drive.py")
    drive = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(drive)
    return drive


def clock(pine):
    return struct.unpack("<f", struct.pack("<I", pine.read32(CLOCK_ADDRESS)))[0]


def crop_title(full, dest):
    """The entry's title over the list counter (`n / 32`): White and Black titles differ by a few pixels, the counter by a digit."""
    subprocess.run(["magick", "(", str(full), "-crop", TITLE_CROP, "+repage", ")", "(", str(full), "-crop", COUNTER_CROP, "+repage",
                    "-gravity", "west", "-extent", "700x60", ")", "-append", "-colorspace", "Gray", "-resize", "50%", str(dest)],
                   check=True)


def differ(a, b):
    """RMSE of two crops, 0 when the files are the same picture."""
    result = subprocess.run(["magick", "compare", "-metric", "RMSE", str(a), str(b), "null:"], capture_output=True, text=True)
    text = result.stderr.strip().split(" ")[0]
    return float(text) if text else 1e9


def settle_title(drive, kb, workdir, previous, timeout=10.0):
    """Take crops until one differs from `previous`, then until two in a row agree (the title fades in)."""
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        full = workdir / "full.png"
        drive.gs_screenshot(kb, str(full))
        crop = workdir / "crop.png"
        crop_title(full, crop)
        changed = previous is None or differ(previous, crop) > 150
        if changed:
            if last is not None and differ(last, crop) < 100:
                return crop
            shutil.copy(crop, workdir / "last.png")
            last = workdir / "last.png"
        else:
            last = None
        time.sleep(0.4)
    return None


def cmd_unlock(args, drive, pine, kb):
    base = pine.read32(G_SETTINGS_POINTER)
    addr = base + UNLOCK_OFFSET
    before = pine.read32(addr)
    pine.write32(addr, 1)
    print("g_settings %#x, word %#x: %d -> %d" % (base, addr, before, pine.read32(addr)))


def cmd_titles(args, drive, pine, kb):
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    work = out / "work"
    work.mkdir(exist_ok=True)
    if args.from_slot is not None:
        pine.loadstate(args.from_slot)
        time.sleep(3.0)
    kb.focus()
    rows = []
    first = work / "first.png"
    drive.gs_screenshot(kb, str(work / "full.png"))
    crop_title(work / "full.png", first)
    shutil.copy(first, out / "00.png")
    rows.append({"index": 0, "file": "00.png"})
    previous = first
    for i in range(1, args.count):
        kb.press_button("down", hold=0.35, after=0.6)
        crop = settle_title(drive, kb, work, previous)
        if crop is None:
            print("entry %d: the title did not change after down (end of the list, or a repeat title)" % i)
            rows.append({"index": i, "file": None})
            continue
        name = "%02d.png" % i
        shutil.copy(crop, out / name)
        rows.append({"index": i, "file": name})
        previous = out / name
        print("entry", i, name, flush=True)
    (out / "titles.json").write_text(json.dumps(rows, indent=1) + "\n")
    subprocess.run(["magick", "montage"] + [str(out / r["file"]) for r in rows if r["file"]] +
                   ["-tile", "4x", "-geometry", "+2+2", str(out / "montage.png")], check=True)
    print("wrote", out / "montage.png")


def press(kb, name, hold=0.35):
    """A key press with the focus set first: a long load can drop it, and a press without it goes to the root window.
    A 0.35 s hold survives a loaded machine, where the VM runs slow and a short press falls between two frames, but on an idle one it
    auto-repeats into two steps: a menu step that must land takes `hold=0.08`."""
    kb.focus()
    kb.press_button(name, hold=hold, after=0.6)


def hud_visible(png):
    """True when the race HUD's mint clock hexagon is drawn (bottom left, 118,216,176 on a race frame)."""
    out = subprocess.run(["magick", str(png), "-crop", "40x40+100+950", "+repage", "-resize", "1x1", "-format",
                          "%[fx:int(255*r)],%[fx:int(255*g)],%[fx:int(255*b)]", "info:"], capture_output=True, text=True).stdout
    try:
        r, g, b = (int(x) for x in out.split(","))
    except ValueError:
        return False, None
    return 90 <= r <= 150 and 190 <= g <= 240 and 150 <= b <= 200, (r, g, b)


def wait_written(path, timeout=60.0):
    """Wait for a savestate file PINE is writing: it appears, grows, and is a readable zip when it is whole."""
    import zipfile
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        time.sleep(1.5)
        try:
            if path.exists() and "eeMemory.bin" in zipfile.ZipFile(path).namelist():
                return
        except (zipfile.BadZipFile, OSError):
            pass
    sys.exit("savestate %s never became a complete zip" % path)


def cmd_race(args, drive, pine, kb):
    state_dir = Path(drive.STATE_DIR)
    serial = args.serial
    if not drive.set_paused(pine, kb, False):
        sys.exit("could not run the VM before the load")
    pine.loadstate(args.menu_slot)
    time.sleep(3.0)
    if not drive.set_paused(pine, kb, False):
        sys.exit("could not run the VM after the load (the previous walk leaves it paused after `frames`)")
    work = Path(args.work)
    work.mkdir(parents=True, exist_ok=True)
    target = args.entry - 1
    for _ in range(80):
        have = pine.read32(TRACK_SELECT_CURSOR)
        if have == target:
            break
        press(kb, "down", hold=0.08 if (target - have) % 32 == 1 else 0.35)
        time.sleep(1.0)
        print("  cursor %d, wanted %d, vm %s" % (have, target, pine.status()), flush=True)
    else:
        sys.exit("Track Select cursor %d, wanted %d" % (pine.read32(TRACK_SELECT_CURSOR), target))
    time.sleep(1.5)
    drive.gs_screenshot(kb, str(work / "track-select.png"))
    press(kb, "cross")
    time.sleep(2.5)
    drive.gs_screenshot(kb, str(work / "ship-select.png"))
    press(kb, "cross")
    started = time.monotonic()
    live = None
    while time.monotonic() - started < args.load_timeout:
        time.sleep(4.0)
        (work / "poll.png").unlink(missing_ok=True)
        drive.gs_screenshot(kb, str(work / "poll.png"))
        ok, rgb = hud_visible(work / "poll.png")
        print("  %3.0f s: hud hexagon %s %s" % (time.monotonic() - started, "VISIBLE" if ok else "no", rgb), flush=True)
        if ok:
            live = time.monotonic() - started
            break
        # the circuit's description page waits for a button; cross is the thrust button once the race runs, so it is only
        # pressed while the HUD is not up
        press(kb, "cross")
    if live is None:
        sys.exit("the race HUD never appeared within %.0f s" % args.load_timeout)
    print("race HUD up after %.0f s real time" % live, flush=True)
    probe_slot, probe_slot_b = 5, 4
    probe = state_dir / ("%s.%02d.p2s" % (serial, probe_slot))
    probe_b = state_dir / ("%s.%02d.p2s" % (serial, probe_slot_b))
    clock_address = None
    for attempt in range(5):
        # two savestates a few seconds apart, the clock being the float that ran on between them (`race_clock_between`): the HUD is up
        # from the start of the countdown and some circuits count down for a while, so the search is repeated until it finds one
        time.sleep(14.0)
        saved = []
        for slot, path in ((probe_slot, probe), (probe_slot_b, probe_b)):
            if slot == probe_slot_b:
                if not drive.set_paused(pine, kb, False):
                    sys.exit("could not resume the VM")
                time.sleep(3.0)
            if not drive.set_paused(pine, kb, True):
                sys.exit("could not pause the VM")
            path.unlink(missing_ok=True)
            pine.savestate(slot)
            wait_written(path)
        clock_address = pcsx2_state.race_clock_between(probe, probe_b)
        found = pcsx2_state.player_craft(probe_b)
        if found is None:
            sys.exit("no player craft with a camera rig in the probe state %s" % probe_b)
        if clock_address is not None:
            break
        shutil.copy(probe, work / ("probe-no-clock-%d.p2s" % attempt))
        print("  probe %d: no single rising race clock yet, running on" % attempt, flush=True)
        if not drive.set_paused(pine, kb, False):
            sys.exit("could not resume the VM")
    if clock_address is None:
        sys.exit("no single word ran on as a race clock (4 s above its HUD partner) between two states")
    print("craft %#x, race clock at %#x" % (found[0], clock_address), flush=True)
    if not drive.set_paused(pine, kb, False):
        sys.exit("could not resume the VM")

    def read_clock():
        return struct.unpack("<f", struct.pack("<I", pine.read32(clock_address)))[0]

    while read_clock() < args.min_clock:
        time.sleep(2.0)
        print("  race clock %.2f" % read_clock(), flush=True)
    pine.write32(WIDESCREEN_OPTION, 1)
    subprocess.run(["uv", "run", "--with", "python-xlib", "python3", str(HERE / "pcsx2-drive.py"), "--slot", str(args.slot),
                    "frames", "4", "--window", "none"], check=True)
    src = state_dir / ("%s.%02d.p2s" % (serial, args.save_slot))
    src.unlink(missing_ok=True)
    pine.savestate(args.save_slot)
    wait_written(src)
    dest = Path(args.saves) / (args.name + ".p2s")
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy(src, dest)
    found = pcsx2_state.player_craft(dest)
    if found is None:
        sys.exit("no player craft in %s" % dest)
    craft, body, pos = found
    record = {"craft": "0x%x" % craft, "body": "0x%x" % body, "position": [round(x, 3) for x in pos],
              "circuit": args.circuit, "title": args.title_label, "widescreen": "DAT_00284fe8 = 1 over PINE, 4 frames advanced",
              "race_clock_address": "0x%x" % clock_address, "race_clock_ram_s": read_clock()}
    pcsx2_state.sidecar(dest).write_text(json.dumps(record, indent=2) + "\n")
    print("saved %s, craft %#x at (%.2f, %.2f, %.2f), race clock %.2f" % (dest, craft, *pos, read_clock()))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--slot", type=int, required=True, help="the instance's PINE slot")
    ap.add_argument("--serial", default="SCES-54748 (F8AE6FF2)")
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("unlock")
    p = sub.add_parser("titles")
    p.add_argument("--out", required=True)
    p.add_argument("--count", type=int, default=32)
    p.add_argument("--from-slot", type=int, help="load this savestate (Track Select) first")
    p = sub.add_parser("race")
    p.add_argument("--menu-slot", type=int, default=6)
    p.add_argument("--save-slot", type=int, default=7)
    p.add_argument("--entry", type=int, required=True, help="Track Select's entry, 1-based as the screen's counter reads it")
    p.add_argument("--name", required=True)
    p.add_argument("--circuit", required=True, help='what the state is, e.g. "03_Track forward"')
    p.add_argument("--title-label", default="")
    p.add_argument("--saves", default=str(HERE.parent / "data" / "saves" / "pulse-ps2"))
    p.add_argument("--work", required=True, help="a scratch directory for the walk's screenshots")
    p.add_argument("--load-timeout", type=float, default=420.0)
    p.add_argument("--min-clock", type=float, default=24.5, help="save once the RAM race clock is past this")
    args = ap.parse_args()
    if "OAG_PCSX2_DATA" not in os.environ or "OAG_PCSX2_DISPLAY" not in os.environ:
        sys.exit("source the lane's own env: OAG_PCSX2_DATA and OAG_PCSX2_DISPLAY must be set")
    drive = load_drive()
    pine = drive.Pine(slot=args.slot)
    kb = drive.Keyboard()
    {"unlock": cmd_unlock, "titles": cmd_titles, "race": cmd_race}[args.cmd](args, drive, pine, kb)


if __name__ == "__main__":
    main()
