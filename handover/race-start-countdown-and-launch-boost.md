# Race start: the countdown state machine, the launch reaction boost, and whether Zone shows a different countdown

Three questions, tracked as one grid across the four titles this project reads. Opened
skeletal, then one static-only pass landed the same day - see below for what each
"partial" and "strong evidence" cell actually rests on before trusting it.

|                                  | Pure | Pulse                          | HD/Fury                        | 2048 |
| -------------------------------- | ---- | ------------------------------ | ------------------------------- | ---- |
| Countdown state machine + timing | open | **Zone's own read in detail** (below) | open                | open |
| Launch reaction speedboost       | open | no lead found yet (below)      | **likely not a thing** (below)  | untried, `StartBoost` string present (below) |
| Zone display: HUD overlay        | **confirmed identical** (below) | **confirmed identical** (below) | **confirmed identical** (below) | untried |
| Zone display: track-side gantry  | untried | see billboard thread (open)    | **matches the user's own description, rendered and confirmed** (below) | asset family exists, untried past a string search |

*(Focus as of 2026-09-02: display/logic only for now, per direction from the user -
the launch-boost rows below are last session's record, not being chased further at
the moment. The display row split in two mid-session: "the countdown display" turned
out to name two different objects, conflated here at first - see below. **The user
then confirmed directly, from playing**: Zone's gantry "draws a small rectangular
track, instead of the 3 2 1 GO" - and that is now rendered straight off the disc and
matches exactly, see the second 2026-09-02 entry below.)*

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
titles. **Two genuinely different objects can both plausibly be "the countdown
display", and this thread's own first two passes conflated them before catching it**:
the on-screen `<Mode3D>` HUD overlay (`ReadyGo`/`Cockpit321Go`, the "3, 2, 1, GO"
graphic) versus the track-side starting-line gantry (`TrackStartup.xml`'s billboard
slot 8, a physical object standing on the circuit, subject of
[a-circuits-billboard-slots-are-a-9-entry.md](a-circuits-billboard-slots-are-a-9-entry.md)).
**The HUD overlay is now confirmed identical across every mode on every title
checked** (Pulse, Pure, HD - see below); **the track-side gantry is confirmed to
genuinely differ per mode on HD**, and is the far more likely home for the user's
memory, since it is the one actually standing in the world rather than drawn on the
screen.

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

2026-09-02, third pass: **the mode-display axis is settled for the HUD overlay on all
three titles checked, and a real difference is confirmed elsewhere - on the
track-side gantry, not the HUD.** These are two different objects, conflated in this
thread's own first two passes above before being caught here - corrected rather than
left standing. Full account, side by side with the direct XML for every file quoted,
in [hud.md](../docs/ui/hud.md#deferred-and-known):

- **The HUD `<Mode3D>` overlay** (the on-screen "3, 2, 1, GO" graphic, `ReadyGo`/
  `Cockpit321Go`): `oag-wad cat` against `pulse-psp-usa.chd` and `pure-psp-usa.chd`
  shows Pulse's `Arcade_HUD.xml`, `TimeTrial_HUD.xml`, `Elimination_HUD.xml` and
  `Zone_HUD.xml` all carrying an identical block - `Src="Data\HUD\Pulse_Ready_Go.vex"`
  plus `Data\HUD\Cockpit_321GO.vex`, same transform - and Pure's `Arcade_HUD.xml`/
  `Zone_HUD.xml` agreeing the same way on `Data\HUD\Ready_GO.vex`. **HD/Fury agrees
  too**: `hud_ready_go.xml`, the fragment every mode's HUD file `LoadXML`s in, is
  byte-identical across every skin checked on the disc (default, `2097_hud`,
  `wo3_hud`, `splitscreenzone_hud`), all naming `Data\HUD\Pulse_Ready_Go.vex`. So
  **the on-screen graphic does not vary by mode on any of the three titles checked.**
  Confidence 95 - literal XML off each disc, not decompiled or inferred.
- **The track-side starting-line gantry** (`TrackStartup.xml`'s billboard slot 8, a
  physical object standing on the circuit - see
  [billboards.md](../docs/ghidra/functions/psp-pulse-usa/billboards.md) and
  [a-circuits-billboard-slots-are-a-9-entry.md](a-circuits-billboard-slots-are-a-9-entry.md)):
  **a real difference, now confirmed on HD by listing the actual archive rather than
  string-searching the executable.** `PS3_GAME/USRDIR/DATA00.PSARC`/`DATA02.PSARC`
  carry four `.vex` meshes under `/data/billboards/hd_adverts/321go/` with four
  distinct byte sizes (`321go_startfinish.vex` 12,544 B, `321go_zone.vex` 8,368 B,
  `321go_hd_zone_battle.vex` 10,304 B, `321go_hd_detonator.vex` 11,568 B) - real
  geometry differences, not aliases sharing one file the way the HUD overlay does.
  **Which one a given race actually instantiates remains the billboard thread's own
  open question**, unresolved by this session too. 2048's four-plus-two count from
  last session was string search only, not yet cross-checked against a full archive
  listing the way HD was here.

**So: the on-screen graphic is ruled out as the source of the user's memory on every
title checked; the track-side gantry is a real, disc-confirmed difference on HD (and
very likely 2048), still open on whether it actually renders per mode at runtime.**
That reframing also answers why this thread's earlier passes felt like they were
chasing two different things at once - they were.

2026-09-02, fourth pass: **the user confirmed directly, from playing** - "it's the
text on the gantry/billboard, or lack thereof, in a zone race, it draws a small
rectangular track, instead of the 3 2 1 GO". That is exactly the track-side gantry
from the pass above, and rendering `321Go_Zone.vex` straight off the disc matches it
precisely. `oag-view "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC"
--mesh "/data/billboards/hd_adverts/321go/321go_zone.vex" --screenshot out.png`
renders a small flat rectangular panel carrying a stylised red/green/blue track-loop
icon - no digits, no light arch. `321go_startfinish.vex` rendered the same way is a
large 19-node, 1,530-triangle structure with a checkered-flag/sponsor banner, sized
for a full gantry - 222 triangles on 3 nodes for Zone's, by contrast. The texture
alone predicted this before the render did: `321_go_zone.gtf` is 2048x1024, dwarfing
the 64x128 digit-strip texture (`321_go_64.gtf`) the circuit gantry's actual
countdown numbers use. Full account, screenshots included, in
[billboards.md](../docs/ghidra/functions/ps3-hdfury-eu/billboards.md#the-four-321go_vex-shapes-are-confirmed-as-genuinely-different-content-by-rendering-them).
Confidence 90 that this is what the user remembers - the content match is exact and
the mesh is the only Zone-specific object of this shape on the disc; **confidence
still low (unchanged from the pass above) on the runtime mechanism** that actually
selects it during a real race, which is `mode_descriptor`'s own unresolved pointer
in `billboards.md`, not settled by a render.

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
  connection, or check from the Ghidra GUI directly, before assuming Pure's own
  *code* needs Ghidra at all for a given question, though - the mode-display axis
  turned out answerable straight off the disc with `oag-wad`, no Ghidra needed, and
  that workaround is worth trying first for anything else disc-XML-shaped.
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
- ~~No screenshot comparison of a Zone countdown against a circuit-race countdown~~
  **Done for HD's gantry mesh content, off the disc via `oag-view`** - see the fourth
  pass above. **Still open**: an actual in-game screenshot (this session rendered
  the standalone `.vex`, not a running race, so lighting/scale/camera framing as
  seen in a real race is unconfirmed) and the same check on 2048.
- HD/Fury's `TrackStartup_Load`-adjacent vocabulary sweep (`Countdown`, `StartLight`,
  `RaceStart`, `Grid`) found nothing under those exact names - only `321`/`Ready`/
  `Boost` searches paid off. Worth trying `Light`, `Klaxon`/`Lights`, `Sequence`
  next, since the engine may simply not use those words.
- Pulse's `ready`/`321_GO`/`go` announcer cue set
  (`docs/ghidra/functions/psp-pulse-usa/zone-mode.md:252`) is still unxrefed.
- **Pure and 2048's own countdown *state machine*/logic (as opposed to the display
  asset, now settled for Pure) are still completely unstarted** - Pure's code is
  blocked on the ghidra-mcp gap above; 2048's is simply not attempted yet, though its
  HD-lineage naming (`321Go`/`StartBoost`/`Cockpit321Go` all present, prior session)
  makes it the more promising of the two to try first once the false-lead pattern
  from `Race_CreateModeObject` above is worked out on Pulse.

## Next Steps

**Display/logic, current focus:**

1. **The 18-entry table at `0x08a7a1e8` is a dead end for this, confirmed rather
   than just suspected** - both entries checked land on genuine mid-function
   re-entry points inside one HUD-text formatter (`0x0881d458`, raw
   `disassemble_bytes` shows no function prologue at either), a PSP-overlay
   explanation was checked and ruled out, and it most likely reads as a *later*,
   unrelated per-mode dispatch inside `Race_CreateModeObject` (position-label
   formatting) rather than the mode-object-type switch `zone-mode.md`'s own
   `## Identification` section already documents for `case 6`. **Re-read
   `Race_CreateModeObject` end to end for that first switch instead** - it is
   what actually selects `Zone_Create` vs. whatever a circuit race calls, and
   finding a non-Zone mode's own countdown handler needs that one, then its
   vtable `+0x1c` slot - the direct way to see whether a circuit race's
   countdown matches Zone's 40-tick-cue, two-part-gate
   shape or differs from it.
2. ~~`create_function` for Zone's inner-substate setter~~ **Done this session**:
   named `RaceMode_SetSubstate` (`0x08827454`, confidence 80); the outer-state
   call at substate 4 turned out to already be `RaceMode_SetState` itself.
3. Resolve substate 1's second gate condition (the `fVar17` call) from the raw
   disassembly of `0x08829e6c` rather than a guessed address.
4. **Mode-display axis is closed for Pulse and Pure** - no further work needed there
   (see `hud.md`). For HD/Fury and 2048, **try the same primary-source approach that
   worked here before returning to Ghidra**: both ship data as archives this project
   already reads (`oag-tools`/`oag-wad`-equivalent per `docs/formats/README.md`) -
   if either's HUD/front-end data names its countdown model per mode the way Pulse's
   and Pure's XML does (even if the *file name* itself carries the mode, as
   `321Go_Zone.vex` already suggests), reading that data file directly could settle
   the display question without ever resolving the mode-descriptor pointer. Try this
   before continuing the harder Ghidra trace below.
5. Finish tracing the mode-descriptor pointer from
   [a-circuits-billboard-slots-are-a-9-entry.md](a-circuits-billboard-slots-are-a-9-entry.md)'s
   open item 1 to a `321Go_*.vex` shape, if step 4 doesn't settle it first - the
   fallback lead for the mode-display axis on HD/2048.
6. Get a live screenshot of a Zone countdown next to a circuit-race one on HD/Fury or
   2048 before publishing either a positive or a negative claim about how they
   render - the billboard thread's own stated lesson, from a mistake made on this
   exact asset family. Not needed for Pulse/Pure any more (settled by XML, no render
   ambiguity).

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
