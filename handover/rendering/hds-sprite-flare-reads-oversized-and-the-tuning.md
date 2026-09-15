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
[engine-trail.md](../../docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md),
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
([known to work](../../docs/reverse-engineering/rpcs3-debugger.md#z0-breakpoints-fire-but-only-under-the-interpreter),
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

**2026-09-15, a ninth session read the function whole - full account on
engine-trail.md ("Ninth session: the complete decompile").** The "bad
instruction data" every session since the fourth worked around was an
undecoded Cell `lvlx` at `0x002a0be8`; the program was reimported under a
language that decodes it and `EngineFlare_RenderTick` now decompiles in
full. What that settles: **this function builds the sprite's quad** (four
half-float clip-space corners at `this+0x274..`, colour
`0xffffff00 | fade*255`, a four-vertex submit through `FUN_002c4ad0` with
the texture at `this+0x1a0`), so the three-session hunt for the draw is
over; **the size law is** `half_height = Flare Radius Min + Flare Radius *
clamp(fade, 0, 1) + Max Radius Jitter * rand01`, `half_width = 4 *
half_height` (the texture is 1024 x 256); **`this+0x18c` is the sprite's
alpha** - `min((1 - saturate((dist * k - 15) / 15)) * powf(view_dot, Flare
Highlight Power = 32) * alpha_walk * Flare Highlight Boost, Flare Opacity
Max)` - not an occlusion parameter; **`0x002a1210` is the occlusion query**
(z-pass count on a `Flare Occluder Radius` proxy, read back two frames
later) and it rejoins the fade path rather than drawing anything; and the
tuning block is the file in row order from row 14 at `+0x44c`, which
corrects three fourth-session labels (`+0x480` is `Flare Radius Min`,
`+0x490` `Max Chromatic Dispersion`, `+0x47c` `Flare Radius`). What it
narrows rather than settles: the only gate left between the occlusion
query and the fade math is `0x002a0bb4`, a test of `craft+0x7a60` (the
owning local player index, `-1` for AI, per its two constructor writers)
against the current view index - which reads as **the sprite is not drawn
for the craft the camera belongs to**, exactly what the fifth session's
zero hits already said, and would mean the compact glow on the player's
own nozzles in the original captures is the flame model, not this sprite.
Confidence 75; one breakpoint at `0x002a0bac` logging `r10` per craft
settles it.

**2026-09-15, later, a tenth session ran that breakpoint - full account on
engine-trail.md ("Tenth session: the owner gate, measured").** Three boots
of a Talon's Junction single race under the interpreter, muted
(`scripts/hd-flare-owner-break.py flare` / `flare-gate`). The render queue
calls `EngineFlare_RenderTick` for all eight flares every frame in a fixed
order, and at `0x002a0bac` the seven AI crafts read `+0x7a60 = -1`,
`r10 = 1` on 30 of 30 frames each, while the player's craft read
`+0x7a60 = 0`, `r10 = 0` on 30 of 30 - with the view index `*0x008c1430`
at `-1` and the camera object targeting no craft throughout. The fade
path's own footprints agree: the noise countdown at `+0x190` cycles 10..1
on every AI flare and never moves on the player's, `+0x18c` stays 0.0 on
the player's. Controls from the first boot: the four-vertex submit at
`0x002a1104` was reached 27 times, all on one AI craft; the occlusion
answer at `0x002a1428` read `1` on all 40 hits including 11 on the player's
craft, so the query is not what stops it. **The sprite is not drawn for
the viewing player's own craft - confidence 92** (runtime-verified, one
binary; split-screen and the camera-object override not exercised).
Getting every craft rather than the first of each frame needed a *hop*
(arm `address + 4`, resume) between hits - `vCont;s` never moved the
thread and a 30 ms free run sampled one craft 172 times in 214; the script
carries it, and `rpcs3_debugger.py`'s new `step()` docstring records the
negative.

## Open

- ~~**The draw is read; what is open is a live confirmation and the
  renderer.**~~ **Landed 2026-09-15, later the same day.**
  `oag_render::exhaust::hd::Sprite` now carries the ninth session's law
  exactly - `half_height = Flare Radius Min (2.0) + Flare Radius (3.0) *
  clamp(fade, 0, 1) + Max Radius Jitter (0.5) * rand01`, `half_width = 4 *
  half_height`, vertex alpha = `fade = min((1 - saturate((dist * k - 15) /
  15)) * powf(view_dot, 32) * alpha_walk * 1.0, 1.0)`, no quad at all on a
  non-positive view dot - and `race::effects::hd_sprite_quad` skips slot 0.
  The `hd.rs` doc comment that said the law was unread is gone. What the
  code labels **chosen, not measured**: `k` (the per-view table at
  `*(r2+0x5aa0)`, never read; 1.0) and the hemisphere of the view dot (the
  page's two open sign conventions; looking into the nozzle is taken).
  Verified on this engine's grid by
  `the_sprite_flare_skips_the_players_craft_and_fades_the_rest_by_the_law`:
  the field stands 34..148 units from the player's camera, so every
  opponent's quad is at alpha 0 there - which is what the tenth session's
  `+0x18c` column read too - and the player's disc is gone. A start-grid
  picture with an opponent inside 15 units needs `--camera-pose` placed
  behind one; this engine's own grid spacing never puts one that close.
- ~~**Whether the original draws this sprite on the player's own craft at all
  - it reads as no.**~~ **Measured 2026-09-15, it does not - confidence
  92** (engine-trail.md "Tenth session"). The gate at `0x002a0bb4` turned
  the player's craft (`+0x7a60 = 0`, `r10 = 0`) away on every one of 30
  frames and passed all seven AI crafts (`+0x7a60 = -1`, `r10 = 1`) on
  every one; the submit at `0x002a1104` was reached only for an AI craft
  and the occlusion query answered `1` for the player's craft too. So the
  compact glow on the player's nozzles in all three original captures is
  the flame `.rcsmodel` plus whatever `Engine Flare Particles` draws, and
  this engine's oversized disc is a sprite the original never puts on the
  player's craft. What stays unmeasured: the split-screen case (`view >=
  0`, read `-1` throughout) and the camera-object override (`+0x1ec` read
  `0` throughout, mode 10 - outside the `{2, 6, 7, 8, 11}` set, so even a
  targeting camera in this mode would not be an exception). Both are the
  listing's word alone.
- Still unlocated consumers: `Flare Max Rotate Angle` (`+0x478`), `Flare
  Size Clamp` (`+0x4b0`), `Flare Depth Bias` (`+0x4b4`) - `RenderTick`
  never loads them; `FUN_002c4ad0` (the four-vertex submit) and the
  occluder proxy draw `0x005f0c10` are the two places left to read.
  `Engine Flare Particles` (`+0x450` enable, `+0x44c` min alpha): no `.pob`
  or emitter matched; the bright dashes under the nozzle in
  `original-rpcs3.png` remain the candidate.
- The trail itself is also suspected inaccurate per the user's report.
  **Checked and refuted**: a separate HD-vs-Fury trail asset (no - one
  asset, disc-wide, colour-mixed by the already-implemented `engineTrail`
  parameter) and a Zone-specific trail material (no - neither the trail's
  nor the flame's material declares any of the 16 known zone-parameter
  hashes). See "Checked: no separate trail for HD vs. Fury" on
  engine-trail.md. If the trail still reads wrong in Zone specifically, the
  live candidate is HD's still-unlocated Zone environment recolour
  ([zone-effectsettings-loader.md](../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md))
  reaching ships too, not a second trail asset - unconfirmed either way.

## Next Steps

- ~~**One picture or one breakpoint run, then the renderer follows from
  it.**~~ **The breakpoint run is done, 2026-09-15** - see the tenth
  session; `scripts/hd-flare-owner-break.py flare-gate` reproduces it in
  about fifteen minutes. The start-grid picture was not taken and is no
  longer load-bearing.
- ~~**Then the renderer, in this order**: (1) the player's craft is
  confirmed skipped, so stop drawing the sprite on the viewing player's craft in
  `race::effects::hd_sprite_quad` - the original's own rule, not a fitted
  multiplier - and the eyeballed `0.57x` decision recorded in the eighth
  session is moot; (2) replace `hd::Sprite`'s radius law and the square
  quad with the traced one (min + radius * clamp(fade) + jitter * rand,
  4:1 wide, alpha = fade), which needs the view dot and the camera
  distance the sim already has; (3) fix the `hd.rs` doc comment that still
  says the law is unread. `CRAFT_ROW_SCALE` stays: the traced half-height
  is in the same model space as before.~~ **All three landed 2026-09-15**
  - see the first "Open" bullet above for what the code labels chosen.
  What would close the two chosen values: read the per-view table at
  `*(r2+0x5aa0)` (one `read_memory` of the TOC slot, then the floats it
  points at) for `k`, and one live sample of the view dot's sign at
  `0x002a0c8c` on an AI craft the camera is behind, for the hemisphere.
- Read `FUN_002c4ad0` (four-vertex submit with a dispersion scalar) and the
  occluder proxy draw `0x005f0c10` for `Flare Depth Bias`, `Flare Size
  Clamp` and `Flare Max Rotate Angle` - the only two functions on this path
  not yet opened. Low priority next to the two items above.
- Separately assess the trail's own accuracy against the original, per the
  user's report - not investigated.
