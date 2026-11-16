# The race load: what kind 4 of the loading screen draws, and the law of its meter

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image base `0x81000000`. Every name here is
applied from [names.tsv](names.tsv). This page continues [loading-screen.md](loading-screen.md) (the class, its kinds
and the call-site census) with what that page left unread, and replaces its "not read" list. The format-side page is
[2048-loading.md](../../../formats/2048-loading.md). **Implemented** since lane `vita-race-load-wire` except the items under "Open" on [2048-loading.md](../../../formats/2048-loading.md): `oag_2048::TITLE.loading` is `Some`.

Lane `vita-race-load-re`, 2026-11-16. Static reading in Ghidra, then three captures of a campaign race load on
Vita3K (Queens Mall, `2048 - Event 2`, the maintainer's save) with the screen's own fields read out of the guest's
memory every few milliseconds and a frame grabbed four times a second. Reproduce with
`scripts/vita-2048-race-load.py` (its docstring has the command).

## What a player sees (measured, three runs)

| time after the Play tap | what happens | evidence |
| --- | --- | --- |
| 0.63 s | `GameMode_SetCurrent(event)`; the card's photo flag `0x816c7834` goes `1 -> 0` and its tip flag `0x816c7838` `0 -> 1`; `LoadingScreen_Start(4)`: shown `0`, ceiling `94.62`, rate `0.115` | memory, all three runs |
| 0.63 s to about 3 s | the **event card stays where it was** (x 420, the page it was on) and a **GAME TIP panel replaces the track photo**; the percentage box, `LOADING...`, the bar and the triangle meter are drawn over the front end's blurred map | frame at 3.36 s of run B: `8%`, tip "MISSILE" |
| 1.6 to 2.8 s | the ceiling is raised to `124.5`, `157.7`, `162.68` (0.75, 0.95, 0.98 of 166), each time rescaling the rate | memory, see "The meter" |
| 2.7 to 3.6 s | the race manager is built: the rate is frozen, the card gains a **mode page** and turns to it, photo and tip are both switched off, and the card **slides 202 px left** to the middle of the screen | memory: page list `0 1 4` becomes `0 1 4 6`, page `0 -> 3`; frames: panel edge at x 218 |
| until the load ends | the meter runs to 98 % and waits there (11 s on an idle machine, 50 s on a loaded one) | runs A and C sat at `162.68` for 47 s, run B for 8 s |
| the end | `LoadingScreen_SetProgress(1.0)`: shown and ceiling `166` for one sample, then the object is freed and the front end's card is briefly visible over the unblurred map | memory; the frame after shows no box |

The screen `scripts/vita-2048-pair.py --walk` calls "the mode card" **is this loading screen** after the slide: its
second `(882, 480)` tap lands on a load already running and does nothing (run B made no second tap and loaded the
same). Confidence 90.

Frame measurements (960 by 544, two frames, pixel-exact against the operands): the box is x 16 to 85, y 16 to 85,
colour `(25, 41, 93)` = `Blue2048` `0xff19295d`; the bar is x 86 to 423, y 82 to 85; the tip panel's left edge is x 16;
the slid card's left edge is x 218 (`420 - 202`); the meter's blue pixels span x 693 to 917, y 462 to 501.

## The mode test: vtable slot `+0x60`

`LoadingScreen_Draw` kind 4 (`0x812830c8`) asks the current mode object (`GameMode_Current`, `g_CurrentGameMode`
`0x81965b58`) `(*mode->vtable[0x60 / 4])(mode)`. **`0` draws the event card, non-zero draws the still.** The slot was
read out of every `GameModeBase` vtable in the image (a scan of the read-only segment for tables sharing at least six
slots with the base's, each identified by the string its slot `+0x5c` returns):

| class (slot `+0x5c` string) | vtable | slot `+0x60` | returns |
| --- | --- | --- | --- |
| `Game Mode` (the base) | `0x81515f88` | `0x813ea230` (`movs r0,#0; bx lr`) | 0 |
| `RACE` | `0x8151611c` | `0x813ea230` | 0 |
| **`ATTRACT`** | `0x815161cc` | **`0x813eba00` (`movs r0,#1; bx lr`)** | **1** |
| `CHECKPOINT` | `0x8151628c` | `0x813ea230` | 0 |
| `COMBAT` | `0x81516340` | `0x813ea230` | 0 |
| `SPEED` | `0x815163f0` | `0x813ea230` | 0 |
| `ZOMBIE` | `0x815164b8` | `0x813ea230` | 0 |
| `ZONE` | `0x81516568` | `0x813ea230` | 0 |

So the slot is "is the attract mode": **only the attract demo's load draws a still** (`+0x18`: for a mode word on the
HD lineage one of the three controls stills, otherwise `Title_Screen.gxt`, at (-32, 144), 1024 by 256). Every race a
player launches draws a card. `GameModeAttract_Construct` (`0x812bd128`) stores that vtable; its one caller
(`0x810763d8`) names the object `GameModeAttractMode`, makes it current and fires `Launch 2048`. The live run agrees:
Event 2's object has vtable `0x8151611c` (`RACE`) and drew the card. Confidence 92 (the table is bytes; the scan
would miss a vtable sharing fewer than six slots with the base, and none of the seven `GameMode_*` names is missing).

`CampaignEventCard_BuildPageList` asks the same slot (`0x81051544` onwards) to choose the leaderboard tab, which is
why it exists on the base class.

## How the launched event's card reaches the screen

There is no hand-over: the loading screen draws the **front end's own card** through its statics.

1. `CampaignEventCard_HandleInput` (Play) calls `GameMode_SetCurrent(event)` (`0x810f2474`) and
   `CampaignEventCard_EnterLoad` (`0x81051820`, called at `0x810f2524`).
2. `LoadingScreen_Draw` kind 4 calls `CampaignEventCard_DrawPanel(0, 0, 1.0, GameMode_Current())`
   (`0x81055a16`): the *event* argument of the painter is the mode object itself (`2048 - Event 2` at `mode+0xc4` in
   the live run). The page list, the page index, the photo and tip flags and the slide are the card's statics
   (`0x816c7804..0x816c783c`), left as the front end had them.
3. The front end keeps running under the loading thread: kind 4 first calls slot `+0xc` of `DAT_818b9ea8` and of
   `DAT_8151d7f0` with `0x3d088889` (1/30 s) and ends with slot `+0x1c` of the latter, which is the blurred map behind
   the card.

The other three branches of the backdrop, for completeness:

| current mode | drawn | when |
| --- | --- | --- |
| none, and the mode word (`0x8153fd24`) is 16 to 22 (`0x810006fc(.., 0)`) | `CampaignEventCard_DrawPanel(0, 0, 1.0, 0)`: the no-event branch (tournament header) | multiplayer mode words |
| none, otherwise, and `DAT_8153fcd4 != 0` | `CampaignEventCard_DrawCellPanel(0, 0, 1.0)` (`0x8105021c`) | the HD and Fury campaign cells, which race without a `GameModeBase` |
| a mode with `m_options & 4` (`mode+0x1a0`) | `0x81056e90`, a wrapper of `DrawPanel(DAT_8151e998 + x, mode, ..)` | online events |
| a mode whose slot `+0x60` is non-zero | the still | the attract demo |

### What `CampaignEventCard_EnterLoad` does: the game tip

When the profile setting `TutorialsActive` reads `ON` (the string at `0x81427118`), it picks a tip
(`GameTip_Select(-1)`), clears the photo flag and sets the tip flag, so `DrawPanel` calls `GameTip_DrawPanel(x + 16)`
where it drew the track photo. It also turns the card to its pass-to-unlock page when `0x816c7854` is set.

`GameTip_DrawPanel` (`0x8106dd36`, `(x, y, alpha)`; the frame puts the panel at (16, 92)):

| piece | operands |
| --- | --- |
| panel | (x, y) to (x + 398, y + 334), `Transparent2048` |
| `?` mark | `data/fe/newimages/tips/question_mark`, (x, y), 64 by 64, tinted `Orange2048` |
| header | `TUT_TIP_HEADER`, face `default`, scale 1.0, `Orange2048`, at (x + 64, y + 10) |
| body | the row's body key, face `NEOSANS_BOLD`, scale 0.7, `Blue2048`, wrap 360, pitch 20, at (x + 12, y + 330 - h), `h` the wrapped text's measured height |
| title | the row's title key, scale 1.0, fitted to 383 wide, at (x + 12, y + 304 - h) |
| picture | the row's texture, white, centred on (x + 199, y + 50 + (284 - h - 25) / 2), drawn at its own size and scaled by `(258 - h) / 163` when `258 - h < 163` |

Rows `0x22` to `0x24` (the three controls diagrams) are drawn by `0x8106d3ce` instead (**not read**).

`g_GameTipTable` (`0x8151d230`) is 37 rows of three pointers: texture path, title key, body key. Index: texture stem
(under `data/fe/newimages/tips/`, the executable says `.gtf`, the package ships `.gxt`), title, body.

| # | texture | title | body | # | texture | title | body |
| ---: | --- | --- | --- | ---: | --- | --- | --- |
| 0 | `Tut_Absorb_0` | `Tut_Absorb_0` | `Tut_Absorb_2` | 19 | `Tut_Weapon_Leech_Beam_0` | `FE_LEECHBEAM` | `Tut_Weapon_Leech_Beam_1` |
| 1 | `Tut_Airbrakes_0` | `OPT_CTRL_AIRBRAKES` | `Tut_Airbrakes_2` | 20 | `Tut_Weapon_Mines_Bombs_0` | `Tut_Weapon_Mines_Bombs_0` | `Tut_Weapon_Mines_Bombs_1` |
| 2 | `Tut_Event_A_Plus_Challenge_0` | same | `.._1` | 21 | `Tut_Weapon_Missile_0` | `FE_MISSILE` | `Tut_Weapon_Missile_1` |
| 3 | `Tut_Event_Combat_0` | same | `.._1` | 22 | `Tut_Weapon_Plasma_0` | same | `.._1` |
| 4 | `Tut_Event_Proto_Ship_Challenge_0` | same | `.._1` | 23 | `Tut_Weapon_Quake_0` | `FE_QUAKE` | `Tut_Weapon_Quake_1` |
| 5 | `Tut_Event_Race_0` | same | `.._1` | 24 | `Tut_Weapon_Rockets_0` | same | `.._1` |
| 6 | `Tut_Event_Speed_Lap_0` | same | `.._1` | 25 | `Tut_Weapon_Shield_0` | same | `.._1` |
| 7 | `Tut_Event_Speed_Lap_0` | `Tut_Event_Time_Trial_0` | `Tut_Event_Time_Trial_1` | 26 | `Tut_Weapon_Turbo_0` | `FE_TURBO` | `Tut_Weapon_Turbo_1` |
| 8 | `Tut_Event_Zone_0` | same | `.._1` | 27 | `Tut_XP_SP_0` | `Tut_XP_MP_1_1` | `Tut_XP_MP_1_0` |
| 9 | `Tut_Green_Pads_0` | same | `Tut_Green_Pads_2` | 28 | `Tut_XP_Rank_0` | same | `.._1` |
| 10 | `Tut_Yellow_Pads_0` | same | `Tut_Yellow_Pads_2` | 29 | `Tut_XP_SP_0` | `Tut_XP_SP_1` | `Tut_XP_SP_0` |
| 11 | `Tut_Pilot_Assist_0` | same | `.._1` | 30 | `Tut_barrel_roll` | `Tut_Barrel_Roll_0` | `Tut_Barrel_Roll_2` |
| 12 | `Tut_Event_Proto_Ship_Challenge_0` | `Tut_Prototype_Ships_0` | `Tut_Prototype_Ships_1` | 31 | `Tut_side_shift` | `Tut_Sideshift_0` | `Tut_Sideshift_2` |
| 13 | `Tut_Ship_Types_0` | same | `.._1` | 32 | `Tut_flip_0` | `Tut_Combat_Spin_0` | `Tut_Combat_Spin_1` |
| 14 | `Tut_Skillcuts_0` | same | `.._1` | 33 | `Tut_boost_start` | `Tut_Boost_Start_0` | `Tut_Boost_Start` |
| 15 | `Tut_Speed_Classes_0` | same | `.._1` | 34 | `Wipeout_cntrls` | `Tut_Controls_Wipeout_2` | none |
| 16 | `Tut_Speed_Pads_0` | same | `Tut_Speed_Pads_2` | 35 | `Racer_cntrls` | `FE_CTRL_RACER` | none |
| 17 | `Tut_Weapon_Autopilot_0` | `FE_AUTOPILOT` | `Tut_Weapon_Autopilot_1` | 36 | `Motion_cntrls` | `Tut_Controls_Motion_2` | none |
| 18 | `Tut_Weapon_Cannon_0` | `FE_CANNON` | `Tut_Weapon_Cannon_1` | | | | |

All 35 distinct textures and `question_mark` ship in `data.psarc` (36 entries under `data/FE/NewImages/tips/`,
case aside). Run B drew row 21 (`FE_MISSILE`, "MISSILE") on a weapons event.

`GameTip_Pick` (`0x8106ce84`), in order:

1. The first two loads of a profile (a counter at profile `+0x325a2`, below 2) show the controls row of the profile's
   `Motion Sensor` setting: `RACER` 35, `MOTION` 36, `WIPEOUT` 34.
2. The first load of each event class shows that class's row, once (flags at profile `+0x325a3..7`): class `2` race
   row 5, `0` zone row 8, `1` combat row 3, `3` with button shape 5 or 6 speed lap row 6, `3` otherwise time trial
   row 7. The class is `0x812b3414(mode)`.
3. Otherwise a uniform draw (`rand() % n`) from a list built for the event: always rows `1, 11, 12, 13, 14, 15, 16,
   28, 30, 31, 33, 0`; when weapons are on (slot `+0x64` returns 0) and the active weapon pads (`mode+0x1f0`, `m_activeWeaponPads`) have bit 0 or 2,
   rows `10, 18, 22, 23, 24, 21`; when it has bit 1 or 3, rows `9, 17, 19, 20, 25, 26`; row `32` for a combat event;
   row `27` for an online event (`m_options & 4`) else `29`; and the event's own row (button shape 8: row 2, shape 4:
   row 4, else the class row of step 2).

Confidence 80 for the pick (read, with one live draw consistent with it: a missile tip on an offensive-weapons event),
85 for the panel (operands read, one frame).

### What the race manager does: the mode page and the slide

The race manager's constructor (`0x812ad888`, `GameModes/GameMode_RaceManager.cpp`) reaches
`CampaignEventCard_ShowModePage` (`0x810517ae`, through `0x811743e4`): when `DAT_8153fee8 == 0` it appends page kind
`6` to the page list if absent, turns to the last page, and clears both the photo and the tip flag. With neither set
the card's slide (`0x816c783c`) climbs `0.08` a draw to `1.0` and the whole card is drawn at
`x - (1 - (1 - s)^2) * 202` (read in `CampaignEventCard_DrawCellPanel`, `0x434a0000`; measured on `DrawPanel`: the
slide read `0.16`, `0.72`, `1.0` on consecutive frames and the panel came to rest at x 218).

`CampaignEventCard_DrawModePage` (`0x8104fc5a`, page kind 6, `(x, y, alpha, event)`), for an event that is not a
prototype challenge (button shape `mode+0x19c != 4`):

| piece | operands |
| --- | --- |
| body | by class: zone `Tut_Event_Zone_1`, combat `Tut_Event_Combat_1`, race `Tut_Event_Race_1`, class 3 `Tut_Event_Speed_Lap_1` (button shape 5 or 6) else `Tut_Event_Time_Trial_1`; face `NEOSANS_BOLD`, scale 1.0, wrap 375, pitch 21, at (x + 500, y' + 30) |
| `y'` | `y + max(198, 284 - (h + 30) / 2)`, `h` the body's measured height |
| title | `GameMode_GetKindLabel(event)`, face `default`, scale 0.82, at (x + 558, y' - 10) |
| class icon | `0x81061040(x + 524, y' + 8, 0.7, class, colour)` |

A prototype challenge (shape 4) writes `Tut_Proto_Event_<team>_0` and `_1` by the forced craft's team, body at scale
0.7 and pitch 18, no icon. The frame agrees with `x = -202`: the icon at about x 322, `RACE` at 355, the body at 297,
the fourth page dot orange. In a multiplayer mode word the page is a "waiting for" list instead (not read further).
Confidence 85.

## The box, the bar and the meter

Read in `LoadingScreen_Draw` kind 4, all confirmed by the frames above:

| piece | operands |
| --- | --- |
| box | `0x81282ff6(.., 0x10, 0x10, 0x46, 0x46, Blue2048)`: 70 by 70 at (16, 16) |
| bar | `(.., 0x56, 0x52, 0x152, 4, Blue2048)`: 338 by 4 at (86, 82). A plain fill: **it does not grow** |
| caption | `FE_LOADINGDOT`, face `default`, scale 1.0, `Blue2048`, left at (105, 30) |
| readout | `"%d%%"` of `(int)(shown * 0.006024096 * 100)`, scale 0.6, `White2048`, centred on (51, 38) |
| meter | `FrontEnd_DrawLoadingMeter(shown / 166, 790, 475, Blue2048)` |

### `FrontEnd_DrawLoadingMeter` - `0x8105d4a8`, and `FrontEnd_DrawLoadingMeterTriangle` - `0x8105d1f0`

Seven triangles, `i = 0..6`, centred at `x = 790 + 105 - 30 i` (895 down to 715), `y = 475`. A frame counter
(`0x8151cd38`) steps once a call and wraps from 64 to 0; triangle `i` gets phase `(counter + 7 i) mod 64` and
`p = clamp((1 - fraction) * 7 - i, 0, 1)` (the lower clamp is `vcmpe s17, #0; vmov.lt` at `0x8105d266`).

- **Pulse** `u`: `phase / 10` below 10, `(64 - phase) / 10` above 54, else 1.
- **Shape**: three quads between an outer triangle of radius `28 u` and an inner one of radius `14 u p - 1`, corners
  at `a0 + k * 2pi/3` (`2.0943935`). Even `i`: `a0 = 0`, centre `y`. Odd `i`: `a0 = pi/3` (`1.0471967`), centre
  `y + 14`. So `p = 1` is a hollow triangle and `p = 0` a solid one.
- **Reading**: the triangles are hollow at the start and **fill in from the left** (`i = 6`) as the load runs, each
  breathing on a 64-draw cycle, seven draws apart. At 20 % (`(1 - 0.2) * 7 = 5.6`) five are hollow, one part filled,
  one solid, as the frame shows; at 98 % all are solid.

Confidence 85 (operands read; the fill direction and the hollow and solid states match frames at 8, 20, 61 and 98 %).

### The meter's law

The object (`g_LoadingScreen`, `0x815248c8`): `+0x88` shown, `+0x8c` ceiling, `+0x90` rate, `+0x94` the rate restored
on reaching the ceiling, all in columns of 166.

- **Every draw** (kind 4 and 7): `shown += rate`; when `shown > ceiling`, `shown = ceiling` and `rate = +0x94`.
- **Draw rate**: the loading thread (`0x81284398`) sleeps 10 ms (`0x2710` to `sceKernelDelayThread`), draws and
  flips. Measured 30.09 and 30.10 draws a second (199 steps of the rate in 6.61 s), and the thread ticks the front
  end with 1/30 s. On Vita3K; a real Vita was not measured (confidence 75 that it is 30 there too).
- **`LoadingScreen_Start`**: shown `0`, ceiling `94.62` (57 %), rate and reset `0.115`.
- **`LoadingScreen_SetCeiling(f)`** (`0x81284550`): `old_gap = ceiling - shown`; `ceiling = f * 166`; when
  `old_gap > 0` and `shown > 0`, `rate *= (old_gap + new_gap) / new_gap` with `new_gap = f * 166 - shown`. Five
  callers, three values:

  | caller | `f` | literal |
  | --- | ---: | --- |
  | `0x8121bfb8` in `0x8121bce2` | 0.75 | `vmov.f32 s0, #0.75` |
  | `0x812ad20e` in `0x812ad1dc`, `0x8115f408` in `0x8115f3de` | 0.95 | `0x3f733333` |
  | `0x81177e78` in `0x81177cd8`, `0x812af4f6` in `0x812af492` | 0.98 | `0x3f7ae148` |

  Which loader step each caller is was **not read** (the live order was 0.75, 0.95, 0.98, between 1.0 and 2.8 s after
  the start).
- **`LoadingScreen_SetRate(r)`** (`0x812845b8`): rate and reset `= r`. The race manager's constructor calls it with
  `LoadingScreen_GetRate() * (8.0 / mode+0x188)` (`mode+0x188` read `8` on Event 2, so the rate was kept and frozen).
  A restart (`0x8115f09a`, the `InGame Restart` and next-race path) calls it with `0.5` after its own
  `LoadingScreen_Start(4)` unless the mode word is 4 or `0x11`.
- **`LoadingScreen_SetProgress(f, keep_max)`** (`0x8128474c`): shown and ceiling `= f * 166` (the larger of that and
  shown when `keep_max`). Called with `(1.0, 0)` when the load is done; `LoadingScreen_Stop` (`0x812846cc`) then waits
  for the thread and frees the object.

The rescale was checked against the live values, to the last printed digit:

| run | shown at the call | ceiling | predicted rate | read rate |
| --- | ---: | --- | ---: | ---: |
| B | 4.140 | 94.62 -> 124.5 | `0.115 * (90.48 + 120.36) / 120.36` = 0.20145 | 0.20145 |
| B | 9.982 | 124.5 -> 157.7 | `0.20145 * (114.518 + 147.718) / 147.718` = 0.35762 | 0.35762 |
| B | 17.135 | 157.7 -> 162.68 | `0.35762 * (140.565 + 145.545) / 145.545` = 0.70301 | 0.70301 |
| C | 3.565, 7.194, 11.139 | the same three | 0.20159, 0.35871, 0.70562 | 0.20159, 0.35871, 0.70562 |

So on a campaign race the readout climbs slowly to about 10 % in three seconds, then at 21 columns a second (12.7 %
a second) to 98 %, holds there until the load is done, and shows 100 % for an instant. **The earlier "the ceiling is
57 % until the loader raises it" is the start value only**; the 56 % a stalled Vita3K load sat at is a load that
never reached the first `SetCeiling`. Confidence 92 (law read, and reproduced numerically on two runs).

## Names

| address | name | confidence |
| --- | --- | ---: |
| `0x815248c8` | `g_LoadingScreen` | 90 |
| `0x81284550` | `LoadingScreen_SetCeiling` | 90 |
| `0x812845b8` | `LoadingScreen_SetRate` | 88 |
| `0x81284794` | `LoadingScreen_GetRate` | 88 |
| `0x8128474c` | `LoadingScreen_SetProgress` | 82 |
| `0x812846cc` | `LoadingScreen_Stop` | 78 |
| `0x8105d4a8` | `FrontEnd_DrawLoadingMeter` | 85 |
| `0x8105d1f0` | `FrontEnd_DrawLoadingMeterTriangle` | 85 |
| `0x81051820` | `CampaignEventCard_EnterLoad` | 80 |
| `0x810517ae` | `CampaignEventCard_ShowModePage` | 82 |
| `0x8104fc5a` | `CampaignEventCard_DrawModePage` | 85 |
| `0x8106dd36` | `GameTip_DrawPanel` | 85 |
| `0x8106d1e6` | `GameTip_Select` | 80 |
| `0x8106ce84` | `GameTip_Pick` | 80 |
| `0x8151d230` | `g_GameTipTable` | 88 |
| `0x812bd128` | `GameModeAttract_Construct` | 88 |

## Against Omega

**Checked, differs: Omega never starts this screen.** The base `eboot.bin`'s `LoadingScreen_Start` (`0x01708fe0`,
[ps4-omega-eu/loading-screen-start.md](../ps4-omega-eu/loading-screen-start.md)) is called from six sites with the
kinds 8, 7, 8, 7, 9 and 0, the 1.07 patch's from five with 10, 9, 10, 9 and 0: its state 5 (this kind 4) has no
caller in either build, so nothing on this page is drawn by Omega, whose race load is HD's feature screen. Omega also
ships **no tip art** (no entry under a `tips/` directory in any of its nine archives), so the tip panel has nothing to
draw there. The meter's start values are the same in Omega's starter (ceiling `0x42bd3d70`, rate `0x3deb851f`).

## Not read

1. `mode+0x188`, the divisor of the race manager's rate (`8.0 / n`): an `int`, `1` from `GameModeBase_Construct`, `8`
   from the attract constructor and `8` live on an eight-craft race. Probably the craft count (confidence 60). Read
   recipe: `scripts/vita-2048-race-load.py` on a Zone event (its `f188` sample), or the writers of `param_1[0x62]` in
   the `GameMode_*` constructors.
2. Which loader step each `LoadingScreen_SetCeiling` caller is (addresses in the table above).
3. `0x8106d3ce`, the controls-diagram variant of the tip panel (rows 34 to 36), and the multiplayer "waiting for"
   branch of the mode page.
4. A load on a real Vita (the 30 draws a second are Vita3K's).
