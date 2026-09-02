# Race start: the countdown state machine, the launch reaction boost, and whether Zone shows a different countdown

Three questions, tracked as one grid across the four titles this project reads. Opened
skeletal, then one static-only pass landed the same day - see below for what each
"partial" and "strong evidence" cell actually rests on before trusting it.

|                                  | Pure | Pulse                          | HD/Fury                        | 2048 |
| -------------------------------- | ---- | ------------------------------ | ------------------------------- | ---- |
| Countdown state machine + timing | open | **Zone's own read in detail** (below) | open                | open |
| Launch reaction speedboost       | open | no lead found yet (below)      | **likely not a thing** (below)  | untried, `StartBoost` string present (below) |
| Zone/view display variants       | open | logic axis: separate impl. confirmed, not yet *differing* (below) | strong evidence, asset axis (below) | strong evidence, asset axis (below) |

*(Focus as of 2026-09-02: display/logic only for now, per direction from the user -
the launch-boost rows below are last session's record, not being chased further at
the moment.)*

**Countdown state machine + timing**: what drives `ReadyText`/`GoText`/`CountdownTime`
and the `<Mode3D>` models (`Pulse_Ready_Go`, `Cockpit_321GO`) from race load to green -
durations, the false-start stall `race-modes.md:470-473` already names ("silently kills
the engine if you hold thrust before the lights") but has never traced, and what state
holds it.

**Launch reaction speedboost**: whether landing the accelerate press at the right instant
relative to the lights grants a speed boost - the mechanism is unknown going in: a
timestamp on first thrust press compared to green, or throttle held through the
countdown with a false-start penalty (or both are the same mechanism read two ways). Two
different searches, not one - see the Open section.

**Zone/view display variants**: the user's memory is that HD/Fury's Zone races show a
different countdown display/render than a circuit race; unsure about the other three
titles. Two independent axes may both be true and must not be collapsed into one: a
**mode** axis (the `321Go_*.vex` family already carries `StartFinish`/`Zone`/
`HD_Zone_Battle`/`hd_detonator` variants per
[a-circuits-billboard-slots-are-a-9-entry.md](a-circuits-billboard-slots-are-a-9-entry.md)),
and a **view** axis (`hud.md:622` already names two separate Pulse countdown models,
`Pulse_Ready_Go` for the chase/external HUD and `Cockpit_321GO` for the cockpit view -
independent of mode).

2026-09-02, first pass, static only, both HD/Fury and Pulse, no live debugger this
session: **the launch boost most likely is not a player-timing mechanic, and Zone's
countdown display very likely is different, from both a mode axis and a view axis.**

**HD/Fury's `StartBoost` is AI grid-slot tuning, not a player reaction window.**
`ps3-hdfury-eu`'s `.rodata` carries a plain tunable-name table (`AI track data`,
readable inline: `LookAheadSecs`, `SteerMul`, ..., `BaseThrust`, `StartBoost`,
`SkillScalePoint1..3`, ..., `StartStats`) - the same family `docs/gameplay/ai.md`'s
"`StartStats` and `SkillScale`" section already documents from Pulse's and Pure's own
`AIRaceStats.xml`/equivalent, confidence 85: **"the launch is staggered by grid
position rather than by anything the player does"**, `GridPlace1..8` each carrying
their own `StartBoost`. HD ships the identical two names (`BaseThrust`, `StartBoost`)
in the same table shape, so this reads as the same system, not a coincidence -
**not confirmed by tracing HD's own parser**, only by the string table's presence
and naming, so call this confidence 60 for HD specifically pending the read `ai.md`
already did for Pulse/Pure. If the AI's own boost is purely a scripted per-slot value
and needs no reaction window to stay competitive, that weakens (does not kill) the
premise that the *player* has a skill-based one - a human-only mechanic wouldn't
need an AI counterpart to balance around it, so this is suggestive, not decisive.
No string, function name, or table found this session that reads as a player-side
"time since lights went green" or "throttle held before green" capture on either
title - the search was a name/string sweep only, not a trace of the actual
countdown-to-throttle path, so **absence here is weak evidence, not a finding.**

**HD/Fury: four distinct `321Go_*.vex` countdown-gantry models, confirmed by name,
role not yet traced to a mode.** `search_strings` on `ps3-hdfury-eu` found
`321Go_StartFinish.vex`, `321Go_Zone.vex`, `321Go_HD_Zone_Battle.vex` and
`321go_hd_detonator.vex` (note the lowercase `321go` on the last one - authored
inconsistently, not a typo introduced here) sitting together in `.rodata`, plus a
separate `Cockpit321Go` string. This directly corroborates the user's memory:
**Zone does not share the circuit-race gantry model** - it has its own, and so do
HD's Zone Battle and Detonator modes, each a fourth. This is the same four-name
family [a-circuits-billboard-slots-are-a-9-entry.md](a-circuits-billboard-slots-are-a-9-entry.md)
already found via a mode-descriptor pointer that replaces the disc-authored `Num==7`
billboard mesh - **that thread's own top open item (tracing the pointer to one of
these four shapes) is the same evidence this thread needs**, not yet closed by
either. `Cockpit321Go` is the same finding `docs/ui/hud.md:622` already recorded for
Pulse (`Pulse_Ready_Go` vs `Cockpit_321GO`) - **the view axis is real on HD too**,
independent of mode, and the two axes are confirmed distinct (four mode-keyed
gantry models, plus a separate cockpit-specific one on top of whichever mode model
is active). Confidence 70 for "four distinct mode-keyed models exist and are
authored"; confidence unassigned for "and Zone's is drawn differently in practice"
because nothing has watched one render - the billboard thread's own stated lesson
about publishing a render claim without a screenshot applies here just as much.

**Pulse: `Ship_SetState`'s previously-undocumented states 6, 7 and 8 read.** Full
account, all nine arms, in
[shield.md](../docs/ghidra/functions/psp-pulse-usa/shield.md#the-full-nine-arm-table-and-a-race-start-hypothesis-for-states-6-and-7).
State 6 sets the *same timer field* states 4/5 use for the destruction sequence
(`entity->0x874`) to `2.0`s (local player) or `0.8`s (everyone else, split on the
already-confirmed-at-80 `entity->0x368` field) and silences something before calling
one more function on the entity - the shape of "stall the engine, arm a timer",
matching `race-modes.md`'s existing false-start claim. Confidence 55, still not
runtime-verified. **State 7's per-mode dispatch is now resolved and read - and it
walks back the countdown hypothesis this thread first reached for.** The two
addresses its own disassembly builds turned out to need the same
`+ 0x08804000` correction `workflow.md` already documents for `jal` targets, now
generalised to a `lui`/`lw`-pair jump-table base too (new trap, written up in
`workflow.md` and `shield.md` both). Corrected, 18 of 19 entries land on just two
targets, both of which toggle the *same* "craft destroyed" bit (`entity->0x860`
bit `0x1000`) `zone-mode.md` already names - one conditionally, with a call; one
unconditionally, without. Confidence 55: **state 7 reads as a "clear the destroyed
flag" transition whose exact shape depends on a game-mode class, not as a
countdown-specific per-mode dispatch.** The per-mode table itself is real and
resolved; what it was hoped to show (a countdown that visibly branches on mode)
is not what it turned out to hold. That does not settle the Zone-display question -
it just means state 7 isn't where the answer lives; the asset-side evidence below
is unaffected.

**2048: seven `321Go_*.vex` models, not four - confirms the engine-lineage link and
adds two of its own.** `vita-2048-eu-v104` carries every HD/Fury name
(`321Go_StartFinish.vex`, `321Go_Zone.vex`, `321Go_HD_Zone_Battle.vex`,
`321go_hd_detonator.vex`, same inconsistent-lowercase one included) plus two new
ones, `321Go_2048.vex` and `321Go_2048_Combat.vex` - almost certainly this title's
own signature mode and its combat variant. `Cockpit321Go` is present too, same as
Pulse and HD - the view axis holds on a third title. `StartBoost` appears twice in
`.rodata`, unread beyond the string existing (not traced to confirm it is the same
AI-tuning table HD's is, though the name and title's engine lineage both point that
way). This is the strongest cross-title corroboration for the Zone/view axis this
session found - three of four titles now show the same two-axis shape (mode-keyed
gantry model, separate cockpit model), and it cost one string search per title.
Pure's binary (`psp-pure-usa`) was **not reachable this session** - `list_instances`
reports it open in the underlying Ghidra project, but every `search_functions`/
`search_strings`/`switch_program` call against `/psp-pure-usa/BOOT.BIN` returned
"Program not found", with only 4 of the 9-11 listed programs actually addressable
through this MCP connection. Worth a fresh connection or Ghidra-side check before
assuming Pure lacks the same asset family - the absence here is a tooling gap, not
a finding.

2026-09-02, second pass, display/logic only: **Zone's own countdown state machine is
now read in real detail on the logic side**, and it is architecturally separate from
whatever a circuit race uses - though "separate" and "different" are not yet the same
finding. Full account in
[zone-mode.md](../docs/ghidra/functions/psp-pulse-usa/zone-mode.md#zone_updatestates-own-five-states-and-state-0s-nested-countdown).
In short: `Zone_UpdateState` (already named) dispatches five outer states; state 0 -
the countdown - is a *second*, nested five-substate machine (`0x08829e6c`) with a
real frame-tick counter (`obj+0x1a04`, new offset, confidence 65), a sound cue fired
at exactly tick 40 remaining, and a two-part gate (tick count plus a second,
unresolved float condition) before it hands off to state 1 and racing begins.
Confidence 65 for the shape. This directly answers part of "what state holds the
false-start stall and what drives `CountdownTime`" from the intro above: it is this
nested machine, not (as last session guessed) anything in `Ship_SetState`. **What it
does not yet show is whether a circuit race's own countdown looks the same** - one
attempt to find and decompile a non-Zone mode's equivalent state-0 handler (via
`Race_CreateModeObject`'s per-mode dispatch table) landed on an unrelated HUD-text
function both times tried, so the comparison is still open, written up as a dead end
rather than silently dropped (see `zone-mode.md`'s own account). `CountdownTime`/
`ReadyText`/`GoText` themselves still have no direct code xref - they read as
XML-driven HUD-widget attribute names resolved through a hash lookup, the same shape
`hud.md` already documents for the rest of the HUD, not a literal string compare a
function references - so the nested countdown's tick counter is the closest thing to
"what drives the display" found so far, not a confirmed direct link.

## Open

- **Whether a circuit race's own state-0 countdown handler matches Zone's shape**
  (same 40-tick cue, same two-part gate) or differs is the single most direct
  open question for the display/logic focus. One attempt this session to reach it
  through `Race_CreateModeObject`'s dispatch table failed (landed on an unrelated
  function, see `zone-mode.md`) - needs the table re-derived from raw disassembly,
  not reused from this session's addresses.
- **The second gate condition (`fVar17`) substate 1 waits on**, alongside the tick
  counter reaching zero, was not resolved to a real function this session - a
  guessed address decompiled to an unrelated matrix-inverse routine.
- **`CountdownTime`/`ReadyText`/`GoText` have no direct code xref** - consistent
  with a hash-resolved HUD-XML attribute rather than a literal string a function
  loads, but not traced to the hash lookup itself, so this is an inference from the
  shape of the rest of the HUD system, not a read of this specific path.
- **Pure (`psp-pure-usa`/`-eu`) was unreachable via ghidra-mcp this session** despite
  `list_instances` listing it as open - only 4 of 9-11 listed programs answered
  `search_strings`/`search_functions`/`switch_program` calls. Try a fresh MCP
  connection, or check from the Ghidra GUI directly, before concluding anything
  about Pure's own countdown assets.
- **State 7's mode-class value is still unnamed.** `0x08aae7e3` and the global
  pointer's `+0xb8` field feeding the resolved per-mode table are read but not
  identified - which ~19 mode/network variants map to which of the two clear-bit
  behaviours is unknown, and so is `FUN_0003c7b0`, the call the conditional path
  makes.
- **No live capture of an actual false start exists.** Everything about states 6/8 in
  `shield.md` is a branch-clear static reading; a PPSSPP watchpoint on `entity->0x874`
  and `entity->0x8c` during a real held-thrust-before-lights start would settle which
  state fires, the real timer value, and whether 6 or 8 is the false start (they are
  mirror images of each other and nothing here distinguishes which is which).
- **No player-side launch-timing mechanism has been found on either title searched.**
  This session only ran name/string sweeps, not a trace of the throttle-input path
  through the countdown - a `Ship_UpdateThrust`-adjacent function reading the
  countdown clock during the lights, if one exists, is still unfound. Given HD's
  `StartBoost` reads as pure grid-slot AI tuning, the working hypothesis going in
  should now be "the boost is not player-skill-timed", but that has not been
  positively confirmed, only made less likely by an absence.
- The billboard thread's own top open item - the mode-descriptor pointer replacing
  `Num==7`'s mesh, traced to one of the four `321Go_*.vex` shapes - still doubles as
  this thread's Zone-display-variant question and is still open in both places.
- **No screenshot comparison of a Zone countdown against a circuit-race countdown has
  been taken**, on any title. The asset-name evidence is strong; the render itself is
  unconfirmed.
- HD/Fury's `TrackStartup_Load`-adjacent vocabulary sweep (`Countdown`, `StartLight`,
  `RaceStart`, `Grid`) found nothing under those exact names - only `321`/`Ready`/
  `Boost` searches paid off. Worth trying `Light`, `Klaxon`/`Lights`, `Sequence`
  next, since the engine may simply not use those words.
- Pulse's `ready`/`321_GO`/`go` announcer cue set
  (`docs/ghidra/functions/psp-pulse-usa/zone-mode.md:252`) is still unxrefed.
- 2048 (`vita-2048-*`) and Pure (`psp-pure-*`) are completely unstarted here.

## Next Steps

**Display/logic, current focus:**

1. Re-derive `Race_CreateModeObject`'s per-mode dispatch table from raw disassembly
   (do not reuse this session's `0x08a7a1e8`/entry addresses without rechecking) and
   decompile a non-Zone entry's own vtable `+0x1c` slot - the direct way to see
   whether a circuit race's countdown matches Zone's 40-tick-cue, two-part-gate
   shape or differs from it.
2. ~~`create_function` for Zone's inner-substate setter~~ **Done this session**:
   named `RaceMode_SetSubstate` (`0x08827454`, confidence 80); the outer-state
   call at substate 4 turned out to already be `RaceMode_SetState` itself.
3. Resolve substate 1's second gate condition (the `fVar17` call) from the raw
   disassembly of `0x08829e6c` rather than a guessed address.
4. Finish tracing the mode-descriptor pointer from
   [a-circuits-billboard-slots-are-a-9-entry.md](a-circuits-billboard-slots-are-a-9-entry.md)'s
   open item 1 to a `321Go_*.vex` shape - still the best lead for the mode-display
   axis on the asset side.
5. Get a live screenshot of a Zone countdown next to a circuit-race one (any title)
   before publishing either a positive or a negative claim about how they differ -
   the billboard thread's own stated lesson, from a mistake made on this exact asset
   family.
6. Get Pure reachable through ghidra-mcp (fresh connection, or check the Ghidra GUI
   directly for why only 4 of 9-11 listed programs answer this session) and then
   run the same three-string sweep (`321Go`, `StartBoost`, `Cockpit321Go`) that
   worked cleanly on both HD and 2048, plus the vocabulary from step 1 once it has
   real names.

**Launch boost, parked for now - not being chased at the moment:**

7. Live-capture a false start in PPSSPP: hold thrust before the lights, watchpoint
   `entity->0x874` and `entity->0x8c` on the player's own entity, and read which
   state (6 or 8) actually fires and for how long.
8. Trace the throttle/thrust input path during the countdown (starting from
   `Ship_UpdateThrust` or its equivalent, per `docs/gameplay/ai.md`'s and
   `engine.md`'s existing naming) for anything that reads a countdown clock or a
   "pressed near green" timestamp.
9. Identify the ~19-entry mode-class value `Ship_SetState` state 7 reads
   (`0x08aae7e3` / global`+0xb8`) and `FUN_0003c7b0`.
