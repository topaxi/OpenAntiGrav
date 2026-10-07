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
    python3 scripts/rpcs3-drive.py stop             # stop RPCS3 and Xvfb :77

Xvfb is started with `-listen tcp -nolisten unix` and addressed as
`127.0.0.1:77` deliberately: a sandboxed session may not be able to write
`/tmp/.X11-unix`, and then the unix socket never appears while the TCP port
does. `QT_QPA_PLATFORM=xcb` and dropping `WAYLAND_DISPLAY` are needed for the
same reason - RPCS3 is Qt, and on a Wayland session it will not look at an X
display unless told to.

**Always run `stop` at the end of a session.** `boot`/`race`/`shot`/`capture`/
`browse`/`record` each stop their own RPCS3 process *on a normal exit*
(`Session.__exit__`) - but that only runs if the driving script gets there. A
run that is killed, crashes, or loses its terminal detaches RPCS3 instead
(`start_new_session=True`), and it keeps running with nothing attached to
it - measured directly as an emulator left up for thirty minutes. `stop`
covers that: it stops RPCS3 if one is up, unconditionally, since RPCS3 does
not support a second instance at all so there is no "someone else's" RPCS3 to
avoid the way there can be someone else's display. The display itself is a
separate gap: it is deliberately long-lived across many of the calls above,
so nothing tears it down between them either - measured directly as an
orphaned two-day-old Xvfb with no client attached. Unlike the emulator, `stop`
only ever stops a display this script itself started (tracked in
`~/.cache/oag-rpcs3-drive/xvfb.owner.json`); one that was already running
when `display` ran is left alone. Never `pkill -x Xvfb` by hand - it would
reach every virtual display on the machine, not just this one's.

Subcommands:

    display   start (or check) the virtual display everything else needs.
    stop      stop RPCS3 (if up) and that display (if this tooling started it).
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
import math
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import rpcs3_pad
import xvfb_display

def xdg_cache():
    """`$XDG_CACHE_HOME`, or `~/.cache` - where RPCS3 puts `TTY.log` and the log.

    A member running its own RPCS3 under a private `XDG_CACHE_HOME` gets its
    own `TTY.log`; reading `~/.cache` regardless would wait on the wrong file.
    """
    return os.environ.get("XDG_CACHE_HOME") or os.path.expanduser("~/.cache")


def xdg_config():
    return os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config")


TTY = os.path.join(xdg_cache(), "rpcs3", "TTY.log")
DISPLAY_NUMBER = int(os.environ.get("OAG_RPCS3_DISPLAY", "77"))
DISPLAY = "127.0.0.1:%d" % DISPLAY_NUMBER

#: Records the pid of the `Xvfb` *this tooling* started, so a later `stop` -
#: in a different process, possibly minutes on - can tell it apart from a
#: display that was already there. Not under `/tmp`: this script has no
#: cache dir of its own the way pcsx2-drive.py does, so it gets one. See
#: `xvfb_display` for why this exists at all.
DISPLAY_MARKER = os.path.join(xdg_cache(), "oag-rpcs3-drive", "xvfb.owner.json")
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
# colour, and too coarse to measure a 17x7-pixel widget corner - a measurement
# this ceiling was hit trying and could not get. Raise both this and `Resolution Scale`
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

#: RPCS3's own configuration, the one its GUI edits. Never written by this
#: script: everything below reads it and writes a *copy*.
STOCK_CONFIG = os.path.join(xdg_config(), "rpcs3", "config.yml")

#: Where the generated copy goes. Under `data/` because that is gitignored,
#: and beside the other tool state rather than under `/tmp` (a 32 GiB tmpfs
#: this project has wedged before).
SCRATCH_CONFIG = os.environ.get("OAG_RPCS3_SCRATCH_CONFIG") or str(
    Path(__file__).resolve().parent.parent
    / "data" / "tools" / "rpcs3-scratch-config.yml")

#: `OAG_RPCS3_GDB=127.0.0.1:2391` moves the GDB stub off the shared 2345 in the
#: generated copy, so two members' emulators never contend for the port.
GDB_SERVER = os.environ.get("OAG_RPCS3_GDB")

#: `Session(config=MUTED)` - the default - generates the copy below and passes
#: it as `--config`. `config=None` launches on the stock file untouched.
MUTED = "muted"

#: `RPCS3.log`, where the emulator dumps the configuration it actually booted
#: with (`Used configuration:`) - what `Session.config_report` reads back.
RPCS3_LOG = os.path.join(xdg_cache(), "rpcs3", "RPCS3.log")


def scratch_config(path=SCRATCH_CONFIG, interpreter=False, source=STOCK_CONFIG):
    """Write a full copy of `config.yml` with the audio muted, and return it.

    RPCS3 accepts `--config <path>`, and that path **replaces** the whole
    configuration rather than overlaying it - a three-line file would reset
    `Miscellaneous: GDB Server` and `Core: Assume External Debugger` to their
    defaults, and the failure would read as "breakpoints never fire". So this
    copies every line of the stock file and edits exactly two keys, each
    matched inside its own top-level section (`Video:` has a `Renderer:` too):

    - `Audio: Renderer` -> `"Null"`, always, **with the quotes**: a bare
      `Null` is YAML's null, RPCS3 drops it and boots `Cubeb` regardless -
      measured 2026-09-15, `Used configuration:` still said `Cubeb` with the
      unquoted form. The stock file quotes its own `"Null"`s (`Keyboard`,
      `Microphone Type`) for the same reason. A scripted run on the virtual
      display has no business on the user's speakers.
    - `Core: PPU Decoder` -> `Interpreter (static)` when `interpreter` is
      set. `Z0` breakpoints only fire under it; the stock `Recompiler (LLVM)`
      answers `OK` and never stops (rpcs3-debugger.md). Boot takes ~75 s
      instead of ~30 s under it, so it is opt-in.

    The copy is regenerated on every `Session.__enter__`, so an edit to the
    stock file is picked up by the next launch and nothing here goes stale.
    What actually came up is in `RPCS3.log`'s own `Used configuration:` dump;
    `Session.config_report()` pulls the two lines out of it.
    """
    edits = {("Audio", "Renderer"): "\"Null\""}
    if GDB_SERVER:
        edits[("Miscellaneous", "GDB Server")] = GDB_SERVER
    if interpreter:
        edits[("Core", "PPU Decoder")] = "Interpreter (static)"
    section = None
    out = []
    applied = set()
    with open(source) as handle:
        for line in handle:
            if line and not line[0].isspace() and line.rstrip().endswith(":"):
                section = line.rstrip()[:-1]
            key = line.strip().split(":", 1)[0] if line.startswith("  ") else None
            if (section, key) in edits and (section, key) not in applied:
                line = "  %s: %s\n" % (key, edits[(section, key)])
                applied.add((section, key))
            out.append(line)
    missing = set(edits) - applied
    if missing:
        raise SystemExit("%s: no line for %s - the copy would silently run "
                         "with the stock value" % (source, sorted(missing)))
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_text("".join(out))
    return path


def rpcs3_log_text():
    try:
        with open(RPCS3_LOG, errors="replace") as handle:
            return handle.read()
    except FileNotFoundError:
        return ""


def config_report(keys=("Audio: Renderer", "Core: PPU Decoder")):
    """The settings RPCS3 says it booted with, off its own log.

    `RPCS3.log` opens with a `Used configuration:` dump of the whole
    effective config, so what came up is read back from the emulator rather
    than assumed from what was passed. Returns `{key: value}` for each
    `Section: Key` asked for; a key that never appeared maps to `None`, and
    that is the answer to trust over any launch line.
    """
    found = {}
    section = None
    for line in rpcs3_log_text().splitlines():
        if "Used configuration:" in line:
            section = None
            continue
        if line and not line[0].isspace() and line.rstrip().endswith(":"):
            section = line.rstrip()[:-1]
            continue
        if section and line.startswith("  ") and ":" in line:
            key, _, value = line.strip().partition(":")
            full = "%s: %s" % (section, key)
            if full in keys and full not in found:
                found[full] = value.strip()
    return {key: found.get(key) for key in keys}



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
    return xvfb_display.display_running(DISPLAY_NUMBER)


def start_display():
    """Bring up Xvfb :77. Returns True if this call started it.

    Ownership is recorded in `DISPLAY_MARKER` so `stop` - a later, separate
    invocation - can tear this down without ever touching a display it did
    not start. See `xvfb_display`.
    """
    return xvfb_display.bring_up(DISPLAY_NUMBER, DISPLAY_GEOMETRY,
                                 DISPLAY_MARKER)


def stop_display():
    """Tear down Xvfb :77, but only if this tooling started it.

    Idempotent: no-op and no error when the display was never ours, is
    already down, or `stop` runs a second time.
    """
    return xvfb_display.tear_down(DISPLAY_NUMBER, DISPLAY_MARKER)


def recordings(title_id="BCES00664"):
    """Every recording RPCS3 has written for a title, oldest first.

    The path is `recordings/<TITLE_ID>/*.mp4` - a *subdirectory* per title,
    which is worth stating because globbing `recordings/*` finds nothing and
    reads as "recording silently did not work".
    """
    root = os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config")
    pattern = os.path.join(root, "rpcs3", "recordings", title_id, "*.mp4")
    return sorted(glob.glob(pattern), key=os.path.getmtime)


#: The binary name every helper below matches on. A parameter rather than a
#: literal in each call so a test can point `emulator_running`/`stop_emulator`
#: at a harmless fake name instead of the real `rpcs3` - see
#: `scripts/pcsx2-drive.py`'s identical `stop_emulator`, which this mirrors.
EMULATOR_BINARY = "rpcs3"

#: What the emulator's process is actually *called*, which is not always the
#: binary it was launched as. The AppImage layout `rpcs3-bin` installs to
#: `/opt/rpcs3` runs `AppRun` (a shell script) that `exec`s
#: `AppRun.wrapped -> usr/bin/rpcs3`, so the surviving process's name is
#: `AppRun.wrapped` and `pgrep -x rpcs3` finds nothing. Found 2026-09-11 the
#: expensive way: `stop` reported only Xvfb stopped, the orphaned emulator
#: kept the GDB port bound, and every later `capture` timed out on
#: `qSupported` - trap 2 in `rpcs3_debugger.py`, reached without any client
#: ever having disconnected.
EMULATOR_PROCESS_NAMES = (EMULATOR_BINARY, "AppRun.wrapped")


def emulator_running(binary=EMULATOR_BINARY):
    names = EMULATOR_PROCESS_NAMES if binary == EMULATOR_BINARY else (binary,)
    return any(subprocess.run(["pgrep", "-x", name],
                              capture_output=True).returncode == 0
               for name in names)


def _kill_emulator(binary, signal_flag=None):
    names = EMULATOR_PROCESS_NAMES if binary == EMULATOR_BINARY else (binary,)
    for name in names:
        args = ["pkill"] + ([signal_flag] if signal_flag else []) + ["-x", name]
        subprocess.run(args, capture_output=True)


def stop_emulator(binary=EMULATOR_BINARY, quiet=False):
    """Stop a running RPCS3, if one is up.

    `Session.__exit__` already terminates the process it started on a normal
    exit - `boot`/`race`/`shot`/`capture`/`browse`/`record` all clean up after
    themselves that way. It runs under `start_new_session=True`, though, so a
    driving script that dies uncleanly (killed, crashed, the terminal closed)
    detaches RPCS3 rather than taking it down with it - measured directly as
    an emulator left running for thirty minutes with nothing attached to it.
    This is `stop`'s half of covering that case; RPCS3 does not support a
    second instance at all (see `clear_stale_lock` below), so unlike Xvfb
    there is no "someone else's" RPCS3 to avoid - matching pid, matching
    RPCS3 was never possible to begin with.
    """
    if not emulator_running(binary):
        if not quiet:
            print("no %s running" % binary)
        return False
    _kill_emulator(binary)
    for _ in range(20):
        time.sleep(0.5)
        if not emulator_running(binary):
            if not quiet:
                print("%s stopped" % binary)
            return True
    _kill_emulator(binary, "-9")
    return True


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
    if emulator_running():
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

    def __init__(self, image, log_dir, config=MUTED, interpreter=False):
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
        # `MUTED` (the default) generates `scratch_config()` on enter; a path
        # is passed through as-is; `None` launches on the stock file, sound
        # and all - the one caller that wants that has to say so.
        self.config = config
        self.interpreter = interpreter
        self.pad = None
        self.proc = None

    def __enter__(self):
        clear_stale_lock()
        start_display()
        # The pad has to exist before RPCS3 does: it binds pads when it
        # enumerates devices and does not rescan.
        self.pad = rpcs3_pad.Pad()
        open(TTY, "w").close()
        if self.config == MUTED:
            self.config = scratch_config(interpreter=self.interpreter)
        config_args = ["--config", str(self.config)] if self.config else []
        self.proc = subprocess.Popen(
            ["rpcs3", "--no-gui", "--input-config", rpcs3_pad.INPUT_CONFIG_NAME]
            + config_args + [str(self.image)],
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


def cmd_stop(args):
    """Stop RPCS3 if it is up, and Xvfb :77 too - but only what this run owns.

    `boot`/`race`/`shot`/`capture`/`browse`/`record` already stop their own
    RPCS3 process *on a normal exit* (`Session.__exit__`) - but that only
    runs if the driving script gets to it; a killed or crashed run leaves
    RPCS3 detached and running, which is what `stop_emulator` here is for.
    The display is a separate gap: it is deliberately meant to outlive any
    one of those calls, so nothing tears it down between them either. Run
    this when the whole session - not just one race - is done. Safe to run
    twice, or with nothing up at all.

    Order matters: stop RPCS3 first, then clear its stale lock -
    `clear_stale_lock` is itself guarded on there being no live process, so
    running it before the kill would just skip.
    """
    stop_emulator()
    clear_stale_lock()
    if stop_display():
        print("Xvfb :%d stopped" % DISPLAY_NUMBER)
    elif display_running():
        print("Xvfb :%d left running (not started by this tooling)"
              % DISPLAY_NUMBER)
    else:
        print("no Xvfb :%d running" % DISPLAY_NUMBER)
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
    with open_session(args) as session:
        print("rpcs3 pid %d, waiting for the Main Menu" % session.proc.pid,
              flush=True)
        if not session.wait_for_screen_pressing("Main Menu", args.timeout):
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
    with open_session(args) as session:
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
    with open_session(args) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen_pressing("Main Menu", args.timeout):
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
# 917,504 bytes, about a minute at the stub's 41 ms a packet. Measured to hold
# nothing but the seven static cube-face/shadow matrices
# (`docs/reverse-engineering/rpcs3-capture.md`, "`viewProj` is not in the
# executable's data") - no longer `capture`'s default region, kept as a named
# constant for a manual `--region` against it.
DATA_SEGMENT = (0x00860000, 0x000E0000)

# The RSX pushbuffer the draw commands actually live in - `viewProj`'s own
# home, per `rpcs3-capture.md`'s "`viewProj` is in the pushbuffer, and here is
# where". `capture`'s default `--region` set: the four auxiliary contexts and
# the two IO targets they jump to.
PUSHBUFFER_REGIONS = (
    ("0x40010000", 0x4000),
    ("0x40060000", 0x20000),
    ("0x40080000", 0x20000),
)


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
    regions = [parse_region(r) for r in args.region] or list(PUSHBUFFER_REGIONS)

    with open_session(args) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen_pressing("Main Menu", args.timeout):
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

        # Camera picking is cross-frame (see `ps3_pose.pick_camera`), so every
        # shot's candidates have to be in hand before any of them can be
        # picked - nothing is written to disk inside this loop.
        shots = []
        with Debugger(port=int(GDB_SERVER.rsplit(":", 1)[1]) if GDB_SERVER
                      else Debugger.__init__.__defaults__[0]) as gdb:
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

                candidates = []
                for at, blob in blobs:
                    candidates.extend(ps3_pose.packet_candidates(blob, base=at))
                print("  %s: %d packet candidate(s) read" % (stem, len(candidates)),
                      flush=True)
                shots.append((stem, shot, candidates))
                if args.keep_dumps:
                    for at, blob in blobs:
                        (out / ("%s-%08x.bin" % (stem, at))).write_bytes(blob)
            gdb.resume()

    picks = ps3_pose.pick_camera([candidates for _, _, candidates in shots])
    for (stem, shot, _), (camera, reason, count) in zip(shots, picks):
        record = describe(camera, reason, count, track, shot,
                           args.team, args.hull_variant)
        (out / ("%s.json" % stem)).write_text(json.dumps(record, indent=2) + "\n")
        if camera:
            print("  %s: eye %s regs %s unit error %.2e" % (
                stem, ["%.1f" % v for v in camera["eye"]],
                camera["registers"], camera["unit_error"]), flush=True)
            render_with = record["camera"].get("render_with")
            if render_with:
                print("       " + render_with, flush=True)
        else:
            print("  %s: camera null (%s)" % (stem, reason), flush=True)

    print("done; %d pose(s) in %s" % (args.shots, out), flush=True)
    return 0


def cmd_place(args):
    """One boot, many placements: teleport the craft, settle, photograph.

    Each `--pose x,y,z[,yaw]` writes the player's rigid body while the target
    is paused, resumes for `--settle` seconds, then pauses and records the
    pose the game kept next to the screenshot. See `rpcs3_place`.
    """
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import ps3_pose
    import rpcs3_place
    from rpcs3_debugger import Debugger

    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    oag_game = Path(args.oag_game).resolve()
    attitudes = []
    for text in args.pose:
        pose = rpcs3_place.parse_pose(text)
        forward, up = rpcs3_place.our_attitude(
            oag_game, Path(args.image).resolve(), args.track, pose, out)
        attitudes.append((text, pose, rpcs3_place.basis_rows(forward, up)))

    with open_session(args) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen_pressing("Main Menu", args.timeout):
            print("never reached the Main Menu (last screen: %s)"
                  % current_screen(), file=sys.stderr)
            return 1
        time.sleep(args.settle_menu)
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
        print("track: %s" % (track_name() or "<not logged>"), flush=True)
        # The race opens on a fly-over with a `START RACE` prompt, and the
        # craft is pinned to its grid slot until the countdown ends: a write
        # before then is overwritten every tick.
        session.tap("cross", settle=args.countdown)

        with Debugger(port=int(GDB_SERVER.rsplit(":", 1)[1]) if GDB_SERVER
                      else Debugger.__init__.__defaults__[0]) as gdb:
            gdb.pause()
            ship, body = rpcs3_place.find_player(gdb)
            print("player ship %#x body %#x" % (ship, body), flush=True)
            gdb.resume()
            for n, (text, pose, rows) in enumerate(attitudes):
                stem = "%02d" % n
                gdb.pause()
                before = rpcs3_place.read_pose(gdb, body)
                rpcs3_place.write_pose(gdb, body, pose[:3], rows, args.speed)
                gdb.resume()
                time.sleep(args.settle)
                gdb.pause()
                kept = rpcs3_place.read_pose(gdb, body)
                shot = screenshot(out / ("%s.png" % stem), trim=True)
                for attempt in range(args.dump_attempts):
                    for text_region in args.dump:
                        chain, size = parse_region(text_region)
                        at = resolve_chain(gdb, chain)
                        if at:
                            try:
                                blob = gdb.read(at, size)
                            except Exception as error:
                                print("  %s: dump %#x+%#x failed: %s"
                                      % (stem, at, size, error), flush=True)
                                continue
                            (out / ("%s-%08x.bin" % (stem, at))
                             ).write_bytes(blob)
                            print("  %s: dumped %#x+%#x" % (stem, at, size),
                                  flush=True)
                    if not args.hook:
                        break
                    import importlib.util
                    spec = importlib.util.spec_from_file_location(
                        "place_hook", args.hook)
                    hook = importlib.util.module_from_spec(spec)
                    spec.loader.exec_module(hook)
                    retry = False
                    for round_no in range(24):
                        extra = hook.regions(out, stem, round_no)
                        if extra is None:
                            retry = True
                            break
                        if not extra:
                            break
                        for addr, size in extra:
                            try:
                                blob = gdb.read(addr, size)
                            except Exception as error:
                                print("  %s: hook dump %#x+%#x failed: %s"
                                      % (stem, addr, size, error), flush=True)
                                continue
                            (out / ("%s-%08x.bin" % (stem, addr))
                             ).write_bytes(blob)
                        print("  %s: hook round %d dumped %d span(s)"
                              % (stem, round_no, len(extra)), flush=True)
                    if not retry:
                        break
                    print("  %s: hook asked for a retry (attempt %d)"
                          % (stem, attempt), flush=True)
                    for old in out.glob("%s-*.bin" % stem):
                        old.unlink()
                    gdb.resume()
                    time.sleep(args.dump_gap)
                    gdb.pause()
                candidate_sets = []
                for k in range(args.camera_shots):
                    if k:
                        gdb.resume()
                        time.sleep(args.camera_gap)
                        gdb.pause()
                    blobs = []
                    for chain, size in PUSHBUFFER_REGIONS:
                        at = resolve_chain(gdb, chain)
                        if at:
                            blobs.append((at, gdb.read(at, size)))
                    candidate_sets.append([
                        c for at, blob in blobs
                        for c in ps3_pose.packet_candidates(blob, base=at)])
                    print("  %s: camera read %d, %d candidate(s)"
                          % (stem, k, len(candidate_sets[-1])), flush=True)
                gdb.resume()
                time.sleep(args.recheck)
                gdb.pause()
                later = rpcs3_place.read_pose(gdb, body)
                gdb.resume()
                drift = math.dist(kept["pos"], pose[:3])
                moved = math.dist(later["pos"], kept["pos"])
                print("  %s: asked %s, kept %s (drift %.1f), %.1f s later "
                      "moved %.1f"
                      % (stem, text, ["%.1f" % v for v in kept["pos"]], drift,
                         args.recheck, moved), flush=True)
                kept_text = ",".join(
                    ["%.2f" % v for v in kept["pos"]]
                    + ["%g" % v for v in pose[3:]])
                render = ("target/release/oag-game <image> --race --track %s "
                          "--team %s --variant %s --pose=%s --ticks 30 "
                          "--screenshot ours.png"
                          % (args.track, args.team, args.variant, kept_text))
                camera = None
                if candidate_sets:
                    pick = ps3_pose.pick_camera(candidate_sets)[-1]
                    camera, reason, _ = pick
                    if camera and ps3_pose.decompose(camera["view_proj"]):
                        found = ps3_pose.decompose(camera["view_proj"])
                        camera = dict(camera, **found)
                        camera["render_with"] = ps3_pose.command_line(
                            found, args.track)
                    print("  %s: camera %s (%s)"
                          % (stem, "found" if camera else "null", reason),
                          flush=True)
                record = {
                    "track": track_name(), "team": args.team,
                    "hull_variant": args.variant, "asked": text,
                    "start_pose": before["pos"], "settled": kept,
                    "later": later, "settle_s": args.settle,
                    "drift": drift, "moved_after_recheck": moved,
                    "screenshot": str(shot) if shot else None,
                    "render_with": render, "camera": camera,
                }
                (out / ("%s.json" % stem)).write_text(
                    json.dumps(record, indent=2) + "\n")
    print("done; %d placement(s) in %s" % (len(attitudes), out), flush=True)
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

    with open_session(args) as session:
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


def describe(camera, reason, candidate_count, track, shot, team=None, hull_variant=None):
    """The capture record: what a frame needs to be reproduced, and nothing more.

    `camera`, `reason` and `candidate_count` are one element of
    `ps3_pose.pick_camera`'s own return - this does not re-decide anything,
    it only turns the pick into the record. `camera["camera"]` is JSON `null`
    when the cross-frame discriminator could not pick exactly one candidate -
    `"camera_reason"` says why and `"camera_candidates"` says how many
    survived every filter but the last, rather than silently reporting
    nothing or, worse, a plausible-looking wrong pose (the same honest-absence
    rule this project applies to a missing asset).

    `team`/`hull_variant` are never detected - no `TTY.log` line names either
    on the walk into a race, checked directly against both the Campaign
    default walk and Racebox's - so they are whatever the caller passed
    `--team`/`--hull-variant`, `None` when not supplied. Recorded beside
    `track` regardless, so a render command built from this file races the
    same craft the screenshot shows rather than whatever `--team` defaults to.

    Deliberately not a memory dump. The raw bytes are game memory - extracted
    executable data, which `just audit-leakage` refuses and CI's leakage job
    would fail on - so they stay under `data/` and out of this file, which
    carries only the numbers a renderer is set up from.
    """
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import ps3_pose

    record_camera = None
    if camera:
        pose = ps3_pose.decompose(camera["view_proj"])
        record_camera = dict(camera)
        if pose:
            record_camera.update(pose)
            record_camera["render_with"] = ps3_pose.command_line(pose, track)
    return {
        "track": track,
        "team": team,
        "hull_variant": hull_variant,
        "screenshot": str(shot) if shot else None,
        "camera": record_camera,
        "camera_reason": reason,
        "camera_candidates": candidate_count,
    }


def screen_slug(name):
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-") or "unnamed"


def cmd_bootchain(args):
    """Watch the boot chain screen by screen and photograph each step.

    This is what upgraded `oag_hd::frontend::BOOT` from
    `oag_title::Provenance::Declared` to `Measured` on 2026-09-05, and it is
    here rather than in a scratch file so the measurement is repeatable from
    the repository. See docs/formats/hd-frontend.md.

    **Move the savedata aside first or the chain you watch is the wrong one.**
    With `~/.config/rpcs3/dev_hdd0/home/00000001/savedata/BCES00664-AUTO-`
    present, HD skips `FirstPlay` entirely and goes `EpilepsyWarning ->
    Save Warning` - a *shorter* chain than the XML declares, and one that looks
    like a complete boot unless you know the eighth step is missing. Put it back
    afterwards.

    No GDB connection: everything here is `TTY.log` and the X root window, and
    connecting would pause the emulation this exists to watch.

    `--film` is not redundant with the per-screen shots. RPCS3 flushes
    `TTY.log` in **bursts**, so five transitions can arrive bearing one
    timestamp, and a shot keyed on a name change cannot catch a screen that
    auto-redirected inside a burst. A fixed-interval grab can.
    """
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    log, presses = [], []
    current = "?"
    with open_session(args, out / "emu") as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        t0 = time.time()
        seen = 0
        last_change = time.time()
        shot_index = 0
        film_dir = out / "film"
        film_index = 0
        next_film = time.time()
        if args.film:
            film_dir.mkdir(exist_ok=True)
        while time.time() - t0 < args.timeout:
            if args.film and time.time() >= next_film:
                screenshot(film_dir / ("f%03d-%06.2f.png"
                                       % (film_index, time.time() - t0)),
                           trim=True)
                film_index += 1
                next_film = time.time() + args.film
            if session.proc.poll() is not None:
                print("RPCS3 exited early", file=sys.stderr)
                break
            hits = SCREEN_LINE.findall(tty_text())
            if len(hits) > seen:
                for frm, to in hits[seen:]:
                    stamp = round(time.time() - t0, 2)
                    log.append({"t": stamp, "from": frm, "to": to})
                    print("  %7.2f  %-24s -> %-24s" % (stamp, frm or '""', to),
                          flush=True)
                seen = len(hits)
                current = hits[-1][1]
                last_change = time.time()
                time.sleep(args.settle)
                # Re-read: the settle may have carried us onward already, and a
                # shot taken then would be labelled with the screen before it.
                if len(SCREEN_LINE.findall(tty_text())) == seen:
                    name = "%02d-%s.png" % (shot_index, screen_slug(current))
                    screenshot(out / name, trim=True)
                    log[-1]["shot"] = name
                    shot_index += 1
                continue
            if current == args.stop:
                break
            if time.time() - last_change > args.press_after:
                was, now = session.tap("cross", settle=2.0)
                presses.append({"t": round(time.time() - t0, 2),
                                "at": was, "after": now})
                print("  %7.2f  press cross at %s"
                      % (time.time() - t0, was), flush=True)
                last_change = time.time() - args.press_after + 4.0
            time.sleep(0.5)
        print("arrived: %s, holding %g s" % (current, args.hold), flush=True)
        time.sleep(args.hold)
        screenshot(out / ("%02d-%s-held.png"
                          % (shot_index, screen_slug(current))), trim=True)
        shutil.copy(TTY, out / "TTY.log")
    (out / "chain.json").write_text(json.dumps(
        {"transitions": log, "presses": presses, "arrived": current}, indent=2))
    print("wrote %s" % (out / "chain.json"))
    return 0 if current == args.stop else 1


def cmd_record(args):
    before = set(recordings())
    with open_session(args) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen_pressing("Main Menu", args.timeout):
            print("never reached the Main Menu (last screen: %s)"
                  % current_screen(), file=sys.stderr)
            return 1
        time.sleep(args.settle)
        print("walking the menus:", flush=True)
        if session.walk_to_race() not in RACE_ARRIVED:
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


def cmd_countdown(args):
    """Record the whole load and countdown, thrust held from the HUD on.

    `record` starts its capture after a fixed wait, which is after the
    countdown has run; this starts it on `Team Selection`, one press before the
    race loads, so the gantry's `3 2 1 GO` and the race clock's first tick are
    both in the video. The recording is the observable (30 fps, every frame a new
    image on the countdown: measured); `scripts/hd-countdown-frames.py` reads it.
    """
    before = set(recordings())
    with open_session(args) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen_pressing("Main Menu", args.timeout):
            print("never reached the Main Menu", file=sys.stderr)
            return 1
        time.sleep(args.settle)
        recording = False
        for index in range(1, 16):
            screen = current_screen()
            if screen in RACE_ARRIVED:
                break
            if screen == "Team Selection" and not recording:
                time.sleep(SCREEN_SETTLE_BEFORE_RECORD)
                session.toggle_recording()
                recording = True
                print("recording started at %s" % screen, flush=True)
            was, now = session.press_once("cross")
            print("  press %2d  %-22s -> %s" % (index, was, now), flush=True)
        if not recording:
            print("never saw Team Selection; no recording", file=sys.stderr)
            return 1
        started = time.time()
        held = False
        while time.time() - started < args.load + args.drive:
            if not held and time.time() - started > args.load:
                # The race opens on a fly-over of the track with a START RACE
                # prompt; a tap skips it and the countdown follows.
                session.pad.press("cross", 0.15)
                time.sleep(args.skip_gap)
                session.pad.set("cross", True)
                held = True
                print("thrust held at +%.1f s (%s)"
                      % (time.time() - started, current_screen()), flush=True)
            time.sleep(0.2)
        session.pad.set("cross", False)
        session.toggle_recording()
        time.sleep(8.0)
    fresh = [path for path in recordings() if path not in before]
    for path in fresh:
        print("%s  %.1f MiB" % (path, os.path.getsize(path) / 1048576.0))
    return 0 if fresh else 1


SCREEN_SETTLE_BEFORE_RECORD = 2.0

#: `RaceManager_GetInstance` (`0x00054628`) is `lwz r9,-0x6cc4(r2); lwz r3,0(r9)`
#: under TOC `0x008ad4d8`: the instance pointer lives at this address.
RACE_MANAGER_SLOT = 0x0095AE78

#: The game state `RaceManager_Update` reads through TOC-0x6ce0 - a static
#: object, not a pointer; `+0xc` is the race's lap count.
GAME_STATE = 0x00936FE8

#: The TOC floats holding the lap-0 window, `[3.83, 5.25)`.
PRE_LAP_FROM = 0x008A6A74
PRE_LAP_TO = 0x008A6A90


def _word(gdb, address):
    return int.from_bytes(gdb.read(address, 4), "big")


def _board_state(gdb):
    """`(manager, ship, lap, crossings, total, phase, gantry_time)` right now."""
    import struct
    manager = _word(gdb, RACE_MANAGER_SLOT)
    ship = _word(gdb, manager + 0x13E8)
    total = _word(gdb, GAME_STATE + 0xC)
    phase = _word(gdb, manager + 0x1970)
    lap = _word(gdb, ship + 0x7810)
    crossings = _word(gdb, ship + 0x7814)
    # The gantry billboard's `.vex` node, `+0x40` of the slot the manager
    # keeps at `+0x1950`; its own `+0xc0` is the time when it is the
    # MeshImporter itself (`AnimNode_GetTime`, 0x002c0d78).
    node = _word(gdb, _word(gdb, manager + 0x1950) + 0x40)
    time_bits = gdb.read(node + 0xC0, 4)
    return (manager, ship, lap, crossings, total, phase,
            struct.unpack(">f", time_bits)[0])


def cmd_lapboard(args):
    """Park on the grid, force the player's lap counter, photograph the gantry.

    The gantry's lap windows (`RaceManager_Update`, 0x0005e948) key on
    `ship+0x7810`. Driving laps to reach them is slow and the board is only in
    view from the grid, so this releases the start without thrust, then writes
    each lap value through the GDB stub and screenshots the board it selects.
    """
    import struct
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from rpcs3_debugger import Debugger

    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    port = int(GDB_SERVER.rpartition(":")[2]) if GDB_SERVER else 2345
    log = open(out / "lapboard.txt", "w")

    def note(text):
        print(text, flush=True)
        log.write(text + "\n")
        log.flush()

    with open_session(args) as session:
        note("rpcs3 pid %d" % session.proc.pid)
        if not session.wait_for_screen_pressing("Main Menu", args.timeout):
            note("never reached the Main Menu")
            return 1
        time.sleep(args.settle)
        for index in range(1, 16):
            if current_screen() in RACE_ARRIVED:
                break
            was, now = session.press_once("cross")
            note("  press %2d  %-22s -> %s" % (index, was, now))
        time.sleep(args.load)
        session.pad.press("cross", 0.15)
        note("intro skipped; no thrust; waiting %g s past the release" % args.wait)
        time.sleep(args.wait)
        screenshot(out / "00-pre.png", trim=True)
        with Debugger(port=port) as gdb:
            gdb.pause()
            state = _board_state(gdb)
            note("manager %#x ship %#x lap %d crossings %d total %d phase %d "
                 "gantry %.3f s" % state)
            ship, total = state[1], state[4]
            gdb.resume()
            for spec in args.window:
                # The lap-0 window's bounds are TOC floats `RaceManager_Update`
                # loads every frame (`lfs f31,-0x6a64(r2)`, `lfs f0,-0x6a48(r2)`);
                # writing another window's bounds there makes the original play
                # that window on the grid, whatever the lap counter says.
                low, high = (float(v) for v in spec.split(","))
                gdb.pause()
                gdb.write(PRE_LAP_FROM, struct.pack(">f", low))
                gdb.write(PRE_LAP_TO, struct.pack(">f", high))
                note("window [%g, %g)" % (low, high))
                gdb.resume()
                for shot in range(args.shots):
                    time.sleep(args.interval)
                    path = out / ("w%g-%02d.png" % (low, shot))
                    screenshot(path, trim=True)
            for lap in [int(v) for v in args.laps.split(",") if v]:
                lap = {-1: total - 1, -2: total}.get(lap, lap)
                gdb.pause()
                gdb.write(ship + 0x7810, lap.to_bytes(4, "big"))
                if args.crossings:
                    # The lap is rewritten every frame from elsewhere; the
                    # crossing count at `+0x7814` starts at 2 on the grid.
                    gdb.write(ship + 0x7814, (lap + 2).to_bytes(4, "big"))
                note("wrote lap %d%s" % (lap, " and crossings %d" % (lap + 2)
                                          if args.crossings else ""))
                gdb.resume()
                for shot in range(args.shots):
                    time.sleep(args.interval)
                    path = out / ("lap%d-%02d.png" % (lap, shot))
                    screenshot(path, trim=True)
                    gdb.pause()
                    note("  %s: lap %d crossings %d total %d phase %d gantry %.3f s"
                         % ((path.name,) + _board_state(gdb)[2:]))
                    gdb.resume()
    return 0



def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--image",
                        default="data/images/hdfury-ps3-eu-dec.iso",
                        help="the layer-1 DECRYPTED disc image")
    parser.add_argument("--log-dir", default="/tmp/rpcs3-drive",
                        help="where the emulator log and screenshots go")
    parser.add_argument("--config", default=MUTED, metavar="PATH",
                        help="a config.yml to pass RPCS3 as --config. The "
                             "default writes a full copy of the stock file "
                             "with `Audio: Renderer: Null` to %s and uses "
                             "that, so a scripted run is never audible"
                             % SCRATCH_CONFIG)
    parser.add_argument("--stock-config", action="store_true",
                        help="launch on ~/.config/rpcs3/config.yml itself, "
                             "sound included; --config is ignored")
    parser.add_argument("--interpreter", action="store_true",
                        help="also set `Core: PPU Decoder: Interpreter "
                             "(static)` in the generated copy - the only "
                             "decoder Z0 breakpoints fire under; ~75 s to "
                             "boot instead of ~30 s")
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("display").set_defaults(run=cmd_display)
    sub.add_parser("stop").set_defaults(run=cmd_stop)
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
                          "the RSX pushbuffer (PUSHBUFFER_REGIONS) - the "
                          "EBOOT's data and BSS (DATA_SEGMENT) holds only "
                          "static cube-face/shadow matrices, never the live "
                          "camera.")
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
    cap.add_argument("--team", default=None,
                     help="the team this capture's craft is confirmed to be, "
                          "e.g. 'feisar' - recorded beside track, never "
                          "detected: no TTY.log line names the team on the "
                          "walk into a race (checked directly, both the "
                          "Campaign default walk and Racebox's), so this has "
                          "to come from the caller having read it off a "
                          "Team Selection screenshot (--nav-shots) or a "
                          "live memory read.")
    cap.add_argument("--hull-variant", default=None,
                     help="the ship model variant this capture's craft is "
                          "confirmed to be, e.g. 'concept1' - same caveat as "
                          "--team: not detected, supplied by the caller.")
    cap.add_argument("--keep-dumps", action="store_true",
                     help="also write the raw memory, which is game data and "
                          "stays under data/")

    place = sub.add_parser("place",
                           help="teleport the player's craft to a world "
                                "position and photograph it settled")
    place.add_argument("--pose", action="append", required=True,
                       metavar="X,Y,Z[,YAW]",
                       help="a world position as oag-game's --pose reads it; "
                            "repeatable, one boot for all of them")
    place.add_argument("--track", default=r"Data\Environments\Talons_Junction\track.vex",
                       help="oag-game's name for the circuit being raced, used "
                            "to take the attitude from `oag-game --pose`")
    place.add_argument("--out", required=True)
    place.add_argument("--nav", action="append", default=[],
                       help="SCREEN=BUTTONS, as in `capture`")
    place.add_argument("--oag-game", default="target/release/oag-game")
    place.add_argument("--team", default="feisar_c1",
                       help="recorded beside the shot and used in the render "
                            "command; the default walks race the feisar_c1 "
                            "concept1 hull, see `Racebox races the same hull`")
    place.add_argument("--variant", default="concept1")
    place.add_argument("--camera-shots", type=int, default=0,
                       help="read the RSX pushbuffer this many times per pose "
                            "(~20 s each) and pick the camera across them")
    place.add_argument("--camera-gap", type=float, default=1.0)
    place.add_argument("--dump-attempts", type=int, default=1)
    place.add_argument("--dump-gap", type=float, default=0.7)
    place.add_argument("--hook", default=None, metavar="PY",
                       help="a module with regions(out, stem, round) -> "
                            "[(addr, size)], called after the --dump files "
                            "are written while the target is still paused; "
                            "its spans are dumped, and it is called again "
                            "until it returns nothing")
    place.add_argument("--dump", action="append", default=[],
                       metavar="CHAIN:LEN",
                       help="write guest memory (same chain syntax as "
                            "capture --region) beside each shot as "
                            "NN-<addr>.bin, read while the target is paused")
    place.add_argument("--speed", type=float, default=0.0,
                       help="velocity along the new forward")
    place.add_argument("--settle", type=float, default=8.0,
                       help="seconds of emulation between the write and the shot")
    place.add_argument("--recheck", type=float, default=3.0,
                       help="seconds after the shot before the second pose read")
    place.add_argument("--settle-menu", type=float, default=20.0)
    place.add_argument("--load", type=float, default=70.0)
    place.add_argument("--countdown", type=float, default=25.0,
                       help="seconds after the START RACE tap before the "
                            "first write, so the grid is released")
    place.add_argument("--timeout", type=float, default=240.0)
    place.set_defaults(run=cmd_place)

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

    chain = sub.add_parser("bootchain",
                           help="watch the boot chain screen by screen and "
                                "photograph each step")
    chain.set_defaults(run=cmd_bootchain)
    chain.add_argument("--out", default="data/reference/hd-boot-chain/run",
                       help="where the shots, TTY.log and chain.json go")
    chain.add_argument("--stop", default="Main Menu",
                       help="the screen the chain ends at")
    chain.add_argument("--timeout", type=float, default=420.0)
    chain.add_argument("--settle", type=float, default=3.0,
                       help="pause after a screen change before its shot")
    chain.add_argument("--press-after", type=float, default=8.0,
                       help="seconds a screen may sit still before cross is "
                            "sent; an auto-redirect is never hurried and a "
                            "dialog is never left waiting")
    chain.add_argument("--hold", type=float, default=15.0,
                       help="how long to sit on --stop before quitting")
    chain.add_argument("--film", type=float, default=1.5,
                       help="also grab a frame every N seconds regardless of "
                            "TTY.log; 0 disables")

    rec = sub.add_parser("record")
    rec.add_argument("--timeout", type=float, default=180.0)
    rec.add_argument("--settle", type=float, default=12.0)
    rec.add_argument("--load", type=float, default=50.0)
    rec.add_argument("--drive", type=float, default=30.0,
                     help="seconds of held thrust to capture")
    rec.set_defaults(run=cmd_record)

    cd = sub.add_parser("countdown",
                        help="record the load and the start countdown")
    cd.add_argument("--timeout", type=float, default=180.0)
    cd.add_argument("--settle", type=float, default=12.0)
    cd.add_argument("--load", type=float, default=25.0,
                    help="seconds after the last press before the intro is "
                         "skipped with a tap and thrust is held")
    cd.add_argument("--drive", type=float, default=40.0,
                    help="seconds recorded after --load")
    cd.add_argument("--skip-gap", type=float, default=1.0,
                    help="seconds between the intro-skipping tap and holding "
                         "thrust")
    cd.set_defaults(run=cmd_countdown)

    lb = sub.add_parser("lapboard",
                        help="park on the grid, force the lap counter, "
                             "photograph the gantry's lap windows")
    lb.add_argument("--timeout", type=float, default=180.0)
    lb.add_argument("--settle", type=float, default=12.0)
    lb.add_argument("--load", type=float, default=25.0)
    lb.add_argument("--wait", type=float, default=15.0,
                    help="seconds after the intro skip before the first write")
    lb.add_argument("--laps", default="1,-1,-2",
                    help="lap values to write in order; -1 is total-1, -2 total")
    lb.add_argument("--shots", type=int, default=8)
    lb.add_argument("--interval", type=float, default=0.6)
    lb.add_argument("--out", default="data/reference/hd-lapboard")
    lb.add_argument("--window", action="append", default=[],
                    metavar="FROM,TO",
                    help="write these bounds over the lap-0 window and "
                         "photograph; repeatable, run before --laps")
    lb.add_argument("--crossings", action="store_true",
                    help="also write the crossing count at +0x7814 as lap + 2")
    lb.set_defaults(run=cmd_lapboard)

    args = parser.parse_args(argv)
    if args.stock_config:
        args.config = None
    elif args.interpreter and args.config != MUTED:
        parser.error("--interpreter edits the generated copy; with an "
                     "explicit --config, set `PPU Decoder` in that file")
    return args.run(args)


def open_session(args, log_dir=None):
    """A `Session` built from the top-level options, for every subcommand.

    Named distinctly from the `session` local every call site binds via
    `as session` - `with open_session(args) as session:` shadows the call target
    with its own result inside the same statement, so the second reference to
    `session` (the call) resolves to the not-yet-assigned local rather than
    this function, and every subcommand raised `UnboundLocalError` before
    ever launching RPCS3. Confirmed in isolation with a two-line repro; this
    function existing under the same name as its own `with` target is
    sufficient, no subprocess required.
    """
    return Session(args.image, log_dir if log_dir is not None else args.log_dir,
                   config=args.config, interpreter=args.interpreter)


if __name__ == "__main__":
    sys.exit(main())
