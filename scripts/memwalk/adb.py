#!/usr/bin/env python3
# docs/architecture/memory.md, "A finished race held under its replay (ADR-0099 step 5)".
"""Run one walk inside the installed app on a phone and sample it over adb.

    python3 scripts/memwalk/adb.py pulse|hd OUT_DIR --serial SERIAL

The walk writes settings, records and a replay, so the app's `files/config` and
`files/data/oag/replays` are renamed to `*.keep` for the run and renamed back
after it. The app must not be running. OUT_DIR gets `samples.jsonl`,
`state.jsonl`, `walk.drive`, `logcat.txt`, `battery.txt`, the screenshots and one
`meminfo-<point>.txt` each. Read runs with `tables.py`.
"""

import argparse
import json
import re
import signal
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import points  # noqa: E402

PKG = "org.openantigrav.game"
FILES = f"/sdcard/Android/data/{PKG}/files"
PROFILE = (f"{FILES}/config", f"{FILES}/data/oag/replays")
WALK_FILES = (f"{FILES}/walk-shots", f"{FILES}/walk-empty", f"{FILES}/walk-state.jsonl",
              f"{FILES}/walk.drive", f"{FILES}/oag-args.txt")

parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
parser.add_argument("title", choices=sorted(points.IMAGES))
parser.add_argument("out")
parser.add_argument("--serial", required=True)
parser.add_argument("--walk", help="a walk file to run instead of walk.sh's")
args = parser.parse_args()


def adb(*command, check=True):
    return subprocess.run(["adb", "-s", args.serial, *command], check=check, capture_output=True,
                          text=True).stdout


def shell(command, check=True):
    return adb("shell", command, check=check)


def exists(path):
    return shell(f"test -e {path} && echo yes", check=False).strip() == "yes"


def running():
    return shell(f"pidof {PKG}", check=False).strip()


def set_profile_aside():
    """Renames the profile away. Returns the paths that did not exist before."""
    kept = [path for path in PROFILE if exists(path + ".keep")]
    if kept:
        raise SystemExit(f"{kept[0]}.keep is on the device: an earlier run did not restore it. "
                         "Move it back by hand before running again.")
    absent = [path for path in PROFILE if not exists(path)]
    for path in PROFILE:
        if path not in absent:
            shell(f"mv {path} {path}.keep")
    return absent


def restore_profile(absent):
    for path in PROFILE:
        if exists(path + ".keep"):
            shell(f"rm -rf {path} && mv {path}.keep {path}")
        elif path in absent:
            shell(f"rm -rf {path}")


def parse_meminfo(text):
    found = {}
    for line in text.splitlines():
        words = line.split()
        if words[:2] == ["Native", "Heap"] and len(words) >= 10:
            columns = ("pss", "private_dirty", "private_clean", "swap", "rss",
                       "heap_size", "heap_alloc", "heap_free")
            for name, value in zip(columns, words[2:]):
                found[f"native_{name}"] = int(value)
        m = re.match(r"\s*Graphics:\s+(\d+)", line)
        if m:
            found["graphics"] = int(m.group(1))
        # `dumpsys gpu --gpumem`, in bytes.
        m = re.match(r"Proc \d+ total: (\d+)", line)
        if m:
            found["gpumem"] = int(m.group(1)) // 1024
        for m in re.finditer(r"TOTAL (PSS|RSS|SWAP PSS):\s+(\d+)", line):
            found["total_" + m.group(1).lower().replace(" ", "_")] = int(m.group(2))
    return found


def parse_state(line):
    try:
        return json.loads(line)
    except ValueError:
        return {}


def probe(pid):
    tail = f"tail -n 1 {FILES}/walk-state.jsonl 2>/dev/null"
    return shell(
        f"{tail}; echo; echo @@; "
        f"grep -E 'VmRSS|VmHWM|RssAnon|RssFile|RssShmem|VmSwap' /proc/{pid}/status; echo @@; "
        f"dumpsys meminfo {pid}; dumpsys gpu --gpumem | grep -E '^Proc {pid} '; echo @@; {tail}",
        check=False)


def sample(pid, end):
    seen, dumps = {}, {}
    t0 = time.monotonic()
    changed = (None, t0)
    with open(out / "samples.jsonl", "w") as samples:
        while time.monotonic() - t0 < 900:
            parts = probe(pid).split("@@\n")
            if len(parts) < 4 or "VmRSS" not in parts[1]:
                if not running():
                    break
                time.sleep(0.2)
                continue
            before, after = parse_state(parts[0].strip()), parse_state(parts[3].strip())
            label = points.window(before, seen, end)
            # A dumpsys call takes a good part of a second: the sample counts
            # only if the point was the same before and after it.
            if label and points.window(after, {**seen, label: 0}, end) != label:
                label = None
            row = {"t": round(time.monotonic() - t0, 3), "window": label,
                   "tick_before": before.get("sim_tick"), "tick_after": after.get("sim_tick"),
                   "stage": before.get("stage"), "page": before.get("page")}
            for line in parts[1].splitlines():
                key, _, rest = line.partition(":")
                if rest.split():
                    row[key] = int(rest.split()[0])
            row.update(parse_meminfo(parts[2]))
            if label:
                seen.setdefault(label, row["t"])
                dumps[label] = parts[2]
            samples.write(json.dumps(row) + "\n")
            samples.flush()
            if "d" in seen and row["t"] - seen["d"] > 20:
                break
            # The walk has quit (or stalled) when its state log stops moving.
            line = parts[3].strip()
            if line != changed[0]:
                changed = (line, time.monotonic())
            elif time.monotonic() - changed[1] > 120:
                break
            time.sleep(0.05)
    for label, dump in dumps.items():
        (out / f"meminfo-{label}.txt").write_text(dump)
    return seen


walk = points.walk_text(args.title, args.walk)
end = points.end_tick(walk)
if running():
    raise SystemExit(f"{PKG} is running on {args.serial}: close it first")
out = points.new_directory(args.out)
walk = walk.replace("SHOTS", f"{FILES}/walk-shots")
(out / "walk.drive").write_text(walk)
# The `dist` build has no `--seed`.
launch = [f"{FILES}/data/oag/images/{points.IMAGES[args.title]}", "--no-audio",
          "--no-soundtrack", "--no-intro", "--autopilot", "--drive-file", f"{FILES}/walk.drive",
          "--state-log", f"{FILES}/walk-state.jsonl", f"OAG_MODS={FILES}/walk-empty",
          f"OAG_TRACKS={FILES}/walk-empty"]
(out / "oag-args.txt").write_text("\n".join(launch) + "\n")

signal.signal(signal.SIGTERM, lambda *_: sys.exit(143))
started = False
absent = set_profile_aside()
try:
    shell(f"rm -rf {' '.join(WALK_FILES)} && mkdir -p {FILES}/walk-shots {FILES}/walk-empty")
    adb("push", str(out / "walk.drive"), f"{FILES}/walk.drive")
    adb("push", str(out / "oag-args.txt"), f"{FILES}/oag-args.txt")
    adb("logcat", "-c")
    battery = shell("dumpsys battery")
    shell(f"am start -n {PKG}/android.app.NativeActivity")
    started = True
    pid = ""
    for _ in range(50):
        pid = running()
        if pid:
            break
        time.sleep(0.1)
    if not pid:
        raise SystemExit("the app did not start")
    print("pid", pid, flush=True)
    seen = sample(pid, end)
    adb("pull", f"{FILES}/walk-state.jsonl", str(out / "state.jsonl"), check=False)
    adb("pull", f"{FILES}/walk-shots", str(out / "shots"), check=False)
    (out / "logcat.txt").write_text(
        adb("logcat", "-d", "-s", "oag", "AndroidRuntime", "ActivityManager", check=False))
    (out / "battery.txt").write_text(battery + "\n---\n" + shell("dumpsys battery"))
finally:
    # Only the instance started here: one already running was refused above.
    if started:
        shell(f"am force-stop {PKG}", check=False)
    shell(f"rm -rf {' '.join(WALK_FILES)}", check=False)
    restore_profile(absent)
print(json.dumps({"windows": sorted(seen)}))
sys.exit(0 if {"a", "b", "c", "d"} <= set(seen) else 1)
