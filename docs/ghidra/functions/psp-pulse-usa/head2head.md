# Head2Head (mode 9): a two-craft campaign race, and its own HUD swap

**Binary:** `pulse-psp` `BOOT.BIN` (USA), image base `0x08804000`.

Deliverable 4 of the pass that read [tournament.md](tournament.md)'s law was never
attempted - see that page's own "Head2Head entirely" gap. This page closes it far
enough to build the mode.

## What makes a race Head2Head

**Mode 9, constructed through `ArcadeRace_Construct` - the same entry point [`Race`
(mode 3) uses](state-machine.md), `0x1a10` layout id.** Confirmed at
[state-machine.md:179](state-machine.md). Not a distinct mode object, a distinct
`g_game_mode` value into the same construction path.

**Field size is two: the player plus one AI opponent, not a design choice.** Two
independent measurements agree:

- [state-machine.md:222](state-machine.md) - the grid-size table read out of
  `FUN_0880502c`: mode 9 is the *only* row besides the eight-craft default that
  isn't `1`, and its value is `2`.
- [race-campaign.md:124](race-campaign.md) - every authored campaign cell carries
  `AICount="1"` for Head2Head, against `7` for Race/Tournament/Elimination.
- This session's own extraction (`oag-wad cat --expand`, all 16
  `Data\Plugins\grids\grid_NN.xml`, PSP USA) censuses **all 23 Head2Head cells**
  and finds `AICount="1"` on every one, no exception.

**Laps, weapons, damage: the same census, flat, no exception across all 23 cells:**

| Attribute | Value on every Head2Head cell |
| --- | --- |
| `Weapons` | `off` |
| `damage` | `on` |
| `ship` | `None` (never forces a craft) |
| `Gold Target` | `1` |
| `Silver Target` | `0` |
| `Bronze Target` | `0` |
| `laps` | `4` (Flash, Rapier) / `5` (Phantom) - **no Venom-class Head2Head cell exists** (6 Flash, 6 Rapier, 11 Phantom = 23) |

**Weapons are off and locked, not merely defaulted off.** `shield.md`'s own
`g_weapons_enabled` switch (`FUN_08896b84`, `0x08896b84`) puts mode 9 in *both*
the default-off set (`5/9/10/15/17`) and the no-override set
(`5/8/9/10/15/17`) - so even a cell that authored `Weapons="on"` (none do) could
not turn them on. Checked live on the Custom Race screen 2026-08-10 too: "Head to
Head" shows `WEAPONS: Off`, greyed, unlike Tournament's editable row. See
[shield.md:317-361](shield.md).

**Damage defaults on and is not in either locked set** (`shield.md`'s per-mode
default table, [state-machine.md:229](state-machine.md)), so the cell's own
`damage` attribute is a real setting here, authored `on` on all 23 cells.

**AI skill on a campaign launch is the cell's own `skillEasy`/`skill`/`skillHard`
lerp, which *replaces* the mode-modifier arithmetic - already read in full** at
[race-campaign.md:564-582](race-campaign.md#ai_resolveskillscale-0x08834df4---the-one-place-the-campaign-touches-this-table).
Outside a campaign cell, `AI_ResolveSkillScale` adds
`track->ModeModifiers[class].HeadToHead` to the base track skill for mode 9
specifically - a term this project has no campaign-launch reason to implement
separately, since every one of the 23 cells is a campaign cell and the lerp wins.

**Win condition: gold-or-nothing, off the same `Cell_MedalPoints`/target-triple
mechanism every other mode uses** - `1`/`0`/`0` means only finishing 1st scores
anything, corroborated by `race-campaign.md`'s existing "win or nothing" reading
and now flat-censused rather than sampled.

## What is still open: which team the opponent flies

**Not chased further this pass, on purpose - it is the same unresolved question
[grid.md](grid.md#which-team-flies-which-slot-the-id-is-a-real-field-on-a-struct-of-eight)
already carries for `Race`.** No Head2Head cell authors an opponent identity -
`ship="None"` on all 23, the same as every other mode - and `grid.md`'s own
finding is that even the code path that *does* carry a real per-entrant team id
(`Race_SpawnGrid`'s `DAT_00057c44`-keyed mechanism) has its write site
unresolved, while the path this project's Custom Race capture actually exercises
hardcodes `id = 0` for every racer. Sinking Ghidra time into a second attempt at
that same open question, for one more mode that reaches it the same way Race
does, was not this pass's job. This project's own `oag_livery::teams_for_slots`
- already labelled "this project's own rule rather than the original's" for
Race - is what Head2Head uses too; see the implementation's own doc comment.

**Grid placement of the sole opponent is extrapolated, not independently
measured for `g_racer_count == 2`.** `Race_SpawnGrid`'s compaction rule -
"bubble every assigned slot toward the high end" - is read at confidence 82
([grid.md:14-52](grid.md)), and "the local player is forced to the back" is
separately read at confidence 75 ([grid.md:72-78](grid.md)). Composing the two
puts the sole AI opponent at slot 7, immediately ahead of the player at slot 8 -
but no live capture of an actual two-craft grid exists to confirm it, the same
gap `grid.md` already flags for the general N-racer case. This project's
`start.rs` places the opponent at the same grid pose Race's slot-7 opponent
uses, labelled as extrapolated-not-measured at its own call site.

## The HUD swaps Position for a head-to-head gap readout, and it is fully decompiled but not wired

**`Arcade_HUD.xml` is shared with `Race`** (no `Head2Head_HUD.xml` exists -
`docs/ui/hud.md`'s five-layout census), but `Hud_BindWidgets` (`0x0881fbec`)
branches on the live game mode for one widget cluster:

```c
// Hud_BindWidgets, 0x08820748 (inside the "Position" bit-0x40 branch)
if (mode == 9 || mode == 0xf) {   // Head2Head, Multiplayer Head2Head
    // hides "PositionTxt" ("POS" label)
    // repoints "Position_Outof" (now the 2nd-place slot) onto "Position"'s
    // own transform, and clears its own
    // binds "HeadToHeadBar" -> stored at hud+0x22c
}
```

`s_HeadToHeadBar_08a7a0ac` (`0x08a7a0ac`) has exactly one xref, this bind site -
confirmed via `search_strings`/`get_xrefs_to`. Matches `docs/ui/hud.md`'s own
finding that `HeadToHeadBar` is a `<Image>` with no `Src` and `height="0"`, "one
widget across all five layouts, in arcade and eliminator" - eliminator's copy is
present but never bound, since `Hud_BindWidgets`'s bit-0x40 branch that reaches
this code also requires the position-count check (`FUN_08826d80` returning
`>1`) which an eliminator's kill-count ending never satisfies the same way.

**The per-tick update, `FUN_0881d458` (`0x0881d458`), called from `Hud_Update`
whenever HUD flag `0x40` is set:**

```c
if (mode == 9 || mode == 0xf) {
    count = FUN_08826d80(g_race_manager, pair);  // first g_racer_count craft, [+0x78] each
    if (count > 1) {
        gap = abs(pair[0]->+0xad0 - pair[1]->+0xad0);      // per-craft progress/distance field
        half = clamp(gap * 0.5, 60.0, 180.0);
        // sets "1ST"/"2ND" text and colour (green/red-ish) on the
        // Position/Position_Outof widgets, whichever craft is ahead
        // FUN_0897c784(gap) + "%s%3.0fm" -> a "+123m"/"-123m" gap label
        // on hud+0x228, positioned relative to the bar's own width
        // (hud+0x22c, +0xa0 offset field) via the widget's own getWidth vtable slot
    }
}
```

**Read in full, both functions - not renamed** (`FUN_0881d458` also carries the
ordinary `0x800` position-list branch, so no single `Head2Head` name fits it).

### Field semantics pinned 2026-09-29 (h2h-hud)

Static only; **no live PPSSPP measurement of the original's bar was taken** (the
walk to a Head2Head cell in a fresh profile was not attempted this pass). Each
row says what the claim rests on.

| Field | Meaning | Evidence | Conf |
| --- | --- | --- | --- |
| `craft+0xad0` | unwrapped arc progress, the same number `Standing::distance` ports | [race-progress.md](race-progress.md) `Craft_UpdateLapProgress` writes it (`0x08842b8c`..`0x08842c14`); the gap is `abs(a - b)` printed with `%3.0fm` | 90 |
| `craft+0x48` | "is the non-player craft of the pair" (an is-AI byte): `iVar3 = pair[0]` if it is set, else `pair[1]`, and `iVar3+0x798` becomes the *opponent's* name in both branches | decompile only, one consumer | 60 |
| `craft+0x798` | the model id `Ship_LoadModel` stores (`0x08843298`), run through the string table for the row's name | `search_instructions` on `0x798(`: three hits, one writer; live, the opponent row reads `Goteki 45`, its own hull's team (id not read at a breakpoint) | 75 |
| `<Image>` `+0x94/+0x98/+0x9c/+0xa0` | `x`, `y`, **`Width`, `Height`** | the Image `<Values>` loader `FUN_088a6f38` (`0x088a6f38`, reads `CalcBlur`/`Color1-4`/`TxtrWidth`): attribute strings at `0x08a7f220..0x08a7f230` write `+0x94,+0x98,+0x9c,+0xa0` in that order | 92 |
| Image vtable slot `+0xec` | **`GetY`** (returns `+0x98`); `+0xe8` is its this-adjust | the class's vtable at `0x08acce34`: consecutive methods `0x088a682c/34/3c/44` are `SetX/GetX/SetY/GetY`, `0x088a684c/5c` are `GetWidth/GetHeight` (`+0x9c`/`+0xa0` times `+0xe8`), and `Hud_BindWidgets` reads slot `+0xbc` (width) and `+0xdc` (x) on `SpeedBarBg`/`SpeedBarMark` consistently | 85 |
| Text widget `+0x9c/+0xa0` | `x`, `y` (the PosTag branch writes `480 - w - ...` into `+0x9c`; `Position Outof`'s `+0x9c` is copied from `Position`'s) | decompile only | 75 |
| Text widget `+0x168/+0x16c` | align / vertalign (`0/1/2` = left-top / centre-middle / right-bottom): the bind copies `Position`'s `+0x168` onto `Position Outof` and `PositionOf`, and sets `+0x16c` to `0` (top) on the second row and `1` (middle) on the gap label | decompile only; the values fit the layout, the enum was not read | 65 |
| Text widget flag `+0x2c & 0x200` | set on the **player's** row, cleared on the other; **live, the player's row throbs** (see "Live measurement"); that the flag causes it is inferred from which row carries it | decompile plus live frames | 70 |

**What the swap draws** (all inside `<Item OffsetX="445" OffsetY="5">`, so every
`y` below is local to it):

- `half = clamp(gap * 0.5, 60, 180)`.
- `Position` (`hud+0x240`) keeps its authored anchor (`y = 30`, right-aligned,
  bottom-aligned) and shows the **leader**: `IG_HUD_1ST` + a name.
- `Position Outof` (`hud+0x244`) becomes the second row: `IG_HUD_2ND` + a name,
  scale `0.8` (`0x3f4ccccd`), `y = half`, top-aligned.
- `HeadToHeadBar` (`hud+0x22c`): authored `x=15 y=30 width=5 height=0`. Its
  runtime `Height = half - GetY() = half - 30`, so it is a vertical connector from
  the first row's baseline to the second row's top. This is why the authored
  height is `0`. Its colour is never touched, so it stays the authored
  `HudColour2`.
- `PositionOf` (`hud+0x228`), authored as the `"/"`, becomes the gap label
  `"%s%3.0fm"` at `y = (GetY + half) / 2`, middle-aligned, right-aligned at the
  same `x` as `Position`. The sign string is `-` (`0x08a79e84`) when the player's
  place is 1 and `+` (`0x08a79e88`) otherwise - the strings were read from
  memory, not inferred. The number is the *full* gap, not the halved one.
- Colours (ARGB, the same packing as an authored `Color`): player leading -
  the player's row and the gap label `0xff30ff30` (green), the other row white;
  player trailing - the player's row (now the second) and the gap label
  `0xffff3030` (red), the other row white.
- `PositionTxt` (`POS`) is hidden (flag bit 4 cleared).
- Single-player names: the opponent's row is `craft+0x798` through the string
  table, the player's is the profile's pilot name at `DAT_08b31774+0x457`.
  Multiplayer (`g_game_mode >= 0xe`) uses `FUN_08966848`/`FUN_0895ebf0`
  (network names), not read further.

**An earlier reading on this page called the bar a horizontal width/fill.** It is
vertical; `+0xa0` on an `<Image>` is `Height`.

### Implemented

`crates/hud/src/head_to_head.rs` draws exactly the above from the layout's
own widgets (`Layout::fill("HeadToHeadBar")`, `label("Position")` etc.), fed by
`Race::head_to_head` in `crates/raceplay/src/telemetry.rs`
(`|Standing::distance(player) - Standing::distance(opponent)|`, and
`player_place() == 1`). `--mode head_to_head` on `oag-game --race` now reaches it
headlessly (a verification aid; a player reaches the mode through a campaign
cell). Checked with a headless screenshot, `--autopilot --ticks 900`: `1.` at the
top-right anchor, a red `+ 3m`, the mint 5-px bar, `2.` below it, `POS` gone.

**Not drawn, each with its address:**

- The two **names** - measured live to be `Goteki 45` (the opponent team's
  display name) and `AAA` (the profile tag). A source exists for the opponent
  (the opponent slot's team, resolved to its display name as the tournament
  table's `ER_TEAM` column needs); none is persisted here for the player's tag
  (`oag_ui::screen::tag_input` edits one but nothing stores it in `Readout`
  or settings). Deliberately **not wired**: without the `Default` font the row
  in the HUD font runs 2-3x wider, so names and font belong together.
- The rows' font: the original uses `Default` (`pulse_text.fnt`) for both; ours
  uses the HUD font.
- The `0x200` throb on the player's row (a live observation; law unread).
- The multiplayer branch (`g_game_mode == 0xf`).
- ~~Unmeasured against the original: the bar's pixel size at a known gap, and
  the colours~~ - measured 2026-09-29, see "Live measurement".

## Live measurement (PPSSPP v1.20.4, 2026-09-29)

`pulse-psp-usa.chd`, Xvfb :95, own profile, **Racebox `Head to Head`, Talon's
Junction White, Assegai, Venom** (not a campaign cell: both reach the same
`mode == 9` branch in `Hud_BindWidgets`/`FUN_0881d458`). Frames are under
`data/reference/psp-head2head/` (gitignored): `r3.png` (gap 62, trailing),
`z.png`/`g1.png`/`g2.png`/`g4.png`/`g6.png` (gap 780 to 2700, trailing),
`ld3.png`/`ld4.png` (player leading, produced by teleporting the player 45
units ahead of the opponent's body with the debugger, so the pose is
artificial and the HUD is the original's own), `blink.png` (40 frames of the
player row). Screen is 960x544 (2x native), offset (160, 88) in the capture.

| Claim | Result | Evidence | Conf |
| --- | --- | --- | --- |
| Bar is a vertical mint connector, 5 native px wide | **confirmed** | 10 px wide at 2x, x = 920 local (460 native), top y = 70 local (35 native), colour the layout's `HudColour2` | 90 |
| Height `clamp(gap/2, 60, 180) - 30` | **confirmed at both ends** | gap 62 (and 20, on the grid): 60 px = 30 native; gap 780 to 6700: 300 px = 150 native. Intermediate value not sampled | 90 |
| Gap label `"%s%3.0fm"`, `+` red when trailing | **confirmed** | `+ 62m`, `+ 20m` in red (the padding space is visible); label centred on the bar's midpoint, right edge about 8 px past the rows' | 92 |
| `-` and green when leading | **confirmed** | `- 42m`, `- 35m`, `- 8m` in green `0xff30ff30`-like, and the rows swap: `1st AAA` (green) on top, `2nd Goteki 45` (white) below | 90 |
| `POS` hidden | **confirmed** | no `POS` caption in any frame | 90 |
| Second row at scale 0.8, y = `half` | **confirmed** (bind sets `+0xa4/+0xa8 = 0.8`; second row sits about 6 native px below the bar's bottom) | frames above; scale itself is not separable from the font change below | 75 |
| Row texts `"%s %s"` = ordinal + name | **confirmed** | `1st Goteki 45` / `2nd AAA` | 92 |
| Opponent name = `craft+0x798` through the string table | **consistent, value read**: the display name of the opponent's team, `Goteki 45` | the opponent flies Goteki 45 (visible on its hull); the id at `+0x798` was not read at a breakpoint | 75 |
| Player name = `DAT_08b31774+0x457` | **consistent**: the profile tag typed at first boot, `AAA` | first-boot `TagSetup2FromBoot` left the default | 80 |
| `0x200` flag on the player's row | **observed: a throb**, not a static highlight (that `0x200` causes it is inferred from which row carries the flag). The player's row cycles between its colour and white, about 3.1 s per cycle (red minima at 1.9, 5.2, 8.5, 11.4, 14.5, 17.5 s of wall clock, emulator at 100%); the other row stays white | `blink.png`, a timed pixel count of the row | 80 for "throbs", period unmeasured to better than 0.1 s; the waveform is not read |
| Both rows use the `Default` font | **new**: `Hud_BindWidgets` calls `FUN_088cd3b4(row, NULL)` on both, which stores `"Default"` (`0x08a81c7c`) and looks the font up; digits are 14 px tall against 16 px for the gap label at HUD scale 0.6 | decompile plus `z.png` | 85 |
| Rows right-align at the same x | **new**: the bind copies `Position`'s `+0x9c`/`+0x168` onto `Position Outof`; right edges at 888 and 891 local | `z.png` | 88 |

**Code changed by this pass:** the second row now uses `Position`'s `x` (it
used its own authored 16, which drew it right of the leader row). Not changed:
the `Default` font (no text bucket for it), the names (the readout carries no
team or tag), the throb (law unread). `oag-game --race --mode head_to_head --ticks 600`
at 960x544, before (`ours_e_600.png`) and after (`ours_after_600.png`) under
`data/reference/psp-head2head/`: after the fix `1st` and `2nd` end within 1 px
of each other and the gap label sits about 3 native px past them, as live.

**Still open:** the id at `craft+0x798` was not read at a breakpoint; the
`0x200` throb's law (phase, waveform, whether it is a generic Text-widget
behaviour); the multiplayer branch; the campaign-cell launch itself.

## 2026-10-08 (pulse-h2h): `Race_RecordResult`'s arm, and a campaign cell walked live

**`Race_RecordResult` (`0x0880ae54`) switches on `g_game_mode - 3`, and mode 9 is
`case 6`, the arm it shares with mode 3 (`Race`), `0xe` and `0xf` (`case 0`, `6`,
`0xb`, `0xc`).** Decompiled again: the arm takes the finishing place
(`param_3`), writes it to the profile's best place for the cell record when it
beats the stored one (or the stored one is `0`), and sets `uVar8 =
Cell_EvaluateMedal(DAT_08b30ffc, place)`. So Head2Head's medal is the Race medal
on the place: the flat `1/0/0` targets make place 1 gold and nothing else a
medal. Confidence 85 (decompile, same function the `Race` evidence on
`race-campaign.md` rests on; the place-to-medal compare itself was not
re-walked to a finish live). `oag_game`'s `campaign_medal` already treats `Race`
and `Head2Head` alike (`crates/game/src/main/race_stage.rs`).

**Live, campaign cell `grid4_5_2`** (Head to Head, `04_Track` = Tech de Ra
White, Flash, Weapons Off, 4 laps; the dev-unlock byte opened Grid 5): the
HUD shows `1st Qirex` / `2nd AAA` with a red `+3424M` gap at GO growing to
`+4524M` ten seconds later (the player stood still), the `Lap 1/4` counter, no
weapon panel. **The opponent flies Qirex in this cell** (one sample; the custom
race in the section above had Goteki 45), so the team is cell-dependent or
drawn, and this build's cyclic choice stays chosen, not measured. Screenshots
`ht3.png`, `ht4.png`. Quitting this race lands on `Cell Selection`:
[campaign-quit.md](campaign-quit.md).

**Unread:** `FUN_088099b4`'s test that picks the `Show Unlocks` redirect, the
finish of a Head2Head cell watched live to its medal, and the opponent
selection rule.

## Names landed

None. See "not renamed" above for the HUD function; the grid-compaction and
skill-formula functions this page leans on were already named by
[grid.md](grid.md) and [race-campaign.md](race-campaign.md) respectively.

## Evidence

- `oag-wad cat --expand "pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad" 'Data\Plugins\grids\grid_NN.xml'`
  for `NN` = `00`..`15`, censused for `mode="Head2Head"` - 23 matches, every
  attribute flat as tabulated above. Not committed to the repository (game
  content, per `docs/overview/legal.md`).
- `search_strings "HeadToHeadBar"` -> `0x08a7a0ac`, one match.
- `get_xrefs_to 0x08a7a0ac` -> one xref, `Hud_BindWidgets` at `0x08820748`.
- `decompile_function 0x08820748` (`Hud_BindWidgets`), `0x0881bf50` (`Hud_Update`),
  `0x0881d458`, `0x08826d80` - all four read in full this pass.
- 2026-09-29: `Hud_BindWidgets` `0x0881fbec` (Head2Head branch), `FUN_0881d458`,
  `FUN_088a6f38` (Image `<Values>` loader), the Image vtable at `0x08acce34`
  (`disassemble_bytes` `0x088a6820..0x088a686f`, `0x088a6274..0x088a629b`),
  `search_instructions` `0xad0(` and `0x798(`, `read_memory 0x08a79e50` (format
  and sign strings).
