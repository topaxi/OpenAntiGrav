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
    record    the same, capturing the driven part as video.
    browse    walk to a named screen, then screenshot it at every step of a
              carousel without ever confirming into a race.

**Video is the observable a race actually has.** `TTY.log` goes quiet the moment
the front end hands over, and the GDB stub answers nothing while the target
runs, so a driven lap has no per-tick channel at all - but RPCS3's own overlay
records one, and HD's HUD puts speed, position, lap number and lap time in the
frame. `record` drives that: start, drive, stop, and the file lands in
`~/.config/rpcs3/recordings/<TITLE_ID>/`.

Needs `evdev` for anything that presses a button; run those through
`uv run --with evdev`. See docs/reverse-engineering/rpcs3-debugger.md.
"""

import argparse
import json
import glob
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
# Taller than 720p on purpose: RPCS3's own home-menu overlay is nine rows and
# does not scroll, so at 1280x720 the last three - `SaveState` among them - are
# simply not on screen, and the highlight walking off the bottom reads exactly
# like a dead d-pad.
#
# **This is a ceiling on what a capture can measure, not only a menu fix.** The
# frame that lands in a screenshot is whatever RPCS3 presents into its own
# window, and with `Start games in fullscreen mode` that is the display's size
# capped by the game's `Resolution Scale`. HD's own output is 1280x720, so a
# capture at scale 100 is 1278x718 after the black trim - fine for reading a
# colour, and too coarse to measure a 17x7-pixel widget corner, which is the
# measurement `handover/hds-strip-tabs-have-no-shape-and-no-scene-behind-them.md`
# actually needed and could not get. Raise both this and `Resolution Scale`
# together when a capture has to resolve geometry; raising either alone does
# nothing, since the smaller of the two is what the frame ends up at.
DISPLAY_GEOMETRY = os.environ.get("OAG_RPCS3_GEOMETRY", "1600x1200x24")
SCREEN_LINE = re.compile(r'Switching Screen "(.*?)" to "(.*?)"')

# RPCS3's *own* home menu, opened by the PS button - not the game's. Measured
# 2026-08-19 at 1600x1200; the list wraps, and at 720p only the first six rows
# are visible with no scrolling, which reads exactly like a dead d-pad. Reached
# from the top by `up`, since counting backwards from `Exit Game` is both fewer
# presses and further from selecting it by accident.
HOME_MENU = [
    "Resume Game", "Settings", "Trophies", "Take Screenshot",
    "Start/Stop Recording", "Toggle Fullscreen", "SaveState",
    "Restart Game", "Exit Game",
]

# Measured: every step is the default highlighted row, so the walk into a race
# needs no d-pad at all. It is written as the *screens* rather than as six
# presses because a press is genuinely dropped now and then - HD runs at about
# 9 fps here, and a `cross` that lands mid-transition does nothing. Keying on
# `TTY.log` and re-pressing is the difference between a walk that works and one
# that ends on `Team Selection` about a third of the time.
RACE_WALK = [
    "Campaign Selection",
    "Grid Selection Fury",
    "Cell Selection",
    "Team Selection",
    "Launch Game",
    "InGame",
]

# Where the walk is finished. `Launch Game` is *transient* - it can come and go
# between two polls - so nothing here waits for a named screen: the walk presses
# until the screen changes at all and stops when it recognises one of these.
# Waiting for `Launch Game` by name overshoots straight past `InGame` into
# `HUD`, which is what the first version of this did.
RACE_ARRIVED = {"InGame", "HUD"}

# HD prints this on every track load, which is where a capture gets its
# circuit from for free - the GDB stub is never asked.
TRACK_LINE = re.compile(r"Loading track model (\S+)")


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
        ["Xvfb", ":%d" % DISPLAY_NUMBER, "-screen", "0", DISPLAY_GEOMETRY,
         "-listen", "tcp", "-nolisten", "unix"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        start_new_session=True)
    for _ in range(20):
        time.sleep(0.5)
        if display_running():
            return True
    raise RuntimeError("Xvfb :%d did not come up" % DISPLAY_NUMBER)


def recordings(title_id="BCES00664"):
    """Every recording RPCS3 has written for a title, oldest first.

    The path is `recordings/<TITLE_ID>/*.mp4` - a *subdirectory* per title,
    which is worth stating because globbing `recordings/*` finds nothing and
    reads as "recording silently did not work".
    """
    root = os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config")
    pattern = os.path.join(root, "rpcs3", "recordings", title_id, "*.mp4")
    return sorted(glob.glob(pattern), key=os.path.getmtime)


def clear_stale_lock():
    """Remove RPCS3's instance lock when no RPCS3 is running.

    **`__exit__` terminates the emulator, and a terminated RPCS3 does not clean
    up after itself.** It leaves `~/.cache/rpcs3/RPCS3.buf`, and the next launch
    sees it and refuses - so one scripted run poisons every run after it until
    somebody deletes a file nothing told them about.

    How that refusal presents has changed and the new form is worse. This file's
    own docs record it as "the new process is gone within seconds"; on build
    0.0.42-19777 it is a **modal dialog** on the virtual display instead
    ("Another instance of RPCS3 is running"), so the process stays up, writes
    nothing to `TTY.log`, and every wait here times out against what looks
    exactly like a game that booted and stalled. Two runs were lost to it.

    Guarded on there being no live process, so this cannot pull the lock out
    from under a real one - a second instance is genuinely not supported.
    """
    if subprocess.run(["pgrep", "-x", "rpcs3"], capture_output=True).returncode == 0:
        return False
    lock = Path(os.environ.get("XDG_CACHE_HOME")
                or os.path.expanduser("~/.cache")) / "rpcs3" / "RPCS3.buf"
    if not lock.exists():
        return False
    lock.unlink()
    print("removed a stale %s left by a terminated run" % lock, flush=True)
    return True


def emulator_env():
    env = dict(os.environ)
    env["DISPLAY"] = DISPLAY
    env["QT_QPA_PLATFORM"] = "xcb"
    env.pop("WAYLAND_DISPLAY", None)
    return env


def screenshot(path, trim=False):
    """Grab the virtual root window. Returns the path, or None if it failed.

    `trim` crops the pure-black border away, which matters because the root
    window is the *display's* size and the emulator's frame is a rectangle
    somewhere inside it - a comparison against a rendered frame has to be of
    the frame, not of the desktop it happened to be presented on. The crop is
    on exactly `#000000` and nothing near it, so a dark scene keeps its own
    black; only pixels no renderer wrote are removed.
    """
    result = subprocess.run(
        ["import", "-display", DISPLAY, "-window", "root", str(path)],
        capture_output=True)
    if result.returncode != 0:
        print("screenshot failed: %s" % result.stderr.decode()[:160],
              file=sys.stderr)
        return None
    if trim:
        subprocess.run(
            ["mogrify", "-bordercolor", "black", "-fuzz", "0%", "-trim",
             "+repage", str(path)],
            capture_output=True)
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
        clear_stale_lock()
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

    def wait_for_screen_pressing(self, name, timeout=180.0, press_interval=5.0):
        """Like `wait_for_screen`, but presses `cross` while it waits.

        The boot chain ahead of `Main Menu` carries at least one real dialog,
        not just auto-redirects: `EpilepsyWarning`'s `<Dialog>` has
        `StartEnabled="false"` on its `<Redirect>` twin, so nothing here
        advances it without a press - measured 2026-08-30, where a plain
        `wait_for_screen("Main Menu", ...)` sat at `EpilepsyWarning` for the
        whole timeout with the emulator otherwise healthy. Pressing `cross`
        elsewhere in the chain is a no-op on an auto-redirect screen, so one
        button serves the whole walk. Stops **before** ever pressing at `name`
        itself - the same press means something different once arrived, e.g.
        `cross` on `Main Menu` is `walk_to_race`'s first step, not this one's.
        """
        deadline = time.time() + timeout
        while time.time() < deadline:
            if current_screen() == name:
                return True
            if self.proc.poll() is not None:
                raise RuntimeError("RPCS3 exited before reaching %r" % name)
            was, now = self.press_once("cross", settle=press_interval)
            print("  press      %-22s -> %-22s" % (was, now), flush=True)
        return current_screen() == name

    def tap(self, button, settle=4.0):
        was = current_screen()
        self.pad.press(button, 0.15)
        time.sleep(settle)
        return was, current_screen()

    def home_menu_select(self, item, settle=1.0):
        """Open RPCS3's overlay and highlight `item`, without confirming it.

        Only the `SaveState` path has been walked end to end; the rest is the
        same arithmetic on a list that was read off a screenshot, so treat a
        first use of another row as unverified.
        """
        if item not in HOME_MENU:
            raise KeyError("no such home-menu item: %r" % item)
        self.pad.press("ps", 0.30)
        time.sleep(4.0)
        for _ in range(len(HOME_MENU) - HOME_MENU.index(item)):
            self.pad.press("up", 0.12)
            time.sleep(settle)

    def take_screenshot(self):
        """RPCS3's own frame grab, which is not the same picture as `screenshot`.

        `screenshot` grabs the X root window, so what it returns is whatever
        the emulator *presented* into its own window - 1278x718 after the black
        trim, because the window is 1280x720 and stays that size on a bare
        Xvfb with no window manager to fullscreen it, whatever the display's
        own geometry or `Resolution Scale` say. This one goes through the home
        menu instead and writes the *framebuffer*, which is the picture to
        measure a widget's geometry off. It lands in
        `~/.config/rpcs3/screenshots/<TITLE_ID>/`.
        """
        self.home_menu_select("Take Screenshot")
        self.pad.press("cross", 0.15)
        time.sleep(3.0)

    def toggle_recording(self):
        """Start or stop RPCS3's own capture. The overlay item is a toggle."""
        self.home_menu_select("Start/Stop Recording")
        self.pad.press("cross", 0.15)
        time.sleep(3.0)

    def press_once(self, button, settle=5.0):
        """Press, then wait for the screen to change. Returns (was, now)."""
        was = current_screen()
        self.pad.press(button, 0.15)
        deadline = time.time() + settle
        while time.time() < deadline and current_screen() == was:
            time.sleep(0.3)
        return was, current_screen()

    #: Where `photograph` writes, set by a caller that wants the menus seen.
    screens_seen = None
    shot_dir = None

    #: Seconds to let a screen settle before photographing it. HD animates
    #: every transition and `TTY.log` names the new screen at its *start*, so a
    #: shot taken on the name change catches the animation: the first pass of
    #: this photographed two menus mid-flight and neither was readable.
    SCREEN_SETTLE = 3.0

    def photograph(self, was_screen):
        """One frame per distinct screen, for a caller writing a `--nav` plan."""
        if self.screens_seen is None or was_screen in self.screens_seen:
            return
        self.screens_seen.add(was_screen)
        time.sleep(self.SCREEN_SETTLE)
        name = "".join(c if c.isalnum() else "-" for c in was_screen)
        screenshot((self.shot_dir or self.log_dir) / ("screen-%s.png" % name),
                   trim=True)

    def navigate(self, plan):
        """Presses buttons at named screens on the way into a race.

        `plan` maps a screen name to a list of buttons, so
        `{"Cell Selection": ["right", "right", "cross"]}` moves two cells
        along before confirming. It exists because the default walk always
        lands on the same circuit - every step is the highlighted row - and a
        reference frame is only worth capturing for the circuit being
        investigated.

        The d-pad produces **no `TTY.log` line**: moving a highlight is not a
        screen change. So a move cannot be confirmed the way a transition can,
        and this settles for a fixed pause per press rather than pretending to
        observe one.
        """
        for button in plan.get(current_screen(), []):
            if button in ("cross", "circle"):
                self.press_once(button)
            else:
                self.tap(button, settle=1.2)
            print("    nav %-8s at %s" % (button, current_screen()), flush=True)

    def walk_to_race(self, max_presses=12, plan=None):
        """Press cross until HD is in a race, or give up and say where it got to.

        Deliberately not a fixed count: HD runs at about 9 fps here and a press
        landing mid-transition does nothing, so `RACE_WALK` is what the path was
        *measured* to be and this loop is what survives a dropped press.
        """
        for index in range(1, max_presses + 1):
            if current_screen() in RACE_ARRIVED:
                break
            if plan:
                self.navigate(plan)
                if current_screen() in RACE_ARRIVED:
                    break
            self.photograph(was_screen=current_screen())
            was, now = self.press_once("cross")
            print("  press %2d  %-22s -> %-22s%s"
                  % (index, was, now, "" if now != was else "   (dropped)"),
                  flush=True)
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


def cmd_shot(args):
    """Boot to a named screen and take RPCS3's own framebuffer screenshot.

    For the case a root-window grab cannot serve: measuring a widget's own
    geometry, where the question is what the console rasterised rather than
    what colour it came out. See `Session.take_screenshot`.
    """
    before = set(emulator_screenshots())
    with Session(args.image, args.log_dir) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen_pressing(args.screen, args.timeout):
            print("never reached %r (last screen: %s)"
                  % (args.screen, current_screen()), file=sys.stderr)
            return 1
        time.sleep(args.settle)
        session.take_screenshot()
    new = [p for p in emulator_screenshots() if p not in before]
    if not new:
        print("the home menu did not write a screenshot", file=sys.stderr)
        return 1
    for path in new:
        print("wrote %s" % path, flush=True)
    return 0


def emulator_screenshots(title_id="BCES00664"):
    """Every screenshot RPCS3 has written for a title, oldest first."""
    root = os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config")
    pattern = os.path.join(root, "rpcs3", "screenshots", title_id, "*.png")
    return sorted(glob.glob(pattern), key=os.path.getmtime)


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
        if screen not in RACE_ARRIVED:
            print("ended on %r rather than in a race" % screen, file=sys.stderr)
            return 1
        print("%s; waiting %g s for the track to load"
              % (current_screen(), args.load), flush=True)
        # **A burst rather than one shot at the end, because the loading screen
        # is transient.** The grid shot below is taken once the load is over, so
        # nothing here used to see the screen that covers it - and the load is
        # the only place that screen is ever up. HD runs at about 9 fps here and
        # the load takes tens of seconds, so an even sweep across the window
        # catches it several times over; which frames are the loading screen is
        # read off the pictures afterwards rather than predicted.
        if args.shots and args.load_shots > 0:
            step = args.load / args.load_shots
            for index in range(args.load_shots):
                time.sleep(step)
                screenshot(session.log_dir
                           / ("01b-loading-%02d.png" % (index + 1)))
        else:
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



# The EBOOT's initialised data and the BSS behind it - where a statically
# allocated render global lives. `readelf` puts segment 1 at 0x860000 with a
# file size of 0xd6f80 and a memory size past it; this span covers both and is
# 917,504 bytes, about a minute at the stub's 41 ms a packet.
DATA_SEGMENT = (0x00860000, 0x000E0000)


def parse_region(text):
    """A place to read, as `<chain>:<len>`.

    A chain is an address expression with `@` as a **suffix** meaning "read the
    32-bit pointer here and carry on from there":

        0x860000                 a literal address
        r2                       the PPU's TOC pointer, out of the registers
        0x936fd4@                the pointer stored at that address
        r2+0x6828@+0x14@         `[[r2 + 0x6828] + 0x14]`

    A suffix rather than a prefix because that is the order the reads happen
    in, and because it makes a chain of any length one rule instead of two.

    **The register form is what reaches a heap object at all.** `viewProj` is
    not in the executable's data or BSS: a live dump of the whole `0xe0000`
    span holds seven perspective matrices and every one is a static constant -
    exact to the bit, eye at the origin - so the live camera is in a heap
    allocation, and the only handles on those are chains that start at a
    register or a global.
    """
    body, _, size = text.rpartition(":")
    if not body:
        body, size = size, ""
    return (body, int(size, 0) if size else 0x10000)


def resolve_chain(gdb, chain):
    """Walks a `parse_region` chain against a stopped target, or `None`.

    A null anywhere ends the walk with `None` rather than reading address zero:
    a render context that has not been built yet is an ordinary state early in
    a boot, not an error to raise through.
    """
    terms = chain.split("@")
    value = _term(gdb, terms[0])
    for step in terms[1:]:
        if not value:
            return None
        # The null test is on what the pointer *held*, before the offset is
        # added. Testing after it turns a null pointer plus `+0x14` into the
        # perfectly non-zero address `0x14`, which the stub then answers `E01`
        # for - and that read is what killed a whole boot.
        value = int.from_bytes(gdb.read(value, 4), "big")
        if not value:
            return None
        value += _offset(step)
    return value or None


def _term(gdb, text):
    """One chain term: a number, or `rN` with offsets after it."""
    text = text.strip()
    if text[:1] == "r" and text[1:2].isdigit():
        head, rest = text, ""
        for k, ch in enumerate(text):
            if ch in "+-":
                head, rest = text[:k], text[k:]
                break
        return gdb.gpr(int(head[1:])) + _offset(rest)
    return _offset(text)


def _offset(text):
    """A sum of signed numbers, `""` being zero."""
    total, sign, number = 0, 1, ""
    text = text.strip()
    if text and text[0] not in "+-":
        text = "+" + text
    for ch in text:
        if ch in "+-":
            if number:
                total += sign * int(number, 0)
            sign, number = (1 if ch == "+" else -1), ""
        else:
            number += ch
    if number:
        total += sign * int(number, 0)
    return total


def cmd_capture(args):
    """One boot, many poses: screenshot and camera, paired at the same instant.

    The pairing is the point. `screenshot()` grabs the virtual root window, so
    it shows the last frame RPCS3 presented and is unaffected by the target
    being stopped - which means the stop can come *first*, and the memory read
    describes the frame that is on screen rather than one a few frames later.

    One debugger session per emulator launch is not a style choice - see
    `rpcs3_debugger`'s trap list - so this loops inside a single connection
    rather than booting per pose.
    """
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import ps3_pose
    from rpcs3_debugger import Debugger

    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    regions = [parse_region(r) for r in args.region] or [
        ("%#x" % DATA_SEGMENT[0], DATA_SEGMENT[1])]

    with Session(args.image, args.log_dir) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen("Main Menu", args.timeout):
            print("never reached the Main Menu (last screen: %s)"
                  % current_screen(), file=sys.stderr)
            return 1
        time.sleep(args.settle)
        # One boot that photographs every screen is what makes a navigation
        # plan writable at all: the menus are the disc's, nothing describes
        # them, and `TTY.log` names a screen without saying what is on it.
        if args.nav_shots:
            session.screens_seen = set()
            session.shot_dir = out
        plan = {}
        for item in args.nav:
            screen, _, buttons = item.partition("=")
            plan[screen] = [b.strip() for b in buttons.split(",") if b.strip()]
        if session.walk_to_race(plan=plan) not in RACE_ARRIVED:
            print("ended on %r rather than in a race" % current_screen(),
                  file=sys.stderr)
            return 1
        print("in a race; waiting %g s for the track to load" % args.load,
              flush=True)
        time.sleep(args.load)

        track = track_name()
        print("track: %s" % (track or "<not logged>"), flush=True)

        with Debugger() as gdb:
            for n in range(args.shots):
                session.pad.set("cross", True)
                gdb.resume()
                time.sleep(args.interval)
                gdb.pause()
                session.pad.set("cross", False)

                stem = "%02d" % n
                shot = screenshot(out / ("%s.png" % stem), trim=True)
                blobs = []
                for chain, size in regions:
                    # A region that will not resolve or will not read is
                    # reported and skipped, never raised: the boot that got
                    # here cost three minutes and the other regions are still
                    # worth having.
                    try:
                        at = resolve_chain(gdb, chain)
                    except Exception as error:  # noqa: BLE001 - see above
                        print("  %s: %s" % (chain, error), flush=True)
                        continue
                    if not at:
                        print("  %s resolves to nothing yet" % chain, flush=True)
                        continue
                    print("  reading %#x bytes at %#010x (%s)"
                          % (size, at, chain), flush=True)
                    try:
                        blobs.append((at, gdb.read(at, size)))
                    except Exception as error:  # noqa: BLE001 - see above
                        print("  %#010x: %s" % (at, error), flush=True)

                pose = describe(blobs, track, shot)
                (out / ("%s.json" % stem)).write_text(
                    json.dumps(pose, indent=2) + "\n")
                found = pose["camera"]
                if found:
                    print("  %s: eye %s fov %.2f deg" % (
                        stem,
                        ["%.1f" % v for v in found.get("eye", [])],
                        found.get("fov_y_deg", float("nan"))), flush=True)
                    if "render_with" in found:
                        print("       " + found["render_with"], flush=True)
                else:
                    print("  %s: no camera in the regions read" % stem, flush=True)
                if args.keep_dumps:
                    for at, blob in blobs:
                        (out / ("%s-%08x.bin" % (stem, at))).write_bytes(blob)
            gdb.resume()

    print("done; %d pose(s) in %s" % (args.shots, out), flush=True)
    return 0


def cmd_browse(args):
    """Screenshot a carousel screen at every step, without ever racing it.

    `capture`'s `--nav` only fires a plan's buttons once, on arrival at a named
    screen, and the loop that runs it always follows with `cross` - so a plan
    of many `right`s would move a carousel's highlight that many times with
    nothing photographed in between, then commit to whatever it landed on.
    That is right for reaching one chosen circuit to race, and wrong for a
    screen meant to be *read*: Racebox's `Track Creation` carousel, which is
    what `entries.xml`'s circuit names actually reach the front end through,
    is the case this exists for - see
    docs/formats/hd-frontend.md#a-circuits-name-is-in-a-different-archive-from-the-circuit-list.

    So this walks the `--nav` plan to reach `--screen` exactly as `capture`
    does, then stops confirming: from there it only taps `--button`,
    screenshotting after each one, for `--steps` presses.
    """
    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    plan = {}
    for item in args.nav:
        screen, _, buttons = item.partition("=")
        plan[screen] = [b.strip() for b in buttons.split(",") if b.strip()]

    with Session(args.image, args.log_dir) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen_pressing("Main Menu", args.timeout):
            print("never reached the Main Menu (last screen: %s)"
                  % current_screen(), file=sys.stderr)
            return 1
        time.sleep(args.settle)
        for index in range(1, args.max_presses + 1):
            if current_screen() == args.screen:
                break
            session.navigate(plan)
            if current_screen() == args.screen:
                break
            was, now = session.press_once("cross")
            print("  press %2d  %-22s -> %-22s" % (index, was, now),
                  flush=True)
        if current_screen() != args.screen:
            print("ended on %r rather than %r"
                  % (current_screen(), args.screen), file=sys.stderr)
            return 1
        print("at %r; browsing %d step(s) with %r"
              % (args.screen, args.steps, args.button), flush=True)
        screenshot(out / "00.png", trim=True)
        for n in range(1, args.steps + 1):
            session.tap(args.button, settle=args.settle_step)
            screenshot(out / ("%02d.png" % n), trim=True)
            print("  %02d" % n, flush=True)

    print("done; %d shot(s) in %s" % (args.steps + 1, out), flush=True)
    return 0


def track_name():
    """The circuit RPCS3's own TTY log says was loaded, or `None`.

    Free: HD prints `Loading track model Data\\Environments\\...` on every
    load, so the one thing the capture needs that is not in the camera costs no
    packets at all.
    """
    hits = TRACK_LINE.findall(tty_text())
    return hits[-1] if hits else None


def describe(blobs, track, shot):
    """The capture record: what a frame needs to be reproduced, and nothing more.

    Deliberately not a memory dump. The raw bytes are game memory - extracted
    executable data, which `just audit-leakage` refuses and CI's leakage job
    would fail on - so they stay under `data/` and out of this file, which
    carries only the numbers a renderer is set up from.
    """
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import ps3_pose

    best = None
    for at, blob in blobs:
        for hit in ps3_pose.candidates(blob, base=at):
            address, order, error, eye, matrix = hit
            if best is None or error < best[2]:
                best = hit
    camera = None
    if best:
        address, order, error, eye, matrix = best
        pose = ps3_pose.decompose(matrix)
        camera = {
            "address": "%#010x" % address,
            "order": order,
            "unit_error": error,
            "view_proj": matrix,
        }
        if pose:
            camera.update(pose)
            camera["render_with"] = ps3_pose.command_line(pose, track)
        else:
            camera["eye"] = list(eye)
    return {
        "track": track,
        "screenshot": str(shot) if shot else None,
        "camera": camera,
    }


def cmd_record(args):
    before = set(recordings())
    with Session(args.image, args.log_dir) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen("Main Menu", args.timeout):
            print("never reached the Main Menu (last screen: %s)"
                  % current_screen(), file=sys.stderr)
            return 1
        time.sleep(args.settle)
        print("walking the menus:", flush=True)
        # One boot that photographs every screen is what makes a navigation
        # plan writable at all: the menus are the disc's, nothing describes
        # them, and `TTY.log` names a screen without saying what is on it.
        if args.nav_shots:
            session.screens_seen = set()
            session.shot_dir = out
        plan = {}
        for item in args.nav:
            screen, _, buttons = item.partition("=")
            plan[screen] = [b.strip() for b in buttons.split(",") if b.strip()]
        if session.walk_to_race(plan=plan) not in RACE_ARRIVED:
            print("ended on %r rather than in a race" % current_screen(),
                  file=sys.stderr)
            return 1
        print("%s; waiting %g s for the track to load"
              % (current_screen(), args.load), flush=True)
        time.sleep(args.load)
        print("starting the recording", flush=True)
        session.toggle_recording()
        print("driving %g s" % args.drive, flush=True)
        session.pad.set("cross", True)
        time.sleep(args.drive)
        session.pad.set("cross", False)
        print("stopping the recording", flush=True)
        session.toggle_recording()
        time.sleep(8.0)
    fresh = [path for path in recordings() if path not in before]
    if not fresh:
        print("no new recording appeared - check RPCS3.log for `video_encoder`",
              file=sys.stderr)
        return 1
    for path in fresh:
        print("%s  %.1f MiB" % (path, os.path.getsize(path) / 1048576.0))
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

    shot = sub.add_parser("shot")
    shot.add_argument("--screen", default="Main Menu")
    shot.add_argument("--timeout", type=float, default=180.0)
    shot.add_argument("--settle", type=float, default=12.0)
    shot.set_defaults(run=cmd_shot)

    race = sub.add_parser("race")
    race.add_argument("--timeout", type=float, default=180.0)
    race.add_argument("--settle", type=float, default=12.0,
                      help="pause after the Main Menu before the first press")
    race.add_argument("--load", type=float, default=50.0,
                      help="pause after 'InGame' for the track to load")
    race.add_argument("--drive", type=float, default=0.0,
                      help="seconds to hold thrust once the race is up")
    race.add_argument("--load-shots", type=int, default=0,
                      help="with --shots, this many evenly spaced screenshots "
                           "across the load window - the only way to catch the "
                           "loading screen, which is up for none of the run "
                           "either side of it")
    race.add_argument("--shots", action="store_true",
                      help="capture the menu, the grid and the driven frame")
    race.set_defaults(run=cmd_race)

    cap = sub.add_parser("capture",
                         help="screenshot and camera, paired, many per boot")
    cap.set_defaults(run=cmd_capture)
    cap.add_argument("--timeout", type=float, default=180.0)
    cap.add_argument("--settle", type=float, default=12.0)
    cap.add_argument("--load", type=float, default=50.0)
    cap.add_argument("--shots", type=int, default=3,
                     help="how many poses to capture in this one boot")
    cap.add_argument("--interval", type=float, default=6.0,
                     help="seconds of thrust between poses")
    cap.add_argument("--out", default="data/reference/hd-capture",
                     help="where the frames and their poses are written")
    cap.add_argument("--region", action="append", default=[],
                     help="addr:len to dump, or @addr:len to dereference "
                          "a pointer at addr first; repeatable. Defaults to "
                          "the EBOOT's data and BSS.")
    cap.add_argument("--nav-shots", action="store_true",
                     help="photograph every distinct screen on the way in, so "
                          "a --nav plan can be written from what is actually "
                          "on them")
    cap.add_argument("--nav", action="append", default=[],
                     metavar="SCREEN=BUTTONS",
                     help="buttons to press at a named screen before "
                          "continuing, e.g. \"Cell Selection=right,right\". "
                          "Repeatable; this is how a capture reaches a circuit "
                          "other than the one every default row leads to.")
    cap.add_argument("--keep-dumps", action="store_true",
                     help="also write the raw memory, which is game data and "
                          "stays under data/")

    browse = sub.add_parser("browse",
                            help="screenshot a carousel screen at every step, "
                                 "with no race entered")
    browse.set_defaults(run=cmd_browse)
    browse.add_argument("--timeout", type=float, default=180.0)
    browse.add_argument("--settle", type=float, default=12.0,
                        help="pause after the Main Menu before the first "
                             "press")
    browse.add_argument("--settle-step", type=float, default=1.5,
                        help="pause after each carousel press before its "
                             "screenshot")
    browse.add_argument("--nav", action="append", default=[],
                        metavar="SCREEN=BUTTONS",
                        help="buttons to press at a named screen on the way "
                             "to --screen, same syntax as capture's --nav, "
                             "e.g. \"Main Menu=right\"")
    browse.add_argument("--max-presses", type=int, default=8,
                        help="cross-presses allowed while walking the --nav "
                             "plan before giving up on reaching --screen")
    browse.add_argument("--screen", default="Track Creation",
                        help="the screen to browse once reached")
    browse.add_argument("--button", default="right",
                        help="the button that advances the carousel")
    browse.add_argument("--steps", type=int, default=27,
                        help="how many times to press --button, one "
                             "screenshot each, after the unpressed 00.png")
    browse.add_argument("--out", default="data/reference/hd-browse",
                        help="where the screenshots are written")

    rec = sub.add_parser("record")
    rec.add_argument("--timeout", type=float, default=180.0)
    rec.add_argument("--settle", type=float, default=12.0)
    rec.add_argument("--load", type=float, default=50.0)
    rec.add_argument("--drive", type=float, default=30.0,
                     help="seconds of held thrust to capture")
    rec.set_defaults(run=cmd_record)

    args = parser.parse_args(argv)
    return args.run(args)


if __name__ == "__main__":
    sys.exit(main())
