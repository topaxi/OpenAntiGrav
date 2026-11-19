# Game Options that reach the HUD: Dynamic HUD and Multiplayer Tags (Pulse, PSP)

Addresses are `pulse-psp-usa` `BOOT.BIN`, image base `0x08804000`. Read 2026-11-06 with a headless, read-only
`analyzeHeadless` run on the maintainer's project (`/pulse/BOOT-psp-pulse-usa.BIN`): string xrefs, then the decompiler.
The XML side (the lists and their entries) is in `docs/ui/menus-original.md`, "Game Options".

| Address | Name | Conf | What it is |
| --- | --- | ---: | --- |
| `0x0881bf50` | `Hud_Update` | 80 | Reads the `Dynamic HUD` and `Multiplayer Tags` list values once after the profile's dirty byte (`+0x45b`) is set |
| `0x0881ba58` | `Hud_UpdateMultiplayerTags` | 80 | Per-frame update of the eight player tags (`PlrTag0`-`PlrTag7`) |
| `0x08ab083d` | `g_multiplayer_tags_on` | 88 | The `Multiplayer Tags` option as a byte: `FE_ON` 1, `FE_OFF` 0 |
| `0x08966848` | `MpTag_GetPlayerName` | 70 | A slot's name: `table + slot*0x2c + 0xc4` when its first byte is set, else the string `ONL_CRJ_PLAYER` (looked up through the language table) |
| `0x08919c2c` | `Hud_WarpVertexY` | 92 | The `Dynamic HUD` warp: a vertex's new `y` from its `(x, y)`; see "The warp" |
| `0x088a686c` | `Image_SetVertexCount` | 85 | An image's strip length (`+0x10c`, at least 4) and its segment count (`+0x110 = n / 2 - 1`) |
| `0x08919cf8` | `Text_BuildGlyphQuads` | 72 | Lays a string's glyphs out as sprites, or as two triangles each with every corner through `Hud_WarpVertexY` while the warp is on |
| `0x088cc9f4` | `Text_LayoutKey` | 70 | A 20-byte key over colour, x, y, z and the warp flag; `Text_Draw` lays the string out again when it changes |
| `0x08abf585` | `g_text_warp` | 85 | The warp bit `Text_Draw` hands the glyph builder |
| `0x08abf580` | `g_hud_warp_amplitude` | 92 | `35.0` |
| `0x08abf588` | `g_hud_warp_centre_y` | 92 | `136.0`, the PSP screen's middle row |
| `0x08abf58c` | `g_hud_warp_frequency` | 92 | `0x3bd67750`, `pi / 480` |

## Dynamic HUD, in `Hud_Update` (`0x0881bf50`..)

The profile's list value for `Dynamic HUD` is fetched (`FUN_08808664(profile, "Dynamic HUD", 0, 1)`) and compared with
`strcasecmp`. `hud` is `param_2`, `hud+0x274` and `hud+0x278` are the two panel groups:

| Value | `hud+0x121` (HUD shown) | bit 1 of the group's `+0xd0` |
| --- | --- | --- |
| `FE_OFF` | 0, and `RaceManager_ShowNowPlaying` clears the marquee | unchanged |
| `FE_DEFAULT` | 1 | `race_manager+0x6f & 1` (a per-race flag) |
| `FE_FLAT` | 1 | 0 |
| anything else (`FE_CONCAVE`) | 1 | 1 |

The second group's bit copies the first's. Confidence 85 for the table (decompiler, branches read in order).

**`FE_DEFAULT` is the camera view.** `race_manager+0x2c0` is the player's craft, and its `+0x6f` is a copy
of `+0x6d`, which `Camera_UpdatePlayerView` sets to 1 for `OPT_INT` and 0 for the two external views
(`0x0883c23c`, `0x0883c2c4`, then `0x0883c2d0` copies it; [camera.md](camera.md)). The same SELECT press
writes the settings-changed byte (`sb fp,0x45b` at `0x0883c174`), so `Hud_Update` re-reads the list on every
camera change, and `RaceManager_Construct` sets it to 1 (`0x088295b4`), so a race starts with the list
already applied. Seen live (below): under `FE_DEFAULT` the bit was 0 in the far and close views and 2 in the
internal one, and only the internal frame was bent. Confidence 90.

## The warp (2026-11-19)

Read in the decompiler and disassembly, then confirmed on PPSSPP 1.20.4 (software renderer, USA disc,
`time-trial-talons-venom-grid.ppst` driven on, Talon's Junction, VENOM).

**Which widgets.** `hud+0x274` is the group named `HUD` (looked up in the HUD's own tree, `0x08819dd8`, string
`0x08a79bf8`), `hud+0x278` the global UI's `InGame` (`0x08819df4`). Every widget of the race layouts
(`Arcade_HUD.xml` and the others) is in `<Screen name="HUD">`, so the bit reaches every image and text widget of
the HUD. The weapon sights, `ReadyGo` and `Cockpit321Go` are `Model`s, whose draw does not read it. Three readers
test `widget+0x60` (the parent) `+0xd0 & 2`:

1. `Image_Draw` (`0x088a7a5c`, at `0x088a84f8`) runs `Hud_WarpVertexY(x, y)` over each of the image's
   0x18-byte strip vertices and writes the result over `y` (`+0x10`). An image is a strip of `+0x10c` vertices,
   4 by default (`FUN_088a6294` calls `Image_SetVertexCount(image, 4)` at `0x088a63b0`): its four corners.
2. `Hud_UpdateSpeedBar` (`0x0881a3e0`) gives `SpeedBarBg`, `SpeedBar`, `ShieldBarBg` and `ShieldBar`
   (`hud+0x198`, `+0x19c`, `+0x1a8`, `+0x1b0`; names from `Hud_BindWidgets`' stores at `0x0881fcb0`..`0x0881fe20`)
   10 vertices while the bit is set and 4 otherwise: five columns, four equal segments across the drawn width,
   the texture window interpolated with them, so the bars curve.
3. `Text_Draw` (`0x088ce668`) hands the bit to `Text_LayoutKey` (so a change lays the string out again) and to
   `g_text_warp` (`0x08abf585`), which `Text_BuildGlyphQuads` reads: glyphs become two triangles (prim 3, six
   vertices) instead of a sprite (prim 6, two), and each of the four corners goes through `Hud_WarpVertexY`.

**The law** (`Hud_WarpVertexY`, `f12 = x`, `f13 = y`):

```
if y >= 136:  y' = y - 35 * sin(x * pi/480) * (y - 136) / 136
else:         y' = y + 35 * sin(x * pi/480) * (136 - y) / 136
```

which is one expression, `y' = y + 35 * sin(pi * x / 480) * (1 - y / 136)`, in the PSP's pixels; `x` is
unchanged. The left and right edges stay, the middle column's top edge comes down 35 and its bottom edge up 35,
the middle row stays. `sinf` is `0x0897e300`. Confidence 92: the constants are read off the image and the
law reproduces the live frame below.

**Live A/B.** With the CPU stopped, `g_hud` (`0x08ab0838`) read `0x09a0c430`, its groups `0x09a0e620` and
`0x08ef3d20`, both `+0xd0` 0. Clearing then setting bit 1 on both, one 0.2 s run between, and one frame each:

| Shield bar top edge, `x` | 310 | 330 | 360 | 390 | 420 | 440 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| flat | 247 | 247 | 247 | 247 | 247 | 247 |
| bit set | 222 | 224 | 227 | 231 | 236 | 240 |
| the law at `y = 247` | 221.4 | 223.2 | 226.8 | 231.1 | 236.1 | 239.6 |

The lap digit's top moved 19 to 21 (law 1.6) and the `Lap` caption's 30 to 36 (law 5.0). The earlier "moved
nothing" note on [now-playing.md](now-playing.md) was two frames with the camera moving between them.

**Ours.** `oag_hud::concave` holds the law and the four column names; `oag_game`'s UI quad carries per-corner
offsets the shader adds; the reticle is drawn after the rest, flat; the boost and absorb glow stamps
(a rectangle-only bloom chain reading the HUD's pixels) are cut into the same columns and grown to the bent
corners, so the glow stays on a bent bar. On the same layout the shield bar's top edge
in ours matches the original's to the pixel at all six columns (frame pair, `oag-game pulse-psp-usa.chd --race
--mode time_trial --camera-view close --ticks 200 --size 480x272 --screenshot` with `[display] dynamic_hud =
"concave"`). Ours offers `concave` on every other title too, at the same share of the screen's height; that is
ours, not the original's.

**PS2** (`SCES_547.48`, the same code). `FUN_001355d8` reads the list the same way, the bit at the group's
`+0xd4` (`*(hud+0x2a4)`, `*(hud+0x2a8)`), `FE_DEFAULT` from the craft's `+0xfc & 1` (presumed the internal-view
flag, its writer not read); `FUN_00136748` gives the four
bars 10 vertices against 4 (`0x001368e4`..); the image draw (`0x001934dc`) runs `FUN_001e5e98`, the same law with
35.0 at `0x00282ab0`, the centre row `FUN_001e4fe0` (screen height `/ 2`) and the frequency `FUN_001e5000`
(`pi / ` screen width), both from the display globals `0x00284fe0`/`0x00284fe4` (512 and 512 in the image, written
at run time by `FUN_00215c80`; not read live). The executable compares `FE_FLAT` though its list does not offer it.
Confidence 80 for "the same law, 35 grid units on the PS2's own screen"; the PS2 grid's size at run time is
unmeasured. Not renamed on the PS2 binary.

**HD and Omega.** Checked, absent: neither executable carries `Dynamic HUD`, `FE_CONCAVE` or `FE_FLAT`
(`search_strings` on `EBOOT-ps3-hdfury-eu.elf` and `eboot-ps4-omega-eu.bin`), and HD's options list has no such
row ([menus-original.md](../../../ui/menus-original.md), "Game Options").

## Multiplayer Tags, in `Hud_Update` and `Hud_UpdateMultiplayerTags` (`0x0881ba58`)

`Hud_Update` writes `FE_OFF` to 0 and `FE_ON` to 1 in `g_multiplayer_tags_on` (a value that is neither leaves it). The
only reader is `Hud_UpdateMultiplayerTags`, which drives the eight `PlrTag` text widgets of `MPTag_HUD.xml` (`hud+0x48`
is the widget array, `hud+0x40` the eight-entry source list, stride 0x10):

1. A slot whose active byte (`entry+0x64`) is clear, or whose craft `FUN_0883e37c` rejects, gets the empty string.
2. A craft in `Ship_State` 1 or 2 gets a tag: its fade `entry+0x68` rises by `dt * 1.2` up to 1.
3. The anchor is the craft's position (`craft+0x794` block, `+0x30/+0x34/+0x38`) **raised by `fade * 2.0` units**, projected to
   the screen by `FUN_08819900`, which also returns the distance `d`.
4. Alpha is `min(fade, f(d))` with `f = 1` up to `d = 20`, then falling linearly to 0 at `d = 60`
   (`(40 - (d - 20)) / 40`, constants `0x08a79980` = 20.0 and `0x08a79984` = 40.0).
5. **`g_multiplayer_tags_on == 0` forces the alpha to 0**, so the option hides the tag, it does not stop the update.
6. Colour is white, `alpha << 24 | 0xffffff`; the scale is 0.5 (the XML's own); the text is `MpTag_GetPlayerName` of the
   craft's slot (`craft+0x364`).

Confidence 80. Unread: what `FUN_0883e37c` tests (it compares the display's viewport mask against the craft's, which
reads as "visible in this view", confidence 50), and whether the local player's own craft gets a tag. Ours: wired as
`oag_hud::player_tags` (law and tests) and `oag_raceplay`'s `player_tags` (fades, projection, names); `docs/ui/menus-original.md`,
"Game Options" lists what is chosen.
