# Omega's Team Selection poses its craft by rest values, with no fit and no settle

2026-11-19, lane `hd-ship-craft`. Read statically (GUI bridge, program
`/omega/eboot-ps4-omega-eu-v107.bin`, the **1.07 patch** build) and checked against one shadPS4
frame of the screen. **Every address on this page is in the patch eboot**, not the base one
`names.tsv` covers ([patch-v107-addresses.md](patch-v107-addresses.md)), so no name is added to
`names.tsv`; the names below are descriptive. Omega's `Team_Selection_Definition.xml` is HD's file
in HD's dialect (`<Model name="ShipModel">` authors `OriginX=960 OriginY=540 nearZ=0.1 x=0.5 y=0.1
z=-2.05 RotX=0.4 RotY=-0.5 ObeySafeZone=true`, from `data09.psarc`,
`data/plugins/frontend/gui/team_selection_definition.xml`); this page is what the screen's code
does to that widget.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x015a3b80` | `TeamSelectionScreen_Update` | 80 (same constant run and `pos%d` fit as HD's `0x00254348`, [team-selection-model.md](../ps3-hdfury-eu/team-selection-model.md)) |
| `0x015a89d0` | `TeamSelectionScreen_Init` | 65 (stores the rest constants below; the rest of it unread) |
| `0x01492080` | `ModelItem_RenderScene` | 80 (`tanf(fov * pi / 360)`, as HD's `0x001d35e8`) |
| `0x01618bb0` | `FeModel_DrawAtOrigin` | 60 (takes `OriginX/Y` against `960`, `540`; 3,345 decompiled lines, not read past that) |

## What the update writes over the widget, every frame

`item = *(st + 0x8e * 4)` (the `ShipModel` item; `st` is the screen). All values at rest, the
player's turn, pan and zoom (`st[0x7aa..0x7b0]`, the pad axes) being zero. Confidence 85.

| Item field | Written | Source |
| --- | --- | --- |
| rotation X `+0x134` | `st[0x7ac] + 0.43076983` | the literal in the update (line 454 of the decompile) |
| rotation Y `+0x138` | `st[0x7ad] - 0.4437625` | same |
| position `+0x128`, `+0x12c`, `+0x130` | `0.4 - st[0x7af]`, `0.1 - st[0x7b0]`, `-2.05` | `st[0x7b7..0x7b9]` are set once by `TeamSelectionScreen_Init` from the four floats at `0x01854b20` (`13.0`, `0.4`, `0.1`, `-2.05`; the first is the fit's depth target) |
| scale `+0x144`, `+0x148`, `+0x14c` | `st[0x7ba]` = `0.1` (`0x3dcccccd`, stored by Init at `st + 0x1ee8`) | the update blends it towards `0.75` or `1` and the position towards another with the zoom axis `st[0x7be]`, which is `0` at rest |
| lens `+0x158` | `100.0` (`0x42c80000`) | `ModelItem_RenderScene` reads `+0x158` as `fov` (`param_1[0x2b]`) and takes `tanf(fov * 0.008726646)`: a **100 degree** vertical field |

The origin (`960`, `540`), `nearZ` (`0.1`) and `ObeySafeZone` stay the widget's. Where the fields
sit against HD's item: rotation `+0xb8` there, `+0x134` here (a 0x7c shift, which holds for the
position, rotation and the three scales and breaks at `+0x118`, where Omega keeps a pointer).

## The fit runs and nothing reads it

The update runs HD's depth fit (the eight corners, the running minimum and maximum, the window
`-389..389`, `-535..535`, the floor `11.0`, the `pos%d` debug names) with **another projector**: the
matrix it builds has `0.4717`, `0.8393` on the diagonal, which is `tan(50 degrees)` in the item's
own lens, the 100 degree field above (HD's is `1 / tan(0.5)`, 57.3 degrees). It writes the targets
`st[0x7b4..0x7b6]` and eases them into `st[0x7bb..0x7bd]` at `0.0625`, `0.0625`, `0.125`.

**No instruction reads `st[0x7bb..0x7bd]`**, and none the fit's inputs for any other purpose:
every operand search for `+ 0x1eec`, `+ 0x1ef0`, `+ 0x1ef4` finds only the update's own load and
store, and the wider loads that could cover them (`+ 0x1ec8`, `+ 0x1ed0`, `+ 0x1ed4`, `+ 0x1ee4`,
`+ 0x1ee8`) find only the update and the Init store. The one other hit for `0x1ee8`, in
`FUN_0163bcd0`, is a `lea` in a destructor of another class (a run of 0x318-stride members from
`+0x15a0`). Confidence 78 (static, one function's worth of operand searches; a reader through a
pointer to the field block would not show).

So **Omega's craft is not framed by a depth fit and a step has no settle**: the craft is drawn at
the rest pose above whatever model is selected. The earlier reading ("the same state, so a step
plays HD's law", `ship_preview_settle`, **inherited from HD**) is withdrawn.

## What is wired

`oag_omega::TITLE.front_end.ship_preview_pose` (`oag_title::ShipPose`): rotation, position, scale
and lens as read, over the widget's own origin and `nearZ`
(`oag_game::preview::ship_model::matrices`, the Safe Area scaling the picture and the origin as
HD's does). `ship_preview_fit` is `None`, and `ship_preview_settle` is gone from `oag_title` (Omega
was its only user).

## The 2.5 nobody explains

The values above, drawn as read (100 degrees, scale `0.1`, depth `2.05`), put Feisar's concept
hull **2.5 times smaller** than the original's frame. The frame
(`data/reference/compare-page/omega/menu-ship-select/theirs.png`, shadPS4, a fresh profile,
Feisar `concept1`, Safe Area not recorded) fixes everything else: with the rotation, position and
scale as read the hull's shape and foreshortening land on the frame's, which depend only on
`scale / depth`, and only the overall size needs a lens of about 51 degrees (`tan 25.5`) instead
of 100 (`tan 50`), a ratio of **2.50** (sweep: IoU of the craft's pixels 0.31 at 57.3, 0.61 at
52, 0.72 at 51, 0.61 at 50, 0.50 at 49 degrees). The factor is round, which suggests a read
constant, and `ModelItem_RenderScene` (read whole) carries none, and no `2.5` or `0x40200000` literal appears in
`FeModel_DrawAtOrigin`'s decompile either (its offset is `(x - 960) / 960`, `(y - 540) / -540`;
the 3,345 lines were searched, not read through). **Wired as `ShipPose::zoom = 2.5`, a zoom of the finished picture about
the widget's origin: fitted to that one frame, no confidence score, its cause open.** On this
screen the origin is the screen's centre, so it is the same picture as a 51 degree lens.

Residuals against the frame (the craft's pixels against the page; the frame is
slightly letterboxed by shadPS4): left/right 1029/1357 against 1030/1357, top/bottom 351/691
against 358/685 (the vertical extent 340 against 327: 4 per cent).

## The frame, three states

Pair `omega/menu-ship-select`, mean absolute grey difference of unblurred crops (the page's
scorer), against the same original frame. The committed pair drew the wrong team (the cursor
sat on Assegai with every craft unlocked; the original's fresh profile sits on Feisar
`concept1`), so the selection and the pose are separated:

| State | Whole frame | `SHIP MODEL` frame (700,150 1060x610) | Craft (980,320 440x420) | Craft edges (left, right, top, bottom) |
| --- | --- | --- | --- | --- |
| The committed pair (Assegai, all unlocked, main's pose) | 29.18 | 42.32 | 63.20 | |
| Feisar `concept1`, locked profile, main's pose (HD's authored `z=-24`, a fitted lens) | 26.13 | 32.73 | 43.36 | 796, 1703, 192, 687 |
| The same selection, the update's pose | **20.03** | **13.17** | **19.63** | **1029, 1357, 351, 691** |
| The original | | | | 1030, 1357, 358, 685 |

What is left in the frame is not the craft: the page's ghost-craft backdrop against the original's
smeared one, the hexagon column's contents (ours draws the cell icons on red, the original
greys), and the statistics bars' colours (the red and orange). Those are other items.

A long hull (Qirex `_c1`, 4.5 long) and a 2048-era hull (`Feisar2048` `1`) at the same pose sit
inside the `SHIP MODEL` frame with margin, so the fixed pose does not push a long hull out.

Pinned by `hd_ship_preview_ground_truth::omega_poses_feisars_concept_hull_where_the_original_draws_it`
(the hull's vertices through `matrices` against those edges, within 10 pixels: it guards the wiring,
and the zoom was fitted to the same frame) and `omega_draws_every_teams_ps4_hull_at_the_updates_rest_pose`
(the widget carries the pose).

## HD and 2048

- **HD: checked, differs.** HD's update writes the same kinds of field with a depth fit that the
  draw does read, and a 57.3 degree lens
  ([team-selection-model.md](../ps3-hdfury-eu/team-selection-model.md)); Omega's rest rotation,
  position, scale and lens are its own constants.
- **2048: not checkable.** 2048 authors no `Team Selection` screen.
