"""A synthetic gamepad for RPCS3, and the preflight that says why there isn't one.

RPCS3 has no input API - not in its GDB stub, not on its command line - so the
only way a script presses a button is to hand the *kernel* a gamepad and let
RPCS3's evdev/SDL pad handler read it like any other device. That works in
`--headless`, needs no window and no focus, and runs at full emulator speed,
which is what makes it the PS3 analogue of PPSSPP's `input.buttons`
(`scripts/ppsspp_debugger.py`).

The cost is a permission. Creating a virtual device needs `/dev/uinput`, which
on a stock Arch install is `0600 root:root` because nothing creates a udev rule
for it - so the harness fails at the first press with a bare `PermissionError`
and no hint of what to do. Everything in `preflight()` exists to turn that into
a diagnosis and a command the user can paste:

    uv run --with evdev scripts/rpcs3_pad.py preflight

It checks the things that can each be wrong on their own - the `evdev`
dependency, the uinput module, the device node, its group, the caller's own
membership of it, and RPCS3's input config - and prints the fix for exactly the
ones that are, because "run this whole script" when only the last line is needed
is how a session ends up with a udev rule it did not want. The one deliberate
short-circuit: with no `/dev/uinput` at all, nothing after it is checked, since
its mode and group are not yet facts.

`pad_problems()` and `emulator_problems()` are kept apart on purpose. Creating a
kernel device and RPCS3 being configured to read one are separate failures, and
`Pad` asserts only the first - a pad is still worth having when the emulator
profile is missing, and diagnosing that as "cannot create a virtual pad" sends
the reader to fix the wrong thing.

**The input config is the second half of the trap, and it is silent.** With no
`input_configs/global/*.yml`, RPCS3 logs `Input configuration empty. Adding
default keyboard pad handler` and binds a keyboard - which in `--headless` has
no window to receive keys, so every press is swallowed and the emulator looks
like it is ignoring the pad. A three-line config naming the evdev handler and
the device is enough; RPCS3 fills every other key from its defaults, and answers
`Pad 0: device='...', handler=Evdev` / `Evdev device 0 connected` when it takes.
`install-config` writes it and `--input-config` selects it by name.

Needs `evdev`; run through `uv run --with evdev`, the way the PSP scripts take
`websocket-client`.

See docs/reverse-engineering/rpcs3-debugger.md.
"""

import grp
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import emu_guard  # noqa: E402

UINPUT = "/dev/uinput"
UINPUT_GROUP = "input"
INPUT_CONFIG_NAME = "oag"
UDEV_RULE = 'KERNEL=="uinput", GROUP="input", MODE="0660"'
UDEV_RULE_PATH = "/etc/udev/rules.d/99-uinput.rules"

# The device presents as a generic gamepad: RPCS3's evdev handler binds by
# capability rather than by name, and SDL's fallback mapping wants the
# `BTN_SOUTH`-family names rather than the legacy `BTN_A` aliases.
# Chosen to look like a DualShock 3. Only the **Evdev** handler has been tested,
# and RPCS3 logged `VID=0x0, PID=0x0` through it, so these ids are not doing any
# work that has been observed; they are here for the SDL handler, which is
# untested.
VENDOR = 0x054C
PRODUCT = 0x0268
#: `OAG_RPCS3_PAD_NAME` gives a member's virtual pad its own name, so two members'
#: RPCS3 instances (each with its own input profile naming its own pad) never
#: bind each other's device.
DEVICE_NAME = os.environ.get("OAG_RPCS3_PAD_NAME", "OpenAntiGrav Virtual Pad")

BUTTONS = {
    "cross": "BTN_SOUTH",
    "circle": "BTN_EAST",
    "square": "BTN_WEST",
    "triangle": "BTN_NORTH",
    "l1": "BTN_TL",
    "r1": "BTN_TR",
    "l2": "BTN_TL2",
    "r2": "BTN_TR2",
    "select": "BTN_SELECT",
    "start": "BTN_START",
    "ps": "BTN_MODE",
    "l3": "BTN_THUMBL",
    "r3": "BTN_THUMBR",
}

# The d-pad is a hat rather than four buttons, which is what both SDL and
# RPCS3's evdev handler expect from a gamepad.
DPAD = {
    "left": ("ABS_HAT0X", -1),
    "right": ("ABS_HAT0X", 1),
    "up": ("ABS_HAT0Y", -1),
    "down": ("ABS_HAT0Y", 1),
}

STICK_RANGE = 32767

# L2/R2 are also analog axes: RPCS3's stock evdev profile, which `oag.yml`
# leaves in place by naming only the device, reads the triggers from
# `ABS_Z`/`ABS_RZ`, so a `BTN_TL2` press alone never reaches the game
# (measured 2026-10-08, hd-handling: HD's airbrakes did not respond to it).
# `set("l2", ...)` writes the button and the full-scale axis together.
TRIGGERS = {"l2": "ABS_Z", "r2": "ABS_RZ"}
TRIGGER_RANGE = 255


def input_config_path(name=INPUT_CONFIG_NAME):
    """Where RPCS3 looks for the profile `--input-config <name>` selects."""
    root = os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config")
    return os.path.join(root, "rpcs3", "input_configs", "global", name + ".yml")


def input_config_body(device=None):
    return (
        "Player 1 Input:\n"
        "  Handler: Evdev\n"
        "  Device: %s\n" % (device or DEVICE_NAME)
    )


def install_input_config(name=INPUT_CONFIG_NAME, device=None):
    """Write the profile, creating the directory. Returns the path written."""
    path = input_config_path(name)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as handle:
        handle.write(input_config_body(device))
    return path


class PadUnavailable(RuntimeError):
    """`/dev/uinput` cannot be used, with the reason and the fix in the message."""


def pad_problems():
    """Every reason *this process* cannot create a virtual device.

    Deliberately narrower than `preflight()`: it covers the `evdev` dependency
    and `/dev/uinput`, and says nothing about RPCS3, because a missing emulator
    profile does not stop a kernel device being made and diagnosing it as if it
    did sends the reader to fix the wrong thing.

    Returns a list of `(problem, fix)` pairs, empty when a pad can be made, and
    never raises - `require()` is the one that stops a run.
    """
    problems = []

    try:
        import evdev  # noqa: F401
    except ImportError:
        problems.append(
            (
                "the `evdev` module is not importable, so no virtual device can "
                "be created at all",
                "just rpcs3-preflight"
                "   # or: uv run --with evdev python3 scripts/rpcs3_pad.py ...",
            )
        )

    if not os.path.exists(UINPUT):
        problems.append(
            (
                "%s does not exist - the uinput module is not loaded" % UINPUT,
                "sudo modprobe uinput",
            )
        )
        return problems

    info = os.stat(UINPUT)
    try:
        owner = grp.getgrgid(info.st_gid).gr_name
    except KeyError:
        owner = str(info.st_gid)

    if not os.access(UINPUT, os.W_OK):
        node_problems = []
        if owner != UINPUT_GROUP or not (info.st_mode & 0o060):
            node_problems.append(
                (
                    "%s is mode %04o group %s, so only root can open it - no udev "
                    "rule grants it to a group" % (UINPUT, info.st_mode & 0o777, owner),
                    "echo '%s' | sudo tee %s\n"
                    "sudo udevadm control --reload-rules && sudo udevadm trigger %s"
                    % (UDEV_RULE, UDEV_RULE_PATH, UINPUT),
                )
            )
        if owner == UINPUT_GROUP and UINPUT_GROUP not in _own_groups():
            node_problems.append(
                (
                    "%s belongs to group '%s' and this account is not in it"
                    % (UINPUT, UINPUT_GROUP),
                    "sudo usermod -aG %s $USER   # then log out and back in"
                    % UINPUT_GROUP,
                )
            )
        if not node_problems:
            node_problems.append(
                (
                    "%s exists and is not writable by this process, for a reason "
                    "this check does not cover (mode %04o, group %s)"
                    % (UINPUT, info.st_mode & 0o777, owner),
                    "ls -l %s   # and check any MAC policy (AppArmor, SELinux)"
                    % UINPUT,
                )
            )
        problems += node_problems
    return problems


def emulator_problems(name=INPUT_CONFIG_NAME, device=DEVICE_NAME):
    """Every reason RPCS3 would not *read* a pad that exists.

    One entry, and it is the silent one: with no profile RPCS3 logs `Input
    configuration empty. Adding default keyboard pad handler` and binds a
    keyboard, which in `--headless` has no window to feed it.
    """
    path = input_config_path(name)
    if not os.path.exists(path):
        return [
            (
                "RPCS3 has no '%s' input profile (%s), so it falls back to the "
                "keyboard pad handler - which in --headless has no window to "
                "receive keys, and swallows every press in silence" % (name, path),
                "just rpcs3-input-config",
            )
        ]
    with open(path) as handle:
        body = handle.read()
    if "Evdev" not in body or device not in body:
        return [
            (
                "%s does not name the Evdev handler and '%s', so RPCS3 will bind "
                "something else" % (path, device),
                "just rpcs3-input-config",
            )
        ]
    return []


def preflight():
    """Everything between a script and a button press reaching the game.

    `pad_problems()` then `emulator_problems()`, in the order to fix them.
    """
    return pad_problems() + emulator_problems()


def _own_groups():
    """The caller's group names. A gid with no entry is skipped, not fatal."""
    names = set()
    for gid in os.getgroups():
        try:
            names.add(grp.getgrgid(gid).gr_name)
        except KeyError:
            continue
    return names


def report(stream=sys.stderr):
    """Print the preflight verdict. Returns True when a pad can be created."""
    problems = preflight()
    if not problems:
        print("OK: %s is writable and RPCS3's '%s' input profile names the "
              "evdev handler.\n    Select it with `--input-config %s`. Note "
              "that presses reaching the emulator is not the\n    same as the "
              "game being at a screen to receive them: `--headless` never gets "
              "there.\n    See docs/reverse-engineering/rpcs3-debugger.md and "
              "scripts/rpcs3-drive.py."
              % (UINPUT, INPUT_CONFIG_NAME, INPUT_CONFIG_NAME), file=stream)
        return True
    print("Synthetic input into RPCS3 will not work on this machine:", file=stream)
    for problem, fix in problems:
        print("\n  %s\n" % problem, file=stream)
        for line in fix.splitlines():
            print("      %s" % line, file=stream)
    if any(UINPUT in problem for problem, _ in problems):
        print(
            "\nThe udev rule is the persistent form; `modprobe`/`chmod` alone "
            "are undone by a reboot.",
            file=stream,
        )
    print("\nRe-check with: just rpcs3-preflight", file=stream)
    return False


def require():
    """Raise `PadUnavailable` if a device cannot be created.

    Checks `pad_problems()` only. A pad is worth having even when RPCS3 is not
    configured to read one - `preflight` is where that gets reported.
    """
    problems = pad_problems()
    if not problems:
        return
    lines = ["cannot create a virtual pad:"]
    for problem, fix in problems:
        lines.append("  %s" % problem)
        lines += ["      %s" % line for line in fix.splitlines()]
    lines.append("  full check: just rpcs3-preflight")
    raise PadUnavailable("\n".join(lines))


class Pad:
    """A virtual gamepad, held open for as long as the harness needs it.

    RPCS3 binds a pad when it enumerates devices, so **create this before
    launching the emulator** - a pad that appears afterwards is not picked up
    without a rescan.
    """

    def __init__(self, name=DEVICE_NAME):
        require()
        from evdev import AbsInfo, UInput, ecodes

        self.ecodes = ecodes
        caps = {
            ecodes.EV_KEY: [getattr(ecodes, code) for code in BUTTONS.values()],
            ecodes.EV_ABS: [
                (ecodes.ABS_X, AbsInfo(0, -STICK_RANGE, STICK_RANGE, 0, 0, 0)),
                (ecodes.ABS_Y, AbsInfo(0, -STICK_RANGE, STICK_RANGE, 0, 0, 0)),
                (ecodes.ABS_RX, AbsInfo(0, -STICK_RANGE, STICK_RANGE, 0, 0, 0)),
                (ecodes.ABS_RY, AbsInfo(0, -STICK_RANGE, STICK_RANGE, 0, 0, 0)),
                (ecodes.ABS_HAT0X, AbsInfo(0, -1, 1, 0, 0, 0)),
                (ecodes.ABS_HAT0Y, AbsInfo(0, -1, 1, 0, 0, 0)),
                (ecodes.ABS_Z, AbsInfo(0, 0, TRIGGER_RANGE, 0, 0, 0)),
                (ecodes.ABS_RZ, AbsInfo(0, 0, TRIGGER_RANGE, 0, 0, 0)),
            ],
        }
        try:
            self.ui = UInput(caps, name=name, vendor=VENDOR, product=PRODUCT,
                             version=0x0100)
        except PermissionError as exc:
            raise PadUnavailable(
                "%s became unwritable between the preflight and the open: %s"
                % (UINPUT, exc)
            ) from exc
        self.held = set()
        # udev and RPCS3 both need a moment to notice the new device.
        time.sleep(0.5)

    @property
    def device(self):
        return self.ui.device.path

    def close(self):
        self.release_all()
        self.ui.close()

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()

    def set(self, name, down):
        """Hold or release one button or d-pad direction."""
        emu_guard.beat()
        if name in BUTTONS:
            self.ui.write(self.ecodes.EV_KEY,
                          getattr(self.ecodes, BUTTONS[name]), 1 if down else 0)
            if name in TRIGGERS:
                self.ui.write(self.ecodes.EV_ABS, getattr(self.ecodes, TRIGGERS[name]),
                              TRIGGER_RANGE if down else 0)
        elif name in DPAD:
            axis, value = DPAD[name]
            self.ui.write(self.ecodes.EV_ABS, getattr(self.ecodes, axis),
                          value if down else 0)
        else:
            raise KeyError("no such button: %r" % name)
        if down:
            self.held.add(name)
        else:
            self.held.discard(name)
        self.ui.syn()

    def press(self, name, seconds=0.12):
        """Tap a button. The default is long enough for a 60 Hz poll to see it."""
        self.set(name, True)
        time.sleep(seconds)
        self.set(name, False)

    def stick(self, x=0.0, y=0.0, right=False):
        """Left or right stick, each axis in -1.0 .. 1.0."""
        ax = self.ecodes.ABS_RX if right else self.ecodes.ABS_X
        ay = self.ecodes.ABS_RY if right else self.ecodes.ABS_Y
        self.ui.write(self.ecodes.EV_ABS, ax, int(max(-1.0, min(1.0, x)) * STICK_RANGE))
        self.ui.write(self.ecodes.EV_ABS, ay, int(max(-1.0, min(1.0, y)) * STICK_RANGE))
        self.ui.syn()

    def release_all(self):
        for name in list(self.held):
            self.set(name, False)
        self.stick(0.0, 0.0)
        self.stick(0.0, 0.0, right=True)


def main(argv):
    command = argv[1] if len(argv) > 1 else "preflight"
    if command == "preflight":
        return 0 if report(stream=sys.stdout) else 1
    if command == "install-config":
        path = install_input_config()
        print("wrote %s\n\n%s" % (path, input_config_body()), end="")
        print("Select it with: rpcs3 --input-config %s" % INPUT_CONFIG_NAME)
        return 0
    if command == "pad":
        for problem, fix in emulator_problems():
            print("warning: %s\n         %s" % (problem, fix), file=sys.stderr)
        with Pad() as pad:
            print("virtual pad at %s - holding for %s seconds"
                  % (pad.device, argv[2] if len(argv) > 2 else 10))
            time.sleep(float(argv[2]) if len(argv) > 2 else 10.0)
        return 0
    print("usage: rpcs3_pad.py [preflight|install-config|pad [seconds]]",
          file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
