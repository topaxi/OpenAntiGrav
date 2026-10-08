#!/usr/bin/env python3
"""Sample the HD player craft's engine-crossfade inputs while a race is driven.

The evidence reproducer for `docs/ghidra/functions/ps3-hdfury-eu/xfade.md`.
Boots the disc in RPCS3 on a private virtual display, walks into a race, then
at each stage (grid, thrust held, thrust plus steering, thrust plus airbrake)
pauses the target over the GDB stub and reads, from the craft with
`craft+0x7a60 == 0`:

  craft+0x5f20 / +0x5f24 / +0x5f2c / +0x5f30   the four channel values
  craft+0x628c                                  role
  *(craft+0x5fac)+0x260                         "X", the unidentified term
  *(craft+0x5fac)+0x84 -> +4,+8,+0xc            ctrl
  *(craft+0x6944)+0x4c4                         the speed field's source
  craft+0x344                                   the 4.12 value physics.md names

    OAG_RPCS3_DISPLAY=93 OAG_RPCS3_GDB=127.0.0.1:2343 \\
      OAG_RPCS3_SCRATCH_CONFIG=<path> XDG_CACHE_HOME=<dir> \\
      env -u WAYLAND_DISPLAY uv run --with evdev python3 \\
        scripts/rpcs3-hd-engine-xfade-probe.py <out.json>
"""
import importlib.util, json, struct, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
spec = importlib.util.spec_from_file_location("rpcs3_drive", ROOT / "scripts" / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger

CRAFT_ARRAY = 0x0098D7C0
CRAFT_VTABLE = 0x008636E0
GDB_PORT = int(sys.argv[2]) if len(sys.argv) > 2 else 2343


def u32(b, o=0): return struct.unpack_from(">I", b, o)[0]
def i32(b, o=0): return struct.unpack_from(">i", b, o)[0]
def f32(b, o=0): return struct.unpack_from(">f", b, o)[0]


DUMP = True


def read_big(d, address, size):
    out = b""
    while len(out) < size:
        out += d.read(address + len(out), min(512, size - len(out)))
    return out


def find_player(d):
    """The craft whose `+0x7a60` is 0, found by vtable among the array's words.

    `physics.md` and `absorb-feedback.md` both say "the craft array at
    `0x0098d7c0`"; neither records the stride, so every word in a 0x200-byte
    window is tried as a pointer and kept when it points at `g_CraftVtable`.
    """
    window = d.read(CRAFT_ARRAY, 0x200)
    print("array head", window[:64].hex(" ", 4), flush=True)
    crafts = []
    for at in range(0, 8 * 4, 4):
        w = u32(window, at)
        head = u32(d.read(w, 4))
        role = i32(d.read(w + 0x7A60, 4))
        h5f18 = u32(d.read(w + 0x5F18, 4))
        print("entry", hex(w), "first word", hex(head), "+0x7a60", role,
              "+0x5f18", hex(h5f18), flush=True)
        crafts.append((w, role, h5f18))
    for w, role, h5f18 in crafts:
        if role == 0 and h5f18:
            return w
    return None


def sample(d, craft, label):
    d.pause()
    head = d.read(craft + 0x5F10, 0x30)
    role = i32(d.read(craft + 0x628C, 4))
    obj = u32(d.read(craft + 0x5FAC, 4))
    body = u32(d.read(craft + 0x6944, 4))
    x = f32(d.read(obj + 0x260, 4)) if obj else None
    ctrl = u32(d.read(obj + 0x84, 4)) if obj else 0
    cf = [f32(d.read(ctrl + o, 4)) for o in (4, 8, 12)] if ctrl else None
    field = f32(d.read(body + 0x4C4, 4)) if body else None
    out = dict(
        label=label,
        ch0=i32(head, 0x10), ch3=i32(head, 0x14), ch1=i32(head, 0x1C), ch2=i32(head, 0x20),
        flag5f1c=head[0x0C], mode5f14=i32(head, 0x04), role=role, X=x, ctrl=cf,
        node4c4=field, speed_field=None if field is None else field * 3.6,
        c344=f32(d.read(craft + 0x344, 4)),
    )
    if DUMP:
        out["obj"] = read_big(d, obj, 0x3A0).hex() if obj else None
        out["body"] = read_big(d, body, 0x600).hex() if body else None
        out["craft"] = read_big(d, craft + 0x300, 0x100).hex()
        vt = u32(d.read(obj + 8, 4)) if obj else 0
        out["obj_vtable"] = vt
        out["obj_vtable_words"] = read_big(d, vt, 0x80).hex() if vt else None
    d.resume()
    return out


out_path = Path(sys.argv[1])
rows = []
log_dir = out_path.parent / "probe-logs"
with drive.Session(str(ROOT / "data/images/hdfury-ps3-eu-dec.iso"), str(log_dir)) as s:
    print("rpcs3 pid", s.proc.pid, flush=True)
    if not s.wait_for_screen_pressing("Main Menu", 240):
        sys.exit("no main menu")
    s.settle_menu(20)
    screen = s.walk_to_race()
    if screen not in drive.RACE_ARRIVED:
        sys.exit("ended on %r" % screen)
    s.wait_for_load(70)
    d = Debugger(port=GDB_PORT)
    try:
        d.pause()
        craft = find_player(d)
        print("player craft", hex(craft) if craft else None, flush=True)
        d.resume()
        if craft is None:
            sys.exit("no player craft")
        rows.append(sample(d, craft, "grid"))
        s.pad.set("cross", True)
        for step in range(6):
            time.sleep(4)
            rows.append(sample(d, craft, "thrust-%d" % step))
        s.pad.set("left", True)
        for step in range(2):
            time.sleep(3)
            rows.append(sample(d, craft, "thrust-left-%d" % step))
        s.pad.set("left", False)
        s.pad.set("l1", True)
        for step in range(2):
            time.sleep(3)
            rows.append(sample(d, craft, "thrust-airbrake-%d" % step))
        s.pad.set("l1", False)
        s.pad.set("cross", False)
        time.sleep(4)
        rows.append(sample(d, craft, "coast"))
        d.resume()
    finally:
        out_path.write_text(json.dumps(rows, indent=1))
        for r in rows:
            print(r, flush=True)
