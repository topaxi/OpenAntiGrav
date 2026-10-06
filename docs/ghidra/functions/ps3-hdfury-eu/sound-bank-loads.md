# Which `.bnk` files Wipeout HD's executable names, and who loads them

Wipeout HD / Fury, PS3 EU (`/hdfury/EBOOT-ps3-hdfury-eu.elf`, image base `0`).
**No function is renamed on this page**: the lane that read it was scoped to the
sound-bank question, and the addresses below stay `FUN_` / Ghidra's own names.
It exists so `oag_hd::race::SOUND_BANKS.track.shared` has evidence beside it.

## The literals

`search_strings` for `.bnk` finds 19 literals, all `Data\Sound\<name>.bnk`, in
two clusters. None names a circuit bank: those come from each circuit's
`trackstartup.xml` `<LoadSoundBank Filename>` and sit beside the track.

| Cluster | Literals (address) |
| --- | --- |
| race banks, `0x00780320..0x00780648` | `env0_det` (`780320`), `env0_zone` (`780338`), `weapons`, `speech_elim`, `weapons_nitro`, `speech_zbattle`, `ShipHD` (`7805b8`), `weapons_det`, `speech_det`, `speech_zone`, `speech` (`780630`), **`crowd`** (`780648`) |
| boot banks, `0x007a3af0..0x007a3ba8` | `frontend`, `speech_results`, **`generaltrack`** (`7a3b28`), **`voppler`** (`7a3b48`), **`speech_PreRaceChatter`** (`7a3b60`), `frontend_fliers`, `speech_fe` |

## `0x00301338`: the boot loader (confidence 85)

`.opd.FUN_00301338` is called from `SoundManager_Construct` (at `0x00301624`)
and from `0x0030185c`. It calls `FUN_00301038(param_1, <path>)` once per bank
and stores each result in a table at `PTR_DAT_008b4b54 + 0xc4..0xd4`, in this
order: `frontend.bnk`, `speech_results.bnk`, **`generaltrack.bnk`**,
**`voppler.bnk`**, **`speech_PreRaceChatter.bnk`**. They are loaded when the
sound manager is built, so they are resident for the whole session, a race
included.

The path literals are reached through a pointer table (`0x008b4bd4..0x008b4be4`),
not inline, which is why the cross-reference to `generaltrack` reads `[DATA]`
at `0x008b4bdc`.

## `0x003f1f84`: `crowd.bnk` in a race-setup function (confidence 65)

`crowd.bnk`'s only code reference is a read at `0x003f1f80` (`lwz r28,
-0x5380(r2)`, the pointer-table entry at `0x008a8158`) inside the large function
Ghidra names `Shader_InitEngineParams` (`0x003f1300..0x003f25b7`). That name is
almost certainly wrong for what this region does: the pointer table at
`0x008a8120` holds the race-bank literals above in address order, and the region
loads each through `FUN_005a2090`/`FUN_005d4cf8`. The function is not renamed
here (below 70 and out of the lane's scope).

The lower score is deliberate: the table is the **per-mode** bank list
(`env0_det`, `env0_zone`, `speech_elim`, `speech_zbattle` sit beside `crowd`), and
which entries an ordinary race loads was not followed. `crowd` is in the shared
list because the circuit nodes spell it and a bank of that label exists nowhere
else, not because the mode condition was read.

## What it fixes

The five banks are `shared` in `SOUND_BANKS.track`, in `Data\Sound\`:
`generaltrack` (label `gentrak`), `voppler` (`voppler`), `crowd` (`crowd`),
`speech_preracechatter` (`radios`) and `shiphd` (`shipHD`, loaded as the ship
bank). `docs/formats/hd-audio.md` has the per-circuit result.
