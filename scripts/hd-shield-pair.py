#!/usr/bin/env python3
"""A comparison pair from one frame of `rpcs3-hd-shield.py`: HD's raised Shield, or the frame after a hit on it.

    python3 scripts/hd-shield-pair.py --capture DIR --stem s01 --out PAIRDIR --lag 4 [--camera-from STEM] [--no-render]

Writes `theirs.png` (the capture's frame), `field.csv` (every craft's RAM pose), `ours.inputs` and `pair.json`, then
renders ours through `compare-rerender-ours.py`'s `rerender`.

Ours holds the player at the frame's pose and camera, gives itself a Shield and fires it so that the shell is the
picture's age (the shield object's clock `+0xf8` less the lag). A frame of a `flash` run also forces a hit
(`--force-hit`) so that ours' shell has run as many colour substeps since it as the picture has: the number of
substeps in the RAM read is `ln(red) / ln(0.85)` off the live colour (amber's red is 1, the target's 0, so it is
read exactly where the team's own red is 0, as Feisar's is), less the lag.

`--lag T` is how many ticks the picture is behind the RAM read of the same pause; pick it by rendering several
and scoring the craft box against `theirs.png`, and say what was scanned in `--lag-basis`.
"""
import argparse
import datetime
import importlib.util
import json
import math
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MAIN = Path("/home/topaxi/projects/OpenAntiGrav")
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402
import compare_provenance  # noqa: E402

IMAGE = MAIN / "data/images/hdfury-ps3-eu-dec.iso"
STATE = "data/saves/hd-fury/hd-racebox-talons-venom-weapons-grid-rpcs3-20574.SAVESTAT.zst"
EMULATOR_VERSION = "0.0.43-20574"
TRACK = r"Data\Environments\Talons_Junction\track.vex"
#: `ShipShield_Activate`'s colour rate: the live colour keeps this share of its distance to the target each substep.
COLOUR_KEEP = 0.85


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


def numbers(values):
    return ",".join("%.6f" % v for v in values)


def substeps_since_hit(live_red):
    """Colour substeps since `ShipShield_Hit`, for a team whose authored `ShieldColour` has no red."""
    if live_red < 1e-3:
        return None
    return round(math.log(live_red) / math.log(COLOUR_KEEP))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--capture", type=Path, required=True)
    ap.add_argument("--stem", required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--id", help="the pair's id; default hd-shield-<out dir>")
    ap.add_argument("--lag", type=int, required=True, help="ticks the picture is behind the RAM read")
    ap.add_argument("--lag-basis", default="picked by rendering ours at several lags and scoring against theirs.png")
    ap.add_argument("--camera-from", help="another stem's camera, for a frame whose camera is missing (a resting camera)")
    ap.add_argument("--team", default="Feisar", help="the hull the original must fly; another one is refused")
    ap.add_argument("--oag-game", type=Path, default=HERE.parent / "target/debug/oag-game")
    ap.add_argument("--no-render", action="store_true")
    args = ap.parse_args()
    capture, stem, out = args.capture.resolve(), args.stem, args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    record = json.loads((capture / ("%s.json" % stem)).read_text())
    clock = json.loads((capture / ("%s.clock.json" % stem)).read_text())
    run = json.loads((capture / "run.json").read_text())
    flash = run["mode"] == "flash"
    #: The control run raised no Shield, so ours holds none either.
    plain = run["mode"] == "plain"
    ship = record["ship"]
    shield = ship["shield"]
    camera = record.get("camera")
    camera_note = None
    if camera is None:
        if not args.camera_from:
            sys.exit("%s has no camera (%s); name one with --camera-from" % (stem, record.get("camera_reason")))
        camera = json.loads((capture / ("%s.json" % args.camera_from)).read_text())["camera"]
        camera_note = "the camera is frame %s's: the held player's camera does not move" % args.camera_from
    if clock.get("flame_clock_s") is None or clock.get("world_clock_offset_s") is None:
        sys.exit("%s: the grab recorded no flame or world clock (%s)" % (stem, clock.get("reason")))
    plat = ce.platform("hd-fury")
    go = ce.go_tick(plat)
    shutil.copy(capture / ("%s.png" % stem), out / "theirs.png")
    crafts = ship["craft"]
    rows = [{"slot": n, "player": c["player"], "team": c["team"], "left": c["rows"][0], "up": c["rows"][1],
             "fwd": c["rows"][2], "pos": c["pos"], "vel": c["vel"]} for n, c in enumerate(crafts)]
    ce.write_field_csv(out / "field.csv", rows, with_player=True)
    player = next(c for c in crafts if c["player"])
    if player["team"].lower() != args.team.lower():
        sys.exit("refused: the original flies %s, not %s (--team)" % (player["team"], args.team))

    ticks = go + round(ship["race_clock_s"] * 60) - args.lag
    age_ticks = round(shield["time_s"] * 60) - args.lag
    fire_tick = ticks - age_ticks
    ram_substeps = substeps_since_hit(shield["live_rgba"][0]) if flash else None
    hit_tick = None
    picture_substeps = None
    if flash:
        picture_substeps = ram_substeps - args.lag
        if picture_substeps < 0:
            sys.exit("lag %d puts the picture before the hit (%d substeps in RAM)" % (args.lag, ram_substeps))
        hit_tick = ticks - 1 - picture_substeps
    (out / "ours.inputs").write_text("600 none\n" if plain else "%d none\n1 square\n600 none\n" % fire_tick)

    pose = ",".join("%.4f" % v for v in player["pos"])
    ours_offset = clock["world_clock_offset_s"] - go / 60
    lap_time = ship["race_clock_s"] - args.lag / 60
    track = (record.get("track") or TRACK).replace(".rcsmodel", ".vex")
    flags = ["--race", "--track", track, "--team", player["team"].upper(),
             "--camera-pose=" + numbers(list(camera["eye"]) + list(camera["forward"]) + list(camera["up"])),
             "--camera-fov", "%.4f" % camera["fov_y_deg"], *ce.ours_flags(plat), "--mode", "single_race",
             "--field-from", str(out / "field.csv"), "--pose=" + pose, "--pose-hold", "--pose-hold-ticks",
             *([] if plain else ["--give", "shield"]), "--input-script", str(out / "ours.inputs"), "--ticks", str(ticks),
             "--world-clock-offset", "%.3f" % ours_offset,
             "--hud-state", "8,1,%.4f,%.4f,0.0000,100.0000" % (lap_time, lap_time)]
    if hit_tick is not None:
        flags += ["--force-hit", "%d:0" % hit_tick]

    pair = {
        "profile": "hd-fury",
        "output": list(plat["output"]),
        "original": {
            "emulator": {"name": "RPCS3", "capture_note": "scripts/rpcs3-hd-shield.py, %s/%s" % (capture.name, stem),
                         "version": EMULATOR_VERSION},
            "state": {"file": STATE, "circuit": "talons", "mode": "single_race", "team": player["team"]},
            "race_clock_s": ship["race_clock_s"],
            "shield": {k: v for k, v in shield.items() if k != "raw"},
            "ram_substeps_since_hit": ram_substeps,
            "world_clock_offset_s": clock["world_clock_offset_s"],
            "flame_clock_s": clock["flame_clock_s"],
            "pilot_assist_byte": ship["pilot_assist"],
        },
        "ours": {"flags": flags, "ticks": ticks, "image": str(IMAGE), "world_clock_offset_s": ours_offset,
                 "shield_age_ticks": age_ticks, "hit_tick": hit_tick, "picture_substeps_since_hit": picture_substeps},
        "meta": {},
    }
    prov = {
        "schema": compare_provenance.SCHEMA,
        "pair_kind": "race",
        "basis": "recorded by the pair script",
        "capture_path": {"kind": "savestate_restore", "state_file": STATE},
        "presented_lag": {"value": args.lag, "unit": "ticks", "basis": args.lag_basis,
                          "what": "the picture behind the RAM read of the same pause"},
        "autopilot": {"armed": False, "timer_written": None, "timer_authored": None, "source": "never armed"},
        "ram_writes": [
            {"target": "the Pilot Assist option byte", "value": "0", "why": "ours renders with Pilot Assist off",
             "when": "before the Shield is raised", "source": "scripts/rpcs3-hd-shield.py"},
        ] + ([] if plain else [{"target": "the player's pickup slot", "value": "shield (5)", "why": "give the Shield",
             "when": "before the grab", "source": "scripts/rpcs3-hd-shield.py"}]) + ([{"target": "the player's pending damage (pickup slot +0x120)", "value": "10.0",
               "why": "an absorbed hit, the word a weapon hit posts", "when": "before the grab",
               "source": "scripts/rpcs3-hd-shield.py flash"}] if flash else []),
        "pair_script_ram_writes": [],
        "emulator": {"name": "RPCS3", "version": EMULATOR_VERSION},
        "twin": {"agreed": None, "rmse_vs_previous": None, "basis": "twin boot captured, not paired"},
        "overrides": [],
    }
    prov["emulator"]["settings_sha"] = compare_provenance.settings_sha(pair["original"]["emulator"])
    pair["original"]["provenance"] = prov
    what = ("%d colour substeps after an absorbed hit" % picture_substeps) if flash else "none" if plain else "settled"
    pair["meta"] = {
        "id": args.id or "hd-shield-%s" % out.name,
        "title": "Talon's Junction, the player's Shield %s" % (
            "flashing, " + what if flash else "not raised (control)" if plain else "raised and settled"),
        "track": "Talon's Junction, Racebox Single Race, Venom",
        "craft": player["team"],
        "state": "RPCS3 %s/%s (scripts/rpcs3-hd-shield.py)" % (capture.name, stem),
        "note": "Shield clock %.2f s on the original (sin(t)/4 + 3/4 = %.2f), ours the same age less the %d tick "
                "picture lag; %s%s. The HUD's "
                "pickup row, shield countdown and message are not matched, and the original's hull shake is not patched "
                "out.%s" % (shield["time_s"], math.sin(shield["time_s"]) / 4 + 0.75, args.lag, what,
                            " (the hit is a write of the pending-damage word on the original, --force-hit on ours)"
                            if flash else "", " %s." % camera_note if camera_note else ""),
    }
    pair["created"] = datetime.datetime.now().astimezone().isoformat(timespec="seconds")
    (out / "pair.json").write_text(json.dumps(pair, indent=2) + "\n")
    print("%s: ticks %d, shield fired at %d, hit tick %s, picture lag %d" % (out.name, ticks, fire_tick, hit_tick, args.lag))
    if args.no_render:
        return
    rerender = load("compare_rerender_ours", "compare-rerender-ours.py")
    print(rerender.rerender(out / "pair.json", args.oag_game.resolve()))


if __name__ == "__main__":
    main()
