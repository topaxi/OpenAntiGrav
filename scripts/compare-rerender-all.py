#!/usr/bin/env python3
"""Re-make every comparison pair with one command: boot each platform's emulator, restore each pair's state, re-run
the pair's script, stop the emulator, and print one before/after table.

    just compare-rerender-all [--out DIR] [--platform psp|rpcs3|pcsx2 ...] [--only SUBSTRING] [--resume] [--dry-run]

For each platform that has pairs under SRC (`data/reference/compare-page` by default) this

1. starts the platform's emulator headless, on its own Xvfb, HOME, port and config, from the profile in
   `scripts/compare-environment.toml` (PPSSPP software renderer with `compare_env.py ppsspp-ini`; RPCS3 per the HD
   profile, a private config tree from `scripts/emu-env.sh`; PCSX2 booted with `OAG_COMPARE_PLATFORM`);
2. per pair, restores its savestate the way the pair was made (RPCS3: `emu-restore-state.sh`, `--pause` for a race pair
   whose script reads the field at the state's own tick, `--restart` for a Time Trial taken after `Restart Race`;
   PPSSPP and PCSX2 load the state inside the pair script) and runs `scripts/compare-rerender.py`'s `remake`, which
   re-runs the pair script from the `argv` in its `pair.json` with the CURRENT `oag-game`: pose, camera and clock are
   read out of the emulator's RAM, never taken from a stored file;
3. stops the emulator by the PID its start script recorded;
4. rebuilds the page manifests (`fresh.toml` for the grids, `race-shots.toml` for the race shots) next to the pairs.

`--out DIR` copies SRC to DIR first and re-makes the copy; nothing under SRC is written (the default re-makes SRC in
place). A pair whose fresh run is refused by a parity check, or fails, keeps its old files and makes the exit status
non-zero. `--resume` skips a pair this DIR's `rerender-all.json` already holds as done, so a run that stopped halfway
(a stalled emulator, the tool's time limit) continues. The table is printed at the end and written to
`DIR/rerender-all.md`: per pair the drift of the original's own state between the two captures and the overlap
scores (`edge_ncc`, `rmse`, mean grey difference over the frame, 0-255), before and after, and the mean per platform.

Environment per platform, all overridable: `--lane` names the scratch directory `data/scratch/<lane>/`; PPSSPP takes
`--psp-display 93 --psp-port 9317`, RPCS3 `--rpcs3-display 94 --rpcs3-gdb 2349`, PCSX2 `--pcsx2-display 92 --pcsx2-slot 28015`.
No emulator is ever started on the maintainer's display and every one is muted.
"""

import argparse
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402

_spec = importlib.util.spec_from_file_location("compare_rerender", HERE / "compare-rerender.py")
rerender = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(rerender)

#: The emulator that makes each profile's pairs.
PLATFORM_OF = {"pulse-psp": "psp", "pure-psp": "psp", "hd-fury": "rpcs3", "pulse-ps2": "pcsx2"}
#: The manifests the page is built from: which pair directories each lists.
MANIFESTS = {"fresh.toml": ("pulse-psp", "hd", "pulse-ps2"), "race-shots.toml": ("pulse-psp-race", "hd-race")}
PAIR_TIMEOUT = 540


def log(msg):
    print("[rerender-all] " + msg, file=sys.stderr, flush=True)


def sh(cmd, env=None, check=True, timeout=None, **kw):
    log("+ " + " ".join(str(c) for c in cmd))
    return subprocess.run([str(c) for c in cmd], env=env, check=check, timeout=timeout, **kw)


def source_env(path):
    """The variables a `source`d env.sh sets or changes (not the whole environment, which would put the
    maintainer's own XDG directories back over the private ones)."""
    def read(cmd):
        out = subprocess.run(["bash", "-c", cmd, "_", str(path)], capture_output=True, check=True)
        return dict(kv.split("=", 1) for kv in out.stdout.decode().split("\0") if "=" in kv)
    before = read("env -0")
    after = read('set -a; source "$1"; env -0')
    return {k: v for k, v in after.items() if before.get(k) != v}


class Emulator:
    """One platform's emulator and the environment its pair scripts run in."""

    def __init__(self, args, scratch):
        self.args, self.scratch = args, scratch

    def base_env(self):
        env = dict(os.environ)
        for var in ("WAYLAND_DISPLAY",):
            env.pop(var, None)
        profile = self.scratch / "profile"
        for sub in ("cfg", "data", "state"):
            (profile / sub).mkdir(parents=True, exist_ok=True)
        env.update(XDG_CONFIG_HOME=str(profile / "cfg"), XDG_DATA_HOME=str(profile / "data"),
                   XDG_STATE_HOME=str(profile / "state"))
        return env

    def start(self):
        raise NotImplementedError

    def stop(self):
        raise NotImplementedError

    def prepare(self, rec):
        """Put the emulator in the pair's state (when the pair script does not) and return `(env, remap)`."""
        return self.env, self.remap


class Psp(Emulator):
    def start(self):
        a = self.args
        w = self.scratch / "pp"
        ini = self.scratch / "pp.ini"
        ini.write_text(ce.ppsspp_ini(ce.platform("pulse-psp")))
        env = self.base_env()
        env["PPSSPP_EXTRA_INI"] = str(ini)
        sh([HERE / "ppsspp-start.sh", a.lane, a.psp_display, a.psp_port, ce.MAIN / ce.platform("pulse-psp")["image"]], env=env)
        env = {**self.base_env(), **source_env(w / "env.sh")}
        env["OAG_SHOT_DISPLAY"] = env["DISPLAY"]
        self.env = env
        self.remap = {"--port": a.psp_port, "--home": env["PP_HOME"]}
        from ppsspp_debugger import Debugger
        for _ in range(60):
            try:
                Debugger(a.psp_port)
                return
            except Exception:  # noqa: BLE001 - the debugger comes up a few seconds after the window
                time.sleep(1)
        raise SystemExit("PPSSPP's debugger never came up on port %s" % a.psp_port)

    def stop(self):
        sh([HERE / "ppsspp-start.sh", self.args.lane, "stop"], check=False)


class Rpcs3(Emulator):
    def start(self):
        a = self.args
        sh([HERE / "emu-env.sh", a.lane, a.rpcs3_display, a.rpcs3_gdb])
        env = {**self.base_env(), **source_env(self.scratch / "env.sh")}
        env.update(OAG_COMPARE_PLATFORM="hd-fury", UV_CACHE_DIR=str(Path.home() / ".cache/uv"), UV_OFFLINE="1")
        self.env = env
        self.remap = {}

    #: Track Select presses from the first circuit, by pair directory (docs/reverse-engineering/rpcs3-capture.md), and Ship Select's
    #: `right` presses per `--team` (Assegai 7, Harimau 9; `_c1` is the Fury hull, the classic hull is one `down` more).
    CIRCUITS = {"vineta-k": 0, "anulpha-pass": 1, "moa-therma": 2, "chenghou-project": 3, "metropia": 4, "sebenco-climb": 5,
                "ubermall": 6, "sol-2": 7, "talons": 8, "talons-assegai-c1": 8, "the-amphiseum": 9, "modesto-heights": 10,
                "tech-de-ra": 11}
    TEAMS = {"harimau": (9, 1), "Assegai_c1": (7, 0)}

    def prepare(self, rec):
        argv = rec["argv"]
        state = argv[argv.index("--state") + 1]
        env = dict(self.env)
        env.pop("OAG_RPCS3_ATTACH", None)
        pair_dir = Path(argv[argv.index("--out") + 1]).name
        team = argv[argv.index("--team") + 1] if "--team" in argv else ""
        if "--field" not in argv and self.args.hd_cold and pair_dir in self.CIRCUITS and team in self.TEAMS:
            # A restored savestate paints garbage on HD's glass; a cold boot to the same race draws it clean
            # (docs/reverse-engineering/rpcs3-capture.md, 2026-10-21): boot, walk Time Trial, Restart Race.
            rights, classic = self.TEAMS[team]
            sh([HERE / "emu-hd-glass-boot.sh", self.args.lane, self.CIRCUITS[pair_dir], rights, classic, "cold-" + pair_dir],
               env=env, timeout=600)
            attach = dict(env, OAG_RPCS3_ATTACH="1")
            sh(["uv", "run", "--with", "evdev", "python3", HERE / "emu-run.py", "--", "python3", "-u", HERE / "rpcs3-drive.py",
                "--log-dir", self.scratch / "restart", "restart"], env=attach, timeout=300)
            path = "cold boot (no savestate): the glass stays clean"
            kind = "cold_boot"
        else:
            flag = "--pause" if "--field" in argv else "--restart"
            sh([HERE / "emu-restore-state.sh", self.args.lane, state, flag], env=env, timeout=300)
            path = "restored savestate (RPCS3 paints garbage on the glass of a restored state; the original is no reference there)"
            kind = "savestate_restore"
        return dict(self.env, OAG_RPCS3_ATTACH="1", OAG_HD_CAPTURE_PATH=path, OAG_HD_CAPTURE_KIND=kind), self.remap

    def stop(self):
        sh(["python3", HERE / "rpcs3-drive.py", "stop"], env=self.env, check=False)


class Pcsx2(Emulator):
    def start(self):
        a = self.args
        data = self.scratch / "ps2"
        data.mkdir(parents=True, exist_ok=True)
        env = self.base_env()
        env.update(OAG_PCSX2_DISPLAY=str(a.pcsx2_display), OAG_PCSX2_DATA=str(data), OAG_COMPARE_PLATFORM="pulse-ps2")
        # HOME stays the real one: the BIOS is read from ~/.config/PCSX2/bios, and everything PCSX2 writes goes to the
        # private OAG_PCSX2_DATA directory
        self.env = env
        self.remap = {"--slot": a.pcsx2_slot}
        image = ce.MAIN / ce.platform("pulse-ps2")["image"]
        sh(["uv", "run", "--with", "python-xlib", "python3", HERE / "pcsx2-drive.py", "--slot", a.pcsx2_slot, "boot",
            "--image", image], env=env, timeout=300)

    def stop(self):
        sh(["python3", HERE / "pcsx2-drive.py", "--slot", self.args.pcsx2_slot, "stop"], env=self.env, check=False)


EMULATORS = {"psp": Psp, "rpcs3": Rpcs3, "pcsx2": Pcsx2}


def backfill_meta(root):
    """A pair made before `--meta` existed has no `meta` in its `pair.json`, so a regenerated manifest would name it by
    its directory. Take the hand-written fields (id, title, track, craft, state, note) from the manifest entry that lists
    the pair's `theirs.png`, and keep them in `pair.json` (`meta`) and in its recorded `argv` (`--meta`), so every
    later re-make and every regenerated manifest carries them. Returns the pairs it filled."""
    import tomllib
    entries = {}
    for toml in root.glob("*.toml"):
        for e in tomllib.loads(toml.read_text()).get("entry", []):
            if e.get("original"):
                entries[(toml.parent / e["original"]).resolve().parent] = e
    filled = []
    for pj in root.rglob("pair.json"):
        rec = json.loads(pj.read_text())
        e = entries.get(pj.parent.resolve())
        if rec.get("meta") or e is None:
            continue
        meta = {k: e[k] for k in ("id", "title", "track", "craft", "state") if e.get(k)}
        note = e.get("note", "").split(" Not matched: ")[0].strip()
        if note:
            meta["note"] = note
        rec["meta"] = meta
        if rec.get("argv") and "--meta" not in rec["argv"]:
            rec["argv"] += ["--meta", json.dumps(meta)]
        pj.write_text(json.dumps(rec, indent=2) + "\n")
        filled.append(str(pj.parent.relative_to(root)))
    return filled


def row_of(pj, platform, rec_before):
    """The table row of a re-made pair: drift of the original's own state, and the overlap before and after."""
    new = json.loads(pj.read_text())
    ov = new.get("overlap") or {}
    prev = (new.get("previous") or {}).get("overlap") or {}
    return {"platform": platform, "status": "ok", "drift": new.get("drift", {}), "before": prev, "after": ov}


def fmt(v, spec="%.2f"):
    return "-" if v is None else spec % v


def table(results):
    lines = ["| pair | ship drift (units, deg) | clock drift s | edge_ncc | rmse | mean grey |", "| --- | --- | --- | --- | --- | --- |"]
    sums = {}
    for name in sorted(results):
        r = results[name]
        if r["status"] != "ok":
            lines.append("| %s | **%s** | | | | |" % (name, r["status"]))
            continue
        d, b, a = r["drift"], r["before"], r["after"]
        both = lambda key, spec: "%s to %s" % (fmt(b.get(key), spec), fmt(a.get(key), spec))
        lines.append("| %s | %s, %s | %s | %s | %s | %s |" % (
            name, fmt(d.get("ship_units"), "%.2f"), fmt(d.get("ship_heading_deg"), "%.2f"), fmt(d.get("race_clock_s"), "%+.3f"),
            both("edge_ncc", "%.3f"), both("rmse", "%.3f"), both("mean_grey", "%.2f")))
        for key in ("edge_ncc", "rmse", "mean_grey"):
            if b.get(key) is not None and a.get(key) is not None:
                s = sums.setdefault(r["platform"], {}).setdefault(key, [0.0, 0.0, 0])
                s[0] += b[key]
                s[1] += a[key]
                s[2] += 1
    lines += ["", "| platform | pairs | edge_ncc | rmse | mean grey |", "| --- | --- | --- | --- | --- |"]
    for plat, s in sorted(sums.items()):
        n = max(v[2] for v in s.values())
        cell = lambda key, spec: ("%s to %s" % (spec % (s[key][0] / s[key][2]), spec % (s[key][1] / s[key][2]))) if key in s else "-"
        lines.append("| %s | %d | %s | %s | %s |" % (plat, n, cell("edge_ncc", "%.3f"), cell("rmse", "%.3f"), cell("mean_grey", "%.2f")))
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("src", nargs="?", type=Path, default=ROOT / "data/reference/compare-page")
    ap.add_argument("--out", type=Path, help="re-make a copy of SRC here instead of in place")
    ap.add_argument("--platform", action="append", choices=sorted(EMULATORS), help="only these emulators (repeatable)")
    ap.add_argument("--only", help="substring of the pair directory path")
    ap.add_argument("--resume", action="store_true")
    ap.add_argument("--dry-run", action="store_true", help="list the pairs and whether every path in their argv resolves")
    ap.add_argument("--lane", default="rerender-all")
    ap.add_argument("--psp-display", default="93")
    ap.add_argument("--psp-port", default="9317")
    ap.add_argument("--rpcs3-display", default="94")
    ap.add_argument("--rpcs3-gdb", default="2349")
    ap.add_argument("--pcsx2-display", default="92")
    ap.add_argument("--pcsx2-slot", default="28015")
    ap.add_argument("--timeout", type=int, default=PAIR_TIMEOUT, help="seconds per pair")
    ap.add_argument("--no-hd-cold", dest="hd_cold", action="store_false",
                    help="restore HD's savestates instead of cold-booting each Time Trial (about 40 s a pair against 4 minutes, glass garbage)")
    ap.add_argument("--attempts", type=int, default=2, help="tries per pair: an emulator stalls, a capture is ambiguous, a toast covers the frame; a refusal that repeats is reported")
    args = ap.parse_args()

    scratch = ce.MAIN / "data/scratch" / args.lane
    scratch.mkdir(parents=True, exist_ok=True)
    root = args.src.resolve()
    if args.out:
        root = args.out.resolve()
        if not root.exists():
            shutil.copytree(args.src, root, symlinks=True)
    for name in backfill_meta(root):
        log("meta backfilled from the manifest entry: " + name)
    state_file = root / "rerender-all.json"
    results = json.loads(state_file.read_text()) if args.resume and state_file.exists() else {}

    pairs = {}
    for pj in sorted(root.rglob("pair.json")):
        rec = json.loads(pj.read_text())
        if rec.get("refused") or (args.only and args.only not in str(pj.parent)):
            continue
        plat = PLATFORM_OF.get(rec["profile"])
        if plat is None or (args.platform and plat not in args.platform):
            continue
        pairs.setdefault(plat, []).append(pj)

    if args.dry_run:
        for plat, pjs in pairs.items():
            for pj in pjs:
                rec = json.loads(pj.read_text())
                argv = rec.get("argv") or []
                state = argv[argv.index("--state") + 1] if "--state" in argv else None
                alt = ce.MAIN / "data/saves" / rerender.SAVES_DIR.get(rec["profile"], rec["profile"]) / Path(state).name if state else None
                where = "ok" if state and Path(state).exists() else ("remapped to main saves" if alt and alt.exists() else "STATE MISSING")
                print("%-6s %-60s %-20s state %s" % (plat, pj.parent.relative_to(root), argv[0] if argv else "NO ARGV", where))
        return

    failed = 0
    for plat, pjs in pairs.items():
        todo = [pj for pj in pjs if str(pj.parent.relative_to(root)) not in results or results[str(pj.parent.relative_to(root))]["status"] != "ok"]
        if not todo:
            continue
        emu = EMULATORS[plat](args, scratch)
        log("%s: starting the emulator for %d pair(s)" % (plat, len(todo)))
        try:
            emu.start()
            for pj in todo:
                name = str(pj.parent.relative_to(root))
                rec = json.loads(pj.read_text())
                t0 = time.time()
                for attempt in range(1, args.attempts + 1):
                    try:
                        env, remap = emu.prepare(rec)
                        problems, warn = rerender.remake(pj, env=env, remap=remap, timeout=args.timeout,
                                                          record_dir=(args.src.resolve() / name) if args.out else None)
                    except (subprocess.SubprocessError, OSError) as exc:
                        problems, warn = ["emulator step failed: %s" % exc], []
                    if not problems:
                        break
                    log("attempt %d of %d failed (%s)" % (attempt, args.attempts, "; ".join(problems)[:200]))
                if problems:
                    failed += 1
                    results[name] = {"platform": plat, "status": "FAILED: " + "; ".join(problems)}
                    log("FAILED %s (%.0f s): %s" % (name, time.time() - t0, "; ".join(problems)))
                else:
                    results[name] = row_of(pj, plat, rec)
                    log("ok     %s (%.0f s)%s" % (name, time.time() - t0, "".join("  WARNING: " + w for w in warn)))
                state_file.write_text(json.dumps(results, indent=2) + "\n")
        finally:
            log("%s: stopping the emulator" % plat)
            emu.stop()

    for fname, dirs in MANIFESTS.items():
        have = [root / d for d in dirs if (root / d).exists()]
        if have:
            ce.manifest(root / fname, have)
    text = table(results)
    (root / "rerender-all.md").write_text(text + "\n")
    print(text)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
