# 2048's race start: two cue families, chosen by the mode id

`/2048/eboot-vita-2048-eu-v104.elf`, read statically (decompile and disassembly), 2026-10-19, lane
`race-countdown-audio`. **No function is renamed here**: the two start functions are read at branch level
and stay `FUN_`. Nothing here was captured live.

| Address | Reading | Confidence |
| --- | --- | --- |
| `0x811743e4` | The race-state setter (the sibling of Pulse's `RaceMode_SetState`). State 1 starts the countdown | 80 |
| `0x81175d12` | The countdown's end: runs when the whole seconds left reach zero (`(int)max(0, +0x11b44 - +0x11d48) == 0`), the same test as Pulse's `RaceMode_UpdateCountdown` | 80 |
| `0x81000930` | `GameMode_IsHdLineage`, already on [ships-effects.md](ships-effects.md): mode id `< 0x17` | 85 |

## What plays

State 1 (`0x811743e4`), when not in the `FUN_810016e8` case (the id is `2`, or the mode object's vtable `+0x60` says so):

| Mode id | Cue | Bank handle | Also |
| --- | --- | --- | --- |
| `< 0x17` (a named mode: Arcade, Zone, Elimination, ...) | `"ready"` | `DAT_818c4df4`, the race's speech bank | `"HBEAT"` from the same bank |
| `>= 0x17` (a CRC id: **2048's own campaign and multiplayer events**) | `"321_NGP"` | `DAT_818c4df0` | `"RACE_Combat_Start"` from the speech bank when `FUN_8100193c` (ids 7, 8, 0x14, or the combat flags) holds |

The end (`0x81175d12`), same split: `< 0x17` plays the name at `0x814820b0`, which reads **`"go"`**, from the
speech bank; `>= 0x17` plays `"Go_NGP"` from `DAT_818c4df0`, then `"Ship_Gear_up_PirProto"` when `FUN_8100188e`
holds. Priority argument `2` on all four.

[ships-effects.md](ships-effects.md) shows every named mode is below `0x17` and that none of the 577
instance names in `SP.xml` and `MP.xml` is in the table, so **2048's own events take the `321_NGP` / `Go_NGP`
side**, and only a free-play named mode takes `ready` / `go`.

## What the banks author

Names are FNV-hashed (`Bank::is_hashed`), resolved with `oag_formats::sblk::cue::name_hash`
(`crates/formats/examples/bank_name_probe.rs`).

| Cue | Bank | Cue / commands | Body |
| --- | --- | --- | --- |
| `ready` | `speech.bnk` | 21, cmds 59..61 | two key-ons (a stereo pair), no delay |
| `go` | `speech.bnk` | 24, cmds 65..71 | a goto into its marker: the pair at cmds 68-69 |
| `321_GO` | `speech.bnk` | 27, cmds 75..90 | three stages on delays 125, 205, 205 ticks of the 240 Hz clock, each followed by a `0x1e` register write (`0x1fc`, `0x2fc`, `0x3fc`) and a final `0x1a` random wait. Nothing in the executable names `321_GO` |
| `321_NGP` | `Weapons_NGP.bnk` | 2, cmds 7..11 | opcode `0x04` (an LFO), key-ons at delays `0x96` and `5` (volume 80), then `0x29` with `0x384` |
| `Go_NGP` | `Weapons_NGP.bnk` | 44, cmds 356..370 | a `0x2e` switch and four marker arms (1, 4, 3, 2), each one key-on (volume 110) ending in a goto to marker `0x63` |
| `ready` / `go` / `321_GO` | `speech_det.bnk` | 0 / 2 / 1 | the Detonator build of the same trio |

## What the port plays (2048)

Nothing for the start: `ready` and `go` resolve by hash only (`cue_named_or_hashed`), and the countdown
loader uses `cue_named`, which finds nothing on a hashed bank (`sfx: ready not loaded: "ready" names no cue
in SPEECH`). **Left unwired on purpose**: wiring the hashed `ready`/`go` would play the named-mode pair on
2048's own events, which the original voices with `321_NGP` / `Go_NGP`. Wiring those needs two things not
done: opcode `0x04` (the LFO in `321_NGP`) and `0x2e` (`Go_NGP`'s switch and which arm the call selects)
are unread, so a timeline would be a guess.

## Omega

Checked, not checkable: Omega's `speech*.bnk` do not parse as `SBlk` banks with this project's reader
(`Bank::parse` fails), so neither the `ready`/`go` pair nor a `321_NGP` can be looked up. Omega re-ships
2048's circuits and the HD front end; its executable was not read for the start.
