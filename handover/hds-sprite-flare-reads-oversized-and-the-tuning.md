# HD's sprite flare reads oversized against the original, and the tuning file's other 19 rows are read

2026-09-01. Started from a user report from play: the exhaust flare "too
big/too far out", with the trail also suspected. Confirmed the first half
with a matched-ish screenshot pair rather than guesswork -
`data/reference/hd-capture/flare-size/` holds `original-rpcs3.png` (a driven
Talon's Junction lap via `scripts/rpcs3-drive.py race --drive 15 --shots`)
against `ours.png` (`just play hd --race --press cross --ticks 300`), same
track and a comparable chase framing. The original's core is a compact glow
at each nozzle, narrower than the hull; this engine's is one white disc
spanning past both nozzles, roughly hull-width. Full writeup, including what
was ruled out (a second locator, the constants themselves) and the six
tuning-file rows read for the first time (`Flare Highlight Power/Boost`,
`Flare Size Clamp`, `Engine Flare Particles Min Alpha`/`Enable`, on top of
confirming `Spikes Thrust/Boost Max Scale` and `Shockwave Cycle Speed` stay
unread) is on
[engine-trail.md](../docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md),
"All 41 rows, read 2026-09-01".

**Why this isn't a code change yet.** `Flare Fadeout Dist`/`Range` (15.0
world units each) is the leading candidate - a near/far fade landing "small
at ordinary chase distance" matches both screenshots - but it is a reading
of two numbers, not a traced consumer, and `hd.rs`'s existing doc already
flagged the same shader pair (`engineflare_vp`/`fp`) as unresolved through
the registry script two sessions ago. Writing a fade curve to fit two
screenshots is exactly the invented-stand-in shape CLAUDE.md rules out, so
`hd::Sprite`'s radius law is untouched. Two static leads were run down and
closed rather than followed further: `EngineFlare_Init`'s `+0xe4`/`+0x144`
writes are never read inside `EngineFlare_PlaceShapes` or
`EngineFlare_Update` (grepped both functions' full decompile), and the
function sitting between `EngineFlare_Init` and `EngineFlare_PlaceShapes` in
address order (`FUN_002a17e8`) turned out to be the engine-sound cue state
machine, not a draw call. The function that actually turns `Flare Radius`
into a world-space quad is still unlocated.

**2026-09-01, a second session pushed further on this specifically and still
did not find it - full account on engine-trail.md ("What two sessions'
worth of reading could not settle").** A correction first: `+0xe4` is not
radius storage, it is the exhaust intensity field - decompiling
`EngineFlare_Update` in full (not done in the first pass) shows it ramped
0.25/s up, 0.5/s down, clamped `[0, 1]`, the wrong range for a 2-3-unit
radius. `Flare Size Clamp` and `Engine Flare Particles`
(from the first session's read) are still unread past the tuning file.

Then a live pass: `scripts/hd-flare-sprite-dump.py` dumps the whole
`EngineFlare` object and flags every 4-byte word in RSX range
(`0xC0000000..0xD0000000`), reasoning that the sprite's own vertex buffer
would settle the true half-size independent of the still-open camera
problem. It found six such pointers (`+0x70`/`0x74` a pair, `+0xb8` static,
`+0x1f0`/`0x1f4` duplicated at `+0x230`/`0x234`, `+0x240` single) but every
one reads back near-all-zero - not plausible vertex positions. Best
reading: occlusion-query result buffers (double/triple-buffered for
readback latency), matching the still-unread `Flare Occluder Radius`/
`Flare Depth Bias`. `Billboard.cpp` (found via the `__FILE__` attribution
trick) was also checked and ruled out - track sponsor signage, unrelated.
Artefacts (the object dump, the six candidate buffers, the meta) are under
`data/reference/hd-capture/flare-size/sprite-dump/`.

**2026-09-02, a third session found the draw call - full account on
engine-trail.md ("Third session: it was on `EngineFlare`'s own vtable, not a
separate manager").** Neither of the two untried angles the second session
left open was quite it: not a separate manager object, and no pushbuffer scan
was needed. `EngineFlare`'s own vtable (`0x00869d30`) has two class-specific
slots besides `EngineFlare_Update` that neither prior session had opened -
slot 5 and slot 7, the identical two slots `TrailEffectManager`'s vtable uses
for `Trail_Enqueue`/`Trail_RenderTick`. Read live off the vtable (8-byte OPD
stride, no symbol to decompile from since neither had a name yet): slot 5 is
`EngineFlare_Enqueue` (`0x002a06e0`, confidence 84 - byte-for-byte
`Trail_Enqueue`'s own function, confirmed to write into the *same* render-queue
object via two different TOC slots holding an identical pointer value); slot 7
is `EngineFlare_RenderTick` (`0x002a08a8`, confidence 78 - gated on a flag in
the same shared per-frame global `Trail_RenderTick` also gates on, at a
different byte offset, and reaching `Rsx_SetMethod` through a chain of
TOC-fixup trampolines). This is the missing consumer. What it does not yet
settle: which of its loads is `Flare Radius` itself - most of the function's
body past the gate is RSX submission plumbing, not the tuning values.

**2026-09-02, a fourth session read that middle and confirmed the tuning
block, but found a puzzle rather than closing it - full account on
engine-trail.md ("Fourth session: the tuning block itself, and a puzzle it
did not settle").** The "bad instruction data" Ghidra's decompiler reported
in `EngineFlare_RenderTick`'s middle is a decompiler-analysis failure, not
bad bytes - `disassemble_bytes` reads it as ordinary AltiVec. Read that way:
a camera-relative inverse-length term feeds a `saturate()`-shaped linear
fade, `1 - saturate((f30*f28 - f23)/f24)`, multiplied by a per-instance value
and clamped, then stored at `this+0x18c` right before an early-out. A fixed
and rerun `scripts/hd-flare-tuning-dump.py` (the first run had an
un-followed second pointer dereference and read every offset as `0.0`)
confirms `f23`/`f24` are **exactly** `Flare Fadeout Dist`/`Range` (15.0/15.0,
at `+0x484`/`+0x488` of the tuning block) - the session-one hypothesis is now
structural, not just numerical - and along the way pinned live runtime
addresses for nine of the twelve previously-"unread" tuning rows at once
(table on engine-trail.md), correcting the `Flare Size Clamp` guess: the
clamp this formula actually applies is a **different** field reading `1.0`
live, not `50.0`.

**What it could not settle: the sprite visibly draws, but this function's
own fade output never left `0.0`.** Four half-second-spaced live samples of
`this+0x18c`, taken from a craft confirmed mid-race, thrusting and visibly
flared in the same session's screenshot
(`data/reference/hd-capture/flare-size/tuning-dump/race.png`), all read
`0.0` - despite both gates this session could check reading as open (one
corrects a session-three misreading: the count-style gate is an *unsigned*
compare, so `craft+0x5fa4 == 1` passes it, not fails it as `count >= 7`
would suggest). Either `EngineFlare_RenderTick` is not the branch drawing
this craft's visible sprite (a third, untraced gate, or the wrong vtable
slot), or it is the right function and `this+0x18c` feeds something other
than the visible quad - an occlusion-query parameter is at least as
plausible as a size or alpha term, given the clamp ceiling turned out to be
`Flare Occluder Radius`'s own value. Settling this needs a breakpoint trace
under `PPU Decoder: Interpreter (static)`
([known to work](../docs/reverse-engineering/rpcs3-debugger.md#z0-breakpoints-fire-but-only-under-the-interpreter),
~75s to boot against the default 30s) - not attempted this session.

**2026-09-02, a fifth session ran a first breakpoint trace, got one solid
result and one wrong reading of a second - full account on engine-trail.md
("Fifth session: a real result (the entry fires), and a wrong reading of a
second one"), corrected there after an advisor review caught it before it
went further.** `scripts/hd-flare-rendertick-break.py` (new) confirmed
`EngineFlare_RenderTick`'s *entry* fires on the very first 0.4s poll of a
driven race - real runtime verification the function is not dead code, it
runs essentially every frame. It then armed a second breakpoint at
`0x002a1110`, believing that address to be the landing both early-out gates
share, and read "12 of 12 hits there" as "the gate stayed closed every
time." **That reading is wrong: `0x002a1110` is the function's one shared
epilogue**, returned through by every path including the full-draw one
(`0x002a1434: b 0x002a1110` sits right after the RSX submission block that
actually issues the draw calls) - so 12/12 hits there is equivalent to
12/12 calls returning, which was never in question. What still stands: a
*different* breakpoint in the same run, `0x002a0d78` (only reached if the
fade value just computed is `> 0`), recorded zero hits - consistent with
either a gate closing before the fade math runs, or the fade math running
every time and evaluating to `<= 0`, which this session's checkpoint
placement cannot distinguish. A separate bug in the same script
(`step_off_breakpoint` not checking which address it stopped at before
re-arming) means even that zero-count isn't fully trusted yet - a hit could
have been silently swallowed. The GDB-stub trap the script found (resuming
a thread parked on an armed breakpoint does not step over it, hanging the
next command) is unaffected by any of this and still real.

**Same session, the corrected rerun: a clean negative result.** Rewrote the
script with a three-point ladder *inside* the fade math (`0x002a0bb8` where
it begins, `0x002a0d70` the store itself, `0x002a0d78` only if positive)
and fixed the swallowed-hit bug. **Zero hits at any of the three in 45
seconds of active, visibly-flared racing** - roughly 2,500 calls at this
function's every-frame rate, none of which reach `0x002a0bb8`. This is a
real, trustworthy finding this time: `EngineFlare_RenderTick`'s fade math is
not entered at all for this craft, not merely computing a non-positive
result. See engine-trail.md ("Fifth session", the "corrected rerun"
paragraph at its end) for the full account and the archived artefacts
(`data/reference/hd-capture/flare-size/rendertick-break-v2/`).

**2026-09-02, a sixth session found and confirmed which gate - full account
on engine-trail.md ("Sixth session: the third gate, found and confirmed
live").** Both known gates measured open (`scripts/hd-flare-gate-check.py`,
new: 1 sample of `count_field_ble`, 4 of `byte_flag_beq`, all "would not
skip"), so re-reading the stretch between the byte-flag gate and the fade
math's own start found a **third** branch neither prior session had traced:
`0x002a0b44`, a `beq` on `FUN_006765e8()`'s return value, branching to
`0x002a1210` - genuinely different code, not the shared epilogue - when
false. **Confirmed live, 2/2 samples: it is false every time**, which is
what has been closing the fade math this whole thread. `FUN_006765e8`'s
only other callers, `Game_PresentLoop_q` and `PhotoMode_Update`, and its
own two-field compare-then-flag shape, read like a per-frame budget or
throttle check - a hypothesis from the call-site pattern, not a traced
value. **What reopens the question rather than closing it**: `0x002a1210`
is not a dead branch - past two more gates of its own (which can loop back
to the same entry the fade math's `r10` computation uses) it builds a small
ring-buffer entry and reaches the same family of RSX-submission
trampolines the fade path does. So the function has two branches that both
touch the RSX pipeline, and which one (if either) actually builds the
visible sprite's quad is now the real open question - not just "why does
the fade math not run."

## Open

- **The gate that closes the fade math is found and confirmed live, and a
  seventh session then ruled the fade math out as the oversized-flare cause
  regardless.** `EngineFlare_RenderTick` (`0x002a08a8`, confidence 78)
  computes `1 - saturate((dist_term*k - 15.0)/15.0)` (`Flare Fadeout
  Dist`/`Range`, confirmed live at `+0x484`/`+0x488` of the shared tuning
  block) but never reaches that computation for the sampled craft - the
  byte-flag gate (`struct_base+0x494`) and count-field gate
  (`craft+0x5fa4`) are both confirmed **open** (5 live samples, none skip),
  and the actual gate is `0x002a0b44`, a branch on `FUN_006765e8()`'s
  return value, confirmed **closed** (2/2 live), which sends execution to
  `0x002a1210` instead. **But the fade's own numbers rule it out as the
  size cause even if it did run**: a 15-unit fade start leaves anything
  closer than 15 units - both engines' chase camera sits a few units out -
  fully faded *in*, undiminished, so reaching this branch would not shrink
  the sprite. Neither branch this thread has found writes the sprite's
  half-size at all; that is a flat renderer-side constant
  (`oag_render::exhaust::hd::SPRITE_RADIUS`, 3.0) with no distance term.
  `0x002a1210`'s own shape - a `craft+0x270`-indexed ring buffer entry
  reaching the same RSX-submission trampolines - reads more like the
  occlusion query `Flare Occluder Radius`/`Flare Depth Bias` want than a
  second draw path, and an occlusion query would not set the size either.
  **The size question now points at the renderer** (a missing
  distance/occlusion clip against the craft's own mesh, or a wrong
  hull-to-flare scale ratio), confirmed by three independent RPCS3
  screenshots that all show a small tight glow against this engine's large
  blob at comparable chase framing. See engine-trail.md "Sixth session" and
  "Seventh session" for the full account.
- `Flare Size Clamp` (50.0): **corrected 2026-09-02** - not what clamps the
  fade result computed above (that clamp uses a *different* tuning-block
  field, `+0x4a8`, which reads `1.0` live and is more likely
  `Flare Occluder Radius` reused as a ceiling). `Flare Size Clamp` itself has
  a confirmed runtime address (`+0x4b0`, still 50.0 live) but no traced
  consumer.
- `Engine Flare Particles` (`Enable`, `Min Alpha` 0.25): a third flare
  component distinct from the always-on flame model and the sprite, neither
  of which this project draws. The small bright dashes trailing under the
  nozzle in `original-rpcs3.png` are the leading candidate for what this
  is. No `.pob` or emitter table has been matched to it. Runtime storage for
  `Min Alpha` confirmed live at `+0x464` of the tuning block, 2026-09-02.
- `Flare Highlight Power`/`Boost` (32.0/1.0), `Spikes Thrust/Boost Max
  Scale` (1.5/2.0), `Shockwave Cycle Speed` (0.01): all now have a confirmed
  live runtime address in the shared tuning block (table on engine-trail.md,
  "Fourth session") but no traced consumer past that.
- The trail itself is also suspected inaccurate per the user's report.
  **Checked and refuted**: a separate HD-vs-Fury trail asset (no - one
  asset, disc-wide, colour-mixed by the already-implemented `engineTrail`
  parameter) and a Zone-specific trail material (no - neither the trail's
  nor the flame's material declares any of the 16 known zone-parameter
  hashes). See "Checked: no separate trail for HD vs. Fury" on
  engine-trail.md. If the trail still reads wrong in Zone specifically, the
  live candidate is HD's still-unlocated Zone environment recolour
  ([zone-effectsettings-loader.md](../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md))
  reaching ships too, not a second trail asset - unconfirmed either way.

## Next Steps

- **Check whether the original's flare is actually depth/occlusion-clipped
  against the craft's own hull mesh, and this engine's is not.** The
  screenshot comparison (engine-trail.md "Seventh session") shows the
  original's flare tucked tightly between the engine nozzles - plausibly
  because most of a larger quad is hidden behind the hull geometry around
  it - while this engine's draws the full unclipped blob. Check the
  sprite's draw call in `crates/render/src/mesh_render.rs` (the
  `depth_stencil` blocks around lines 559-707) for whether it is issued
  with a depth test against the already-drawn opaque hull, and if so,
  whether the flare's world position sits far enough inside the hull's own
  geometry for that test to matter, or if it's placed in open air below the
  ship where nothing would clip it either way. This is a renderer/asset
  question, not a Ghidra one - no emulator needed to start it.
- **Separately, check whether `SPRITE_RADIUS` (3.0 world units) is
  plausible against this craft's own hull scale.** No measured hull-width
  constant was found in this session's search of `crates/physics/src/params.rs`
  (`Hull width` field, `HULL_SCALE = 0.75` in `crates/physics/src/pair.rs`) -
  reading an actual craft's authored width and comparing its half-extent to
  a 3.0-unit sprite radius would say whether the constant itself is
  disproportionate to the ship, independent of any occlusion question.
- If both come back clean (sensible radius, correct depth test, still
  oversized), then `0x002a1210` and `FUN_006765e8()`'s frame-budget-throttle
  hypothesis (global at `PTR_DAT_008b7838`, two 8-byte fields, called only
  from `Game_PresentLoop_q`/`PhotoMode_Update`) are still open, live-tracing
  targets - but they bear on *whether* something like an occlusion query
  runs, not on the sprite's size, so they are lower priority than the two
  bullets above.
- Separately assess the trail's own accuracy against the original, per the
  user's report - not investigated this session.
