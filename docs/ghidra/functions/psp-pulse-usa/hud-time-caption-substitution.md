# The HUD's mode-code caption substitution, read for one widget pair

**Binary:** `pulse-psp-usa` `BOOT.BIN`, image base `0x08804000`.

[`docs/ui/hud.md`](../../../ui/hud.md#and-one-structural-finding) found that a
frame reads `record` where `TimeTrial_HUD.xml` authors `TotalTimeTxt` with
`idstring="IG_HUD_TOTAL"` - code substitutes a different `IG_HUD_*` key into a
widget the layout already positioned, and left the mechanism unread. This page
reads it, for that one widget pair.

## The substitution table, confidence 90

`Hud_UpdateTimeCluster` (`0x0881c9d0`-`0x0881cdc3`) contains

```c
uVar2 = 0x275e04;                 // IG_HUD_TOTAL   (default)
if (iVar3 < 2) {
    if (-1 < iVar3) {
        if (iVar3 < 1) {
            uVar2 = 0x275e14;     // IG_HUD_BRONZE  (tier == 0)
        } else {
            uVar2 = 0x275e24;     // IG_HUD_SILVER  (tier == 1)
        }
    }
    // tier == -1 falls through: stays IG_HUD_TOTAL
} else if (iVar3 < 3) {
    uVar2 = 0x275e34;             // IG_HUD_GOLD    (tier == 2)
} else if (iVar3 < 4) {
    uVar2 = 0x275e40;             // IG_HUD_RECORD  (tier == 3)
}
// tier >= 4, tier != 5: stays IG_HUD_TOTAL
```

where `iVar3` is `*(int *)(*(int *)(param_1 + 0x3c) + 0x34)` - an ordinal read
off a race-context pointer the function is also using for other fields (see
below). The five literals are this database's pre-relocation immediates
(`docs/ghidra/workflow.md#why-a-psp-import-silently-loses-every-relocation`);
adding the image base gives, checked one for one against `search_strings`:

| Literal | `+ 0x08804000` | String at that address |
| --- | --- | --- |
| `0x275e04` | `0x08a79e04` | `IG_HUD_TOTAL` |
| `0x275e14` | `0x08a79e14` | `IG_HUD_BRONZE` |
| `0x275e24` | `0x08a79e24` | `IG_HUD_SILVER` |
| `0x275e34` | `0x08a79e34` | `IG_HUD_GOLD` |
| `0x275e40` | `0x08a79e40` | `IG_HUD_RECORD` |

All five match exactly, and the fifth is corroborated a second way: the
`addiu a0, a0, 0x5e40` at `0x0881cd50` that builds `0x275e40` (`lui` supplies
the high half `0x27`, shared with every other literal in the table) is the
**only** reference to `IG_HUD_RECORD`'s address anywhere in the binary -
`search_instructions` over all 525,049 disassembled instructions, both as
`addiu` and `ori`, found this one hit. `IG_HUD_TOTAL`'s own address has
**zero** code references at all, which is the other half of the same finding:
nothing ever loads it as a pointer, because nothing has to - it is only ever
the layout XML's own `idstring`, read the ordinary way, until this table
overrides it.

Score **90**, up from the decompilation-only 84 this table shipped with
until 2026-09-28: `iVar3` (`param_1+0x3c` then `+0x34`) is now a live,
write-watchpointed field - see "What the tier itself is" below - and two
real frames from the same PPSSPP session corroborate this table's own branch
structure directly. A plain, non-campaign Venom Time Trial on Talon's
Junction, `target`/`tier`/`redden` read live at `0x09a4a728`/`0x09a4a72c`/
`0x09a4a730` (`*(hud+0x3c)+0x30`/`+0x34`/`+0x38`): mid-countdown, `tier=3,
redden=0`, the frame shows `record` in the layout's own authored mint green
(`HudColour2`); parked past the track's own 117.0 s Venom record (read off
`DAT_08b310b4+0xa0`), `tier=3, redden=1`, the frame shows the identical
caption `record` but the numeric widget alone turns solid red, `target`
clamped to `0`, exactly the `tier==3 -> IG_HUD_RECORD` row and the
`FUN_088cd290(..., 0xffff0000, ...)` recolour call this decompile shows.
Capped below 95 (not 100) because only the USA pressing was measured and
`Hud_UpdateTimeCluster`'s own program counter was not itself caught at a
breakpoint - the corroboration is the drawn frame plus the live register
reads that predict it, not an execution trace of this exact function.

The chosen key is looked up through the language plugin's string table
(`func_0x0008f8ec(_DAT_002ad160, key_address, 0)`, the same primitive
[hud.md](../../../ui/hud.md#localisation-keys) already identifies as the
`idstring` resolver) and written into the caption widget with
`func_0x000c8f6c`, **only when `iVar3` differs from a cached copy at
`param_1 + 0x400`** - so the lookup is a one-shot on a tier transition, not a
per-frame cost.

## What the tier itself is, closed 2026-09-28

`iVar3` selects the string; **what makes it `0`..`3` is now read, at
`PlayerStatus_Update`'s own campaign branch** -
[race-progress.md](race-progress.md#the-target-time-readout-0x780x7c0x80-closed-2026-09-28)
carries the full law, the live write-watchpoint evidence (five PCs, all
inside `PlayerStatus_Update`'s real body, `0x0883b3b8`-`0x0883c0cb`), and
the two reference frames
this page's own confidence section above cites. Summary: `param_1+0x3c` is
the `"PLAYER_HUD"` `PlayerStatus` object (`self+0x48`, per
[race-progress.md](race-progress.md#from-the-hud-to-the-counter)), and its
`+0x30`/`+0x34`/`+0x38` are `PlayerStatus_Update`'s own `+0x78`/`+0x7c`/
`+0x80` - a target-time countdown, the tier this table switches on, and a
"missed it" redden bool - compared, for a campaign Time Trial or Speed Lap
cell, against that cell's own `gold`/`silver`/`bronze` fields
(`oag_tables::race_campaign::Cell`, `DAT_08b30ffc+0xa0/0xa4/0xa8`) rather
than anything invented, **except when the player's own stored personal
best for the cell already beats gold and the live pace beats that too**, in
which case the original shows `RECORD` instead (the stored best is
`Profile_GetBestRaceTime`, `0x088091a0`, read 2026-09-30). Otherwise (no
campaign cell) the tier is always `RECORD`, compared against that same
personal-best store narrowed by the track's own `RaceTimes` record - see
[race-progress.md](race-progress.md#the-target-time-readout-0x780x7c0x80-closed-2026-09-28)
for the full branch structure and
[the stored best](race-progress.md#the-stored-best-fun_088091a0-and-the-record-branch-closed-2026-09-30).

What race-progress.md leaves open, restated here since this page named it
first: `param_1 + 0x30` (`iVar5` above, `-1` hides both the numeric and
caption widgets) is the same field as the target-time countdown itself, not
a separate placement/timer flag - closed by the same pass, not a remaining
gap. `param_1 + 0x5a`, gating the *other* caption pair this function updates
(`IG_HUD_CURRENT`/`MSC_RACE_ENDS` at `+0x1f0`/`+0x1f4`), is unrelated and
still unread. `tier == 5` (skips the caption block, calls
`func_0x000157f0` instead of `func_0x000156e4` on the same value) is
`g_game_mode == 7`'s own branch in `PlayerStatus_Update` - **Free Play**
([`state-machine.md`](state-machine.md#the-game-mode-enum-g_game_mode-0x08b31048)'s
enum: 5 Time Trial, 7 Free Play, 10 Speed Lap, `0x11` Multiplayer Time Trial),
which shows the race manager's own clock in the same widgets; not a fifth medal
tier.

## `param_1 + 0x30 == -1` hides both widgets, and that is most modes

The `iVar6 == -1` branch at the top of the target-time block clears bit `0x4`
(visible) of `+0x2c` on `param_1 + 0x200` and `param_1 + 0x204`, and writes
nothing else. `PlayerStatus_Update` sets the field to `-1` every tick and
overwrites it only for Time Trial, Multiplayer Time Trial, Speed Lap and Free
Play, so **the pair is hidden in Zone, Eliminator, single race, Head2Head and
Tournament**. Confirmed live on an Eliminator race, 2026-09-30, confidence
**95** - the evidence (the field read `-1`, both widgets' flag words read `0xb082`
against `0xf086` for the widgets that draw, and a frame with no `TOTAL` on screen)
is on
[`race-progress.md`](race-progress.md#which-modes-hide-the-clock-confirmed-live-2026-09-30).
Zone is the odd one: `Zone_HUD.xml` authors neither widget, so there is nothing to
hide there.

## `param_1` is the same struct `Hud_BindWidgets`'s dispatcher builds

Cross-checked, not assumed: `param_1 + 0x1f0`, `+ 0x1f4` (`+ 500` in the
decompilation), `+ 0x1f8`, `+ 0x1fc`, `+ 0x200` and `+ 0x204` are all read
here and are exactly the offsets `FUN_0881fbec` - the widget-bind function
[hud.md](../../../ui/hud.md#lap-counting-was-the-one-real-blocker) already
names as "the widget bind" - writes widget lookups into. So this function
consumes widgets that function found by name: `hud+0x1f0` is `"CurrentTime"`,
`+0x1f4` `"CurrentTimeTxt"`, `+0x1f8` `"BestTime"`, `+0x1fc` `"BestTimeTxt"`, `+0x200`
`"TotalTime"` and `+0x204` `"TotalTimeTxt"` - read directly off
`Hud_BindWidgets`'s decompile, where each `FUN_08973390` name string
(`s_CurrentTime_08a79fb8` and its five neighbours) is passed to the widget
lookup immediately before the store. That block is entered only when `hud+0x40 &
0x20 == 0`, so in Zone (the mode whose `Zone`/`Score`/`SpeedClass` binds sit under
the same flag, read from the other branch) the six slots stay null.
Confidence **92** for the names (a direct decompile of both the binder and this
function, and the live flag words at exactly these slots); the earlier "candidates"
wording is retired.

## No caller found, and that is the relocation bug again

`get_function_callers` and a `jal` operand search both return nothing for
`0x0881c9d0`, in a database where automatic xrefs are known to be empty end to
end - see
[workflow.md](../../workflow.md#why-a-psp-import-silently-loses-every-relocation).
`jal` targets do not need relocation to resolve (the 26-bit target is absolute
within the current 256 MiB segment and needs no `lui`/`addiu` split), so a
missing `jal` is a weaker signal than a missing data xref - but it is still
possible this function is reached only through a function-pointer table, the
same pattern `FUN_0881fbec`'s per-widget-kind dispatch already uses. Reimport
under the fix that page describes before spending more time on this by hand.

## What this does and does not settle for the implementation

**Settled:** the substitution is a real, single, table-driven mechanism keyed
on an ordinal state - not five independent code paths, not a per-mode
`match`. `oag_hud::draw::caption`'s fallback, resolving `TotalTimeTxt`
straight off the layout's own `idstring`, is correct only for the tiers this
table maps back to `IG_HUD_TOTAL`: a race in a mode outside `{5, 0x11, 10, 7}`
(where the widget pair is hidden anyway), `DAT_08ab0de0 == 0`, and a track whose
record did not load.

**Settled and implemented, 2026-09-30:** all four captions.
`IG_HUD_BRONZE`/`SILVER`/`GOLD` for a campaign Time Trial/Speed Lap cell
(2026-09-28), and `IG_HUD_RECORD` for a plain Time Trial or Speed Lap and for a
campaign cell whose stored best already beats gold -
[race-progress.md](race-progress.md#the-stored-best-fun_088091a0-and-the-record-branch-closed-2026-09-30)
has the source (`min(stored best, <RaceTimes>/<LapTimes>)`) and the live frame
it was checked on: `record 1.33.2` at `0.23.7` on a Venom Time Trial on Talon's
Junction, reproduced as `1.33.2` at `0.23.8` on ours. `draw.rs`'s
`TotalTime`/`TotalTimeTxt` arms read
[`Readout::time_trial_pace`](../../../../crates/hud/src/lib.rs), which
`RaceStage::draw_hud` and the headless capture both fill; `None` leaves the
plain reading above.

**Still not settled, and chosen rather than measured:** the stored best. The
original's store is per team (`Profile_GetBestRaceTime`, `0x088091a0`);
`oag_game::records::Key` is circuit, mode and class, so a run in one team's
ship races the best of any team's. No live frame has a *stored* best on it
(the profile used holds none), so the `min` and the campaign `RECORD` branch
are the decompile plus unit tests, not a frame.
