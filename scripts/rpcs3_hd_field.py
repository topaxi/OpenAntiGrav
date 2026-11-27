"""The whole field of a Wipeout HD race out of RPCS3's guest memory, and the HUD's own record.

Shared by `scripts/rpcs3-hd-race-hunt.py` (finds a mid-pack moment) and `scripts/rpcs3-drive.py capture --field`
(reads the field at the pause a screenshot is taken at). Every offset is measured and documented:

    0x0098d7c0            u32[8], the eight craft (`rpcs3_place.CRAFT_ARRAY`)
    ship+0x7a60           role word, 0 on the player
    ship+0x5fac -> +0x270 the craft's rigid body; body+0x1d0 basis rows, +0x200 position, +0x190 velocity
    ship+0x6298 -> +0x1b4 the team's path string, `Data\\Ships\\<team>_c1`
    ship+0x7014           the craft's running lap clock, seconds
    *0x00936fd4 + 0xc4    the frame context's boot clock, seconds: engine shader parameter slot 0 (`time`), what the
                         engine flame's noise scrolls by (docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md,
                         "The flame's clock at a pair's grab")
    the HUD record       `lap` +0xc, `laps` +0x10, `place` +0x14 (1-based), `speed` +0x0, shield +0x48 and its
                         maximum +0x50 (docs/ghidra/functions/ps3-hdfury-eu/hud-readouts.md); the heap moves boot to
                         boot, so it is found by scanning for those fields

Raw reads are game memory: nothing here is written to a tracked file.
"""

import struct
import time

GUEST = 0x300000000
CRAFT_ARRAY = 0x0098D7C0
RECORD_SCAN = (0x30000000, 0x36000000, 0x40000)
MAX_SHIELD = 140.0
#: `Render_FrameContextPtr_q`: the frame context the present loop hands the render calls; its `+0xc4` is the boot clock.
FRAME_CONTEXT_PP = 0x00936FD4
FRAME_CLOCK_OFFSET = 0xC4


def flame_clock(m):
    """The frame context's boot clock in seconds (engine parameter slot 0), or `None` before the context exists.

    It runs at game time and stops with the emulator; HD's flame program reads it one to two frames behind the PPU.
    """
    ctx = m.u32(FRAME_CONTEXT_PP)
    return m.f32(ctx + FRAME_CLOCK_OFFSET) if ctx else None


#: The options block (`*(*0x008b098c)`); its byte `+0x473` is Pilot Assist, which `Hud_UpdateAssistIndicator` reads every frame
#: to draw the indicator's bracket under the craft and `Craft_Update` passes on to the craft
#: (`docs/ghidra/functions/ps3-hdfury-eu/pilot-assist.md`).
OPTIONS_PP = 0x008B098C
PILOT_ASSIST_OFFSET = 0x473


def pilot_assist_at(m):
    """The address of the Pilot Assist option byte, or `None` before the options block exists."""
    holder = m.u32(OPTIONS_PP)
    opts = m.u32(holder) if holder else 0
    return opts + PILOT_ASSIST_OFFSET if opts else None


def pilot_assist(m):
    """The Pilot Assist option byte (0 is off), or `None` before the options block exists."""
    at = pilot_assist_at(m)
    return m.rd(at, 1)[0] if at is not None else None


#: The race manager (`*0x0095ae78`): `+0x1950 +0x20` is the phase word and `+0x1950 +0x18` the seconds in it. Phase 1 is
#: the countdown (`docs/ghidra/functions/ps3-hdfury-eu/hud-start-gate.md`).
RACE_MANAGER_PP = 0x0095AE78
RACE_PHASE_BLOCK = 0x1950
COUNTDOWN_PHASE = 1


def race_phase(m):
    """`{"phase": int, "elapsed_s": float}` of the race manager, or `None` before it exists."""
    manager = m.u32(RACE_MANAGER_PP)
    if not manager:
        return None
    block = m.rd(manager + RACE_PHASE_BLOCK, 0x28)
    return {"phase": struct.unpack_from(">I", block, 0x20)[0], "elapsed_s": struct.unpack_from(">f", block, 0x18)[0]}


def wait_countdown(m, elapsed_s, timeout, clock=time.monotonic, sleep=time.sleep, say=print):
    """Poll, without pausing, until the race manager is `elapsed_s` into its countdown; returns that `race_phase`.

    The countdown is about 3 s, shorter than a capture script takes to start, so the wait runs inside the capture, with
    only the debugger attach after it. Raises `TimeoutError`.
    """
    began = told = clock()
    while True:
        now = race_phase(m)
        if now and now["phase"] == COUNTDOWN_PHASE and now["elapsed_s"] >= elapsed_s:
            return now
        if clock() - began > timeout:
            raise TimeoutError("no countdown within %.0f s (race manager %s)" % (timeout, now))
        if clock() - told >= 10:
            say("  waiting for the countdown (race manager %s)" % (now,))
            told = clock()
        sleep(0.05)


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


def model_hull(m, model):
    """`Feisar_c1` out of `Data\\Ships\\Feisar_c1`: the string a `PI_TeamModel`'s `+0x1b4` points at. A craft holds its
    model at `ship+0x6298`, Team Selection the selected one at `+0x194` of its screen object (`hd_ship_step.py`)."""
    text = m.u32(model + 0x1B4) if model else 0
    raw = m.rd(text, 48).split(b"\0")[0].decode("ascii", "replace") if text else ""
    return raw.replace("/", "\\").rsplit("\\", 1)[-1]


def team_of(m, ship):
    """The hull directory the craft was loaded from (`model_hull` of its `PI_TeamModel`)."""
    return model_hull(m, m.u32(ship + 0x6298))


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


STAMP_OFFSET = 0x15C
#: The first second the global clock can read outside every gantry window; below it a gantry node could pass for scenery.
GANTRY_WINDOW_END = max(b for _, b in GANTRY_WINDOWS)


def scenery_nodes(m, lo=0x30000000, hi=0x36000000, chunk=0x100000):
    """`(address, time, stamp)` of every `MeshImporter` in the heap, read in one pass over its blocks."""
    needle = struct.pack(">I", MESH_IMPORTER_VTABLE)
    found = []
    for base in range(lo, hi, chunk):
        try:
            block = m.rd(base, chunk)
        except OSError:
            continue
        at = block.find(needle)
        while at >= 0:
            if at % 4 == 0 and at + STAMP_OFFSET + 4 <= len(block):
                found.append((base + at, struct.unpack(">f", block[at + TIME_OFFSET:at + TIME_OFFSET + 4])[0],
                              struct.unpack(">f", block[at + STAMP_OFFSET:at + STAMP_OFFSET + 4])[0]))
            at = block.find(needle, at + 4)
    return found


#: `DetonatorMine`'s vtable (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine.md`, "The three vtables"); an object's
#: `+0x40` is its flags (1 active), `+0x80` its position, `+0xc6`/`+0xc8` its track entry and lane, `+0x160` "live".
DETONATOR_MINE_VTABLE = 0x008642F0
MINE_OBJECT_BYTES = 0x170


#: The words `DetonatorMine_AnimateToMusic` writes for the draw each update, by object offset
#: (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-material.md`, "Live mine").
MINE_DRAW_WORDS = (("football_scale", 0xD8), ("brightness", 0x100), ("master_volume", 0x104), ("pitch_offset", 0x108),
                   ("pitch_volume", 0x10C), ("bass_hit", 0x110), ("treble_hit", 0x114), ("tempo", 0x118))


def mine_draw_words(o):
    """The draw words of one mine object's bytes `o`, by name."""
    return {name: struct.unpack(">f", o[at:at + 4])[0] for name, at in MINE_DRAW_WORDS}


def mine_state(o):
    """The words of one mine object's bytes `o` that change while it stands: `live`, `age_s`, the two envelopes and the draw words."""
    return {"live": o[0x160], "age_s": struct.unpack(">f", o[0xC0:0xC4])[0],
            "football_s": struct.unpack(">f", o[0xF4:0xF8])[0], "spikes_t": struct.unpack(">f", o[0xF8:0xFC])[0],
            **mine_draw_words(o)}


#: `PulseBullet`'s vtable (`0x00863d98`); its age `+0x4c`, flags `+0x40`, launch speed (`500 + 3.6 v`) `+0x124`
#: (`docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`, "The round").
PULSE_BULLET_VTABLE = 0x00863D98
PULSE_BULLET_BYTES = 0x1C0
#: The pickup slot `*(ship+0x5edc)` keeps the clip's volleys left at `+0x164`; the race manager `**0x008a6814`
#: counts the game frames the trigger was held with a loaded, ready cannon at `+0x6de44`.
PICKUP_SLOT = 0x5EDC
CLIP_ROUNDS = 0x164
RACE_MANAGER = 0x008A6814
HELD_FRAMES = 0x6DE44


def pulse_bullets(m, lo=0x30000000, hi=0x40000000, chunk=0x100000):
    """The addresses of every `PulseBullet` in the heap (the pool of thirty, live or not)."""
    needle = struct.pack(">I", PULSE_BULLET_VTABLE)
    found = []
    for base in range(lo, hi, chunk):
        try:
            block = m.rd(base, chunk)
        except OSError:
            continue
        at = block.find(needle)
        while at >= 0:
            if at % 4 == 0:
                found.append(base + at)
            at = block.find(needle, at + 4)
    return sorted(found)


def cannon_state(m, ship, pool):
    """The cannon at one pause: game frames (1 or 2 ticks each) the trigger was held so far, volleys left, and every round with an age under its 1 s life
    (`{address, age_s, launch_speed, flags, position, prev}`)."""
    manager = m.u32(m.u32(RACE_MANAGER))
    rounds = []
    for address in pool:
        o = m.rd(address, PULSE_BULLET_BYTES)
        age = struct.unpack(">f", o[0x4C:0x50])[0]
        if 0.0 < age < 1.05:
            rounds.append({"address": address, "age_s": age, "flags": struct.unpack(">I", o[0x40:0x44])[0],
                           "launch_speed": struct.unpack(">f", o[0x124:0x128])[0],
                           "position": list(struct.unpack(">3f", o[0x170:0x17C])), "prev": list(struct.unpack(">3f", o[0xE0:0xEC]))})
    return {"held_frames": m.u32(manager + HELD_FRAMES), "volleys_left": m.u32(m.u32(ship + PICKUP_SLOT) + CLIP_ROUNDS),
            "rounds": rounds}


def detonator_mines(m, lo=0x30000000, hi=0x36000000, chunk=0x100000, keep_raw=False):
    """Every active `DetonatorMine` in the heap: `{address, flags, position, entry, lane, live (the byte at +0x160), age_s, football_s and spikes_t (the music envelopes `+0xf4` and `+0xf8`)}`, read in one pass
    (`keep_raw` adds the object's bytes as `raw`, hex, for a survey of its other fields)."""
    needle = struct.pack(">I", DETONATOR_MINE_VTABLE)
    found = []
    for base in range(lo, hi, chunk):
        try:
            block = m.rd(base, chunk + MINE_OBJECT_BYTES)
        except OSError:
            continue
        at = block.find(needle)
        while 0 <= at < chunk:
            if at % 4 == 0 and at + MINE_OBJECT_BYTES <= len(block):
                o = block[at:at + MINE_OBJECT_BYTES]
                flags = struct.unpack(">I", o[0x40:0x44])[0]
                if flags & 1:
                    found.append({"address": base + at, "flags": flags,
                                  "position": list(struct.unpack(">3f", o[0x80:0x8c])),
                                  "entry": struct.unpack(">H", o[0xC6:0xC8])[0], "lane": o[0xC8],
                                  **mine_state(o),
                                  **({"raw": o.hex()} if keep_raw else {})})
            at = block.find(needle, at + 4)
    return found


def _node_time(m, node):
    """`(time, stamp)` of one node, or `None` when it is no longer a `MeshImporter`."""
    block = m.rd(node, STAMP_OFFSET + 4)
    if struct.unpack(">I", block[:4])[0] != MESH_IMPORTER_VTABLE:
        return None
    return (struct.unpack(">f", block[TIME_OFFSET:TIME_OFFSET + 4])[0],
            struct.unpack(">f", block[STAMP_OFFSET:STAMP_OFFSET + 4])[0])


def _scenery_mode(pairs):
    seen = {}
    for t, stamp in pairs:
        if t > GANTRY_WINDOW_END and abs(t - stamp) < 0.03:
            seen[t] = seen.get(t, 0) + 1
    if not seen:
        return None
    best = max(seen, key=seen.get)
    return best, seen[best]


def scenery_clock(m, nodes=None):
    """The time the scenery's animation nodes run on, read out of RAM in the same pause as the frame.

    Every `MeshImporter` that moves is advanced from one global clock (`MeshImporter_AdvanceTime`,
    `docs/ghidra/functions/ps3-hdfury-eu/gantry-clock.md`): its time (`+0xc0`) equals its last stamp (`+0x15c`), the
    global clock, because it was set to 0 when that clock read 0. Returns `{"time_s", "nodes", "total"}`, the most common
    time among nodes whose time and stamp agree and sit past the gantry windows (the gantry's own nodes are reset into a
    window), or `None` when none do. The clock keeps running through the pre-race wait, so `time_s - lap clock` is the
    per-capture `world_clock_offset_s`. `nodes` limits the read to those addresses (a sample cached from an earlier
    full scan); a node that stopped being a `MeshImporter` is skipped.
    """
    if nodes is None:
        pairs = [(t, stamp) for _, t, stamp in scenery_nodes(m)]
    else:
        pairs = [tp for tp in (_node_time(m, n) for n in nodes) if tp is not None]
    mode = _scenery_mode(pairs)
    if mode is None:
        return None
    return {"time_s": mode[0], "nodes": mode[1], "total": len(pairs)}


#: How many nodes of the dominant group a cache keeps: enough that a few stale ones do not move the mode.
CLOCK_SAMPLE = 24


def world_clock_record(m, cache, grabbed_at=None):
    """The original's world clock for a frame grab, always a dict: the clock, or `null` with a reason.

    `{"world_clock_s", "lap_clock_s", "world_clock_offset_s", "nodes", "total", "reason", "scan", "flame_clock_s"}`;
    `flame_clock_s` (`flame_clock`) is read on every screen, a race or not. The scenery clock is
    only recorded while a race is running: a player craft in the craft array, a lap clock that is a finite non-negative
    number, and scenery nodes whose time and stamp agree. After a race the freed nodes still read plausible, so a
    menu screen with no player craft records `null` and says why. `cache` is a dict the caller keeps between grabs:
    the first grab scans the heap (`scan: "full"`), the next ones re-read a `CLOCK_SAMPLE` of the nodes it found
    (`scan: "cached"`, tens of reads instead of 96 MiB) and rescan when the sample no longer agrees.
    The offset `world_clock_s - lap_clock_s` is stable across a run's pauses; the two numbers are read within a few
    milliseconds of each other, and `paused` is not known here, so a film frame's `world_clock_s` itself is the moment of the read.
    """
    rec = {"world_clock_s": None, "lap_clock_s": None, "world_clock_offset_s": None, "nodes": None, "total": None,
           "reason": None, "scan": None, "flame_clock_s": None}
    try:
        rec["flame_clock_s"] = flame_clock(m)
    except Exception as error:  # noqa: BLE001 - the flame clock is a bonus beside the scenery clock
        rec["flame_clock_reason"] = "%s: %s" % (type(error).__name__, error)
    try:
        rows = field_rows(m)
    except Exception as error:  # noqa: BLE001 - a menu has no craft array, which is the answer
        rec["reason"] = "no craft array (%s: %s)" % (type(error).__name__, error)
        return rec
    player = next((r for r in rows if r["player"]), None)
    if player is None or not (player["clock"] == player["clock"]) or player["clock"] < 0 or player["clock"] > 1e5:
        rec["reason"] = "no race running (no player craft with a lap clock)"
        return rec
    lap = player["clock"]
    clock = None
    if cache.get("nodes"):
        clock = scenery_clock(m, cache["nodes"])
        rec["scan"] = "cached"
        if clock is None or clock["nodes"] * 2 < clock["total"]:
            clock = None
    if clock is None:
        found = scenery_nodes(m)
        rec["scan"] = "full"
        mode = _scenery_mode([(t, stamp) for _, t, stamp in found])
        clock = mode and {"time_s": mode[0], "nodes": mode[1], "total": len(found)}
        cache["nodes"] = [a for a, t, stamp in found if clock and t == clock["time_s"] and abs(t - stamp) < 0.03][:CLOCK_SAMPLE]
    if clock is None:
        rec["reason"] = "no scenery MeshImporter node agrees with its stamp"
        return rec
    rec.update(world_clock_s=clock["time_s"], lap_clock_s=lap, world_clock_offset_s=clock["time_s"] - lap,
               nodes=clock["nodes"], total=clock["total"])
    return rec
