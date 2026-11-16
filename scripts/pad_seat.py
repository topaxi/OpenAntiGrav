"""Pad seats: virtual gamepads that one agent's emulator sees and no other agent's does.

A uinput pad is a system-wide device, so every emulator on the machine sees every pad.
A seat gives one agent's pads their own identity and tells that agent's emulators to look at nothing else:

* `OAG_PAD_SEAT=N` (1..255) selects the seat. Unset keeps the original pads (`045E:028E`, "OAG test pad A
  (Microsoft X-Box 360 pad)"), which `oag-game`'s split-screen walk reads through gilrs.
* A seat's pads are named `OAG seat N pad X` and carry USB vendor `045E` with product `4F00 + N`.
* An SDL-based emulator gets `SDL_GAMECONTROLLER_IGNORE_DEVICES_EXCEPT=0x045E/0x4F0N` (SDL2 and SDL3 read the same
  name) and `SDL_GAMECONTROLLERCONFIG` with an Xbox-360 layout for that id, since SDL keys its built-in mapping on the
  id. `sdl_env(seat)` returns both.
* RPCS3's evdev handler binds by device name: `rpcs3_name(seat)`.
* An emulator that takes no pad through SDL (PCSX2 and Vita3K are driven by X key events) gets `sdl_blind_env()`, which
  makes it ignore every pad.

    from pad_seat import seat, sdl_env, own_pads
    s = seat()
    if own_pads(s): sys.exit("seat %s already has a pad" % s)
    env = {**os.environ, **sdl_env(s)}

`docs/tools/compare-page.md`, "Pad seats", has the operating notes.
"""

import os
import re

SEAT_ENV = "OAG_PAD_SEAT"
VENDOR = 0x045E
LEGACY_PRODUCT = 0x028E
#: Seat N's product id is SEAT_PRODUCT_BASE + N ("O" for OpenAntiGrav, 0x4F01..0x4FFF).
SEAT_PRODUCT_BASE = 0x4F00
MAX_SEAT = 255
LEGACY_NAME = "OAG test pad {letter} (Microsoft X-Box 360 pad)"
SEAT_NAME = "OAG seat {seat} pad {letter}"
#: The id no pad has: an EXCEPT list naming it hides every pad from the process.
BLIND_ID = (0x0000, 0x0000)

#: Xbox-360 layout in SDL's mapping syntax for the evdev pad `uinput-pads.py` makes (axes and hat as the Linux
#: `xpad` driver reports them: left stick ABS_X/Y, right stick ABS_RX/RY, triggers ABS_Z/RZ, hat ABS_HAT0).
XBOX360_LAYOUT = (
    "a:b0,b:b1,x:b2,y:b3,back:b6,guide:b8,start:b7,leftstick:b9,rightstick:b10,leftshoulder:b4,rightshoulder:b5,"
    "dpup:h0.1,dpdown:h0.4,dpleft:h0.8,dpright:h0.2,leftx:a0,lefty:a1,rightx:a3,righty:a4,lefttrigger:a2,righttrigger:a5,"
)


def seat():
    """The seat number from `OAG_PAD_SEAT`, or None for the original pads. Refuses a value outside 1..255."""
    raw = os.environ.get(SEAT_ENV, "").strip()
    if not raw:
        return None
    if not raw.isdigit() or not 1 <= int(raw) <= MAX_SEAT:
        raise SystemExit("%s must be a whole number 1..%d, got %r" % (SEAT_ENV, MAX_SEAT, raw))
    return int(raw)


def ids(seat_no):
    """(vendor, product) of a seat's pads; the original Xbox-360 ids for None."""
    if seat_no is None:
        return VENDOR, LEGACY_PRODUCT
    return VENDOR, SEAT_PRODUCT_BASE + seat_no


def pad_name(seat_no, letter):
    return (LEGACY_NAME if seat_no is None else SEAT_NAME).format(seat=seat_no, letter=letter.upper())


def rpcs3_name(seat_no):
    """The name RPCS3's input profile binds. `OAG_RPCS3_PAD_NAME` still wins (`emu-env.sh` writes it)."""
    explicit = os.environ.get("OAG_RPCS3_PAD_NAME")
    if explicit:
        return explicit
    return "OpenAntiGrav Virtual Pad" if seat_no is None else SEAT_NAME.format(seat=seat_no, letter="rpcs3")


def except_value(vendor, product):
    return "0x%04x/0x%04x" % (vendor, product)


def sdl_env(seat_no):
    """The environment that makes an SDL emulator see this seat's pads only. Empty for None: the original pads
    are what an unconfigured SDL program already sees."""
    if seat_no is None:
        return {}
    vendor, product = ids(seat_no)
    return {
        "SDL_GAMECONTROLLER_IGNORE_DEVICES_EXCEPT": except_value(vendor, product),
        "SDL_GAMECONTROLLERCONFIG": mapping(seat_no),
    }


def sdl_blind_env():
    """For an emulator that reads no pad through SDL: it sees none, whoever's seat they belong to."""
    return {"SDL_GAMECONTROLLER_IGNORE_DEVICES_EXCEPT": except_value(*BLIND_ID)}


def mapping(seat_no):
    """An SDL mapping line for the seat's pads. The GUID's CRC field is left zero, which SDL matches against any name."""
    vendor, product = ids(seat_no)
    le = lambda v: "%02x%02x" % (v & 0xFF, v >> 8)
    guid = "0300" + "0000" + le(vendor) + "0000" + le(product) + "0000" + le(0x0110) + "0000"
    return "%s,%s,platform:Linux,%s" % (guid, "OAG seat %d pad" % seat_no, XBOX360_LAYOUT)


def _devices():
    """(name, vendor, product) of every input device the kernel lists."""
    text = open("/proc/bus/input/devices", errors="replace").read()
    out = []
    for block in text.split("\n\n"):
        ident = re.search(r"Vendor=([0-9a-f]+) Product=([0-9a-f]+)", block)
        name = re.search(r'^N: Name="(.*)"', block, re.M)
        if ident and name:
            out.append((name.group(1), int(ident.group(1), 16), int(ident.group(2), 16)))
    return out


def tag(seat_no):
    """A suffix that keeps two seats' runtime directories apart (`""` for the original pads)."""
    return "" if seat_no is None else "-s%d" % seat_no


def require_free(seat_no):
    """Exit when a pad that would answer to this seat's presses already exists (a previous run's, or another
    process's on the same seat). Other seats' pads are not this run's business."""
    taken = own_pads(seat_no)
    if taken:
        raise SystemExit(
            "refusing to start: %s already exists, so it would answer to this run's presses%s"
            % (taken, "" if seat_no is not None else " (set %s=N to run on a seat of your own)" % SEAT_ENV))


def own_pads(seat_no):
    """Names of pads already on the machine that would answer to this seat's presses: those carrying the seat's
    product id, or for None any original Xbox-360-shaped pad (another run's, or a real one)."""
    vendor, product = ids(seat_no)
    return [name for name, v, p in _devices() if (v, p) == (vendor, product)]
