#!/usr/bin/env python3
"""Boot WipEout HD in RPCS3, walk it into a race, and drive - with no window.

This is the PS3 counterpart of `scripts/psp-drive.py`, and it is shaped
differently because RPCS3 gives and withholds different things than PPSSPP does:

- **There is no input API**, so a button press is a real kernel device -
  `scripts/rpcs3_pad.py`, and its preflight is why a missing permission reads as
  a diagnosis instead of a `PermissionError`.
- **There is no state API worth pacing on** - the GDB stub answers nothing at
  all while the target runs (`scripts/rpcs3_debugger.py`). What replaces it is
  the game's own `printf`: RPCS3 writes it to `~/.cache/rpcs3/TTY.log`, and HD
  logs `Switching Screen "A" to "B"` on every front-end transition. That single
  line is the whole state machine, free, and it is what every wait here keys on.

**`--headless` cannot reach the front end, and the way it fails is a trap.** The
null renderer boots, prints as far as `TROPHY: Checking free disk space`, and
then stops for good with the main thread parked inside `cellGameDataCheck` -
looking exactly like a game waiting for a button, which is what cost a session
here. It is not: presses land and change nothing, because the game never got to
a screen. A *real* renderer on a virtual display reaches `Main Menu` in about 45
seconds, and from there six taps of cross start a race on Talon's Junction.

So the display is not optional and is also not the user's desktop:

    python3 scripts/rpcs3-drive.py display          # start Xvfb :77
    uv run --with evdev python3 scripts/rpcs3-drive.py race --drive 20 --shots

Xvfb is started with `-listen tcp -nolisten unix` and addressed as
`127.0.0.1:77` deliberately: a sandboxed session may not be able to write
`/tmp/.X11-unix`, and then the unix socket never appears while the TCP port
does. `QT_QPA_PLATFORM=xcb` and dropping `WAYLAND_DISPLAY` are needed for the
same reason - RPCS3 is Qt, and on a Wayland session it will not look at an X
display unless told to.

Subcommands:

    display   start (or check) the virtual display everything else needs.
    preflight the pad, the input profile and the display, with each fix.
    boot      launch and wait for the Main Menu, then hold it there.
    race      boot, walk the menus into a race, optionally drive and screenshot.

Needs `evdev` for anything that presses a button; run those through
`uv run --with evdev`. See docs/reverse-engineering/rpcs3-debugger.md.
"""

import argparse
import os
import re
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import rpcs3_pad

TTY = os.path.expanduser("~/.cache/rpcs3/TTY.log")
DISPLAY_NUMBER = 77
DISPLAY = "127.0.0.1:%d" % DISPLAY_NUMBER
SCREEN_LINE = re.compile(r'Switching Screen "(.*?)" to "(.*?)"')

# Measured: Main Menu -> Campaign Selection -> Grid Selection Fury ->
# Cell Selection -> Team Selection -> Launch Game -> InGame. Every step is the
# default highlighted row, so the walk needs no d-pad at all - which is the
# reason it is this short and this reproducible.
MENU_WALK = ["cross"] * 6


def tty_text():
    try:
        with open(TTY, errors="replace") as handle:
            return handle.read()
    except FileNotFoundError:
        return ""


def current_screen():
    """The last screen HD said it switched to, or `?` before the first one."""
    hits = SCREEN_LINE.findall(tty_text())
    return hits[-1][1] if hits else "?"


def display_running():
    probe = subprocess.run(
        ["python3", "-c",
         "import socket,sys;"
         "s=socket.socket();s.settimeout(2);"
         "sys.exit(0 if s.connect_ex(('127.0.0.1', %d)) == 0 else 1)"
         % (6000 + DISPLAY_NUMBER)],
        capture_output=True)
    return probe.returncode == 0


def start_display():
    if display_running():
        return False
    subprocess.Popen(
        ["Xvfb", ":%d" % DISPLAY_NUMBER, "-screen", "0", "1280x720x24",
         "-listen", "tcp", "-nolisten", "unix"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        start_new_session=True)
    for _ in range(20):
        time.sleep(0.5)
        if display_running():
            return True
    raise RuntimeError("Xvfb :%d did not come up" % DISPLAY_NUMBER)


def emulator_env():
    env = dict(os.environ)
    env["DISPLAY"] = DISPLAY
    env["QT_QPA_PLATFORM"] = "xcb"
    env.pop("WAYLAND_DISPLAY", None)
    return env


def screenshot(path):
    """Grab the virtual root window. Returns the path, or None if it failed."""
    result = subprocess.run(
        ["import", "-display", DISPLAY, "-window", "root", str(path)],
        capture_output=True)
    if result.returncode != 0:
        print("screenshot failed: %s" % result.stderr.decode()[:160],
              file=sys.stderr)
        return None
    return path


class Session:
    """One emulator run, with the pad it must not outlive."""

    def __init__(self, image, log_dir):
        # Resolved here rather than passed through: `data/` is gitignored and
        # does not travel into a worktree, so a relative default silently
        # becomes a path RPCS3 answers `Invalid file or folder` for.
        self.image = Path(image).resolve()
        if not self.image.exists():
            raise SystemExit(
                "no such image: %s\n"
                "  `data/` is gitignored and absent from a worktree - run this "
                "from the main checkout, or pass --image with an absolute path"
                % self.image)
        self.log_dir = Path(log_dir)
        self.log_dir.mkdir(parents=True, exist_ok=True)
        self.pad = None
        self.proc = None

    def __enter__(self):
        start_display()
        # The pad has to exist before RPCS3 does: it binds pads when it
        # enumerates devices and does not rescan.
        self.pad = rpcs3_pad.Pad()
        open(TTY, "w").close()
        self.proc = subprocess.Popen(
            ["rpcs3", "--no-gui", "--input-config", rpcs3_pad.INPUT_CONFIG_NAME,
             str(self.image)],
            stdout=open(self.log_dir / "rpcs3.log", "w"),
            stderr=subprocess.STDOUT, start_new_session=True,
            env=emulator_env())
        return self

    def __exit__(self, *_):
        if self.proc is not None:
            self.proc.terminate()
        if self.pad is not None:
            self.pad.close()

    def wait_for_screen(self, name, timeout=180.0):
        deadline = time.time() + timeout
        while time.time() < deadline:
            if current_screen() == name:
                return True
            if self.proc.poll() is not None:
                raise RuntimeError("RPCS3 exited before reaching %r" % name)
            time.sleep(2)
        return False

    def tap(self, button, settle=4.0):
        was = current_screen()
        self.pad.press(button, 0.15)
        time.sleep(settle)
        return was, current_screen()

    def walk_to_race(self, settle=4.0):
        for index, button in enumerate(MENU_WALK, 1):
            was, now = self.tap(button, settle)
            print("  %d/%d  %-7s  %-22s -> %s"
                  % (index, len(MENU_WALK), button, was, now), flush=True)
        return current_screen()


def cmd_display(args):
    started = start_display()
    print("Xvfb :%d %s; address it as DISPLAY=%s"
          % (DISPLAY_NUMBER, "started" if started else "already running", DISPLAY))
    return 0


def cmd_preflight(args):
    ok = rpcs3_pad.report(stream=sys.stdout)
    if display_running():
        print("display OK: Xvfb is listening on %s" % DISPLAY)
    else:
        print("\n  no virtual display on %s - RPCS3 needs a real renderer to "
              "get past its trophy check, and --headless never will" % DISPLAY)
        print("\n      python3 scripts/rpcs3-drive.py display")
        ok = False
    return 0 if ok else 1


def cmd_boot(args):
    with Session(args.image, args.log_dir) as session:
        print("rpcs3 pid %d, waiting for the Main Menu" % session.proc.pid,
              flush=True)
        if not session.wait_for_screen("Main Menu", args.timeout):
            print("never reached the Main Menu (last screen: %s)"
                  % current_screen(), file=sys.stderr)
            return 1
        print("Main Menu. Holding for %g s." % args.hold, flush=True)
        time.sleep(args.hold)
    return 0


def cmd_race(args):
    with Session(args.image, args.log_dir) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen("Main Menu", args.timeout):
            print("never reached the Main Menu (last screen: %s)"
                  % current_screen(), file=sys.stderr)
            return 1
        print("Main Menu; settling", flush=True)
        time.sleep(args.settle)
        if args.shots:
            screenshot(session.log_dir / "01-menu.png")
        print("walking the menus:", flush=True)
        screen = session.walk_to_race()
        if screen != "InGame":
            print("ended on %r rather than InGame" % screen, file=sys.stderr)
            return 1
        print("InGame; waiting %g s for the track to load" % args.load,
              flush=True)
        time.sleep(args.load)
        if args.shots:
            screenshot(session.log_dir / "02-grid.png")
        if args.drive:
            print("holding thrust for %g s" % args.drive, flush=True)
            session.pad.set("cross", True)
            time.sleep(args.drive)
            session.pad.set("cross", False)
            if args.shots:
                screenshot(session.log_dir / "03-driven.png")
        print("done; artefacts in %s" % session.log_dir, flush=True)
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--image",
                        default="data/images/hdfury-ps3-eu-dec.iso",
                        help="the layer-1 DECRYPTED disc image")
    parser.add_argument("--log-dir", default="/tmp/rpcs3-drive",
                        help="where the emulator log and screenshots go")
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("display").set_defaults(run=cmd_display)
    sub.add_parser("preflight").set_defaults(run=cmd_preflight)

    boot = sub.add_parser("boot")
    boot.add_argument("--timeout", type=float, default=180.0)
    boot.add_argument("--hold", type=float, default=30.0)
    boot.set_defaults(run=cmd_boot)

    race = sub.add_parser("race")
    race.add_argument("--timeout", type=float, default=180.0)
    race.add_argument("--settle", type=float, default=12.0,
                      help="pause after the Main Menu before the first press")
    race.add_argument("--load", type=float, default=50.0,
                      help="pause after 'InGame' for the track to load")
    race.add_argument("--drive", type=float, default=0.0,
                      help="seconds to hold thrust once the race is up")
    race.add_argument("--shots", action="store_true",
                      help="capture the menu, the grid and the driven frame")
    race.set_defaults(run=cmd_race)

    args = parser.parse_args(argv)
    return args.run(args)


if __name__ == "__main__":
    sys.exit(main())
