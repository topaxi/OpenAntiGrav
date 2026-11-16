#!/usr/bin/env python3
"""Log every sampler the `wipeout_pc` native port builds, from the game's own S# (what Omega asks of the GPU), and
optionally strip the anisotropic filters to see what they draw.

    uv run --with evdev python3 scripts/emu-run.py --silence 200 --total 560 -- \\
        python3 scripts/wipeout-pc-sampler-log.py run --out data/scratch/<lane>/s --game <game dir> --port <port checkout> \\
        --track-steps 5 [--game-mode 2048|hd] [--strip]
    python3 scripts/wipeout-pc-sampler-log.py decode data/scratch/<lane>/s/port/port.log

The port's probe runs under `gdb` (Yama ptrace_scope 1 only allows a parent to attach, so the rig starts the probe through
`GOW3_PROBE`, a wrapper that execs gdb; the two query invocations `run.py` captures, `--cpu-features` and `--gamepad`,
bypass it). A silent breakpoint on `VideoCore::Sampler::Sampler` prints one `SAMP` line per unique sampler: the S#'s two
64-bit words, the `bool is_depth` argument and the port's extra LOD bias. `--strip` first rewrites each S#'s filters
`AnisoPoint/AnisoLinear` to `Point/Bilinear`, so the port's `anisotropyEnable` is off for that sampler (its own forced level is
0 unless `gow3.ini` sets `anisotropy`). The walk is `wipeout-pc-omega-pair.py`'s; the 2048 grid is found by tapping cross
and grabbing frames (the HUD detector needs reference masks made from a game frame), frames 3 to 5 are the grid before GO.
`docs/formats/omega-status.md`, "The port's anisotropy", has the results.
"""
import argparse
import collections
import importlib.util
import json
import os
import stat
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent

GDB = """set pagination off
set confirm off
set debuginfod enabled off
set breakpoint pending on
set startup-with-shell off
handle SIGSEGV SIGILL SIGBUS SIGUSR1 SIGUSR2 SIGALRM SIGPIPE SIGCHLD SIGURG nostop noprint pass
break VideoCore::Sampler::Sampler(Vulkan::Instance const&, AmdGpu::Sampler const&, AmdGpu::BorderColorBuffer, bool, float)
commands
silent
%s
printf "SAMP %%lx %%lx %%lx %%lx %%f\\n", *(unsigned long*)$rdx, *(unsigned long*)($rdx+8), $rcx, $r8, $xmm0.v4_float[0]
continue
end
run
"""
STRIP = """set $s1 = *(unsigned long*)($rdx+8)
set *(unsigned long*)($rdx+8) = ($s1 & ~(0xfUL << 20)) | ((($s1 >> 20) & 1UL) << 20) | ((($s1 >> 22) & 1UL) << 22)"""
WRAPPER = """#!/usr/bin/env bash
case "${1:-}" in --cpu-features|--gamepad) exec "%s" "$@" ;; esac
exec gdb -q -batch -x "%s" --args "%s" "$@"
"""

FILT = {0: "Point", 1: "Bilinear", 2: "AnisoPoint", 3: "AnisoLinear"}
MIP = {0: "None", 1: "Point", 2: "Linear"}
ANISO = {0: 1, 1: 2, 2: 4, 3: 8, 4: 16}


def bits(v, lo, n):
    return (v >> lo) & ((1 << n) - 1)


def decode_one(r0, r1, is_depth, extra_bias):
    """`AmdGpu::Sampler`'s fields (gpu/shadps4/video_core/amdgpu/resource.h of the port)."""
    bias = (bits(r1, 0, 14) ^ 0x2000) - 0x2000
    return {"mag": FILT[bits(r1, 20, 2)], "min": FILT[bits(r1, 22, 2)], "mip": MIP.get(bits(r1, 26, 2), "?"),
            "max_aniso": ANISO.get(bits(r0, 9, 3), "?"), "lod_bias": bias / 256.0, "depth": is_depth & 1, "extra_bias": extra_bias}


def decode(path):
    rows = []
    for line in open(path, errors="replace"):
        if line.startswith("SAMP "):
            _, r0, r1, _, r8, xb = line.split()
            rows.append(decode_one(int(r0, 16), int(r1, 16), int(r8, 16), float(xb)))
    count = collections.Counter((d["min"], d["mag"], d["mip"], d["max_aniso"], d["lod_bias"], d["extra_bias"], d["depth"]) for d in rows)
    print("%d samplers\nmin mag mip max_aniso lod_bias extra_bias is_depth : count" % len(rows))
    for key, n in sorted(count.items(), key=lambda kv: -kv[1]):
        print(*key, ":", n)
    return rows


def run(args):
    spec = importlib.util.spec_from_file_location("port_pair", HERE / "wipeout-pc-omega-pair.py")
    pp = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(pp)
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    port = Path(args.port).resolve()
    (out / "sampler.gdb").write_text(GDB % (STRIP if args.strip else ""))
    wrapper = out / "probe-gdb.sh"
    wrapper.write_text(WRAPPER % (port / "out/gow3-probe", out / "sampler.gdb", port / "out/gow3-probe"))
    wrapper.chmod(wrapper.stat().st_mode | stat.S_IXUSR)
    os.environ["GOW3_PROBE"] = str(wrapper)

    def grid_frames_2048(rig, a, out_dir):
        frames, t0, last = [], time.time(), None
        while time.time() - t0 < 25:
            if last is None or time.time() - last >= 2.5:
                rig.press("cross", 0.2)
                last = time.time()
            frames.append(rig.grab(out_dir / ("frame%02d.png" % len(frames))))
            time.sleep(1)
        return frames, {"taps": "every 2.5 s", "frames": len(frames)}

    pp.grid_frames_2048 = grid_frames_2048
    args.class_steps, args.load_wait, args.load_wait2 = 0, 12, 14
    rig = pp.Rig(out / "port", args.game, port, "sampler-log")
    if pp.pad_seat.own_pads(rig.seat):
        sys.exit("seat %s already has a pad" % rig.seat)
    note = {"track_steps": args.track_steps, "game_mode": args.game_mode, "strip": args.strip, "ini": dict(pp.GOW3_INI)}
    try:
        pick, seen = pp.capture_original(rig, args, out)
        note["grid"] = {"pick": str(pick), **{k: v for k, v in seen.items() if k != "scan_scores_labels_timer"}}
        time.sleep(3)
    finally:
        (out / "note.json").write_text(json.dumps(note, indent=1))
        rig.stop()
    decode(out / "port/port.log")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    d = sub.add_parser("decode")
    d.add_argument("log", type=Path)
    r = sub.add_parser("run")
    r.add_argument("--out", type=Path, required=True)
    r.add_argument("--game", required=True, help="the game folder (`wipeout-pc-omega-pair.py prepare-game`)")
    r.add_argument("--port", required=True, help="the port's checkout (built: out/gow3-probe)")
    r.add_argument("--track-steps", type=int, default=0)
    r.add_argument("--game-mode", choices=("hd", "2048"), default="2048")
    r.add_argument("--strip", action="store_true", help="rewrite Aniso* filters to Point/Bilinear before the port builds the sampler")
    args = ap.parse_args()
    decode(args.log) if args.cmd == "decode" else run(args)


if __name__ == "__main__":
    main()
