# The pickup backdrop's colour, per weapon

Recovered 2026-10-30 from `BOOT.BIN` (`pulse-psp-usa`) and verified live in PPSSPP 1.20.4
(software renderer). It closes the open question in [pickups.md](../../../gameplay/pickups.md)
("The backdrop is colour-coded per weapon"): **the colour is authored data in a 14-row widget
table, translucent (alpha `0x80`), and the three-way grouping the loading screens suggested is
exactly right.** Before this, `oag_pulse::hud::PICKUP_COLOURS` was blended screen samples.

| Address (Pulse USA) | Name | Confidence |
| --- | --- | ---: |
| `0x0881cdc4` | `Hud_UpdatePickupWidgets` | 88 |
| `0x08ab0844` | `g_hud_pickup_widget_table` (data, 14 rows of 24 bytes) | 92 |
| `0x08819644` | `Colour_UnpackToFloat4` | 85 |
| `0x088194b8` | `Colour_PackFromFloat4` | 85 |

`Hud_BindWidgets` (`0x0881fbec`, [countdown-widgets.md](countdown-widgets.md)) fills the table's
widget slots; `PlayerStatus_Update` (`0x0883b3b8`, [sub-weapon.md](sub-weapon.md)) writes the
icon index the update reads.

## The table

`0x08ab0844`, 14 rows of 6 words (`0x18` bytes). `Hud_BindWidgets` (`0x0881ff88..0x08820014`)
looks each row's name up under `HUD->` and stores the widget at word 1, then two widget
properties at words 4 and 5 (the authored scale pair, read at `+0x9c`/`+0xa0` by the pulse).

| Word | Offset | Content |
| --- | ---: | --- |
| 0 | `+0x00` | name pointer (`PickupBackground`, then `RocketIcon` ... `ShurikenIcon`) |
| 1 | `+0x04` | widget pointer, filled at bind time |
| 2 | `+0x08` | **the icon's colour**: `0xFFFFFFFF` in every row (white) |
| 3 | `+0x0c` | **the backdrop colour while this row's weapon is held**: the values below |
| 4, 5 | `+0x10`, `+0x14` | `1.0f`, `1.0f` (backdrop scale pair) |

Row 0 is the backdrop itself; rows 1-13 are the weapons.

## The law

`Hud_UpdatePickupWidgets` (`0x0881cdc4`), per frame, with `idx = *(*(status+0x3c)+0x48)`
(the icon index below; 0 = nothing held):

- **Backdrop** (row 0's widget, `0x08ab0848`): while a weapon is held the colour word at
  `+0xac/+0xb0/+0xb4/+0xb8` (all four corners) is rewritten every frame from **row `idx`'s word 3**:
  `Colour_UnpackToFloat4` (`x = byte2, y = byte1, z = byte0, w = byte3`), all four scaled **alpha
  included** by `clamp(T * 2 * 1.5, 0, 1)`, then `Colour_PackFromFloat4` (same byte order). `T` is
  `status+0x44`, which accumulates `dt * 1.2` per frame and is capped at `0.5`, so the scale is `1.0`
  (the table word verbatim) from `T = 1/3` (about 0.28 s after the grant) and stays there. The
  store is `0x0881cf9c..0x0881cfa8`, `sw s1,0xb8/0xb4/0xb0/0xac(a0)` with `a0` the backdrop widget
  and `s1` the repacked word, in the first branch of `Hud_UpdatePickupWidgets` (the `T <= 0.5`
  held branch, right after the `0x9c`/`0xa0` scale-pair stores); **live PC `0x0881cf9c`** (below).
  Nothing held: the word is stored as `0`.
- **After use** (decompile-only, not watched): the per-row timers at `status+0x48+4*i` count down and
  the backdrop keeps the last weapon's row colour (the direct-store path through
  `table[last*6+3]`) while that row's timer is above `0.1`, then fades as `(2 - 2t) * 1.5`.
- **Icon** (row `idx`'s widget, word 2, white): the same scaled form, gated on `T >= 0.4`
  (decompile-only). Hold white `0xFFFFFFFF` at rest.
- **The word is `0xAARRGGBB`**: byte 2 is red. Confirmed on screen (below): the Mine's word
  `0x802B1CBC` draws **blue**, which `0xAABBGGRR` would have made red.

### The 13 values

`PlayerStatus_Update` (`0x0883b3b8`, its `switch` on `pickup+0x1bc` writing `status+0x90`) maps the held id to
the icon index; the table row order is the icon index.

| Held id (`pickup+0x1bc`) | Icon index | Weapon (row name; Mine/Bomb labels inferred from it, not measured) | Word | A | R | G | B |
| ---: | ---: | --- | --- | ---: | ---: | ---: | ---: |
| 0 | 1 | Rocket | `0x80BE1E69` | 128 | 190 | 30 | 105 |
| 1 | 2 | Missile | `0x80BE1E69` | 128 | 190 | 30 | 105 |
| 2 | 3 | Quake | `0x80BE1E69` | 128 | 190 | 30 | 105 |
| 4 | 4 | Turbo | `0x801CBC1C` | 128 | 28 | 188 | 28 |
| 5 | 5 | Shield | `0x801CBC1C` | 128 | 28 | 188 | 28 |
| 6 | 6 | Autopilot | `0x801CBC1C` | 128 | 28 | 188 | 28 |
| 7 | 7 | Plasma | `0x80BE1E69` | 128 | 190 | 30 | 105 |
| 8 | 8 | Mine | `0x802B1CBC` | 128 | 43 | 28 | 188 |
| 9 | 9 | Bomb | `0x802B1CBC` | 128 | 43 | 28 | 188 |
| 3 | 10 | Cannon | `0x80BE1E69` | 128 | 190 | 30 | 105 |
| 10 | 11 | LeachBeam | `0x80BE1E69` | 128 | 190 | 30 | 105 |
| 11 | 12 | Repulser | `0x80BE1E69` | 128 | 190 | 30 | 105 |
| 12 | 13 | Shuriken | `0x80BE1E69` | 128 | 190 | 30 | 105 |

Three colours, the loading screens' grouping exactly (pink eight, green three, blue two). The
icon order is the order of the `*Icon` names in the table's own name column, read from the
binary, not assumed. **Alpha is `0x80` = 50.2 %** for all three; HD fixes `0x5A` ([hd-hud.md](../../../formats/hd-hud.md)).
The brighter `Turbo` frame of 2026-09-04 is that translucency over open sky, as the translucent
hypothesis in `pickups.md` guessed: Turbo has the same green as Shield and Autopilot.

## Evidence

Confidence **92** for the table and the 13 words, **88** for the function's name; the fade timing is decompile-only.

1. **Static**: the table bytes (`read_memory 0x08ab0844 +336`), the single reader of word 3
   (`0x0881cf90`/`0x0881d198`, `FUN_0881cdc4`), and `Colour_UnpackToFloat4`/`Colour_PackFromFloat4`
   (decompiled, mutually inverse).
2. **Live, all 13 held ids** (PPSSPP 1.20.4 software, `pulse-psp-usa.chd`, Talons Venom
   `race-talons-venom-alongside` state): with the CPU stopped, `craft_weapon+0x1bc` was written
   with each id 0-12, the game ran 90 frames, and the backdrop widget's four colour words
   (`widget+0xac..0xb8`, widget `0x09A275B0`) read back the table word of the mapped row for
   every id, all four equal. Nothing else contributed a value.
3. **Write watchpoint**: a halting `memory.breakpoint.add` (write) on `widget+0xac`, 16 bytes,
   stopped at PC `0x0881cf9c` (`ra 0x0881cf7c`, both in `Hud_UpdatePickupWidgets`), with a
   non-halting read watchpoint on the craft clock counting beside it as the positive control
   (4 hits). The same watchpoint armed non-halting counted 0 hits in an earlier run, so the halting
   form is the one to trust here.
4. **On screen** (software renderer, `import -window root`, 2026-10-30): the Mine hexagon over open
   sky reads `(138, 141, 217)` and Shield `(134, 216, 139)`: blue and green, so byte 2 is red.
   Blending the table colour at 50 % over a sky estimated at `(230, 240, 250)` predicts
   `(136, 134, 219)` and `(134, 214, 139)`; that is **consistent with 50 % source-over, with the
   background estimated and not measured** (Missile `(134, 78, 133)` and Turbo were sampled once
   each and not fitted). The alpha *byte* is read straight from the table and the widget (confidence
   92); the on-screen blend factor is about 70.

Not measured: the fade-in, hold and fade-out sequence (decompile only), the **size pulse** (`+0x9c`/`+0xa0` scaled by `1/f`, `f`) on the backdrop during the
fade is read statically only, and the **PS2** binary was not searched (PSP is the reference); a byte search of `SCES_547.48` for the PSP words found nothing, so PS2 stores them in another form.

## Checked on the other titles

- **HD/Fury**: **checked, differs**. `Hud_UpdatePickup` writes a fixed `0x5AFFFFFF` over
  `PickupBackground`; no per-weapon table ([hd-hud.md](../../../formats/hd-hud.md)).
- **2048 and Omega**: **not checked** in this lane (they inherit HD's law per the project rule);
  the check is a read of their `Hud_UpdatePickup` for a table like this one.
- **Pure**: **not checked**; Pure's HUD is a different layout (`crates/pure/src/hud.rs`).
