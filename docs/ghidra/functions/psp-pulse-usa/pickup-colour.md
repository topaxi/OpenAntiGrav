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
| `0x08819374` | `Ease_Cosine` | 90 |
| `0x088193ec` | `Ease_GaussianBump` | 90 |
| `0x08819458` | `Ease_CosineWithBump` | 90 |

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
  included** by `Ease_CosineWithBump(T * 2 * 1.5, 1.0)`, then `Colour_PackFromFloat4` (same byte order,
  each channel truncated and clamped to 255). **This is not `clamp(3T)`** (the decompile's
  `FUN_08819458` was read as a clamp until 2026-10-30): see "The easing" below. `T` is `status+0x44`,
  `dt * 1.2` per frame, capped at `0.5` while held, so the curve runs from `x = 0` to `x = 1.5` over
  0.417 s. The store is `0x0881cf9c..0x0881cfa8`, `sw s1,0xb8/0xb4/0xb0/0xac(a0)` with `a0` the backdrop
  widget and `s1` the repacked word, in the first branch of `Hud_UpdatePickupWidgets` (the `T <= 0.5`
  held branch, right after the `0x9c`/`0xa0` size stores); **live PC `0x0881cf9c`** (below).
  Nothing held and `T == 0`: the word is stored as `0`.
- **Size pulse** (same widget, same branch): `f = Ease_CosineWithBump(T * 2 * 0.99 + 0.01, 0)` (the
  second argument `0` drops the bump), then `+0x9c = width * (1/f)` and `+0xa0 = height * f`, where
  `width`/`height` are table row 0's words 4 and 5 as `Hud_BindWidgets` filled them from the layout
  (66.0 and 60.0 on Talon's Junction's HUD; the table's static `1.0f` pair is only the pre-bind
  placeholder). At the grant the hexagon is a hair-thin 267495 x 0.015 strip (`f = 0.000247`) that
  folds out to 66 x 60 as `T` reaches 0.5 (`f = 1`); after use the same curve plays backwards.
- **After use** (idx back to 0), live: `T` is not reset, it keeps counting from `0.5` (`+dt*1.2` per
  frame; the first idx-0 frame adds `dt` once more, `0.5201`), while the per-row timer at
  `status+0x44+4*row` (`status+0x48+4*(row-1)`) counts **down** at `dt * 1.2` from its cap `0.5` and
  `status+0x170` remembers the last row. The backdrop **holds the last weapon's table word verbatim**
  (`0x0881d198` store, no scaling) while that row's timer is above `0.1`; then `T` runs on from there
  and the colour is `Ease_CosineWithBump((2 - 2T) * 1.5, 1.0)` times the word, with the size pulse on
  `Ease_CosineWithBump((2 - 2T) * 0.99 + 0.01, 0)`, until `T > 1.0` clamps to `1.0`, clears
  `status+0x170`, and the next frame resets `T` to `0` (colour word `0`). From a fully grown row
  (timer `0.5`) that is `0.4 / 1.2 = 0.333` s of hold plus `0.5 / 1.2 = 0.417` s of fade-out. The
  fade-out replays the grant's curve in reverse, overshoot included.
- **Icon** (row `idx`'s widget, word 2, white): gated on `T >= 0.4`, which is **0.333 s after the
  grant** (live: the first non-zero icon word is the sample whose `T` is `0.4011`). Then the icon's own
  row timer grows from `0` at `dt * 1.2` (cap `0.5`) and the icon word is white scaled by
  `Ease_CosineWithBump(timer * 3, 1.0)` (`0x02020202` at timer `0.0208`, `0xFFFFFFFF` from timer
  `0.22`; a bump past 1.0 is clamped). After use the same timer counts down and the icon fades out
  through the same curve (white until timer `0.2`, gone at `0.02`) **while the backdrop is still
  holding**. Rest state is white `0xFFFFFFFF`.
- **The word is `0xAARRGGBB`**: byte 2 is red. Confirmed on screen (below): the Mine's word
  `0x802B1CBC` draws **blue**, which `0xAABBGGRR` would have made red.

### The easing

`FUN_08819458(x, k)` is the shaper every fade above goes through (three tiny functions, decompiled and
fitted against 29 live backdrop words, the whole fade-out, 40 icon words and every size word):

```
Ease_Cosine(x)       = 0                       x < 0
                       1                       x > 1
                       (1 - cos(pi * x)) / 2   otherwise                 (0x08819374)
Ease_GaussianBump(x) = exp(-25 * (u - 0.5)^2)  with u = x, 0 <= u <= 1
                       0                       otherwise                 (0x088193ec)
Ease_CosineWithBump(x, k) = Ease_Cosine(x) + k * Ease_GaussianBump((x - 0.5) * 2)   (0x08819458)
```

With `k = 1` the curve climbs to about `1.85` at `x = 0.75` (a bump centred there, `u = 0.5`) and
settles to exactly `1.0` by `x = 1.5`: the backdrop **overshoots to roughly twice the table's
alpha and brightness a fifth of a second after the grant, then relaxes onto the table word**. The
pack clamps channels to 255, so Missile's pink saturates green at `T = 0.24`. Live words, Missile
row (`0x80BE1E69`), `T` at the sampled call: `0.0401 -> 0x04060103`, `0.1602 -> 0x3C590E31`,
`0.2402 -> 0xDEFF34B6`, `0.2603 -> 0xE5FF35BC` (the peak: alpha `0xE5` against `0x80`),
`0.3003 -> 0x8ACC2071`, `0.3403 -> 0x80BE1E69` (settled).

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

Confidence **92** for the table and the 13 words, **88** for the function's name, **90** for the fade law (step 5).

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

5. **The fade, live (2026-10-30, `pickup-fade-titles`, PPSSPP 1.20.4 software, `pulse-psp-usa.chd`,
   the same `race-talons-venom-alongside` state)**: a halting exec breakpoint at `0x0881cdc4`
   (one sample per HUD update, `a0` = the status block, `f12` = `dt`), reading `T`, the held icon index,
   `status+0x170`, the 13 row timers, the backdrop word `widget+0xac`, the size words `+0x9c`/`+0xa0` and
   every icon widget's word. The state's Autopilot running bit (`pickup+0x1b8 & 0x800`) was cleared and
   `pickup+0x1bc` set to `-1` until `T` and every timer were `0` (a real clean slate: a bare switch
   from one weapon to another leaves `T` at `0.5` and shows no fade-in), then `pickup+0x1bc = 1`
   (Missile, row 2) for 40 updates, then `-1` again. Results, measured `dt` `0.0160..0.0175` s:
   - `T` grows `0.0200..0.0202` per update (`dt * 1.2`) from `0` and stops at `0.5` on the 26th
     update (0.43 s); the 29 backdrop words from `T = 0` to `0.5` equal the `Ease_CosineWithBump`
     prediction (channels truncated) to the byte on all 29;
   - the icon word is `0` until the update whose `T` is `0.4011` and then follows
     `Ease_CosineWithBump(3 * timer, 1)` to within one count of 255 on all 40 samples;
   - the size words equal `66 / f` and `60 * f` with `f = Ease_Cosine(2T*0.99 + 0.01)` on every sample
     (`267495.16` at `T = 0`, `66.0` at `T = 0.5`);
   - after use: `T` runs `0.5, 0.5201, 0.5201 ...` (held while the timer decays `0.4202 -> 0.1198` in 17
     updates, 0.28 s), then `0.5402 -> 1.0` in 24 updates (0.40 s), the backdrop word equal to the
     reverse-curve prediction to the byte on all 24; at `T = 1.0` the word is `0`, the next update
     `T = 0` and it stays there.
   Confidence **90** for the fade-in, hold, fade-out, size pulse and icon gate as one law. **What is
   poked, not natural**: the grant and the use are the `pickup+0x1bc` write, which is all
   `Hud_UpdatePickupWidgets` reads (through `status+0x3c -> +0x48`, set by `PlayerStatus_Update`), so
   a real pickup pad or a real fire reaches the same code; a natural pad and a natural fire were not
   reproduced here (the previous lane's Square did not reach the restored state either), so whether a
   real use clears the held id in one update or lingers is unread.

6. **On screen, the size axes (2026-10-30)**: the same clean-slate grant, the CPU stopped at the call whose
   `T` first reached 0.10, 0.26 and 0.50 (grant) and 0.72, 0.86 (after use), the software renderer's frame
   read with `import -window root`. At `T = 0.10` (`f = 0.0245`, `66/f` wide, `60 f` tall) nothing is
   visible; at `T = 0.26` a **pink hexagon about 1.5 times wider and about half as tall** as the settled
   one hangs at the top centre (`+0x9c` is the width, `+0xa0` the height; the settled size at `T = 0.50`
   is the layout's 66 x 60 hexagon, pink); at `T = 0.72` after use it is back to a wide flat shape in
   fading pink and at `T = 0.86` it is gone. This fixes the axes the words alone left open. Confidence 80
   for the drawn picture (five frames, one weapon, one circuit).

Not measured: the edge case of **a new grant during the fade-out** (decompile only: with `T` in
`(0.5, 1]` the held branch is skipped entirely, so the new weapon gets no fade-in until `T` has run
out and reset), and the **PS2** binary, which was not searched (PSP is the reference); a byte search of
`SCES_547.48` for the PSP words found nothing, so PS2 stores them in another form. PS2 inherits PSP's
law.

### Live samples (a test fixture for the wire)

Missile (row 2, word `0x80BE1E69`), one row per `Hud_UpdatePickupWidgets` call, the state at the call's
entry (so the word was written by the previous call from this `T`). Layout size 66 x 60.

Fade-in, from `T = 0` (the grant):

| `T` | backdrop word | `+0x9c` | `+0xa0` |
| ---: | --- | ---: | ---: |
| 0.0000 | `0x00000000` | 267495.16 | 0.015 |
| 0.0202 | `0x01010000` | 10735.80 | 0.369 |
| 0.0401 | `0x04060103` | 3362.22 | 1.178 |
| 0.0602 | `0x0A0E0208` | 1626.40 | 2.435 |
| 0.0801 | `0x1119040E` | 962.64 | 4.114 |
| 0.1001 | `0x1A270615` | 639.15 | 6.196 |
| 0.1204 | `0x2436081E` | 456.27 | 8.679 |
| 0.1402 | `0x30470B27` | 346.44 | 11.431 |
| 0.1602 | `0x3C590E31` | 273.06 | 14.502 |
| 0.1806 | `0x4A6D113C` | 221.41 | 17.886 |
| 0.2002 | `0x61901650` | 185.88 | 21.304 |
| 0.2202 | `0x98E2237D` | 158.79 | 24.938 |
| 0.2402 | `0xDEFF34B6` | 138.22 | 28.650 |
| 0.2603 | `0xE5FF35BC` | 122.27 | 32.388 |
| 0.2802 | `0xB0FF2990` | 109.78 | 36.072 |
| 0.3003 | `0x8ACC2071` | 99.82 | 39.673 |
| 0.3203 | `0x81BF1E69` | 91.82 | 43.126 |
| 0.3403 | `0x80BE1E69` | 85.39 | 46.374 |
| 0.3603 | `0x80BE1E69` | 80.21 | 49.371 |
| 0.3803 | `0x80BE1E69` | 76.06 | 52.065 |
| 0.4011 | `0x80BE1E69` | 72.66 | 54.502 |
| 0.4204 | `0x80BE1E69` | 70.22 | 56.394 |
| 0.4404 | `0x80BE1E69` | 68.32 | 57.961 |
| 0.4607 | `0x80BE1E69` | 67.00 | 59.107 |
| 0.4804 | `0x80BE1E69` | 66.25 | 59.778 |
| 0.5000 | `0x80BE1E69` | 66.00 | 60.000 |
| 0.5000 | `0x80BE1E69` | 66.00 | 60.000 |

After use (idx back to 0; `T` continues from 0.5, the row timer counts down from 0.4202):

| `T` | row timer | backdrop word | `+0x9c` | `+0xa0` |
| ---: | ---: | --- | ---: | ---: |
| 0.5201 | 0.1397 | `0x80BE1E69` | 66.00 | 60.000 |
| 0.5201 | 0.1198 | `0x80BE1E69` | 66.00 | 60.000 |
| 0.5402 | 0.0997 | `0x80BE1E69` | 67.04 | 59.067 |
| 0.5602 | 0.0797 | `0x80BE1E69` | 68.37 | 57.924 |
| 0.5808 | 0.0591 | `0x80BE1E69` | 70.35 | 56.287 |
| 0.6002 | 0.0397 | `0x80BE1E69` | 72.86 | 54.354 |
| 0.6204 | 0.0195 | `0x80BE1E69` | 76.19 | 51.975 |
| 0.6407 | 0 | `0x80BE1E69` | 80.45 | 49.225 |
| 0.6603 | 0 | `0x80BE1E69` | 85.57 | 46.278 |
| 0.6803 | 0 | `0x81BF1E69` | 92.04 | 43.023 |
| 0.7003 | 0 | `0x8ACD2071` | 100.08 | 39.568 |
| 0.7204 | 0 | `0xB2FF2992` | 110.14 | 35.955 |
| 0.7404 | 0 | `0xE6FF36BD` | 122.74 | 32.263 |
| 0.7603 | 0 | `0xDCFF33B5` | 138.75 | 28.540 |
| 0.7804 | 0 | `0x96DF237B` | 159.56 | 24.819 |
| 0.8005 | 0 | `0x608F164F` | 187.01 | 21.176 |
| 0.8204 | 0 | `0x496C113C` | 223.73 | 17.700 |
| 0.8407 | 0 | `0x3B580D30` | 275.84 | 14.356 |
| 0.8612 | 0 | `0x2F460B26` | 352.45 | 11.236 |
| 0.8806 | 0 | `0x2436081D` | 463.21 | 8.549 |
| 0.9006 | 0 | `0x1A260615` | 647.90 | 6.112 |
| 0.9209 | 0 | `0x1019030D` | 985.47 | 4.018 |
| 0.9407 | 0 | `0x090E0207` | 1670.96 | 2.370 |
| 0.9608 | 0 | `0x04060103` | 3501.11 | 1.131 |
| 0.9807 | 0 | `0x01010000` | 11574.18 | 0.342 |
| 1.0000 | 0 | `0x00000000` | 267495.16 | 0.015 |

Icon word (white scaled by `Ease_CosineWithBump(3 * timer, 1)`), growing then falling timer, every 2nd update:

| phase | row timer | icon word |
| --- | ---: | --- |
| held | 0.0208 | `0x2020202` |
| held | 0.0601 | `0x13131313` |
| held | 0.1001 | `0x34343434` |
| held | 0.1402 | `0x5F5F5F5F` |
| held | 0.1802 | `0x92929292` |
| held | 0.2203 | `0xFFFFFFFF` |
| held | 0.2602 | `0xFFFFFFFF` |
| held | 0.3009 | `0xFFFFFFFF` |
| held | 0.3403 | `0xFFFFFFFF` |
| held | 0.3804 | `0xFFFFFFFF` |
| used | 0.4202 | `0xFFFFFFFF` |
| used | 0.3599 | `0xFFFFFFFF` |
| used | 0.3199 | `0xFFFFFFFF` |
| used | 0.2791 | `0xFFFFFFFF` |
| used | 0.2398 | `0xFFFFFFFF` |
| used | 0.1998 | `0xC1C1C1C1` |
| used | 0.1597 | `0x77777777` |
| used | 0.1198 | `0x48484848` |
| used | 0.0797 | `0x22222222` |
| used | 0.0397 | `0x8080808` |

## Wired (2026-10-30, `pickup-fade-wire`)

`oag_hud::pickup_fade` ports the law (`Fade::advance` per rendered tick off the held weapon and `dt`,
`Fade::look` for the factors); `oag_hud::pickup` draws it (`scale_word`, the size pulse about the
widget's centre, the icon scaled by its own timer); `oag_raceplay`'s `SlotHud` holds the per-slot state,
render-side and out of the hash. Tests: the 27 fade-in and 24 fade-out words above are reproduced from
the law (within the table's four-decimal `T`), a linear clamp cannot make the overshoot, the size axes,
and the drawn sprites. Headless Pulse PSP frames (`--give missile`, grant at tick 0): tick 3 nothing, 8
a faint wide flat strip, 13 the wide flat bright pink peak, 19 the hexagon settling, 24 the icon
appearing, 30 and 45 settled. **Chosen, not measured**: the size pulse's fixed point is the widget's
centre (the sprite is `Centred`), and a grant during the fade-out run-on waits for it to finish. The
first idle update's extra `dt` (`0.5201`) is not modelled (it moves the fade-out by one update).
PS2 shares Pulse's `ART`, so it plays this law **inherited from PSP**.

## Checked on the other titles

Read 2026-10-30 (`pickup-fade-titles`); the static readings are labelled as such.

- **HD/Fury**: **ported 2026-10-30**. HD does carry this table (this corrects an earlier claim).
  `PTR_008a7800`'s pointee (`0x00920598`) holds the same 14 rows of 6 words with the same 13 words.
  `Hud_UpdatePickupBar`'s idle `0x5AFFFFFF` is the *nothing-held* value only. The fade's constants were
  read: rate **1.5** (Pulse 1.2), cap `0.5`, `x3`, `pi`, `-25` the same; the hold, icon gate and
  fade-out branch are inherited from Pulse. [hd-pickup-table.md](../ps3-hdfury-eu/hd-pickup-table.md).
- **Omega**: **ported 2026-10-30**, rate inherited from HD, static. Same table at `0x0193f750`.
  [omega-pickup-table.md](../ps4-omega-eu/omega-pickup-table.md).
- **2048**: **checked, applies, not wired** (static). The same table twice in the ELF (`0x8151f104`),
  and `FUN_81154172` has the same fade with a `1.5` rate where Pulse has `1.2` (HD's rate is `1.5` too);
  what draws it in the race HUD is unverified, the visible arc being `PickupBgFrame` with fixed
  colours. Needs one Vita3K frame first.
  [pickup-backdrop.md](../vita-2048-eu-v104/pickup-backdrop.md).
- **Pure**: **checked, differs** (live, USA disc). No `PickupBackground`; ten authored `<Model>` icons,
  never recoloured at runtime. [pickup-icons.md](../psp-pure-usa/pickup-icons.md).
