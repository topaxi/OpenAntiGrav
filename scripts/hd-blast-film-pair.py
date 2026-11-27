#!/usr/bin/env python3
"""A comparison pair from one frame of `rpcs3-hd-blast-film.py`'s Missile, Bomb or Rocket-on-the-wall film.

    python3 scripts/hd-blast-film-pair.py --film DIR --stem a0233 --out PAIRDIR --speed 385 \\
        --lag 4 --camera-back 3 [--lag-basis TEXT] [--oag-game target/debug/oag-game] [--shot-at X,Y,Z] [--no-render]

Writes `theirs.png` (the film's frame), `field.csv` (every craft's RAM pose, moved back to the frame's instant by
`compare_env.film_pair_field`), `ours.inputs` and `pair.json`, then renders ours through
`compare-rerender-ours.py`'s `rerender`, which also draws `side.png` and the scores.

Ours holds the player at the frame's own pose, fires the weapon, moves it to the original's blast point
(`--force-shot-at`; default the blast object's centre in `hit.json`, `--shot-at` overrides it) and sets it off on
the struck craft's field slot (`--force-missile-hit`, `--force-bomb-trip`). `--speed` is the HUD speed read off
the frame.

**The picture's lag is fitted per pair, not assumed.** A stopped RPCS3 shows the last frame it presented, which
falls further behind RAM on a frame the blast makes heavy. `--lag T` is how many ticks the picture is behind the
RAM read: every craft is moved back `T` ticks of its own velocity, ours' blast is `T` ticks younger than the
original's at the stop (the Missile pool's `+0x170`, the Bomb's lap clock since the hit) and its clocks are `T`
ticks earlier. `--camera-back K` moves the push-buffer camera back `K` ticks along the player's velocity (the
push buffer is itself 1 or 2 ticks behind RAM, so `K` is about `T` less that). Pick both by rendering ours at
several values and taking the lowest whole-frame RMSE, and say what was scanned in `--lag-basis`; a resting
player's camera needs no `K` (`docs/rendering/particle-fill.md`, "The Bomb's and the Missile's explosions
paired").

A `rocket-wall` film has no struck craft: ours holds the player at the parked pose and fires its own Rockets, which
meet the same wall. `--fire-shift N` presses ours `N` ticks after the original's press (`hit.json`
`pressed_clock`); with the player at rest only `--lag` plus `N` moves the blast, so fix `--lag` and scan `N`
(`docs/rendering/particle-fill.md`, "A Rocket on the wall paired").
"""
import argparse
import datetime
import importlib.util
import json
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
#: The save state's circuit: a restored run prints no track line for the film to record.
TRACK = r"Data\Environments\Talons_Junction\track.vex"
#: A Bomb is dropped this many ticks before ours sets it off, past its arming delay.
BOMB_LAID_TICKS = 60
#: A dropped Bomb is moved to the blast point this many ticks after the press, before the craft that dropped
#: it, held on top of it, can trip it itself.
BOMB_MOVE_TICKS = 5
#: A Missile is in flight this many ticks before ours moves it to the blast point.
MISSILE_FLIGHT_TICKS = 3


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


def numbers(values):
    return ",".join("%.6f" % v for v in values)


def wall_meta(film, hit, ship, player, args, camera_note):
    return {
        "id": "hd-rocket-wall-%04d" % round(ship["age_since_hit_s"] * 1000),
        "title": "Talon's Junction, Rockets on the track wall, %.3f s after the press" % ship["age_since_hit_s"],
        "track": "Talon's Junction, Racebox Single Race, Venom",
        "craft": player["team"],
        "state": "RPCS3 film %s (scripts/rpcs3-hd-blast-film.py --weapon rocket-wall --yaw %g)" % (film.name, hit["yaw"]),
        "note": "Both sides hold the player still, turned into the wall, and press fire; ours' press is %d ticks after "
                "the original's and its Rockets meet our wall with nothing forced. Ours' other craft are where its own "
                "race put them; the HUD's pickup row and weapon name are not matched.%s"
                % (args.fire_shift, " %s." % camera_note if camera_note else ""),
    }


def blast_meta(film, stem, weapon, force, ship, player, args, age_ticks, ram_age, camera_note):
    ours_name = "ours moves its own %s to the original's blast point and sets it off there (--force-shot-at, %s)" % (
        weapon.capitalize(), force)
    return {
        "id": "hd-%s-craft-%s" % (weapon, stem.lstrip("a")),
        "title": "Talon's Junction, a %s on a craft, %.3f s after the hit" % (weapon.capitalize(), ship["age_since_hit_s"]),
        "track": "Talon's Junction, Racebox Single Race, Venom",
        "craft": player["team"],
        "state": "RPCS3 film %s (scripts/rpcs3-hd-blast-film.py)" % film.name,
        "note": "%s; the blast is %d ticks old on both sides (the original's %.3f s at the stop less %d). Ours' other "
                "craft are where its own race put them after the countdown, only the struck one is moved; the HUD's "
                "pickup row, sight and weapon name are not matched, and the original's hull shake is not patched out.%s"
                % (ours_name, age_ticks, ram_age, args.lag, " %s." % camera_note if camera_note else ""),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--film", type=Path, required=True)
    ap.add_argument("--stem", required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--speed", type=float, required=True, help="the HUD speed read off the frame, km/h")
    ap.add_argument("--shot-at", help="X,Y,Z ours moves the weapon to; default the film's blast centre")
    ap.add_argument("--fire-shift", type=int, default=0, help="rocket-wall: ticks ours presses after the original")
    ap.add_argument("--lag", type=int, required=True, help="ticks the picture is behind the RAM read")
    ap.add_argument("--camera-back", type=float, default=0.0, help="ticks the push-buffer camera is moved back")
    ap.add_argument("--lag-basis", default="picked by rendering ours at several lags and scoring against theirs.png",
                    help="how --lag and --camera-back were picked, recorded in the pair")
    ap.add_argument("--camera-from", help="another stem's camera, for a frame whose camera is missing (a resting camera)")
    ap.add_argument("--oag-game", type=Path, default=HERE.parent / "target/debug/oag-game")
    ap.add_argument("--no-render", action="store_true")
    args = ap.parse_args()
    film, stem, out = args.film, args.stem, args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    hit = json.loads((film / "hit.json").read_text())
    weapon = hit["weapon"]
    wall = weapon == "rocket-wall"
    record = json.loads((film / ("%s.json" % stem)).read_text())
    clock = json.loads((film / ("%s.clock.json" % stem)).read_text())
    ship = record["ship"]
    camera = record.get("camera")
    camera_note = None
    if camera is None:
        if not args.camera_from:
            sys.exit("%s has no camera (%s); name one with --camera-from" % (stem, record.get("camera_reason")))
        camera = json.loads((film / ("%s.json" % args.camera_from)).read_text())["camera"]
        camera_note = "the camera is frame %s's: the held player's camera does not move" % args.camera_from
    elif camera.get("picked"):
        camera_note = camera["picked"]
    if clock.get("flame_clock_s") is None or clock.get("world_clock_offset_s") is None:
        sys.exit("%s: the grab recorded no flame or world clock (%s)" % (stem, clock.get("reason")))

    plat = ce.platform("hd-fury")
    go = ce.go_tick(plat)
    shutil.copy(film / ("%s.png" % stem), out / "theirs.png")
    crafts = ship["craft"]
    rows = [{"slot": n, "player": c["player"], "team": c["team"], "left": c["rows"][0], "up": c["rows"][1],
             "fwd": c["rows"][2], "pos": c["pos"], "vel": c["vel"]} for n, c in enumerate(crafts)]
    ce.write_field_csv(out / "field.csv", rows, with_player=True)
    opponents = [c for c in crafts if not c["player"]]
    struck = None if wall else next(n for n, c in enumerate(opponents, 1) if c["ship"] == hit["struck"])
    player = next(c for c in crafts if c["player"])

    ram_age = (ship["blasts"][0]["age"] if weapon == "missile" else ship["age_since_hit_s"])
    centre = None if wall else hit["blasts_at_hit"][0]["centre"]
    shot_at = [float(v) for v in args.shot_at.split(",")] if args.shot_at else centre

    pair = {
        "profile": "hd-fury",
        "output": list(plat["output"]),
        "original": {
            "emulator": {"name": "RPCS3", "capture_note": "scripts/rpcs3-hd-blast-film.py, %s/%s" % (film.name, stem),
                         "version": EMULATOR_VERSION},
            "state": {"file": STATE, "circuit": "talons", "mode": "single_race", "team": player["team"]},
            "race_clock_s": ship["race_clock_s"],
            "hit_age_s": ship["age_since_hit_s"],
            "blast_ram_age_s": ram_age,
            "blast_centre": centre,
            "world_clock_offset_s": clock["world_clock_offset_s"],
            "flame_clock_s": clock["flame_clock_s"],
            "pilot_assist_byte": ship["pilot_assist"],
        },
        "ours": {},
        "meta": {},
    }
    (out / "pair.json").write_text(json.dumps(pair, indent=2) + "\n")
    ce.write_field_rows(out / "field.csv", ce.rewound_field_rows(ce.read_field_rows(out / "field.csv"), args.lag))
    ce.record_craft_ticks(
        out, {"craft_ticks": args.lag, "camera_back_ticks": args.camera_back, "craft_basis": args.lag_basis},
        "field.csv moved back craft_ticks ticks of each craft's own velocity",
        picture="the camera is the push-buffer matrix moved back camera_back_ticks ticks along the player's velocity")
    rec = json.loads((out / "pair.json").read_text())
    ticks = go + round(ship["race_clock_s"] * 60) - args.lag
    hit_tick = move_tick = None
    if wall:
        fire_tick = go + round(hit["pressed_clock"] * 60) + args.fire_shift
        age_ticks = ticks - fire_tick
    else:
        age_ticks = max(0, round(ram_age * 60) - args.lag)
        hit_tick = ticks - 1 - age_ticks
        fire_tick = hit_tick - (MISSILE_FLIGHT_TICKS if weapon == "missile" else BOMB_LAID_TICKS)
        move_tick = hit_tick if weapon == "missile" else fire_tick + BOMB_MOVE_TICKS
    (out / "ours.inputs").write_text("%d none\n1 square\n600 none\n" % fire_tick)

    field = ce.read_field_rows(out / "field.csv")
    pose = ",".join("%.4f" % float(field[0]["pos_" + a]) for a in "xyz")
    force = {"missile": "--force-missile-hit", "bomb": "--force-bomb-trip"}.get(weapon)
    ours_offset = clock["world_clock_offset_s"] - go / 60
    lap_time = ship["race_clock_s"] - args.lag / 60
    eye = [e - v * args.camera_back / 60.0 for e, v in zip(camera["eye"], player["vel"])]
    track = (record.get("track") or TRACK).replace(".rcsmodel", ".vex")
    flags = ["--race", "--track", track, "--team", player["team"].upper(),
             "--camera-pose=" + numbers(eye + list(camera["forward"]) + list(camera["up"])),
             "--camera-fov", "%.4f" % camera["fov_y_deg"], *ce.ours_flags(plat), "--mode", "single_race",
             "--field-from", str(out / "field.csv"), "--pose=" + pose, "--pose-hold", "--pose-hold-ticks",
             "--give", "rocket" if wall else weapon, "--input-script", str(out / "ours.inputs"),
             *([] if wall else ["--force-shot-at", "%d:%s" % (move_tick, ",".join("%.4f" % v for v in shot_at)),
                                force, "%d:%d" % (hit_tick, struck)]),
             "--ticks", str(ticks),
             "--world-clock-offset", "%.3f" % ours_offset,
             "--hud-state", "8,1,%.4f,%.4f,%.4f,100.0000" % (lap_time, lap_time, args.speed)]

    rec["ours"] = {"flags": flags, "ticks": ticks, "image": str(IMAGE), "world_clock_offset_s": ours_offset}
    rec["ours"].update({"press_age_ticks": age_ticks, "fire_tick": fire_tick, "fire_shift": args.fire_shift} if wall
                       else {"blast_age_ticks": age_ticks, "hit_tick": hit_tick, "struck_slot": struck})
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
             "when": "before the race starts", "source": "scripts/rpcs3-hd-pilot-assist.py --off"},
            {"target": "the player's pickup slot", "value": weapon, "why": "give the weapon",
             "when": "before the grab", "source": "film script"},
            ({"target": "the player craft pose", "value": "held where the race left it, turned %g degrees" % hit["yaw"],
              "why": "aim the shot at the wall", "when": "every game frame from before the shot", "source": "film script"}
             if wall else
             {"target": "the player craft pose", "value": "%g units behind the nearest rival" % hit["back"],
              "why": "aim the shot", "when": "before the grab", "source": "film script"} if weapon == "missile" else
             {"target": "the player and one rival craft pose",
              "value": "player parked %g units behind the drop, then the rival parked on it" % hit["back"],
              "why": "put the blast ahead of the camera and set it off", "when": "before the grab",
              "source": "film script"}),
        ],
        "pair_script_ram_writes": [],
        "emulator": {"name": "RPCS3", "version": EMULATOR_VERSION},
        "twin": {"agreed": None, "rmse_vs_previous": None, "basis": "no second capture of this pair"},
        "overrides": [],
    }
    prov["emulator"]["settings_sha"] = compare_provenance.settings_sha(rec["original"]["emulator"])
    rec["original"]["provenance"] = prov
    rec["meta"] = wall_meta(film, hit, ship, player, args, camera_note) if wall else blast_meta(
        film, stem, weapon, force, ship, player, args, age_ticks, ram_age, camera_note)
    rec["created"] = datetime.datetime.now().astimezone().isoformat(timespec="seconds")
    (out / "pair.json").write_text(json.dumps(rec, indent=2) + "\n")
    what = "press tick %d" % fire_tick if wall else "hit tick %d (age %d), struck slot %d" % (hit_tick, age_ticks, struck)
    print("%s: ticks %d, %s, picture lag %d" % (out.name, ticks, what, args.lag))
    if args.no_render:
        return
    rerender = load("compare_rerender_ours", "compare-rerender-ours.py")
    print(rerender.rerender(out / "pair.json", args.oag_game.resolve()))


if __name__ == "__main__":
    main()
