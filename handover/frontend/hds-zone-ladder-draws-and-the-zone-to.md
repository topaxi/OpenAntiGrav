---
categories: [frontend, rendering]
---

# HD's Zone ladder draws, and looking for its speed-class table found the stage writer nobody could find

2026-08-31, branch `worktree-hd-zone-hud`. Full write-up in
[hd-hud.md](../../docs/formats/hd-hud.md#zones-ladder-draws-and-the-widget-it-is-missing-is-missing-for-a-reason).
**The one to know**: the gap was three gaps, and each one alone leaves the
picture empty - the sprites were off `oag_hd::hud::ALWAYS_ON` (the set was read
off a *speed lap* frame, which authors none of them), the eleven `ZonePlus<N>`
labels carry no `idstring` and so fell off `draw_list`'s text allow-list, and
`RotationTheta` was neither parsed nor correctly applied. Checked against a Zone
frame of the running original the maintainer supplied
(`data/shots/hd_zone_hud_original.png`, gitignored) and reproduced at
`data/shots/hd_zone_hud_ours.png`.

## A renderer bug fell out of it, and it had been latent since the reticle landed

`ui.wgsl` rotated the **unit square** and scaled by the rectangle afterwards -
`scale . rotate`, which shears every quad that is not square. `ZoneBG` is a
676x153 bar at a quarter turn and came out lying on its side at the wrong size.
Every previous caller of `Draw::RotatedSprite` was the lock-on reticle at 8
pixels square, where the two orders agree exactly, so nothing had ever shown it.
Now in pixels. **Worth knowing for any future rotated HUD widget**: a square
test case cannot catch this.

## What is established

- **The fifteen rungs' names**, `oag_hd::hud::ZONE_SPEED_CLASSES`, confidence 84.
  Three independent readings agree one for one: `zonemode.effectsettings`'
  palette keys (`0 Start` .. `14 Supersonic`), the language plugin's own HUD
  strings (`MSC_SVENOM` = `SUB-VENOM` .. `IG_HUD_SUPSON`, with `IG_HUD_MACH1`
  filling `13 Mach 1`), and the reference frame reading `SUB-VENOM` **with the
  string table's hyphen** rather than the palette key's space.
- **`ZonePlus<N>` is `zone + N`**, confidence 85: the widget name, the authored
  placeholders (`1`..`10`, with `ZonePlus0`'s empty - the same list at zone 0),
  and the frame reading `1` to `11` at zone 1.
- **The two retro skins author `CurrentZonePanel` (and, on `2097_hud`, `ZoneBG`)
  as bare grouping elements with no `<Values>`**, so their ladders have no
  column and no highlighted row. 13 / 12 / 11 sprites, asserted.

## The table was found, and it closed a second question on the way

The maintainer asked for it to be recovered rather than fitted, and it is:
`g_ZoneSpeedClassTable` at `0x00860d44`, fourteen 8-byte records of
`{ u32 zoneThreshold, u32 stringIdPointer }` descending to zero, walked by
`Hud_UpdateZoneSpeedClass` (`0x00049718`). Full evidence, addresses, three
accessors and a reproduce script in
[zone-speed-class-table.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-speed-class-table.md);
it is `oag_hd::race::ZONE_STAGES`. Bands `0`-`1`, `2`, `3`-`4`, `5`-`6`,
`7`-`11`, widening to fifteen at the top.

**How it was found is worth writing down, because the obvious searches had all
failed.** Not from the code side at all: the *string ids* the HUD shows
(`MSC_SVENOM`, `IG_HUD_MACH1`, ...) were already known from the language plugin,
and `grep`ping the ELF for one of them landed in a contiguous fourteen-string
blob; the fourteen pointers to it were the table. Three earlier passes had
searched for the consumer by offset and by dataflow and come back empty. **On a
binary where an offset search is defeated by folded index bias, a known string
is a better handle than a known field.**

**And the walker's last instruction is `stw r3, 0x640(r29)` with `r3 = 14 - i`** -
the writer of the per-craft Zone stage index that
[hd-zone-stage-textures-are-grounded.md](../rendering/hd-zone-stage-textures-are-grounded.md)
and [2048-and-hd-ship-an-unread-effectsettings-table.md](../rendering/2048-and-hd-ship-an-unread-effectsettings-table.md)
both had open as "no writer found". So **HD's Zone colour grade escalates now**,
`oag_title::RaceDefaults::zone_stages` is filled in for this title, and the HUD's
class name and the circuit's palette are the same index by construction. It is
2048's architecture exactly: the HUD widget drives the grade.

`14 - i` lands on `zonemode.effectsettings`' fifteen rungs one for one, which is
fourteen independent agreements between a table in `.data` and a table in an
asset file.

## Open

- **What sets the float `Hud_UpdateZoneSpeedClass` compares.** It arrives as
  `f1` from its single caller, which forwards its own argument; nothing traced
  it back to the zone counter's storage. The units rest on the two observations
  from the running game instead (the frame's `SUB-VENOM` at zone 1, and the
  maintainer's zone 2 = Venom), which is a stronger footing than a dataflow
  trace on this binary usually gets.
- **On this reading, a Zone race with its HUD hidden would freeze the colour
  grade**, the store to `+0x640` being inside the HUD-text update. That is what
  the code says about the **original**, and it is still unchecked against the
  running game - no HUD-hide toggle exists on real hardware or in RPCS3 either.
  **This port's own equivalent is answered, 2026-09-07 (see the dated section
  below): this codebase's `sync_zone_grade` does not depend on the HUD draw
  call at all**, which is a statement about the port's architecture, not a new
  reading of the original's unfound writer.
- **The rotation's sign is passed through, not verified.** `ZoneBG`'s quarter
  turn covers the same pixels either way and the ticks are too small to read off
  the frame.
- `ZonePlusLight*` author `color="FEGlobals->HD_Blue"`, which no HUD file
  declares, so they draw white. `FEGlobals` is a live binding rather than a
  load-time constant (`docs/formats/fexml.md`); what the runtime binds is
  unread.
- ~~**The cross-fade's own rate is still unrecovered**, on both titles~~
  **Recovered on HD, 2026-09-15**: the weight `H[e].+0x18` advances by a
  literal `0.01f` per call of `Environment_UpdateStageBlend` (`0x003dd398`,
  TOC slot `0x008b7c28`) and clamps at `1.0`, so the colour cross-fade is 100
  frames long; the sphere radius runs on its own quadratic beside it. See the
  thirtieth pass of
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md).
  2048's rate is still unread.

## A zone-8 frame settled the placement and confirmed the table a third time

The maintainer supplied it (`data/shots/hd_zone_hud_original_zone8.png`,
gitignored; low contrast, the Zone palette at that rung being nearly white). It
reads `8  SUB-RAPIER` on the current row and `RAPIER` on row `12`, four down.

- **The table's widest band so far, confirmed**: zones `7`-`11` on Sub Rapier
  with the bump at `12`. A five-zone band is the case no one-rung-per-zone
  reading could ever produce, so this is a genuinely independent check rather
  than a restatement of the zone-1 frame.
- **The next class's name sits on its row's own line**, same font and size as the
  digit beside it, inset by the 66 units `SpeedClass` already has from
  `ZonePlus0`. The zone-1 frame could not separate that from a competing reading
  - they are six screen pixels apart at `n=1` - and a four-row gap can.
  `the_zone_eight_frame_is_reproduced_row_for_row` pins it.

## 2026-08-31, later still: the class-change sound is wired, the class-change blend is not - and why

The maintainer asked whether a class change's blend effect and sound could be
reproduced. Investigated both; one is, one is written up instead.

**The sound is wired.** `Data\Sound\speech_class.bnk` (`DATA01.PSARC`) is a
**dedicated** fifteen-cue bank - extracted and parsed directly with
`oag_formats::sblk`, not assumed from a name list - naming `ZONEMALE` at cue
`0` and fourteen consecutive cues at `1`-`14`: `MR_SVE`, `MR_VEN`, `MR_SFL`,
`MR_FLA`, `MR_SRA`, `MR_RAP`, `MR_SPH`, `MR_PHA`, `MR_SUP`, `MR_ZEN`, `MR_SUZ`,
`MR_Z_SUB`, `MR_Z_M1`, `MR_Z_SUP` - a 14/14 order match against this thread's
own `ZONE_STAGES` non-`Start` names. The same fourteen also live inside the
general `speech_zone.bnk` at cue indices `26`-`39`, which is where an earlier
pass of this thread's sibling had already spotted them and left them unwired.
Now `oag_title::ZoneClassAnnouncer` /
[`crate::audio::sfx::ClassAnnouncer`](../../crates/game/src/audio/sfx/announcer.rs),
fired on the same `ZoneStages::stage_for` edge the HUD text and the colour
grade already key off - see `crates/game/src/race/tick.rs`. Ground-truthed
against the real disc in `crates/hd/tests/hd_title_ground_truth.rs`'s
`the_speed_class_announcer_names_the_disc_s_own_dedicated_bank_by_cue_index`,
checking both copies by cue index rather than by trusting either one's own
print order. **On the same confidence footing this thread's own milestone
announcer already stands on for HD** - no call site read in the executable for
either ladder - so this is not a new class of inference, just a second axis of
the same one.

**Checked that decoded actually means audible, not just that the report reads
well.** `just play hd --race --mode zone --ticks 1` and
`crates/game/tests/sfx_ground_truth.rs`'s
`wipeout_hd_s_speed_class_announcer_decodes_thirteen_of_fourteen_cues` both
confirm thirteen of the fourteen cues decode to real waveforms. **`MR_SUZ`
("Super Zen") does not** - both its waveforms sit in the second, unidentified
codec `docs/formats/psp-audio.md`'s "A third of HD's waveforms are not
PS-ADPCM" section already names as a disc-wide gap, with no PS-ADPCM
alternate the way `MR_Z_SUB`/`MR_Z_M1`/`MR_Z_SUP` each have. So a Zone race
reaching Super Zen is silent today, honestly - the load report names the
miss - rather than quietly. Full detail and the pinned decode set in
`docs/formats/psp-audio.md`.

**And the two ladders can collide.** `ZONE_STAGES` steps at zones 2, 3, 5, 7,
12, 16, 20, 27, 35, 42, 50, 60, 75; `ZONE_ANNOUNCER`'s milestones are every
five to 50 then every ten to 100. Five zone numbers - 5, 20, 35, 50, 60 - are
on both lists, so this port raises a milestone cue and a class cue on the same
tick there, both dry speech-bus voice lines mixed together with no
arbitration. Whether the original does the same is unread; recorded as an open
question with the exact numbers named, in `docs/formats/psp-audio.md`, rather
than guessed at either way.

**A candidate for the non-verbal half, found and deliberately left unwired.**
`env0_zone.bnk` (HD's Zone environment bank, not `speech_zone.bnk`) names a
cue `ZONEBAR_TRANS` - plausibly the HUD ladder widget's own transition - but
nothing traces a call site for it, and a single unread label is not the
fourteen-way order match the `MR_*` ladder has. **Also found and corrected**:
`docs/formats/psp-audio.md` previously listed a third cue, `HBEAT_ZCHANGE`,
beside `HBEAT`/`HBEAT_GO`. Checked directly this session -
`speech_zone.bnk`'s own header gives `cue_count = 42`, cues `0`-`41`, and no
`HBEAT_ZCHANGE` string is anywhere in the file. The claim was wrong and is
fixed in that doc rather than carried forward.

**The blend is not wired, and the reason is now a complete list rather than a
single open bullet.** Reread
[zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md)'s
twenty-fourth/twenty-fifth passes with this question in mind. The mechanism
*is* understood at 78: a Zone stage change is a sphere - centred on
`zoneOrigin`, radius `zoneColourTint.w` - expanding through the world; inside
it the new stage's colours apply, outside the old stage's. Three of its inputs
are specifically unrecovered, not just "the rate":

1. ~~**`zoneOrigin` (`0x00c81550`) has no writer at all**, confidence 82~~
   **Stale as written - this was only ever true of
   `Environment_UpdateStageBlend` itself.** The writer was found on 2026-09-03
   one call frame up, in `Scene_PrepareFrame` at `0x003ad8d0`-`0x003ad8dc`,
   and re-confirmed on the complete (post-`lvlx`) image on 2026-09-15: `attrib
   0x00c81550` names exactly that function. The source is the float4 at
   `+0xb0` of the entity `session[id]->+0x6adc` names (the twenty-seventh pass
   dropped a `li r0,0x30` and wrote `+0x80`; corrected in the thirtieth pass).
   See [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md),
   passes twenty-seven and thirty.
2. ~~**The radius itself, `H[e].f32@0x08`, has an unchased writer.**~~
   **Found, 2026-09-15**: `Environment_UpdateStageBlend` advances it itself,
   on the `+0x04 == +0x00` arm nobody had read past - `radius += speed`
   (`+0x10`) while under `20000`, `speed += accel` (`+0x14`), and the colour
   weight `+0x18 += 0.01f` to a clamp at `1.0`, all per call. Thirtieth pass,
   confidence 85.
3. ~~**The two schema keys ... are flag-2, developer-only fields.**~~ True,
   and now placed: `Transition start speed` registers to `H[idx]+0x0c` (the
   value the commit copies into `speed`) and `Transition acceleration` to
   `H[idx]+0x14`. Unauthored on the shipped disc, so the `.data` defaults
   `0.5f` and `0.1f` are what the game ran with. Same pass, confidence 85.

All three unknowns are now read. What is still not a measured number is the
*unit* of the radius against the world (the shader's `zoneColourTint.w`
compares it to a world-space distance; whether `oag_render`'s units match is a
renderer question, not an RE one). The port side is untouched by this: the
showing stage's own unblended palette is still what draws, and
[`ZoneGrade::commit`](../../crates/game/src/race/zone_grade.rs)'s own doc
comment still says the transition effect is not drawn - the reason has moved
from "unidentified" to "not yet wired".

## 2026-08-31, later still: the maintainer's own play answers both discriminators

Asked directly, from the maintainer's memory of playing HD/Fury rather than
from anything on disc - so this is play-based evidence, the same kind of
source this thread's own zone-1 and zone-8 HUD frames rest on, not a reading
with a confidence score.

- **The sound is both together**: a spoken class name and a non-verbal tone at
  once, not one or the other. That is independent corroboration for wiring the
  `MR_*` voice line the way this session did, and it strengthens the case for
  chasing `ZONEBAR_TRANS` next rather than treating it as a name-only guess -
  the maintainer's own ear says there really is a second, non-verbal cue to
  find, not just a plausible label sitting in a bank.
- **The visual is a wavefront**, the maintainer's own word, and they place its
  direction as travelling from behind the ship toward ahead of it, along the
  driving direction - stated with a hedge ("I think") on which end leads.

**That second answer corroborates the sphere reading and complicates it in the
same breath.** A wavefront that tracks the driving direction rather than
appearing the same everywhere on the circuit is not what a single **static**
`zoneOrigin` fixed once per race would produce - a sphere centred on one
motionless world-space point looks the same from any approach angle, and
nothing about "behind you, catching up" reads off a point that never moves. Two
readings fit what was said instead: `zoneOrigin` is not static at all but
re-centred on the ship (or the track position under it) every frame, or the
sphere's growth is fast enough from a fixed point that what looks like
"catching up from behind" is really "the near edge of an expanding sphere
sweeping past" - which would look directionless from the driver's seat rather
than tracking behind-to-ahead specifically. The maintainer's own hedge on
which end leads is the detail that would settle between these, and neither
was distinguishable from the static reading alone. **Still not enough to wire
anything**: this sharpens what a watchpoint on `zoneOrigin` should look for
(a value that moves frame to frame versus one written once) rather than
supplying the value itself.

## 2026-09-07: a long race does walk the ladder, but nothing survives to the top of it

The first Next Step, finally watched. `just play hd --race --mode zone --ticks
40000 --screenshot` **as literally written does not reach far**: with no input
held, the craft has no steering, drifts into the walls and is eliminated by
tick 3230 (zone 5, stage 4) - `RaceState::eliminate` ends the race exactly the
way a single race or Zone always can
(`crates/game/src/race/tick.rs`), and `RaceState::update`'s own early return on
`self.finished` (`crates/race/src/state.rs`) then stops `advance_zone` cold, so
the zone counter (and with it the grade) freezes at whatever it reached. That
is not a grade bug - it is what "drive with the pad untouched" always does in
this mode, since Zone's auto-speed needs no accelerate but still needs
steering, which nothing here was supplying.

**With `--autopilot` added, the ladder does escalate, and it is checked at
every rung reached, not just the last frame.** Captures at ticks 600, 1200,
1800, 3000, 4200, 7200, 9600, 12000, 16200, 21000, 25200 and 40000 on the
default Zone track (`/data/environments/zone_1`) each print the exact `zone N:
the colour grade steps to stage S` line
`crate::race::scene::queries::sync_zone_grade` raises, and the HUD's own
ladder column and current-row name step in lock with it: `SUB-VENOM` (zone 1)
-> `VENOM` (2, teal-green) -> `FLASH` (5, purple) -> `SUB-RAPIER` (7) ->
`RAPIER` (12) -> `SUB-PHANTOM` (16, copper) -> `PHANTOM` (20, orange) ->
`SUPER-PHANTOM` (27) -> `ZEN` (35, white/silver) -> `SUPER ZEN` (42). Every one
of those matches `ZONE_STAGES`' own thresholds exactly - this is the same
table `docs/ghidra/functions/ps3-hdfury-eu/zone-speed-class-table.md` names,
now watched stepping in a continuous run rather than at one forced
`--zone-stage`. Nine of these frames are saved at `data/shots/` as
`hd_zone_ladder_zone<N>_stage<S>.png` (zones 1, 2, 5, 16, 20, 35, 42, plus the
eliminated end state and the `zone_2`-track zone-52 frame below), gitignored
like every other capture this thread cites. The full tick-by-tick log and the
rest of this session's working are in `.claude/scratch/zone-grade-report.md`
inside the worktree this landed from - not linked further, since a worktree
scratch path does not survive the worktree.

**Nothing survives to the top two rungs (`Mach 1` at zone 60, `Supersonic` at
75), and that turned out to be Zone mode working as designed, not a defect.**
`oag_race::zone::thrust` scales
speed by the zone number with no ceiling
(`crates/race/src/zone.rs`), so the craft goes
faster every ten seconds forever; eventually every track and every AI skill
level tried (`novice` default and `ace`) loses the craft to a wall and
`RaceState::eliminate` ends the race - "how far can you get before you die" is
the mode. On the default track the autopilot always dies at the same tick
(25466, zone 42, deterministic - same seed, same course) regardless of skill;
the purpose-built `zone_2` arena did better, reaching zone 52-54 (stage 12,
`SUBSONIC`) before the same fate, one screenshot away from `MACH 1`'s zone-60
label already visible as the next row. **Zone 75 (`Supersonic`) was not
reached in any run tried**, and is not verified live - only that the table and
the HUD agree on every rung this session's runs did cross.

**Both of the thread's own open questions got answered along the way, for
free:**

1. **"Would a Zone race with its HUD hidden freeze the colour grade?" - no, on
   this port, and the code says why without needing a toggle that does not
   exist.** `Scene::sync_zone_grade` is called from `RaceStage::render`
   (`crates/game/src/main/race_stage.rs`) unconditionally, before and
   independent of `RaceStage::draw_hud` - the HUD only *reads*
   `Scene::zone_stage()` afterwards to print the class name, it never drives
   the sync. The headless `--screenshot` path shows the same split even more
   starkly: `race::capture` runs the whole tick loop with no rendering at all,
   then calls `scene.sync_zone_grade(&race)` exactly once
   (`crates/game/src/race/capture.rs`), before the HUD overlay is even built.
   So in this port the grade is a function of the zone counter alone, not of
   whether a HUD widget ever draws - **this is a statement about this
   codebase's own architecture**, not a new reading of the original's
   `+0x640` store, which still has no traced writer independent of the HUD
   update and stays open on that side.
2. **The two ladders do collide, and the code path itself arbitrates
   nothing between them - confirmed by reading the call sites, not just
   predicted.** `crate::race::tick` pushes a milestone announcement and (when
   the same `zone_advanced` edge also crosses a `ZONE_STAGES` boundary) a
   class announcement into two separate queues, unconditionally, on the same
   tick (`crates/game/src/race/tick.rs`). `crate::audio::sfx::race_tick`
   drains both queues and calls `mixer.play` once per entry, both to
   `Bus::Speech`, back to back, with nothing between them that checks whether
   the bus already has a voice open - and each call's own result is discarded
   (`let _ = mixer.play(...)`) so a refusal is not surfaced either
   (`crates/game/src/audio/sfx.rs`). Zone 20 (`PHANTOM` + the "20" milestone)
   and zone 35 (`ZEN` + "35") were both crossed live in this session's
   40,000-tick run; its own health log shows voice refusals scattered
   throughout (40 over 25,466 ticks, not concentrated at those two ticks), so
   **whether both cues actually sound together there, or one loses a refused
   slot to something else, is not established** - only that nothing in the
   code path treats a collision differently from an ordinary single-cue zone
   step. Full detail in `docs/formats/psp-audio.md`'s own 2026-09-07 addendum.

## Next Steps

- ~~Check the grade actually escalates in a long race now~~ **Answered,
  2026-09-07 - see the dated section above.** It does, rung for rung, from
  zone 1 to zone 54 across several runs; nothing tried survived to zones 60 or
  75, which is Zone mode's own escalating-speed design ending the race rather
  than a grade defect.
- **Zones 60 (`Mach 1`) and 75 (`Supersonic`) are still unverified live** -
  every autopilot run tried died first, deterministically per track/seed. A
  hand-flown or held-input run that outlasts the autopilot, or a `--seed`
  sweep against the `zone_2`/`zone_3`/`zone_4` arenas, is the way to actually
  see them rather than trusting the table for the two rungs closest to the
  cross-fade blend's own home turf.
- Look for the same `{threshold, stringId}` shape on **Detonator**, whose own
  ladder is recovered by a different mechanism (`RaceManager->+0x2e10`) and may
  or may not share this table's walker.
- ~~An RPCS3 write watchpoint on `zoneOrigin` (`0x00c81550`) and the radius
  field is the one instrument that could close the blend~~ **Both writers
  are read statically, 2026-09-15** - see the corrected item 1 and 2 in the
  2026-08-31 section above and the thirtieth pass of
  [zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md).
  `zoneOrigin` is rewritten every frame from an entity's `+0xb0` float4, which
  is the reading the maintainer's "wavefront from behind" observation asked
  for: the sphere is re-centred on something that moves with the player, not
  fixed once per race. ~~A watchpoint would now only confirm *which* entity
  `session[id]->+0x6adc` is~~ **Confirmed live, 2026-09-15, thirty-first
  pass**: a `Z0` on the store itself, 210 consecutive frames of a Vineta K
  Zone race - the entity is the local craft by pointer identity (the same
  address the flare gate reports as the craft with owner index `0`), the
  `+0x80` block is its transform (three orthogonal rows of norm 0.75, then
  the position with `w = 1`), and the position moved on 207 of 207 frame
  pairs, 0.28 to 2.0 units a frame as the Zone speed climbed. The radius
  law reproduced to the tenth at two frame counts (`k = 53`: 164.4;
  `k = 252`: 3288.7), the weight stepped `0.01` a frame, and the radius
  parks at `20001.86` while the speed keeps growing. `0x009384dd` is the
  pause flag (`g_GamePaused`, 85): `0` in play, `1` with the `GAME PAUSED`
  menu up, `0` again on resume - the wavefront freezes while paused and
  resumes where it was.
- **Wire the transition** once someone owns the renderer side: origin =
  the local craft's world position each frame (measured, 92), radius
  `r_k = 0.1 + 0.5k + 0.05k(k-1)` per frame after a stage commit, capped at
  20000, colour weight `min(0.01k, 1)` - all three now runtime-verified
  (thirty-first pass, 94 on the law) - and both frozen while `g_GamePaused`
  is set. Every number is the disc's (`.data` defaults for the two
  unauthored keys, the constants in `Environment_UpdateStageBlend`), so this
  is no longer a chosen stand-in. The one number still not measured against
  the world is the radius's *unit*: `burst-1.png` in
  `data/reference/hd-capture/flare-owner/zone/` shows the boundary a few
  hundred units ahead of the craft at radius 799, which is consistent with
  world units but is a picture, not a measurement.
- ~~If the maintainer plays past a class change and can say whether the world
  visibly repaints outward from a point versus changing everywhere at once~~
  **Answered, 2026-08-31 - see the dated section above.** It is a wavefront,
  which corroborates the reading; what it corroborates *against* (a fixed
  world-space `zoneOrigin`) is the open question that answer raises.
