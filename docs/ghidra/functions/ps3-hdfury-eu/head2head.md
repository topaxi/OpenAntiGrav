# Head to Head on HD/Fury, and the census on Pure, 2048 and Omega (2026-10-14, `h2h-re`)

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`), with the shipped XML of
`hdfury-ps3-eu-dec.iso`. Pulse's Head to Head, single and two-person, is on
[`../psp-pulse-usa/head2head.md`](../psp-pulse-usa/head2head.md) and
[`../ps2-pulse-eu/head2head.md`](../ps2-pulse-eu/head2head.md).

## Answer

**HD/Fury carries Head2Head in its executable and offers it nowhere.** Every shipped
menu that could set the mode lists it as `<aEntry>`, an element the front end's list
reader never reads; no campaign cell is `Head2Head`. So HD has no Head to Head a player
can reach, single or two-person, and the authored `splitscreen_hud` files' `HeadToHeadBar`
is a widget left over from Pulse's layout, not a two-person Head to Head mode. Confidence
85: the XML as shipped across all seven archives, plus the reader's own string table;
not walked live on RPCS3 on this pass.

## The evidence

**The mode is in the executable.** `search_strings` on the EBOOT finds `Head2Head`
(`0x00779068`) and `Multiplayer Head2Head` (`0x00779100`) in the mode-name block Pulse
also has, `HeadToHeadBar` (`0x0077fb00`), `HeadToHead` (`0x0078fb98`, the track
`ModeModifiers` attribute), `Data\FE\Images\head2Head_BW.gtf` (`0x00793258`) and
`/Data/Fe/Images/Head2Head_Icon` (`0x007a0660`). HD's campaign code still names mode 9 in
its difficulty-button set ([race-campaign.md](race-campaign.md)) and its loyalty
multiplier ([endrace-loyalty.md](endrace-loyalty.md)), and `stats_definition.xml`
(`DATA05`) charts a `Head2Head` slice (`StatID="12"`).

**No menu offers it.** `data/plugins/frontend/gui/racebox_definition.xml`, `Single
Player` screen, `<List name="Mode">`:

| Archive | Entries |
| --- | --- |
| `DATA02` (HD) | `Arcade`, `<aEntry idstring="Head2Head">`, `Time Trial`, `Speed Lap`, `Tournament`, `Zone` |
| `DATA03` (HD) | the same, then `<aEntry idstring="Ghost Viewer">` |
| `DATA00` (Fury) | `Arcade`, `<aEntry idstring="Head2Head">`, `Time Trial`, `Speed Lap`, `Eliminator`, `Zone`, `NitroBattle`, `Detonator`, `Tournament`, `<aEntry idstring="Ghost Viewer">` |

`network_definition.xml`'s online `Mode` list does the same in `DATA02`, `DATA03` and
`DATA04` (`<aEntry idstring="Head2Head" value="Multiplayer Head2Head">`, beside an
`<aEntry>` Elimination), and Fury's `DATA00` copy drops the line. **The EBOOT has no
`aEntry` string at all** (`search_strings`), while `Entry` appears many times, so the list
reader only ever matches `Entry` and these lines are switched off in the data. The same
RACEBOX page is where HD authors its local two-person race (`NumberOfPlayers`,
`SplitScreen`, [hd-frontend.md](../../../formats/hd-frontend.md#racebox-authors-local-split-screen-a-players-list-and-a-cut)),
so a two-person Head to Head would have been chosen here, and it is not offered.

**No campaign cell.** None of HD's 167 grid cells is `Head2Head` (checked 2026-10-08,
`docs/overview/status.md`).

**What the strings say it was meant to be.** HD's English `MSC_EVENT_HTH` reads "Head to
head: a two player single race of intense rivalry...", where Pulse's speaks of being
"pitted against an opponent". The design note survives; the mode does not ship.

## Census: Pure, 2048, Omega

| Title | Head to Head? | Evidence | Conf |
| --- | --- | --- | ---: |
| **Pure** (`psp-pure-eu` `BOOT.BIN`) | **None, single or multi.** | `strings`: the race managers are `SPArcade`, `SPTimeTrial`, `SPTournament`, `SPZone`, `SPFreePlay`, `Demo`, `AIBatch`, `MPArcade`, `MPTimeTrial`, `MPTournament`; no `Head2Head`, `HeadToHead` or `HeadToHeadBar` string at all | 85 |
| **2048** (`vita-2048-eu-v104`) | **None reachable.** The mode-name strings and `HeadToHeadBar` are inherited, nothing sets the mode. | `search_strings`: `Head2Head` (`0x81419094`), `Multiplayer Head2Head` (`0x814193b4`), `HeadToHeadBar` (`0x8147bce4`); the six `GameMode_*` classes are `ArcadeRace`, `CheckPointRace`, `EliminatorRace`, `SpeedLapRace`, `ZombieRace`, `ZoneRace` ([2048-campaign.md](../../../formats/2048-campaign.md)); 2048 has no racebox; no `Head2Head` cell in the 16 `data/plugins/grids/grid_NN.xml` of `data.psarc` or `data2.psarc`, which are HD-lineage files (they carry Fury's NitroBattle and Detonator) rather than 2048's own campaign, so the `GameMode_*` list is the real negative (`mode` census: Race 44, Speed Lap 34, Time Trial 22, Elimination 18, NitroBattle 13, Detonator, Tournament and Zone 12 each); outside the language files only HUD position files name it | 82 |
| **Omega** (`ps4-omega-eu`, patch 1.07) | **None reachable**, HD's case exactly. | `search_strings`: `Multiplayer Head2Head` (`0x0182c0a6`), `Head2Head` (`0x0183085a`), `HeadToHeadBar` (`0x018115c3`); `racebox_definition.xml` in `data00`, `data08` and `data09` all carry `<aEntry idstring="Head2Head" intvalue="1">`; the 20 grid files in each of `data00`, `data08`, `data09` hold the same mode census as 2048's, no `Head2Head` | 82 |

**2048 and Omega checked against each other** (CLAUDE.md's rule): **checked, applies**:
both inherit the mode name and the HUD widget and neither authors a route to the mode,
and their grid files carry the same cell-mode census. Omega's executable
(`/omega/eboot-ps4-omega-eu.bin`, the base package's) has `Entry` (`0x01822c30`) and no
`aEntry` string, as HD's.

**A correction to `docs/overview/status.md`:** its Omega cell said `Head2Head` cells
launch with one opponent. No Omega grid file has a `Head2Head` cell; the test it cited
holds a rule over every cell and is true of zero `Head2Head` cells.

## The mode 9 writer: `Hud_UpdatePositionCounter` (2026-10-16, `hd-h2h-bar`)

**Binary:** `/hdfury/EBOOT-ps3-hdfury-eu.elf`. No new names (both functions are already
named). Static only: **no live RPCS3 check**, and no menu reaches mode 9 (above), so a live
check would need the mode forced. Omega: **ported** (it reads HD's layouts; the bar draws there too, `omega_draws_hds_rows_and_gap_label`).

### Correction: HD does author a gap label

The 2026-10-16 `h2h-readout-titles` pin read "no gap label" because `PositionOf`, Pulse's
name, is absent. HD's name is **`DistanceApart`**: `splitscreen_positions_screen.xml` and
`hud_positions.xml` (and the `2097_hud` / `wo3_hud` twins, `DATA00`-`DATA03`) author
`<Text name="DistanceApart"><Values string="" scale="0.6" font="HUD" align="right"
vertalign="bottom" x="180" y="104" color="FEConst->HudColour2" .../>` beside `Position`
(centred, `x=182 y=92`), `Position Outof` (`x=255 y=80`, scale 0.6) and `HeadToHeadBar`
(`x=200 y=120 width=20 height=0 color=0xB40048FF`). The test pin now asserts
`DistanceApart` present and `PositionOf` absent.

### What binds it

`Hud_BindWidgets` (`0x0009bb30`, the mode 9 / 0x12 branch near `0x0009e0ec` is inside it, not a function of its own), HUD flag `0x40`, TOC base `0x008ad4d8` (slots
`0x008a7b40..` hold the strings at `0x0077fac0..`: `Cockpit321Go`, `Position`,
`Position Outof`, `PositionTxt`, `HeadToHeadBar` (`0x0077fb00`), `DistanceApart`
(`0x0077fb10`), `PositionPanel`):

| Field | Widget | Evidence |
| --- | --- | --- |
| `hud+0x29c` | `Position` | bound `required=1` |
| `hud+0x2a0` | `Position Outof` | same |
| `hud+0x288` | `HeadToHeadBar` | slot `0x008a7b50` |
| `hud+0x284` | `DistanceApart` | slot `0x008a7b54` |

In mode 9 or 0x12 the bind hides `PositionTxt` and `PositionPanel` (`flags &= ~6`), so the
original does **not** keep the authored place: `Position` and `Position Outof` carry the two
rows. It sets both rows' scale (`+0xb8/+0xbc`: `Position` the TOC constant, `Position Outof`
`1.0`), and gives `DistanceApart` `+0xd4 = Position's +0xd4`, `+0xd8 = 1`.

### The per-frame law (`Hud_UpdatePositionCounter`, `0x00096088`, mode 9 / 0x12 tail)

Read from the decompile; constants read from memory.

- `gap = |craft0[+0x7818] - craft1[+0x7818]|` (the two ranked racers' progress floats).
- `D = clamp(2 * gap, 240.0, 720.0)` (`0x008a7934` = `0x43700000`, `0x008a7938` =
  `0x44340000`). One clamp: **no stacked variant** in HD. Against Pulse's live-measured
  `clamp(gap/2, 60, 180)` on a 480-wide HUD this is exactly 4x on a 1920-wide one.
- `Position Outof` `+0xb4 = D`. (Text `+0xb0/+0xb4` are x/y: the PosTag branch shifts `+0xb0`
  by 150, and the 0xad tail derives `Position`'s `+0xb0` from `Outof`'s.)
- Label text `"%s%3.0fm"` (`0x0077f100`), sign `"-"` (`0x0077f0f0`, player leads) or `"+"`
  (`0x0077f0f8`), written to `DistanceApart`. Green `0xff30ff30` when the player leads, red
  `0xffff3030` otherwise; the `Position`/`Outof` rows take the same colours with `0x200` toggled on
  the player's own row (Pulse's throb flag).
- Rows: `IG_HUD_1ST` / `IG_HUD_2ND` with `"%s %s"` (`0x0077f0e8`) below game mode `0x10`;
  `"PLAYER %d"` / the net name from `0x10` up. The tail below is shared.
- Bar: `bar+0xb0 = D - bar.vt[0x74]()`; label: `DistanceApart+0xb4 = (D + bar.vt[0x74]()) * 0.5`
  (TOC `0x008a764c` = 0.5). This is Pulse's relation exactly: bar height `y_second - GetY`, label at
  the midpoint.

### Confidence

| Claim | Conf | Why |
| --- | --- | --- |
| `D = clamp(2 * gap, 240, 720)` places `Position Outof`; gap = progress difference | 75 | decompile and constants, 4x agreement with Pulse's measured law; no live run |
| Label `"%s%3.0fm"`, `-`/`+`, `0xff30ff30`/`0xffff3030` | 80 | strings and colours read; identical to Pulse's live-measured set |
| `DistanceApart` is HD's gap label, midpoint of bar top and second row | 70 | name binding certain (85); midpoint formula reads as Pulse's but depends on the next row |
| **The runtime sets the bar's height: `bar+0xb0 = D - GetY()`, `+0xb0` is an `<Image>`'s Height** | **78** | settled statically 2026-10-16 (`hd-h2h-bar-size`), see the next section; not seen live. No rename, no new name |
| Mode 9 is reachable by a player | 85 | no (above) |

### The Image field map, settled statically (2026-10-16, `hd-h2h-bar-size`)

The thread's two named addresses were the wrong ones: `FUN_004ef30c` is a syscall wrapper and
`FUN_004d539c` a TOC initialiser, neither reads XML. The map comes from two functions that
already crop an `<Image>`:

- **`Hud_BindWidgets` (`0x0009bb30`), the stretch `0x0009bee0`-`0x0009bf28`** reads `DamageBar` (`hud+0x1b0`, an `<Image>` in every
  shipped HUD file): `vt[0x60]()` -> `hud+0x1dc` (H), `+0xd0` -> `hud+0x1e0` (V0), `+0xd8` -> `hud+0x1e4` (TH),
  `vt[0x74]()` -> `hud+0x1d8` (y0).
- **`Hud_UpdateShieldReadout` `0x00086750`-`0x000867ac`** writes it back: `+0xb0 = f * H`,
  SetY (`vt[0x70]`) to `y0 + (1 - f) * H * 0.5`, `+0xd8 = f * TH`, `+0xd0 = V0 + (1 - f) * TH`. A bottom-pinned
  vertical crop needs `+0xb0` to be the height (it is cropped together with the source height at `+0xd8`)
  and `vt[0x70]`/`vt[0x74]` to be SetY/GetY.

| Image field | Offset |
| --- | --- |
| width (`ThrustBar`, and the `hud+0x190` tail: `+0xac`, with source width `+0xd4` and U `+0xcc`) | `+0xac` |
| **height** | **`+0xb0`** |
| source V / source height | `+0xd0` / `+0xd8` |
| vtable | `0x60` GetHeight, `0x70` SetY, `0x74` GetY |

That is Pulse's x,y,w,h shifted by `0x10`; "could be Width" is dead. Applied to the bar: `bar+0xb0 =
D - GetY()` is a height, and the label's `(D + GetY()) * 0.5` is the midpoint of a bar running from `GetY()`
to `D`. The authored bar is `<Image name="HeadToHeadBar"><Values x="200" y="120" width="20" height="0"
color="0xB40048FF"/>` in every shipped layout (`DATA00`, `DATA02`, `DATA03`, `hud_positions`,
`splitscreen_positions` and the `2097`/`wo3` twins), with no alignment attribute, so it is top-left
anchored and `GetY()` is the authored `120`; `D >= 240` keeps its height at least `120`.

What stays inferred (hence 78, not 85): that `HeadToHeadBar` is the same `Image` class as `DamageBar` (both
are `<Image>` elements; the vtable was not read), and that `GetY` returns the local `y`. Not seen live: no
menu reaches mode 9, and the lane did not force one on RPCS3.

### 2048 / Omega

Omega **ported**: it reads HD's layouts (the same split layouts, pinned by
`hd_2048_and_omega_ship_the_same_split_layouts_540_apart`), so `DistanceApart` is there too, and the wiring
below runs on it (`omega_draws_hds_rows_and_gap_label`, looked at both cuts).
2048 authors no Head to Head widget (merged 2026-10-16): **checked, differs**.

### Wired (same day)

`oag_hud::head_to_head` takes HD's shape when a layout authors `DistanceApart` and no `PositionOf`
(layout data, no `title.name` branch): `PositionTxt`, `PositionPanel` and the place arc are hidden;
`Position` carries `1ST`, `Position Outof` carries `2ND` bottom-aligned at `clamp(2*gap, 240, 720)` (layout-local,
so the second seat's layout, 540 lower, follows); `DistanceApart` carries `-`/`+` and the gap in green/red at the
midpoint of the bar's authored top and the second row. Rows are in the layout's authored font and anchors.

- **Bar: drawn** (2026-10-16, `hd-h2h-bar-size`, confidence 78): a `20` wide solid rectangle from the authored `y = 120` to the second row, `D - 120` tall, in the authored colour `0xB40048FF`.
- **Chosen, not measured:** the rows are the ordinal alone. The original's `"%s %s"` adds the team name, and at
  `Position`'s authored size (scale 1.0, HUD font) `1ST FEISAR` ran out of its view in a first picture.
- **Top-and-bottom cut:** the clamp has no stacked variant, so the second row sits at layout y 240..720 in a view
  540 high: it fits up to a gap of 270 and is cut off by the view's scissor beyond that. The side-by-side cut
  (1080 high) fits the whole range. Evidence the original's split mode was never tuned for this mode.
- Pure and 2048 keep the borrowed gap-under-the-place readout.
- Pins: `hd_second_row_follows_the_4x_clamp_and_the_gap_uses_distance_apart`,
  `hd_colours_the_leader_green_and_the_trailer_red` (`oag-hud`); `hd_draws_its_rows_and_its_own_gap_label`,
  `omega_draws_hds_rows_and_gap_label` (`oag-game`; both now also assert one bar per view inside the view, both cuts).
