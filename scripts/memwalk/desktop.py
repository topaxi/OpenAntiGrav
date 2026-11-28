#!/usr/bin/env python3
# docs/architecture/memory.md, "A finished race held under its replay (ADR-0099 step 5)".
"""Run one walk headless on this machine and sample the game's memory. Linux, glibc, amdgpu.

    python3 scripts/memwalk/desktop.py pulse|hd OUT_DIR --gpu 1002:747e

OUT_DIR gets the run's profile, `samples.jsonl`, `state.jsonl`, `mallinfo.jsonl`,
`walk.drive`, `game.log` and `result.json`. Read runs with `tables.py`.
"""

import argparse
import glob
import json
import os
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import points  # noqa: E402

sys.path.insert(0, str(points.HERE.parent))
import oag_tmp  # noqa: E402

parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
parser.add_argument("title", choices=sorted(points.IMAGES))
parser.add_argument("out")
parser.add_argument("--gpu", required=True, help="vendor:device for MESA_VK_DEVICE_SELECT")
parser.add_argument("--binary", default=str(oag_tmp.REPO / "target" / "debug" / "oag-game"))
parser.add_argument("--image", help="default: the title's image under data/images")
parser.add_argument("--walk", help="a walk file to run instead of walk.sh's")
args = parser.parse_args()

walk = points.walk_text(args.title, args.walk)
end = points.end_tick(walk)
image = args.image or str(oag_tmp.REPO / "data" / "images" / points.IMAGES[args.title])
out = points.new_directory(args.out)
profile = out / "profile"
for sub in ("c", "d", "s", "no-mods", "no-tracks", "shots"):
    (profile / sub).mkdir(parents=True)
state_log = out / "state.jsonl"
shim_log = out / "mallinfo.jsonl"
(out / "walk.drive").write_text(walk)


def status(pid):
    values = {}
    with open(f"/proc/{pid}/status") as f:
        for line in f:
            key, _, rest = line.partition(":")
            if key in ("VmRSS", "VmHWM", "RssAnon", "RssFile", "RssShmem"):
                values[key] = int(rest.split()[0])
    return values


def drm(pid):
    """VRAM and GTT in KiB, counted once per DRM client."""
    clients = {}
    for path in glob.glob(f"/proc/{pid}/fdinfo/*"):
        try:
            with open(path) as f:
                text = f.read()
        except OSError:
            continue
        if "drm-client-id" not in text:
            continue
        fields = dict(line.split(":", 1) for line in text.splitlines() if ":" in line)
        key = (fields.get("drm-pdev", "").strip(), fields["drm-client-id"].strip())
        clients[key] = tuple(int(fields.get(name, "0 KiB").split()[0])
                             for name in ("drm-memory-vram", "drm-memory-gtt"))
    return {"vram": sum(v for v, _ in clients.values()),
            "gtt": sum(g for _, g in clients.values())}


def mallinfo(child, label, sig, at):
    with open(shim_log, "a") as f:
        f.write(json.dumps({"window": label, "at": at, "sig": sig}) + "\n")
    child.send_signal(signal.SIGUSR2 if sig == "usr2" else signal.SIGUSR1)


def sample(child, t0):
    last, offset, seen, done = {}, 0, {}, set()
    trimmed = False
    gpu_next, gpu_now = 0.0, {}
    with open(out / "samples.jsonl", "w") as samples:
        while child.poll() is None:
            try:
                with open(state_log, "rb") as f:
                    f.seek(offset)
                    chunk = f.read()
                whole = chunk.rfind(b"\n") + 1
                for line in chunk[:whole].splitlines():
                    last = json.loads(line)
                offset += whole
            except FileNotFoundError:
                pass
            now = time.monotonic()
            try:
                row = status(child.pid)
                if now >= gpu_next:
                    gpu_now = drm(child.pid)
                    gpu_next = now + 0.25
            except (OSError, ProcessLookupError):
                break
            row.update(gpu_now)
            row["t"] = round(now - t0, 3)
            for key in ("stage", "page", "endrace", "replay", "sim_tick"):
                row[key] = last.get(key)
            label = points.window(last, seen, end)
            if label:
                seen.setdefault(label, now)
                held = now - seen[label]
                if held > 1.0 and label not in done:
                    done.add(label)
                    mallinfo(child, label, "usr1", row["t"])
                # Only the post-race menu is trimmed: a trim elsewhere would
                # change what the later points read.
                if label == "d" and held > 2.5 and not trimmed:
                    trimmed = True
                    mallinfo(child, "d_trim", "usr2", row["t"])
                if label == "d" and trimmed:
                    label = "d_trim" if held > 3.5 else None
            row["window"] = label
            samples.write(json.dumps(row) + "\n")
            time.sleep(0.02)
    return seen


# A SIGTERM (a `timeout` around the run) unwinds, so the game is not left running.
signal.signal(signal.SIGTERM, lambda *_: sys.exit(143))
with tempfile.TemporaryDirectory(dir=oag_tmp.temp_root()) as build:
    shim = os.path.join(build, "mallshim.so")
    subprocess.run(["gcc", "-O2", "-shared", "-fPIC", "-o", shim,
                    str(points.HERE / "mallshim.c"), "-lpthread"], check=True)
    env = dict(os.environ)
    env.pop("DISPLAY", None)
    env.pop("WAYLAND_DISPLAY", None)
    env.update(
        XDG_CONFIG_HOME=str(profile / "c"),
        XDG_DATA_HOME=str(profile / "d"),
        XDG_STATE_HOME=str(profile / "s"),
        OAG_MODS=str(profile / "no-mods"),
        OAG_TRACKS=str(profile / "no-tracks"),
        MESA_VK_DEVICE_SELECT=args.gpu + "!",
        LD_PRELOAD=shim,
        MALLSHIM_OUT=str(shim_log),
    )
    command = [args.binary, image, "--no-audio", "--no-soundtrack", "--no-intro", "--seed", "7",
               "--autopilot", "--drive", walk.replace("SHOTS", str(profile / "shots")),
               "--state-log", str(state_log)]
    load_start = os.getloadavg()
    with open(out / "game.log", "w") as log:
        t0 = time.monotonic()
        child = subprocess.Popen(command, env=env, stdout=log, stderr=log)
        print("pid", child.pid, flush=True)
        try:
            seen = sample(child, t0)
            child.wait()
        finally:
            if child.poll() is None:
                child.kill()
                child.wait()
result = {"rc": child.returncode, "seconds": round(time.monotonic() - t0, 1),
          "windows": sorted(seen), "load_start": load_start, "load_end": os.getloadavg()}
(out / "result.json").write_text(json.dumps(result) + "\n")
print(json.dumps(result))
sys.exit(1 if child.returncode else 0)
