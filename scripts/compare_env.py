#!/usr/bin/env python3
"""The environment profile of the comparison pairs, and the checks that enforce it.

    python3 scripts/compare_env.py show pulse-psp
    python3 scripts/compare_env.py ppsspp-ini pulse-psp        # the [Graphics] text for PPSSPP
    python3 scripts/compare_env.py manifest OUT.toml DIR...   # one [[entry]] per non-refused pair.json
    python3 scripts/compare_env.py film-field FILM STEM FIELD_IN FIELD_OUT   # a film frame's field at the frame's instant
    python3 scripts/compare_env.py film-pair-field FILM STEM PAIR_DIR        # the same, in place in a pair, recorded

`scripts/compare-environment.toml` is the profile. The pair scripts (`psp-pulse-pair.py`,
`pcsx2-pulse-pair.py`, `rpcs3-hd-pair.py`) import this module: they build a `Pair`, record
what each side actually ran with, and `Pair.finish` writes `pair.json` and REFUSES (exit 3,
every mismatch named) a pair whose environment differs from the profile or whose two
sides differ beyond the profile's tolerance. `manifest` carries the environment into the
compare page's manifest, so the page shows it.
"""

import datetime
import hashlib
import json
import math
import os
import re
import struct
import subprocess
import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
PROFILE = HERE / "compare-environment.toml"
MAIN = Path("/home/topaxi/projects/OpenAntiGrav")


def load():
    return tomllib.loads(PROFILE.read_text())


def platform(key):
    """The merged table of one platform (`inherits` resolved, ours defaults and tolerance attached)."""
    doc = load()
    table = doc["platform"].get(key)
    if table is None:
        sys.exit("compare_env: no platform %r in %s (have %s)" % (key, PROFILE.name, sorted(doc["platform"])))
    if "inherits" in table:
        base = platform(table["inherits"])
        merged = {**base, **{k: v for k, v in table.items() if k != "inherits"}}
        merged["emulator"] = {**base.get("emulator", {}), **table.get("emulator", {})}
        table = merged
    out = dict(table)
    out["key"] = key
    out["ours"] = dict(doc["ours_defaults"])
    if "shadows" in table:
        out["ours"]["shadows"] = table["shadows"]
    if "anisotropy" in table:
        out["ours"]["anisotropy"] = table["anisotropy"]
    if "pilot_assist" in table:
        out["ours"]["pilot_assist"] = table["pilot_assist"]
    if "hud_safe_area" in table:
        out["ours"]["hud_safe_area"] = table["hud_safe_area"]
    if "exposure_hold" in table:
        out["ours"]["exposure_hold"] = table["exposure_hold"]
    out["tolerance"] = dict(doc["tolerance"])
    out["profile_sha"] = hashlib.sha256(PROFILE.read_bytes()).hexdigest()[:12]
    return out


def detonator_pair_mines(mines, reads=()):
    """The original's mine objects a Detonator pair carries into ours: the armed ones (`flags & 9 == 9`), plus any mine that left
    the pool during the shot's own mine reads (`reads`, `ship.mine_reads`). A mine a cannon round triggered is compacted out of the
    pool within two updates and keeps only flag `1`, as does one gone long before; the difference is that its age (`+0xc0`, which
    stops when it leaves) still advanced between the shot's first read and the pause. Ours needs that mine laid, or its replayed
    round has nothing to hit and shows no blast and no `+100` where the original's frame has both
    (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine.md`, "Carrying the destroyed mine")."""
    first = reads[0]["mines"] if reads else {}

    def left_during_shot(m):
        before = first.get(str(m.get("address")), {}).get("age_s")
        return m["flags"] & 1 and before is not None and m.get("age_s") is not None and m["age_s"] > before

    return [m for m in mines if m["flags"] & 9 == 9 or left_during_shot(m)]


def detonator_input_lines(fire, go_tick_):
    """The lines of a Detonator pair's `ours.inputs`: idle through the countdown and up to the original's press, SQUARE (ours'
    cannon button) over the window the original's button was held, idle after; `fire` is `original.detonator.fire` of
    `pair.json` (`None` for a pair with no shot).

    Where the rounds' ages gave each volley's tick (`volley_clocks_s`) ours holds SQUARE for that one tick per volley and
    lets go between: a release zeroes ours' trigger, so the next press fires on its own tick. The original's frames are
    one or two ticks long, so its volleys are not spaced as a held trigger's are at 60 Hz."""
    if not fire:
        return ["%d none" % (go_tick_ + 1)]
    volleys = fire.get("volley_clocks_s")
    if volleys:
        lines, at = [], 0
        for clock in volleys:
            tick = go_tick_ + round(clock * 60)
            lines += ["%d none" % (tick - at), "1 square"]
            at = tick + 1
        return lines + ["1 none"]
    before = round(fire["press_clock_before_s"] * 60)
    held = max(1, round((fire["press_clock_after_s"] - fire["press_clock_before_s"]) * 60))
    return ["%d none" % (go_tick_ + before), "%d square" % held, "1 none"]


def ensure_detonator_inputs(rec, pair_dir):
    """Writes `ours.inputs` and `field.csv` beside a Detonator pair's `pair.json` when missing (the private repository keeps `pair.json`
    and `theirs.png` only; the script is made again from the recorded `original.detonator.fire`, the pose file from `ours.pose_row`).
    Returns whether it wrote."""
    det = (rec.get("original") or {}).get("detonator")
    if det is None:
        return False
    wrote = False
    path = Path(pair_dir) / "ours.inputs"
    if not path.exists():
        path.write_text("\n".join(detonator_input_lines(det.get("fire"), go_tick())) + "\n")
        wrote = True
    row = (rec.get("ours") or {}).get("pose_row")
    field = Path(pair_dir) / "field.csv"
    if row and not field.exists():
        write_field_csv(field, [{"player": True, "vel": [0.0, 0.0, 0.0], **{k: row[k] for k in ("team", "left", "up", "fwd", "pos")}}],
                        with_player=True)
        wrote = True
    return wrote


#: Flags of ours' `oag-game` whose value is a file the pair's own original produced (the pose rows, the field rows, an
#: input script). A re-render reads them from the pair directory, so they are the pair's recorded inputs.
INPUT_FILE_FLAGS = ("--pose-from", "--field-from", "--input-script")
#: A recorded path that no longer exists: a lane's scratch directory or a reaped worktree.
STALE_PATH = re.compile(r"\{scratch\}/|OpenAntiGrav-worktrees/|/data/scratch/")


def inputs_made_again(rec):
    """File names `ensure_detonator_inputs` writes into a pair directory on a re-render, so they need not be stored there."""
    names = set()
    if (rec.get("original") or {}).get("detonator") is not None:
        names.add("ours.inputs")
        if (rec.get("ours") or {}).get("pose_row"):
            names.add("field.csv")
    return names


def resolve_input_flags(rec, flags, pair_dir):
    """`(flags, problems)`: ours' recorded flags with every input file and every stale path read from `pair_dir`.

    An `INPUT_FILE_FLAGS` value, and any other value naming a lane's scratch or a reaped worktree, becomes the file of the
    same name in the pair directory; one that is not there (and that `inputs_made_again` does not write) is a problem
    naming the file. `compare-rerender-ours.py` skips a pair with a problem, and `compare_provenance.py scan` fails it."""
    pair_dir = Path(pair_dir)
    again = inputs_made_again(rec)
    out, problems = [], []
    for i, f in enumerate(flags):
        f = str(f)
        if STALE_PATH.search(f) or (i and str(flags[i - 1]) in INPUT_FILE_FLAGS):
            name = Path(f).name
            if (pair_dir / name).exists() or name in again:
                f = str(pair_dir / name)
            else:
                problems.append("%s: no %s in the pair directory" % (f, name))
        out.append(f)
    return out, problems


def go_tick(plat=None):
    """Our race's tick at which the race clock leaves zero: the countdown's length, read from the source of truth
    (`oag_race::COUNTDOWN_TICKS`, `crates/race/src/state.rs`) rather than copied into each pair script. A platform
    table may carry its own `go_tick` once a title's countdown is measured to differ; today ours has one length
    for every title, so every platform reads the same number."""
    if plat and "go_tick" in plat:
        return int(plat["go_tick"])
    m = re.search(r"pub const COUNTDOWN_TICKS: u64 = (\d+);", (ROOT / "crates/race/src/state.rs").read_text())
    if not m:
        sys.exit("compare_env: COUNTDOWN_TICKS not found in crates/race/src/state.rs")
    return int(m.group(1))


def size_text(plat):
    return "%dx%d" % tuple(plat["output"])


def profile_anisotropy_flags(flags, plat, kind=None):
    """`flags` with `--anisotropy` at the profile's level. A recorded flag beats the settings file, so a race pair recorded before
    the profile's level changed (Omega, `off` to `16x`) would otherwise re-render at the stored one. Menu and loading pairs keep
    their recorded flags: only race grids were measured."""
    if kind in ("menu", "loading"):
        return list(flags)
    flags = list(flags)
    level = plat["ours"]["anisotropy"]
    if "--anisotropy" in flags:
        flags[flags.index("--anisotropy") + 1] = level
    else:
        flags += ["--anisotropy", level]
    return flags


def profile_pilot_assist_flags(flags, plat, kind=None):
    """`flags` with `--pilot-assist` at the profile's setting. A recorded flag beats the profile, so a pair recorded while the
    profile said `on` would keep drawing an indicator the original's recapture lacks. Menu and loading pairs hold no craft."""
    flags = list(flags)
    level = plat["ours"].get("pilot_assist")
    if level is None or kind in ("menu", "loading") or "--pilot-assist" not in flags:
        return flags
    flags[flags.index("--pilot-assist") + 1] = level
    return flags


def ppsspp_ini(plat):
    """The `[Graphics]` lines the instance's ini carries for this profile (PPSSPP picture settings)."""
    e = plat["emulator"]
    return ("[Graphics]\nSoftwareRenderer = True\nInternalResolution = %d\nAnisotropyLevel = 0\n"
            "FrameSkip = %d\nTextureFiltering = 1\nPostShaderList =\n"
            % (e["internal_resolution"], e["frameskip"]))


def ours_flags(plat):
    """`oag-game` flags that carry the profile's render settings (every one that has a flag)."""
    o = plat["ours"]
    flags = ["--no-audio", "--size", size_text(plat),
             "--render-scale", str(o["render_scale"]), "--reconstruction", o["reconstruction"],
             "--msaa", o["msaa"], "--motion-blur", o["motion_blur"],
             "--lod", o["model_detail"], "--texture-detail", o["texture_detail"],
             "--trail-order", o["trail_order"],
             "--screen-filter", o["screen_filter"], "--anisotropy", o["anisotropy"]]
    if "pilot_assist" in o:
        flags += ["--pilot-assist", o["pilot_assist"]]
    flags += exposure_hold_flags(o)
    # A still pair cannot match a random shake frame for frame, so ours draws every hull at its body's pose; HD
    # originals are captured with theirs patched out (`scripts/rpcs3-hd-no-hull-shake.py`). A no-op on a title without
    # one (Pulse, Pure).
    flags += ["--no-hull-shake"]
    return flags


def exposure_hold_flags(ours):
    """`--hold-exposure E` when the profile holds the exposure (2048: the original under Vita3K never adapts)."""
    return ["--hold-exposure", str(ours["exposure_hold"])] if "exposure_hold" in ours else []


def gantry_pin(gantry_s, lag_ticks):
    """The `--gantry-seconds` value for a picture that lags RAM by `lag_ticks`.

    The gantry node's time is read from RAM in the same pause as the HUD clock; the RSX had presented a frame
    `lag_ticks` older (`HUD clock text vs RAM`, 3 ticks on HD). Ours already runs `--ticks` less that lag, so the
    board's pin takes it too: the GO board pulses with a 0.6 s period (swept in ours), so an
    uncorrected pin sat on its rising edge (Tech de Ra 4.3687, Modesto Heights 4.9670) and drew the glyph the
    original had not yet drawn. `None` (no windowed node: the board is gone) stays `None`.
    """
    return None if gantry_s is None else gantry_s - lag_ticks / 60


#: How far HD's flame program's `time` lags the frame context clock read in the same pause: the fragment program in the
#: command ring held 0.0334 s and 0.0167 s less than `+0xc4` on two pauses (`docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`,
#: "The flame's clock at a pair's grab"); the mean, so the pin is within 0.009 s (2 % of the noise's 0.5 s period) of either.
FLAME_CLOCK_LAG_S = 0.025


def flame_pin(flame_clock_s):
    """The `--flame-clock` value for a pair whose original read the frame context clock `flame_clock_s` in its pause."""
    return None if flame_clock_s is None else flame_clock_s - FLAME_CLOCK_LAG_S


def picture_tick(go_tick_, clock_s, total_lag_ticks):
    """Ours' tick for a picture that is `total_lag_ticks` older than the RAM read at lap clock `clock_s`: a Detonator pair
    whose lag was measured runs its rounds, mine hits, score and ammo to the picture's moment, not the HUD clock's."""
    return go_tick_ + round(clock_s * 60 - total_lag_ticks)


def detonator_mine_volume_flags(mines):
    """`--detonator-mine-volumes` for `mines` in `--detonator-mines` order: each ring's `MasterVolume` and `PitchVolume`
    (`+0x104`, `+0x10c`) as the original drew them, `[]` unless every mine carries both. Ours has no level of its own under
    `--no-audio` (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-material.md`, "The visualiser's level")."""
    if not mines or not all(m.get("master_volume") is not None and m.get("pitch_volume") is not None for m in mines):
        return []
    return ["--detonator-mine-volumes", ";".join("%.4f,%.4f" % (m["master_volume"], m["pitch_volume"]) for m in mines)]


def recorded_mine_volume_flags(rec, flags):
    """`flags` with the mines' volumes added from `original.detonator.mines` (the read at the picture's clock) for a pair
    recorded before `--detonator-mine-volumes` existed; unchanged when they are there or no mine was carried."""
    if "--detonator-mines" not in flags or "--detonator-mine-volumes" in flags:
        return flags
    mines = ((rec.get("original") or {}).get("detonator") or {}).get("mines") or []
    volumes = detonator_mine_volume_flags(mines)
    carried = len(flags[flags.index("--detonator-mines") + 1].split(";"))
    if volumes and len(mines) != carried:
        raise ValueError("the record names %d mines, --detonator-mines %d" % (len(mines), carried))
    return flags + volumes


def detonator_picture_flags(rec, flags, go_tick_):
    """`flags` with `--ticks` and the gantry pin at the picture's moment, for a Detonator pair whose lag was measured
    (`original.presented_lag.picture_total_ticks`); any other pair's flags unchanged. Records ours' tick and clock."""
    orig = rec.get("original") or {}
    total = (orig.get("presented_lag") or {}).get("picture_total_ticks")
    clock = orig.get("race_clock_s")
    if "--pose-hold-ticks" not in flags or total is None or clock is None:
        return flags
    flags = list(flags)
    ticks = picture_tick(go_tick_, clock, total)
    rec["ours"]["ticks"] = ticks
    rec["ours"]["race_clock_s"] = (ticks - go_tick_) / 60
    if "--ticks" in flags:
        flags[flags.index("--ticks") + 1] = str(ticks)
    else:
        flags += ["--ticks", str(ticks)]
    if "--gantry-seconds" in flags and orig.get("gantry_seconds") is not None:
        flags[flags.index("--gantry-seconds") + 1] = "%.6f" % gantry_pin(orig["gantry_seconds"], total)
    return flags


def tenths_matched_ticks(clock):
    """Ticks after GO whose clock shows the same tenth as the original's `clock` seconds.

    Pulse's HUD writes `trunc(clock * 100)` centiseconds and then the tenth by truncation (measured live
    2026-10-21: the string in RAM equalled the truncated tenth of the clock float on 40 of 40 stops). Rounding to the nearest tick
    can land up to half a tick (8 ms) past the original's clock, and where a tenth boundary falls in that gap ours
    reads one tenth later than the frame it is paired with (Amphiseum Black: clock 13.2967, 798 ticks, `13.3` against
    the original's `13.2`). Ours is quantised to ticks, so the nearest tick is kept unless it crosses a tenth.
    """
    ticks = round(clock * 60)
    if ticks * 10 // 60 > int(clock * 10):
        ticks -= 1
    return ticks


def ours_env(profile_dir):
    return {"XDG_CONFIG_HOME": str(profile_dir / "cfg"), "XDG_DATA_HOME": str(profile_dir / "data"),
            "XDG_STATE_HOME": str(profile_dir / "state")}


def settings_path(profile_dir):
    return profile_dir / "cfg" / "oag" / "settings.toml"


SAVE_RECORD_NAME = "Safe Area Setting"


def parse_safe_area(data):
    """The `Safe Area Setting` percent out of an HD save's `DATA.BIN` bytes, or None.

    Every option record is a 0x40-byte name field, then a u32 value length (the string and its NUL), a u32 0x40 (the
    value field's size), a runtime pointer and the value as a string at name+0x60: `96%` or `96` (docs/formats/hd-hud.md,
    "The whole HUD is drawn at the Safe Area Setting"). A record that does not have that shape reads as None."""
    at = data.find(SAVE_RECORD_NAME.encode() + b"\0")
    if at < 0 or at + 0x60 + 8 > len(data):
        return None
    length, field = struct.unpack_from(">II", data, at + 0x44)
    if field != 0x40 or not 2 <= length <= 8:
        return None
    m = re.fullmatch(rb"(\d{1,3})%?\0*", data[at + 0x60:at + 0x60 + length])
    return int(m.group(1)) if m else None


def hd_save_file(config_home=None):
    """The HD save the running RPCS3 reads: `<config>/rpcs3/dev_hdd0/home/*/savedata/BCES00664-AUTO-/DATA.BIN` under the
    private `XDG_CONFIG_HOME` the pair script runs in (else `~/.config`)."""
    home = Path(config_home or os.environ.get("XDG_CONFIG_HOME") or Path.home() / ".config")
    found = sorted((home / "rpcs3/dev_hdd0/home").glob("*/savedata/BCES00664-AUTO-/DATA.BIN"))
    return found[0] if found else None


def read_hd_safe_area(config_home=None):
    """`original.safe_area` for a pair taken on HD: `{"percent", "source", "file_sha256_12", "file_mtime"}` read off the
    save the capture's RPCS3 ran with, or None when there is no save or no readable record. A cold-booted pair (the menus)
    is exactly the save's value; a pair that restores a save state carries the RAM of the state, which may differ from
    the file (the option is read at boot), so a caller on that path says so in `source`."""
    path = hd_save_file(config_home)
    if path is None:
        return None
    data = path.read_bytes()
    percent = parse_safe_area(data)
    if percent is None:
        return None
    return {"percent": percent, "source": "save file", "file_sha256_12": hashlib.sha256(data).hexdigest()[:12],
            "file_mtime": datetime.datetime.fromtimestamp(path.stat().st_mtime).astimezone().isoformat(timespec="seconds")}


def record_safe_area(pair, recorded=None, restored_state=False):
    """Record the Safe Area the original ran at into `pair.rec["original"]["safe_area"]` for a platform whose profile
    pins one (`hud_safe_area`), and check it is there. `recorded` defaults to `read_hd_safe_area()`. Returns the percent
    ours is drawn at: the recorded value, else the profile's pin (the pair then carries a caveat saying so)."""
    pin = pair.plat["ours"].get("hud_safe_area")
    if pin is None:
        return None
    recorded = recorded or read_hd_safe_area()
    if recorded and restored_state:
        recorded = dict(recorded, source="save file at capture time; the restored save state carries its own RAM, "
                                         "which the file may not match")
    if recorded:
        pair.rec["original"]["safe_area"] = recorded
    pair.check("original Safe Area Setting recorded", bool(recorded),
               "%s%%, %s" % (recorded["percent"], recorded["source"]) if recorded else
               "none read: ours falls back to the profile's %d%%" % pin)
    return recorded["percent"] if recorded else pin


def recorded_safe_area(rec):
    """The percent a stored pair's original ran at (`original.safe_area.percent`), or None."""
    return ((rec.get("original") or {}).get("safe_area") or {}).get("percent")


def write_ours_settings(plat, profile_dir, safe_area=None):
    """Write the private settings.toml: the keys with no command-line flag (shadows `original`,
    boost fov kick, fov percent, aspect, brightness/gamma), plus the rest, so the file the run
    rewrites can be read back and compared. `safe_area` (percent) is the pair's own recorded
    original value and overrides the profile's `hud_safe_area` pin, which stays the fallback."""
    o = dict(plat["ours"])
    if safe_area is not None and "hud_safe_area" in o:
        o["hud_safe_area"] = int(safe_area)
    plat = dict(plat, ours=o)
    for sub in ("cfg/oag", "data", "state"):
        (profile_dir / sub).mkdir(parents=True, exist_ok=True)
    text = ("[display]\naspect = \"%s\"\nbrightness = %d\ngamma = %d\n\n"
            "[graphics]\nanisotropy = \"%s\"\nfov = %d\nboost_fov_kick = %d\n\n"
            "[render_profiles.\"%s\"]\nrender_scale = %d\nreconstruction = \"%s\"\nmsaa = \"%s\"\n"
            "motion_blur = \"%s\"\nshadows = \"%s\"\nmodel_detail = \"%s\"\ntexture_detail = \"%s\"\n"
            "trail_order = \"%s\"\n"
            "screen_filter = \"%s\"\n"
            % (plat["aspect"], o["brightness"], o["gamma"], "16x" if o["anisotropy"] == "16x" else o["anisotropy"],
               o["fov_percent"], o["boost_fov_kick"], plat["profile_key"], o["render_scale"],
               o["reconstruction"], o["msaa"], o["motion_blur"], o["shadows"], o["model_detail"],
               o["texture_detail"], o["trail_order"], o["screen_filter"]))
    if "hud_safe_area" in o:
        text = text.replace("boost_fov_kick = %d\n" % o["boost_fov_kick"],
                            "boost_fov_kick = %d\nhud_safe_area = %d\n" % (o["boost_fov_kick"], o["hud_safe_area"]), 1)
    settings_path(profile_dir).write_text(text)


def read_back(plat, profile_dir, safe_area=None):
    """What ours ran with, read out of the settings file the run rewrote, and the mismatches. A platform that pins a
    `hud_safe_area` has it read back too, against the pair's own `safe_area` when given, else the pin."""
    doc = tomllib.loads(settings_path(profile_dir).read_text())
    o = dict(plat["ours"])
    if safe_area is not None and "hud_safe_area" in o:
        o["hud_safe_area"] = int(safe_area)
    rp = doc.get("render_profiles", {}).get(plat["profile_key"], {})
    got = {"aspect": doc["display"]["aspect"], "brightness": doc["display"]["brightness"],
           "gamma": doc["display"]["gamma"], "anisotropy": doc["graphics"]["anisotropy"],
           "fov_percent": doc["graphics"]["fov"], "boost_fov_kick": doc["graphics"]["boost_fov_kick"]}
    for k in ("render_scale", "reconstruction", "msaa", "motion_blur", "shadows", "model_detail",
              "texture_detail", "trail_order", "screen_filter"):
        got[k] = rp.get(k)
    if "hud_safe_area" in o:
        got["hud_safe_area"] = doc["graphics"].get("hud_safe_area")
    want = dict(o, aspect=plat["aspect"])
    bad = ["ours setting %s is %r, the profile says %r" % (k, got[k], want[k])
           for k in got if got[k] != want[k]]
    return got, bad


def vec_angle(a, b):
    dot = sum(x * y for x, y in zip(a, b))
    n = math.dist(a, (0, 0, 0)) * math.dist(b, (0, 0, 0))
    return math.degrees(math.acos(max(-1.0, min(1.0, dot / n))))


def csv_row(path, tick):
    """One row of a trace CSV (the header line starts `tick,`; log lines around it are skipped)."""
    lines = Path(path).read_text().splitlines()
    start = next(i for i, ln in enumerate(lines) if ln.startswith("tick,"))
    head = lines[start].split(",")
    for ln in lines[start + 1:]:
        cells = ln.split(",")
        if len(cells) == len(head) and int(float(cells[0])) == tick:
            return dict(zip(head, (float(c) for c in cells)))
    raise SystemExit("compare_env: %s has no tick %d" % (path, tick))


def pose_of(row):
    return {"pos": [row["pos_x"], row["pos_y"], row["pos_z"]],
            "fwd": [row["fwd_x"], row["fwd_y"], row["fwd_z"]],
            "vel": [row["vel_x"], row["vel_y"], row["vel_z"]],
            "speed": row.get("speed")}


def speed_fov(row, base=60.0):
    """The original's vertical fov at this row: `base + 0.075 * dot(fwd, vel)`, `base` the view's authored fov (60 for
    the chase views, 65 for Pulse's `<InternalCamera>`; docs/rendering/projection-vs-the-original.md, confidence 94)."""
    dot = sum(row[f"fwd_{a}"] * row[f"vel_{a}"] for a in "xyz")
    return base + 0.075 * dot


def state_facts(state_path):
    """Mode, circuit, layout colour, class and team DECLARED by a saved state's file name
    (`time-trial-<circuit>-<white|black>-<class>-grid`, `hd-timetrial-<circuit>-<team>-...`)
    or its `.json` sidecar. Declared, not read out of the emulator's memory."""
    p = Path(state_path)
    side = Path(str(p) + ".json")
    if side.exists():
        data = json.loads(side.read_text())
        return {"source": "sidecar", **{k: data[k] for k in data if k in ("circuit", "mode", "class", "team", "layout")}}
    name = p.name.split(".")[0]
    facts = {"source": "file name", "name": name}
    if name.startswith("time-trial-"):
        facts["mode"] = "time_trial"
        bits = name[len("time-trial-"):].replace("-grid", "").split("-")
        facts["class"] = bits[-1] if bits[-1] in ("venom", "flash", "rapier", "phantom") else "venom"
        facts["colour"] = "black" if "black" in bits else "white"
        facts["circuit"] = "-".join(b for b in bits if b not in ("black", "white", facts["class"]))
    elif name.startswith("hd-timetrial-"):
        facts["mode"] = "time_trial"
        facts["circuit"] = name[len("hd-timetrial-"):].rsplit("-start", 1)[0]
    return facts


PSP_TRACE_COLUMNS = (
    "tick,dt,grounded,throttle,brake,steer,airbrake_l,airbrake_r,speed_cached,stun_timer,timer_2e0,shield,"
    "fov_intercept,fov_additive,controller_class,ss_tap_window_l,ss_tap_window_r,ss_shift_l,ss_shift_r,"
    "ss_lockout,steer_lean,cam_lean,cam_lean_follower,roll_phase,roll_eased,right_x,right_y,right_z,up_x,up_y,"
    "up_z,fwd_x,fwd_y,fwd_z,pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,speed,avel_x,avel_y,avel_z,omega_x,omega_y,"
    "omega_z,cam_right_x,cam_right_y,cam_right_z,cam_up_x,cam_up_y,cam_up_z,cam_fwd_x,cam_fwd_y,cam_fwd_z,"
    "cam_pos_x,cam_pos_y,cam_pos_z").split(",")


def write_trace_row(path, values):
    """A one-row trace CSV in the capture's own columns (what `oag-game --pose-from` reads); columns not
    in `values` are zero."""
    row = {c: 0.0 for c in PSP_TRACE_COLUMNS}
    row.update(values)
    Path(path).write_text(",".join(PSP_TRACE_COLUMNS) + "\n" + ",".join(repr(float(row[c])) for c in PSP_TRACE_COLUMNS) + "\n")


FIELD_COLUMNS = ("slot,team,left_x,left_y,left_z,up_x,up_y,up_z,fwd_x,fwd_y,fwd_z,pos_x,pos_y,pos_z,"
                 "vel_x,vel_y,vel_z").split(",")
FIELD_LEAN_COLUMNS = ["steer_lean", "roll_phase"]


def write_field_csv(path, rows, with_player=False):
    """`oag-game --field-from`'s CSV: the opponents in slot order, slot numbered 1..N, and with `with_player`
    a first row numbered 0 that carries the player's own exact pose. Every row carries the craft's velocity as the
    original's RAM held it (`vel`, units per second), which `--field-warmup` flies the exhaust history on. When every
    row has them (a Pulse stop reads them, `psp-race-field.py`; an older `field.json` and the RPCS3 path do not) it
    also carries the drawn lean and barrel-roll phase (`steer_lean`, `roll_phase`) that `--field-from` puts on each hull."""
    lean = all(c in r for r in rows for c in FIELD_LEAN_COLUMNS)
    lines = [",".join(FIELD_COLUMNS + (FIELD_LEAN_COLUMNS if lean else []))]

    def cells(n, r):
        values = (*r["left"], *r["up"], *r["fwd"], *r["pos"], *r["vel"])
        if lean:
            values += tuple(r[c] for c in FIELD_LEAN_COLUMNS)
        return ",".join([str(n), r["team"]] + [repr(v) for v in values])

    if with_player:
        lines.append(cells(0, next(r for r in rows if r["player"])))
    others = sorted((r for r in rows if not r["player"]), key=lambda r: r["slot"])
    for n, r in enumerate(others, 1):
        lines.append(cells(n, r))
    Path(path).write_text("\n".join(lines) + "\n")
    return [r["team"] for r in others]


#: A craft slower than this per tick has no measurable pause skew: its frame hull and RAM body coincide.
AT_REST_UNITS_PER_TICK = 0.05


def walked_hull_matrices(spans, start=0x1000):
    """Every craft-shaped world matrix the frame's command stream loads as vertex constants, as `ps3_pose.hull_matrices`
    yields them. `spans` are `(base_address, bytes)` of the dumped IO memory. Only what the walked stream loads counts: a
    dump also holds the next frame's push buffer, built a tick later, which a scan of raw memory finds too."""
    sys.path.insert(0, str(HERE))
    import ps3_pose
    import rsx_fifo

    walk = rsx_fifo.Walk(rsx_fifo.Mem([(base - 0x40000000, blob) for base, blob in spans]))
    for _pos, method, _noninc, args in walk.run(start):
        if method == 0x1efc and len(args) >= 17:
            yield from ps3_pose.hull_matrices(struct.pack(">%dI" % (len(args) - 1), *args[1:]))


def frame_craft_ticks(spans, player_pos, player_vel):
    """`(ticks, record)`: how many whole 60 Hz ticks the RAM player body is ahead of the hull the frame draws.

    An RPCS3 pause leaves RAM 1 or 2 whole ticks ahead of the push buffer for every craft and the camera alike
    (`docs/tools/compare-page.md`, "The skew is the pause, not the original's draw order"); a pair that takes its camera
    from the push buffer and its craft from RAM draws every craft that far up the road. A craft at rest reads 0. Raises
    `ValueError` when no hull is loaded near the body or the skew is not a whole number of ticks."""
    sys.path.insert(0, str(HERE))
    import ps3_pose

    if math.sqrt(sum(v * v for v in player_vel)) / 60.0 < AT_REST_UNITS_PER_TICK:
        return 0, {"craft_ticks": 0, "craft_basis": "the player is at rest: no skew to measure"}
    hull = ps3_pose.hull_near(list(walked_hull_matrices(spans)), player_pos)
    if hull is None:
        raise ValueError("the frame's command stream loads no hull matrix six times within 15 units of the player's body")
    ticks, off_axis = ps3_pose.whole_ticks(hull["translation"], player_pos, player_vel)
    return ticks, {"craft_ticks": ticks, "hull_translation": hull["translation"], "hull_loads": hull["count"],
                   "off_axis_units": off_axis, "ram_body": list(player_pos),
                   "craft_basis": "the hull world matrix the frame's command stream loads, against the RAM body, one pause"}


def rewound_field_rows(rows, ticks):
    """`--field-from` CSV rows (dicts) with every craft moved back `ticks` ticks of its own velocity."""
    out = []
    for r in rows:
        moved = dict(r)
        for a in "xyz":
            moved["pos_" + a] = repr(float(r["pos_" + a]) - float(r["vel_" + a]) * ticks / 60.0)
        out.append(moved)
    return out


def write_field_rows(path, rows):
    """`--field-from` CSV rows (dicts, one header) written the way the pair's own `field.csv` is."""
    import csv

    with Path(path).open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()), lineterminator="\n")
        w.writeheader()
        w.writerows(rows)


def read_field_rows(path):
    import csv

    with Path(path).open() as f:
        return list(csv.DictReader(f))


def film_frame_field(film, stem, field_in, field_out):
    """Write `field_out`: `field_in` (a film frame's RAM field, player in row 0) moved back to the frame's own instant,
    read off the frame's dumped command ring (`<film>/ring/<stem>-<addr>.bin`). Returns the skew record."""
    rows = read_field_rows(field_in)
    player = next(r for r in rows if r["slot"] == "0")
    pos = [float(player["pos_" + a]) for a in "xyz"]
    vel = [float(player["vel_" + a]) for a in "xyz"]
    spans = [(int(p.stem.rsplit("-", 1)[1], 16), p.read_bytes()) for p in sorted(Path(film, "ring").glob("%s-4*.bin" % stem))]
    ticks, record = frame_craft_ticks(spans, pos, vel)
    write_field_rows(field_out, rewound_field_rows(rows, ticks))
    return record


def check_not_rewound(pair_dir):
    """A pair whose record already names `craft_ticks` has a moved `field.csv`: moving it again would double the rewind."""
    rec = json.loads((Path(pair_dir) / "pair.json").read_text())
    done = (rec["original"].get("presented_lag") or {}).get("craft_ticks")
    if done is not None:
        raise ValueError("%s already records craft_ticks %s: its field.csv is not rewound a second time" % (pair_dir, done))


def record_craft_ticks(pair_dir, record, note, picture=None):
    """Merge a skew record into the pair's `original.presented_lag` with the note saying how `field.csv` moved; `picture`
    replaces the record's statement about the picture's own instant."""
    pj = Path(pair_dir) / "pair.json"
    rec = json.loads(pj.read_text())
    lag = rec["original"].get("presented_lag") or {}
    lag.update(record)
    lag["craft_note"] = note
    if picture is not None:
        lag["picture"] = picture
    rec["original"]["presented_lag"] = lag
    pj.write_text(json.dumps(rec, indent=2) + "\n")


def film_pair_field(film, stem, pair_dir):
    """Move a film pair's `field.csv` to its frame's instant in place and record the skew in its `pair.json`
    (`original.presented_lag.craft_ticks` and how it was read). Returns the record."""
    pair_dir = Path(pair_dir)
    check_not_rewound(pair_dir)
    tmp = pair_dir / "field.csv.new"
    record = film_frame_field(film, stem, pair_dir / "field.csv", tmp)
    tmp.replace(pair_dir / "field.csv")
    record_craft_ticks(pair_dir, record, "field.csv moved back craft_ticks ticks of each craft's own velocity to the "
                                         "frame's instant (compare_env.py film-pair-field)")
    return record


def rewind_pair_field(pair_dir, ticks):
    """Move the `field.csv` of a pair with no dumped command ring back `ticks` whole ticks of each craft's own velocity
    and record it. The count is not read from the frame: it is picked by rendering ours at each rewind against the
    original (`scripts/hd-pair-lag-scan.py <pair> 0:0 0:1 0:2`), which the record says."""
    ticks = int(ticks)
    if ticks < 1:
        raise ValueError("ticks must be a whole number of ticks the craft are ahead, got %d" % ticks)
    pair_dir = Path(pair_dir)
    check_not_rewound(pair_dir)
    write_field_rows(pair_dir / "field.csv", rewound_field_rows(read_field_rows(pair_dir / "field.csv"), ticks))
    record = {"craft_ticks": ticks,
              "craft_basis": "picked by rendering ours with every craft moved back whole ticks and scoring each against "
                             "theirs.png (hd-pair-lag-scan.py); no command ring of the frame was kept"}
    record_craft_ticks(pair_dir, record, "field.csv moved back craft_ticks ticks of each craft's own velocity "
                                         "(compare_env.py rewind-pair-field); the camera is not moved",
                       picture="field.csv moved back to the picture's instant for the craft; the camera is the "
                               "recorded push-buffer matrix, unmoved")
    return record


def selftest_frame_field():
    """A hull two ticks behind the body, loaded by the walked stream, rewinds every craft two ticks; a hull only in memory
    the stream never reaches is not the frame's; a craft at rest reads none."""
    body, vel = (529.36, -25.02, -31.0), (98.3, 16.65, 86.59)

    def world(ticks):
        return [0.75, 0, 0, 0, 0, 0.75, 0, 0, 0, 0, 0.75, 0] + [b - ticks * v / 60.0 for b, v in zip(body, vel)] + [1.0]

    load = struct.pack(">I", (17 << 18) | 0x1efc) + struct.pack(">I", 256) + struct.pack(">16f", *world(2))
    stream = bytearray(0x1000) + load * 8 + bytes(4 * 300)
    stray = bytearray(0x400)
    for k in range(8):
        stray[k * 100:k * 100 + 64] = struct.pack(">16f", *world(1))
    spans = [(0x40000000, bytes(stream)), (0x40400000, bytes(stray))]
    ticks, record = frame_craft_ticks(spans, body, vel)
    assert ticks == 2 and record["hull_loads"] == 8, record
    other = {"slot": "1", "pos_x": "10.0", "pos_y": "0.0", "pos_z": "0.0", "vel_x": "60.0", "vel_y": "0.0", "vel_z": "0.0"}
    player = {"slot": "0", **{"pos_" + a: repr(b) for a, b in zip("xyz", body)}, **{"vel_" + a: repr(v) for a, v in zip("xyz", vel)}}
    moved = rewound_field_rows([player, other], ticks)
    assert max(abs(float(moved[0]["pos_" + a]) - f) for a, f in zip("xyz", world(2)[12:15])) < 1e-4, moved[0]
    assert abs(float(moved[1]["pos_x"]) - 8.0) < 1e-9, moved[1]
    assert frame_craft_ticks(spans, body, (0.09, 0.34, 0.06))[0] == 0
    try:
        frame_craft_ticks([(0x40000000, bytes(0x1400)), (0x40400000, bytes(stray))], body, vel)
    except ValueError:
        pass
    else:
        raise AssertionError("a hull the stream never loads must not be read as the frame's")
    selftest_rewind_pair_field()


def selftest_rewind_pair_field():
    """A pair with no ring: `field.csv` moves every craft back by its own velocity, the record names the count and
    replaces the claim that the picture needed no correction, and a second run is refused."""
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        d = Path(tmp)
        write_field_rows(d / "field.csv", [{"slot": str(n), "pos_x": "10.0", "pos_y": "0.0", "pos_z": "0.0",
                                            "vel_x": repr(60.0 * (n + 1)), "vel_y": "0.0", "vel_z": "0.0"} for n in range(2)])
        (d / "pair.json").write_text(json.dumps({"original": {"presented_lag": {"hud_clock_ticks": 3, "picture": "none"}}}))
        rewind_pair_field(d, "2")
        assert [float(r["pos_x"]) for r in read_field_rows(d / "field.csv")] == [8.0, 6.0]
        lag = json.loads((d / "pair.json").read_text())["original"]["presented_lag"]
        assert lag["craft_ticks"] == 2 and lag["hud_clock_ticks"] == 3 and lag["picture"] != "none", lag
        try:
            rewind_pair_field(d, 2)
        except ValueError:
            pass
        else:
            raise AssertionError("a second rewind of the same field.csv must be refused")
        assert float(read_field_rows(d / "field.csv")[0]["pos_x"]) == 8.0


def pack_check(pair, place, cam_eye, cam_fwd, player_pos, others, place_range=(3, 6), near=45.0,
               max_dist=110.0, half_fov=28.0, min_in_view=2, min_near=1):
    """A race shot is only a race shot when the frame IS a pack: the player's place in `place_range`, at least
    `min_in_view` other craft inside the camera's cone (within `max_dist` of the eye) and `min_near` of those within
    `near` of the player. Checked on the captured RAM, recorded, and a miss REFUSES the pair (the capture instant
    cannot be chosen: a restored state runs a few ticks before it is stopped, so rerun it)."""
    cos_limit = math.cos(math.radians(half_fov))
    seen, close = [], []
    for pos in others:
        d = [p - e for p, e in zip(pos, cam_eye)]
        dist = math.sqrt(sum(x * x for x in d))
        if 6.0 <= dist <= max_dist and sum(a * b for a, b in zip(d, cam_fwd)) / dist >= cos_limit:
            seen.append(round(dist, 1))
            if math.dist(pos, player_pos) <= near:
                close.append(round(math.dist(pos, player_pos), 1))
    pair.rec["pack"] = {"place": place, "in_view": seen, "near_player": close, "criteria": {
        "place": list(place_range), "near": near, "max_dist": max_dist, "half_fov": half_fov,
        "min_in_view": min_in_view, "min_near": min_near}}
    return pair.check("the frame is a pack (player %d-%d, %d+ craft in view, %d+ within %g units)"
                      % (place_range[0], place_range[1], min_in_view, min_near, near),
                      place_range[0] <= place <= place_range[1] and len(seen) >= min_in_view and len(close) >= min_near,
                      "place %s, craft in view at %s from the camera, near the player at %s" % (place, seen, close))


def set_ours_weapons_off(profile_dir):
    """`[race] weapons = "Off"` in the private settings file, whether or not an earlier run of `oag-game`
    already rewrote the file with its own `[race]` table (a second `[race]` header is a TOML error)."""
    path = settings_path(profile_dir)
    lines = path.read_text().splitlines()
    start = next((i for i, ln in enumerate(lines) if ln.strip() == "[race]"), None)
    if start is None:
        lines += ["", "[race]", 'weapons = "Off"']
    else:
        end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith("[")), len(lines))
        hit = next((i for i in range(start + 1, end) if lines[i].split("=")[0].strip() == "weapons"), None)
        if hit is None:
            lines.insert(start + 1, 'weapons = "Off"')
        else:
            lines[hit] = 'weapons = "Off"'
    path.write_text("\n".join(lines) + "\n")


def git_head():
    return subprocess.run(["git", "-C", str(ROOT), "rev-parse", "--short", "HEAD"],
                          capture_output=True, text=True).stdout.strip()


def git_dirty():
    out = subprocess.run(["git", "-C", str(ROOT), "status", "--porcelain", "--untracked-files=no"],
                         capture_output=True, text=True).stdout
    return bool(out.strip())


def build_hash(path):
    """The commit hash the binary itself reports (`oag-game --version` reads `0.1.0 (abc1234)`), or None
    when it embedded none (`unknown`, or `test-data` from a `just test-data` build)."""
    try:
        out = subprocess.run([str(path), "--version"], capture_output=True, text=True, timeout=30).stdout
    except (OSError, subprocess.TimeoutExpired):
        return None
    if "(" not in out or ")" not in out:
        return None
    h = out[out.rindex("(") + 1:out.rindex(")")].strip()
    return None if h in ("", "unknown", "test-data") else h


def binary_record(path):
    p = Path(path)
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 22), b""):
            h.update(chunk)
    return {"path": str(p), "sha256_12": h.hexdigest()[:12], "tree_head": git_head(), "tree_dirty": git_dirty(),
            "build": build_hash(p)}


def build_text(binary):
    """`build abc1234` from the binary's own hash, else the tree's HEAD at render time (`+dirty` when the
    tracked tree had changes), so a pair names the code that drew it."""
    h = binary.get("build")
    if h:
        return "build %s" % h
    head = binary.get("tree_head")
    if not head:
        return None
    return "build %s%s (tree at render)" % (head, "+dirty" if binary.get("tree_dirty") else "")


#: Where the original's display sits that ours (centred, as in play) does not: `(profile, pair kind)` to the offset.
#: Pulse PS2 sets `DISPLAY1/2.DY = 52` and PCSX2 puts frame row `r` on output row `r + (DY - 50)`: its NTSC default `DY` is 50,
#: so the picture sits exactly 2 frame rows low. A synthetic frame (flat grey, white bars at known rows and columns, the game's
#: own display registers) replayed on two dumps gave 2.000 rows at every bar from row 20 to 410 (scale 2.000, no x offset) and 0.000
#: with `DY = 50` (`docs/ps2/aspect-ratio.md`, "Measured against PCSX2's frame"). A shift past 2 rows would absorb ours' own
#: sub-pixel content residual against PCSX2's draw of textured quads (about half an output pixel in both axes, at every upscale
#: and output size measured), which is not display.
#: Compare shots only: play keeps the console port centred.
DISPLAY_OFFSETS = {
    ("pulse-ps2", "menu"): {
        "frame_rows": 2.0, "frame_height": 448, "direction": "down",
        "basis": "measured (confidence 90): game DISPLAY1/2.DY = 52 less PCSX2's default 50, bar probe exact on two dumps "
                 "(scale 2.000, 0.000 at DY 50), PCSX2 default anti-blur",
    },
}


def display_offset_record(profile, kind):
    """The offset record for a pair of `profile` and `kind`, or `None` (every other pair is drawn unshifted)."""
    rec = DISPLAY_OFFSETS.get((profile, kind))
    return dict(rec) if rec else None


def apply_display_offset(png, offset):
    """Shift `png` down by `offset` (a `DISPLAY_OFFSETS` record) in place, bicubic, black above; return pixels moved."""
    from PIL import Image
    img = Image.open(png).convert("RGB")
    px = offset["frame_rows"] * img.size[1] / offset["frame_height"]
    out = img.transform(img.size, Image.AFFINE, (1, 0, 0, 0, 1, -px), resample=Image.BICUBIC, fillcolor=(0, 0, 0))
    out.save(png)
    return round(px, 3)


def image_size(path):
    out = subprocess.run(["magick", "identify", "-format", "%w %h", str(path) + "[0]"],
                         capture_output=True, text=True, check=True).stdout.split()
    return [int(out[0]), int(out[1])]


def toast_over_frame(png, crop="120x10+8+4"):
    """True when PPSSPP's green on-screen notification ("Game controller connected: ...", raised when ANY pad appears on the
    machine, such as another lane's virtual pad) covers the top-left of a 480x272 frame. The toast is a solid, saturated
    green banner (about 0, 110, 35); the scenes' own greens are lit and never reach red 0. `crop` is inside the banner
    and clear of the lap counter. One Outpost 7 White grab in 50 carried it (mean grey difference 6.2 to 13.7)."""
    out = subprocess.run(["magick", str(png), "-crop", crop, "+repage", "-format",
                          "%[fx:mean.r] %[fx:mean.g] %[fx:mean.b] %[fx:standard_deviation]", "info:"],
                         capture_output=True, text=True, check=True).stdout.split()
    r, g, b, sd = (float(v) for v in out)
    return r < 0.08 and g > 0.30 and b < 0.25 and sd < 0.12


def overlap_score(theirs, ours):
    """A cheap overlap score of two same-size frames: normalised RMSE of the pictures, and the normalised
    cross-correlation of their blurred edge maps (1.0 = the edges lie on each other; a pair that lines up badly
    reads low). Returns {} when ImageMagick cannot score them."""
    out = {}
    try:
        r = subprocess.run(["compare", "-metric", "RMSE", str(theirs), str(ours), "null:"], capture_output=True, text=True)
        out["rmse"] = float(r.stderr.split("(")[1].split(")")[0])
        raws = []
        for p in (theirs, ours):
            r = subprocess.run(["magick", str(p), "-resize", "960x960>", "-colorspace", "Gray", "-blur", "0x1.2", "-edge", "1",
                                "-depth", "8", "gray:-"], capture_output=True, check=True)
            raws.append(r.stdout)
        a, b = raws
        n = len(a)
        ma, mb = sum(a) / n, sum(b) / n
        sab = sum((x - ma) * (y - mb) for x, y in zip(a, b))
        saa = sum((x - ma) ** 2 for x in a)
        sbb = sum((y - mb) ** 2 for y in b)
        out["edge_ncc"] = sab / ((saa * sbb) ** 0.5) if saa and sbb else 0.0
        # mean absolute grey difference over the frame, 0-255 (the number the handover threads quote)
        g = subprocess.run(["magick", str(theirs), str(ours), "-colorspace", "Gray", "-compose", "difference", "-composite",
                            "-format", "%[fx:mean*255]", "info:"], capture_output=True, text=True, check=True)
        out["mean_grey"] = float(g.stdout)
    except Exception as exc:  # noqa: BLE001 - a score is a convenience, never a reason to lose a pair
        out["error"] = str(exc)
    return out


class Pair:
    """One pair's recorded environment and its checks; `finish` refuses on any failed check."""

    def __init__(self, plat_key, out):
        self.plat = platform(plat_key)
        self.out = Path(out)
        self.rec = {"profile": plat_key, "profile_sha": self.plat["profile_sha"],
                    "output": self.plat["output"], "original": {}, "ours": {}}
        self.checks = []
        self.rec["caveats"] = []
        # how this pair was made, so `compare-rerender.py --recapture-original` can run the same capture again
        self.rec["argv"] = [Path(sys.argv[0]).name] + sys.argv[1:]

    def caveat(self, text):
        """A difference the pair KNOWS about and was allowed to ship with; the manifest shows it."""
        self.rec["caveats"].append(text)
        print("CAVEAT %s" % text, file=sys.stderr)

    def stamp(self, side):
        """Record the wall-clock time `side`'s picture was made: `original.captured_at` or `ours.rendered_at`, ISO 8601
        with the local offset. Taken from the clock at the moment it is called and never from a file time, so a pair
        made before this existed shows no time at all."""
        key = {"original": "captured_at", "ours": "rendered_at"}[side]
        self.rec[side][key] = datetime.datetime.now().astimezone().isoformat(timespec="seconds")

    def check(self, name, ok, detail):
        self.checks.append({"name": name, "ok": bool(ok), "detail": detail})
        print("%s %s: %s" % ("ok     " if ok else "MISMATCH", name, detail), file=sys.stderr)
        return ok

    def failed(self):
        return [c for c in self.checks if not c["ok"]]

    def check_size(self, side, path):
        got = image_size(path)
        self.rec[side]["image_size"] = got
        return self.check("%s image size" % side, got == list(self.plat["output"]),
                          "%dx%d, profile %s" % (got[0], got[1], size_text(self.plat)))

    def check_pose(self, theirs, ours, fov_theirs=None, fov_ours=None):
        tol = self.plat["tolerance"]
        d = math.dist(theirs["pos"], ours["pos"])
        a = vec_angle(theirs["fwd"], ours["fwd"])
        self.rec["original"]["pose"], self.rec["ours"]["pose"] = theirs, ours
        self.check("ship position at the rendered tick", d <= tol["pos_units"],
                   "%.3f units apart (tolerance %.2f)" % (d, tol["pos_units"]))
        self.check("ship heading at the rendered tick", a <= tol["heading_deg"],
                   "%.3f deg apart (tolerance %.2f)" % (a, tol["heading_deg"]))
        if fov_theirs is not None:
            self.rec["original"]["fov_deg"], self.rec["ours"]["fov_deg"] = fov_theirs, fov_ours
            self.check("field of view", abs(fov_theirs - fov_ours) <= tol["fov_deg"],
                       "%.3f vs %.3f deg (tolerance %.2f)" % (fov_theirs, fov_ours, tol["fov_deg"]))

    def check_equal(self, name, theirs, ours):
        return self.check(name, theirs == ours, "original %r, ours %r" % (theirs, ours))

    def finish(self):
        import compare_provenance as cp  # the provenance rules are pair checks like the rest
        cp.finish(self)
        bad = self.failed()
        if (self.out / "theirs.png").exists() and (self.out / "ours.png").exists():
            self.rec["overlap"] = overlap_score(self.out / "theirs.png", self.out / "ours.png")
        self.rec["refused"] = bool(bad)
        # The pair scripts capture the original and render ours inside one run, so both are stamped here unless a
        # script stamped its side earlier, at the moment the picture was written.
        for side, key in (("original", "captured_at"), ("ours", "rendered_at")):
            if key not in self.rec[side]:
                self.stamp(side)
        self.rec["environment"] = environment_text(self.plat, self.rec)
        (self.out / "pair.json").write_text(json.dumps(self.rec, indent=2) + "\n")
        if bad:
            print("REFUSED: %s" % "; ".join("%s (%s)" % (c["name"], c["detail"]) for c in bad), file=sys.stderr)
            raise SystemExit(3)
        print("parity checks passed: %d" % len(self.checks), file=sys.stderr)


def environment_text(plat, rec):
    e = plat["emulator"]
    o = plat["ours"]
    bits = ["profile %s@%s" % (plat["key"], plat["profile_sha"]),
            "output %s (%s)" % (size_text(plat), plat["output_note"]),
            "original: %s %s %s" % (e["name"], e.get("version", ""), e.get("renderer", "")),
            "ours: %s, render scale %d, %s, shadows %s, motion blur %s, aspect %s, HUD %s, camera %s"
            % (size_text(plat), o["render_scale"], "no upscaler" if o["reconstruction"] == "off" else o["reconstruction"],
               o["shadows"], o["motion_blur"], plat["aspect"], o["hud"], o["camera"])]
    if "pilot_assist" in o:
        bits.append("Pilot Assist %s on both sides (%s)" % (o["pilot_assist"], "the original's frame draws its indicator"
                    if o["pilot_assist"] != "off" else "the original's frame draws no indicator"))
    if "exposure_hold" in o:
        bits.append("exposure held at %s on ours (Vita3K never reads the original's gate back, so its exposure never "
                    "leaves %s; a Vita and ours in play adapt)" % (o["exposure_hold"], o["exposure_hold"]))
    if "hud_safe_area" in o:
        got = recorded_safe_area(rec)
        bits.append("Safe Area %s%% on both sides (%s)" % ((got, rec["original"]["safe_area"]["source"]) if got is not None
                    else (o["hud_safe_area"], "the profile's pin: no value recorded for this pair")))
    ov = rec.get("overlap") or {}
    if "edge_ncc" in ov:
        bits.append("overlap: edge correlation %.2f, RMSE %.3f" % (ov["edge_ncc"], ov["rmse"]))
    binary = rec.get("ours", {}).get("binary")
    if binary:
        build = build_text(binary)
        if build:
            bits.append(build)
        bits.append("binary %s" % binary["sha256_12"])
    return "; ".join(bits)


METRICS = (
    ("rmse", "RMSE", "%.3f", "Average per-pixel colour error between the two pictures. 0 is identical; lower is better."),
    ("edge_ncc", "Edge correlation", "%.2f",
     "Compares where edges are in the two pictures: 1 is identical, higher is better. It shows whether shapes line up, "
     "independent of colour."),
    ("mean_grey", "Mean grey difference", "%.2f",
     "Average brightness difference between the two pictures on a 0-255 scale; lower is better."),
)


#: Colour bands for the page: (green up to, amber up to) for a lower-is-better metric, (green from, amber from) for
#: edge correlation. Chosen, not measured: set on 2026-10-21 against the spread of the 42 pairs then (Pulse PSP grid
#: pairs mostly green, the HD pairs amber to red), so a band reads as "close", "partly", "off".
METRIC_BANDS = {
    "rmse": {"better": "lower", "green": 0.06, "amber": 0.12},
    "edge_ncc": {"better": "higher", "green": 0.40, "amber": 0.25},
    "mean_grey": {"better": "lower", "green": 5.0, "amber": 12.0},
}

def environment_fields(rec):
    """The environment as structured fields, built from what the pair RECORDED (`pair.json`), never from the live
    profile: a pair made under an older profile keeps showing the one it was made under. A field the pair does not
    carry is absent, so the page shows nothing for it rather than a guess."""
    f = {}
    if rec.get("profile"):
        f["profile"] = "%s@%s" % (rec["profile"], rec.get("profile_sha", "?"))
    if rec.get("output"):
        f["output"] = "%dx%d" % tuple(rec["output"])
    emu = rec.get("original", {}).get("emulator") or {}
    if emu.get("name"):
        f["original_emulator"] = " ".join(x for x in (emu["name"], emu.get("version", "")) if x)
    if emu.get("renderer"):
        f["original_renderer"] = emu["renderer"]
    ours = rec.get("ours", {})
    rb = ours.get("settings_readback") or {}
    if rb:
        f["ours_settings"] = ", ".join("%s %s" % (k.replace("_", " "), v) for k, v in rb.items())
    flags = ours.get("flags") or []
    if "--hold-exposure" in flags[:-1]:
        f["ours_exposure"] = ("exposure held at %s (a comparison switch: the original under its emulator never adapts; "
                              "in play ours does)" % flags[flags.index("--hold-exposure") + 1])
    sa = rec.get("original", {}).get("safe_area")
    if sa:
        f["safe_area"] = "%s%% (%s)" % (sa["percent"], sa["source"])
    ov = rec.get("overlap") or {}
    for key, _, fmt, _ in METRICS:
        if key in ov:
            f[key] = fmt % ov[key]
    binary = ours.get("binary")
    if binary:
        build = build_text(binary)
        if build:
            f["build"] = build.replace("build ", "", 1)
        f["binary"] = binary["sha256_12"]
    if rec.get("original", {}).get("captured_at"):
        f["captured_at"] = rec["original"]["captured_at"]
    if ours.get("rendered_at"):
        f["rendered_at"] = ours["rendered_at"]
    return f


def toml_str(s):
    return '"' + str(s).replace("\\", "\\\\").replace('"', '\\"') + '"'


def refuse_pair(pj, why):
    sys.exit("compare_env: %s: %s" % (pj, why))


def missing_safe_area(dirs):
    """The pairs under `dirs` whose profile pins a `hud_safe_area` and whose `pair.json` records no `original.safe_area`;
    each is printed. `python3 scripts/compare_env.py missing-safe-area DIR...` exits 1 when there are any."""
    out = []
    for d in dirs:
        for pj in sorted(Path(d).rglob("pair.json")):
            rec = json.loads(pj.read_text())
            if "hud_safe_area" in platform(rec["profile"])["ours"] and recorded_safe_area(rec) is None:
                out.append(pj)
                print("no original.safe_area: %s" % pj)
    return out


def manifest(out_path, dirs):
    """A page manifest with one entry per non-refused `pair.json` under `dirs`, paths relative to it."""
    out = Path(out_path).resolve()
    entries = []
    for d in dirs:
        for pj in sorted(Path(d).rglob("pair.json")):
            rec = json.loads(pj.read_text())
            if rec.get("refused"):
                print("skipped (refused): %s" % pj, file=sys.stderr)
                continue
            plat = platform(rec["profile"])
            if "hud_safe_area" in plat["ours"] and recorded_safe_area(rec) is None:
                print("WARNING no original.safe_area on %s: drawn at the profile's %d%%" % (pj, plat["ours"]["hud_safe_area"]),
                      file=sys.stderr)
            # Resolved like `out`: in a worktree `data/` is a symlink into the main checkout, and
            # relpath from an unresolved pair to a resolved manifest names the worktree, which is
            # reaped after its merge.
            base = pj.parent.resolve()
            if "mean_grey" not in (rec.get("overlap") or {}):
                rec["overlap"] = overlap_score(base / "theirs.png", base / "ours.png")
            m = rec.get("meta") or {}
            if not m.get("title") or not m.get("id"):
                refuse_pair(pj, "no `meta` with an id and a title: the page shows a pair's own words, never its directory name")
            rel = lambda name: Path(__import__("os").path.relpath(base / name, out.parent))
            entries.append("\n".join([
                "[[entry]]",
                "id = %s" % toml_str(m["id"]),
                "game = %s" % toml_str(plat["game"]),
                "platform = %s" % toml_str(plat["platform"]),
                "title = %s" % toml_str(m["title"]),
                "original = %s" % toml_str(rel("theirs.png")),
                "ours = %s" % toml_str(rel("ours.png")),
                "track = %s" % toml_str(m.get("track", "")),
                "craft = %s" % toml_str(m.get("craft", "")),
                "state = %s" % toml_str(m.get("state", "")),
                "setting = %s" % toml_str("%s %s, %s; ours %s" % (plat["emulator"]["name"], plat["emulator"].get("version", ""),
                                                                 size_text(plat), size_text(plat))),
                "note = %s" % toml_str(" ".join([m.get("note", "")] + [("Not matched: " + c.rstrip(".") + ".") for c in rec.get("caveats", [])]).strip()),
                "",
                "[entry.environment]",
                *("%s = %s" % (k, toml_str(v)) for k, v in environment_fields(rec).items()),
            ]))
    out.write_text('[page]\ntitle = "Original vs ours"\nintro = "Fresh set of 2026-10-20: environment parity checked per pair (see Environment)."\n\n'
                   + "\n\n".join(entries) + "\n")
    print("wrote %d entries to %s" % (len(entries), out))


def main():
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    if cmd == "show":
        print(json.dumps(platform(sys.argv[2]), indent=2))
    elif cmd == "ppsspp-ini":
        print(ppsspp_ini(platform(sys.argv[2])), end="")
    elif cmd == "manifest":
        manifest(sys.argv[2], sys.argv[3:])
    elif cmd == "selftest":
        assert gantry_pin(None, 3) is None
        assert abs(gantry_pin(4.368715286254883, 3) - 4.318715) < 1e-5
        assert gantry_pin(5.0, 0) == 5.0
        assert detonator_input_lines(None, 272) == ["273 none"]
        fire = {"press_clock_before_s": 1.8667, "press_clock_after_s": 1.9}
        assert detonator_input_lines(fire, 272) == ["384 none", "2 square", "1 none"]
        armed = {"address": 1, "flags": 9, "age_s": 3.0}
        shot = {"address": 2, "flags": 1, "age_s": 2.5}
        gone = {"address": 3, "flags": 1, "age_s": 0.5}
        reads = [{"mines": {"1": {"age_s": 1.0}, "2": {"age_s": 1.0}, "3": {"age_s": 0.5}}}]
        assert detonator_pair_mines([armed, shot, gone], reads) == [armed, shot]
        assert detonator_pair_mines([armed, shot, gone]) == [armed]
        selftest_frame_field()
        print("compare_env selftest ok")
    elif cmd == "film-field":
        print(json.dumps(film_frame_field(*sys.argv[2:6])))
    elif cmd == "film-pair-field":
        print(json.dumps(film_pair_field(*sys.argv[2:5])))
    elif cmd == "rewind-pair-field":
        print(json.dumps(rewind_pair_field(*sys.argv[2:4])))
    elif cmd == "missing-safe-area":
        sys.exit(1 if missing_safe_area(sys.argv[2:]) else 0)
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()


#: `PlayerStatus_Update`'s held id (`record+0x1bc`) to the weapon's `type`
#: (`docs/ghidra/functions/psp-pulse-usa/pickup-colour.md`, "The 13 values"); -1 is empty.
PULSE_HELD_ID_TYPES = ["Rocket", "Missile", "Quake", "Cannon", "Turbo", "Shield", "Autopilot", "Plasma", "Mine", "Bomb",
                       "LeachBeam", "Repulser", "Shuriken"]


def shield_flash_flags(hud):
    """`["--hud-shield-flash", "TIMER,BLINK"]` for a pair record's `original.hud` that read the shield bar's flash timer
    above zero (Pulse PSP `hud+0x11c`, with the blink accumulator `hud+0x1dc`), or `[]`: the bar draws forced red while the
    timer runs, and ours' posed capture runs no tick that arms its own."""
    timer = (hud or {}).get("shield_flash_s")
    if timer is None or not timer > 0.0:
        return []
    return ["--hud-shield-flash", "%.6f,%.6f" % (timer, hud.get("shield_blink_s") or 0.0)]


def held_pickup_flags(held_word):
    """`["--hud-pickup", TYPE]` for a Pulse PSP held id, or `[]` when the slot is empty or the id is not a weapon's."""
    if held_word is None or not 0 <= held_word < len(PULSE_HELD_ID_TYPES):
        return []
    return ["--hud-pickup", PULSE_HELD_ID_TYPES[held_word]]
