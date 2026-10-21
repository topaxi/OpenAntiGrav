#!/usr/bin/env python3
"""How a comparison pair's ORIGINAL was made, and the rules that refuse an untrustworthy one.

    compare_provenance.py selftest                 # one constructed bad record per refusal rule; needs no data
    compare_provenance.py scan [DIR]               # every pair.json under DIR (default data/reference/compare-page,
                                                   #   or $OAG_PAIR_DIR) carries provenance, breaks no rule, names no
                                                   #   worktree or scratch path; skipped loudly when DIR is absent
    compare_provenance.py tracked                  # ratchet: tracked files outside scripts/ and .claude/ naming
                                                   #   `data/scratch/` or `OpenAntiGrav-worktrees/` may not grow
    compare_provenance.py backfill SRC DST         # copy SRC (pair dirs, `cp -rL`) to DST and fill what can be read
    compare_provenance.py table DIR                # markdown table of every pair's provenance and its refusals
    compare_provenance.py portable DST             # rewrite a copy's pair.json paths (no worktree or scratch path)

The pair scripts (`psp-pulse-pair.py`, `pcsx2-pulse-pair.py`, `rpcs3-hd-pair.py`) take `add_arguments` and
`Pair.finish` calls `finish` below, so a refusal is one more `pair.check` and exits 3 like every other one.
Docs: docs/tools/compare-page.md, "Provenance: the trust model".

Never invent: a fact nothing recorded is `unknown`, and `unknown` is a refusal where a rule needs it.
"""

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
MAIN = Path("/home/topaxi/projects/OpenAntiGrav")
DEFAULT_DIR = "data/reference/compare-page"
SCHEMA = 1

CAPTURE_KINDS = ("cold_boot", "savestate_restore", "unknown")
RULES = {
    "hd_savestate_scenery": "an HD scenery or glass pair needs a cold boot; a restored savestate paints garbage on glass "
                            "(docs/reverse-engineering/rpcs3-capture.md); the HD race pair is the one labelled exception",
    "autopilot_timer": "an armed Autopilot's timer was not the authored one at the grab (the HUD shows it), or a race or "
                       "zone state, always flown on the Autopilot, did not record it",
    "unknown_lag": "the presented-frame lag is unknown, so the pose and the picture may be different ticks",
}

#: A path to somewhere that is reaped: another lane's worktree, or the scratch directory.
OFFENDER = re.compile(r"OpenAntiGrav-worktrees/|data/scratch/")
SCRATCH_RE = re.compile(r"(?:/[^\s\"']*?)?data/scratch/[^/\s\"']+/?")
WORKTREE_RE = re.compile(r"/[^\s\"']*?OpenAntiGrav-worktrees/[^/\s\"']+/")


def refusals(prov, profile, pair_kind=None):
    """`[(rule, detail)]` for every rule the record breaks and does not override. An override is
    `{"rule": name, "reason": text}` in `prov["overrides"]`, recorded in the pair, never silent."""
    pair_kind = pair_kind or prov.get("pair_kind", "scenery")
    out = []
    cp = prov.get("capture_path", {})
    if profile == "hd-fury" and pair_kind != "race" and cp.get("kind") != "cold_boot":
        out.append(("hd_savestate_scenery", "capture_path is %s, %s" % (cp.get("kind", "missing"), pair_kind)))
    ap = prov.get("autopilot", {})
    armed = ap.get("armed", "unknown")
    if armed is True and ap.get("timer_authored") is not True:
        out.append(("autopilot_timer", "armed, timer written %r, authored: %r" % (ap.get("timer_written"), ap.get("timer_authored"))))
    elif armed != True and armed is not False and pair_kind in ("race", "zone"):  # noqa: E712 - "unknown" is a string
        out.append(("autopilot_timer", "a %s state is flown on the Autopilot and its arming was not recorded" % pair_kind))
    lag = prov.get("presented_lag", {})
    if lag.get("value") is None or lag.get("basis", "unknown") == "unknown":
        out.append(("unknown_lag", "presented_lag %s" % json.dumps(lag)))
    accepted = {o.get("rule") for o in prov.get("overrides", []) if o.get("reason")}
    return [(r, d) for r, d in out if r not in accepted]


def path_offenders(obj, where=""):
    """`[(json path, string)]` for every string naming a worktree or a scratch directory."""
    found = []
    if isinstance(obj, dict):
        for k, v in obj.items():
            found += path_offenders(v, "%s.%s" % (where, k))
    elif isinstance(obj, list):
        for i, v in enumerate(obj):
            found += path_offenders(v, "%s[%d]" % (where, i))
    elif isinstance(obj, str) and OFFENDER.search(obj):
        found.append((where.lstrip("."), obj))
    return found


def portable_text(s):
    """A string with no reaped path in it: scratch becomes `{scratch}/`, a worktree and the main checkout the
    repository-relative path (every worktree is a checkout of the same tree, `data/` linked into the main one)."""
    s = SCRATCH_RE.sub("{scratch}/", s)
    s = WORKTREE_RE.sub("", s)
    return s.replace(str(MAIN) + "/", "")


def portable(obj):
    if isinstance(obj, dict):
        return {k: portable(v) for k, v in obj.items()}
    if isinstance(obj, list):
        return [portable(v) for v in obj]
    if isinstance(obj, str):
        return portable_text(obj)
    return obj


def settings_sha(emulator):
    keys = {k: v for k, v in emulator.items() if k not in ("capture_note",)}
    return hashlib.sha256(json.dumps(keys, sort_keys=True).encode()).hexdigest()[:12]


def add_arguments(ap):
    """The pair script's provenance flags; every one is recorded in `pair.json` `argv`, so a re-make repeats them."""
    g = ap.add_argument_group("provenance")
    g.add_argument("--autopilot", choices=("off", "authored", "timer"), default=None,
                   help="how the original's Autopilot stood at the grab: off, armed with the authored timer, or armed with a "
                        "timer this run's state-making wrote (`--autopilot-timer`); unset records `unknown`")
    g.add_argument("--autopilot-timer", type=float, help="the timer written, seconds (with --autopilot timer)")
    g.add_argument("--ram-write", action="append", default=[], metavar="TARGET=VALUE:WHY",
                   help="a memory write the state-making made before the grab, repeatable (weapons off, a flag, a timer)")
    g.add_argument("--accept-provenance", action="append", default=[], metavar="RULE=REASON",
                   help="override one refusal rule, recorded in the pair (%s)" % ", ".join(RULES))
    return g


def build(pair, args, kind, capture_kind, state_file=None, lag=None):
    """Fill `pair.rec["original"]["provenance"]` from what the script knows. `lag` is the script's own record:
    `{"value", "unit", "basis", "what"}`."""
    rec = pair.rec
    plat = pair.plat
    ap = {"armed": "unknown", "timer_written": None, "timer_authored": None, "source": "not given on the command line"}
    if args.autopilot == "off":
        ap = {"armed": False, "timer_written": None, "timer_authored": None, "source": "--autopilot off"}
    elif args.autopilot == "authored":
        ap = {"armed": True, "timer_written": "authored", "timer_authored": True, "source": "--autopilot authored"}
    elif args.autopilot == "timer":
        ap = {"armed": True, "timer_written": args.autopilot_timer if args.autopilot_timer is not None else "huge (value not recorded)",
              "timer_authored": False, "source": "--autopilot timer"}
    writes = []
    for item in args.ram_write:
        target, _, rest = item.partition("=")
        value, _, why = rest.partition(":")
        writes.append({"target": target, "value": value, "why": why, "when": "before the grab", "source": "--ram-write"})
    overrides = []
    for item in args.accept_provenance:
        rule, _, reason = item.partition("=")
        if rule not in RULES or not reason:
            sys.exit("--accept-provenance wants RULE=REASON with RULE one of %s" % ", ".join(RULES))
        overrides.append({"rule": rule, "reason": reason})
    emulator = rec["original"].get("emulator", {})
    rec["original"]["provenance"] = {
        "schema": SCHEMA, "pair_kind": kind, "basis": "recorded by the pair script",
        "capture_path": {"kind": capture_kind, "state_file": state_file},
        "presented_lag": lag or {"value": None, "unit": None, "basis": "unknown"},
        "autopilot": ap, "ram_writes": writes, "pair_script_ram_writes": [],
        "emulator": {"name": emulator.get("name", plat["emulator"]["name"]), "version": emulator.get("version"),
                     "settings_sha": settings_sha(emulator)},
        "twin": {"agreed": None, "rmse_vs_previous": None, "basis": "no second capture of this pair"},
        "overrides": overrides,
    }


def finish(pair):
    """Called by `Pair.finish`: each rule is a `pair.check`, and the record is made portable before it is written."""
    prov = pair.rec["original"].get("provenance")
    if prov is None:
        pair.check("provenance recorded", False, "the pair script recorded none (compare_provenance.build)")
    else:
        broken = {r: d for r, d in refusals(prov, pair.rec["profile"])}
        for rule in RULES:
            if rule in broken:
                pair.check("provenance: " + rule, False, "%s; override with --accept-provenance %s=REASON" % (broken[rule], rule))
        for o in prov.get("overrides", []):
            pair.caveat("provenance rule %s overridden: %s" % (o["rule"], o["reason"]))
    pair.rec.update(portable(pair.rec))


def note_twin(new, drift):
    """After a re-make: the new capture against the previous one is the pair's twin comparison. The number is
    recorded; no threshold is documented, so `agreed` stays unknown."""
    prov = new.get("original", {}).get("provenance")
    if prov is not None:
        prov["twin"] = {"agreed": None, "rmse_vs_previous": drift.get("theirs_rmse_vs_previous"),
                        "basis": "re-capture of the same state on the same emulator; no agreement threshold is documented"}


def backfill_pair(rec, rel):
    """Provenance read from today's `pair.json` (and the facts the docs record about how its state was made).
    Nothing is invented: what was never recorded is `unknown`."""
    o = rec["original"]
    argv = rec.get("argv", [])
    profile = rec["profile"]
    kind = "zone" if "--zone" in argv or "zone" in o else "race" if "--field" in argv or "pack" in rec else "scenery"
    state = o.get("state") if isinstance(o.get("state"), dict) else {}
    sfile = portable_text(state.get("file", "")) or None
    emu = o.get("emulator", {})
    if profile == "hd-fury":
        raw = o.get("capture_path")
        if raw:
            ck = "cold_boot" if str(raw).startswith("cold boot") else "savestate_restore"
            cbasis = "recorded in pair.json capture_path"
        else:
            ck, cbasis = "savestate_restore", ("inferred: the state is a SAVESTAT and compare-rerender-all restored it (its --hd-cold "
                                               "arrived afterwards); pair.json predates the capture_path field")
        lag = {"value": 3, "unit": "ticks", "basis": "measured",
               "what": "HUD clock text vs RAM (13 grid pairs); picture not corrected: the RSX had presented the paused frame "
                       "(hull check on hd-race/talons-pack), per-tick picture lag not measured"}
    elif profile == "pulse-ps2":
        ck, cbasis = "savestate_restore", "pcsx2-pulse-pair.py always loads the .p2s state"
        lag = {"value": 4.25, "unit": "seconds", "basis": "measured",
               "what": "the HUD lap clock is the RAM race clock less 4.25 s (3 grabs of one state); the craft rests, so its pose needs no lag"}
    else:
        ck, cbasis = "savestate_restore", "psp-pulse-pair.py always loads the .ppst state (psp-state.py load)"
        n = o.get("presented_lag_ticks")
        if n is not None:
            lag = {"value": n, "unit": "ticks", "basis": "measured", "what": "picture, psp-presented-lag.py, 7 of 7 shots"}
        else:
            lag = {"value": 1, "unit": "ticks", "basis": "platform_measurement",
                   "what": "PPSSPP v1.20.4 picture lag measured on a race state; this pair predates presented_lag_ticks"}
    ap = {"armed": "unknown", "timer_written": None, "timer_authored": None, "source": "not recorded by the pair or its state"}
    writes = []
    if kind == "race" and profile == "hd-fury":
        ap = {"armed": True, "timer_written": 10000, "timer_authored": False,
              "source": "docs/tools/compare-page.md, HD race recipe: *(ship+0x5edc)+0x200 |= 0x800, +0x158 = 10000"}
        writes = [{"target": "*(ship+0x5edc)+0x200", "value": "|= 0x800", "why": "arm the Autopilot", "when": "state making, over serve memw",
                   "source": "docs recipe"},
                  {"target": "*(ship+0x5edc)+0x158", "value": "10000", "why": "Autopilot timer", "when": "state making", "source": "docs recipe"}]
    elif kind == "race":
        ap = {"armed": True, "timer_written": "huge (value not recorded)", "timer_authored": False,
              "source": "docs/tools/compare-page.md, race recipe: psp-race-field.py hunt, fire bit 0x1000, timer record+0x148 written huge"}
        writes = [{"target": "pickup record fire bit", "value": "0x1000", "why": "arm the Autopilot", "when": "state making", "source": "docs recipe"},
                  {"target": "record+0x148", "value": "huge (not recorded)", "why": "Autopilot timer", "when": "state making", "source": "docs recipe"}]
    elif kind == "zone":
        ap = {"armed": True, "timer_written": None, "timer_authored": None,
              "source": "docs/tools/compare-page.md, Zone pairs: the state is made flying on the Autopilot; timer not recorded"}
    elif profile == "pulse-ps2":
        writes = [{"target": "DAT_00284fe8 (EE RAM)", "value": "1", "why": "the game's widescreen option on", "when": "state making, over PINE, 4 frames advanced",
                   "source": "state sidecar"}]
    drift = rec.get("drift") or {}
    twin = {"agreed": None, "rmse_vs_previous": drift.get("theirs_rmse_vs_previous"),
            "basis": "re-capture of the same state on the same emulator; no agreement threshold is documented" if drift
            else "no second capture of this pair"}
    return {"schema": SCHEMA, "pair_kind": kind, "basis": "backfilled from pair.json, argv, docs and the state sidecar (not recorded by the script)",
            "capture_path": {"kind": ck, "state_file": sfile, "basis": cbasis}, "presented_lag": lag, "autopilot": ap,
            "ram_writes": writes, "ram_writes_complete": "unknown", "pair_script_ram_writes": [],
            "emulator": {"name": emu.get("name"), "version": emu.get("version"), "settings_sha": settings_sha(emu)},
            "twin": twin, "overrides": []}


def pairs_under(d):
    return sorted(Path(d).rglob("pair.json"))


def backfill(src, dst):
    import shutil
    dst = Path(dst)
    if dst.exists():
        shutil.rmtree(dst)
    subprocess.run(["cp", "-rL", str(src), str(dst)], check=True)
    n = 0
    for pj in pairs_under(dst):
        rec = json.loads(pj.read_text())
        prov = rec["original"]["provenance"] = backfill_pair(rec, pj.parent.relative_to(dst))
        argv = rec.get("argv", [])
        if "--out" in argv:
            argv[argv.index("--out") + 1] = "%s/%s" % (DEFAULT_DIR, pj.parent.relative_to(dst))
        if prov["pair_kind"] == "race" and "--autopilot" not in argv:
            argv += ["--autopilot", "timer"] + (["--autopilot-timer", "10000"] if rec["profile"] == "hd-fury" else [])
        pj.write_text(json.dumps(portable(rec), indent=2) + "\n")
        n += 1
    print("backfilled %d pair.json under %s" % (n, dst))


def portable_dir(dst):
    """Rewrite a copy's `pair.json` and `*.toml` manifests so no path names a worktree or scratch directory."""
    n = 0
    for pj in pairs_under(dst):
        rec = json.loads(pj.read_text())
        pj.write_text(json.dumps(portable(rec), indent=2) + "\n")
        n += 1
    for t in Path(dst).glob("*.toml"):
        t.write_text(portable_text(t.read_text()))
    print("made %d pair.json under %s portable" % (n, dst))


def scan(d):
    d = Path(d)
    if not d.is_dir():
        print("check-pair-provenance: SKIPPED the pair scan, %s does not exist (no data/ here)" % d)
        return 0
    bad = 0
    files = pairs_under(d) + sorted(d.glob("*.toml"))
    for f in files:
        if f.suffix == ".toml":
            for i, line in enumerate(f.read_text().splitlines(), 1):
                if OFFENDER.search(line):
                    print("PATH   %s:%d names a worktree or scratch path" % (f, i))
                    bad += 1
            continue
        rec = json.loads(f.read_text())
        for where, s in path_offenders(rec):
            print("PATH   %s %s = %s" % (f, where, s[:90]))
            bad += 1
        prov = rec.get("original", {}).get("provenance")
        if prov is None:
            print("MISSING %s has no original.provenance" % f)
            bad += 1
            continue
        for rule, detail in refusals(prov, rec["profile"]):
            print("REFUSED %s %s: %s" % (f.parent.relative_to(d), rule, detail))
            bad += 1
    print("check-pair-provenance: %d pair.json, %d problem(s)" % (len([f for f in files if f.suffix == ".json"]), bad))
    return 1 if bad else 0


BASELINE_FILE = HERE / "provenance-path-baseline.txt"


def tracked_counts():
    out = subprocess.run(["git", "grep", "-c", "-E", "data/scratch/|OpenAntiGrav-worktrees/", "--", ".", ":!scripts", ":!.claude", ":!justfile",
                          ":!scripts/provenance-path-baseline.txt"], cwd=ROOT, capture_output=True, text=True).stdout
    return {l.rsplit(":", 1)[0]: int(l.rsplit(":", 1)[1]) for l in out.splitlines() if l}


def tracked(write=False):
    now = tracked_counts()
    if write:
        BASELINE_FILE.write_text("".join("%s\t%d\n" % (k, v) for k, v in sorted(now.items())))
        print("wrote %d rows to %s" % (len(now), BASELINE_FILE))
        return 0
    base = {}
    for l in BASELINE_FILE.read_text().splitlines():
        k, _, v = l.partition("\t")
        base[k] = int(v)
    grew = [(k, base.get(k, 0), v) for k, v in sorted(now.items()) if v > base.get(k, 0)]
    for k, was, v in grew:
        print("GREW   %s names data/scratch/ or a worktree %d time(s), baseline %d: cite the evidence itself, not a path that is reaped" % (k, v, was))
    shrunk = sum(1 for k, v in base.items() if now.get(k, 0) < v)
    print("check-pair-provenance: tracked-file ratchet, %d files over, %d below baseline (lower the baseline: --write-baseline)" % (len(grew), shrunk))
    return 1 if grew else 0


def table(d):
    print("| pair | kind | capture | state | lag | autopilot | writes | twin rmse | refused by |")
    print("| --- | --- | --- | --- | --- | --- | --- | --- | --- |")
    d = Path(d)
    for pj in pairs_under(d):
        rec = json.loads(pj.read_text())
        p = rec["original"].get("provenance")
        name = str(pj.parent.relative_to(d))
        if not p:
            print("| %s | - | MISSING | | | | | | |" % name)
            continue
        cp, lag, ap = p["capture_path"], p["presented_lag"], p["autopilot"]
        apt = "off" if ap["armed"] is False else "unknown" if ap["armed"] == "unknown" else "armed, timer %s%s" % (
            ap["timer_written"], "" if ap.get("timer_authored") is not False else " (not authored)")
        rm = (p["twin"].get("rmse_vs_previous") or "-").split(" ")[0]
        ref = ", ".join(r for r, _ in refusals(p, rec["profile"])) or "-"
        print("| %s | %s | %s | %s | %s %s (%s) | %s | %d%s | %s | %s |" % (
            name, p["pair_kind"], cp["kind"], Path(cp["state_file"] or "-").name[:34], lag["value"], lag["unit"], lag["basis"], apt,
            len(p["ram_writes"]), "" if p.get("ram_writes_complete") != "unknown" else "+?", rm, ref))


def selftest():
    good_hd = {"pair_kind": "scenery", "capture_path": {"kind": "cold_boot"}, "presented_lag": {"value": 3, "unit": "ticks", "basis": "measured"},
               "autopilot": {"armed": False}, "overrides": []}
    assert refusals(good_hd, "hd-fury") == [], "a clean HD cold pair must pass"
    bad = dict(good_hd, capture_path={"kind": "savestate_restore"})
    assert [r for r, _ in refusals(bad, "hd-fury")] == ["hd_savestate_scenery"], "HD scenery from a restore"
    assert refusals(dict(bad, pair_kind="race"), "hd-fury") == [], "the HD race pair is the labelled exception"
    assert [r for r, _ in refusals(dict(bad, capture_path={"kind": "unknown"}, pair_kind="zone"), "hd-fury")] == ["hd_savestate_scenery"], "HD zone is not the race exception"
    assert refusals(bad, "pulse-psp") == [], "the cold-boot rule is HD's alone"
    armed = dict(good_hd, autopilot={"armed": True, "timer_written": 1e9, "timer_authored": False})
    assert [r for r, _ in refusals(armed, "pulse-psp")] == ["autopilot_timer"], "armed with a non-authored timer"
    assert refusals(dict(armed, autopilot={"armed": True, "timer_written": "authored", "timer_authored": True}), "pulse-psp") == [], "authored timer passes"
    assert [r for r, _ in refusals(dict(good_hd, pair_kind="race", autopilot={"armed": "unknown"}), "pulse-psp")] == ["autopilot_timer"], "race state, arming unrecorded"
    for lag in ({}, {"value": None}, {"value": 1, "unit": "ticks", "basis": "unknown"}):
        assert [r for r, _ in refusals(dict(good_hd, presented_lag=lag), "hd-fury")] == ["unknown_lag"], "unknown lag: %r" % lag
    over = dict(armed, overrides=[{"rule": "autopilot_timer", "reason": "state predates the field, looked at"}])
    assert refusals(over, "pulse-psp") == [], "an explicit override with a reason passes"
    assert [r for r, _ in refusals(dict(armed, overrides=[{"rule": "autopilot_timer", "reason": ""}]), "pulse-psp")] == ["autopilot_timer"], "an override needs a reason"
    assert path_offenders({"a": ["x", {"b": "/h/OpenAntiGrav-worktrees/lane/t"}]}) == [("a[1].b", "/h/OpenAntiGrav-worktrees/lane/t")]
    assert path_offenders({"a": "data/scratch/lane/x"}) and not path_offenders({"a": "data/saves/x"})
    fixed = portable({"a": "/home/topaxi/projects/OpenAntiGrav-worktrees/lane/target/debug/oag-game", "b": "--home /x/OpenAntiGrav/data/scratch/lane/pp/home",
                      "c": str(MAIN) + "/data/saves/pulse-psp/s.ppst"})
    assert fixed == {"a": "target/debug/oag-game", "b": "--home {scratch}/pp/home", "c": "data/saves/pulse-psp/s.ppst"}, fixed
    assert not path_offenders(fixed)
    import tempfile
    import types
    sys.path.insert(0, str(HERE))
    import compare_env as ce
    with tempfile.TemporaryDirectory() as tmp:
        def finished(profile, kind, capture, autopilot=None, extra=()):
            pair = ce.Pair(profile, tmp)
            ns = types.SimpleNamespace(autopilot=autopilot, autopilot_timer=1e9 if autopilot == "timer" else None, ram_write=[],
                                       accept_provenance=list(extra))
            build(pair, ns, kind, capture, None, {"value": 1, "unit": "ticks", "basis": "measured", "what": "t"})
            try:
                pair.finish()
            except SystemExit as exc:
                return exc.code, json.loads((Path(tmp) / "pair.json").read_text())
            return 0, json.loads((Path(tmp) / "pair.json").read_text())
        code, rec = finished("hd-fury", "scenery", "savestate_restore", "off")
        assert code == 3 and rec["refused"], "Pair.finish must exit 3 on an HD scenery pair from a restore"
        assert finished("hd-fury", "scenery", "cold_boot", "off")[0] == 0, "a clean cold pair passes through Pair.finish"
        assert finished("hd-fury", "race", "savestate_restore", "authored")[0] == 0, "the race exception passes"
        assert finished("pulse-psp", "race", "savestate_restore", "timer")[0] == 3, "a huge Autopilot timer refuses"
        code, rec = finished("pulse-psp", "race", "savestate_restore", "timer", ["autopilot_timer=looked at, state predates the flag"])
        assert code == 0 and rec["original"]["provenance"]["overrides"][0]["rule"] == "autopilot_timer", "the override is recorded in the pair"
    print("selftest ok: 3 refusal rules, the exception, the override and the path rule")
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("selftest")
    s = sub.add_parser("scan")
    s.add_argument("dir", nargs="?", default=os.environ.get("OAG_PAIR_DIR", str(ROOT / DEFAULT_DIR)))
    t = sub.add_parser("tracked")
    t.add_argument("--write-baseline", action="store_true")
    b = sub.add_parser("backfill")
    b.add_argument("src")
    b.add_argument("dst")
    tb = sub.add_parser("table")
    tb.add_argument("dir")
    p = sub.add_parser("portable")
    p.add_argument("dst")
    a = ap.parse_args()
    if a.cmd == "selftest":
        sys.exit(selftest())
    if a.cmd == "scan":
        sys.exit(scan(a.dir))
    if a.cmd == "tracked":
        sys.exit(tracked(a.write_baseline))
    if a.cmd == "backfill":
        backfill(a.src, a.dst)
    if a.cmd == "table":
        table(a.dir)
    if a.cmd == "portable":
        portable_dir(a.dst)


if __name__ == "__main__":
    main()
