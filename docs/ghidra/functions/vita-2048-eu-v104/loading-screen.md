# The loading screen: which kind a race load starts, and what each draws

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image base `0x81000000`. Every name here
is applied from [names.tsv](names.tsv). The screen is HD's `LoadingScreen` class carried over
([ps3-hdfury-eu/loading-screen.md](../ps3-hdfury-eu/loading-screen.md) is the reading it is compared against); the
format-side page is [2048-loading.md](../../../formats/2048-loading.md). Implemented, in part, by
`oag_2048::loading` (state 7, see "The race load starts kind 4, not kind 7").

## The object and its kinds

`LoadingScreen_Start` (`0x812845ce`, confidence 75) takes the kind, allocates a `0x900`-byte object
(`FUN_812e5110`), runs `LoadingScreen_Construct` (`0x8128074c`), stores it in `DAT_815248c8`, sets the bar's
ceiling (`+0x8c = 94.62`, `0x42bd3d70`) and rate (`+0x90 = +0x94 = 0.115`, `0x3deb851f`), sets the surface to
960 by 544 (`FUN_812f2140(960, 544, ..)`, `FUN_81291cb8(0x3c0, 0x220)`) and runs `LoadingScreen_LoadAssets`
(`0x81281ab2`), which starts a thread named `LoadingScreenThread`. The kind is `object+4`.

| kind | what `LoadAssets` loads, `Draw` draws | started by |
| ---: | --- | --- |
| 0 | `scepresents/scee_presents_<lang>.gxt`, full screen | `Game_Main` (`0x81003eee`, `movs r0,#0`) |
| 4 | the **race load**: a card or still, then a percentage box (below) | `FUN_8115f09a` (`0x8115f198`, `0x8115f2be`), `FUN_811494fa` (`0x81149538`) |
| 6 | no pictures | `Game_Main` (`0x81003e8a`), `0x8114939e`, `FUN_81149414` (`0x8114946a`) |
| 8 | `boot_epilepsy_{US,FR,SP}.gxt` | `Game_Main` (`0x81003ece`) |
| 5, 7, 9 | 5: `DEMO_*_upsell.gxt`; 7: HD's feature screen; 9: `boot_ESRB.gxt` | **no call site** |

Every call of `LoadingScreen_Start` was found with `get_xrefs_to` and `search_instructions` (eight calls); the
argument is a literal `movs r0, #N` at all of them except the two `FUN_8115f09a` calls and `FUN_811494fa`, which
are `FUN_812845ce(4)` in the decompile. **Kind 7 is never started**: confidence 80 (a call through a computed
kind would not show; none of the eight does).

## Kind 7: HD's feature screen (`LoadingScreen_Draw`, `0x812830c8`)

The same screen as HD's `LoadingScreen_Draw` (`0x002b61c8`) and Omega's state 8, compared by operand. The kind
exists in the code and is dormant.

`LoadAssets` kind 7 loads the six marks in this order, as `.gxt`: `data/FE/Images/corner`, `dot`, `data/Fe/Images/square`,
`line`, `Title_Arrow_HD`, `Subtitle_Arrow_HD` (object `+0x2c` to `+0x40`), then, when the type is `0`
(`+0x3d0`), the feature's picture at `+0x44`: `data/Fe/Images/{Barrel_Roll, Side_Shift_Tap, Pilot_Assist, Absorb, Flip}`
with `_fury` before the extension when `FUN_8105b1c4` is non-zero. `FUN_8105b1c4` is `return DAT_8151cae8;`; that byte
is `0` in the file image and its only reference is that read (confidence 70: a store through a computed address
would not show). So the base art is the one this code reaches.

`LoadingScreen_BuildText` (`0x81281396`) sets the strings: `+0x3f8` the brand, the literal at `0x814d0e24`
(`57 49 50 45 4f 55 54 c2 ae 20 48 44`, `WIPEOUT` + U+00AE + ` HD`, **HD's literal**, not Omega's string id),
`+0x3fc` `FE_MODE_ICON`, `+0x400` `FE_LOADINGDOT` followed by the circuit's string (`"%s %s"`, with `(%d/%d)` for the
tournament modes `4` and `0x11`), `+0x404` `FE_FEATURE_IMAGE`, `+0x408` `FE_FEATURE_DESC`, `+0x414` `FE_PROG_BAR`
(`ER_TOUR_STAN`, `FE_TOUR_TRACKS` for type 1), and per `Feature type` (`+0x3d4`) the title, description and heading:

| type | title (`+0x410`) | description (`+0x40c`) | heading (`+0x418`) |
| ---: | --- | --- | --- |
| 0 | `MAN_2_BR` (`0x814d0710`) | `FE_BR_INST` | `FE_INSTRUCTIONS` |
| 1 | `MAN_2_SS` (`0x814d0f04`) | `FE_SS_INST` | `FE_INSTRUCTIONS` |
| 2 | `FE_PILOT_ASSIST` (`0x814d0ed8`) | `FE_PA_INST` | `ONL_CON_DESC` |
| 3 | `IG_HUD_ABSORB` (`0x814d0754`) | `FE_ABSORB_INST` | `FE_INSTRUCTIONS` |
| 4 | `FE_FLIP` (`0x814d079c`) | `FE_FLIP_INST` | `FE_INSTRUCTIONS` |

Each cell is read from the bytes at the quoted address (the first column of ids is the pointer's target), and
agrees with HD's table row for row. Confidence 90.

`LoadingScreen_Construct` resolves `FE_HD_BG`, `FE_HD_Grey`, `FE_HD_Blue` and `FE_HD_LightGrey` (a CRC-32 of the name,
`0x81222a72`, then the palette lookup `0x8105937a`) into `+0x8e8`, `+0x8ec`, `+0x8f0`, `+0x8f4`. **Kind 7 reads `+0x8e8`
nowhere**: it fills no ground, and the marks and text use `+0x8ec` (the draws at `0x812829da`, `0x81282c62`,
`0x81282e5c` read it), the bar's lit and unlit tiles `+0x8f0` and `+0x8f4`. The type is `1` when the mode word
(`DAT_8153fd24`) is `0x11`, else `0`; the feature draw (`+0x3d4`, from `rand()`) is HD's deck row for row:

| mode word | draw |
| --- | --- |
| `GameMode_IsEliminatorFamily` (`0x8100193c`: `8` or `0x14`, or two further flags of the current mode) | `rand() % 5`, index as drawn |
| `FUN_81001982` (`0xd` or `0x15`) | `rand() % 3`, `0 1 3` |
| `0xe` | `1` |
| `6` | `rand() % 2` |
| otherwise | `rand() % 4` (HD's `% 3` without the Fury byte has no counterpart) |

The draw (`0x812830c8`, `iVar5 == 7`), operands in the 1920 by 1080 grid HD's uses (rect arguments of
`LoadingScreen_DrawImage` are `x, y, w, h`; the grid is the half-resolution surface's own, see "The grid"):

| element | operands here | HD's (`LoadingScreen_HD.toml`) |
| --- | --- | --- |
| top, bottom rule (`line`) | `0xa0, 0x6e, 0x640, 8` and `0xa0, 0x3cf, 0x640, 8` = (160,110) and (160,975), 1600 by 8 | same |
| caption arrow (`Title_Arrow_HD`) | `0xa0, 0x49, 0x20, 0x20` = (160,73), 32 | same |
| brand label | `LoadingScreen_DrawLabel` at (`0xa0`, `0x30`) = (160,48) | same |
| caption | font `Title`, scale 1.0, at (`0x43420000`, `0x42780000`) = (194,62), drawn twice | same |
| the three other labels | (160,670), (1510,670), (160,125), (960,125) | same |
| bracket sets | corners `0x43200000 0x431b0000` = (160,155) to (`0x446d8000`, `0x4423c000`) = (950,655); (960,155) to (1756,655); (1500,700)..; (160,700) to (1500,950); (1510,700) to (1756,950) | same |
| feature picture | `FUN_81282ade(0.875, 0.0625, 0, 1.0, .., 0xb4, 0xbe, 0x380, 0x1e0)` = (180,190), 896 by 480, uv 0.875 by 0.9375 | same |
| name row, heading row | `LoadingScreen_DrawTitleRow` at (`0xaf`,`0xaf`) = (175,175) and (`0x3cf`,`0xaf`) = (975,175) | same |
| paragraph | font `Default`, (`0x4473c000`, `0x43570000`) = (975,215), wrap `0x30c` = 780, pitch `0x28` = 40 | same |
| the bar | two tiled quads from x 168 (`0x43280000`) to x 1496 (`0x44bb0000`), y 708 (`0x44310000`) to 948 (`0x446d0000`), tile 30 (`0x41f00000`) | 166 by 30 dots of 8 |
| the mode icon's panel | the label and the brackets above, **nothing inside them**: no clip, no still | HD plays a Bink clip |

The order is the picture, then the name row and the heading row, then the paragraph, so the rows are drawn over the
picture. Confidence 85 on the operand equality (compared, not measured on a running game).

### The grid

The operands are in a 1920 by 1080 grid, and the surface the starter sets is 960 by 544. Kinds 0, 4 and 8 use
960 by 544 operands (`0x3c0`, `0x220`) on the same surface, so kind 7's coordinates are twice kind 4's. What maps one
to the other (the divisor in `FUN_812f2734(.., 2)` that every `Draw` calls first, probably) is **not read**; the build
draws kind 7 in HD's 1920 by 1080 grid (confidence 55).

### The bar

`+0x88` is the shown fraction, advanced by `+0x90` (0.115) per frame to the ceiling `+0x8c` (94.62), where the rate
is reset to `+0x94`. HD's bar runs to 149.4 of 166 columns at 0.1: **2048's ceiling is lower** (94.62), and the lit
width is `VectorFloatToUnsigned(+0x88) * 8 + 168` px. How it maps onto the 1328 px bar is unread; the build keeps the
time estimate (`progression: None`).

## Kind 4: the race load (`LoadingScreen_Draw`, `0x812830c8`, `iVar5 == 4`)

What a Vita player sees on a race load. Read, **not built** (the Omega page's "state 5").

- A backdrop, chosen by the current game mode (`GameMode_Current`): none yet (the front end is loading) draws
  `FUN_8105021c` or `CampaignEventCard_DrawPanel(0, 0, 1.0, 0)`; a mode whose vtable slot `+0x60` returns `0` draws
  `CampaignEventCard_DrawPanel(0, 0, 1.0, mode)` (or `FUN_81056e90` when `mode+0x1a0 & 4`); any other mode draws a
  still, `+0x18`, at (-32, 144), 1024 by 256: for a mode on the HD lineage (`GameMode_IsHdLineage`) one of
  `LoadingScreens/LoadingScreen_{racer,motion,wipeout}_controls.gxt` by the control scheme (`DAT_818a5098`: 1 racer,
  2 motion, else wipeout), otherwise `Title_Screen.gxt` (`LoadAssets` also loads `Post_Race_Medals.gxt`, `loading_screen_demo_bar.gxt` and one of
  three `Team_Logos_*` at random).
- A percentage box: `Blue2048` (`0xff19295d`) and `White2048` (`0xffe1e4eb`) of `Skin.xml`, a 70 by 70 fill at (16,16),
  a bar fill (86,82) 338 by 4, `FE_LOADINGDOT` at (105,30) in the `default` face, `%d%%` of `+0x88 * 0.006024096 * 100`
  (1/166) at (51,38) at scale 0.6, and `FUN_8105d4a8(+0x88 / 166, 790, 475, Blue2048)` at the bottom right.
- **The ceiling reads 57 %.** `94.62 / 166 = 0.57`, so the readout stops at 57 % until the loader raises the target
  (`FUN_812845b8`), which is the 56 % the Vita3K boots sat at when a load stalled
  ([vita3k-capture.md](../../../reverse-engineering/vita3k-capture.md), "Metro Park's Time Trial never finishes
  loading"). Corroboration, not proof (confidence 65).

## Names

| address | name | confidence |
| --- | --- | ---: |
| `0x8128074c` | `LoadingScreen_Construct` | 85 |
| `0x81281ab2` | `LoadingScreen_LoadAssets` | 80 |
| `0x812830c8` | `LoadingScreen_Draw` | 80 |
| `0x81281396` | `LoadingScreen_BuildText` | 80 |
| `0x812845ce` | `LoadingScreen_Start` | 75 |
| `0x81282ade` | `LoadingScreen_DrawImage` | 78 |
| `0x812829a6` | `LoadingScreen_DrawCorner` | 75 |
| `0x81282e02` | `LoadingScreen_DrawLabel` | 78 |
| `0x81282c08` | `LoadingScreen_DrawTitleRow` | 72 |
| `0x8105b1c4` | `FrontEnd_IsFuryStyle` | 65 |
| `0x8151cae8` | `g_front_end_fury_style` | 65 |

Each matches HD's function of the same shape by arguments, strings and call structure (`LoadingScreen_Construct`
by the four `FE_HD_*` lookups and the `LOADING SCREEN TYPE type == %i` string, `_DrawLabel` by the 16 by 16 bullet and
`x + 22` text, `_DrawCorner` by its eight calls with `(0|1, 0|1)` flips). `FrontEnd_IsFuryStyle` is named by what
`LoadAssets` does with it; its writer is not found, hence 65.
