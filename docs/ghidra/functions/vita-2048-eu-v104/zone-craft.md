# Zone flies one shared hull, `hdships\Zone`, for every craft

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`, Ghidra program `/2048/eboot-vita-2048-eu-v104.elf`. Recovered
2026-10-06 (`zone-craft` lane) against the maintainer's report that Omega's Zone
"does not pick the appropriate ship". The Omega half is
[`ps4-omega-eu/zone-craft.md`](../ps4-omega-eu/zone-craft.md); one source tree
compiled twice, so the two read as **one call site, not two independent ones**.

## `Ship_LoadModelSet` - `0x811b6dae`

**Confidence: 75** (static decompile; no live run - no Vita emulator in this
project's toolchain; cross-build structural match with Omega's `0x01301ba0`).

`FUN_811b6dae(ship, team_name)` builds a craft's model set (hull, LODs, wreck,
death shell, `locators.vex`, `engineflare.vex`, `leacheffect.vex`). It switches
on the game mode `DAT_8153fd24`:

| mode | directory | hull |
| --- | --- | --- |
| 6 | literal `Data\art\published\hdships\Zone` | literal `...\hdships\Zone\Ship.vex` |
| `0xd`, `0x15` | `hdships\%s_n1` (the team name) | `...\%s_n1\ship_lod.vex`, livery `livery1` |
| `0xe` | literal `hdships\Detonator` | `...\Detonator\Ship.vex` |
| anything else | the craft entry's own directory (`+0x1b0`) | `%s\ship_lod.vex` |

**Mode 6 is Zone.** `DAT_8153fd24` is the same word
[`crossfader.md`](crossfader.md) reads (`xfship_ZONE_%s.xfx` in Zone, `0xe` for
Detonator), and mode 6 alone loads `%s\Zoneship_dead\zoneship_dead_whiteshell.vex`
as its death shell - the Zone ship's wreck. Confidence 80 on the mapping.

**Mode 6 never reads the craft.** `param_2` (the team) and the craft entry are
not touched on that path, so the hull is `hdships\Zone\Ship.vex` whichever craft
the player picked and whichever roster it belongs to: HD-era (`hdships\<team>`)
and 2048-native (`Ships\<team>2048\<n>`) alike.

**What the player's pick does select is a livery.** Near the end the function
substitutes materials: in modes 6 and `0xe` the key is the literal
`zoneship_zone` (`0x81493cb4`) and the value the craft entry's livery name
(`+0x1ac`; fallback literal `Zoneship_Faisar`, `0x81493a28`); in every other mode
the key is `livery%d` and the value the same name. The per-team textures it
names ship at `hdships/Zone/Zoneship_<Team>/` (12 HD teams plus `Zoneship_Zone`,
`Zone_Glass.gxt`; `data2.psarc` of the v1.04 patch). Not wired: left open.


## The front-end preview, `0x8113c554` (unnamed, 60)

The team-select page draws `<craft dir>/ship.vex` (`0x814756b8`) and, in modes 6
and `0xe`, remaps `zoneship_zone` to the craft's livery name. So the *menu*
shows the picked craft's own hull wearing the Zone livery key, while the *race*
loads the shared hull. Hypothesis, below 70, not renamed.

## `ship_zone.vex` is not loaded by anything found (65, negative)

The 18 `Data/art/published/Ships/<team>2048/<n>/ship_zone.vex` (+ `.rcsmodel`)
files ship only in `patch-v104/PSP2/data1.psarc`. **No string `ship_zone` exists
in this executable** (`search_strings` `_zone`, `ship_`: 79 and 116 matches, none
spells it), and the one loader reading per-craft models (`0x811b6dae`) never
composes a Zone name. Three generic templates (`%s\%s.vex` `0x814e3388`,
`%s.vex` `0x81464134`, `%s%s.vex` `0x814ca6a0`) were not decompiled: a loader
that builds `ship_zone` out of a stem argument is not excluded. Treat the files
as unreferenced until one is shown to be read.

## The 2026-08-28 "no Zone ship" note

`crates/2048/src/race.rs` and `2048-status.md` recorded, from play, that Zone
flew the player's own native craft. **Checked, differs**: the loader above names
a shared hull for every craft. The play note was right that the player chooses
their ship (it chooses the livery) and wrong that the hull is theirs; the
`OwnShip` guess it replaced was wrong only in composing under the native tree -
`Data\art\published\hdships\Zone\Ship.vex` is in both the base `data.psarc` and
the patch's `data2.psarc`. Whether the base (v1.00) executable already did this
was not read; v1.04 is what is wired.

## Not checked

- ~~Livery texture swap by `zoneship_zone`~~ wired 2026-10-06: the `PI_TeamModel name="zone"` `texturelocation` replaces the path component on the hull's 9 texture requests; native teams author none, so no swap.
- Which livery name 2048-native craft carry (`+0x1ac`): the definition authors none for them and no `Zoneship_feisar2048` exists; the original's fallback is unread, so the default skin is kept.
