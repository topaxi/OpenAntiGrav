#!/usr/bin/env python3
"""Every grab of the HD/Fury original goes through the helper that records the world clock beside the frame.

    check-hd-clock-grab.py            # scan scripts/, then the selftest
    check-hd-clock-grab.py selftest   # constructed offenders only

The one grab path is `grab_with_clock` in `scripts/rpcs3-drive.py` (`screenshot` and `record_clock` wrap it); it writes
`<stem>.clock.json` next to the image. The pair's own record (`original.world_clock_offset_s`) is checked by
`compare_provenance.py scan` (rule `hd_world_clock`). This script fails an RPCS3/HD script that takes a frame
with its own `import -window root`, `xwd`, `grim` or `ffmpeg x11grab` instead, a film (`x11grab`) that does not call
`record_clock`, and a helper that stopped writing the sidecar. PPSSPP, PCSX2, shadPS4 and Vita3K scripts are out of scope.
Docs: docs/tools/compare-page.md, "The world clock and random craft motion, per title".
"""

import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
HELPER = "rpcs3-drive.py"
#: RPCS3/HD scripts: the grabs of the HD original. `rpcs3-drive.py` is the helper and the only allowed raw grab.
IN_SCOPE = re.compile(r"^(rpcs3[-_]hd|hd[-_]|emu-hd|compare-hd|compare-rerender)")
RAW_GRAB = re.compile(r"""["']import["']\s*,|\bimport\s+-(display|window)\b|\bxwd\b|\bgrim\b|x11grab""")
FILM = re.compile(r"x11grab")


def offences(name, text):
    """`[(line, why)]` for one in-scope script's text."""
    found = []
    for n, line in enumerate(text.splitlines(), 1):
        if line.lstrip().startswith("#") or not RAW_GRAB.search(line):
            continue
        if FILM.search(line):
            if "record_clock(" not in text:
                found.append((n, "a film grab that never calls drive.record_clock(...)"))
        else:
            found.append((n, "grabs a frame itself; use drive.screenshot(path, crop=, display=) so the clock is written beside it"))
    return found


def helper_problem(text):
    """Why the helper no longer writes the clock beside a frame, or `None`."""
    body = text.split("def grab_with_clock(", 1)
    if len(body) < 2:
        return "rpcs3-drive.py has no grab_with_clock"
    fn = body[1].split("\ndef ", 1)[0]
    if "world_clock_now(" not in fn or "clock_sidecar_path(" not in fn or "write_text(" not in fn:
        return "grab_with_clock does not read the clock and write the sidecar"
    shot = text.split("def screenshot(", 1)[1].split("\ndef ", 1)[0] if "def screenshot(" in text else ""
    if "grab_with_clock(" not in shot:
        return "screenshot() does not go through grab_with_clock"
    return None


def scan():
    bad = 0
    scripts = sorted(p for p in HERE.iterdir() if p.suffix in (".py", ".sh") and IN_SCOPE.match(p.name)
                     and p.name != Path(__file__).name)
    for p in scripts:
        for n, why in offences(p.name, p.read_text()):
            print("GRAB   scripts/%s:%d %s" % (p.name, n, why))
            bad += 1
    problem = helper_problem((HERE / HELPER).read_text())
    if problem:
        print("HELPER scripts/%s: %s" % (HELPER, problem))
        bad += 1
    print("check-hd-clock-grab: %d HD scripts scanned, %d problem(s)" % (len(scripts), bad))
    return 1 if bad else 0


def selftest():
    assert offences("rpcs3-hd-x.py", 'subprocess.run(["import", "-display", d, "-window", "root", p])'), "a raw import grab"
    assert offences("emu-hd-x.sh", "import -window root shot.png"), "a shell grab"
    assert offences("hd-x.py", 'run(["xwd", "-root"])'), "xwd"
    assert offences("hd-x.py", 'ff = ["ffmpeg", "-f", "x11grab"]'), "a film without record_clock"
    assert not offences("hd-x.py", 'ff = ["ffmpeg", "-f", "x11grab"]\ndrive.record_clock(out)'), "a film with it"
    assert not offences("hd-x.py", "drive.screenshot(out / 'a.png', trim=True)"), "the helper"
    assert not offences("hd-x.py", "# import -window root is what screenshot() does"), "a comment"
    good = "def grab_with_clock(p):\n    c = world_clock_now()\n    clock_sidecar_path(p).write_text(c)\n\ndef screenshot(p):\n    return grab_with_clock(p)[0]\n"
    assert helper_problem(good) is None
    assert helper_problem(good.replace("clock_sidecar_path(p).write_text(c)", "pass")), "a helper that drops the sidecar"
    assert helper_problem(good.replace("return grab_with_clock(p)[0]", "return p")), "a screenshot that skips the helper"
    assert IN_SCOPE.match("rpcs3-hd-pair.py") and IN_SCOPE.match("hd-grid-capture.py") and not IN_SCOPE.match("psp-pair-capture.py")
    print("check-hd-clock-grab selftest ok")
    return 0


if __name__ == "__main__":
    if sys.argv[1:] == ["selftest"]:
        sys.exit(selftest())
    sys.exit(selftest() or scan())
