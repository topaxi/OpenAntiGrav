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

**What bit 1 of `+0xd0` does is partly read (confidence 60).** Three places test it: `Hud_UpdateSpeedBar` (`0x0881a3e0`, at
`0x0881a538`) passes `10` instead of `4` to `FUN_088a686c` for the bar's four elements; `Image_Draw` (`0x088a7a5c`, at
`0x088a84f8`) runs `FUN_08919c2c(x, y)` over every 0x18-byte vertex and writes the result back into `+0x10` (a per-vertex
warp); `Text_Draw` (`0x088ce668`, at `0x088cec28`) hands the flag to the glyph layout call `FUN_088cc9f4` as its last
argument. So the bit is a warp mode applied to text and image vertices: it reads as the "concave" (curved) HUD. The warp's
maths (`FUN_08919c2c`) and the layout call are **not read**, so the two entries are not offered.

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
reads as "visible in this view", confidence 50), and whether the local player's own craft gets a tag. This engine's
split screen has no player names to show and no projected-text path exists in `oag-hud`, so the tags are not drawn here yet.
