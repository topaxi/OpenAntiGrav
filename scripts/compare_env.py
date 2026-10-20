#!/usr/bin/env python3
"""The environment profile of the comparison pairs, and the checks that enforce it.

    python3 scripts/compare_env.py show pulse-psp
    python3 scripts/compare_env.py ppsspp-ini pulse-psp        # the [Graphics] text for PPSSPP
    python3 scripts/compare_env.py manifest OUT.toml DIR...   # one [[entry]] per non-refused pair.json

`scripts/compare-environment.toml` is the profile. The pair scripts (`psp-pulse-pair.py`,
`pcsx2-pulse-pair.py`, `rpcs3-hd-pair.py`) import this module: they build a `Pair`, record
what each side actually ran with, and `Pair.finish` writes `pair.json` and REFUSES (exit 3,
every mismatch named) a pair whose environment differs from the profile or whose two
sides differ beyond the profile's tolerance. `manifest` carries the environment into the
compare page's manifest, so the page shows it.
"""

import hashlib
import json
import math
import re
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
    out["tolerance"] = dict(doc["tolerance"])
    out["profile_sha"] = hashlib.sha256(PROFILE.read_bytes()).hexdigest()[:12]
    return out


def size_text(plat):
    return "%dx%d" % tuple(plat["output"])


def ppsspp_ini(plat):
    """The `[Graphics]` lines the instance's ini carries for this profile (PPSSPP picture settings)."""
    e = plat["emulator"]
    return ("[Graphics]\nSoftwareRenderer = True\nInternalResolution = %d\nAnisotropyLevel = 0\n"
            "FrameSkip = %d\nTextureFiltering = 1\nPostShaderList =\n"
            % (e["internal_resolution"], e["frameskip"]))


def ours_flags(plat):
    """`oag-game` flags that carry the profile's render settings (every one that has a flag)."""
    o = plat["ours"]
    return ["--no-audio", "--size", size_text(plat),
            "--render-scale", str(o["render_scale"]), "--reconstruction", o["reconstruction"],
            "--msaa", o["msaa"], "--motion-blur", o["motion_blur"],
            "--lod", o["model_detail"], "--texture-detail", o["texture_detail"],
            "--screen-filter", o["screen_filter"], "--anisotropy", o["anisotropy"]]


def ours_env(profile_dir):
    return {"XDG_CONFIG_HOME": str(profile_dir / "cfg"), "XDG_DATA_HOME": str(profile_dir / "data"),
            "XDG_STATE_HOME": str(profile_dir / "state")}


def settings_path(profile_dir):
    return profile_dir / "cfg" / "oag" / "settings.toml"


def write_ours_settings(plat, profile_dir):
    """Write the private settings.toml: the keys with no command-line flag (shadows `original`,
    boost fov kick, fov percent, aspect, brightness/gamma), plus the rest, so the file the run
    rewrites can be read back and compared."""
    o = plat["ours"]
    for sub in ("cfg/oag", "data", "state"):
        (profile_dir / sub).mkdir(parents=True, exist_ok=True)
    text = ("[display]\naspect = \"%s\"\nbrightness = %d\ngamma = %d\n\n"
            "[graphics]\nanisotropy = \"%s\"\nfov = %d\nboost_fov_kick = %d\n\n"
            "[render_profiles.\"%s\"]\nrender_scale = %d\nreconstruction = \"%s\"\nmsaa = \"%s\"\n"
            "motion_blur = \"%s\"\nshadows = \"%s\"\nmodel_detail = \"%s\"\ntexture_detail = \"%s\"\n"
            "screen_filter = \"%s\"\n"
            % (plat["aspect"], o["brightness"], o["gamma"], "16x" if o["anisotropy"] == "16x" else o["anisotropy"],
               o["fov_percent"], o["boost_fov_kick"], plat["profile_key"], o["render_scale"],
               o["reconstruction"], o["msaa"], o["motion_blur"], o["shadows"], o["model_detail"],
               o["texture_detail"], o["screen_filter"]))
    settings_path(profile_dir).write_text(text)


def read_back(plat, profile_dir):
    """What ours ran with, read out of the settings file the run rewrote, and the mismatches."""
    doc = tomllib.loads(settings_path(profile_dir).read_text())
    o = plat["ours"]
    rp = doc.get("render_profiles", {}).get(plat["profile_key"], {})
    got = {"aspect": doc["display"]["aspect"], "brightness": doc["display"]["brightness"],
           "gamma": doc["display"]["gamma"], "anisotropy": doc["graphics"]["anisotropy"],
           "fov_percent": doc["graphics"]["fov"], "boost_fov_kick": doc["graphics"]["boost_fov_kick"]}
    for k in ("render_scale", "reconstruction", "msaa", "motion_blur", "shadows", "model_detail",
              "texture_detail", "screen_filter"):
        got[k] = rp.get(k)
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


def speed_fov(row):
    """The original's vertical fov at this row: `60 + 0.075 * dot(fwd, vel)`
    (docs/rendering/projection-vs-the-original.md, confidence 94)."""
    dot = sum(row[f"fwd_{a}"] * row[f"vel_{a}"] for a in "xyz")
    return 60.0 + 0.075 * dot


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


def git_head():
    return subprocess.run(["git", "-C", str(ROOT), "rev-parse", "--short", "HEAD"],
                          capture_output=True, text=True).stdout.strip()


def binary_record(path):
    p = Path(path)
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 22), b""):
            h.update(chunk)
    return {"path": str(p), "sha256_12": h.hexdigest()[:12], "tree_head": git_head()}


def image_size(path):
    out = subprocess.run(["magick", "identify", "-format", "%w %h", str(path) + "[0]"],
                         capture_output=True, text=True, check=True).stdout.split()
    return [int(out[0]), int(out[1])]


class Pair:
    """One pair's recorded environment and its checks; `finish` refuses on any failed check."""

    def __init__(self, plat_key, out):
        self.plat = platform(plat_key)
        self.out = Path(out)
        self.rec = {"profile": plat_key, "profile_sha": self.plat["profile_sha"],
                    "output": self.plat["output"], "original": {}, "ours": {}}
        self.checks = []

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
        bad = self.failed()
        self.rec["checks"] = self.checks
        self.rec["refused"] = bool(bad)
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
    if rec.get("ours", {}).get("binary"):
        bits.append("binary %s" % rec["ours"]["binary"]["sha256_12"])
    return "; ".join(bits)


def toml_str(s):
    return '"' + str(s).replace("\\", "\\\\").replace('"', '\\"') + '"'


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
            base = pj.parent
            rel = lambda name: Path(__import__("os").path.relpath(base / name, out.parent))
            m = rec.get("meta", {})
            entries.append("\n".join([
                "[[entry]]",
                "id = %s" % toml_str(m.get("id", base.name)),
                "game = %s" % toml_str(plat["game"]),
                "platform = %s" % toml_str(plat["platform"]),
                "title = %s" % toml_str(m.get("title", base.name)),
                "original = %s" % toml_str(rel("theirs.png")),
                "ours = %s" % toml_str(rel("ours.png")),
                "track = %s" % toml_str(m.get("track", "")),
                "craft = %s" % toml_str(m.get("craft", "")),
                "state = %s" % toml_str(m.get("state", "")),
                "setting = %s" % toml_str("%s %s, %s; ours %s" % (plat["emulator"]["name"], plat["emulator"].get("version", ""),
                                                                 size_text(plat), size_text(plat))),
                "environment = %s" % toml_str(rec["environment"]),
                "note = %s" % toml_str(m.get("note", "")),
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
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
