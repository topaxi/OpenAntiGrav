# The 2048 campaign map under a pad: focus, neighbour pick, scroll and look

2026-11-13, member `omega-map-pad-re`. Functions in `eboot.bin` (Omega Collection
PS4, `/omega/eboot-ps4-omega-eu.bin`). **Static reading only**: shadPS4 runs Omega
but gives no memory readout, so nothing here is above the "decompilation only,
consistent call sites" ceiling of 84.

This corrects a negative. Omega's `Campaign2048_Definition.xml` carries no focus
or pad attribute, and an earlier reading concluded "nothing authored". The
behaviour is in code: Omega's `TouchCampaign` item reads the d-pad, which Vita
2048's does not (`../vita-2048-eu-v104/pad-focus.md`: `TouchCampaign`'s update
`0x810f2f04` never calls the button query).

## Names

| Address | Name | Confidence | What |
| --- | --- | --- | --- |
| `0x014df620` | `TouchCampaign_Update` | 78 | `TouchCampaign` vtable (`0x01920990`) slot 3; reads the d-pad and moves focus |
| `0x013de330` | `Canvas_FindButtonInDirection` | 84 | the neighbour pick (law below); its only caller is `TouchCampaign_Update` |
| `0x014e32c0` | `TouchCampaign_BuildEventButtons` | 76 | vtable slot 11; one canvas button per event, prints `Added %d 2048 canvas buttons` |
| `0x014e1b10` | `TouchCampaign_UpdateEvent` | 74 | per event per frame: the focus pulse, touch focus, the card's three touch rects |
| `0x014e4de0` | `TouchCampaign_OnEventFocused` | 72 | an event button's callback: select sounds, select the event, scroll to it |
| `0x014e5220` | `TouchCampaign_SetActive` | 62 | screen becomes active: picks the start event, focuses it, centres the view |
| `0x014e0f50` | `TouchCampaign_FindStartEvent` | 72 | which event holds focus first |
| `0x014e0be0` | `TouchCampaign_SetScrollLimits` | 76 | the `TouchScroll` limits from the events' projected bounds |
| `0x013da000` | `Canvas3D_Update` | 72 | `FE3DCanvas` update: plays the focus ripple on the lattice |
| `0x013d7ec0` | `CanvasButton_Activate` | 76 | canvas button vtable (`0x01918f60`) slot 0: calls `button+0x10(button, button+0x18, arg)` |
| `0x013d8160` | `CanvasButton_GetSeason` | 72 | slot `+0x50`: the event's `+0x308`, its season |
| `0x01749330` | `Pad_ActionPressed` | 84 | pad vtable (`0x0192cba0`) slot `+0x60`: `pad+0xe70+action`, a press edge |
| `0x01749310` | `Pad_ActionHeld` | 84 | slot `+0x58`: `pad+0xe10+action`, held |
| `0x017494d0` | `Pad_ActionByName` | 84 | action names to indices |
| `0x0151d530` | `Campaign2048Screen_Update` | 72 | the `Campaign2048` screen class's update (vtable `0x01922398` slot 3, registered by `0x0151ec20`) |

## Pad actions (confidence 84)

`Pad_ActionByName` maps `up` 0, `down` 1, `left` 2, `right` 3, `circle` 4,
`triangle` 5, `cross` 6, `square` 7, `l1` 8, `r1` 9, `l2` 10, `r2` 11, `l3` 12,
`r3` 13, `select` 14, `start` 15, `any` 31. `forward` and `activate` are cross,
or circle when `DAT_01946e58` is set (the confirm-button swap); `backward` and
`cancel` are the other one.

`Pad_UpdateState` (`0x01747c90`) fills `pad+0xe10` from `scePadReadState`: d-pad
up `0x10` to action 0, down `0x40` to 1, left `0x80` to 2, right to 3. The left
stick sets actions `0x11` to `0x14` (`pad+0xe21..+0xe24`), **not** 0 to 3, so the
stick does not move the map's focus. Press edge `pad+0xe70` = held and not held
last frame. The action table at `0x01946e60` (stride `0x18`, names `Up`, `Down`,
`Left`, `Right` at `0x0183a21a`) flags actions 0 to 3 with bit 8: **auto-repeat**.
With `pad+0x10cc` set (the constructor sets it), a held direction re-fires after
`pad+0x10d0` = 0.4 s and then every `pad+0x10d4` = 0.12 s (constructor
`0x017497c0` writes `0x3ecccccd` and `0x3df5c28f`).

## What holds focus (confidence 80)

A **canvas button per event**. `TouchCampaign_BuildEventButtons` allocates one
`0x80`-byte button for each event of the season list and registers it in the
canvas's 700-slot table `canvas+0x5e148` (`canvas` = `DAT_01aeb330`, the
`FE3DCanvas`). Button fields read here:

| Offset | What |
| --- | --- |
| `+0x10`, `+0x18` | callback `TouchCampaign_OnEventFocused` and its owner, the `TouchCampaign` |
| `+0x20` | the event |
| `+0x34`, `+0x38` | lattice column and row of the button's corner |
| `+0x3c`, `+0x40` | its width and height in lattice cells: 3 x 2 by default, 7 x 4 for event types 1, 2, 9, 10, 2 x 2 or 2 x 1 for others, 11 x 6 for type 11 |
| `+0x50` | float size, 8.0 by default (5.0, 7.0, 10.0 for others): the ripple radius below is half of it |
| `+0x5c` | state; only state 2 can take focus |

The focused button is `canvas+0x5e138`. `CanvasLabel`s also get a button
(`"Dummy label button"`, `0x013e0250`) but with no event (`+0x20` = 0), and the
pick skips those: labels never hold focus.

## The neighbour pick: `Canvas_FindButtonInDirection(canvas, action)` (confidence 84)

For every registered button that has an event, is not the current one, is in
state 2, and whose event's season (`CanvasButton_GetSeason`) equals
`canvas+0x5f880` (the season on show; so **focus never crosses seasons**):

```
dx = (b.x - cur.x) + (b.w - cur.w) * 0.5        // centre to centre, lattice cells
dy = (b.y - cur.y) + (b.h - cur.h) * 0.5
a  = fmod(atan2(dx, dy) * 57.295776 + 360, 360)  // 0 = +row, 90 = +column
in cone:  right (3): 25 <= a < 155     up (0):   115 <= a < 245
          left (2):  205 <= a < 335    down (1): a >= 295 or a < 65
pick the in-cone button with the smallest sqrt(dx*dx + dy*dy); none: stay
```

Evidence: at `0x013de480` the call is `atan2f` with `XMM0` = the column delta and
`XMM1` = the row delta, so the angle is measured from the row axis. The constants
`0x01803114..0x0180314c` read `FLT_MAX, 0.5, 57.29578, 360, 155, 25, 245, 115,
335, 205, 295, 65`. Each cone is 130 degrees wide, so neighbouring cones overlap
by 40 degrees; the score is plain Euclidean distance with no sideways penalty.
"Up" is the smaller row. The fields are integer lattice cells, so a port uses
the lattice column and row, not screen pixels.

`TouchCampaign_Update` asks the pad in the order left (2), right (3), up (0),
down (1) and takes the first press, so at most one step per frame. The pad is
the item's own port (`item+0xd0`), or the first pad that reports connected
(`pad vtable+0x70`) and active (`pad+0xf20`) when that is -1.

## What a focus change does (confidence 74)

The same sequence runs after the d-pad pick (`0x014df9a8`), after a touch
(`TouchCampaign_UpdateEvent`, `0x014e1dde`), and on entry (`TouchCampaign_SetActive`,
`0x014e5bac`):

1. A button in state 3 is set to state 2 and its event's `vtable+0xe0(2, 0)` is called.
2. `canvas+0x5e138` = the button; `CanvasButton_Activate(button, canvas, 1)`,
   which calls `TouchCampaign_OnEventFocused(button, campaign, 1)`:
   - plays `newGUI_select` and `newGUI_trans03`;
   - `campaign+0x108` = the event (the selected event: the screen's info column,
     `RecordsButton` and the launch path all read it);
   - sets the `TouchScroll` (`campaign+0x158`) to scroll to the event: target
     `+0x144`/`+0x148` = view half size minus the event's projected position,
     `+0x140` = 1, speed `+0x14c` = 5.0. **The view follows focus.**
   - calls `0x013a8830(event, 1)`, which builds the event's pre-race option list
     (`DAT_01a7b410`, count `DAT_01a7b43c`).
3. A ripple starts: `canvas+0x5f8a0` = 1, `+0x5f8a4` = the button's position
   (`vtable+0x28`), `+0x5f8b0` = radius = `button+0x50` * 0.5, `+0x5f8b4` = 2.5 s,
   `+0x5f8b8` = 0 (elapsed).

## How focus is drawn (confidence 72)

Two effects, both on the canvas lattice, no sprite and no cursor:

- **The focused event pulses** (`TouchCampaign_UpdateEvent`, `0x014e1c2f`): while
  its button is `canvas+0x5e138`, each of the button's three colour bytes is
  `clamp(base + sin(phase) * 32.5 + 32.5, 0, 255)`, `base` the low byte of
  `DAT_01aeb688` (unread at rest, read recipe below). `phase` is
  `campaign+0x208`, advanced by `dt * 5` and wrapped at 2 pi in
  `TouchCampaign_Update`'s default state: a period of 1.257 s, brightening by up
  to 65 levels. Its lattice tiles (`button+0x68`, count `+0x60`) get alpha 0xff.
- **A ripple runs out from it for 2.5 s** (`Canvas3D_Update`, `0x013da619`): for
  each lattice tile within radius `r` of the focus point, at distance squared `d2`
  and elapsed `t`:

  ```
  alpha = (sin(-5.5 t - pi + 0.15 d2) + 1) * 128
          * min(1, 4 t) * clamp((2.5 - t) / 2.5) * clamp((r*r - d2) / (r*r))
  ```

  written to the tile's alpha (`canvas+0x147 + 4 * tile`, tile = `row * 401 + col`)
  for tiles whose priority (`canvas+0x2f124`) is below 5. Position and radius are canvas units; the loop divides by the tile pitch
  `DAT_01aeb850`/`DAT_01aeb854` (runtime globals, unread) to reach tiles. Every tile's alpha then
  decays each frame by `floor(acc * 690)` levels (`DAT_01aeb6d8`), so the wave
  fades behind itself.

Neither effect names `Orange2048`; the colour is the event's own canvas colour,
brightened.

## Where focus starts (confidence 72)

`TouchCampaign_FindStartEvent`: the season's remembered event
(`DAT_01e06780[season]`, seasons `0x800..0x802`) when set; otherwise the event
last raced (`DAT_02039628+0x4e8`) if it is in this season, walking forward along
`event+0x3a8` while the event is completed (`event+0x3c0 > 2`); otherwise the
season's `Event 1`, walked forward the same way. `TouchCampaign_SetActive`
focuses it and scrolls the view to centre it.

## Buttons (confidence 60 to 75)

- **Directions**: d-pad only, with the repeat above (84).
- **Triangle**: `Campaign2048Screen_Update` (`0x0151dd18`): `Pad_ActionPressed(5)`
  with an event selected opens `Campaign Race Records 2048` (75). The XML's
  `RecordsButton` is the prompt (`FE_RECORDS`, button-font glyph `Ϊ`, at 900,988).
  The XML also carries `DifficultyButton` ("Change Difficulty", glyph `δ`, same
  place); its handler was not found.
- **Circle**: the screen's `<Redirect><Values forward="backward" backward="none">`
  and `DefaultPrevious="Grid Selection 2048"`: back to the season chooser (60, read
  from the XML and the HD screen base, not traced in this class).
- **Cross / launch: open.** The launch body is in `TouchCampaign_UpdateEvent`:
  when the selected event's card is up, three rects at x 552, 682, 812, y 422,
  142 x 116 (960 x 544 space) are hit-tested by `0x013b8ef0` (taps) and
  `0x013b8ca0` (holds); the 812 rect checks `PlayerLivery`, sets `InGame2048`'s
  `PreRaceCallout`, plays `newGUI_confirm` and follows the item's redirect
  `Launch 2048`; the 552 rect writes the event to `DAT_02039628+0x4e8` and
  redirects. Both tests read only the pad manager's touch lists
  (`mgr+0x968`/`+0x96c` and `mgr+0xa70`/`+0xa74`, i.e. pad 0's `+0x8e8` and `+0x9f0`),
  and no non-zero writer of those lists was found statically. No `Pad_ActionPressed(6)`
  or `(4)` call sits in any `TouchCampaign` function (the four `+0x60` calls in
  `0x014d0000..0x014f0000` are the directions). Next: the HD screen base's
  forward handling (`0x013c5e70`, called at the end of `TouchCampaign_Update`),
  and the writer of `pad+0x8e8` (`Game_Main`'s five `LEA RBX,[R12+0x8e8]` at
  `0x0163f108..0x0163f2e8`). A live check needs a shadPS4 frame sequence: press
  Cross on a focused event and see whether the race loads.

## PS4-only, not for the port

- `pad+0xb4a` with `pad+0xb4c` in 0..3 synthesises an up, right, down or left
  press (table `0x0183efc0` = 0, 3, 1, 2): a touchpad swipe.
- Two touchpad fingers zoom the canvas: `TouchCampaign_Update`'s default state
  adds `0.03 * (distance change)` to `canvas+0x120`, clamped to
  `[canvas+0x5f88c, canvas+0x5f890]`.
- With no event selected, a touchpad touch (`mgr+0xb79`, list `mgr+0xaf4`) starts a
  ripple on the lattice under it (`Canvas3D_Update`, `0x013dab10`).

## Unread values and recipes

- `DAT_01aeb688` (the pulse's base colour): a runtime global whose writer was not read. Read recipe: `get_xrefs_to 0x01aeb688` on this
  program, decompile the writer.
- `canvas+0x5f88c`/`+0x5f890` (zoom limits) and the default zoom: same recipe on
  those offsets in `0x013db460` (the canvas constructor writes `DAT_01aeb330`).

## 2048 delta

Vita 2048's `TouchCampaign` update (`0x810f2f04`) answers touch only
([pad-focus.md](../vita-2048-eu-v104/pad-focus.md)); Omega's adds the d-pad
focus, the scroll-to-focus, the pulse and the ripple. The XML is not the place
either title authors it. The maintainer's rule (one shared map, Omega's
behaviour on both) makes this the law for 2048 too:
[`2048-frontend.md`](../../../formats/2048-frontend.md), "Controller focus".
