#!/usr/bin/env python3
"""The menu route of the compare-page pair tooling: ours' side of a front-end screen, and the pair's checks.

A race pair (`rpcs3-hd-pair.py`, `psp-pulse-pair.py`) reads a pose, a camera and a clock out of the original and
renders ours at them. A **menu pair** has none of those: a front-end screen is a still. What makes it a pair is that
both sides show the same screen, at the same size, in the same language and styling, with the same row highlighted,
and that ours was drawn from the disc's own front-end XML rather than from this project's `assets/ui/menu.toml`
stand-in pages. `menu_pair` builds one, `Pair.finish` (compare_env.py) writes and refuses it like any other.

    import compare_menu as cm
    cm.menu_pair(pair, theirs_png, screen_seen=..., screen_wanted=..., page="track-select", drawn_from="xml", ...)

A page drawn from `menu.toml` is a **gap**, not a pair (`cm.GAP`): the original's frame is kept, nothing of ours is
set beside it, and `gap.json` says why. Docs: docs/tools/compare-page.md, "Menu pairs".
"""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402

#: What a page's `drawn_from` may be. `xml`: the disc's own screen/layout XML (a campaign, picker or loading page read
#: off the archive). `menu.toml`: this project's own page tree (`assets/ui/menu.toml`), the stand-in a pair must not use.
DRAWN_FROM = ("xml", "menu.toml", "none")
GAP = "menu.toml"

def is_gap(drawn_from):
    """Anything ours does not draw from the disc's XML is a gap: `menu.toml` (the stand-in), or `none` (ours has no such screen)."""
    return drawn_from != "xml"


def log(msg):
    print("[menu-pair] " + msg, file=sys.stderr, flush=True)


def extra_settings(profile_dir, front_end_style=None, race=None):
    """Append the keys a menu still needs to ours' private `settings.toml`: the front-end styling (HD or FURY) and the
    race selection a picker highlights (`race.team`, `race.track`, `race.mode`). `ce.write_ours_settings` wrote the
    render profile; the game rewrites the file on its first run and keeps these values."""
    path = ce.settings_path(profile_dir)
    text = path.read_text()
    if front_end_style:
        text = text.replace("[display]\n", '[display]\nfront_end_style = "%s"\n' % front_end_style, 1)
    if race:
        text += "\n[race]\n" + "".join('%s = "%s"\n' % (k, v) for k, v in race.items())
    path.write_text(text)


def ours_menu_command(oag_game, image, plat, page, size):
    """`oag-game IMAGE --menu-page PAGE ...` with the profile's flags. The race-only flags (`--no-hull-shake`,
    `--pilot-assist`) are dropped: they change nothing on a menu and the record should not claim them."""
    flags = ce.ours_flags(plat)
    drop = {"--no-hull-shake"}
    clean, skip = [], False
    for i, f in enumerate(flags):
        if skip:
            skip = False
        elif f in drop:
            continue
        elif f == "--pilot-assist":
            skip = True
        else:
            clean.append(f)
    clean[clean.index("--size") + 1] = size
    return [str(oag_game), str(image), "--menu-page", page] + clean


def render_ours(oag_game, image, plat, page, out_png, profile_dir, size, extra_args=()):
    """Render ours' page into `out_png`; returns the command that did. Runs under the pair's private XDG profile."""
    cmd = ours_menu_command(oag_game, image, plat, page, size) + list(extra_args) + ["--screenshot", str(out_png)]
    env = dict(os.environ, **ce.ours_env(profile_dir), RUST_LOG="warn")
    log("+ " + " ".join(cmd[2:]))
    subprocess.run(cmd, check=True, env=env, stdout=subprocess.DEVNULL)
    return cmd


def menu_pair(pair, *, theirs_png, oag_game, image, page, drawn_from, xml_file, screen_seen, screen_wanted,
              profile_dir, size, front_end_style, language, original_language, selection, extra_args=(), notes=()):
    """Fill `pair` for one front-end screen and check it. `drawn_from` is how ours draws `page` (`DRAWN_FROM`); the
    pair is refused (exit 3 from `Pair.finish`) when it is `menu.toml`, when the screen names differ, when a size is
    off, or when the language or styling differ. `selection` records the highlighted row on both sides; `{"matched": False, "why": ...}` ships the pair with the difference
    named as a caveat (ours cannot select what the original shows) instead of refusing it."""
    out = pair.out
    plat = pair.plat
    pair.rec["kind"] = "menu"
    pair.rec["original"]["screen"] = {"seen": screen_seen, "wanted": screen_wanted, "source": "TTY.log, Switching Screen"}
    pair.rec["original"]["language"] = original_language
    pair.rec["original"]["front_end_style"] = front_end_style
    pair.rec["original"]["selection"] = selection["original"]
    pair.rec["ours"]["language"] = language
    pair.rec["ours"]["front_end_style"] = front_end_style
    pair.rec["ours"]["selection"] = selection["ours"]
    pair.rec["ours"]["page"] = page
    pair.rec["ours"]["drawn_from"] = {"kind": drawn_from, "xml": xml_file}
    pair.rec["ours"]["binary"] = ce.binary_record(oag_game)
    pair.check("ours draws this page from the disc's XML, not menu.toml", drawn_from == "xml",
               "page %s drawn from %s (%s)" % (page, drawn_from, xml_file))
    pair.check_equal("original screen is the one asked for", screen_seen, screen_wanted)
    pair.check_equal("language", original_language, language)
    if selection.get("matched", True):
        pair.check_equal("selected row", selection["original"], selection["ours"])
    else:
        # the screen is the same and the state is not: the pair ships with the difference named, never as matched
        pair.rec["ours"]["selection_matched"] = False
        pair.caveat("state not matched: the original shows %s, ours %s%s" % (selection["original"], selection["ours"],
                    "; " + selection["why"] if selection.get("why") else ""))
    shutil.copyfile(theirs_png, out / "theirs.png")
    pair.check_size("original", out / "theirs.png")
    if pair.failed():
        return False  # a doomed pair is not rendered; `Pair.finish` names every failed check
    cmd = render_ours(oag_game, image, plat, page, out / "ours.png", profile_dir, size, extra_args)
    pair.rec["ours"]["flags"] = [str(c) for c in cmd[2:]]
    got, bad = ce.read_back(plat, profile_dir)
    pair.rec["ours"]["settings_readback"] = got
    for b in bad:
        pair.check("ours settings", False, b)
    pair.check("ours settings read back", not bad, "%d keys equal the profile" % (len(got) - len(bad)))
    pair.check_size("ours", out / "ours.png")
    for note in notes:
        pair.caveat(note)
    return True


def write_gap(out, theirs_png, page, drawn_from, why):
    """A screen ours draws from `menu.toml`: the original's frame is kept and `gap.json` says why there is no pair."""
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(theirs_png, out / "theirs.png")
    (out / "gap.json").write_text(json.dumps({"page": page, "drawn_from": drawn_from, "why": why}, indent=2) + "\n")
