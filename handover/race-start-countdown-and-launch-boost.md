# Race start: the countdown state machine, the launch reaction boost, and whether Zone shows a different countdown

Three questions, tracked as one grid across the four titles this project reads. Opened
skeletal, then one static-only pass landed the same day - see below for what each
"partial" and "strong evidence" cell actually rests on before trusting it.

|                                  | Pure | Pulse                          | HD/Fury                        | 2048 |
| -------------------------------- | ---- | ------------------------------ | ------------------------------- | ---- |
| Countdown state machine + timing | open | **Zone's read statically; a Time Trial's own duration measured live** (below) | open | open |
| Launch reaction speedboost       | open | **no false-start penalty exists, measured live** (below) | **likely not a thing** (below)  | untried, `StartBoost` string present (below) |
| Zone display: HUD overlay        | **confirmed identical** (below) | **confirmed identical** (below) | **confirmed identical** (below) | untried |
| Zone display: track-side gantry  | untried | **no per-mode model exists** - Pulse ships one gantry and Zone's circuit authors no manifest (below) | **matches the user's own description, rendered and confirmed** (below) | asset family exists, untried past a string search |

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

2026-09-02, fifth pass: **the countdown is measured live, not statically, for the
first time this thread has managed - and it settles the false-start question the
opening paragraph above named as untraced.** Three PPSSPP captures of
`pulse-psp-usa.chd` (Time Trial, Talon's Junction White), `cross` held
continuously from before the track description screen through the whole
countdown into the launch: two entered via `psp-drive.py restart`, one via a
genuine front-end menu walk. All three agree to the tick - full account,
including the two process-boundary traps that cost the first two attempts (a
separate-process capture starts too late, and the pause menu's "back to menu"
path needs an explicit `QUIT RACE`, not just `circle`), in
[race-modes.md](../docs/gameplay/race-modes.md#the-countdown-is-measured) and
[engine.md](../docs/ghidra/functions/psp-pulse-usa/engine.md#the-engine-has-an-early-return-that-produces-no-thrust-at-all):

- The countdown, dialog-dismiss to green, is **272 ticks** (4.5377 s dt-summed),
  identical on all three runs.
- **Holding thrust through it is not a false start.** `throttleState`
  (`craft+0x2b8`) reads flat `0.0` for the gated span regardless of input held,
  then steps straight to the full held value - no ramp, no stall. `craft+0x290`
  and `craft+0x2e0` (the two gates `engine.md` already knew about) both stay at
  `0.0` for the whole 1200-tick capture on every run, which also **refutes**
  `engine.md`'s own standing guess that `craft+0x2e0` fits "a capture taken near
  a race start" - it does not arm during one, so whatever pins throttle to zero
  is a third mechanism this capture does not locate.
- This retires the false-start belief this thread's own opening paragraph (and
  four places in `scripts/`) repeated as fact - it was never measured until now.

**Followed by an implementation**, per direction to proceed once the RE
supported it: `oag_race::RaceState::thrust_gated` and `COUNTDOWN_TICKS` (272)
gate the player's thrust for the measured span in `Race::tick`
(`crates/game/src/race/tick.rs`), scoped to exactly what was measured - thrust
only, not steering/braking/airbrakes; every mode, since only Time Trial was
captured and no evidence points at a difference (the same reasoning
`RaceState::eliminate` already uses). Four new tests, `cargo nextest run
-p oag-race -p oag-game`; full `just` gate green.

**What this does not touch.** The gantry/billboard display question - blocked
on the same transform-writer problem [a-circuits-billboard-slots-are-a-9-entry.md](a-circuits-billboard-slots-are-a-9-entry.md)
already named, unrelated to anything measured this pass - is still open and
was deliberately not implemented from a guess. Single Race, Zone's own duration
against this number, and every other title remain unmeasured; see the grid.

2026-09-05, the Pulse gantry cell is answered, and with a negative: **Pulse has no per-mode
countdown gantry at all.** A `strings` sweep of all three Pulse WADs and of `BOOT.BIN` turns
up one gantry name, `321Go_StartFinish.vex` - no `321Go_Zone`, no variant of any spelling -
and `BOOT.BIN` names no gantry model at all, so the model reaches the engine only through
slot 8 of a circuit's manifest. Zone on Pulse runs `26_Track`, which authors no
`TrackStartup.xml` of its own, so it has no slot 8 either. **HD's four-model shape does not
generalise backwards**; the mode axis is real on the later titles and absent on this one.
Confidence 85.

The same pass recovered what that one model *does*, which is this thread's other half:
its `3`, `2`, `1` and `GO` are four per-vertex UV cells of one mesh node, walked across a
16x32 palette staircase by the material's own `TEXOFFSET` track. Full account, with the
phase table and the 90/55 split between what the asset authors and what the engine does
with it, in [start-gantry.md](../docs/rendering/start-gantry.md), now covering all four
titles in the lineage.
**The asset's `GO` lands ~1.5 s / 90 ticks before the 272-tick gate this thread measured**,
and nothing measures the gap - so nothing was wired to `COUNTDOWN_TICKS`.

2026-09-06, the HUD-overlay route is wired for Pulse, and it is a genuinely different
mechanism from the gantry's - not the same UV-staircase trick read onto a second asset.
**`Data\HUD\Cockpit_321GO.vex`, the `Cockpit321Go` `<Mode3D><Model>` widget's own file,
carries the `3`, `2`, `1` and `GO` as four separate mesh nodes** (`Jons321go:x3`, `x2`,
`x1`, `GO`), each with its own `Anim Transform` scale-burst (rest ~3.5x, peaking ~8x) and
its own per-material `TEXOFFSET` sweep across a pure-white, alpha-only 64x16 texture -
confirmed by dumping both textures' texels and both classes of animation track
(`cargo run -p oag-render --example ready_go_census`, kept as the evidence trail). The
sibling widget, `ReadyGo`/`Pulse_Ready_Go.vex`, is a single static mesh whose one material
loops a **0.983 s** texture sweep with no recovered gate - not drawn, since playing it
against the race clock would blink it roughly 4.6 times through a 272-tick countdown, a
duration nothing here measured.

**Rendered through a new, dedicated pass** (`crates/game/src/hud/countdown.rs`,
`oag_render::mesh_render::write_uniforms_raw` added for it), reusing the generic
`TexAnims`/`NodeAnims` playback `start-gantry.md` already said needed no new mechanism -
true here too, once the actual bug was found. **The bug, worth recording because it will
recur**: an orthographic projection built by mirroring `top`/`bottom` to get "HUD-pixel
down is positive" the cheap way also reverses the winding the rasteriser sees, and every
one of this model's eight draws authors `culled=true` - so `Face::Back` culling silently
discarded the entire model. Remapping the HUD `y` to bottom-up *before* an unmirrored
`orthographic(0, 480, 0, 272, ...)` fixed it. Confirmed by capturing all four glyphs in
sequence at a diagnostic screen-centre position (ticks 47/92/137/210, matching each
glyph's own predicted peak-alpha tick): a clean `3`, `2`, `1`, `GO` in order, culling and
orientation both correct.

**What shipped instead drew a clipped sliver at the screen's left edge**, because the
widget's own authored position is `x="0.0" y="0.0"` and the one validated placement
convention this codebase had (`model_draw`, ground-truth tested against Pure's pickup
icons, which author `x="240" y="250"` - dead centre, near the bottom) read coordinates
verbatim. **Fixed the same day, third pass**: the convention itself only holds for one
of two disc-verified `<Mode3D>` dialects, and the countdown lives in the other one - see
below.

2026-09-06, third pass: **the countdown is now visible and legible, and the "invisible
glyph" symptom the second pass's screenshots showed is the same bug, not a second one.**
The second pass's `(0, 35)` was right about the *value* on the disc and wrong about what
it *means*. Every `<Mode3D>` block on both Pulse and Pure, checked across all four
layouts that carry one, is exactly one of two shapes: `mode="orthographic"` on the
block's own `<Values>`, with real screen pixels on every child `x`/`y` (the sights,
Pure's weapon icons and bars - `model_draw`'s convention was measured against this one
and only this one); or `FirstPass="yes" OriginX="..." OriginY="..."` with no `mode` at
all, and `x="0.0" y="0.0"` on every `ReadyGo`/`Cockpit321Go` checked. `model_draw`'s
"literal HUD pixels" reading was applied to the second dialect too, and that - not the
runtime-placement question the second pass closed - was the actual defect. `x=0, y=0`
in the non-orthographic dialect is where a symmetric perspective camera along `-z`
always projects the optical axis, for **any** FOV, near, far and any `z != 0` (the
horizontal/vertical projection term is `(coordinate / -z) * scale`, and `0 / -z` is `0`
regardless of `scale`) - centred, not a placeholder. That is derivable without
reconstructing the original's actual camera, which is why no projection matrix needed
to be built: a guessed FOV would still contribute nothing to `x=y=0` and would only
risk misplacing a future nonzero-authored widget in this dialect.
`oag_game::hud::Layout::collect` now parses `mode`/`OriginX`/`OriginY` per `<Mode3D>`
block (new `hud::Model::orthographic`/`::origin` fields, threaded down `collect`'s
recursion the same way `<Item>`'s offset already is), and
`crates/game/src/hud/countdown.rs` derives the screen position as
`(240, 136) + (OriginX, OriginY) = (240, 171)` for the zero case every layout actually
ships, erroring rather than guessing if a future widget in this dialect ever authors a
nonzero `x`/`y`. Screenshots at ticks 47/92/137/210 (matching each glyph's predicted
peak-alpha tick) show a clean `3`, `2`, `1`, `GO` in sequence, centred - the second
pass's own diagnostic centred capture already proved the render pipeline was correct
once fed a sane position, and this closes the gap between that and what the disc's own
layout actually produces. `pickup_icon_ground_truth`, `hud_layout_ground_truth`,
`lock_sight_ground_truth`, `hd_hud_ground_truth` and `missile_ground_truth` all still
pass (41/41 against real disc images) - the orthographic dialect's own consumers read
`mode="orthographic"` on every block they touch and are unaffected. Full account,
including the `0/-z` derivation and the dialect table, in
[hud.md](../docs/ui/hud.md#two-mode3d-dialects-distinguished-by-modeorthographic) and
`countdown.rs`'s own module doc.


2026-09-06, a pass targeted squarely at the gantry-placement blocker (the transform
question above, named the highest-priority open item across two prior passes): **still not
recovered, but a real error in the existing account is corrected, two leads are ruled out,
and the open question is reframed onto a cheaper next static step.** Full account in
[billboards.md](../docs/ghidra/functions/psp-pulse-usa/billboards.md#the-two-constructors-and-the-shared-object-builder).
In short:

- **`start_grid.vex` is ruled out as a placement source.** `oag-view --nodes` against
  `16_Track`'s copy shows 4 nodes total - `World`, a camera-rig `Anim Transform` pair, one
  `gridCamera` leaf - a Maya camera scene, nothing billboard-shaped.
- **The "literal identity matrix" claim this thread and `billboards.md` both carried is
  corrected, not merely restated.** Re-disassembling `Billboard_ConstructResource_q`
  (`0x08900220`) directly (not the decompiler summary) shows the fourth transform row is
  loaded whole via one VFPU `lv.q`, then its second word is separately overwritten with
  `0xc1200000` (-10.0f) via a scalar `swc1` - so the row that lands is `(x, -10.0, z, w)`,
  not `(0,0,0,1)`. The write is "identity rotation, fixed non-zero translation.y", not
  fully identity. This does **not** reopen circuit-specific placement - -10.0f is the same
  constant on every call regardless of circuit - but it is a genuine correction, and the
  underlying trap (a VFPU quadword transfer decompiling as four separate scalar loads with
  no marker that they are one unit) is now recorded in
  [`workflow.md`](../docs/ghidra/workflow.md) since it will bite again on any other matrix
  write in this binary. `crates/formats/src/trackstartup.rs:173` carries the same stale
  wording and is outside this pass's lane (`crates/formats`) - flagged, not edited.
- **Two structural negatives, checked directly rather than assumed**:
  `Billboard_ConstructResource_q`'s own `param_3` is declared and never read in its body,
  so the outer caller's own (discarded) `param_1` cannot be a parent/locator reference
  reaching construction; and the alloc/`func_0x00140bd4` parent chain (billboard object ->
  a per-track container -> `_DAT_002ae2b4`) reads as an ownership/lifetime tree for
  cleanup, not the renderer's spatial transform-composition chain.
- **Reframed the next step**: `Billboard_ConstructResource_q` also stores a second object at
  `param_1+0x40` (via `func_0x000fc0f8`, real address `0x089000f8` after the
  `+0x08804000` `jal`-target correction), found by an RTTI-style type match against a type
  descriptor from `func_0x00267b30()`. This is a more promising placement-hook candidate
  than `param_1` itself (structurally matching the world-transform-composer traffic
  `billboards.md`'s own live capture already found on a `+0x40`-shaped object) - resolving
  what that type descriptor names is the cheapest next static step, cheaper than another
  live-capture pass.
- **A live PPSSPP capture was attempted and did not complete** - four tries, two harness
  paths (`PPSSPPHeadless`, then SDL+Xvfb per this project's own documented preference).
  `PPSSPPHeadless` died with `memory.read: CPU not started` right at the race-load
  transition, twice. The SDL+Xvfb attempts never got past first-boot dialogs a second
  time, and the actual cause was found afterward: `timeout 90 uv run --with
  websocket-client python3 <script>` does not kill the `python3` grandchild `uv run`
  execs into, so a timed-out reader survives its own timeout, the parent capture script
  hangs in `wait`, and the still-alive `PPSSPPSDL`/`Xvfb` from one attempt squats the
  debugger port and `:98` for the next - `Failed to bind to port 47860` in the new
  instance's log was the tell. Every process this pass spawned was found and killed by
  exact PID afterward; nothing was left running for the next session. Not retried a fifth
  time: the static reading above already explains what the read would have checked
  (a fixed, non-circuit-specific local offset), so a clean capture would confirm rather
  than resolve the open question.

2026-09-06, a sixth pass, targeted squarely at `ReadyGo`'s own recorded blocker (the
`Pulse_Ready_Go.vex` widget `countdown.rs`'s module doc named "no recovered gate" for):
**closed with a negative result, and the reason changes.** Full account in
[countdown-widgets.md](../docs/ghidra/functions/psp-pulse-usa/countdown-widgets.md#readygos-gate-not-a-digit-sync-problem-but-a-genuinely-unrecovered-one).
In short:

- **`Pulse_Ready_Go.vex` is not a digit display.** Rendered standalone off the disc
  (`oag-view --mesh 'Data\HUD\Pulse_Ready_Go.vex' --screenshot`), its one mesh
  (`thingieShape`, 285 vertices) is a nested ring/swoosh - the same shape family
  `start-gantry.md` already measured for Pure's `Ready_GO.vex`. This retires the
  original worry (looping every 0.983 s would "blink it roughly 4.6 times through" the
  272-tick countdown) as aimed at the wrong problem - there are no digit states for a
  free-running loop to misalign against. Confidence 90.
- **What actually blocks it: `Hud_UpdateCountdownFade` (`0x0881f624`) gives this widget
  a real, code-supplied envelope, not a free pass the way `Cockpit321Go` gets one.**
  Direct decompile (not the summary this page already had): with a timer at `hud+0x16c`
  gated by a byte pair at `hud->view+0x58`/`+0x59` (the same largely-unidentified struct
  `lock-sight.md` already reads at `+0x48`/`+0xe4`), `ReadyGo`'s colour is
  `(alpha<<24)|0x050517` with `alpha` a cubic ease from 255 down to 0 by the timer's own
  halfway point - a fade-*out*, confirmed, not a fade-in. `Cockpit321Go` gets flat
  `0xffffffff` from the same call, so the timer contributes nothing to it; that
  asymmetry is why `Cockpit321Go` can be drawn opaque for the whole measured span while
  `ReadyGo` cannot be drawn at all without inventing what its brief flash is supposed to
  look like. Confidence 85 on the envelope math.
- **No writer of the gate byte was found.** One candidate was checked and probably
  ruled out: a global at `0x08ab2120`, cleared at `+0x58` on local-player ship death
  (`Ship_SetState` case 4, already in `zone-mode.md`) - but `InGame_Update` allocates
  that object at exactly `0x68` bytes, too small to also be `hud->view`, which needs at
  least `0xe8` to hold the `+0xe4` field `lock-sight.md` reads on it. That argument
  leans on `lock-sight.md`'s own confidence-50 read and nothing here confirms what
  writes `hud->0x3c` in the first place (`Hud_BindWidgets`, checked directly, neither
  reads nor writes it) - so recorded at **confidence 60, "probably a different
  object," not a confirmed dead end** the way `0x08a7a1e8` is elsewhere in this thread.
- **Not wired.** Per this project's rule against inventing a gate: drawing the mesh
  without its envelope would show a near-opaque ring for the model's entire visible
  window instead of the original's brief eased flash - a plausible-looking stand-in,
  not a recovery. `countdown.rs`'s module doc and `countdown-widgets.md` both carry the
  precise reason now instead of the old, imprecise "would blink funny" one.

2026-09-06, a seventh pass, and the first to leave Pulse for the question:
**the gantry-placement blocker is reframed off a wrong premise, and no live
emulator work is needed.** The premise six passes shared was that the billboard
system places the gantry. Read on HD/Fury - where Ghidra's call xrefs actually
work - it provably does not, on either title. Full account in
[start-gantry.md](../docs/rendering/start-gantry.md#where-the-placement-actually-comes-from-hdfury-answers-it-for-pulse)
and [ps3-hdfury-eu/billboards.md](../docs/ghidra/functions/ps3-hdfury-eu/billboards.md#2026-09-06-the-transform-question-is-answered-and-the-answer-is-the-track-supplies-it).
In short:

- **`get_xrefs_to` works on `ps3-hdfury-eu` for code, and does NOT for data.**
  20 call sites for `RaceManager_GetInstance`; but a data xref reported against
  `0x008a704c` is really a read of `0x008b6f38`, `0xFEEC` away - the reference
  table was built with the wrong TOC and never recomputed, while the decompiler
  uses the right one. Both halves written up in
  [workflow.md](../docs/ghidra/workflow.md). This matters well past this thread:
  the PSP relocation defect does not carry over to the PS3 database for calls.
- **HD's billboard transform is a literal 4x4 identity**, confirmed from raw
  disassembly with every `vsldoi` shift worked through (88) - the PowerPC
  analogue of the VFPU hazard, and the reason `billboards.md` had left it open.
- **`Billboard_LoadModelAndBind` (`0x003a4da0`, named this pass at 80) touches
  the track in exactly one place**: it builds `"billboard" + num` and rebinds
  that material's texture (84). The disc agrees independently - `billboard7.gtf`
  and `billboard8.gtf` ship in all 17 HD environments (90). **The placement is
  authored per circuit in the track model.**
- **Pulse authors the identical convention**: `billboard8.tga` and
  `321backplate.tga` are in `01/02/03/05/16_Track`'s own `track.vex`. This
  overturns `docs/formats/README.md`'s standing rejection of the route, whose
  count-mismatch argument refutes "every slot binds" but not "binding is by
  name" - HD's `amphiseum` ships 3 placeholders for 8 slots and the executable
  does the lookup anyway. Flagged, not edited (`docs/formats/` out of lane).
- **Not recovered: any numeric transform.** `oag-view --draws` needs a GPU
  adapter this box would not present (two runs, zero bytes, exit 144), and
  Pulse's track `.vex` nodes are unnamed. Confidence 78 that the transform is
  track-authored geometry rather than billboard-system state; the weak link is
  that `psp-pulse-usa` ships no lowercase `billboard` base string, so Pulse's
  own binding path is untraced.
- **Nothing was drawn and nothing was synthesised.** No countdown is closer to
  the screen than it was; what changed is that the next step is a static disc
  read (`--draws`, or `--class 0x125/0x06e --payload`) rather than a live
  PPSSPP breakpoint on `func_0x00140bd4`.

## Open

- ~~What places the countdown's `Cockpit321Go` widget~~ **Answered in two parts, and
  both stand.** (2026-09-06, second pass) `Hud_BindWidgets` (`0x0881fbec`) resolves
  `"HUD->ReadyGo"`/`"HUD->Cockpit321Go"` the same `"HUD->"`-lookup way `HudSight_Bind`
  resolves the sights, but the only other consumer of those two slots,
  `Hud_UpdateCountdownFade` (`0x0881f624`), drives a fade timer and a visibility bit
  and never writes a screen position - no `HudSight_Update` counterpart exists, and the
  mesh bakes no absolute placement either. Full evidence, both `jal`-xref-complete
  searches and the vertex-bounds check, in
  [countdown-widgets.md](../docs/ghidra/functions/psp-pulse-usa/countdown-widgets.md),
  confidence 75. **That remains correct: there genuinely is no runtime position
  writer.** What was wrong is a second, separate claim layered on top of it - that
  `x="0.0" y="0.0"` therefore means the same *screen pixel* it would for the sights.
  (2026-09-06, third pass) it does not: the countdown's `<Mode3D>` block is a
  different, disc-verified dialect (no `mode="orthographic"`), and `(0, 0)` there is
  screen-centre under any symmetric perspective camera, not a pixel coordinate - see
  the third-pass account above. A related finding from the second pass also stands
  unchanged: HD/Fury's own copy of this widget (`hud_ready_go.xml`) is
  `<aMode3D>`/`<aModel>` - disabled by the same tag-rename convention `hd-hud.md`
  already documents, not repositioned - so it was never a second data point on
  placement to begin with.
- ~~Whether `Pulse_Ready_Go.vex`'s 0.983 s material loop needs a gate before it can be
  drawn~~ **Answered 2026-09-06, sixth pass, with a negative: it is not a digit
  display (settled, confidence 90), but its actual envelope is code-supplied and
  that code's own trigger - a byte pair at `hud->view+0x58`/`+0x59` - has no
  recovered writer.** See the sixth-pass account above and
  [countdown-widgets.md](../docs/ghidra/functions/psp-pulse-usa/countdown-widgets.md#readygos-gate-not-a-digit-sync-problem-but-a-genuinely-unrecovered-one).
  **What would close this for good, if anyone returns to it**: find the `sw` that
  writes `hud+0x3c` (`hud->view`) in the first place - `Hud_BindWidgets` was checked
  directly and does neither - which would both identify the struct `lock-sight.md`
  has carried at confidence 50 for its own reasons and settle whether `0x08ab2120` is
  or is not the same object, on hard evidence rather than the size argument this pass
  used.
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
- ~~No live capture of an actual false start exists.~~ **Done, fifth pass: there is
  no false start.** Thrust held continuously through the whole countdown, three
  runs, never armed `craft+0x290` or `craft+0x2e0` and never stalled - see above.
  States 6/8 in `shield.md` remain unidentified as *anything* to do with a race
  start; whatever they are, this capture shows they do not fire from held thrust
  before the lights.
- **No player-side launch-timing mechanism has been found on either title searched.**
  This session only ran name/string sweeps, not a trace of the throttle-input path
  through the countdown - a `Ship_UpdateThrust`-adjacent function reading the
  countdown clock during the lights, if one exists, is still unfound. Given HD's
  `StartBoost` reads as pure grid-slot AI tuning, **and Pulse's own thrust gate
  now measured live is a flat 0-to-full step with no ramp or window to time a
  press against**, the working hypothesis is now "there is no player-skill launch
  boost on Pulse" - stronger than before, still not a search of the code itself.
- The billboard thread's own top open item - the mode-descriptor pointer replacing
  `Num==7`'s mesh, traced to one of the four `321Go_*.vex` shapes - still doubles as
  this thread's Zone-display-variant question and is still open in both places.
- **Slot 8's own world transform - the gantry-placement blocker - is still not
  recovered**, after a fifth pass (2026-09-06, above) aimed squarely at it. What that
  pass adds: a corrected static reading of the one transform construction does write
  (fixed local offset, not identity), two ruled-out leads (`start_grid.vex`, the
  parent/ownership alloc chain), and a reframed, cheaper next step (resolve
  `func_0x00267b30()`'s type descriptor for the object at `param_1+0x40`) - all in
  [billboards.md](../docs/ghidra/functions/psp-pulse-usa/billboards.md). A live PPSSPP
  capture was attempted to settle it directly and did not complete in four tries; see
  that page's own account of why, including a `timeout`/`uv run` trap worth avoiding
  next time.
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

## `Cockpit321Go` is gated, but not on the camera view - refuted 2026-09-06

The maintainer reported that the countdown belongs **on the gantry, not the
HUD**, and a coordinator hypothesis followed from the widget's name: that
`Cockpit321Go` draws only in the cockpit camera view, which would explain it
landing on the player craft's nose in external-camera screenshots.

**That hypothesis is refuted.** Read off `psp-pulse-usa` directly:

- `Hud_UpdateCountdownFade` (`0x0881f624`) hides both widgets unless a byte at
  `+0x6f` is set **or** `hud->0x284 == 6`.
- That `+0x6f` byte is populated in `Hud_Update` (`0x0881bf50`) from a
  **settings lookup**, not a camera state: the option named `"Dynamic HUD"`
  (`0x08a79d9c`), compared against `"FE_OFF"`, `"FE_DEFAULT"` and `"FE_FLAT"`.
  A graphics-quality toggle, not External/Internal/Cockpit.
- `hud->0x284` is written **once**, at widget-bind time inside
  `Hud_BindWidgets`'s `+0x169` latch, so it cannot track a view the player
  cycles mid-race. And `== 6` is already identified at confidence 84 in
  [zone-mode.md](../docs/ghidra/functions/psp-pulse-usa/zone-mode.md) as the
  race mode being **Zone**, not a camera value.
- The camera-view enum is the separate, already-documented `camera+0x1dc`
  ([camera.md](../docs/ghidra/functions/psp-pulse-usa/camera.md), confidence 90),
  which carries its own "do not confuse the two" note.

**What this raises instead, unresolved.** The gate reads: hide unless
("Dynamic HUD" resolves to a non-`FE_OFF` state) **or** (the race is Zone). This
project models no "Dynamic HUD" setting anywhere, so there is no way yet to say
whether a default profile would draw this overlay at all in an Arcade race. It
is therefore possible the maintainer's "no countdown on screen" is not only a
scale/position problem - the original may not draw this widget in a default
profile either, with the gantry or a `Pulse_Ready_Go.vex` fallback showing
instead. **Raised as a possibility on this evidence, not a settled conclusion**;
what "Dynamic HUD" defaults to, and what draws when it is off, were not chased.

**The `0x08a7a1e8` table is confirmed a dead end for a third time.** A pass
mid-session misread it as corroboration before finding this thread's own Next
Steps already naming it a false lead - a text-position formatter inside
`Race_CreateModeObject`, not the mode-object switch. Retracted there and
recorded here so a fourth reader does not spend the same time.

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

7. ~~Live-capture a false start in PPSSPP~~ **Done, fifth pass: there isn't one.**
   No further watchpoint work needed on this specific question; states 6/8 remain
   open but are no longer the leading candidate for a race-start mechanic.
8. Trace the throttle/thrust input path during the countdown (starting from
   `Ship_UpdateThrust` or its equivalent, per `docs/gameplay/ai.md`'s and
   `engine.md`'s existing naming) for anything that reads a countdown clock or a
   "pressed near green" timestamp.
9. Identify the ~19-entry mode-class value `Ship_SetState` state 7 reads
   (`0x08aae7e3` / global`+0xb8`) and `FUN_0003c7b0`.
