#!/usr/bin/env python3
"""HD/Fury's `vwipe` Image widgets in a running RPCS3: find them, and read when a page's wipe starts.

    source data/scratch/<lane>/env.sh ; export XDG_CACHE_HOME=$PWD/data/scratch/<lane>/cache
    OAG_COMPARE_PLATFORM=hd-fury python3 scripts/rpcs3-drive.py --image data/images/hdfury-ps3-eu.iso serve
    OAG_RPCS3_ATTACH=1 python3 scripts/rpcs3-hd-vwipe.py scan --out data/scratch/<lane>/widgets.json
    OAG_RPCS3_ATTACH=1 python3 scripts/rpcs3-hd-vwipe.py watch --widgets data/scratch/<lane>/widgets.json \\
        --page "Track Selection" --buttons cross --out data/scratch/<lane>/track-select.json

Every front-end page's widget tree is built at boot, so each `Image` widget (vtable word `0x866210`, `Image_Item.cpp`,
`docs/ghidra/functions/ps3-hdfury-eu/image-widget.md`) sits at one heap address for the whole session. `scan` walks the
heap for the vtable word and keeps the widgets whose transition mode (`+0x120`) has bit 2 (`vwipe`): their name
(`*(+0x78)`), the parent chain with each ancestor's inline name (`+0x58`), and the transition words `+0x68` (the
alpha `s`), `+0x6c` (in-time), `+0x70` (out-time), `+0x74` (authored `Delay`), `+0x9c` (the elapsed timer) and `+0xa0`
(the delay countdown).

`watch` polls chosen widgets (the page's, picked by `--page`: the ancestor name) without pausing, presses the buttons
and logs a row at every change of any watched widget's `(+0x68, +0x9c, +0xa0)` or of a parent's `+0xc4`, with the
wall-clock seconds since the first press. `Widget_UpdateTransition` runs once a game frame and moves `+0x9c` or `+0xa0`
by that frame's `dt`, so a row is one frame while a countdown or a ramp runs.
"""
import argparse
import importlib
import json
import struct
import sys
import threading
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
drive = importlib.import_module("rpcs3-drive")

GUEST = 0x300000000
HEAP = (0x30000000, 0x40000000)
BLOCK = 0x100000
IMAGE_VTABLE = 0x866210
VWIPE = 2
#: The words of a widget this tool reads: `+0x60 .. +0xa4`.
SPAN = (0x60, 0xA8)


def read(mem, address, size):
    mem.seek(GUEST + address)
    return mem.read(size)


def f32(word):
    return struct.unpack(">f", struct.pack(">I", word))[0]


def cstring(mem, address, size=48):
    try:
        raw = read(mem, address, size)
    except Exception:
        return None
    raw = raw.split(b"\0")[0]
    return raw.decode("latin-1") if raw and all(32 <= c < 127 for c in raw) else None


def widget_name(mem, address):
    """The widget's name: the pointer at `+0x78` (an `Image`'s), else the inline text at `+0x58` (a screen's)."""
    pointer = struct.unpack(">I", read(mem, address + 0x78, 4))[0]
    if HEAP[0] <= pointer < HEAP[1] or 0x00700000 <= pointer < 0x00900000:
        name = cstring(mem, pointer)
        if name:
            return name
    return cstring(mem, address + 0x58)


def find_images(mem):
    """Every heap address whose first word is the `Image` vtable."""
    hits = []
    for base in range(HEAP[0], HEAP[1], BLOCK):
        try:
            data = read(mem, base, BLOCK)
        except Exception:
            continue
        words = np.frombuffer(data[: len(data) & ~3], dtype=">u4")
        hits.extend(base + int(i) * 4 for i in np.nonzero(words == IMAGE_VTABLE)[0])
    return hits


def state(mem, address):
    raw = read(mem, address + SPAN[0], SPAN[1] - SPAN[0])
    w = struct.unpack(">%dI" % (len(raw) // 4), raw)

    def at(offset):
        return w[(offset - SPAN[0]) // 4]
    return {"parent": at(0x64), "s": f32(at(0x68)), "in": f32(at(0x6C)), "out": f32(at(0x70)), "delay": f32(at(0x74)),
            "timer": f32(at(0x9C)), "countdown": f32(at(0xA0))}


def chain(mem, address, limit=12):
    """Ancestor names from the widget's parent up."""
    names = []
    seen = set()
    parent = struct.unpack(">I", read(mem, address + 0x64, 4))[0]
    while parent and parent not in seen and len(names) < limit and HEAP[0] <= parent < HEAP[1]:
        seen.add(parent)
        names.append({"address": parent, "name": cstring(mem, parent + 0x58) or widget_name(mem, parent)})
        parent = struct.unpack(">I", read(mem, parent + 0x64, 4))[0]
    return names


def cmd_scan(args):
    info = drive.live_session()
    if info is None:
        sys.exit("no live `rpcs3-drive.py serve` (OAG_RPCS3_ATTACH=1, the lane's XDG_CACHE_HOME)")
    mem = drive.RemoteMem(info["socket"])
    rows = []
    for address in find_images(mem):
        mode = struct.unpack(">I", read(mem, address + 0x120, 4))[0]
        if not mode & VWIPE:
            continue
        row = {"address": address, "mode": mode, "name": widget_name(mem, address), "chain": chain(mem, address),
               **state(mem, address)}
        rows.append(row)
    Path(args.out).write_text(json.dumps(rows, indent=1) + "\n")
    for row in rows:
        print("%08x %-22s in %.2f delay %.3f s %.3f timer %.3f cd %.3f  %s" % (
            row["address"], row["name"], row["in"], row["delay"], row["s"], row["timer"], row["countdown"],
            " < ".join(str(c["name"]) for c in row["chain"][:3])))
    print("%d widgets with vwipe" % len(rows))


def pick(rows, page):
    """The widgets with an ancestor named `page`, deepest-name match."""
    return [r for r in rows if any(c["name"] and page.lower() in c["name"].lower() for c in r["chain"])]


def cmd_watch(args):
    info = drive.live_session()
    if info is None:
        sys.exit("no live `rpcs3-drive.py serve`")
    mem = drive.RemoteMem(info["socket"])
    pad = drive.RemotePad(info["socket"])
    rows = json.loads(Path(args.widgets).read_text())
    widgets = pick(rows, args.page)
    if not widgets:
        sys.exit("no vwipe widget under a page named %r" % args.page)
    parents = sorted({w["parent"] for w in widgets})
    tops = sorted({c["address"] for w in widgets for c in w["chain"] if args.page.lower() in (c["name"] or "").lower()})
    watched = {w["address"]: w["name"] for w in widgets}
    print("watching %d widgets: %s; parents %s; page nodes %s" % (
        len(widgets), ", ".join(sorted(set(str(w["name"]) for w in widgets))), [hex(p) for p in parents], [hex(t) for t in tops]), flush=True)
    log = []
    buttons = args.buttons.split(",") if args.buttons else []
    for button in (args.setup.split(",") if args.setup else []):
        pad.press(button, 0.15)
        time.sleep(args.settle)
    for run in range(args.runs):
        if run and args.back:
            for button in args.back.split(","):
                pad.press(button, 0.15)
                time.sleep(args.settle)
        time.sleep(args.rest)
        last = None
        t0 = time.time()
        presser = threading.Thread(target=lambda: [(pad.press(b, 0.15), time.sleep(args.gap)) for b in buttons])
        presser.start()
        while time.time() - t0 < args.seconds:
            snap = {}
            for address in watched:
                s = state(mem, address)
                snap[address] = (s["s"], s["timer"], s["countdown"])
            top = tuple(struct.unpack(">I", read(mem, t + 0xC4, 4))[0] for t in tops)
            key = (tuple(sorted(snap.items())), top)
            if key != last:
                last = key
                log.append({"run": run, "t": round(time.time() - t0, 4),
                            "widgets": {"%08x" % a: v for a, v in snap.items()}, "top_c4": list(top)})
        presser.join()
        print("run %d: %d rows" % (run, sum(1 for r in log if r["run"] == run)), flush=True)
    Path(args.out).write_text(json.dumps({"page": args.page, "names": {"%08x" % a: n for a, n in watched.items()},
                                          "tops": tops, "rows": log}, indent=1) + "\n")


def timeline(doc, widgets):
    """Per watched widget, from a `watch` log: the frame the countdown was armed (`+0xa0` jumps from at most 0 to the authored
    `Delay`), the frames the countdown ran, the frame the ramp started (`+0x9c` above 0) and ended (`+0x68` at 1), and
    the wall seconds of each. A frame is a row in which that widget's `(s, timer, countdown)` changed."""
    out = {}
    for address, name in doc["names"].items():
        mine = [(r["t"], tuple(r["widgets"][address])) for r in doc["rows"] if address in r["widgets"]]
        frames = [mine[0]] if mine else []
        for t, key in mine[1:]:
            if key != frames[-1][1]:
                frames.append((t, key))
        authored = next((w["delay"] for w in widgets if "%08x" % w["address"] == address), None)
        arm = next((i for i in range(len(frames)) if frames[i][1][2] > 0), None)
        if arm is None:
            out[address] = {"name": name, "armed": False}
            continue
        ramp = next((i for i in range(arm, len(frames)) if frames[i][1][1] > 0), None)
        end = next((i for i in range(arm, len(frames)) if frames[i][1][0] >= 1.0), None)
        dts = [frames[i][1][2] - frames[i + 1][1][2] for i in range(arm, (ramp or len(frames)) - 1) if frames[i + 1][1][2] > 0]
        out[address] = {"name": name, "armed": True, "authored_delay": authored, "first_countdown": round(frames[arm][1][2], 4),
                        "arm_t": frames[arm][0],
                        "frames_arm_to_ramp": None if ramp is None else ramp - arm,
                        "seconds_arm_to_ramp": None if ramp is None else round(frames[ramp][0] - frames[arm][0], 4),
                        "frames_ramp": None if ramp is None or end is None else end - ramp,
                        "seconds_ramp": None if ramp is None or end is None else round(frames[end][0] - frames[ramp][0], 4),
                        "dt_mean": round(sum(dts) / len(dts), 5) if dts else None, "frames": len(frames) - arm}
    return out


def cmd_analyze(args):
    doc = json.loads(Path(args.log).read_text())
    widgets = json.loads(Path(args.widgets).read_text())
    for run in sorted({r["run"] for r in doc["rows"]}):
        part = dict(doc, rows=[r for r in doc["rows"] if r["run"] == run])
        rows = part["rows"]
        flips = [rows[i]["t"] for i in range(1, len(rows)) if any(rows[i]["top_c4"]) and not any(rows[i - 1]["top_c4"])]
        print(run, "page present flag (+0xc4) rose at t =", flips)
        for address, row in timeline(part, widgets).items():
            print(run, address, json.dumps(row))


def cmd_stop(args):
    """Press `--buttons`, stop the game once the first `--page` widget's countdown reads at or below an aim, read the page's
    widgets in the stop and again 0.3 s later (equal: the game really stood still), resume. One row per aim."""
    from rpcs3_debugger import Debugger
    port = int(__import__("os").environ.get("OAG_RPCS3_GDB", "127.0.0.1:2345").rsplit(":", 1)[1])
    info = drive.live_session()
    if info is None:
        sys.exit("no live `rpcs3-drive.py serve`")
    mem = drive.RemoteMem(info["socket"])
    pad = drive.RemotePad(info["socket"])
    widgets = pick(json.loads(Path(args.widgets).read_text()), args.page)
    if not widgets:
        sys.exit("no vwipe widget under a page named %r" % args.page)
    probe = widgets[0]["address"]
    rows = []
    for aim in [float(a) for a in args.aims.split(",")]:
        if rows:
            for button in args.back.split(","):
                pad.press(button, 0.15)
                time.sleep(args.settle)
        time.sleep(args.rest)
        dbg = Debugger(port=port)
        dbg.resume()
        time.sleep(0.05)
        presser = threading.Thread(target=lambda: [pad.press(b, 0.15) for b in args.buttons.split(",")])
        presser.start()
        t0 = time.time()
        row = {"aim": aim}
        while time.time() - t0 < 4.0:
            if aim < 0:
                if state(mem, probe)["timer"] > 0:
                    break
            elif 0 < state(mem, probe)["countdown"] <= aim:
                break
        dbg.pause()
        row["stopped_after_s"] = round(time.time() - t0, 3)
        row["stop"] = [dict(name=w["name"], **{k: state(mem, w["address"])[k] for k in ("s", "timer", "countdown", "delay", "in")}) for w in widgets]
        time.sleep(0.3)
        row["again"] = [dict(name=w["name"], **{k: state(mem, w["address"])[k] for k in ("s", "timer", "countdown")}) for w in widgets]
        row["still"] = all(a["s"] == b["s"] and a["timer"] == b["timer"] and a["countdown"] == b["countdown"] for a, b in zip(row["stop"], row["again"]))
        dbg.resume()
        dbg.close()
        presser.join()
        time.sleep(1.5)
        rows.append(row)
        print(json.dumps(row), flush=True)
    Path(args.out).write_text(json.dumps(rows, indent=1) + "\n")


def cmd_roots(args):
    """Press buttons and log every change of the page roots' words `+0xb8 .. +0xd0` (which page is present)."""
    info = drive.live_session()
    if info is None:
        sys.exit("no live `rpcs3-drive.py serve`")
    mem = drive.RemoteMem(info["socket"])
    pad = drive.RemotePad(info["socket"])
    rows = json.loads(Path(args.widgets).read_text())
    roots = {}
    for r in rows:
        top = r["chain"][-1]
        roots[top["address"]] = top["name"]
    buttons = args.buttons.split(",") if args.buttons else []
    t0 = time.time()
    presser = threading.Thread(target=lambda: [(pad.press(b, 0.15), time.sleep(args.gap)) for b in buttons])
    presser.start()
    last = {}
    log = []
    while time.time() - t0 < args.seconds:
        for address, name in roots.items():
            words = struct.unpack(">8I", read(mem, address + 0xB8, 32))
            if last.get(address) != words:
                if address in last:
                    log.append({"t": round(time.time() - t0, 4), "root": name, "address": address,
                                "words": ["%08x" % w for w in words]})
                    print("%.3f %-26s %s" % (time.time() - t0, name, " ".join("%08x" % w for w in words)), flush=True)
                last[address] = words
    presser.join()
    Path(args.out).write_text(json.dumps(log, indent=1) + "\n")


def cmd_discover(args):
    """Press buttons and say which vwipe widgets move (`+0x68`, `+0x9c`, `+0xa0`): the page's own set. A poll of every
    widget takes about three frames, so this names the widgets and does not time them."""
    info = drive.live_session()
    if info is None:
        sys.exit("no live `rpcs3-drive.py serve`")
    mem = drive.RemoteMem(info["socket"])
    pad = drive.RemotePad(info["socket"])
    rows = json.loads(Path(args.widgets).read_text())
    buttons = args.buttons.split(",") if args.buttons else []
    presser = threading.Thread(target=lambda: [(pad.press(b, 0.15), time.sleep(args.gap)) for b in buttons])
    t0 = time.time()
    presser.start()
    moved = {}
    last = {}
    while time.time() - t0 < args.seconds:
        for r in rows:
            s = state(mem, r["address"])
            key = (s["s"], s["timer"], s["countdown"])
            if last.get(r["address"]) not in (None, key):
                moved.setdefault(r["address"], []).append((round(time.time() - t0, 3), key))
            last[r["address"]] = key
    presser.join()
    names = {r["address"]: (r["chain"][-1]["name"], r["name"]) for r in rows}
    for address, hits in moved.items():
        print("%08x %-26s %-20s %d changes, first %s last %s" % (address, names[address][0], names[address][1], len(hits), hits[0], hits[-1]))
    print("%d of %d widgets moved" % (len(moved), len(rows)))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("scan", help="list the vwipe Image widgets with their page chain and state")
    s.add_argument("--out", required=True)
    s.set_defaults(run=cmd_scan)
    w = sub.add_parser("watch", help="press buttons and log every change of a page's widgets, unpaused")
    w.add_argument("--widgets", required=True)
    w.add_argument("--page", required=True, help="a substring of an ancestor's name")
    w.add_argument("--buttons", default="")
    w.add_argument("--gap", type=float, default=0.3)
    w.add_argument("--setup", default="", help="buttons pressed once before the first run, `--settle` apart")
    w.add_argument("--back", default="", help="buttons pressed between runs to leave the page again")
    w.add_argument("--settle", type=float, default=3.5)
    w.add_argument("--seconds", type=float, default=4.0)
    w.add_argument("--rest", type=float, default=1.0)
    w.add_argument("--runs", type=int, default=1)
    w.add_argument("--out", required=True)
    w.set_defaults(run=cmd_watch)
    r = sub.add_parser("roots", help="log changes of every page root's present flags while buttons are pressed")
    r.add_argument("--widgets", required=True)
    r.add_argument("--buttons", default="")
    r.add_argument("--gap", type=float, default=0.3)
    r.add_argument("--seconds", type=float, default=6.0)
    r.add_argument("--out", required=True)
    r.set_defaults(run=cmd_roots)
    d = sub.add_parser("discover", help="which vwipe widgets move while buttons are pressed")
    d.add_argument("--widgets", required=True)
    d.add_argument("--buttons", default="")
    d.add_argument("--gap", type=float, default=0.3)
    d.add_argument("--seconds", type=float, default=6.0)
    d.set_defaults(run=cmd_discover)
    t = sub.add_parser("stop", help="stop the game at aimed countdowns after a button press and read the page's widgets")
    t.add_argument("--widgets", required=True)
    t.add_argument("--page", required=True)
    t.add_argument("--buttons", required=True)
    t.add_argument("--back", required=True)
    t.add_argument("--aims", default="0.6,0.3,0.1,-1", help="countdown seconds to stop at; -1 stops at the first ramp frame")
    t.add_argument("--settle", type=float, default=3.5)
    t.add_argument("--rest", type=float, default=1.0)
    t.add_argument("--out", required=True)
    t.set_defaults(run=cmd_stop)
    a = sub.add_parser("analyze", help="the arm, ramp start and end of each widget in a `watch` log")
    a.add_argument("--log", required=True)
    a.add_argument("--widgets", required=True)
    a.set_defaults(run=cmd_analyze)
    args = ap.parse_args()
    return args.run(args)


if __name__ == "__main__":
    sys.exit(main())
