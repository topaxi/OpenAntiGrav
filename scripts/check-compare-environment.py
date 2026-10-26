#!/usr/bin/env python3
"""`just check-docs` runs this: a pair's recorded environment survives pair.json -> manifest -> page record.

Builds one synthetic pair in a temporary directory, writes its manifest with `compare_env.manifest`, loads it with
`compare-page.py`'s own loader and checks that every structured field, both timestamps and the three metric
explanations arrive, that a pair without recorded times shows none (no file-time fallback), and that a pair without
`meta` is refused instead of showing its directory name.
"""
import importlib.util
import json
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402
import compare_menu as cm  # noqa: E402
import compare_provenance as cprov  # noqa: E402

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


def main():
    import tomllib
    with tempfile.TemporaryDirectory() as t:
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
    with tempfile.TemporaryDirectory() as t:
        check_menu_route(Path(t))
    print("compare environment: round trip ok")


if __name__ == "__main__":
    main()
