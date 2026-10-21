"""The whole field of a Wipeout HD race out of RPCS3's guest memory, and the HUD's own record.

Shared by `scripts/rpcs3-hd-race-hunt.py` (finds a mid-pack moment) and `scripts/rpcs3-drive.py capture --field`
(reads the field at the pause a screenshot is taken at). Every offset is measured and documented:

    0x0098d7c0            u32[8], the eight craft (`rpcs3_place.CRAFT_ARRAY`)
    ship+0x7a60           role word, 0 on the player
    ship+0x5fac -> +0x270 the craft's rigid body; body+0x1d0 basis rows, +0x200 position, +0x190 velocity
    ship+0x6298 -> +0x1b4 the team's path string, `Data\\Ships\\<team>_c1`
    ship+0x7014           the craft's running lap clock, seconds
    the HUD record       `lap` +0xc, `laps` +0x10, `place` +0x14 (1-based), `speed` +0x0, shield +0x48 and its
                         maximum +0x50 (docs/ghidra/functions/ps3-hdfury-eu/hud-readouts.md); the heap moves boot to
                         boot, so it is found by scanning for those fields

Raw reads are game memory: nothing here is written to a tracked file.
"""

import struct

GUEST = 0x300000000
CRAFT_ARRAY = 0x0098D7C0
RECORD_SCAN = (0x30000000, 0x36000000, 0x40000)
MAX_SHIELD = 140.0


class Mem:
    """Guest memory through any `seek`/`read`/`write` object (`serve`'s `RemoteMem`, or `/proc/<pid>/mem`)."""

    def __init__(self, mem):
        self.mem = mem

    def rd(self, addr, n):
        self.mem.seek(GUEST + addr)
        return self.mem.read(n)

    def wr(self, addr, data):
        self.mem.seek(GUEST + addr)
        self.mem.write(data)

    def u32(self, addr):
        return struct.unpack(">I", self.rd(addr, 4))[0]

    def f32(self, addr):
        return struct.unpack(">f", self.rd(addr, 4))[0]


def find_record(m, shield, laps_wanted=3):
    """Addresses of the HUD's per-player record: the player's shield (`ship+0x5fa0`, matched bit for bit)
    at +0x48, its maximum 140 at +0x50 and the race's laps at +0x10. The shield is what pins it: the
    lap and place words sit in other orders in different states (see `hud`)."""
    lo, hi, step = RECORD_SCAN
    hits = []
    needle = struct.pack(">f", shield)
    for base in range(lo, hi, step):
        try:
            blob = m.rd(base, step + 0x80)
        except Exception:  # noqa: BLE001 - an unmapped chunk is not a record
            continue
        i = blob.find(needle)
        while i != -1:
            rec = i - 0x48
            if rec >= 0 and rec + 0x58 <= len(blob):
                maximum = struct.unpack(">f", blob[rec + 0x50:rec + 0x54])[0]
                laps = struct.unpack(">I", blob[rec + 0x10:rec + 0x14])[0]
                if maximum == MAX_SHIELD and laps == laps_wanted:
                    hits.append(base + rec)
            i = blob.find(needle, i + 4)
    return hits


def hud(m, record):
    """The record's readable fields. `w0, laps, w2` at +0xc, +0x10, +0x14 are `(lap, laps, place)` in the
    hunt's boot, and one read after a restored game had run on gave `(6, 3, 1)`, which fits `(place, laps, lap - 1)`
    but matched no frame (the only reading matched to a frame is the paused restore's `(2, 3, 6)`: lap 2 of 3, place 6),
    so both readings are decoded and only a state where exactly one is valid answers: `place` and
    `lap` are None otherwise and the pair is refused rather than guessed."""
    b = m.rd(record, 0x58)
    w0, laps, w2 = struct.unpack(">3I", b[0xC:0x18])
    a_ok = 1 <= w0 <= laps and 1 <= w2 <= 8
    b_ok = 1 <= w0 <= 8 and 0 <= w2 < laps
    place = lap = None
    if a_ok and not b_ok:
        lap, place = w0, w2
    elif b_ok and not a_ok:
        place, lap = w0, w2 + 1
    return {"speed": struct.unpack(">f", b[0:4])[0], "thrust": struct.unpack(">f", b[8:12])[0],
            "lap": lap, "laps": laps, "place": place, "words": [w0, laps, w2],
            "shield": struct.unpack(">f", b[0x48:0x4C])[0], "shield_max": struct.unpack(">f", b[0x50:0x54])[0],
            "lap_time": struct.unpack(">f", b[0x30:0x34])[0]}


def team_of(m, ship):
    """`Feisar_c1` out of `Data\\Ships\\Feisar_c1`: the string `*(*(ship+0x6298)+0x1b4)` points at."""
    holder = m.u32(ship + 0x6298)
    text = m.u32(holder + 0x1B4) if holder else 0
    raw = m.rd(text, 48).split(b"\0")[0].decode("ascii", "replace") if text else ""
    return raw.replace("/", "\\").rsplit("\\", 1)[-1]


def field_rows(m):
    """Every craft in array order: player flag, team, body rows (left, up, forward), position, velocity, lap clock."""
    out = []
    ships = struct.unpack(">8I", m.rd(CRAFT_ARRAY, 32))
    for slot, ship in enumerate(ships):
        if not ship:
            continue
        entry = m.u32(ship + 0x5FAC)
        body = m.u32(entry + 0x270)
        rows = [list(struct.unpack_from(">3f", m.rd(body + 0x1D0 + 16 * i, 16))) for i in range(3)]
        out.append({"slot": slot, "ship": ship, "player": m.u32(ship + 0x7A60) == 0, "team": team_of(m, ship),
                    "rows": rows, "pos": list(struct.unpack(">3f", m.rd(body + 0x200, 12))),
                    "vel": list(struct.unpack(">3f", m.rd(body + 0x190, 12))), "clock": m.f32(ship + 0x7014)})
    return out


#: `MeshImporter`'s vtable (`0x00869fd0`, `docs/ghidra/functions/ps3-hdfury-eu/gantry-clock.md`): every scenery
#: node of a circuit is one, its animation time at `+0xc0`.
MESH_IMPORTER_VTABLE = 0x00869FD0
TIME_OFFSET = 0xC0
#: The race manager's windows for the gantry node's time (gantry-clock.md): before the first crossing
#: `[3.83, 5.25)`, then `[6.017, 9.3)`, `[9.5, 9.9)` and `[12.35, 13.3)`.
GANTRY_WINDOWS = ((3.83, 5.25), (6.017, 9.3), (9.5, 9.9), (12.35, 13.3))


def gantry_seconds(m, lo=0x30000000, hi=0x36000000, chunk=0x100000):
    """The start gantry's animation time, read out of RAM in the same pause as the frame.

    The race manager keeps the gantry node's time inside one of `GANTRY_WINDOWS` and resets it to the
    window's start when it leaves, so the original's `GO` board plays a phase the race clock alone does
    not give. A circuit has about 1,300 `MeshImporter` nodes: most sit at 0.0 (static) and the rest on
    the global clock; the gantry's own 19 (Metropia, 2026-10-21) read a value inside a window. Returns
    the most common such value, or `None` when no node is inside one (the board is gone).
    """
    needle = struct.pack(">I", MESH_IMPORTER_VTABLE)
    seen = {}
    for base in range(lo, hi, chunk):
        try:
            block = m.rd(base, chunk)
        except OSError:
            continue
        at = block.find(needle)
        while at >= 0:
            if at % 4 == 0 and at + TIME_OFFSET + 4 <= len(block):
                (t,) = struct.unpack(">f", block[at + TIME_OFFSET:at + TIME_OFFSET + 4])
                if any(a <= t < b for a, b in GANTRY_WINDOWS):
                    seen[t] = seen.get(t, 0) + 1
            at = block.find(needle, at + 4)
    return max(seen, key=seen.get) if seen else None
