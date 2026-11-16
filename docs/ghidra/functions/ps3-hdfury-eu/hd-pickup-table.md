# HD's pickup widget table is Pulse's

Static read of `EBOOT.elf` (HD/Fury EU, `BCES-00664`) on 2026-10-30, lane `pickup-fade-titles`. The
caller's brief said HD "fixes `0x5AFFFFFF`"; that is `Hud_UpdatePickupBar`'s nothing-held value only
([hud-readouts.md](hud-readouts.md), "The pickup bar's idle dim"), and the held path takes a per-weapon
table. This page reads the table.

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x00920598` | `g_hud_pickup_widget_table` (data, what the TOC slot `0x008a7800` points at) | 90 |

## The table

`PTR_008a7800 + hud * 0x150` (`hud` 0 and 1 read; both identical) is 14 rows of 6 big-endian words,
the row shape of Pulse's `g_hud_pickup_widget_table`
([pickup-colour.md](../psp-pulse-usa/pickup-colour.md)): name pointer, widget slot (zero until bound),
the icon word, the backdrop word, two `1.0f`. Row 0 (name at `0x0077f598`) has both words
`0xFFFFFFFF`; rows 1-13 have the icon word `0xFFFFFFFF` and the backdrop word:

| Rows | Word |
| --- | --- |
| 1, 2, 3, 7, 10, 11, 12, 13 | `0x80BE1E69` |
| 4, 5, 6 | `0x801CBC1C` |
| 8, 9 | `0x802B1CBC` |

Byte for byte the thirteen words of Pulse's table, alpha `0x80` included. Read with the bridge's
`read_memory` at `0x00920598` for `0x150 * 2` bytes (16 bytes per call).

`Hud_UpdatePickupBar`'s tail already walks `PTR_008a7800 + hud*0x150 + row*0x18` ("an ARGB pair at
`+8`/`+0xc`") and scales the row colour by an eased per-row timer; the two easing helpers are
`0x00676fe8` (a wrapper over `_FSin`) and `0x00677528` (a wrapper over `expf`), the same pair Pulse's
`Ease_CosineWithBump` is built from.

## The fade constants (read 2026-10-30, `pickup-fade-wire`)

Static, from the decompile of `0x0008df30` and the floats it names (bridge `read_memory`):

| Float | Value | Use | Pulse |
| --- | --- | --- | --- |
| `0x008a776c` | `0x3fc00000` = **1.5** | `step = dt * 1.5`, the first line of the update | **1.2** |
| `0x008a764c` | `0.5` | the cap `T` stops at while held | same |
| `0x008a7768` | `3.0` | `T * 3` into the cosine step | same |
| `0x008a76bc` | `pi` | the cosine step's argument scale | same |
| `0x008a780c` | `-25.0` | the bump's `-25 (u - 0.5)^2`, next to the table's TOC slot | same |

So **HD's rate is 1.5, not Pulse's 1.2**, and the shape is the same (cap `0.5`, `x3`, cosine step,
`exp(-25 ...)` bump). Confidence **80** (static; not run live, RPCS3 has no memory trace of it here).
The 2048 update uses the same `1.5`. The rest was inherited from Pulse until the live read below.

## The rest of the update, read and followed live (2026-11-04, `pickup-backdrop-closeout`)

Floats through the TOC base `0x008ad4d8` (`0x008ad4d8 - 0x5e8c = 0x008a764c`, the `0.5`), read with the
bridge's `read_memory`, then a live RPCS3 trace of the HUD object (`0x32984ea0` on both boots; its
player record at `+0x40`, held row `*(record + 0x58)`, `T` at `+0x4c`, the per-row timers at `+0x50 + 4 (row - 1)`,
the last row at `+0x168`), sampled at about 250 Hz through `/proc/<pid>/mem` without pausing, the slot's
state word written (`*(ship + 0x5edc) + 0x204`, state 1 then 9) and fired with TRIANGLE:

| Item | Static | Live (two boots, states 1 and 9) | Confidence |
| --- | --- | --- | ---: |
| step | `dt * 1.5` | `T` gains `0.0249` to `0.0250` an update (a 60 Hz `dt`) | 90 |
| `T` while held | cap `0.5` | rises from the grant to `0.5` and stays | 90 |
| icon gate | `-0x5d94` = `0x008a7744` = `0x3ecccccd` = `0.4` | the held row's timer starts the update `T` first reads at or above `0.4` (`0.4081`, `0.4176`), gains one step each update, caps at `0.5` | 90 |
| after use | held row 0 | `T` takes one more step at once (`0.5244`, `0.5250`), the last row (`+0x168`) is set, the row timer counts down at the step | 90 |
| hold threshold | `-0x5e10` = `0x008a76c8` = `0x3dcccccd` = `0.1` | `T` stays at `0.5244` while the row timer is above `0.1` (`0.1248` holds); at `0.0998` / `0.1000` it runs on | 90 |
| fade-out | `(T * -2 + 2) * 1.5` into the sine step and the bump (`0x008a7814` = `-2.0`, `0x008a762c` = `2.0`) | `T` runs `0.5244` to `1.0` in 19 updates, then is `0` and the last row clears | 85 |
| backdrop size | `iVar6` (the backdrop image) is used only as `Image_SetVertexColours`'s argument | not traced | 80 |

So HD's fade-in, hold, icon gate and fade-out are Pulse's, with rate `1.5`, and HD **folds no size**:
`oag_hud::pickup_fade` already models all of it (`size_pulse: false`). The one difference left is the
extra step on the first update after use (Pulse has it too, `0.5201`): `Fade::advance` does not take it,
so ours' run-on starts one update (about 17 ms) later. Not wired: a one-update shift of a fade that is
`1.0` at both ends of the shift (`Ease_CosineWithBump` is `1` from `x = 1.5` up) cannot be seen.

## Consequence

`docs/formats/hd-hud.md`'s line that HD fixes `0x5AFFFFFF` over `PickupBackground` is the idle path.
With a weapon held HD's hexagon takes the table word, scaled by the eased timer: **ported, wired
2026-10-30** (`oag_hud::runtime::colour` for the always-on `PickupBackground`, the icon through
`pickup_sprites`; the rate is `oag_hd::hud::PICKUP_FADE_RATE_PERCENT`). Headless HD frames of Missile
(pink), Turbo (green) and Mine (blue) at `--give`, ticks 8 and 10 (the overshoot) and 60 (settled), read
2026-10-30. Omega carries the same table
([omega-pickup-table.md](../ps4-omega-eu/omega-pickup-table.md)).
