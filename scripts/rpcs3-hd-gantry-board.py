#!/usr/bin/env python3
"""HD's start gantry board and Shield countdown, read from RAM beside the frame, on a live RPCS3.

    source data/scratch/<lane>/env.sh      # a `rpcs3-drive.py serve` on the weapons-on Talon's grid state
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-gantry-board.py --out DIR

Three phases in one race, written to `DIR/samples.json` with one PNG per sample:

- `parked`: the craft rests behind the line; at each lap clock the player's crossing counters
  (`ship+0x7810`, `+0x7814`), the race phase and the gantry nodes' animation time are read.
- `shield`: the Shield is raised and the pickup slot's remaining-time word (`+0x1c8`), flags
  (`+0x200`) and the HUD's own copy (`hud+0x1c4`) are read beside each frame.
- `cross`: `cross` is held and the same words are polled without pausing, to see when the board's
  time leaves the pre-lap window.

Addresses and laws: `docs/ghidra/functions/ps3-hdfury-eu/gantry-clock.md` and `hud-readouts.md`.
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


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


drive = load("rpcs3_drive", "rpcs3-drive.py")
pair = load("rpcs3_hd_pair", "rpcs3-hd-pair.py")
shield = load("rpcs3_hd_shield", "rpcs3-hd-shield.py")
probe = load("rpcs3_hud_probe", "rpcs3-hud-probe.py")
import compare_env  # noqa: E402
import rpcs3_hd_field  # noqa: E402
import rpcs3_place as place  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

CROSSINGS = 0x7810
CROSSINGS_2 = 0x7814
RACE_MANAGER_SLOT = 0x008A6814
RACE_PHASE = 0x1970
SLOT_TIMER = 0x1C8
#: The heap windows `rpcs3-hud-probe.py` finds the HUD's player record and the HUD object in.
RECORD_WINDOW = (0x307C0000, 0x30844000)
HUD_WINDOW = (0x32800000, 0x32A40000)
HUD_RECORD = 0x40
HUD_WIDGET = 0x1BC
HUD_TIMER = 0x1C4


def gantry_nodes(mem):
    """`{address: time}` of every `MeshImporter` whose time sits inside a race-manager window now."""
    found = {}
    for address, t, _ in rpcs3_hd_field.scenery_nodes(mem):
        if any(a <= t < b for a, b in rpcs3_hd_field.GANTRY_WINDOWS):
            found[address] = t
    return found


def find_hud(mem):
    """The HUD object whose `+0x40` is the player's record, or `None` (a three-lap race on a Feisar only)."""
    records = []
    for base in range(*RECORD_WINDOW, 0x800):
        blob = mem.rd(base, 0x860)
        records.extend(base + o for o in range(0, 0x800, 4) if probe.is_record(blob[o:o + 0x60]))
    for record in records:
        want = struct.pack(">I", record)
        for base in range(*HUD_WINDOW, 0x800):
            blob = mem.rd(base, 0x860)
            for o in range(0, 0x800, 4):
                if blob[o + HUD_RECORD:o + HUD_RECORD + 4] == want:
                    return base + o
    return None


class Board:
    def __init__(self, mem, ship, nodes):
        self.mem, self.ship, self.nodes = mem, ship, nodes
        self.manager = mem.u32(mem.u32(RACE_MANAGER_SLOT))

    def read(self):
        values = [self.mem.f32(a + rpcs3_hd_field.TIME_OFFSET) for a in self.nodes]
        return {
            "lap_clock": self.mem.f32(self.ship + shield.LAP_CLOCK),
            "crossings": self.mem.u32(self.ship + CROSSINGS),
            "crossings_2": self.mem.u32(self.ship + CROSSINGS_2),
            "race_phase": self.mem.u32(self.manager + RACE_PHASE),
            "gantry_s": sorted(set(round(v, 4) for v in values)),
        }


def shield_words(mem, live, hud):
    widget = mem.u32(hud + HUD_WIDGET) if hud else 0
    return {
        "slot_timer": mem.f32(live.slot + SLOT_TIMER),
        "slot_flags": mem.u32(live.slot + shield.FLAGS),
        "shield_time_s": live.read()["time_s"],
        "hud_widget": widget,
        "hud_widget_shown": bool(widget and mem.u32(widget + 0x34) & 4),
        "hud_timer": mem.f32(hud + HUD_TIMER) if hud else None,
    }


def pose_of(gdb, body):
    return [round(v, 3) for v in place.read_pose(gdb, body)["pos"]]


def wait_lap_clock(gdb, mem, ship, at):
    for _ in range(600):
        gdb.pause()
        clock = mem.f32(ship + shield.LAP_CLOCK)
        if clock >= at:
            return
        gdb.resume()
        time.sleep(max(0.02, min(0.5, (at - clock) / 0.8 - 0.05)))
    sys.exit("the lap clock stopped short of %.2f s" % at)


def grab(out, stem, record, samples):
    time.sleep(2.0)
    drive.screenshot(out / ("%s.png" % stem), trim=True)
    record["frame"] = "%s.png" % stem
    samples.append(record)
    (out / "samples.json").write_text(json.dumps(samples, indent=2) + "\n")
    print(json.dumps(record), flush=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--image", default=str(HERE.parent / "data/images/hdfury-ps3-eu-dec.iso"))
    ap.add_argument("--parked", default="4,10,18,27,40,55", help="lap-clock seconds of the parked samples")
    ap.add_argument("--ages", default="0.5,2.0,3.0,4.0", help="shield-clock seconds of the countdown samples")
    ap.add_argument("--drive-seconds", type=float, default=8.0)
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    pair.set_window(*compare_env.platform("hd-fury")["output"])
    samples = []
    parked = [float(v) for v in args.parked.split(",")]
    with drive.Session(args.image, str(out / "logs"), attach=True) as session:
        gdb = Debugger(port=port)
        mem = rpcs3_hd_field.Mem(session.open_mem())
        ship, body = shield.start_race(session, gdb, parked[0] - 1.0)
        wait_lap_clock(gdb, mem, ship, parked[0])
        mem.wr(rpcs3_hd_field.pilot_assist_at(mem), b"\x00\x00")
        nodes = gantry_nodes(mem)
        board = Board(mem, ship, sorted(nodes))
        print("ship %#x manager %#x gantry nodes %d" % (ship, board.manager, len(nodes)), flush=True)
        for n, at in enumerate(parked):
            wait_lap_clock(gdb, mem, ship, at)
            record = {"phase": "parked", **board.read(), "pos": pose_of(gdb, body)}
            grab(out, "parked%02d" % n, record, samples)
            gdb.resume()

        gdb.pause()
        live = shield.Live(gdb, mem, ship, body)
        hud = find_hud(mem)
        print("hud %s" % (hex(hud) if hud else None), flush=True)
        print("before the shield: %s" % json.dumps(shield_words(mem, live, hud)), flush=True)
        samples.append({"phase": "shield_before", **shield_words(mem, live, hud)})
        gdb.resume()
        shield.raise_shield(session, live, 0.5)
        for n, age in enumerate(float(v) for v in args.ages.split(",")):
            shield.wait_shield_age(live, age)
            record = {"phase": "shield", **board.read(), **shield_words(mem, live, hud)}
            grab(out, "shield%02d" % n, record, samples)
            gdb.resume()

        series = []
        session.pad.set("cross", True)
        start = time.time()
        crossed_at = None
        last = None
        while time.time() - start < args.drive_seconds:
            state = board.read()
            blob = mem.rd(body + place.B_POS, 12)
            state["pos"] = [round(v, 2) for v in struct.unpack(">3f", blob)]
            state["wall_s"] = round(time.time() - start, 3)
            series.append(state)
            if last is not None and state["crossings"] != last and crossed_at is None:
                crossed_at = len(series) - 1
                gdb.pause()
                grab(out, "crossed", {"phase": "cross_first", **board.read(), "pos": pose_of(gdb, body)}, samples)
                gdb.resume()
            last = state["crossings"]
            time.sleep(0.01)
        session.pad.set("cross", False)
        gdb.pause()
        grab(out, "after", {"phase": "after_cross", **board.read(), "pos": pose_of(gdb, body)}, samples)
        gdb.resume()
        (out / "cross.json").write_text(json.dumps({"crossed_index": crossed_at, "series": series}) + "\n")
        print("crossing sample index: %s of %d" % (crossed_at, len(series)), flush=True)


if __name__ == "__main__":
    main()
