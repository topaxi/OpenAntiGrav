"""Pulse PSP's footer ticker, read out of a running PPSSPP (`docs/ghidra/functions/psp-pulse-usa/race-box-screens.md`,
"The footer ticker's message choice").

The ticker widget (`FUN_088d1a64`/`FUN_088d1e04`) is found by its own two numbers: the count of `PI_News` rows at `+0xc8` and
the draw's total weight at `+0x104` (124 and 122 on both PSP pressings, `pulses_news_items_are_the_discs_list`). From there:

    +0xcc   the item index drawn last (an index into the `PI_News` rows in file order)
    +0xc4   the item array (freed after each draw, its pointers still read): item `+0xa0` id string, `+0xa4` format, `+0xa8` class,
            `+0xac` member, `+0xb0` frequency, `+0xb4` max
    +0xb4   where the next message starts: `(int)(width - width * 0.25)`
    +0x330  the speed, pixels a frame (1.0)
    two text records of 0x114 bytes from `+0x108`: `+0x00` active flag, `+0x04` head x (a message enters at the viewport's
            width and moves left), `+0x0c` minus the text width, `+0x10` the NUL-terminated text

The profile is `*0x08b31774`: the lifetime statistics are 200 words from `+0x11c`, the favourites are 0x20-byte strings from `+0x3c`
(team, circuit, speed class, music, event, friend, weapon).

Every read happens with the CPU stopped (`cpu.stepping`): a read against a running CPU costs half a second.
"""

import re
import struct
import subprocess

PROFILE_PTR = 0x08B31774
STATS_AT = 0x11C
STATS_SLOTS = 200
FAVOURITE_AT = 0x3C
#: The profile's player name (11 bytes) and footer tag (4 bytes; `AAA` on a fresh profile): `docs/ghidra/functions/psp-pulse-usa/race-box-screens.md`.
NAME_AT = 0x44C
TAG_AT = 0x457
FAVOURITE_FIELD = 0x20
NEWS_ROWS = 124
NEWS_WEIGHT = 122
RECORD = 0x114
RECORDS_AT = 0x108
WIDGET_BYTES = 0x340
HEAP = (0x08800000, 0x0A000000)
CHUNK = 0x400000

#: The statistic slots `crates/game/src/records/lifetime.rs` counts, and the modes the six time items belong to.
LAPS, TOTAL_TIME, MENU_TIME, SONGS = 2, 5, 85, 69
MODE_TIME_SLOTS = {"single_race": 6, "time_trial": 7, "zone": 8, "tournament": 10, "eliminator": 11, "head_to_head": 12, "speed_lap": 13}
FAVOURITE_MUSIC = 3


FORMATS = {"int": 0, "float": 1, "time": 2, "floattime": 3, "percent": 4, "string": 5}
CLASSES = {"genstats": 0, "profile": 1, "teamstats": 2, "trackstats": 3, "unlockable": 4}
NEWS_ENTRY = r"Data\Plugins\news\Definition.xml"


def load_rows(oag_wad, wad):
    """The `PI_News` rows in file order, which is the index `+0xcc` holds: `oag-wad cat --expand WAD NEWS_ENTRY`. (The widget's own item array is
    freed after each draw and its memory is reused, so the rows cannot be read back out of RAM.) Missing attributes are the original's zeroes."""
    xml = subprocess.run([str(oag_wad), "cat", "--expand", str(wad), NEWS_ENTRY], check=True, capture_output=True, text=True).stdout
    rows = []
    for values in re.findall(r"<Values ([^>]*)>", xml):
        attrs = dict(re.findall(r'(\w+)="([^"]*)"', values))
        rows.append({"id": attrs["NewsID"], "format": FORMATS.get(attrs.get("NewsFormat", "int").lower(), 0), "class": CLASSES.get(attrs.get("NewsClass", "genstats").lower(), 0),
                     "member": int(attrs.get("NewsMember", 0)), "frequency": int(attrs.get("NewsFrequency", 0))})
    return rows


def _find_widget(dbg, rows=NEWS_ROWS, weight=NEWS_WEIGHT):
    """The widget's address: the one place where `+0xc8` holds `rows` and `+0x104` holds `weight`. The CPU must be stopped."""
    key = struct.pack("<I", rows)
    hits = []
    for base in range(HEAP[0], HEAP[1], CHUNK):
        raw = dbg.read(base, CHUNK)
        if base + CHUNK < HEAP[1]:
            raw += dbg.read(base + CHUNK, 0x110)
        at = raw.find(key)
        while at >= 0:
            start = at - 0xC8
            if start >= 0 and start % 4 == 0 and start + 0x108 <= len(raw) and struct.unpack_from("<I", raw, start + 0x104)[0] == weight:
                hits.append(base + start)
            at = raw.find(key, at + 1)
    return hits


class Ticker:
    def __init__(self, dbg, news, weight=NEWS_WEIGHT):
        self.dbg = dbg
        self.news = news
        self.rows = len(news)
        self.weight = weight
        self.widget = None

    def find(self):
        """Locate the widget (stops the CPU for the scan and resumes it); `False` when the front end has none (not on a screen with the footer)."""
        self.dbg.call("cpu.stepping")
        try:
            hits = _find_widget(self.dbg, self.rows, self.weight)
        finally:
            self.dbg.call("cpu.resume")
        if len(hits) != 1:
            return False
        self.widget = hits[0]
        return True

    def sample(self):
        """The ticker's state; the CPU must be stopped. Raises when the widget moved (re-`find`)."""
        blob = self.dbg.read(self.widget, WIDGET_BYTES)
        word = lambda o: struct.unpack_from("<I", blob, o)[0]
        flt = lambda o: struct.unpack_from("<f", blob, o)[0]
        if word(0xC8) != self.rows or word(0x104) != self.weight:
            raise RuntimeError("the ticker widget at 0x%08x no longer reads as one" % self.widget)
        index = word(0xCC)
        item = dict(self.news[index], index=index) if index < self.rows else None
        buffers = []
        for k in range(2):
            at = RECORDS_AT + k * RECORD
            buffers.append({"active": blob[at] != 0, "head_x": flt(at + 4), "width": -flt(at + 12), "text": blob[at + 0x10:at + 0x10 + 0x100].split(b"\0")[0].decode("latin1")})
        profile = struct.unpack("<I", self.dbg.read(PROFILE_PTR, 4))[0]
        stats = struct.unpack("<%dI" % STATS_SLOTS, self.dbg.read(profile + STATS_AT, 4 * STATS_SLOTS))
        favourites = [self.dbg.read(profile + FAVOURITE_AT + k * FAVOURITE_FIELD, FAVOURITE_FIELD).split(b"\0")[0].decode("latin1") for k in range(7)]
        return {"widget": "0x%08x" % self.widget, "item": item, "next_starts_at": word(0xB4), "speed": flt(0x330), "buffers": buffers, "profile": "0x%08x" % profile,
                "stats_u32": {str(i): v for i, v in enumerate(stats) if v}, "stats_f32": {str(i): struct.unpack("<f", struct.pack("<I", v))[0] for i, v in enumerate(stats) if v},
                "favourites": favourites}


def profile_name_tag(dbg):
    """`(name, tag, address of the tag)` of the live profile (`*0x08b31774`): the stored player the footer's tag box draws, or `None` while the pointer is not a heap address (a boot that has not made the profile yet)."""
    profile = struct.unpack("<I", dbg.read(PROFILE_PTR, 4))[0]
    if not HEAP[0] <= profile < HEAP[1]:
        return None
    text = lambda at, size: dbg.read(profile + at, size).split(b"\0")[0].decode("latin1")
    return text(NAME_AT, 11), text(TAG_AT, 4), profile + TAG_AT


class Watch:
    """Which item each text record holds. The item array's `+0xcc` is the item drawn **last**, and a record is refilled with it the moment
    its old message has left the viewport, while the other record is the one on screen: so a record's item is only readable while the record
    waits (inactive with text), and has to be remembered until it goes active."""

    def __init__(self):
        self.pending = {0: None, 1: None}

    def observe(self, sample):
        for k, record in enumerate(sample["buffers"]):
            if not record["active"] and record["text"] and sample["item"]:
                self.pending[k] = sample["item"]

    def visible(self, sample):
        """`(item, head x, text)` of the one message on screen when exactly one record is active, its item is known and ours counts what it
        quotes and can seed from the profile table (`records/lifetime.rs`); else `None`. Total play time (slot 5) is not one: ours derives it from
        the menu and race time rather than storing it, so a seeded row would print a different number."""
        alive = [k for k, record in enumerate(sample["buffers"]) if record["active"]]
        if len(alive) != 1:
            return None
        item = self.pending[alive[0]]
        record = sample["buffers"][alive[0]]
        if item is None or not record["text"]:
            return None
        modelled = item["class"] == 0 and item["format"] != 5 and (
            item["member"] in (LAPS, MENU_TIME, SONGS) or item["member"] in MODE_TIME_SLOTS.values())
        favourite = item["class"] == 1 and item["member"] == FAVOURITE_MUSIC
        if not (modelled or favourite):
            return None
        return item, record["head_x"], record["text"]


def ours_lifetime(sample, title="wipeout pulse"):
    """The `records.toml` `[[lifetime]]` row that puts ours' counters where the original's table is, for the slots ours counts."""
    stats = {int(k): v for k, v in sample["stats_u32"].items()}
    lines = ["[[lifetime]]", 'title = "%s"' % title, "laps = %d" % stats.get(LAPS, 0), "menu_seconds = %.1f" % float(stats.get(MENU_TIME, 0))]
    ticks = {mode: int(stats.get(slot, 0)) * 60 for mode, slot in MODE_TIME_SLOTS.items() if stats.get(slot, 0)}
    if ticks:
        lines.append("[lifetime.race_ticks]")
        lines += ['%s = %d' % (mode, t) for mode, t in ticks.items()]
    songs = int(stats.get(SONGS, 0))
    favourite = sample["favourites"][FAVOURITE_MUSIC]
    if songs and favourite:
        lines += ["[lifetime.songs]", '"%s" = %d' % (favourite.replace('"', '\\"'), songs)]
    return "\n".join(lines) + "\n"
