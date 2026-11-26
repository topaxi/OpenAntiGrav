#!/usr/bin/env python3
"""Paused frames of a Detonator race on RPCS3: frame, camera, pose, clocks, optionally with the cannon held.

    source data/scratch/<lane>/env.sh
    OAG_COMPARE_PLATFORM=hd-fury OAG_RPCS3_ATTACH=1 UV_CACHE_DIR=~/.cache/uv UV_OFFLINE=1 \\
    uv run --with evdev python3 scripts/emu-run.py --child-status <log-dir>/status.json -- \\
        python3 scripts/rpcs3-hd-detonator-shots.py --out data/scratch/<lane>/shots \\
            --shot 6.0 --fire-clock 9.0 --fire-hold 0.5 --shot 9.4

Attached to a `serve` restored from a Detonator Racebox grid state (`scripts/emu-restore-state.sh`, Racebox
RACE TYPE DETONATOR, `GetMode()==14`), this taps START RACE until the lap clock runs, then plays the race on:
for every `--shot T` it pauses at the first stop whose lap clock (`ship+0x7014`) is at or past T and keeps the
frame (`rpcs3-drive.py grab_with_clock`, the world clock beside it), the camera read from the RSX pushbuffer
and the player's pose, to `OUT/NN.png` and `OUT/NN.json` in the shape `rpcs3-drive.py capture` writes (so
`rpcs3-hd-pair.py --json OUT/NN.json` takes it). `--fire-clock F --fire-hold H` holds a button (`--fire-button`, default TRIANGLE, which fired once; SQUARE changed the camera view on four boots) to fire the pulse cannon
for H wall seconds from the first stop at or past lap clock F; the record carries the clocks before and after
the hold, so ours runs the same ticks with the same button held. The Detonator craft drives itself (auto-speed):
no throttle is pressed.

Each shot is taken twice, the second `--interval` seconds later, because the camera pick compares two frames
(`ps3_pose.pick_camera`); the first is the one kept. `docs/ghidra/functions/ps3-hdfury-eu/detonator-mode.md`.
"""
import argparse
import importlib.util
import json
import os
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)
import compare_env  # noqa: E402
import ps3_pose  # noqa: E402
import rpcs3_hd_field  # noqa: E402
import rpcs3_place as place  # noqa: E402
import rpcs3_hd_volley as volley  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

CLOCK = 0x7014
#: The craft's camera rig eye (`hd-frame-skew.py`: 11.6 to 11.8 units from the body, 0.13 from the push-buffer eye once the skew is taken out).
RIG_EYE = 0x69A0
#: `*(STAGE_POINTER)` is the Environment stage block (`0x00c7dfb0`); at `+0x32b8` sixteen raw band floats
#: (`SoundSystem_GetBandLevel(b) * gain`) and at `+0x32f8` the sixteen held ones the floor's `zoneTexVis` reads
#: (`docs/ghidra/functions/ps3-hdfury-eu/zone-equaliser.md`).
STAGE_POINTER = 0x008B7940
STAGE_BLOCK = 0x00C7DFB0
RAW_BANDS = 0x32B8
HELD_BANDS = 0x32F8


#: `*(WEAPON_MANAGER)` is the weapon manager; `+0x84` its `EMPManager`, whose `+0x64` is the one `EMP` object
#: (`EMPManager_Construct`, `0x00113288`; `docs/ghidra/functions/ps3-hdfury-eu/detonator-emp.md`, "What HD draws").
WEAPON_MANAGER = 0x00994198
#: The race manager `**0x008a6814`; `+0x2dd8` the EMP charge (`detonator-emp.md`, "The charge and the gauge").
RACE_MANAGER = 0x008A6814
EMP_CHARGE = 0x2DD8
#: The player's gauge, `ship+0x7da0`.
EMP_GAUGE = 0x7DA0
#: The EMP's point-light colour, the `vec4` `EMP_DrawLights` (`0x0013b300`) loads at `0x00994480 + 0x10` (the TOC holds the address).
EMP_LIGHT_COLOUR = 0x00994490


def u32be(gdb, address):
    return struct.unpack(">I", gdb.read(address, 4))[0]


def read_emp(gdb):
    """The `EMP` object's state, paused: age, extent, both wave matrices, the AI-track cursor, the handles, the light colour."""
    wm = u32be(gdb, WEAPON_MANAGER)
    mgr = u32be(gdb, wm + 0x84)
    emp = u32be(gdb, mgr + 0x64)
    blob = gdb.read(emp, 0x1a0)
    f = lambda o: struct.unpack_from(">f", blob, o)[0]
    w = lambda o: struct.unpack_from(">I", blob, o)[0]
    rows = lambda o: [list(struct.unpack_from(">4f", blob, o + 16 * i)) for i in range(4)]
    return {"emp": emp, "live_count": u32be(gdb, mgr + 0x68), "flags_34": w(0x34), "flags_40": w(0x40),
            "extent_54": f(0x54), "age_158": f(0x158), "handle_5c": w(0x5c), "handle_60": w(0x60),
            "matrix_70": rows(0x70), "matrix_b0": rows(0xb0), "prev_f0": list(struct.unpack_from(">4f", blob, 0xf0)),
            "cursor_168": [w(0x168 + 4 * i) for i in range(4)], "fork_178": [w(0x178 + 4 * i) for i in range(4)],
            "fork_live_188": blob[0x188], "light_colour": list(struct.unpack(">4f", gdb.read(EMP_LIGHT_COLOUR, 16)))}


def race_clock(gdb, ship):
    return struct.unpack(">f", gdb.read(ship + CLOCK, 4))[0]


def run_until(gdb, ship, target, label, recorder):
    """Resume in short steps and stop paused at the first stop whose lap clock is at or past `target`."""
    for _ in range(400):
        gdb.pause()
        now = race_clock(gdb, ship)
        recorder.sample(now)
        if now >= target:
            return now
        gdb.resume()
        time.sleep(max(0.05, min(0.4, (target - now) * 0.5)))
    sys.exit("%s: the lap clock stopped short of %.2f (a race still on START RACE?)" % (label, target))


def read_bands(gdb):
    """The stage block's raw and held music bands in one read, paused; refuses a block that moved."""
    block = struct.unpack(">I", gdb.read(STAGE_POINTER, 4))[0]
    if block != STAGE_BLOCK:
        raise RuntimeError("stage block pointer %#x, expected %#x" % (block, STAGE_BLOCK))
    blob = gdb.read(block + RAW_BANDS, HELD_BANDS - RAW_BANDS + 64)
    floats = struct.unpack(">32f", blob[:128])
    return {"block": block, "raw": list(floats[:16]), "held": list(floats[16:]),
            "raw_offset": RAW_BANDS, "held_offset": HELD_BANDS}


def observe(session, gdb, ship, body, out, stem, flush):
    """One paused frame: the screenshot with its world clock, the pushbuffer camera candidates, the pose."""
    time.sleep(flush)
    shot, grab_clock = drive.grab_with_clock(out / ("%s.png" % stem), trim=True)
    pose = place.read_pose(gdb, body)
    pose["race_clock_s"] = race_clock(gdb, ship)
    # The camera rig's eye in RAM (`ship+0x69a0`): the push buffer's eye is the same rig `skew` ticks earlier, so the difference
    # over the skew is the camera's own velocity, which is not the craft's (the camera's height follows the track, not the body).
    pose["rig_eye"] = list(struct.unpack(">3f", gdb.read(ship + RIG_EYE, 12)))
    pose["zone_bands"] = read_bands(gdb)
    try:
        pose["scene_width"] = __import__("hd_scene_width").live(gdb.read)
    except Exception as error:  # noqa: BLE001 - a bonus
        print("  %s: no scene width level (%s)" % (stem, error), flush=True)
    try:
        pose["gantry_seconds"] = rpcs3_hd_field.gantry_seconds(rpcs3_hd_field.Mem(session.open_mem()))
    except Exception as error:  # noqa: BLE001 - a bonus
        print("  %s: no gantry time (%s)" % (stem, error), flush=True)
    try:
        pose["mines"] = rpcs3_hd_field.detonator_mines(rpcs3_hd_field.Mem(session.open_mem()), keep_raw=bool(os.environ.get("OAG_MINE_RAW")))
        print("  %s: %d active mines" % (stem, len(pose["mines"])), flush=True)
    except Exception as error:  # noqa: BLE001 - a bonus
        print("  %s: no mines (%s)" % (stem, error), flush=True)
    try:
        pose["emp"] = read_emp(gdb)
        print("  %s: emp age %.3f extent %.3f cursor %s centre %s" % (stem, pose["emp"]["age_158"], pose["emp"]["extent_54"],
              pose["emp"]["cursor_168"], ["%.1f" % v for v in pose["emp"]["matrix_70"][3][:3]]), flush=True)
    except Exception as error:  # noqa: BLE001 - a bonus
        print("  %s: no emp (%s)" % (stem, error), flush=True)
    pose["scenery_clock"] = (None if grab_clock is None or grab_clock["world_clock_s"] is None else
                             {"time_s": grab_clock["world_clock_s"], "nodes": grab_clock["nodes"], "total": grab_clock["total"]})
    blobs = []
    for chain, size in drive.PUSHBUFFER_REGIONS:
        at = drive.resolve_chain(gdb, chain)
        if at:
            blobs.append((at, gdb.read(at, size)))
    candidates = []
    hulls = []
    for at, blob in blobs:
        candidates.extend(ps3_pose.packet_candidates(blob, base=at))
        hulls.extend(ps3_pose.hull_matrices(blob, base=at))
        (out / ("%s-%08x.bin" % (stem, at))).write_bytes(blob)
    # Where the frame draws the player's hull: the pause's RAM is 1 or 2 whole ticks ahead of the frame, so the pair feeds
    # ours this translation, not the body's (`compare-page.md`, "The skew is the pause, not the original's draw order").
    pose["hull_pb"] = ps3_pose.hull_near(hulls, pose["pos"])
    print("  %s: %d hull-shaped matrices, player's %s" % (stem, len(hulls), None if pose["hull_pb"] is None else
          "x%d at %s" % (pose["hull_pb"]["count"], ["%.2f" % v for v in pose["hull_pb"]["translation"]])), flush=True)
    print("  %s: lap clock %.3f, world clock %s, %d camera candidate(s), held bands %s" % (
        stem, pose["race_clock_s"], None if grab_clock is None else grab_clock["world_clock_s"], len(candidates),
        " ".join("%.2f" % v for v in pose["zone_bands"]["held"])), flush=True)
    return shot, candidates, pose


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--image", default="/home/topaxi/projects/OpenAntiGrav/data/images/hdfury-ps3-eu-dec.iso")
    ap.add_argument("--shot", type=float, action="append", required=True, metavar="LAP_CLOCK",
                    help="pause at this lap-clock second and keep the frame; repeat, ascending")
    ap.add_argument("--fire-clock", type=float, help="hold SQUARE from the first stop at or past this lap clock")
    ap.add_argument("--fire-button", default="triangle",
                    help="the pad button held for --fire-clock (the button that fires the cannon in the original is measured here, "
                         "not assumed: SQUARE changed the camera view)")
    ap.add_argument("--fire-pause", type=float, metavar="LAP_S",
                    help="with --fire-clock: keep the button held and pause at the first stop LAP_S of lap clock after the press, "
                         "and keep the next --shot frames there (a frame in the volley itself, rounds in flight); the button is "
                         "released after that stop. The press takes 2 to 4 ticks to reach the game and the picture lags the RAM "
                         "by 2 to 4 more, so a frame in the volley needs 0.1 or more")
    ap.add_argument("--fire-hold", type=float, default=0.5, help="wall seconds SQUARE is held (a loaded host drops a short press)")
    ap.add_argument("--reload-clock", type=float, help="tap CIRCLE (reload) at this lap clock")
    ap.add_argument("--steer", metavar="LAP_CLOCK,WALL_HOLD,X", action="append", default=[],
                    help="from the first stop at or past LAP_CLOCK hold the left stick at X (-1..1) for WALL_HOLD wall seconds; repeatable")
    ap.add_argument("--emp-clock", type=float, help="tap L1 (the EMP) at the first stop at or past this lap clock")
    ap.add_argument("--emp-charge", type=int, help="write this EMP charge (and a full gauge) just before the --emp-clock tap")
    ap.add_argument("--emp-poll", type=float, default=0.0,
                    help="after the L1 tap, sample the EMP object paused every ~30 ms for this many lap-clock seconds, to OUT/emp-poll.json")
    ap.add_argument("--interval", type=float, default=0.12, help="seconds between the two frames of a stop")
    ap.add_argument("--flush", type=float, default=3.0, help="seconds a paused frame is given to be presented")
    ap.add_argument("--team", default="feisar", help="recorded beside the camera")
    ap.add_argument("--pre-wait", type=float, default=30.0,
                    help="wall seconds to wait after the restore before the first START RACE tap: a tap before the prompt "
                         "is up is held over into the race")
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    pair = importlib.util.spec_from_file_location("rpcs3_hd_pair", HERE / "rpcs3-hd-pair.py")
    pair_mod = importlib.util.module_from_spec(pair)
    pair.loader.exec_module(pair_mod)
    pair_mod.set_window(*compare_env.platform("hd-fury")["output"])
    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        gdb = Debugger(port=port)
        time.sleep(args.pre_wait)
        # The clock is polled between taps: a six second settle can find it past an early `--fire-clock` or `--shot`.
        for attempt in range(600):
            gdb.pause()
            try:
                ship, body = place.find_player(gdb)
                clock = race_clock(gdb, ship)
            except RuntimeError:
                clock = 0.0
            gdb.resume()
            if clock > 0.05:
                break
            if attempt % 20 == 0:
                session.tap("cross", settle=0.2)
            time.sleep(0.3)
        else:
            sys.exit("the race never started")
        stops = []
        fire = None
        held_button = None
        events = [[t, "shot"] for t in args.shot]
        if args.fire_clock is not None:
            events.append([args.fire_clock, "fire"])
        if args.reload_clock is not None:
            events.append([args.reload_clock, "reload"])
        for item in args.steer:
            c, h, x = (float(v) for v in item.split(","))
            events.append([c, "steer", h, x])
        if args.emp_clock is not None:
            events.append([args.emp_clock, "emp"])
        events.sort(key=lambda e: e[0])
        # The heap scan for the mines and the round pool runs once, long before the first kept frame.
        gdb.pause()
        recorder = volley.Recorder(session, ship)
        gdb.resume()
        for at, kind, *rest in events:
            now = run_until(gdb, ship, at, kind, recorder)
            if kind == "fire" and args.fire_pause is not None:
                before = now
                gdb.resume()
                session.pad.set(args.fire_button, True)
                for _ in range(400):
                    time.sleep(0.03)
                    gdb.pause()
                    after = race_clock(gdb, ship)
                    recorder.sample(after)
                    if after >= before + args.fire_pause:
                        break
                    gdb.resume()
                fire = {"press_clock_before_s": before, "press_clock_after_s": after, "hold_lap_s": args.fire_pause,
                        "button": args.fire_button, "held_at_frame": True}
                held_button = args.fire_button
                print("fire: lap clock %.3f before, paused at %.3f with the button held" % (before, after), flush=True)
                continue
            if kind == "fire":
                before = now
                gdb.resume()
                session.pad.set(args.fire_button, True)
                time.sleep(args.fire_hold)
                session.pad.set(args.fire_button, False)
                gdb.pause()
                after = race_clock(gdb, ship)
                fire = {"press_clock_before_s": before, "press_clock_after_s": after, "hold_wall_s": args.fire_hold, "button": args.fire_button}
                print("fire: lap clock %.3f before, %.3f after the hold" % (before, after), flush=True)
                continue
            if kind == "steer":
                gdb.resume()
                session.pad.stick(rest[1], 0.0)
                time.sleep(rest[0])
                session.pad.stick(0.0, 0.0)
                gdb.pause()
                print("steer %.2f for %.2f wall s from lap clock %.3f to %.3f" % (rest[1], rest[0], now, race_clock(gdb, ship)), flush=True)
                continue
            if kind == "emp":
                if args.emp_charge is not None:
                    rm = u32be(gdb, u32be(gdb, RACE_MANAGER))
                    gdb.write(rm + EMP_CHARGE, struct.pack(">I", args.emp_charge))
                    gdb.write(ship + EMP_GAUGE, struct.pack(">f", 1.0))
                    print("emp: rm %#x ship %#x (rm+0x13e8 %#x), charge %d, gauge 1.0 written" % (rm, ship, rm + 0x13E8, args.emp_charge), flush=True)
                gdb.resume()
                session.pad.set("l1", True)
                time.sleep(0.3)
                session.pad.set("l1", False)
                gdb.pause()
                if args.emp_poll > 0:
                    start = race_clock(gdb, ship)
                    samples = []
                    while True:
                        sample = read_emp(gdb)
                        sample["race_clock_s"] = race_clock(gdb, ship)
                        sample["pos"] = list(place.read_pose(gdb, body)["pos"])
                        samples.append(sample)
                        if sample["race_clock_s"] - start > args.emp_poll or len(samples) > 400:
                            break
                        gdb.resume()
                        time.sleep(0.03)
                        gdb.pause()
                    (out / "emp-poll.json").write_text(json.dumps(samples, indent=1) + "\n")
                    print("emp poll: %d samples from lap clock %.3f" % (len(samples), start), flush=True)
                continue
            if kind == "reload":
                gdb.resume()
                session.pad.set("circle", True)
                time.sleep(0.3)
                session.pad.set("circle", False)
                gdb.pause()
                continue
            stem = "%02d" % len(stops)
            recorder.sample(race_clock(gdb, ship))
            first = observe(session, gdb, ship, body, out, stem, args.flush)
            gdb.resume()
            time.sleep(args.interval)
            gdb.pause()
            recorder.sample(race_clock(gdb, ship))
            second = observe(session, gdb, ship, body, out, stem + "b", args.flush)
            stops.append((stem, first, second))
            if held_button is not None:
                session.pad.set(held_button, False)
                held_button = None
            gdb.resume()
        picks = []
        for stem, first, second in stops:
            picks.append(ps3_pose.pick_camera([first[1], second[1]])[0])
        # A stop whose own second frame was not redrawn (the game ran under a tick between the two reads) has no
        # frame-unique camera of its own: its camera is the value loaded into registers 256 and 260 that no other STOP's
        # frame carries, so a different stop's frame stands in as the discriminator.
        for at, (pick, stop) in enumerate(zip(picks, stops)):
            if pick[0] is None and len(stops) > 1:
                other = stops[(at + 1) % len(stops)]
                retry = ps3_pose.pick_camera([stop[1][1], other[1][1]])[0]
                if retry[0] is not None:
                    picks[at] = retry
        # Every read's mine draw words, by RAM clock: the picture is several ticks older than its own read, so the pair
        # takes each mine's envelope from the read nearest the picture's clock (`rpcs3-hd-pair.py`, `mine_flags`).
        mine_reads = sorted(({"race_clock_s": read[2]["race_clock_s"],
                              "mines": {str(m["address"]): {k: v for k, v in m.items() if k not in ("raw", "position")}
                                        for m in read[2].get("mines") or []}}
                             for _, first, second in stops for read in (first, second) if read[2].get("mines")),
                            key=lambda r: r["race_clock_s"])
        (out / "cannon-samples.json").write_text(json.dumps(recorder.samples) + "\n")
        mine_reads = sorted(mine_reads + recorder.mine_reads, key=lambda r: r["race_clock_s"])
        for (stem, first, second), (camera, reason, count) in zip(stops, picks):
            shot, _, pose = first
            pose["mine_reads"] = mine_reads
            # The next stop's bands, `--interval` of wall time later: how far the held levels move between two reads
            # bounds what the presented frame's age (0 to 3 ticks) can change.
            pose["zone_bands_next_stop"] = dict(second[2]["zone_bands"], race_clock_s=second[2]["race_clock_s"])
            if fire is not None and pose["race_clock_s"] >= fire["press_clock_after_s"]:
                # Rounds read up to this frame's own pause: a later stop's rounds are not in this picture.
                seen = [s for s in recorder.samples if s["race_clock_s"] <= pose["race_clock_s"]]
                pose["fire"] = volley.fire_record(fire, seen)
                pose["cannon"] = seen[-1]
            record = drive.describe(camera, reason, count, drive.track_name(), shot, args.team, None, pose)
            (out / ("%s.json" % stem)).write_text(json.dumps(record, indent=2) + "\n")
            print("  %s: camera %s" % (stem, "eye %s" % ["%.1f" % v for v in camera["eye"]] if camera else "null (%s)" % reason), flush=True)
        gdb.resume()


if __name__ == "__main__":
    main()
