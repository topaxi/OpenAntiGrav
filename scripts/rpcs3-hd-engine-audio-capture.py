#!/usr/bin/env python3
"""Record HD's race audio from RPCS3's own audio dump, with a wall-clock marker track.

RPCS3's `Audio: Dump to file` writes everything the guest mixes to a WAV in the
cache dir (raw float32 stereo 48 kHz after 44 zero header bytes; the header is
never finalised). It writes nothing under the `Null` renderer, so the renderer
is `Cubeb`, pointed at a private PulseAudio null sink (`Audio Device` and
`PULSE_SINK`) so nothing reaches the speakers. The script checks that the
emulator's sink input is on that sink and aborts, killing only its own PID,
if it is anywhere else. This walks the default Fury campaign walk into a
race, then holds `cross` (full throttle) for a while and releases it, writing
`<out>/marks.json`: for each event, the wall clock and the dump's byte size at
that moment, which places the event on the WAV's own time axis.

    OAG_RPCS3_DISPLAY=91 XDG_CACHE_HOME=<dir> \\
      env -u WAYLAND_DISPLAY uv run --with evdev python3 \\
        scripts/rpcs3-hd-engine-audio-capture.py <out-dir> [--hold 20] [--wait 70]
"""
import argparse, importlib.util, json, os, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
spec = importlib.util.spec_from_file_location("rpcs3_drive", ROOT / "scripts" / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)

ap = argparse.ArgumentParser()
ap.add_argument("out")
ap.add_argument("--wait", type=float, default=70.0, help="seconds after arriving before throttle")
ap.add_argument("--rest", type=float, default=6.0, help="seconds of grid idle to record first")
ap.add_argument("--countdown", type=float, default=12.0, help="seconds of untouched grid after the skip tap")
ap.add_argument("--hold", type=float, default=25.0)
ap.add_argument("--coast", type=float, default=8.0)
ap.add_argument("--probe", action="store_true",
                help="also pause over the GDB stub at the phases below and read the xfade layer slots and SCREAM voices")
ap.add_argument("--park", action="store_true",
                help="after reaching the race, take commands from <out>/park.cmd (press|set|mark|stop) instead of the fixed phases")
ap.add_argument("--gdb-port", type=int, default=2341)
ap.add_argument("--image", default=str(ROOT / "data/images/hdfury-ps3-eu-dec.iso"))
args = ap.parse_args()
out = Path(args.out).resolve()
out.mkdir(parents=True, exist_ok=True)

cache = Path(os.environ["XDG_CACHE_HOME"]) / "rpcs3"


def dump_size():
    return sum(p.stat().st_size for p in cache.rglob("*.wav"))


SINK = "oag_hd_level"


def sink_inputs():
    return subprocess.run(["pactl", "list", "sink-inputs"], capture_output=True, text=True).stdout


def config_with_dump():
    path = drive.scratch_config(path=str(out / "config.yml"))
    text = Path(path).read_text()
    lines, section = [], None
    for line in text.splitlines(True):
        if line and not line[0].isspace() and line.rstrip().endswith(":"):
            section = line.rstrip()[:-1]
        if section == "Audio" and line.strip().startswith("Dump to file:"):
            line = "  Dump to file: true\n"
        if section == "Audio" and line.strip().startswith("Renderer:"):
            line = "  Renderer: Cubeb\n"
        if section == "Audio" and line.strip().startswith("Audio Device:"):
            line = '  Audio Device: "%s"\n' % SINK
        lines.append(line)
    Path(path).write_text("".join(lines))
    return path


marks = []
samples = []

TOC_XFADE = 0x008B5064      # holds the pointer whose +0x18 is the instance array
TOC_VOICES = 0x008C0100     # holds the base of the 0x18c-byte SCREAM voice array
TOC_HWSLOTS = 0x008C0044    # holds the base of the 100-byte hardware slot table


def u32(b, o=0):
    import struct
    return struct.unpack_from(">I", b, o)[0]


def i16(b, o=0):
    import struct
    return struct.unpack_from(">h", b, o)[0]


def i32(b, o=0):
    import struct
    return struct.unpack_from(">i", b, o)[0]


def read_big(d, address, size):
    out = b""
    while len(out) < size:
        out += d.read(address + len(out), min(512, size - len(out)))
    return out


def sample(d, label):
    """One pause: every xfade instance's layer slots, the voices they hold, their hardware slots."""
    d.pause()
    rec = dict(label=label, wall=time.time(), dump_bytes=dump_size(), instances=[])
    root = u32(d.read(TOC_XFADE, 4))
    inst_array = u32(d.read(root + 0x18, 4))
    voices = u32(d.read(TOC_VOICES, 4))
    hw = u32(d.read(TOC_HWSLOTS, 4))
    rec.update(root=root, inst_array=inst_array, voices=voices, hw=hw)
    for i in range(24):
        head = read_big(d, inst_array + i * 0x10, 0x10)
        xfx, chans, slots = u32(head, 0), u32(head, 4), u32(head, 8)
        if not xfx or u32(d.read(xfx, 4)) != 0x58464458:
            continue
        nch, nlay, layers_at = u32(d.read(xfx + 0xC, 4)), u32(d.read(xfx + 0x10, 4)), u32(d.read(xfx + 0x18, 4))
        inst = dict(index=i, xfx=xfx, channels=[], layers=[])
        chan = read_big(d, chans, nch * 0x50)
        for c in range(nch):
            o = c * 0x50
            inst["channels"].append(dict(value=(i32(chan, o) + i32(chan, o + 8) + i32(chan, o + 12)) >> 16))
        for n in range(nlay):
            lay = u32(d.read(layers_at + n * 4, 4))
            name = read_big(d, lay + 1, 16).split(b"\0")[0].decode("latin1")
            slot = read_big(d, slots + n * 0x80, 0x80)
            handle = u32(slot, 0x40)
            layer = dict(n=n, name=name, channel=read_big(d, lay + 0x14, 1)[0], slot_vol=i16(slot, 8), slot_vol_c=i16(slot, 0xC), handle=handle)
            if handle and handle != 0xFFFFFFFF and (handle >> 24) & 0x1F == 5:
                v = read_big(d, voices + ((handle >> 16) & 0xFF) * 0x18C, 0x18C)
                layer["voice"] = dict(first=u32(v, 0), v0c=i16(v, 0xC), v84=i16(v, 0x84), v86=i16(v, 0x86),
                                      v16=i16(v, 0x16), v12=i16(v, 0x12), v82=i16(v, 0x82), v9a=i16(v, 0x9A),
                                      v92=i16(v, 0x92), v90=i16(v, 0x90), mask=[u32(v, 0x24 + 4 * k) for k in range(4)])
                slots_hw = []
                for k in range(128):
                    if u32(v, 0x24 + 4 * (k >> 5)) >> (k & 31) & 1:
                        h = read_big(d, hw + k * 100, 100)
                        slots_hw.append(dict(slot=k, level=i32(h, 0x20), pitch=i32(h, 0x24), b58=h[0x58], h5a=i16(h, 0x5A), b1a=h[0x1A], raw=h.hex()))
                layer["hw"] = slots_hw
            inst["layers"].append(layer)
        rec["instances"].append(inst)
    d.resume()
    rec["wall_end"] = time.time()
    rec["dump_bytes_end"] = dump_size()
    samples.append(rec)
    print("sample", label, "instances", len(rec["instances"]), flush=True)


def mark(label):
    marks.append(dict(label=label, wall=time.time(), dump_bytes=dump_size()))
    print(marks[-1], flush=True)


cfg = config_with_dump()
module = subprocess.run(["pactl", "load-module", "module-null-sink", "sink_name=" + SINK],
                        capture_output=True, text=True, check=True).stdout.strip()
os.environ["PULSE_SINK"] = SINK
print("null sink module", module, flush=True)


def descendants(pid):
    """`pid` and every process below it, by walking /proc (own tree only)."""
    kids, found, todo = {}, {pid}, [pid]
    for entry in os.listdir("/proc"):
        if entry.isdigit():
            try:
                ppid = int(open("/proc/%s/stat" % entry).read().rsplit(")", 1)[1].split()[1])
            except (OSError, IndexError, ValueError):
                continue
            kids.setdefault(ppid, []).append(int(entry))
    while todo:
        for kid in kids.get(todo.pop(), []):
            if kid not in found:
                found.add(kid)
                todo.append(kid)
    return found


def check_sink(proc):
    """Abort (own PID only) if one of this emulator's audio streams is on any other sink.

    A stream is ours when its `application.process.id` is in our own process
    tree: another member's RPCS3 shows up in the same list and is none of our business.
    """
    mine_pids = descendants(proc.pid)
    text = sink_inputs()
    sink_id = subprocess.run(["pactl", "list", "short", "sinks"], capture_output=True, text=True).stdout
    mine = [l.split()[0] for l in sink_id.splitlines() if SINK in l]
    for b in text.split("Sink Input #")[1:]:
        pids = [int(l.split("=")[1].strip().strip('"')) for l in b.splitlines()
                if l.strip().startswith("application.process.id")]
        if not pids or pids[0] not in mine_pids:
            continue
        sink = [l.split(":")[1].strip() for l in b.splitlines() if l.strip().startswith("Sink:")]
        if not sink or sink[0] not in mine:
            proc.kill()
            raise SystemExit("rpcs3 audio is not on the null sink: killed pid %d" % proc.pid)
    return True


def watch_sink(proc):
    import threading

    def run():
        while proc.poll() is None:
            try:
                check_sink(proc)
            except SystemExit as exc:
                print(exc, flush=True)
                return
            time.sleep(1.0)

    threading.Thread(target=run, daemon=True).start()


try:
    with drive.Session(args.image, str(out / "logs"), config=cfg) as s:
        print("rpcs3 pid", s.proc.pid, flush=True)
        mark("boot")
        watch_sink(s.proc)
        if not s.wait_for_screen_pressing("Main Menu", 240):
            sys.exit("no main menu")
        mark("main-menu")
        time.sleep(20)
        screen = s.walk_to_race()
        mark("walk-ended-" + screen)
        if screen not in drive.RACE_ARRIVED:
            sys.exit("ended on %r" % screen)
        time.sleep(args.wait)
        if args.park:
            cmdfile = out / "park.cmd"
            cmdfile.write_text("")
            mark("parked")
            done = 0
            while True:
                lines = cmdfile.read_text().splitlines()
                for line in lines[done:]:
                    w = line.split()
                    if not w:
                        continue
                    if w[0] == "stop":
                        raise SystemExit("park: stop")
                    if w[0] == "press":
                        s.pad.press(w[1], float(w[2]))
                    elif w[0] == "set":
                        s.pad.set(w[1], w[2] == "1")
                    elif w[0] == "mark":
                        mark(w[1])
                    elif w[0] == "py":
                        # Run a file with the one GDB client in scope: the stub serves
                        # a single connection per emulator launch.
                        if "dbg" not in globals():
                            from rpcs3_debugger import Debugger
                            dbg = Debugger(port=args.gdb_port)
                        import io, contextlib, traceback
                        buf = io.StringIO()
                        with contextlib.redirect_stdout(buf):
                            try:
                                exec(open(w[1]).read(), dict(globals(), d=dbg))
                            except BaseException:
                                traceback.print_exc(file=buf)
                        Path(w[1] + ".out").write_text(buf.getvalue())
                done = len(lines)
                time.sleep(0.5)
        mark("flyover-idle-start")
        drive.screenshot(out / "shot-flyover.png", trim=True)
        time.sleep(args.rest)
        # The race opens on a fly-over with a START RACE prompt; a tap skips it
        # and the countdown follows. Nothing is held until the hold phase.
        s.pad.press("cross", 0.15)
        mark("tap-skip")
        if args.probe:
            from rpcs3_debugger import Debugger
            dbg = Debugger(port=args.gdb_port)
            time.sleep(3.5)
            sample(dbg, "countdown")
            time.sleep(max(0.0, args.countdown - 3.5 - 4.0))
            sample(dbg, "grid-after-go")
        else:
            time.sleep(args.countdown)
        mark("countdown-done")
        drive.screenshot(out / "shot-grid.png", trim=True)
        s.pad.set("cross", True)
        mark("throttle-on")
        if args.probe:
            for k, wait in enumerate((4.0, 6.0, 6.0)):
                time.sleep(wait)
                sample(dbg, "thrust-%d" % k)
            time.sleep(max(0.0, args.hold - 16.0))
        else:
            time.sleep(args.hold)
        drive.screenshot(out / "shot-climb.png", trim=True)
        s.pad.set("cross", False)
        mark("throttle-off")
        time.sleep(args.coast)
        mark("end")
finally:
    subprocess.run(["pactl", "unload-module", module])
    (out / "marks.json").write_text(json.dumps(marks, indent=1))
    if samples:
        (out / "samples.json").write_text(json.dumps(samples, indent=1))
