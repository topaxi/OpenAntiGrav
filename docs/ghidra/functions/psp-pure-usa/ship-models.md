# Pure's five per-craft model templates, and the boost plume it does not have

**Binary:** `pure-psp-usa` `BOOT.BIN`, image base `0x08804000`; every address
below re-checked on `pure-psp-eu` and identical string for string.
**Status:** string and data recovery. **No function was renamed and no
`names.tsv` row belongs to this page** - a WAD entry name is not a Ghidra
symbol, and writes to `psp-pure-*` are blocked through the current bridge
anyway.

Settles the last unrecovered Pure entry name this project was carrying - the
boost plume, recorded since 2026-08-12 as needing "a *different* name than
Pulse's own". It needs no name. It has no file. See
[`race.rs`](../../../../crates/pure/src/race.rs)'s `ships` module, which is
where the retraction lives.

## Technique, and what was deliberately not used

**`get_xrefs_to` was never consulted.** Ghidra applies no PSP relocation on
either Pure database, so an empty cross-reference list there is a known defect
at confidence 92 and proves nothing at all about what references a target.
Everything here comes from three techniques that are unaffected by it:

1. `search_strings` over the program, and `read_memory` at the resulting
   addresses to read the `.rodata` block byte for byte rather than trusting a
   listing's own string splitting.
2. A CRC name-hash match of each composed path against the archive directory,
   using `oag_formats::wad::hash_name` - the same hash the game's own
   `Wad_HashName` implements.
3. A byte search of each returned blob for the exporter's own source path, so
   the hash match is corroborated by the payload instead of by itself.

The hash pipeline was positive-controlled before any of the negatives below
were trusted: `Data\Ships\Zone_01\handlingstats.xml`, `Data\Zone\01_Zone\track.vex`
and `Data\Plugins\PI001\Definition.xml` all hash to real entries in Pure's
`Data.wad`. Without that control, "none of these hash to anything" would have
been indistinguishable from a broken normalisation.

## The block: `0x08a7a5d4` .. `0x08a7a62c`

Five NUL-terminated templates, contiguous, inside the run of strings belonging
to `c:/Work/Wipeout/Code/Backend/Ships/Ship.cpp` (`0x08a7a4d4`). Read with
`read_memory 0x08a7a5d4 80`:

```text
08a7a5d4  25 73 5c 56 52 5c 53 68 69 70 2e 76 65 78 00 00   %s\VR\Ship.vex..
08a7a5e4  25 73 5c 50 68 61 6e 74 6f 6d 2e 76 65 78 00 00   %s\Phantom.vex..
08a7a5f4  25 73 5c 53 68 69 70 2e 76 65 78 00               %s\Ship.vex.
08a7a600  25 73 5c 50 68 61 6e 74 6f 6d 5f 73 68 69 70 77   %s\Phantom_shipw
08a7a610  72 65 63 6b 2e 76 65 78 00 00 00 00               reck.vex....
08a7a61c  25 73 5c 53 68 69 70 77 ...                       %s\Shipw[reck.vex]
```

Three of the five recur in a second block, `GhostShip.cpp`'s own run at
`0x08a79d68`: `%s\VR\Ship.vex` at `0x08a79d9c`, `%s\Phantom.vex` at `0x08a79dac`
and `%s\Ship.vex` at `0x08a79dbc`, with `Auto Load Ghost` (`0x08a79dc8`)
immediately after them. A third copy of `%s\Phantom.vex` sits at `0x08a7cb54`,
and a sixth template, `%s\VR\Phantom.vex`, at `0x08a7cb64` - that one resolves
to nothing, see below.

The `%s` is a `<PI_Team>` `location` from `Data\Plugins\PI001\Definition.xml`,
not a bare team name: the disc declares eleven, and two of them (`Data\Ships\Zone`,
`Data\Ships\Zone_01`) are not teams in the racing sense at all.

## What resolves, and where the misses are the finding

Composed against all eleven declared locations, on both pressings. `HIT` means
the name hashes to a real `Data.wad` entry.

| Location | `Ship` | `Shipwreck` | `Phantom` | `Phantom_shipwreck` | `VR\Ship` | `VR\Phantom` |
| --- | --- | --- | --- | --- | --- | --- |
| `AG_Systems` | HIT | HIT | HIT | HIT | HIT | - |
| `Assegai` | HIT | HIT | HIT | HIT | HIT | - |
| `Auricom` | HIT | HIT | HIT | HIT | HIT | - |
| `Feisar` | HIT | HIT | HIT | HIT | HIT | - |
| `Harimau` | HIT | HIT | HIT | HIT | HIT | - |
| `Piranha` | HIT | HIT | HIT | HIT | HIT | - |
| `Qirex` | HIT | HIT | HIT | HIT | HIT | - |
| `Triakis` | HIT | HIT | HIT | HIT | HIT | - |
| `Medievil` | HIT | HIT | - | - | HIT | - |
| `Zone` | HIT | HIT | - | - | HIT | - |
| `Zone_01` | HIT | HIT | - | - | - | - |

So a **Phantom-class variant exists for the eight core racing teams and for
nothing else**, and the Zone-mode craft (`Zone_01`, the one the definition marks
`type="Zone"`) is the only location with no VR hull. Neither split was guessed:
`crates/pure/tests/ship_models_ground_truth.rs` asserts the whole matrix,
misses included, on both discs.

**Confidence 96 for the five names.** Each blob carries its own exporter source
path - `Z:/Data/Ships/Feisar/Phantom.mb` inside `Data\Ships\Feisar\Phantom.vex`,
`Z:/Data/Ships/Feisar/VR/Ship.mb` inside `Data\Ships\Feisar\VR\Ship.vex` - so
the archive answered a name this project composed and handed back a payload that
spells the same team and the same leaf in Maya's words. A CRC collision or a
mis-split template would not survive that.

### `Phantom` is the speed class, and what selects the model is now partly read

**Confidence 90 for the identification** (unchanged). The call site is now
found, on both pressings, unblocked by the 2026-09-07 PSP relocation patch: a
direct `get_xrefs_to 0x08a7a5e4` (`psp-pure-usa`) - never usable before that
patch, see "Technique" above - returns exactly one caller, `FUN_08927dac`
(`psp-pure-eu`'s structural counterpart, confirmed field-offset-for-field-offset
identical: `FUN_08927694`). Both are early-exit on an unchanged team
(`*(param_1+0x8b8) == param_2 -> return`), release the live model and wreck
instances, then rebuild both - this is the craft's "switch active team" /
model-(re)load function, not a per-frame update. **Not renamed** - see "Do not
rename yet" below.

`PHANTOM` is the top rung of the five-rung ladder Pure's `handlingstats.xml`
files author (`docs/formats/pure-status.md`), and the executable carries
`PhantomStats` (`0x08a79afc`), `PhantomTweak` (`0x08a56a7c`) and
`Unlock Phantom Class` (`0x08a7c848`). The models are a separate paint set, not
a re-skin: `Data\Ships\Feisar\Textures\feis_phantom1_shinemap.tga` through
`feis_phantom3_shinemap.tga` plus `feis_phantom_lod.tga`, against the base
hull's `feis_01..03_shinemap.tga` and `lod1.tga`.

**The gate is not a class comparison - correct that assumption, don't just
retire it.** `FUN_08927dac`'s decompile (both pressings agree instruction for
instruction, modulo relocated addresses):

```c
cVar6 = DAT_08b1742c;                              // default: off
if ((8 < DAT_08b173e4) &&                          // field +0x1c of the
    (cVar6 = '\0', DAT_08b174fc != 0)) {            // global struct at
  iVar5 = FUN_08804bf8(&DAT_08b173c8,               // DAT_08b173c8, > 8
                        *(int *)(param_1 + 0x480));
  if (*(char *)(DAT_08b174fc + iVar5 + 0x90) == 1) {
    cVar6 = '\x01';                                 // -> Phantom.vex
  }
}
```

So the Phantom swap reads one precomputed flag byte, at `DAT_08b174fc + iVar5
+ 0x90`, where `iVar5` comes from a small lookup helper
(`FUN_08804bf8(base, idx) = *(base + idx*4 + 0x230)`) keyed on the craft's own
`+0x480` field (set from the Ship constructor's third argument, `FUN_089261ac`
(`psp-pure-usa`) / its EU counterpart). **The byte could still be written from
the selected speed class upstream** - this decompile only proves the *read*
side, not that the write side ignores class. Don't write "not class-keyed"
without reading a writer.

**`DAT_08b174fc` is populated by network session setup, which reframes the
question.** `get_xrefs_to 0x08b174fc` (`psp-pure-usa`) returns four writers;
the first read, `FUN_0889aad8`, is short enough to be conclusive on its own:

```c
FUN_08901a6c(&DAT_08b5c6c0, 1, 2, 0, 0, 30000, 0, auStack_bc, "NetGlobals", DAT_08b5c6bc);
FUN_08901c54(&local_c0, DAT_08b5c6c0);
DAT_08b174fc = *(undefined4 *)(local_c0 + 0x1c);
```

`"NetGlobals"` is a literal argument to what reads as a networked/replicated
object registration call. `DAT_08b174fc` is non-null only once that object
exists, and `DAT_08b173e4` (the same struct's `+0x1c` field, thresholded `> 8`
here) is compared elsewhere against 5, 6, 7 and `0xb` (11) in
`FUN_0892195c` - too many distinct values for a boolean, reads as a mode/session
kind enum, not a class count. **Reading together: the Phantom swap plausibly
only ever fires in a network (Link) session**, gated on both "session kind
`> 8`" and "the NetGlobals object is live." Not proven - the three remaining
writers (`FUN_0889ac74`, `FUN_0889aecc`, `FUN_0889dcec`) are unread, and
`DAT_08b173e4`'s actual enum values are unnamed. Do not act on this beyond
"probably networking," and do not name `DAT_08b173e4`/`DAT_08b174fc`/
`DAT_08b173c8` off it.

**`FUN_0892195c`'s own call sites to `FUN_08927dac` are a next/prev craft
cycler reading `strcasecmp` against a list, gated on `DAT_08b173e0` walking
0..4** - reads like a dev-only team browser, not shipping flow, which (if
right) makes the Ship constructor the only *shipping* caller and the flag at
`+0x90` something prepared before a race starts, not computed live.
Hypothesis, not confirmed - noted so it isn't re-derived from scratch.

### Do not rename yet

`FUN_08927dac`/`FUN_08927694` are not in `names.tsv`. The read above is solid
(structurally identical on both pressings, matches every known template
address and offset), but "what writes `+0x90`" is still open, and a name like
`Ship_LoadModelsForTeam` would want that closed first rather than guessed at
`_q` confidence. Also: **do not call a Ghidra rename tool on `psp-pure-usa`
right now without first switching to it** - `list_open_programs`'s
`is_current` can point at an unrelated program (`ps3-hdfury-eu` did, this
session), and `rename_function_by_address`/`create_function` silently act on
whichever program the bridge considers active regardless of the `program`
field passed - see HANDOVER.md, "The Ghidra bridge's `switch_program` can
silently no-op." Land a `names.tsv` row and apply through `just apply-names`
instead of a live rename.

### `VR` is a whole alternate presentation set, and its meaning is not determined

The prefix recurs across the disc: `Data\Weapons\vr_bomb.vex`, `vr_mine.vex`,
`vr_shield.vex`, `vr_shield_cockpit.vex`, `Data\Tex\EngineFlare\vr_engine_noise.mip`,
`Data\Tex\vr_env.tga`, and a per-team `Data\Ships\<Team>\VR\Textures\vr_<team>_01_shinemap.tga`.
Pulse carries exactly one member of that set (`%s\vr_shield_cockpit.vex`) and no
`VR\` craft directory at all. The name is recovered; **what mode uses it is
not**, and no path that selects it has been read.

### `%s\VR\Phantom.vex` (`0x08a7cb64`) resolves nowhere, and there are two readings

The template is in the binary, and composed with a `<PI_Team>` `location` - the
argument the five above are known to take - it hashes to nothing, against
`Data.wad`, `FE.wad` and `FEData.wad`, on both pressings.

**"Authored and cut from the shipped disc" is one reading, and it is not the
only one.** This template sits in a *different* `.rodata` block from the five -
`0x08a7cb54`/`0x08a7cb64`, beside `Unlock Phantom Class` (`0x08a7c848`) - and
the five are known to take a team location only *because they resolve*.
**Whether this block's `%s` is a team location at all is unread**, so "composed
with the wrong argument here" is not excluded. `search_instructions` for the
`lui`/`addiu` pair building `0x08a7cb64` would settle it and has not been run.

Recorded either way, so the next probe of that name finds the answer instead of
re-deriving it. `crates/pure/src/race::ships::VR_PHANTOM_HULL_UNSHIPPED` carries
the same caveat; nothing reads it.

## Pure ships no boost-plume asset

**Confidence 93.** Four probes across three independent sources - the
executable, the archive directory and the model payloads - each with its Pulse
half alongside, because a negative with no positive control beside it is not a
measurement.

| Axis | Pulse | Pure |
| --- | --- | --- |
| Path template | `%s\%sboost.vex` at `0x08a84ccc` (`psp-pulse-usa`) | no counterpart; the five above are the whole `Ship.cpp` block |
| Any `boost` string at all | several, including the template | **two**, neither a path: `HUD_Perfect Boost!` (`0x08a46398`) and `StartBoostSpeed` (`0x08a79bc8`) |
| Archive entry | `Data\Ships\Assegai\shipboost.vex` resolves | `<location>\shipboost.vex` resolves for none of the eleven, either pressing |
| Anchor node in the hull | `Data\Ships\Feisar\Ship.vex` carries `boost_flare` and `boost_flare1` ([exhaust.md](../psp-pulse-usa/exhaust.md)) | no Pure hull carries a node naming `boost`; the flare anchor is `engine_flare` and there is one |

**The DLC packs do not weaken the archive row, because the executable row makes
them moot.** Pure ships seven decrypted PSN packs (`docs/formats/dlc-pack.md`)
and `Data\Ships\Vanuber\Ship.vex` does resolve inside one, so "not in `Data.wad`"
would be an incomplete argument on its own. It is not the argument: with no
template of that shape in the binary there is **no code path to compose a plume
name at all**, whatever a pack happens to carry.

`StartBoostSpeed` sits in the AI tuning-key block, between `Zone`/`Position`/
`SpeedPercent` and `LeadZone` (`0x08a79bfc`), `TailZone`, `AIThrust`,
`SpreadDist` and `NormalRaceSpeedMin`/`Max` (`0x08a79c98`) - so it is a
start-line speed on an AI stat sheet, not a pickup. `HUD_Perfect Boost!` is a Zone-mode
HUD message, beside `HUD_Perfect Zone!` and `HUD_New Zone Record`.

### Not determined: whether Pure's boost is visually inert

The *asset* is absent. The *effect* need not be. Pure carries
`Data\Tex\EngineFlare\grabbedEngineFlare128x64x8.mip` and
`Data\Tex\engineFlare\Engine_noise.mip` - the same two literals Pulse's binary
names, at `0x08a84c80` and `0x08a889e4` there - plus a Pure-only
`Data\Tex\EngineFlare\vr_engine_noise.mip`, and all three resolve on both Pure
pressings. A boost that brightens, widens or recolours the existing
`engine_flare` billboard in code would leave nothing for any of the four rows
above to find.

The flare's own draw path has not been read on Pure. What is settled is only
that **there is no file to look for**, so nothing downstream should keep
reporting a Pure boost model as missing.

### Not determined: what `WO_SHIP_ENGINEFLARE` is doing in one hull

`Data\Ships\Feisar\Ship.vex`, `\Phantom.vex` and `\Phantom_shipwreck.vex` carry a
`Name` attribute whose value is `WO_SHIP_ENGINEFLARE`, and
`Data\Psys\WO_SHIP_ENGINEFLARE.POB` **resolves to nothing** on either pressing -
where all 26 `Data\Psys\WO_*.POB` literals the binary itself lists resolve on
both. No other team's
hull carries it. Whether that is an authoring leftover or a name the code
rewrites before asking the archive is unread; it is recorded here because a
later reader hunting Pure's flare will land on it and should not take it for a
live reference.
