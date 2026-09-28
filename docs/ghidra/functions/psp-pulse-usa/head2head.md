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
does, was not this pass's job. This project's own `crate::livery::teams_for_slots`
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

**Read in full, both functions, this pass - not renamed.** The branch structure,
the field-comparison logic and the `60.0`/`180.0` clamp (`0x42700000`/`0x43340000`,
confirmed by hand-decoding the IEEE-754 bit patterns rather than trusting
Ghidra's float literal rendering) are unambiguous. What is not chased: the exact
semantics of craft`+0x48` (the flag `FUN_0881d458` reads to decide which of the
pair is "ahead" for colour purposes), craft`+0xad0` (read as a race-progress/
distance value, consistent with its use in a "how far apart are these two"
computation, but not cross-checked against another consumer), and the widget
struct's `+0xa0`/`+0xe8`/`+0xec` vtable-slot semantics (position offset and a
"get width" accessor, by convention with other widget code on this page's own
neighbours, not independently confirmed). That is below this project's naming
floor for *meaning*, even though the *mechanics* are confidence ~90 - the same
distinction [grid.md](grid.md#which-team-flies-which-slot-the-id-is-a-real-field-on-a-struct-of-eight)
draws for its own six unnamed functions. `FUN_0881d458`'s `else` branch (HUD flag
`0x800` set, a different bit) is the ordinary multi-racer position-list updater
and is unrelated to Head2Head; only the `0x40`-branch's mode check is.

**Not implemented this pass.** A player racing a Head2Head cell today gets no
gap readout - the "1ST"/"2ND" plus "+123m" HUD swap above is read but not wired
into `oag_ui`/`crates/game/src/hud`. `MSC_EVENT_HTH`'s own text - *"Track the
distance between you and your opponent on the HUD"* - is exactly this widget,
so building it is a player-visible follow-up, not a cosmetic one; see the
handover thread's Next Steps. Left unbuilt rather than approximated, per
`CLAUDE.md`'s "never invent what the assets already author": guessing at the
bar's fill algorithm without the vtable slot's confirmed contract would be
exactly that.

## Names landed

None. See "not renamed" above for both HUD functions; the grid-compaction and
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
