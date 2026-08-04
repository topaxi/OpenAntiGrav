# The handling stats loader (PS2)

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`.

This is the PS2 counterpart of the parameter-block half of
[psp-pulse-usa/engine.md](../psp-pulse-usa/engine.md) and of the camera-block half of
[psp-pulse-usa/camera.md](../psp-pulse-usa/camera.md). It is the most valuable page in
this directory, because it reproduces **every offset those two pages derived
from the PSP binary, exactly**, in a binary built by a different compiler for a
different ISA.

**The names below are applied**, from [names.tsv](names.tsv). Nothing scores
below 70.

## The dispatcher

`Handling_ParseStats` (`0x0014e6f0`) walks `HandlingStats.xml`
(`"Data\XML\HandlingStats.xml"` at `0x002c0f90`) and dispatches one parser per
element, exactly as the PSP function of the same name does. The element names
live as a contiguous run of 8- and 16-byte-aligned literals at
`0x002a5a38`..`0x002a5b30`, read out of memory rather than inferred:

```
0x002a5a38 "Handling"      0x002a5aa0 "ExternalCameraClose"
0x002a5a48 "Global"        0x002a5ab8 "Misc"
0x002a5a50 "Stats"         0x002a5ac0 "AirbrakeGraphics"
0x002a5a58 "InternalCamera"  0x002a5ad8 "Engine"
0x002a5a68 "BonnetCamera"    0x002a5ae0 "Brakes"
0x002a5a78 "BackwardCamera"  0x002a5ae8 "Turning"
0x002a5a88 "ExternalCameraFar" 0x002a5af0 "Airbrake"
                             0x002a5b00 "Antigrav"
                             0x002a5b10 "Physical"
                             0x002a5b20 "Class"
                             0x002a5b28 "Pitch"
```

The class index is a file-scope global, `g_handling_parse_class`
(`0x002da920`), set when a `<Class name="...">` attribute matches one of four
names looked up from a table at `0x002dab08`; the per-class stride is
`class * 0x80`. Same mechanism, same stride, same four classes as PSP.

## Every offset matches the PSP page, to the byte

| Block | Parser (PS2) | Offsets (`+ class * 0x80`) |
| --- | --- | --- |
| `Antigrav` | `HandlingXml_ParseAntigrav` (`0x0014d4a0`) | `ride_height 0x94`, `rebound 0x98`, `landing_rebound 0x9c`, `rebound_jump_time 0xa0`, `grip_ground 0xa4`, `grip_air 0xa8` |
| `Brakes` | `HandlingXml_ParseBrakes` (`0x0014d7c0`) | `gain 0xac`, `amount 0xb0`, `falloff 0xb4` |
| `Engine` | `HandlingXml_ParseEngine` (`0x0014d638`) | `gain 0xb8`, `amount 0xbc`, `falloff 0xc0`, `accelcap 0xc4`, `turbo 0xc8` |
| `Turning` | `HandlingXml_ParseTurning` (`0x0014d8e0`) | `gain 0xcc`, `amount 0xd0`, `falloff 0xd4` |
| `Airbrake` | `HandlingXml_ParseAirbrake` (`0x0014dc90`) | `gain 0xd8`, `falloff 0xdc`, `amount 0xe0`, `turn 0xe4`, `drag 0xe8`, `slidegrip 0xec`, `sideshift 0xf0` |
| `Physical` | `HandlingXml_ParsePhysical` (`0x0014d230`) | `mass 0xf4`, `normal_gravity 0xf8`, `flight_gravity 0xfc`, `track_gravity 0x100` |
| `Pitch` | `HandlingXml_ParsePitch` (`0x0014d9d8`) | `pitch_air 0x104`, `pitch_ground 0x108`, `pitch_damping 0x10c`, `antigrav_height_adjust 0x110` |

Every row above is the same offset the PSP page assigns to the same attribute
name, including the irregularities that page flags: `Antigrav` and `Brakes`
still precede `Engine` in memory, `Airbrake` still follows `Turning`, and
`Airbrake` is still `gain, falloff, amount` where `Engine`, `Brakes` and
`Turning` are `gain, amount, falloff`.

**The four load-time scale factors are also identical**, read off the same
store sites:

| Field | Stored as | PS2 site |
| --- | --- | --- |
| `Engine.amount` | `xml * 0.001` | `HandlingXml_ParseEngine` |
| `Brakes.amount` | `xml * -0.01` | `HandlingXml_ParseBrakes` |
| `Airbrake.amount` | `xml * 0.0001` | `HandlingXml_ParseAirbrake` |
| `Airbrake.slidegrip` | `xml * 0.0001` | `HandlingXml_ParseAirbrake` |

`HandlingXml_ParseAirbrakeGraphics` (`0x0014de78`) stores `amount *
0.017453294` at `0x6c` on the stats base and `up_speed`/`down_speed` raw at
`0x70`/`0x74` - the fifth pre-scaled parameter, and the degrees-to-radians
conversion that [psp-pulse-usa/camera.md](../psp-pulse-usa/camera.md) found, reproduced
to seven digits.

The camera block is likewise identical:

| XML element | Parser (PS2) | Offsets, in struct order |
| --- | --- | --- |
| `<InternalCamera>` | `HandlingXml_ParseInternalCamera` (`0x0014cc98`) | `+0x00` fov, `+0x04` height, `+0x08` length, `+0x0c` pitch, `+0x10` headtilt |
| `<BonnetCamera>` | `HandlingXml_ParseBonnetCamera` (`0x0014ce90`) | `+0x14` .. `+0x20` |
| `<BackwardCamera>` | `HandlingXml_ParseBackwardCamera` (`0x0014cda0`) | `+0x24` .. `+0x30` |
| `<ExternalCameraFar>` | `HandlingXml_ParseExternalCameraFar` (`0x0014cf80`) | `+0x34` pos_height, `+0x38` pos_length, `+0x3c` lookat_height, `+0x40` lookat_length, `+0x44` fov, `+0x48`, `+0x4c` |
| `<ExternalCameraClose>` | `HandlingXml_ParseExternalCameraClose` (`0x0014d0d8`) | `+0x50` .. `+0x68`, same seven in the same order |

`<BackwardCamera headtilt>` is **dead here too**: that parser tests `fov`,
`height`, `length` and `pitch` and nothing else, while `<InternalCamera>` does
store its `headtilt` at `+0x10`. Two builds discarding the same authored
attribute is not an accident of one compiler.

Confidence **92-93** for the parsers, **90** for the dispatcher. Per the
[rubric](../../../reverse-engineering/confidence-rubric.md) this is the
"corroborated in a second binary" leg of the top band; it is capped below 95
only because neither build has been observed loading a real file at runtime.
**The PSP page's 90 should be read as confirmed rather than merely plausible
now**, and its Cross-platform note has been updated to say so.

## What the PS2 answers that the PSP page left open

**`stats_base + 0x90` is `<Misc weight_distribution>`.**
[psp-pulse-usa/engine.md](../psp-pulse-usa/engine.md) records that `Ship_UpdatePitch`
reads "`stats_base + 0x90`, a per-team scalar outside every class block; its
element is not determined". The PS2 binary has an element the PSP page never
mentions, `<Misc>`, parsed by `HandlingXml_ParseMisc` (`0x0014db08`), and it
writes exactly that offset:

| Attribute | Offset (stats base, not per class) |
| --- | --- |
| `width` | `0x78` |
| `length` | `0x7c` |
| `height` | `0x80` |
| `easyshield` | `0x84` |
| `mediumshield` | `0x88` |
| `hardshield` | `0x8c` |
| `shield` | `0x84`, `0x88` **and** `0x8c` at once (a default for all three) |
| `weight_distribution` | `0x90` |

The attribute names were read out of `.rodata` at `0x002a5780`..`0x002a57c0`,
not guessed. Note that `<Misc>` also fills `0x78`..`0x8c`, which sits between
`AirbrakeGraphics` (`0x6c`..`0x74`) and the first class block (`0x94`), so the
stats base is now accounted for from `0x6c` to `0x114` with no gap.

Confidence **88**: the offset is a direct read, and `0x90` is the only
stats-base field the PSP page named without an element. It is not higher
because the *PSP* binary was not re-checked for a `<Misc>` parser in this
session - it may have one that simply was not found, or the element may be
PS2-only, and those two possibilities have different consequences for
`docs/formats/handling-stats.md`.

**`HandlingXml_ParseGlobal` (`0x0014df58`)** handles the `<Global>` sibling of
`<Stats>`: a `<Class>`-keyed set of per-class values written to five separate
`.bss` arrays, plus three sub-elements dispatched to `0x0014e600` and one to
`0x0014e518`. Named at **82** - the dispatch shape is unambiguous, but what the
arrays feed was not traced, so nothing is claimed about their meaning.

**Three of the five are floats and two are strings.** `0x0033c340`,
`0x0033c350` and `0x0027e828` are written through `Xml_AttributeAsFloat`;
`0x0033c330` and `0x0033c320` are written through
`Xml_AttributeAsOwnedString` and therefore hold `char *` into
document-owned allocations, not numbers. An earlier revision of this page called
all five "per-class scalars", which was wrong; see
[xml-reader.md](xml-reader.md) for how the two accessors differ.

## Not determined

- ~~**The XML reader itself.**~~ Done: see [xml-reader.md](xml-reader.md). One
  consequence lands directly on this page - `Xml_AttributeAsFloat`
  (`0x00203868`), which every parser above uses, is hand-rolled and **does not
  understand exponent notation**. `1e-5` in `HandlingStats.xml` would parse as
  `-15`. No shipped value uses it, but a tool regenerating the file must not
  emit it.
- **Whether the PSP build has `<Misc>`.** See above.
- **Nothing here was verified at runtime.**

## Cross-platform

| Function | PS2 (`SCES_547.48`) | PSP (`BOOT.BIN`) |
| --- | --- | --- |
| `Handling_ParseStats` | `0x0014e6f0` | `0x0883a2f0` |
| `HandlingXml_ParseEngine` | `0x0014d638` | `0x0883945c` |
| `HandlingXml_ParseBrakes` | `0x0014d7c0` | `0x0883962c` |
| `HandlingXml_ParseTurning` | `0x0014d8e0` | `0x088398e0` |
| `HandlingXml_ParseAirbrake` | `0x0014dc90` | `0x08839a04` |
| `HandlingXml_ParseAntigrav` | `0x0014d4a0` | `0x08839278` |
| `HandlingXml_ParsePhysical` | `0x0014d230` | `0x08838f50` |
| `HandlingXml_ParsePitch` | `0x0014d9d8` | `0x0883977c` |
| `HandlingXml_ParseAirbrakeGraphics` | `0x0014de78` | `0x08839c68` |
| `HandlingXml_ParseInternalCamera` | `0x0014cc98` | `0x08838820` |
| `HandlingXml_ParseBonnetCamera` | `0x0014ce90` | `0x08838978` |
| `HandlingXml_ParseBackwardCamera` | `0x0014cda0` | `0x08838aa0` |
| `HandlingXml_ParseExternalCameraFar` | `0x0014cf80` | `0x08838bc8` |
| `HandlingXml_ParseExternalCameraClose` | `0x0014d0d8` | `0x08838d8c` |
| `HandlingXml_ParseMisc` | `0x0014db08` | not located |
| `HandlingXml_ParseGlobal` | `0x0014df58` | not located |
| `g_handling_parse_class` | `0x002da920` | `0x08b36bfc` |

## History

- 2026-07-27: first pass. 92-93 from an exact offset-for-offset match with the
  PSP loader across 32 class fields, 27 camera fields, five load-time scale
  factors and one dead attribute.
