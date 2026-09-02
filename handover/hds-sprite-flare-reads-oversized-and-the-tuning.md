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

**2026-09-02, a fifth session ran that breakpoint trace - full account on
engine-trail.md ("Fifth session: the breakpoint trace, and a new stub trap
it cost").** `scripts/hd-flare-rendertick-break.py` (new) armed `Z0`
breakpoints at the shared early-out landing and just past the `this+0x18c`
store, under the interpreter. **12 of 12 hits across two separate races
landed at the early-out; zero reached the fade store.** A simpler first
version also confirmed the function's *entry* fires on the very first 0.4s
poll, so `EngineFlare_RenderTick` is not dead code - it runs essentially
every frame - but for the sampled craft it reliably bails before computing
the fade this session traced statically. This is now the leading
explanation for the fourth session's `0.0` reads: not the wrong function,
but a gate that stayed closed both times sampled. **What it does not
settle**: which of the branches converging on the early-out actually fired
(the breakpoint answers "did not reach the fade store," not "which gate"),
so the byte-flag/count-field reads from the fourth session are still
unconfirmed as *the* cause. A new GDB-stub trap paid for this - resuming a
thread parked on an armed breakpoint does not step over it, so the next
unrelated command hangs for a full timeout - not yet added to
`rpcs3-debugger.md` itself since one script's workaround isn't yet a
general finding.

## Open

- **The draw call is located, its fade formula is read, and a breakpoint
  trace confirms it runs but reliably skips the fade - which gate is closing
  is what's left.** `EngineFlare_RenderTick` (`0x002a08a8`, confidence 78)
  computes `1 - saturate((dist_term*k - 15.0)/15.0)` (`Flare Fadeout
  Dist`/`Range`, confirmed live at `+0x484`/`+0x488` of the shared tuning
  block) and stores the result at `this+0x18c` right before an early-out -
  but 12 of 12 breakpoint hits across two races landed at that early-out,
  never at the store past it, even though the function's entry fires every
  frame. Two explanations remain open: the gate is legitimately closed for
  ordinary chase-view racing (in which case the visible sprite's size/alpha
  comes from somewhere else in the function, or from a wholly different
  branch this session never traced), or the sampled craft/conditions happen
  to always close it and a different craft or camera angle would open it.
  Distinguishing which of the several branches converging on the early-out
  fired - not just that one of them did - is the direct next step; see
  engine-trail.md "Fifth session" for the full account.
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

- **Distinguish which gate is closing.** `scripts/hd-flare-rendertick-break.py`
  answers "reaches the fade store or not" but not "why not" - the early-out
  landing (`0x002a1110`) is shared by at least the byte-flag check
  (`struct_base+0x494`) and the count-field check (`craft+0x5fa4`), and
  possibly others past what this thread's sessions traced. Two ways to tell
  them apart without more static reading: read the flag/count fields
  *immediately before* the breakpoint fires in the same paused state (the
  script already has the connection open at that point, just add the reads),
  or set breakpoints on the individual `beq`/`ble` instructions themselves
  (`0x002a0974` for the byte-flag branch, `0x002a0960` for the count-field
  one) instead of their shared landing.
- If the gate turns out to be legitimately closed for ordinary racing (not a
  wrong-function or wrong-craft artefact), the visible sprite's size/alpha
  is not what this function's traced fade computes, and the search restarts
  for what does - possibly later in the same function, past what this
  thread's sessions decompiled/disassembled so far
  (`0x002a0d78`-`0x002a14ff`), or a genuinely different draw path.
- Once the consumer's radius math is read, either confirm and implement the
  `Flare Fadeout Dist/Range` law or find what actually scales the sprite.
- Separately assess the trail's own accuracy against the original, per the
  user's report - not investigated this session.
