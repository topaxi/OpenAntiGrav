#!/usr/bin/env python3
"""`just check-docs` runs this: a pair's recorded environment survives pair.json -> manifest -> page record.

Builds one synthetic pair in a temporary directory, writes its manifest with `compare_env.manifest`, loads it with
`compare-page.py`'s own loader and checks that every structured field, both timestamps and the three metric
explanations arrive, that a pair without recorded times shows none (no file-time fallback), and that a pair without
`meta` is refused instead of showing its directory name.
"""
import argparse
import importlib.util
import json
import struct
import subprocess
import sys
import tempfile
from pathlib import Path
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import oag_tmp  # noqa: E402

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402
import compare_menu as cm  # noqa: E402
import compare_provenance as cprov  # noqa: E402
import pcsx2_state  # noqa: E402

spec = importlib.util.spec_from_file_location("compare_page", HERE / "compare-page.py")
cp = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cp)


def make_pair(root, name, times, meta=True, flags=None):
    d = root / name
    d.mkdir(parents=True)
    for img, colour in (("theirs.png", "gray30"), ("ours.png", "gray40")):
        subprocess.run(["magick", "-size", "48x27", f"xc:{colour}", str(d / img)], check=True)
    plat = ce.platform("pulse-psp")
    rec = {"profile": "pulse-psp", "profile_sha": "abc123", "output": [480, 272], "caveats": [],
           "original": {"emulator": {"name": "PPSSPP", "version": "v1", "renderer": "software"}},
           "ours": {"settings_readback": {"render_scale": 100},
                    "binary": {"sha256_12": "deadbeef0000", "tree_head": "1234567", "tree_dirty": False, "build": None}}}
    if times:
        rec["original"]["captured_at"] = "2026-10-21T09:00:00+02:00"
        rec["ours"]["rendered_at"] = "2026-10-21T09:05:00+02:00"
    if meta:
        rec["meta"] = {"id": name, "title": "Readable " + name}
    if flags is not None:
        rec["ours"]["flags"] = flags
    assert plat["game"]
    (d / "pair.json").write_text(json.dumps(rec))
    return d


def check_menu_route(root):
    """A menu pair: ours' command drops the race-only flags and takes the profile's size; a page ours draws from `menu.toml` is
    refused by `Pair.finish` (and `write_gap` keeps only the original's frame); a page from the XML with a wrong screen or
    language is refused naming the check; a cold-boot menu needs no presented-frame lag."""
    plat = ce.platform("hd-fury")
    cmd = cm.ours_menu_command("oag-game", "img.iso", plat, "track-select", "1920x1080")
    assert cmd[2:4] == ["--menu-page", "track-select"], cmd
    assert "--no-hull-shake" not in cmd and "--pilot-assist" not in cmd, cmd
    assert cmd[cmd.index("--size") + 1] == "1920x1080", cmd
    routed = cm.ours_menu_command("oag-game", "img", plat, "home", "960x544", ["--until", "Home", "--press", "start"])
    assert routed[2:6] == ["--until", "Home", "--press", "start"] and "--menu-page" not in routed, routed
    theirs = root / "theirs.png"
    subprocess.run(["magick", "-size", "1920x1080", "xc:gray20", str(theirs)], check=True)

    def refused(drawn_from, screen_seen="Track Creation", language="English"):
        pdir = root / ("menu-%s-%s-%s" % (drawn_from.replace(".", ""), screen_seen.replace(" ", ""), language))
        pdir.mkdir()
        pair = ce.Pair("hd-fury", pdir)
        pair.rec["meta"] = {"id": "t", "title": "t"}
        pair.rec["original"]["emulator"] = dict(plat["emulator"])
        ns = type("A", (), {"autopilot": None, "autopilot_timer": None, "ram_write": [], "accept_provenance": []})()
        cprov.build(pair, ns, "menu", "cold_boot", None, {"value": None, "unit": None, "basis": "not applicable: a menu is a still"})
        try:
            cm.menu_pair(pair, theirs_png=theirs, oag_game=__file__, image="img", page="track-select",
                         drawn_from=drawn_from, xml_file="x.xml", screen_seen=screen_seen, screen_wanted="Track Creation",
                         profile_dir=pdir, size="1920x1080", front_end_style="FURY", language=language, original_language="English",
                         selection={"original": "a", "ours": "a"})
            pair.finish()
        except SystemExit as exc:
            return exc.code, json.loads((pdir / "pair.json").read_text()), [c["name"] for c in pair.failed()]
        raise AssertionError("a bad menu pair was not refused")

    code, rec, _ = refused(cm.GAP)
    assert code == 3 and rec["refused"] and not (root / "menu-menutoml-TrackCreation-English" / "ours.png").exists(), rec
    assert refused("xml", screen_seen="Main Menu")[2] == ["original screen is the one asked for"]
    code, _, failed = refused("xml", language="Deutsch")
    assert code == 3 and failed == ["language"], failed
    assert not [c for c in cprov.refusals(rec["original"]["provenance"], "hd-fury")], "a cold-boot menu needs no lag"
    cm.write_gap(root / "gap-x", theirs, "race", "menu.toml", "why")
    assert (root / "gap-x" / "theirs.png").exists() and json.loads((root / "gap-x" / "gap.json").read_text())["page"] == "race"
    assert not (root / "gap-x" / "ours.png").exists()


def check_safe_area(root):
    """The Safe Area record: a save's `DATA.BIN` record parses (`96%` and the bare `96` spellings, any other shape is
    None), an HD pair without a recorded value fails its check, ours' settings take the pair's value over the profile's
    pin, and `missing-safe-area` names a stored HD pair that records none."""
    def record(value, length=None):
        body = value + b"\0"
        blob = bytearray(b"\0" * 0x20 + b"Safe Area Setting\0" + b"\0" * 0x100)
        at = 0x20
        blob[at + 0x44:at + 0x4c] = (length or len(body)).to_bytes(4, "big") + (0x40).to_bytes(4, "big")
        blob[at + 0x60:at + 0x60 + len(body)] = body
        return bytes(blob)
    assert ce.parse_safe_area(record(b"96%")) == 96 and ce.parse_safe_area(record(b"90")) == 90
    assert ce.parse_safe_area(record(b"ab%")) is None and ce.parse_safe_area(b"no record here") is None
    plat = ce.platform("hd-fury")
    pdir = root / "hd-pair"
    pdir.mkdir()
    pair = ce.Pair("hd-fury", pdir)
    assert ce.record_safe_area(pair, {"percent": 84, "source": "save file"}) == 84
    assert pair.rec["original"]["safe_area"]["percent"] == 84 and not pair.failed()
    bare = ce.Pair("hd-fury", root / "hd-bare")
    ce.read_hd_safe_area = lambda config_home=None: None
    assert ce.record_safe_area(bare) == plat["ours"]["hud_safe_area"] and bare.failed(), "an HD pair with no value must fail"
    ce.write_ours_settings(plat, root / "prof", 84)
    assert tomllib_load(ce.settings_path(root / "prof"))["graphics"]["hud_safe_area"] == 84
    ce.write_ours_settings(plat, root / "prof2")
    assert tomllib_load(ce.settings_path(root / "prof2"))["graphics"]["hud_safe_area"] == plat["ours"]["hud_safe_area"]
    assert "Safe Area 84% on both sides (save file)" in ce.environment_text(plat, pair.rec)
    stored = root / "stored" / "p"
    stored.mkdir(parents=True)
    (stored / "pair.json").write_text(json.dumps({"profile": "hd-fury", "original": {}, "ours": {}}))
    assert ce.missing_safe_area([root / "stored"]) == [stored / "pair.json"]
    assert not ce.missing_safe_area([root / "stored" / "nothing"])
    # a menu pair whose profile locked the build keeps the locks and the race selection for a re-render
    locked = root / "locked"
    ce.write_ours_settings(ce.platform("pure-psp"), locked)
    assert not cm.profile_is_locked(locked)
    cm.extra_settings(locked, race={"track": "01_vineta_k"}, locked=True)
    doc = tomllib_load(ce.settings_path(locked))
    assert cm.profile_is_locked(locked) and doc["dev"]["unlock_everything"] is False and doc["race"]["track"] == "01_vineta_k"
    # HD's menu pair script builds its profile locked: its walk starts from a profile with the locks, so Ship Select's grid draws padlocks
    hd_script = importlib.util.spec_from_file_location("hd_menu_pair", HERE / "rpcs3-hd-menu-pair.py")
    hd = importlib.util.module_from_spec(hd_script)
    sys.path.insert(0, str(HERE))
    hd_script.loader.exec_module(hd)
    hd_profile = root / "hd-locked"
    hd.prepare_profile(ce.platform("hd-fury"), hd_profile, 96, "FURY", None)
    assert cm.profile_is_locked(hd_profile), "the HD menu pair script must lock ours' profile"
    assert tomllib_load(ce.settings_path(hd_profile))["display"]["front_end_style"] == "FURY"
    # the screens it photographs with the Fury backdrop hidden are screens of its walk, and ours is told the same state
    walked = {item["id"] for item in hd.WALK if isinstance(item, dict)}
    assert hd.HIDDEN_BACKDROP and set(hd.HIDDEN_BACKDROP) <= walked, "HIDDEN_BACKDROP names a screen WALK does not photograph"
    assert tuple(hd.fb.OURS_FLAGS) == ("--fury-tint", "0"), "ours must be drawn at the tint written over the original's row"


def check_ps2_world_clock(root):
    """A Pulse PS2 pair's animated surfaces (Orcus' neon strips cycle magenta, cyan, yellow) run on `*(g_ingame + 0x40)`, not on ours' tick
    count: the offset comes from a state's `g_ingame` clock, run on to the grab and less ours' `ticks / 60`. A record with no bracket gives none."""
    ram = bytearray(0x400000)
    struct.pack_into("<I", ram, pcsx2_state.G_INGAME, 0x325000)
    struct.pack_into("<f", ram, 0x325000 + pcsx2_state.INGAME_CLOCK, 33.5)
    import zipfile
    with zipfile.ZipFile(root / "state.p2s", "w") as z:
        z.writestr("eeMemory.bin", bytes(ram))
    rec = {"original": {"state": {"file": "state.p2s"}, "race_clock_live_s": {"before_grab": 27.5, "after_grab": 28.0, "state_file": 26.0}},
           "ours": {"ticks": 1683}}
    want = 33.5 + (27.75 - 26.0) - 1683 / 60
    got = pcsx2_state.record_world_clock_offset(rec, root)
    assert got is not None and abs(got - want) < 1e-4, (got, want)
    assert abs(want - 7.2) < 0.01, want
    # the rerender puts it on a pulse-ps2 record that has none, leaves a record that has one, and gives a menu pair (no bracket) nothing
    spec = importlib.util.spec_from_file_location("compare_rerender_ours", HERE / "compare-rerender-ours.py")
    rro = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(rro)
    rro.ROOT = root
    rec["profile"] = "pulse-ps2"
    added = rro.ps2_world_clock_flags(rec, ["--race"])
    assert added[:1] == ["--race"] and added[1] == "--world-clock-offset" and abs(float(added[2]) - want) < 1e-3, added
    assert rec["ours"]["world_clock_offset_s"] == got
    assert rro.ps2_world_clock_flags(rec, ["--world-clock-offset", "1.0"]) == ["--world-clock-offset", "1.0"]
    assert rro.ps2_world_clock_flags({"profile": "pulse-ps2", "ours": {}, "original": {}}, ["--race"]) == ["--race"]
    rec["profile"] = "pulse-psp"
    assert rro.ps2_world_clock_flags(rec, ["--race"]) == ["--race"]
    del rec["original"]["race_clock_live_s"]
    assert pcsx2_state.record_world_clock_offset(rec, root) is None
    struct.pack_into("<I", ram, pcsx2_state.G_INGAME, 0)
    with zipfile.ZipFile(root / "state.p2s", "w") as z:
        z.writestr("eeMemory.bin", bytes(ram))
    assert pcsx2_state.world_clock(root / "state.p2s") is None


def check_hd_flame_clock():
    """An HD grab records the frame context's boot clock (`*0x00936fd4 + 0xc4`) as `flame_clock_s`, and a re-render puts it
    on ours as `--flame-clock` less the measured lag; a record without it, a non-HD record, or flags that carry one keep theirs."""
    import rpcs3_hd_field as field

    class Fake:
        def __init__(self, words):
            self.words = words

        def u32(self, addr):
            return self.words.get(addr, 0)

        def f32(self, addr):
            return self.words.get(addr, 0.0)

    assert field.flame_clock(Fake({0x00936FD4: 0x3005C9B0, 0x3005C9B0 + 0xC4: 136.6355})) == 136.6355
    assert field.flame_clock(Fake({})) is None, "no frame context yet reads no clock"
    assert abs(ce.flame_pin(122.472) - (122.472 - ce.FLAME_CLOCK_LAG_S)) < 1e-9 and ce.flame_pin(None) is None
    assert 0.0167 <= ce.FLAME_CLOCK_LAG_S <= 0.0334, "the lag sits between the two measured reads"
    spec = importlib.util.spec_from_file_location("compare_rerender_ours", HERE / "compare-rerender-ours.py")
    rro = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(rro)
    rec = {"profile": "hd-fury", "original": {"flame_clock_s": 122.472}}
    assert rro.hd_flame_clock_flags(rec, ["--race"]) == ["--race", "--flame-clock", "%.4f" % (122.472 - ce.FLAME_CLOCK_LAG_S)]
    assert rro.hd_flame_clock_flags(rec, ["--flame-clock", "1.0"]) == ["--flame-clock", "1.0"]
    assert rro.hd_flame_clock_flags({"profile": "hd-fury", "original": {}}, ["--race"]) == ["--race"]
    assert rro.hd_flame_clock_flags(dict(rec, profile="pulse-psp"), ["--race"]) == ["--race"]
    # A Detonator pair recorded before `--detonator-mine-volumes` gains them from its picture-clock read on a re-render.
    mines = {"original": {"detonator": {"mines": [{"master_volume": 1.0, "pitch_volume": 0.3154}, {"master_volume": 0.0, "pitch_volume": 0.0}]}}}
    old = ["--detonator-mines", "1,2,3;4,5,6"]
    assert ce.recorded_mine_volume_flags(mines, old) == old + ["--detonator-mine-volumes", "1.0000,0.3154;0.0000,0.0000"]
    again = old + ["--detonator-mine-volumes", "0.5,0.1;0,0"]
    assert ce.recorded_mine_volume_flags(mines, again) == again
    assert ce.recorded_mine_volume_flags({"original": {}}, old) == old
    try:
        ce.recorded_mine_volume_flags(mines, ["--detonator-mines", "1,2,3"])
    except ValueError:
        pass
    else:
        raise AssertionError("a record and flags of different mine counts must not be pinned")


def check_hd_hull():
    """An HD pair refuses an original whose RAM hull is not the one ours draws: a Fury `Harimau_c1` grab with
    `--team harimau` passed every other parity check. A live capture without the read is refused too; an old `--json`
    record without it passes."""
    spec = importlib.util.spec_from_file_location("rpcs3_hd_pair", HERE / "rpcs3-hd-pair.py")
    hd_pair = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(hd_pair)
    import rpcs3_hd_field as field

    class Pair:
        def __init__(self):
            self.rec = {"original": {}}
            self.checks = []

        def check(self, name, ok, detail):
            self.checks.append((name, bool(ok), detail))
            return ok

        def check_equal(self, name, theirs, ours):
            return self.check(name, theirs == ours, "%r %r" % (theirs, ours))

    class Mem:
        def __init__(self, path):
            self.path = path

        def u32(self, addr):
            return {0x1000 + 0x6298: 0x2000, 0x2000 + 0x1B4: 0x3000}.get(addr, 0)

        def rd(self, _addr, n):
            return (self.path.encode() + b"\0" * n)[:n]

    assert field.team_of(Mem("Data\\Ships\\Harimau_c1"), 0x1000) == "Harimau_c1"
    assert field.team_of(Mem("Data\\Ships\\Harimau"), 0x1000) == "Harimau"
    for ram, team, live, ok in (("Harimau", "harimau", True, True), ("Harimau_c1", "harimau", True, False),
                                ("Assegai_c1", "Assegai_c1", True, True), (None, "harimau", True, False),
                                (None, "harimau", False, True)):
        pair = Pair()
        got = hd_pair.hull_check(pair, {} if ram is None else {"team": ram}, team, live)
        assert bool(got) == ok and [c[1] for c in pair.checks] == [ok], (ram, team, live, pair.checks)
        assert pair.rec["original"]["team"] == ram
    zone = Pair()
    assert hd_pair.hull_check(zone, {"team": "Zone"}, hd_pair.ZONE_HULL, True), "a Zone race flies the mode's own hull"


def check_hd_countdown():
    """A `--countdown` Zone pair is refused unless the pause is in the countdown (race manager phase 1, lap clock 0, the
    craft at rest), draws ours without `--hud-state`, and its capture takes the first shot where the attach pauses the game
    (the default 12 s settle outlasts the 5.2 s countdown)."""
    spec = importlib.util.spec_from_file_location("rpcs3_hd_pair", HERE / "rpcs3-hd-pair.py")
    hd_pair = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(hd_pair)
    import rpcs3_hd_field as field

    class Pair:
        def __init__(self):
            self.rec = {"original": {}}
            self.checks, self.caveats = [], []

        def check(self, name, ok, detail):
            self.checks.append((name, bool(ok), detail))
            return ok

        def check_equal(self, name, theirs, ours):
            return self.check(name, theirs == ours, "%r %r" % (theirs, ours))

        def caveat(self, text):
            self.caveats.append(text)

    class Mem:
        """The race manager at 0x4000 with phase 1 and 1.5 s elapsed."""

        def u32(self, addr):
            return 0x4000 if addr == field.RACE_MANAGER_PP else 0

        def rd(self, addr, n):
            assert addr == 0x4000 + field.RACE_PHASE_BLOCK
            block = bytearray(n)
            struct.pack_into(">f", block, 0x18, 1.5)
            struct.pack_into(">I", block, 0x20, 1)
            return bytes(block)

    assert field.race_phase(Mem()) == {"phase": 1, "elapsed_s": 1.5}
    rest = {"phase": 1, "elapsed_s": 1.5}
    for phase, clock, vel, live, ok in ((rest, 0, [0.1, 3.6, 0.1], True, True), ({"phase": 2, "elapsed_s": 4}, 0, [0, 0, 0], True, False),
                                        (rest, 0.1, [0, 0, 0], True, False), (rest, 0, [0, 0, 5.0], True, False),
                                        (None, 0, [0, 0, 0], True, False), (None, 3.0, [0, 0, 30.0], False, True)):
        pair = Pair()
        ship = {"race_clock_s": clock, "vel": vel, **({"race_phase": phase} if phase else {})}
        got = hd_pair.countdown_check(pair, ship, live)
        assert bool(got) == ok, (phase, clock, vel, live, pair.checks)
    flags = hd_pair.zone_flags(Pair(), "1,0", countdown=True)
    assert "--hud-state" not in flags, flags
    assert hd_pair.zone_flags(Pair(), "5,100,4", countdown=False) == ["--hud-state", "1,1,0,0,0,100,0,5,100,0", "--zone-stage", "4"]
    args = argparse.Namespace(interval=0.12, load=0, team="feisar", press="none", field=False, zone="1,0", keep_autopilot=False,
                              countdown=False, countdown_at=1.0)
    cap = [str(c) for c in hd_pair.capture_command(args, Path("o"))]
    assert cap[cap.index("--settle") + 1] == "0" and "--first-at-pause" in cap and "--wait-countdown" not in cap, cap
    args.countdown = True
    cap = [str(c) for c in hd_pair.capture_command(args, Path("o"))]
    assert cap[cap.index("--wait-countdown") + 1] == "1.0", cap
    args.zone = None
    assert "--settle" not in [str(c) for c in hd_pair.capture_command(args, Path("o"))]


def check_hd_countdown_tick():
    """An HD countdown pair renders ours as long before the release as the original's pause was, with the world clock
    rebased onto the new tick; any other pair is left alone."""
    assert ce.countdown_tick(272, 2.4333300590515137) == 238
    assert ce.countdown_tick(272, 3.0) == 272
    rec = {"profile": "hd-fury", "caveats": ["Zone countdown frame: old words", "kept"],
           "original": {"race_phase": {"phase": 1, "elapsed_s": 2.4333300590515137}, "world_clock_s": 287.1652526855469,
                        "zone": {"zone": 1, "score": 0}},
           "ours": {}}
    flags = ce.countdown_flags(rec, ["--race", "--world-clock-offset", "287.165", "--ticks", "0"], 272)
    assert flags[flags.index("--ticks") + 1] == "238", flags
    assert flags[flags.index("--world-clock-offset") + 1] == "283.199", flags
    assert "--pose-hold" in flags and ce.countdown_flags(rec, flags, 272) == flags, flags
    assert rec["ours"]["ticks"] == 238 and rec["caveats"] == [ce.COUNTDOWN_CAVEAT % (1, 0), "kept"], rec
    for other in ({"profile": "hd-fury", "original": {}, "ours": {}},
                  {"profile": "hd-fury", "original": {"race_phase": {"phase": 2, "elapsed_s": 1.0}}, "ours": {}},
                  {"profile": "pulse-psp", "original": rec["original"], "ours": {}}):
        assert ce.countdown_flags(other, ["--ticks", "0"], 272) == ["--ticks", "0"], other


def check_hd_wait_countdown():
    """`wait_countdown` returns only once the race manager is `elapsed_s` into phase 1 (not in the race phase before the
    reload, not at the first tick of the countdown) and gives up on a timeout."""
    import rpcs3_hd_field as field
    reads = iter([(2, 40.0), (0, 1.0), (1, 0.2), (1, 1.1), (2, 0.1)])

    class Mem:
        def u32(self, addr):
            return 0x4000 if addr == field.RACE_MANAGER_PP else 0

        def rd(self, _addr, n):
            phase, elapsed = next(reads)
            block = bytearray(n)
            struct.pack_into(">f", block, 0x18, elapsed)
            struct.pack_into(">I", block, 0x20, phase)
            return bytes(block)

    got = field.wait_countdown(Mem(), 1.0, 30.0, clock=lambda: 0.0, sleep=lambda _s: None, say=lambda _t: None)
    assert got["phase"] == 1 and abs(got["elapsed_s"] - 1.1) < 1e-6, got
    ticks = iter(range(1000))

    class Never(Mem):
        def rd(self, _addr, n):
            return bytes(n)

    try:
        field.wait_countdown(Never(), 1.0, 5.0, clock=lambda: float(next(ticks)), sleep=lambda _s: None, say=lambda _t: None)
    except TimeoutError:
        pass
    else:
        raise AssertionError("no countdown must time out")


def check_hd_pilot_assist():
    """An HD pair refuses an original whose save has Pilot Assist on (the indicator's bracket under the craft) when ours is
    told off, and the reverse; a live capture without the read is refused, an old `--json` record without it passes."""
    spec = importlib.util.spec_from_file_location("rpcs3_hd_pair", HERE / "rpcs3-hd-pair.py")
    hd_pair = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(hd_pair)
    import rpcs3_hd_field as field

    class Pair:
        def __init__(self):
            self.rec = {"original": {}}
            self.checks = []

        def check(self, name, ok, detail):
            self.checks.append((name, bool(ok), detail))
            return ok

        def check_equal(self, name, theirs, ours):
            return self.check(name, theirs == ours, "%r %r" % (theirs, ours))

    class Mem:
        def u32(self, addr):
            return {field.OPTIONS_PP: 0x2000, 0x2000: 0x3000}.get(addr, 0)

        def rd(self, addr, n):
            return bytes([3 if addr == 0x3000 + 0x473 else 0]) * n

    assert field.pilot_assist(Mem()) == 3
    assert ce.platform("hd-fury")["ours"]["pilot_assist"] == "off"
    for byte, profile, live, ok in ((0, "off", True, True), (3, "off", True, False), (1, "on", True, True),
                                    (0, "on", True, False), (None, "off", True, False), (None, "off", False, True)):
        pair = Pair()
        got = hd_pair.pilot_assist_check(pair, {} if byte is None else {"pilot_assist": byte}, profile, live)
        assert bool(got) == ok and [c[1] for c in pair.checks] == [ok], (byte, profile, live, pair.checks)
    assert hd_pair.pilot_assist_check(Pair(), {"pilot_assist": 3}, None, True), "a profile without the setting checks nothing"


def tomllib_load(path):
    import tomllib
    return tomllib.loads(Path(path).read_text())


def check_anisotropy(root):
    """Omega's original asks for 16x anisotropy on its mipmapped colour samplers and the port honours it, so its profile
    runs ours at 16x, written to the settings file and read back; every other platform stays off."""
    assert ce.platform("omega")["ours"]["anisotropy"] == "16x"
    for key in ("pulse-psp", "pulse-ps2", "hd-fury", "2048"):  # 2048 (Vita) is not measured: it keeps the default
        assert ce.platform(key)["ours"]["anisotropy"] == "off", key
    plat = ce.platform("omega")
    ce.write_ours_settings(plat, root / "omega")
    assert tomllib_load(ce.settings_path(root / "omega"))["graphics"]["anisotropy"] == "16x"
    got, bad = ce.read_back(plat, root / "omega")
    assert got["anisotropy"] == "16x" and not bad, bad
    assert ["--anisotropy", "16x"] == ce.ours_flags(plat)[ce.ours_flags(plat).index("--anisotropy"):][:2]
    # a stored race pair that says `off` re-renders at the profile's level; a menu pair keeps its own; every other profile
    # is `off` on the measured claim that HD asks for none, the PSP and PS2 emulators are set to 0, and 2048 (Vita) is unmeasured
    hd = ce.platform("hd-fury")
    assert hd["ours"]["pilot_assist"] == "off"
    assert ce.profile_pilot_assist_flags(["--race", "--pilot-assist", "on"], hd) == ["--race", "--pilot-assist", "off"]
    assert ce.profile_pilot_assist_flags(["--race", "--pilot-assist", "on"], hd, "menu") == ["--race", "--pilot-assist", "on"]
    old = ["--race", "--anisotropy", "off", "--no-hull-shake"]
    assert ce.profile_anisotropy_flags(old, plat) == ["--race", "--anisotropy", "16x", "--no-hull-shake"]
    assert ce.profile_anisotropy_flags(["--race"], plat, "race") == ["--race", "--anisotropy", "16x"]
    assert ce.profile_anisotropy_flags(old, plat, "menu") == old
    assert ce.profile_anisotropy_flags(old, ce.platform("hd-fury")) == old


def check_loading_bar(root):
    """`compare_menu.lit_bar_columns` counts the lit columns of HD's race-load bar, and the tick that states them follows."""
    from PIL import Image, ImageDraw
    for lit in (0, 1, 32, 33, 166):
        img = Image.new("RGB", (1920, 1080))
        draw = ImageDraw.Draw(img)
        for column in range(lit):
            x = 201 + round(column * 7.66)
            draw.rectangle([x, 703, x + 5, 929], fill=(172, 7, 23))
        for column in range(lit, 166):  # the unlit dots are grey and are not counted
            x = 201 + round(column * 7.66)
            draw.rectangle([x, 703, x + 5, 929], fill=(40, 40, 40))
        draw.rectangle([1500, 700, 1700, 900], fill=(172, 7, 23))  # the mode icon is outside the bar
        path = root / ("bar%d.png" % lit)
        img.save(path)
        assert cm.lit_bar_columns(path) == lit, (lit, cm.lit_bar_columns(path))
    assert cm.LOADING_BAR_TICKS_PER_COLUMN == 10 and cm.LOADING_BAR_FIRST_TARGET_COLUMNS == 33


def check_detonator_mine_envelopes():
    """An HD Detonator pair carries every live mine's music envelope into ours (`--detonator-mine-envelopes`), and a shot
    record without them is refused: ours' envelopes have no history, so its near mines draw dark and unspiked."""
    spec = importlib.util.spec_from_file_location("rpcs3_hd_pair", HERE / "rpcs3-hd-pair.py")
    hd_pair = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(hd_pair)

    class Pair:
        def __init__(self):
            self.rec = {"original": {"detonator": {}}}
            self.checks = []

        def caveat(self, _text):
            pass

        def check(self, name, ok, detail):
            self.checks.append((name, bool(ok), detail))
            return ok

    def mine(x, live, s=None, t=None):
        m = {"flags": 9, "entry": 1, "lane": 0, "position": [x, 0.0, 0.0], "live": live}
        if s is not None:
            m.update(football_s=s, spikes_t=t)
        return m

    read = Pair()
    flags = hd_pair.mine_flags(read, {"mines": [mine(1.0, 1, 0.5, 0.75), mine(2.0, 0, 0.0, 0.0)]})
    at = flags.index("--detonator-mine-envelopes")
    assert flags[at + 1] == "0.5000,0.7500;0.0000,0.0000", flags
    assert all(ok for _, ok, _ in read.checks), read.checks
    old = Pair()
    flags = hd_pair.mine_flags(old, {"mines": [mine(1.0, 1), mine(2.0, 1)]})
    assert "--detonator-mine-envelopes" not in flags, flags
    assert [ok for name, ok, _ in old.checks if "envelope" in name] == [False], old.checks
    dark = Pair()
    hd_pair.mine_flags(dark, {"mines": [mine(1.0, 0)]})
    assert all(ok for _, ok, _ in dark.checks), dark.checks
    # The picture is older than its pause's RAM: each mine takes the read nearest the picture's clock, with its draw words.
    lagged = Pair()
    lagged.rec["original"]["presented_lag"] = {"picture_total_ticks": 6.0}
    ram = dict(mine(1.0, 1, 0.25, 1.0), address=7, master_volume=0.5372, pitch_volume=0.1074)
    reads = [{"race_clock_s": c, "mines": {"7": {"football_s": s, "spikes_t": t, "pitch_offset": 8.25, "age_s": c + 53.0,
                                                 "master_volume": min(1.0, 5 * v), "pitch_volume": v}}}
             for c, s, t, v in ((2.55, 0.6, 0.5, 0.1055), (2.6, 0.5, 0.45, 0.3154), (2.7, 0.25, 1.0, 0.1074))]
    flags = hd_pair.mine_flags(lagged, {"mines": [ram], "race_clock_s": 2.7, "mine_reads": reads})
    assert flags[flags.index("--detonator-mine-envelopes") + 1] == "0.5000,0.4500,8.2500", flags
    det = lagged.rec["original"]["detonator"]
    assert det["mine_read"]["read_clock_s"] == 2.6 and det["mines"][0]["pitch_offset"] == 8.25, det
    # The age turns the mine: it comes from the same read as the envelope, not the pause's.
    assert flags[flags.index("--detonator-mine-ages") + 1] == "55.6000", flags
    assert det["mines"][0]["age_s"] == 55.6, det
    # The ring's two volumes swing threefold between reads: the picture's read, not the pause's.
    assert flags[flags.index("--detonator-mine-volumes") + 1] == "1.0000,0.3154", flags
    far = Pair()
    far.rec["original"]["presented_lag"] = {"picture_total_ticks": 6.0}
    flags = hd_pair.mine_flags(far, {"mines": [ram], "race_clock_s": 2.7, "mine_reads": reads[2:]})
    assert flags[flags.index("--detonator-mine-envelopes") + 1] == "0.2500,1.0000", flags
    # No read at the picture's clock: the pause's age is moved back by the measured lag; with no measured lag it is kept.
    aged = dict(ram, age_s=55.0)
    late = Pair()
    late.rec["original"]["presented_lag"] = {"picture_total_ticks": 7.5}
    flags = hd_pair.mine_flags(late, {"mines": [aged], "race_clock_s": 2.7})
    assert flags[flags.index("--detonator-mine-ages") + 1] == "54.8750", flags
    unfitted = Pair()
    flags = hd_pair.mine_flags(unfitted, {"mines": [aged], "race_clock_s": 2.7})
    assert flags[flags.index("--detonator-mine-ages") + 1] == "55.0000", flags


def check_detonator_volley_ticks():
    """A Detonator pair whose rounds were read presses on each volley's own tick, one tick each with a release between; a
    record without them (every pair published before) keeps the one held window, since a re-render makes the script again."""
    sys.path.insert(0, str(HERE))
    import rpcs3_hd_volley
    rpcs3_hd_volley.self_test()
    window = {"press_clock_before_s": 1.8499995, "press_clock_after_s": 2.0499995, "button": "triangle"}
    assert ce.detonator_input_lines(window, 272) == ["383 none", "12 square", "1 none"]
    read = dict(window, volley_clocks_s=[1.8833328, 1.9499995])
    assert ce.detonator_input_lines(read, 272) == ["385 none", "1 square", "3 none", "1 square", "1 none"]


def check_detonator_picture_tick(root):
    """A Detonator pair whose picture lag was measured runs ours to the picture's tick (rounds, mine hits, score, ammo), on
    the first render and on every re-render; the static-scene fit takes only a sharp, interior peak."""
    sys.path.insert(0, str(HERE))
    import hd_picture_lag
    # lap clock 2.0667 s, picture 7.5 ticks older than RAM: 124 - 7.5 ticks after GO, not the HUD clock's 3
    assert ce.picture_tick(272, 2.0666661262512207, 7.5) == 388
    peak = {0.0: 0.44, 1.0: 0.45, 2.0: 0.44, 3.0: 0.50, 3.5: 0.59, 4.0: 0.55, 5.0: 0.38, 6.0: 0.35}
    assert hd_picture_lag.pick_static(peak) == (3.5, None)
    assert hd_picture_lag.pick_static({0.0: 0.38, 1.0: 0.36, 3.0: 0.30, 6.0: 0.12})[0] is None  # at the end
    assert hd_picture_lag.pick_static({0.0: 0.50, 2.0: 0.52, 3.0: 0.53, 6.0: 0.51})[0] is None  # flat

    spec = importlib.util.spec_from_file_location("compare_rerender_ours", HERE / "compare-rerender-ours.py")
    rerender = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(rerender)
    fake = root / "fake-oag-game"
    fake.write_text('#!/bin/sh\nwhile [ "$1" != --screenshot ]; do shift; done\ncp "%s" "$2"\n' % (root / "pair" / "theirs.png"))
    fake.chmod(0o755)
    (root / "pair").mkdir()
    subprocess.run(["magick", "-size", "48x27", "xc:gray30", str(root / "pair" / "theirs.png")], check=True)

    def ticks_after(lag):
        rec = {"profile": "hd-fury", "output": [48, 27], "kind": "race",
               "original": {"race_clock_s": 2.0666661262512207, "gantry_seconds": 7.45,
                            "presented_lag": dict({"hud_clock_ticks": 3}, **lag)},
               "ours": {"ticks": 393, "flags": ["--race", "--pose-hold-ticks", "--gantry-seconds", "7.400", "--ticks", "393"]}}
        (root / "pair" / "pair.json").write_text(json.dumps(rec))
        result = rerender.rerender(root / "pair" / "pair.json", fake)
        assert result.startswith("ok"), result
        flags = json.loads((root / "pair" / "pair.json").read_text())["ours"]["flags"]
        return flags[flags.index("--ticks") + 1], flags[flags.index("--gantry-seconds") + 1]

    assert ticks_after({"picture_total_ticks": 7.5}) == ("388", "%.6f" % (7.45 - 7.5 / 60))
    assert ticks_after({}) == ("393", "%.6f" % (7.45 - 3 / 60))

    # The re-render itself adds a recorded Detonator pair's mine volumes (`ce.recorded_mine_volume_flags`).
    rec = {"profile": "hd-fury", "output": [48, 27], "kind": "race",
           "original": {"detonator": {"mines": [{"master_volume": 0.9663, "pitch_volume": 0.1933}]}},
           "ours": {"ticks": 393, "flags": ["--race", "--detonator-mines", "1,2,3", "--ticks", "393"]}}
    (root / "pair" / "pair.json").write_text(json.dumps(rec))
    assert rerender.rerender(root / "pair" / "pair.json", fake).startswith("ok")
    flags = json.loads((root / "pair" / "pair.json").read_text())["ours"]["flags"]
    assert flags[flags.index("--detonator-mine-volumes") + 1] == "0.9663,0.1933", flags


def main():
    import tomllib
    with tempfile.TemporaryDirectory(dir=oag_tmp.temp_root()) as t:
        root = Path(t)
        make_pair(root / "pairs", "timed", True)
        make_pair(root / "pairs", "untimed", False)
        make_pair(root / "pairs", "held", False, flags=ce.exposure_hold_flags(ce.platform("2048")["ours"]))
        ce.manifest(root / "m.toml", [root / "pairs"])
        doc = tomllib.loads((root / "m.toml").read_text())
        by_id = {e["id"]: e for e in doc["entry"]}
        env = by_id["timed"]["environment"]
        want = {"profile": "pulse-psp@abc123", "output": "480x272", "original_emulator": "PPSSPP v1",
                "original_renderer": "software", "captured_at": "2026-10-21T09:00:00+02:00",
                "rendered_at": "2026-10-21T09:05:00+02:00", "binary": "deadbeef0000"}
        for k, v in want.items():
            assert env.get(k) == v, f"{k}: {env.get(k)!r} != {v!r}"
        for k in ("rmse", "edge_ncc", "mean_grey"):
            assert k in env, f"metric {k} missing"
        assert by_id["timed"]["title"] == "Readable timed"
        assert "captured_at" not in by_id["untimed"]["environment"] and "rendered_at" not in by_id["untimed"]["environment"]
        # A 2048 pair holds ours' exposure at the original's 1.15 and the page says so; no other pair carries the line.
        assert by_id["held"]["environment"]["ours_exposure"].startswith("exposure held at 1.15"), by_id["held"]["environment"]
        assert "ours_exposure" not in by_id["timed"]["environment"]
        assert "ours_exposure" not in ce.platform("pulse-psp")["ours"]
        loaded = cp.load_manifest(root / "m.toml")
        assert isinstance(loaded["entry"][0]["environment"], dict)
        assert set(ce.METRICS[i][0] for i in range(3)) == {"rmse", "edge_ncc", "mean_grey"}
        make_pair(root / "bad", "nometa", True, meta=False)
        try:
            ce.manifest(root / "bad.toml", [root / "bad"])
        except SystemExit as e:
            assert "meta" in str(e)
        else:
            raise AssertionError("a pair without meta was not refused")
    with tempfile.TemporaryDirectory(dir=oag_tmp.temp_root()) as t:
        check_menu_route(Path(t))
    with tempfile.TemporaryDirectory(dir=oag_tmp.temp_root()) as t:
        check_safe_area(Path(t))
    with tempfile.TemporaryDirectory(dir=oag_tmp.temp_root()) as t:
        check_ps2_world_clock(Path(t))
    with tempfile.TemporaryDirectory(dir=oag_tmp.temp_root()) as t:
        check_loading_bar(Path(t))
    with tempfile.TemporaryDirectory(dir=oag_tmp.temp_root()) as t:
        check_anisotropy(Path(t))
    check_detonator_mine_envelopes()
    check_hd_flame_clock()
    check_hd_hull()
    check_hd_pilot_assist()
    check_hd_countdown()
    check_hd_countdown_tick()
    check_hd_wait_countdown()
    check_detonator_volley_ticks()
    with tempfile.TemporaryDirectory(dir=oag_tmp.temp_root()) as t:
        check_detonator_picture_tick(Path(t))
    print("compare environment: round trip ok")


if __name__ == "__main__":
    main()
