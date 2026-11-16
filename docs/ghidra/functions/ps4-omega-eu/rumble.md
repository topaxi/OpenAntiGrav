# Rumble: the pattern library, the pad update and the triggers

Binary: `/omega/eboot-ps4-omega-eu.bin` (base, 1.00). Read 2026-11-04 from the
decompiler and disassembly, no live run. The mag strip handlers came first, on
[ships-effects.md](ships-effects.md#the-rumble-three-handlers-named-by-the-edges-conf-70).
The whole picture is [gameplay/rumble.md](../../../gameplay/rumble.md); the file
format is [rumble-xml.md](../../../formats/rumble-xml.md); HD's copy of the same
library is [its page](../ps3-hdfury-eu/rumble.md).

Every score here is capped at 84 (decompilation, consistent call sites) until a
live run; the two-binary agreement is what puts the laws at the top of that band.

| Address | Name | Conf |
| --- | --- | ---: |
| `0x0174b8d0` | `Rumble_LoadLibrary` | 84 |
| `0x0174abf0` | `Rumble_ParsePatternXml` | 84 |
| `0x0174a880` | `Rumble_QueueOnPad` | 84 |
| `0x0174a4f0` | `Rumble_PlayOnLocalPads` | 80 |
| `0x01747c90` | `Pad_UpdateState` (named on [player-input.md](player-input.md)) | 75 |
| `0x012f8370` | `Ship_UpdateContactRumble` | 82 |
| `0x012fb780` | `Ship_ReportWeaponHit` | 75 |

## Objects

`DAT_020fd940` is the pad manager. Four pads start at `mgr+0x80`, stride `0x1120`.
Each pad embeds the library at offset 0 of the pad's own `+0x80`:

| Pad offset | What |
| --- | --- |
| `+0x80` | ring write cursor |
| `+0xb0 + 8*id` | pattern pointer of slot `id`, 0 to 64 |
| `+0x2c0..+0x7c0` | 20 voices, `0x40` bytes each |
| `+0x7c0` | active voice count, this update |
| `+0x88..+0xa8` | five chosen voices |
| `+0xcf4`, `+0xcf8` | small and large motor, 0.0 to 1.0 |
| `+0x10dc` | mute. Only writes found are 0, at `0x012997aa` and in the constructor |
| `+0x10f0` | `scePadOpen` handle, -1 when closed |

Pattern (`0x998` bytes): `+0x08` positional, `+0x0c` inner, `+0x10` outer, `+0x14`
priority, `+0x18` looping, `+0x1c` class, `+0x20` loop start, `+0x24` loop end,
`+0x28` large count, `+0x2c` small count, `+0x30` large\[1200\], `+0x4e0`
small\[1200\], `+0x990` the voice it last started.

Voice: `+0x00` active, `+0x08` pattern, `+0x10` **release flag** (1: run off the
end, 0: wrap at the loop end), `+0x20` position, `+0x30`, `+0x31` the two output
bytes, `+0x34` frame, starting at -1.

## `Rumble_LoadLibrary` (`0x0174b8d0`)

Called once, from the pad constructor `FUN_017497c0` (`Pad.cpp`). Allocates `0x998`
per pattern and parses a fixed list of 64 paths into `+0x30 + 8*id` of the library:
the table is [on the format page](../../../formats/rumble-xml.md#every-pattern).
Slot 11 is left empty. The decompiler drops the path argument on seven calls; the
table was rebuilt from the disassembly's `LEA RSI` before each call. The 57 it does
show agree.

## `Rumble_ParsePatternXml` (`0x0174abf0`)

Reads `Type`, `Class`, `Behaviour`, `Priority`, `LargeActuator` and `SmallActuator`
under `RumbleApp/RumbleData`, in that order. `FPS` and `RumbleEditor` are never
named. `Class`: `strcmp(text, "UI")` at `0x0183a3f6`, stored `2 - (equal)`.

**Halving at load.** If `DAT_01f999e8 > 1`, both tracks are rebuilt at half length,
each output `(a + b + c) / 3` over overlapping triples. That global is
`*(char *)(member + 0x128) + 1` at `0x0160c305`, is reset to 1 at five other writes,
and gates split screen, so it reads as the local player count. The parser only runs from the
constructor, so at boot it is 0 or 1 and this path looks dead. Confidence 60;
reading when `FUN_017497c0` runs after a session starts would settle it.

## Queueing

`Rumble_QueueOnPad(mgr, pad, id)`, 15 call sites, walks the ring from the cursor
for a free voice, up to 20 tries, and **drops the request when none is free**. It
sets active, release flag 1, frame -1, and stores the voice in the pattern's
`+0x990`. `pad == -1` picks the first pad that reports connected and enabled.

`Rumble_PlayOnLocalPads(mgr, id, position)`, 18 call sites: in two-player split
(`DAT_02037d00`, `1 < DAT_01f999e8` at `FUN_012993b0`), one voice on pad 0 and one on
pad 1, with `position` copied to each. Outside it, one voice on the active pad and
**`position` is ignored** (the `param_3` copies are at the two split branches only),
so the voice's `+0x20` stays zero.

Every other site inlines the same ring walk. Finding them needs the byte pattern
`C0 07 00 00` (the ring end): 60 hits across the binary, of which the 13 functions
named on [gameplay/rumble.md](../../../gameplay/rumble.md#triggers) are rumble. Held
voices are the inlined ones that write release flag 0.

## `Pad_UpdateState` (`0x01747c90`)

Per pad, per update; virtual slot in `0x0192cbb8`.

1. Skip while closed, or while the `pad+4000` countdown runs.
2. Step every active voice: `frame += 1`; if `frame` has passed both track lengths
   the voice ends. Otherwise, if the release flag is 0 and `frame >= loop end`,
   `frame = loop start`. Outputs are the track bytes at `frame`, 0 past a track's
   end; the large byte is scaled by the positional gain when positional.
3. Gate: both bytes become 0 if `(class & DAT_020fda70) == 0`, or if
   `DAT_01f998e8` is set and `class != 1`.
4. Choose voices (below) and sum.
5. Write `scePadSetVibration(handle, {large, small})`, with `large = sum * 1/255`
   clamped, and `small` the **sum of the small bytes**, 1/255 each.

`DAT_020fda70` is the class mask: `FUN_0128de60` and `FUN_012993b0` write 1 (UI
only) outside a race and 3 (UI and Game) in one. `DAT_01f998e8` is the paused byte
(`FUN_0129bd10` clears it on resume).

**Choosing.** Up to 5 voices all play. From 6 up, voices are walked in ring order
into five slots: an empty slot takes the voice, otherwise it overwrites the lowest
priority of the five. A new voice can therefore displace a higher one that is not
the lowest, and it always displaces something, even when its own priority is
lower than all five.

**The small motor differs from HD.** Data values are 0 and 1. HD turns any 1 into
a full on ([its page](../ps3-hdfury-eu/rumble.md)); Omega passes the sum, so a
DualShock 4 gets 1..5 of 255, which is close to nothing. Whether that was intended
cannot be read off the code. Confidence 80 on the arithmetic.

**Gates that are not settings.** `DAT_019470e0` and `DAT_019470c8` guard the motor
writes. They are the `value` fields of `ActuatorLarge` and `ActuatorSmall` in a
`{id, name, value}` device table at `0x019470b8`, both 1, and no code writes them.
`Vibrat` appears in no Omega string outside the audio names, and in none of the
118 candidate files of `data09.psarc`. No in-game vibration option was found;
confidence 70. The console's own setting applies.

## Triggers

Listed on [gameplay/rumble.md](../../../gameplay/rumble.md#triggers), with HD's.

`Ship_UpdateContactRumble` (`0x012f8370`), called from `FUN_012eaf20` and
`FUN_01762e80` only when `ship+0x68fc == 0`, is the contact and landing selector;
see [the law](../../../gameplay/rumble.md#contact-and-landing). Ship fields: `+0x64d4` contact
measure, `+0x722c` idle time, `+0x7230` contact time, `+0x7250` last band,
`+0x64f8` held collision voice, `+0x64f0` held landing voice. HD's constants are
the same numbers.

`Ship_ReportWeaponHit` (`0x012fb780`), from `FUN_012fa3f0` (damage) and
`FUN_01767e30`. Returns unless the mode counts it; adds to per-weapon totals at
`DAT_02039628+0x2a0..0x2bc`. With a weapon source (`param_2 == 2`) it switches on
the weapon id: 0 Rocket, 1 Missile, 2 Plasma, 3 Cannon, 4 Mine, 5 Quake, 6 and 10
Bomb, 7 Leech. **HD has no equivalent: impact rumble is new in Omega.**

`Rumble(0.07, 1.0)` and `Rumble(0.06, 0.3)` on [plasma.md](plasma.md) are not this
library: see [HD's note](../ps3-hdfury-eu/rumble.md#not-this-library).
