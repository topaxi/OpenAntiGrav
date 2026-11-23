"""Pulse PS2's footer ticker and player tag, read out of a running PCSX2 over PINE (`docs/tools/compare-page.md`, "PS2 menu pairs: the footer").

The PSP reader (`psp_ticker.py`) finds the ticker widget by its row count and weight and reads two text records off it. The PS2 widget does not
keep those fields at the PSP's offsets (the row count 121 sits 0x34 before the first record, and the PSP's weight and last-drawn index are not
where `+0x104` and `+0xcc` would put them), so this reader anchors on the **text record**, whose layout is the PSP's:

    +0x00  active flag (1 while the message is on its way across the viewport)
    +0x04  head x, float: the message's left edge in the viewport's own units; it enters at the viewport's width and moves left 1 a frame
    +0x08  the viewport width, float: 493.0 (`0x43f68000`), the PS2 footer ticker's width in `Selection_Definition.xml`
    +0x0c  minus the text width, float
    +0x10  the NUL-terminated message, 0x104 bytes

Two records of 0x114 bytes lie back to back. A record refilled with the next message waits inactive at head x 492 until the one on screen has
left, so a frame holds one message or two. The item id is not read from the widget: a message that is a tip (`TKR_NO*`, what a time item says
with nothing counted) names its item by `UNTRIED`, and any other text names none and is refused, as the PSP reader refuses an item ours does not
count.

The profile record holds the name as an 11-byte space-padded field and the 3-character tag straight after it.

Reads are PINE word reads against a running VM (a paused VM presents no frame, so PCSX2's GS screenshot fails; `docs/tools/compare-page.md`).
"""

import re
import struct

RAM_START = 0x00100000
RAM_END = 0x02000000
CHUNK_WORDS = 0x4000
RECORD = 0x114
VIEWPORT_WIDTH_BITS = 0x43F68000
TEXT_AT = 0x10
TEXT_MAX = 0x100

#: What a time item says with nothing counted, by the item that says it: the same six pairs as `oag_ui_screens::news::untried_tip`.
UNTRIED = {"TKR_NOSR": "TKR_SRTIM", "TKR_NOSL": "TKR_SLTIM", "TKR_NOTT": "TKR_TTTIM", "TKR_NOTOURN": "TKR_TOURTIM", "TKR_NOELIM": "TKR_ELIMTIM",
           "TKR_NOZONE": "TKR_ZTIM"}

PROFILE_NAME_TAG = re.compile(rb"([A-Za-z0-9-]{1,10}) {1,9}\x00([A-Z0-9]{3})\x00")
ENTRY = re.compile(r'<entry id="([^"]+)" string="([^"]*)"')


def read_ram(pine, start=RAM_START, end=RAM_END):
    """The EE RAM from `start` to `end` as bytes (32 MiB takes about a second)."""
    chunks = []
    for base in range(start, end, CHUNK_WORDS * 4):
        words = pine.read_words(base, CHUNK_WORDS)
        chunks.append(struct.pack("<%dI" % len(words), *words))
    return b"".join(chunks)


def _record(blob, at):
    active, head, viewport, minus_width = struct.unpack_from("<Ifff", blob, at)
    text = blob[at + TEXT_AT:at + TEXT_AT + TEXT_MAX].split(b"\0")[0]
    return {"active": active, "head_x": head, "viewport": viewport, "width": -minus_width, "text": text.decode("latin1")}


def find_records(pine):
    """The addresses of the live pair of text records, found by signature; `[]` when the front end has no footer.

    A record has the viewport width at `+8`, a negative text width at `+0xc` and a printable message at `+0x10`; the pair is two such records
    `RECORD` apart. Several 493.0 floats lie in other objects, and those fail the width or the text test."""
    ram = read_ram(pine)
    key = struct.pack("<I", VIEWPORT_WIDTH_BITS)
    found = []
    at = ram.find(key)
    while at >= 0:
        start = at - 8
        if start >= 0 and start % 4 == 0 and start + TEXT_AT + 4 <= len(ram):
            record = _record(ram, start)
            printable = record["text"].isascii() and len(record["text"]) >= 3 and record["text"].isprintable()
            if record["active"] in (0, 1) and -4000 < -record["width"] < 0 and abs(record["head_x"]) < 4000 and printable:
                found.append(RAM_START + start)
        at = ram.find(key, at + 4)
    pairs = [a for a in found if a + RECORD in found]
    return sorted([a for a in found if a in pairs or a - RECORD in pairs])


def read_records(pine, addrs):
    """The records at `addrs`, one PINE read each."""
    return [dict(_record(pine.read_bytes(a, TEXT_AT + TEXT_MAX), 0), addr=a) for a in addrs]


def on_screen(record):
    """True when the record's message is inside the viewport (the head left of the width and the tail right of 0)."""
    return bool(record["active"]) and record["head_x"] < record["viewport"] and record["head_x"] + record["width"] > 0


def lone_message(records, margin):
    """The record of the one message on screen when its head is inside `[margin, viewport - margin]` (so that a read bracket of a few frames
    does not move it out of the viewport or let the next message in); else `None`."""
    visible = [r for r in records if on_screen(r)]
    if len(visible) != 1:
        return None
    record = visible[0]
    if not margin <= record["head_x"] <= record["viewport"] - margin:
        return None
    return record


def load_entries(oag_wad, wad, plugin="PI000"):
    """The English `<entry id=... string=...>` table of a language plugin, `{id: text}`: `oag-wad cat --expand WAD Data\\Plugins\\<plugin>\\entries.xml`."""
    import subprocess
    xml = subprocess.run([str(oag_wad), "cat", "--expand", str(wad), "Data\\Plugins\\%s\\entries.xml" % plugin], check=True, capture_output=True,
                         text=True).stdout
    return dict(ENTRY.findall(xml))


def item_of(text, entries):
    """The `PI_News` item whose untried tip reads `text`, or `None` (the text is not a tip, so ours cannot pin it)."""
    for tip, item in UNTRIED.items():
        if entries.get(tip) == text:
            return item
    return None


def profile_name_tag(ram):
    """`(name, tag, address of the tag)` of the one profile record in `ram` (read from `RAM_START`), or `None` unless exactly one matches."""
    hits = list(PROFILE_NAME_TAG.finditer(ram))
    if len(hits) != 1:
        return None
    m = hits[0]
    return m.group(1).decode("latin1"), m.group(2).decode("latin1"), RAM_START + m.start(2)
