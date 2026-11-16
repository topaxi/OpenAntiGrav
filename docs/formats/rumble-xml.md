# Rumble patterns: `Data/XML/Rumble/*.xml`

Controller vibration authored as data: one XML file per pattern, two byte
tracks stepped once per pad update. HD/Fury, 2048 and Omega ship the same 69
patterns. Pulse (PSP, PS2) and Pure ship none; PS2 Pulse vibrates from code, and
the PSP has no motor. What fires each pattern and how they mix:
[gameplay/rumble.md](../gameplay/rumble.md).

Evidence: [Omega](../ghidra/functions/ps4-omega-eu/rumble.md),
[HD](../ghidra/functions/ps3-hdfury-eu/rumble.md),
[PS2](../ghidra/functions/ps2-pulse-eu/rumble.md).

**Layout confidence 90**: `Rumble_ParsePatternXml` read in two binaries
(Omega `0x0174abf0`, HD `0x00341de0`), and every one of the 69 files parses into
it with no element or attribute left over. No runtime trace; the emulators stayed
off in this pass.

## Reader

**Built.** `oag_tables::rumble::parse` reads one file into a `Pattern` (fields
named as the table below: `kind`, `loop_start`, `loop_end`, `class`, `priority`,
`positional`, `large`, `small`); `Library` holds patterns by name (a `Vec`, case-folded names, an
insert by name replaces, so an override is an insert); `pattern_name` takes the name
out of an archive path. `oag_mod::rumble::parse` reads our TOML form into the same
`Pattern`, and `oag_mod::rumble::from_forms` refuses a name shipped as both `.xml` and
`.toml` (`Error::BothForms`). The TOML reader sits in `oag-mod` because that crate
already owns `toml`, `serde` and `oag-schema` next to `oag-tables`, which has no
dependencies. Nothing plays a pattern yet.

The XML reader **decodes or refuses by name**, stricter than the original: an element
inside `RumbleData` it does not know, a `Type`, `Class`, `Priority` or `Behaviour` word
outside the lists above (the original reads any other `Priority` text as High), an
`FPS` other than 60, a track token that is not a byte, `FallOff*` on a `Global`
pattern, `LoopStart` or `LoopEnd` on a `OneShot`, a duplicate element. None of that occurs
on a disc. It tokenises tracks on white space, so the trailing space is not needed and
a track without it keeps its last value. Absent `Class` and `Priority` stay `None`
in the struct; `effective_class` and `effective_priority` apply the defaults above.

Ground truth (`crates/tables/tests/rumble_ground_truth.rs`, called from the
`ai_stats_lineage_ground_truth` archive walk so each archive opens once): HD disc
`DATA02`, HD PSN `data02`, 2048 EU base `data.psarc` and Omega `data00` each parse all
69 files with no refusal, and the census above holds on each: `OneShot` 57 and
`Looping` 12, `Positional` 7, `Game` 62, `UI` 3 and no `Class` 4, `High` 25,
`Medium` 25, `Low` 15 and no `Priority` 4, 24 files with unequal tracks, small motor
only 0 or 1 (476 ones, 1,085 zeros), 1 to 106 frames. One digest over all 69
patterns' parsed fields is the same on the four archives, which is the "same data"
row below, now measured on the parsed form and not only on the serialised element.
Confidence 90 (unchanged: the layout is from the parser; this adds the full census
through a second reader).

### Our own TOML form

`<name>.toml`, schema `schemas/v1/rumble.tosd` ([ADR-0064](../architecture/adr/0064-project-toml-files-declare-a-toml-schema.md)),
the maintainer's shape (2026-11-04). Words are the XML's, so a pattern converts by eye.

```toml
[toml-schema]
location = "https://oag.topaxi.com/mods/schemas/v1/rumble.tosd"
version = "1.0.0"

[pattern]
type = "Looping"        # "OneShot" | "Looping"
class = "Game"          # optional, "Game" | "UI"
priority = "High"       # optional, "Low" | "Medium" | "High"
loop = [1, 3]           # optional, [start, end], Looping only

[pattern.positional]    # optional; its presence is the Positional behaviour
fall_off_inner = 30
fall_off_outer = 20

[tracks]
large = [255, 200, 0]   # bytes 0..255, one per frame
small = [1, 0]
```

Unknown keys are refused. A `positional` table with no distances reads 0 and 0, as the
XML does. A unit test reads one invented pattern in both forms and asserts the `Pattern`s equal;
another proves the schema and the loader agree on 11 examples.

## Census

Listed from each PSARC's manifest with entries resolved by path digest
(2026-11-04). "Patterns" counts `*.xml` under `data/xml/rumble`, case-folded.

| Title | Archive | Patterns | Same data as HD | Loaded by the executable |
| --- | --- | --- | --- | --- |
| HD/Fury EU disc | `DATA02.PSARC` | 69 | - | 64 |
| HD PSN | `data02.psarc` | 69 | 69 of 69 | not read (same data) |
| 2048 EU, USA base | `data.psarc` (`data/xml/Rumble/`) | 69 | 69 of 69 | **0** |
| 2048 v1.04, DLC | `data1`, `data2`, `dlc1`, `dlc2` | 0 | - | - |
| Omega base | `data00.psarc` (`Data/xml/rumble/`) | 69 | 69 of 69 | 64 |
| Omega patch | `data05`, `07`, `08`, `09` | 0 | - | - |
| Pulse PSP USA, EU | `BEData`, `Data`, `FE`, `FEData` | 0 of 69 | - | 0 |
| Pulse PS2 EU | `PRERACE`, `PS2MUSIC`, `WADS2`, `WADSP` | 0 of 69 | - | 0 |
| Pure PSP USA, EU | `Data`, `FE`, `FEData` | 0 of 69 | - | 0 |

"Same data" compares the serialised `<RumbleData>` element, so the editor block
and the leading line ending do not count. HD's files are one byte shorter than
Omega's and 2048's; `<RumbleData>` is identical in all 69.

A WAD holds only name hashes, so the WAD rows look up `hash_name` of
`Data/XML/Rumble/<name>.xml` for all 69 names in every WAD's directory (18 WADs,
2026-11-04, `oag-wad list`): no hit. That proves these names absent, not a differently
named pattern; the executables' lack of any rumble string covers that.

`omega data04.psarc` also names four `Road_A_rumblestrip_*` textures, and 2048 the
same four: road art, not vibration. Pure's `rumble`, in `ship.bnk`, is a **sound
cue** ([psp-audio.md](psp-audio.md)), and has no bearing here.

**The executables.** No `rumble` string, case-folded, in the Pulse PSP USA or EU
`BOOT.BIN`, the Pure USA or EU `BOOT.BIN`, or `SCES_547.48`. The PSP has no motor
and imports no vibration call, so the three path names on
[cannon-quake-leachbeam.md](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md)
are HD's, as that page says.

**2048 ships all 69 and reads none.** Its `eboot.elf` (base and v1.04) holds no
`Rumble\\` path, no `RumbleApp` and no `LargeActuator`, and imports two `SceCtrl`
functions (`SceCtrl_67E7AB83`, `SceCtrl_A497B150`), neither named as an actuator
call in the tool's database. Two strings beside `System/System/Pad.cpp`
(`Rumble Othr ...`, `Pad is stationary.`, `Pad in use.`) are a debug line about
pad activity. The 69 are data left in the archive. Confidence 80 that 2048 has no
rumble at run time: a NID still unresolved is the gap.

**Five files have no loader in either executable**: `barrel_roll_rumble`,
`collision_light`, `collision_med`, `collision_other_rumble`,
`plasma_charging_rumble`. Each is a one-frame stub (a single `0` on each track)
with no `Class` and no `Priority`, except `barrel_roll_rumble`, which has both.
64 loaded plus 5 unloaded is 69, with no loaded name missing from the archive.

## Structure

A UTF-8 XML declaration, then one line:

```text
RumbleApp
  RumbleEditor            the authoring tool's view; the game never reads it
    Zoom                  float
    Offset                float
    Colours?              Colour0..Colour10, only in some files
  RumbleData
    FPS                   always 60; not read
    Type                  "OneShot" | "Looping"; LoopStart, LoopEnd attributes on Looping
    Class?                "Game" | "UI"
    Priority?             "Low" | "Medium" | "High"
    Behaviour             "Global" | "Positional"
    LargeActuator         FallOffInner, FallOffOuter attributes on Positional
    SmallActuator
```

| Field | Values across the 69 | Read as | Default if absent |
| --- | --- | --- | --- |
| `FPS` | `60` in all 69 | **not read**: a frame is one pad update | - |
| `Type` | `OneShot` 57, `Looping` 12 | looping flag | not looping |
| `LoopStart`, `LoopEnd` | integer frame indices, on Looping only | range; a frame reaching `LoopEnd` wraps to `LoopStart` (held voice only) | 0 |
| `Class` | `Game` 62, `UI` 3, absent 4 | mask bit: UI 1, anything else 2 | 2 (`Game`) |
| `Priority` | `High` 25, `Medium` 25, `Low` 15, absent 4 | Low 0, Medium 1, any other text 2 | 0 (`Low`) |
| `Behaviour` | `Global` 62, `Positional` 7 | positional flag | not positional |
| `FallOffInner`, `FallOffOuter` | whole numbers, on Positional only | distance, stored as `f32` | 0 |
| `LargeActuator` | space-separated bytes 0..255, one per frame | large motor | empty |
| `SmallActuator` | same; only 0 and 1 occur (476 ones, 1085 zeros) | small motor | empty |

Traps for a reader:

- **The tracks are not the same length** in 24 of 69 files: 45 match, the rest
  differ by 1 to 60 frames, the small track usually shorter. A missing frame reads
  0, and the voice ends when the frame passes **both** lengths.
- **Every track ends in a space, and the parser depends on it**: it copies
  `strlen - 1` characters before tokenising, so a track with no trailing space
  loses its last digit. A mod's file should keep it, or a reader of ours should
  tokenise plainly, which reads the shipped files identically.
- **Storage is 1,200 bytes per track.** The longest track shipped is 106.
- **`FallOffInner` can exceed `FallOffOuter`** (`quake_incoming_rumble` 30 and 20;
  `bomb_explosion_rumble` 20 and 20). The gain law is on
  [gameplay/rumble.md](../gameplay/rumble.md#positional-gain); those two are a hard
  cut at `FallOffInner`.
- `Priority`'s test is for `Medium` and `Low`; anything else is High.
- **The rate is implied.** At 60 Hz, 106 frames is 1.77 s. Omega halves a track
  at load when `DAT_01f999e8 > 1` (three-tap average, every second frame), which
  would keep the wall-clock length at half the update rate; see the Omega page.

## Every pattern

`id` is the library slot (the same in HD and Omega). `-` is no loader. Frame
counts, loop range, peak of the large track and the count of small-motor frames:
the values themselves stay on the disc.

| id | File | Type | Class | Priority | Behaviour | Large frames | Small frames | Loop | Large peak | Small on |
| --- | --- | --- | --- | --- | --- | ---: | ---: | --- | ---: | ---: |
| 0 | `collision_heavy_looping` | Looping | Game | High | Global | 61 | 62 | 0..6 | 255 | 13 |
| 1 | `collision_right_med_looping` | Looping | Game | High | Global | 33 | 12 | 0..3 | 0 | 10 |
| 2 | `collision_right_light_looping` | Looping | Game | Medium | Global | 33 | 31 | 0..3 | 0 | 6 |
| 3 | `collision_right_very_light_looping` | Looping | Game | Low | Global | 33 | 6 | 0..3 | 0 | 4 |
| 4 | `collision_left_med_looping` | Looping | Game | High | Global | 12 | 1 | 0..3 | 198 | 0 |
| 5 | `collision_left_light_looping` | Looping | Game | Medium | Global | 8 | 1 | 0..2 | 187 | 0 |
| 6 | `collision_left_very_light_looping` | Looping | Game | Low | Global | 4 | 1 | 0..1 | 207 | 0 |
| 7 | `landing_heavy` | OneShot | Game | High | Global | 31 | 31 |  | 255 | 7 |
| 8 | `landing_med` | OneShot | Game | High | Global | 31 | 31 |  | 253 | 7 |
| 9 | `landing_light` | OneShot | Game | Medium | Global | 31 | 31 |  | 198 | 7 |
| 10 | `landing_very_light` | OneShot | Game | Low | Global | 31 | 31 |  | 124 | 5 |
| 12 | `pad_speed` | OneShot | Game | Medium | Global | 13 | 13 |  | 255 | 7 |
| 13 | `pad_weapon` | OneShot | Game | Low | Global | 13 | 13 |  | 162 | 4 |
| 14 | `rlr_barrell_roll` | OneShot | Game | Low | Global | 35 | 36 |  | 207 | 12 |
| 15 | `lrl_barrell_roll` | OneShot | Game | Low | Global | 34 | 36 |  | 170 | 12 |
| 16 | `energy_critical_rumble` | OneShot | Game | Low | Global | 33 | 33 |  | 151 | 4 |
| 17 | `wrong_way` | OneShot | Game | Low | Global | 7 | 7 |  | 255 | 3 |
| 18 | `turbo_start_rumble` | OneShot | Game | High | Global | 55 | 9 |  | 147 | 9 |
| 19 | `fire_engine` | OneShot | Game | Medium | Global | 10 | 7 |  | 255 | 4 |
| 20 | `enter_mag_rumble` | OneShot | Game | Low | Global | 6 | 5 |  | 132 | 5 |
| 21 | `travel_mag_rumble` | Looping | Game | Low | Global | 39 | 39 | 2..38 | 72 | 3 |
| 22 | `exit_mag_rumble` | OneShot | Game | Low | Global | 6 | 5 |  | 132 | 5 |
| 23 | `absorb_any_pickup_rumble` | OneShot | Game | Low | Global | 56 | 56 |  | 192 | 6 |
| 24 | `turbo_fired_rumble` | OneShot | Game | Medium | Global | 69 | 60 |  | 255 | 9 |
| 25 | `autopilot_active_rumble` | Looping | Game | Low | Global | 47 | 62 | 18..37 | 255 | 10 |
| 26 | `shield_active_rumble` | Looping | Game | Low | Global | 62 | 62 | 6..55 | 255 | 8 |
| 27 | `bomb_fired_rumble` | OneShot | Game | Medium | Global | 15 | 15 |  | 255 | 7 |
| 28 | `cannon_fired_rumble` | OneShot | Game | Medium | Global | 6 | 6 |  | 255 | 3 |
| 29 | `leech_fired_rumble` | OneShot | Game | Medium | Global | 35 | 9 |  | 149 | 9 |
| 30 | `mine_fired_rumble` | OneShot | Game | Medium | Global | 16 | 16 |  | 253 | 6 |
| 31 | `missile_fired_rumble` | OneShot | Game | Medium | Global | 10 | 10 |  | 255 | 7 |
| 32 | `plasma_fired_rumble` | OneShot | Game | High | Global | 64 | 63 |  | 255 | 16 |
| 33 | `quake_fired_rumble` | OneShot | Game | High | Global | 33 | 32 |  | 255 | 18 |
| 34 | `rockets_fired_rumble` | OneShot | Game | Medium | Global | 12 | 12 |  | 255 | 4 |
| 35 | `repulser_fired_rumble` | OneShot | Game | Medium | Global | 7 | 7 |  | 255 | 4 |
| 36 | `shuriken_fired_rumble` | OneShot | Game | Medium | Global | 7 | 7 |  | 255 | 4 |
| 37 | `bomb_impact_rumble` | OneShot | Game | High | Global | 62 | 62 |  | 255 | 14 |
| 38 | `cannon_impact_rumble` | OneShot | Game | High | Global | 6 | 6 |  | 255 | 3 |
| 39 | `leech_impact_rumble` | OneShot | Game | High | Global | 51 | 14 |  | 255 | 14 |
| 40 | `mine_impact_rumble` | OneShot | Game | High | Global | 20 | 20 |  | 190 | 7 |
| 41 | `missile_impact_rumble` | OneShot | Game | High | Global | 19 | 19 |  | 255 | 13 |
| 42 | `plasma_impact_rumble` | OneShot | Game | High | Global | 30 | 30 |  | 255 | 12 |
| 43 | `quake_impact_rumble` | OneShot | Game | High | Global | 62 | 62 |  | 255 | 18 |
| 44 | `rocket_impact_rumble` | OneShot | Game | High | Global | 10 | 10 |  | 255 | 5 |
| 45 | `repulser_impact_rumble` | OneShot | Game | High | Global | 7 | 7 |  | 255 | 4 |
| 46 | `shuriken_impact_rumble` | OneShot | Game | High | Global | 7 | 7 |  | 255 | 4 |
| 47 | `leeching_energy_rumble` | Looping | Game | Medium | Global | 61 | 1 | 0..59 | 184 | 0 |
| 48 | `mine_explosion_rumble` | OneShot | Game | Medium | Positional | 18 | 18 |  | 151 | 5 |
| 49 | `missile_rebound_wall_rumble` | OneShot | Game | Medium | Positional | 18 | 18 |  | 151 | 5 |
| 50 | `bomb_explosion_rumble` | OneShot | Game | Medium | Positional | 18 | 18 |  | 224 | 5 |
| 51 | `plasma_hits_wall` | OneShot | Game | Medium | Positional | 83 | 83 |  | 255 | 31 |
| 52 | `quake_incoming_rumble` | Looping | Game | High | Positional | 17 | 17 | 0..16 | 177 | 3 |
| 53 | `rocket_hits_wall` | OneShot | Game | Medium | Positional | 18 | 18 |  | 151 | 5 |
| 54 | `repulser_out_of_range` | OneShot | Game | Medium | Global | 7 | 7 |  | 255 | 4 |
| 55 | `shuriken_hits_wall` | OneShot | Game | Medium | Global | 7 | 7 |  | 255 | 4 |
| 56 | `lockon_rumble` | OneShot | Game | Low | Global | 13 | 13 |  | 187 | 7 |
| 57 | `lock_on_warning` | OneShot | Game | High | Global | 1 | 1 |  | 0 | 0 |
| 58 | `weapon_impact_shield_rumble` | OneShot | Game | Medium | Global | 7 | 7 |  | 181 | 4 |
| 59 | `ship_exploding_rumble` | OneShot | Game | High | Positional | 106 | 103 |  | 255 | 30 |
| 60 | `ship_exploding_player_rumble` | OneShot | UI | High | Global | 106 | 103 |  | 255 | 30 |
| 61 | `photo_click` | OneShot | UI | High | Global | 1 | 1 |  | 0 | 0 |
| 62 | `side_shift_left` | OneShot | Game | Medium | Global | 33 | 31 |  | 0 | 6 |
| 63 | `side_shift_right` | OneShot | Game | Medium | Global | 9 | 1 |  | 255 | 0 |
| 64 | `heartbeat` | OneShot | UI | High | Global | 14 | 14 |  | 185 | 5 |
| - | `barrel_roll_rumble` | OneShot | Game | High | Global | 1 | 1 |  | 0 | 0 |
| - | `collision_light` | OneShot | (absent) | (absent) | Global | 1 | 1 |  | 0 | 0 |
| - | `collision_med` | OneShot | (absent) | (absent) | Global | 1 | 1 |  | 0 | 0 |
| - | `collision_other_rumble` | OneShot | (absent) | (absent) | Global | 1 | 1 |  | 0 | 0 |
| - | `plasma_charging_rumble` | OneShot | (absent) | (absent) | Global | 1 | 1 |  | 0 | 0 |

Id 11 has no file on purpose: it is the "no landing" value of the landing
selector ([gameplay/rumble.md](../gameplay/rumble.md#contact-and-landing)).
Ids 0 to 64 span 65 slots with 64 files.

## 2048 and Omega

**Ported, and read** (the reader above runs on both, with its ground truth): all 69
files of 2048's base `data.psarc` and Omega's `data00.psarc` parse, with the same census and the
same digest as HD, so the 69 patterns are equal in `<RumbleData>` on all of them. 2048's v1.04
DLC archives and Omega's patch archives hold none. Omega
reads them with the same parser, slot numbering and voice law as HD. **Checked,
differs**: 2048 ships them and has no reader, so its rumble is nothing on the
original; see the provenance table on [gameplay/rumble.md](../gameplay/rumble.md#provenance-per-title).
