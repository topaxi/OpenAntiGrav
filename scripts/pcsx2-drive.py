#!/usr/bin/env python3
"""Drive PCSX2 with no window and no human: boot, press buttons, screenshot.

The PSP side of this project drives PPSSPP from a websocket debugger
(`docs/reverse-engineering/ppsspp-debugger.md`) and the PS3 side drives RPCS3
from a GDB stub plus a kernel-level virtual pad
(`docs/reverse-engineering/rpcs3-debugger.md`). This is the PS2 equivalent, and
it is a third transport again:

    Xvfb :78 ── PCSX2 (OpenGL, -nogui -batch) ── PINE socket   (memory, savestates)
                          ▲
                          └── XTEST synthetic key events        (buttons, hotkeys)

Why each piece, in the order the alternatives were eliminated - the long form
is in `docs/reverse-engineering/pcsx2-debugger.md`:

- **A virtual display, not headless.** PCSX2 has no `--headless`. Its `-nogui`
  hides the *main* window and still opens a render window, which is where the
  frames and the key events both go.
- **`Renderer = 12` (OpenGL), not the default `Auto`.** Auto picks Vulkan,
  Vulkan finds no present queue on Xvfb, and PCSX2 exits with
  "Failed to create GS device" before the VM ever starts.
- **XTEST, not `/dev/uinput`.** PCSX2's `Keyboard/*` bindings are Qt key events
  from the X server; Xvfb has no evdev driver, so the virtual pad that works
  for RPCS3 is invisible here. XTEST puts the events in at the X server, which
  is the one place both PCSX2 and Xvfb agree on.
- **Focus has to be set explicitly.** There is no window manager on the virtual
  display, so focus is `PointerRoot` and the pointer sits at screen centre,
  outside a 640x480 render window pinned at the top-left. Every key press is
  delivered to the root window and silently lost. `_focus_render_window` fixes
  it, and it is the single trap most likely to cost a run.

Commands:

    python3 scripts/pcsx2-drive.py display                  # start Xvfb :78
    python3 scripts/pcsx2-drive.py config                   # write the datapath ini
    python3 scripts/pcsx2-drive.py boot                     # boot the disc, wait for PINE
    python3 scripts/pcsx2-drive.py press cross cross        # abstract button names
    python3 scripts/pcsx2-drive.py shot /tmp/frame.png      # GS screenshot, exact output
    python3 scripts/pcsx2-drive.py input                    # read the game's own pad mask
    python3 scripts/pcsx2-drive.py stop                      # stop pcsx2-qt AND Xvfb :78

`display`, `config` and `boot` are all idempotent and `boot` runs the first two
for you, so the one-liner is:

    uv run --with python-xlib python3 scripts/pcsx2-drive.py boot --shot /tmp/f.png

Nothing this script writes goes anywhere near the repository: the emulator's
data path is `~/.cache/oag-pcsx2`, screenshots go where you name them, and the
disc image is read out of `data/`, which is gitignored.

**Always end a session with `stop`.** Xvfb :78 is deliberately long-lived
across `boot`/`press`/`shot`/`input` calls - nothing tears it down between
them - so `stop` is the one explicit step that closes it out. It only ever
stops a display this script itself started (tracked in
`~/.cache/oag-pcsx2/xvfb.owner.json`); a display already there when `display`
ran is left alone. Never `pkill -x Xvfb` by hand - it would reach every
virtual display on the machine, not just this one's.
"""

import argparse
import glob
import os
import shutil
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from pcsx2_pine import Pine, PineError, STATUS_NAMES  # noqa: E402
import xvfb_display  # noqa: E402

DISPLAY_NUMBER = 78
DISPLAY = "127.0.0.1:%d" % DISPLAY_NUMBER
DISPLAY_GEOMETRY = "1600x1200x24"

DATA_PATH = os.path.expanduser("~/.cache/oag-pcsx2")

#: Records the pid of the `Xvfb` *this tooling* started, so a later `stop` -
#: in a different process - can tell it apart from a display that was already
#: there. Lives in the same cache dir as everything else this script writes,
#: never under `/tmp`. See `xvfb_display` for why this exists.
DISPLAY_MARKER = os.path.join(DATA_PATH, "xvfb.owner.json")
INI_DIR = os.path.join(DATA_PATH, "PCSX2", "inis")
SNAP_DIR = os.path.join(DATA_PATH, "PCSX2", "snaps")
STATE_DIR = os.path.join(DATA_PATH, "PCSX2", "sstates")
LOG_FILE = "/tmp/oag-pcsx2.log"

DEFAULT_IMAGE = "data/images/pulse-ps2-eu.chd"
BIOS_DIR = os.path.expanduser("~/.config/PCSX2/bios")

#: Abstract button -> the host key PCSX2 binds it to. These are PCSX2's own
#: stock `[Pad1]` bindings, written out explicitly because a `[Pad1]` section
#: that names only `Type` produces **no bindings at all** and no warning - see
#: the doc page's "the silent gate" section.
PAD1_BINDINGS = [
    ("Up", "Keyboard/Up"),
    ("Right", "Keyboard/Right"),
    ("Down", "Keyboard/Down"),
    ("Left", "Keyboard/Left"),
    ("Triangle", "Keyboard/I"),
    ("Circle", "Keyboard/L"),
    ("Cross", "Keyboard/K"),
    ("Square", "Keyboard/J"),
    ("Select", "Keyboard/Backspace"),
    ("Start", "Keyboard/Return"),
    ("L1", "Keyboard/Q"),
    ("L2", "Keyboard/1"),
    ("R1", "Keyboard/E"),
    ("R2", "Keyboard/3"),
    ("L3", "Keyboard/2"),
    ("R3", "Keyboard/4"),
    ("LUp", "Keyboard/W"),
    ("LRight", "Keyboard/D"),
    ("LDown", "Keyboard/S"),
    ("LLeft", "Keyboard/A"),
    ("RUp", "Keyboard/T"),
    ("RRight", "Keyboard/H"),
    ("RDown", "Keyboard/G"),
    ("RLeft", "Keyboard/F"),
]

#: Abstract button -> X keysym name, derived from PAD1_BINDINGS above. Callers
#: say `cross`, never `k`.
BUTTON_KEYSYM = {
    "up": "Up", "down": "Down", "left": "Left", "right": "Right",
    "triangle": "i", "circle": "l", "cross": "k", "square": "j",
    "select": "BackSpace", "start": "Return",
    "l1": "q", "l2": "1", "r1": "e", "r2": "3", "l3": "2", "r3": "4",
    "lup": "w", "lright": "d", "ldown": "s", "lleft": "a",
    "rup": "t", "rright": "h", "rdown": "g", "rleft": "f",
    # hotkeys, not pad buttons
    "screenshot": "F8", "frameadvance": "F7", "pause": "space",
}

#: `g_input` in `SCES_547.48`, from
#: `docs/ghidra/functions/ps2-pulse-eu/names.tsv`. It holds a **pointer** to the
#: per-pad block; the offsets below are into `*g_input` and come from
#: `docs/ghidra/functions/ps2-pulse-eu/input.md`.
G_INPUT = 0x00302EC0
PAD_CONNECTED = 0x3C
PAD_HELD = 0x44
PAD_PRESSED = 0x50
PAD_STRIDE = 0x138

#: `[EmuCore/GS] ScreenshotSize`. Measured on SCES-54748, upscale multiplier 1:
#:
#:     0  screen resolution                     640x480  (the render window)
#:     1  internal resolution, aspect corrected 682x512
#:     2  internal resolution, uncorrected      512x512  <- the GS framebuffer
#:
#: 2 is the default here because it is the only one that is not resampled: the
#: PS2 draws Pulse into a 512x512 buffer and the PAL CRTC stretches it to
#: 640x512 for display. Anything compared against our own renderer should be
#: compared against what the console actually rasterised.
SCREENSHOT_INTERNAL = 2

#: The internal resolution SCES-54748 renders at, from the dimensions PCSX2's
#: own `ScreenshotSize = 2` shot comes back at. Used to size the render window
#: so a paused X grab is 1:1 with that buffer.
INTERNAL_RESOLUTION = (512, 512)

#: A word in `SCES_547.48`'s own data that increments by exactly one per
#: emulated frame. Found by scanning 2 MB of EE RAM across four verified
#: single-frame steps: exactly two words in that range increment by one every
#: time, `0x0027a7e8` and `0x002850fc`, and they track each other. It carries no
#: name here on purpose - naming it is a Ghidra-corpus change with its own
#: evidence page, and this harness only needs "did the frame I asked for
#: happen". See `docs/reverse-engineering/pcsx2-debugger.md`.
FRAME_COUNTER = 0x0027A7E8

#: Abstract bit layout, from the same page. Used only to pretty-print a mask.
ABSTRACT_BITS = [
    (0x0001, "up"), (0x0002, "down"), (0x0004, "left"), (0x0008, "right"),
    (0x0010, "circle"), (0x0020, "cross"), (0x0040, "triangle"),
    (0x0080, "square"), (0x0100, "l1"), (0x0200, "r1"),
]

INI_TEMPLATE = """\
; Written by scripts/pcsx2-drive.py. This is a throwaway data path for scripted
; runs; the user's own ~/.config/PCSX2 is never touched.
[UI]
SettingsVersion = 1
SetupWizardIncomplete = false
ConfirmShutdown = false
StartFullscreen = false
HideMouseCursor = true

[Folders]
Bios = {bios}

[EmuCore]
EnablePINE = true
PINESlot = {slot}
EnableFastBoot = true
SaveStateOnShutdown = false
WarnAboutUnsafeSettings = false

[EmuCore/GS]
Renderer = {renderer}
VsyncEnable = false
; Stretch, not the default `Auto 4:3/3:2`: with the render window resized to the
; game's internal resolution this makes the presented image the internal buffer
; 1:1, which is what an X grab of a *paused* frame can then capture. Under 4:3
; the same window is letterboxed and the grab is 30 % RMSE away from the GS
; output. Headless, there is no other reason to care what the window looks like.
AspectRatio = Stretch
ScreenshotSize = {screenshot_size}
ScreenshotFormat = 0
OsdShowSpeed = false
OsdShowFPS = false
OsdShowIndicators = false
OsdShowMessages = {osd_messages}

[Logging]
EnableFileLogging = true
EnableTimestamps = true
EnableEEConsole = true
EnableIOPConsole = true

[Hotkeys]
Screenshot = Keyboard/F8
FrameAdvance = Keyboard/F7
TogglePause = Keyboard/Space
ToggleTurbo = Keyboard/Tab

[Pad]
MultitapPort1 = false
MultitapPort2 = false

[Pad1]
Type = DualShock2
Deadzone = 0
AxisScale = 1.33
{pad1}
"""


def log(message):
    print(message, flush=True)


def display_running():
    return xvfb_display.display_running(DISPLAY_NUMBER)


def start_display():
    """Bring up Xvfb :78. Returns True if this call started it.

    `-listen tcp -nolisten unix` and the `127.0.0.1:78` address are not
    cosmetic: a sandboxed session may be unable to write `/tmp/.X11-unix`, and
    then the unix socket never appears while the server itself is perfectly
    fine. The RPCS3 harness hit this first; it costs a run every time.

    Ownership is recorded in `DISPLAY_MARKER` so `stop` - a separate
    invocation later - can tear this down without ever touching a display it
    did not start. See `xvfb_display`.
    """
    return xvfb_display.bring_up(DISPLAY_NUMBER, DISPLAY_GEOMETRY,
                                 DISPLAY_MARKER)


def stop_display():
    """Tear down Xvfb :78, but only if this tooling started it.

    Idempotent: no-op and no error when the display was never ours, is
    already down, or `stop` runs a second time.
    """
    return xvfb_display.tear_down(DISPLAY_NUMBER, DISPLAY_MARKER)


def write_config(slot, renderer=12, screenshot_size=SCREENSHOT_INTERNAL,
                 osd_messages=False):
    """Write the scripted-run data path's `PCSX2.ini`.

    Two things about the layout, both of which looked like a broken command
    line the first time: `-datapath <dir>` appends `PCSX2/` of its own, so the
    ini goes in `<dir>/PCSX2/inis/`; and `[Folders] Bios` is resolved relative
    to that same directory unless it is absolute, so it points at the user's
    real BIOS directory rather than a copy.
    """
    if not os.path.isdir(BIOS_DIR):
        raise RuntimeError("no BIOS directory at %s" % BIOS_DIR)
    for path in (INI_DIR, SNAP_DIR, STATE_DIR,
                 os.path.join(DATA_PATH, "PCSX2", "memcards"),
                 os.path.join(DATA_PATH, "PCSX2", "cache"),
                 os.path.join(DATA_PATH, "PCSX2", "logs")):
        os.makedirs(path, exist_ok=True)
    pad1 = "\n".join("%s = %s" % row for row in PAD1_BINDINGS)
    ini = INI_TEMPLATE.format(
        bios=BIOS_DIR, slot=slot, renderer=renderer,
        screenshot_size=screenshot_size,
        osd_messages="true" if osd_messages else "false", pad1=pad1)
    target = os.path.join(INI_DIR, "PCSX2.ini")
    with open(target, "w") as handle:
        handle.write(ini)
    return target


def emulator_running():
    return subprocess.run(["pgrep", "-x", "pcsx2-qt"],
                          capture_output=True).returncode == 0


def stop_emulator(quiet=False):
    if not emulator_running():
        if not quiet:
            log("no pcsx2-qt running")
        return False
    subprocess.run(["pkill", "-x", "pcsx2-qt"], capture_output=True)
    for _ in range(20):
        time.sleep(0.5)
        if not emulator_running():
            if not quiet:
                log("pcsx2-qt stopped")
            return True
    subprocess.run(["pkill", "-9", "-x", "pcsx2-qt"], capture_output=True)
    return True


def start_emulator(image, binary="pcsx2-qt", extra=()):
    env = dict(os.environ)
    env["DISPLAY"] = DISPLAY
    env["QT_QPA_PLATFORM"] = "xcb"
    env.pop("WAYLAND_DISPLAY", None)
    command = [binary, "-datapath", DATA_PATH, "-batch", "-nogui", "-fastboot",
               "-earlyconsolelog", "-logfile", LOG_FILE]
    command.extend(extra)
    command.extend(["--", os.path.abspath(image)])
    subprocess.Popen(command, env=env, stdout=subprocess.DEVNULL,
                     stderr=subprocess.DEVNULL, start_new_session=True)
    return command


class Keyboard:
    """XTEST key injection into the virtual display.

    Import of `Xlib` is deferred so that `display`, `config` and `stop` work
    without the dependency; run the input-touching commands under
    `uv run --with python-xlib`.
    """

    def __init__(self):
        try:
            from Xlib import X, XK, display as xdisplay
            from Xlib.ext import xtest
        except ImportError as exc:
            raise RuntimeError(
                "python-xlib is missing; run this under "
                "`uv run --with python-xlib python3 ...`") from exc
        self.X, self.XK, self.xtest = X, XK, xtest
        self.display = xdisplay.Display(DISPLAY)
        self.root = self.display.screen().root
        self.window = None
        self.geometry = None

    def find_render_window(self):
        """The mapped PCSX2 render window, or None if it is not up yet."""
        for child in self.root.query_tree().children:
            if child.get_attributes().map_state != self.X.IsViewable:
                continue
            geometry = child.get_geometry()
            if geometry.width > 100 and geometry.height > 100:
                self.window, self.geometry = child, geometry
                return child
        return None

    def focus(self):
        """Point the keyboard at the render window. See the module docstring."""
        if self.window is None and self.find_render_window() is None:
            raise RuntimeError(
                "no mapped PCSX2 render window on %s - did the emulator start?"
                % DISPLAY)
        self.window.set_input_focus(self.X.RevertToParent, self.X.CurrentTime)
        self.xtest.fake_input(
            self.display, self.X.MotionNotify,
            x=self.geometry.x + self.geometry.width // 2,
            y=self.geometry.y + self.geometry.height // 2)
        self.display.sync()

    def resize(self, width, height):
        """Resize the render window. Works with no window manager.

        `-fullscreen` does *not* - with nobody to honour the request the window
        stays 640x480 - but a plain `ConfigureWindow` is a direct X request and
        PCSX2's swapchain follows it. Measured at 512x512, 1024x1024 and
        1280x1024, parent and child both. This is what lets a paused grab be
        1:1 with the internal buffer instead of a scaled 640x480.
        """
        if self.window is None and self.find_render_window() is None:
            raise RuntimeError("no render window to resize")
        self.window.configure(width=width, height=height, x=0, y=0)
        self.display.sync()
        time.sleep(1.0)
        self.geometry = self.window.get_geometry()
        return self.geometry

    def _keycode(self, keysym_name):
        keysym = self.XK.string_to_keysym(keysym_name)
        if keysym == 0:
            raise RuntimeError("unknown keysym %r" % keysym_name)
        return self.display.keysym_to_keycode(keysym)

    def down(self, keysym_name):
        self.xtest.fake_input(self.display, self.X.KeyPress,
                              self._keycode(keysym_name))
        self.display.sync()

    def up(self, keysym_name):
        self.xtest.fake_input(self.display, self.X.KeyRelease,
                              self._keycode(keysym_name))
        self.display.sync()

    def tap(self, keysym_name, hold=0.12, after=0.35):
        self.down(keysym_name)
        time.sleep(hold)
        self.up(keysym_name)
        time.sleep(after)

    def press_button(self, name, hold=0.12, after=0.35):
        key = BUTTON_KEYSYM.get(name.lower())
        if key is None:
            raise RuntimeError(
                "unknown button %r; known: %s"
                % (name, ", ".join(sorted(BUTTON_KEYSYM))))
        self.tap(key, hold=hold, after=after)


def newest_snapshot():
    files = sorted(glob.glob(os.path.join(SNAP_DIR, "*.png")),
                   key=os.path.getmtime)
    return files[-1] if files else None


def gs_screenshot(keyboard, destination=None, timeout=20.0):
    """Take PCSX2's own screenshot (the F8 hotkey) and optionally move it.

    This is the frame you want for a renderer comparison: it is the GS output
    at the configured internal resolution, not a grab of a scaled window on the
    virtual display. `root_screenshot` is the fallback that needs no hotkey.

    The retry is load-bearing. Until the GS has presented a frame, F8 logs
    `Failed to render/download screenshot` and writes nothing - so a shot taken
    a couple of seconds after boot fails, and it fails the same way a lost key
    press does.
    """
    before = newest_snapshot()
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        keyboard.focus()
        keyboard.tap("F8", after=0.2)
        for _ in range(8):
            latest = newest_snapshot()
            if latest and latest != before and os.path.getsize(latest) > 0:
                _wait_until_written(latest)
                if destination:
                    shutil.move(latest, destination)
                    return destination
                return latest
            time.sleep(0.25)
    return None


def _wait_until_written(path, timeout=5.0):
    """Block until `path` stops growing.

    PCSX2 encodes the PNG on a worker thread, so the file appears before it is
    complete and moving it immediately truncates it - the symptom is
    ImageMagick's `unexpected end-of-file`, which reads like a corrupt frame
    rather than a race.
    """
    deadline = time.monotonic() + timeout
    last = -1
    while time.monotonic() < deadline:
        size = os.path.getsize(path)
        if size == last and size > 0:
            return
        last = size
        time.sleep(0.25)


def root_screenshot(path):
    """Grab the whole virtual display with ImageMagick. Needs no hotkey."""
    result = subprocess.run(["import", "-display", DISPLAY, "-window", "root",
                             str(path)], capture_output=True)
    if result.returncode != 0:
        log("root screenshot failed: %s" % result.stderr.decode()[:160])
        return None
    return path


def set_paused(pine, keyboard, paused, attempts=8):
    """Drive the VM to paused or running. `TogglePause` is a toggle, not a set.

    Every caller that assumed "press space -> paused" got it right half the time
    and silently ran the other half; ask PINE what state it is actually in.
    """
    from pcsx2_pine import STATUS_PAUSED, STATUS_RUNNING
    want = STATUS_PAUSED if paused else STATUS_RUNNING
    for _ in range(attempts):
        if pine.status() == want:
            return True
        keyboard.focus()
        keyboard.tap("space", after=0.6)
    return pine.status() == want


#: How long a `FrameAdvance` tap is held, and how long to wait after it. These
#: two numbers are the difference between exact and approximate. Measured over
#: 20 taps each, counting the guest frame counter either side:
#:
#:     hold=0.02 after=0.02 -> {0 frames: 11, 1: 8, 2: 1}
#:     hold=0.02 after=0.05 -> {0: 6, 1: 13, 2: 1}
#:     hold=0.02 after=0.15 -> {0: 2, 1: 17, 2: 1}
#:     hold=0.05 after=0.30 -> {1: 20}
#:
#: A tap that is too short is *dropped*, and one whose wait is too short lets
#: the emulator run on past the single frame it was asked for. Both directions
#: are silent. 0.05/0.30 costs about 3 stepped frames a second and is exact.
ADVANCE_HOLD = 0.05
ADVANCE_AFTER = 0.30


def advance_frames(pine, keyboard, count, counter=FRAME_COUNTER, retries=6):
    """Step exactly `count` emulated frames, verifying each one landed.

    This is the whole deterministic-capture story and it does not work without
    the verification. Measured, from one savestate replayed twice: a **verified**
    step reproduces EE RAM bit for bit, and sixty *unverified* taps do not - one
    run's sixty taps moved the guest 61 frames and the other's moved it 66, so
    every comparison after that is against a different point in the race.

    Raises if a step cannot be made to land, or if one overshoots: an overshoot
    is not recoverable by stepping (you cannot go back a frame), so the honest
    answer is to fail rather than to report a frame index that is wrong.
    """
    keyboard.focus()
    stepped = 0
    for _ in range(count):
        before = pine.read32(counter)
        for _ in range(retries):
            keyboard.tap("F7", hold=ADVANCE_HOLD, after=ADVANCE_AFTER)
            delta = (pine.read32(counter) - before) & 0xFFFFFFFF
            if delta == 1:
                stepped += 1
                break
            if delta > 1:
                raise RuntimeError(
                    "frame advance overshot by %d at step %d of %d; the frame "
                    "index is no longer known - reload the savestate"
                    % (delta - 1, stepped + 1, count))
        else:
            raise RuntimeError(
                "frame %d of %d did not advance (counter stuck at %d)"
                % (stepped + 1, count, before))
    return stepped


def describe_mask(mask):
    names = [name for bit, name in ABSTRACT_BITS if mask & bit]
    return "0x%04x%s" % (mask, (" (" + "+".join(names) + ")") if names else "")


def read_pad(pine, pad=0):
    """The game's own abstract input state for `pad`, via `*g_input`.

    Returns `(connected, held, pressed)`. This is the end-to-end check that an
    injected key actually became a button the *game* sees, rather than one the
    emulator merely received.
    """
    base = pine.read32(G_INPUT)
    if not 0x00100000 <= base < 0x02000000:
        raise PineError(
            "g_input reads 0x%08x, which is not an EE RAM pointer - either no "
            "game is running or this is not SCES-54748" % base)
    base += pad * PAD_STRIDE
    words = pine.read_words(base + PAD_CONNECTED, 6)
    return words[0], words[2], words[5]


def cmd_display(args):
    started = start_display()
    log("Xvfb :%d %s; address it as DISPLAY=%s"
        % (DISPLAY_NUMBER, "started" if started else "already running", DISPLAY))
    return 0


def cmd_config(args):
    path = write_config(args.slot, renderer=args.renderer,
                        screenshot_size=args.screenshot_size)
    log("wrote %s" % path)
    log("  BIOS      %s" % BIOS_DIR)
    log("  snaps     %s" % SNAP_DIR)
    log("  savestates %s" % STATE_DIR)
    return 0


def cmd_boot(args):
    if emulator_running():
        if not args.restart:
            log("pcsx2-qt is already running; pass --restart to replace it")
            return 1
        stop_emulator(quiet=True)
    start_display()
    write_config(args.slot, renderer=args.renderer,
                 screenshot_size=args.screenshot_size)
    if not os.path.exists(args.image):
        log("no disc image at %s - data/ is gitignored and does not travel "
            "into a worktree or a sandbox" % args.image)
        return 1
    start_emulator(args.image, binary=args.binary)
    pine = Pine(slot=args.slot)
    started = time.monotonic()
    try:
        game = pine.wait_for_game(timeout=args.timeout)
    except PineError as exc:
        log("boot failed: %s" % exc)
        log("the emulator log is %s" % LOG_FILE)
        return 1
    log("%s up after %.0f s (%s)"
        % (game, time.monotonic() - started, pine.title()))
    if args.press or args.shot:
        keyboard = Keyboard()
        for _ in range(40):
            if keyboard.find_render_window():
                break
            time.sleep(0.5)
        keyboard.focus()
        for button in args.press:
            keyboard.press_button(button)
            log("pressed %s" % button)
        if args.shot:
            time.sleep(args.settle)
            path = gs_screenshot(keyboard, args.shot)
            log("screenshot %s" % (path or "FAILED"))
    return 0


def cmd_press(args):
    keyboard = Keyboard()
    keyboard.focus()
    for button in args.buttons:
        keyboard.press_button(button, hold=args.hold, after=args.after)
        log("pressed %s" % button)
    return 0


def cmd_shot(args):
    if args.root:
        return 0 if root_screenshot(args.path) else 1
    keyboard = Keyboard()
    path = gs_screenshot(keyboard, args.path)
    log("screenshot %s" % (path or "FAILED"))
    return 0 if path else 1


def cmd_input(args):
    pine = Pine(slot=args.slot)
    connected, held, pressed = read_pad(pine, args.pad)
    log("pad %d connected=%d held=%s pressed=%s"
        % (args.pad, connected, describe_mask(held), describe_mask(pressed)))
    if args.button:
        keyboard = Keyboard()
        keyboard.focus()
        key = BUTTON_KEYSYM[args.button.lower()]
        keyboard.down(key)
        time.sleep(0.25)
        _, held, _ = read_pad(pine, args.pad)
        keyboard.up(key)
        log("while holding %-8s held=%s" % (args.button, describe_mask(held)))
        if held == 0:
            log("  the game saw nothing. Either the render window has no focus,"
                " or [Pad1] has no bindings - see the doc page.")
            return 1
    return 0


def cmd_frames(args):
    pine = Pine(slot=args.slot)
    keyboard = Keyboard()
    if args.window and args.window.lower() != "none":
        width, height = (int(part) for part in args.window.lower().split("x"))
        geometry = keyboard.resize(width, height)
        log("render window %dx%d" % (geometry.width, geometry.height))

    def anchor():
        """Put the VM on an exactly known frame, if the caller named a state.

        The order is load-*while-paused*, and it is the whole difference between
        a repeatable capture and a nearly-repeatable one. Loading while the VM
        runs leaves the guest free-running for however long the script takes to
        notice, which is hundreds of frames - measured at 770 frames of drift
        across one `loadstate`, a `status` call and a six-second sleep.
        """
        if args.from_state is None:
            return
        if not set_paused(pine, keyboard, True):
            raise RuntimeError("could not pause the VM")
        pine.loadstate(args.from_state)
        time.sleep(args.settle)
        if not set_paused(pine, keyboard, True):
            raise RuntimeError("could not re-pause after loading state %d"
                               % args.from_state)

    def attempt():
        if not set_paused(pine, keyboard, True):
            raise RuntimeError("could not pause the VM")
        if args.hold:
            keyboard.focus()
            for button in args.hold:
                keyboard.down(BUTTON_KEYSYM[button.lower()])
        try:
            return advance_frames(pine, keyboard, args.count,
                                  counter=args.counter)
        finally:
            for button in args.hold:
                keyboard.up(BUTTON_KEYSYM[button.lower()])

    for retry in range(args.retries + 1):
        try:
            anchor()
            stepped = attempt()
            break
        except RuntimeError as exc:
            # An overshoot cannot be stepped back out of. If the caller told us
            # which savestate the run starts from we can simply do it again;
            # otherwise the honest answer is to fail rather than to hand back a
            # frame index that is off by one.
            if args.from_state is None or retry == args.retries:
                raise
            log("%s - reloading state %d and retrying" % (exc, args.from_state))
    log("advanced %d frames (counter now %d)"
        % (stepped, pine.read32(args.counter)))
    if args.shot:
        # F8 writes nothing while the VM is paused, so the only capture that
        # works here is a grab of the virtual display.
        if root_screenshot(args.shot + ".full.png"):
            geometry = keyboard.geometry
            subprocess.run(
                ["magick", args.shot + ".full.png", "-crop",
                 "%dx%d+%d+%d" % (geometry.width, geometry.height,
                                  geometry.x, geometry.y),
                 "+repage", args.shot], check=True)
            os.unlink(args.shot + ".full.png")
            log("screenshot %s" % args.shot)
    if args.resume:
        set_paused(pine, keyboard, False)
    return 0


def cmd_state(args):
    pine = Pine(slot=args.slot)
    if args.action == "save":
        pine.savestate(args.slot_index)
        log("saved slot %d into %s" % (args.slot_index, STATE_DIR))
    else:
        pine.loadstate(args.slot_index)
        log("loaded slot %d" % args.slot_index)
    return 0


def cmd_status(args):
    log("Xvfb :%d      %s" % (DISPLAY_NUMBER,
                              "up" if display_running() else "down"))
    log("pcsx2-qt      %s" % ("running" if emulator_running() else "stopped"))
    try:
        pine = Pine(slot=args.slot, timeout=3.0)
        code = pine.status()
        log("PINE          %s (%s)" % (pine.version(),
                                       STATUS_NAMES.get(code, code)))
        if code != 2:
            log("game          %s %s" % (pine.game_id(), pine.title()))
    except PineError as exc:
        log("PINE          unreachable (%s)" % exc)
    return 0


def cmd_stop(args):
    """Stop pcsx2-qt, and Xvfb :78 too - but only the display this ran started.

    Always run this at the end of a drive session. `display`/`boot` are
    deliberately long-lived across many `press`/`shot` calls, so nothing
    tears the display down automatically between them; this is the explicit
    step that closes it out. Safe to run twice, or with nothing up at all.
    """
    stop_emulator()
    if stop_display():
        log("Xvfb :%d stopped" % DISPLAY_NUMBER)
    elif display_running():
        log("Xvfb :%d left running (not started by this tooling)"
            % DISPLAY_NUMBER)
    else:
        log("no Xvfb :%d running" % DISPLAY_NUMBER)
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--slot", type=int, default=28011)
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("display").set_defaults(func=cmd_display)

    p_config = sub.add_parser("config")
    p_config.add_argument("--renderer", type=int, default=12)
    p_config.add_argument("--screenshot-size", type=int, default=1)
    p_config.set_defaults(func=cmd_config)

    p_boot = sub.add_parser("boot")
    p_boot.add_argument("--image", default=DEFAULT_IMAGE)
    p_boot.add_argument("--binary", default="pcsx2-qt")
    p_boot.add_argument("--renderer", type=int, default=12)
    p_boot.add_argument("--screenshot-size", type=int, default=1)
    p_boot.add_argument("--timeout", type=float, default=180.0)
    p_boot.add_argument("--restart", action="store_true")
    p_boot.add_argument("--press", nargs="*", default=[],
                        help="buttons to tap once the game is up")
    p_boot.add_argument("--shot", help="write a screenshot here when done")
    p_boot.add_argument("--settle", type=float, default=1.0)
    p_boot.set_defaults(func=cmd_boot)

    p_press = sub.add_parser("press")
    p_press.add_argument("buttons", nargs="+")
    p_press.add_argument("--hold", type=float, default=0.12)
    p_press.add_argument("--after", type=float, default=0.35)
    p_press.set_defaults(func=cmd_press)

    p_shot = sub.add_parser("shot")
    p_shot.add_argument("path")
    p_shot.add_argument("--root", action="store_true",
                        help="grab the whole virtual display instead of the "
                             "GS output")
    p_shot.set_defaults(func=cmd_shot)

    p_input = sub.add_parser("input")
    p_input.add_argument("--pad", type=int, default=0)
    p_input.add_argument("--button", help="hold this button while reading")
    p_input.set_defaults(func=cmd_input)

    p_frames = sub.add_parser("frames")
    p_frames.add_argument("count", type=int)
    p_frames.add_argument("--hold", nargs="*", default=[],
                          help="buttons held down across the advance")
    p_frames.add_argument("--shot", help="grab the paused frame here")
    p_frames.add_argument("--counter", type=lambda t: int(t, 0),
                          default=FRAME_COUNTER)
    p_frames.add_argument("--resume", action="store_true",
                          help="unpause when done")
    p_frames.add_argument("--from-state", type=int, default=None,
                          help="savestate slot this run starts from; lets an "
                               "overshoot be retried instead of failing")
    p_frames.add_argument("--retries", type=int, default=3)
    p_frames.add_argument("--settle", type=float, default=2.0)
    p_frames.add_argument("--window", default="%dx%d" % INTERNAL_RESOLUTION,
                          help="resize the render window before grabbing, so "
                               "the paused frame is 1:1 with the internal "
                               "buffer; `none` to leave it alone")
    p_frames.set_defaults(func=cmd_frames)

    p_state = sub.add_parser("state")
    p_state.add_argument("action", choices=("save", "load"))
    p_state.add_argument("slot_index", type=int, nargs="?", default=1)
    p_state.set_defaults(func=cmd_state)

    sub.add_parser("status").set_defaults(func=cmd_status)
    sub.add_parser("stop").set_defaults(func=cmd_stop)

    args = parser.parse_args(argv)
    try:
        return args.func(args)
    except (PineError, RuntimeError) as exc:
        print("error: %s" % exc, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
