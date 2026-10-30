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

- **Backdrop** (row 0's widget, `0x08ab0848`): colour word at `+0xac/+0xb0/+0xb4/+0xb8` (all four
  corners) is set to **row `idx`'s word 3 verbatim** once the fade has finished
  (`0x0881d198` reads `0x08ab0850 + idx*0x18`; the store sits just after, live PC `0x0881cf9c`).
- **Fade in**: while the pickup is arriving the word is unpacked to four floats
  (`Colour_UnpackToFloat4`: `x = byte2, y = byte1, z = byte0, w = byte3`), scaled **all four
  including alpha** by `clamp(t * 3, 0, 1)` (`t` = the seconds-since-grant timer at `status+0x44`,
  so full strength after one third of a second), and repacked (`Colour_PackFromFloat4`, the same
  byte order). Nothing held: the word is `0`.
- **Icon** (row `idx`'s widget): the same scaled fade of word 2 (white), so `0xFFFFFFFF` at rest.
- **The word is `0xAARRGGBB`**: byte 2 is red. Confirmed on screen (below): the Mine's word
  `0x802B1CBC` draws **blue**, which `0xAABBGGRR` would have made red.

### The 13 values

`PlayerStatus_Update` (`0x0883b3b8`, its `switch` on `pickup+0x1bc` writing `status+0x90`) maps the held id to
the icon index; the table row order is the icon index.

| Held id (`pickup+0x1bc`) | Icon index | Weapon | Word | A | R | G | B |
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

Confidence **92** for the table and the 13 words, **88** for the function's name and the fade.

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
4. **On screen** (software renderer, `import -window root`, 2026-10-30): over open sky the Mine
   hexagon reads `(138, 141, 217)`, Shield `(134, 216, 139)`. Blending the table colour at 50 %
   with a sky of about `(230, 240, 250)` predicts `(136, 134, 219)` and `(134, 214, 139)`. Missile
   and Turbo read pink and green on the same scale.

Not measured: the **size pulse** (`+0x9c`/`+0xa0` scaled by `1/f`, `f`) on the backdrop during the
fade is read statically only, and the **PS2** binary was not searched (PSP is the reference).

## Checked on the other titles

- **HD/Fury**: **checked, differs**. `Hud_UpdatePickup` writes a fixed `0x5AFFFFFF` over
  `PickupBackground`; no per-weapon table ([hd-hud.md](../../../formats/hd-hud.md)).
- **2048 and Omega**: **not checked** in this lane (they inherit HD's law per the project rule);
  the check is a read of their `Hud_UpdatePickup` for a table like this one.
- **Pure**: **not checked**; Pure's HUD is a different layout (`crates/pure/src/hud.rs`).
