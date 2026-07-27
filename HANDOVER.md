# Handover

State that is **not** inferrable from the repository itself. Everything about
formats, decisions and the plan lives in [`docs/`](docs/README.md); this file
covers what a fresh reader would otherwise have to rediscover.

Written 2026-07-27. The pass before this one opened M4: five crates, two formats
decoded, and the control force law recovered. **It was never committed**, and
this pass starts by committing it as three commits (`be5da47`, `9d14acd`,
`ce3a958`) before doing anything else. If you are reading this after a session
that ended abruptly, check `git status` first - a whole milestone's work sat
uncommitted in the working tree for a day.

This pass then opened **M3**, and that is the news: the original can now be
driven and read from a script. Four things that were static readings are now
runtime-verified, and one open question the roadmap tracked is answered.

## Read this first: the disc images are in place

`data/images/` now holds all three, copied from `~/Downloads` this session:

| File | SHA-256 matches [`source-images.md`](docs/reverse-engineering/source-images.md) |
| --- | --- |
| `pulse-psp-usa.chd` | yes |
| `pulse-ps2-eu.chd` | yes |
| `pure-psp-usa.chd` | yes |

All three hashes were recomputed and match the recorded values, so **every
finding in `docs/` is reproducible against exactly this data**, and
`just test-data` works. Earlier sessions had them and a later one did not, which
cost that session the ability to verify anything; check they are still there
before concluding a ground-truth test "cannot run". The originals in
`~/Downloads` were copied, not moved, and are untouched.

## In flight

A new pass started same day: **both binaries are now in one Ghidra project.**
`SCES_547.48` (PS2, `r5900:LE:32:default`, image base `0x00100000`, 28 overlay
spaces, 5,234 auto-analysed functions) is imported alongside `BOOT.BIN` and was
confirmed live via `list_instances`/`list_open_programs` - it is the *current*
program; `BOOT.BIN` is in the project but closed. Roadmap M2's "Load
`SCES_547.48` into Ghidra" checkbox was stale (unchecked despite being done) and
is now fixed.

Two background agents were launched to work this concurrently, kept to two
rather than the allowed three to avoid Ghidra/build contention:

1. **PS2 Ghidra names catch-up** - cross-reference the PSP's 120 documented,
   evidence-backed symbols (`docs/ghidra/functions/psp-pulse/names.tsv` and the
   pages under that directory) against `SCES_547.48` by string/constant/
   call-graph correlation rather than address (addresses don't correspond
   across the two images), following ADR-0005 exactly: same base name across
   platforms, `_q` under confidence 70, no rename at all under 50. Scoped to
   `docs/ghidra/functions/ps2-pulse/**` plus "Cross-platform" table edits on the
   existing PSP pages, and told to always pass `program="SCES_547.48"`
   explicitly on every Ghidra MCP call so it can't collide with concurrent work
   against `BOOT.BIN`.
2. **Fall-through-floor / jittery-camera bug** - a user report against
   `just play --race`: the ship falls through the floor and the chase camera
   jumps up and down even at rest with no input held. This may be the same
   roll-oscillator instability already on record below (`h <= c/k = 0.005` vs
   the specified `1/180` sub-step, ~11% too large - which by construction should
   show up at rest too, not only while driving), one of the two "cheap to test"
   M4 explanations (hover target exceeding probe reach; missing inertia tensor),
   a regression since `19611e0`/`5ad69f3`, or a camera-side bug reading stale
   ship state independent of the ship actually moving. Scoped to
   `crates/physics`/`crates/gameplay`/`crates/render`, told explicitly not to
   tune a magnitude to hide the known oscillator instability without evidence.

Both report back before editing `HANDOVER.md` further; findings get folded in
here once they land, not written concurrently by the agents themselves.

`just` last passed on this branch: fmt-check, clippy `-D warnings`, **546
tests**, check-docs. `just audit-leakage` is clean.

## The PS2 binary just gave the whole tree its second-binary leg

`docs/ghidra/functions/ps2-pulse/` went from a stub to five subsystem pages -
**48 symbols named in `SCES_547.48`** (20 at 90-95, 22 at 80-89, 6 at 78-79,
nothing below 70) - correlated against the PSP's 120 by string/constant/
call-graph shape, never by address, since the two images share no address
space. `docs/ghidra/functions/README.md` now indexes both binaries side by
side, and each PSP page that gained a PS2 counterpart
(`wad-subsystem.md`, `engine.md`, `input.md`, `camera.md`) got a
**Cross-platform** section rather than a silent update.

Per the [confidence rubric](docs/reverse-engineering/confidence-rubric.md), a
second binary is worth more than a second reading of the first, and this is
the first time this project has had one for the physics-adjacent claims that
matter most:

- **The 32-field handling block (`0x94`-`0x114`, stride `0x80`) and its four
  load-time scale factors are exact matches**, from independent PS2 parsers,
  under a different compiler and a different ISA (Allegrex vs Emotion Engine).
  Same for the 27-float camera block, the `pi/180` `AirbrakeGraphics`
  conversion, and the WAD name hash (CRC-32, reflected, seeded to 0, `\`->`/`,
  uppercase-folded, `#`-hex escape) - two independent PS2 implementations,
  both matching the PSP reading.
- **`stats_base + 0x90` is answered**: `<Misc weight_distribution>`. The PSP
  `engine.md` page recorded that offset as an unidentified per-team scalar; the
  PS2 binary has a `<Misc>` XML element the PSP page never mentions, and its
  parser writes `0x90` directly.
- **Five deliberate, unreconciled divergences**, recorded as findings rather
  than papered over: PS2's WAD lookup restarts its scan every time instead of
  rotating from the last hit; `cancel`/`backward` bind to `triangle` on PS2
  where PSP uses `circle`; PS2's `Input_ConsumePress` clears the *entire*
  pressed mask and ignores the index argument, contradicting the PSP page's
  confidence-70 "clears one bit" reading; **the chase camera's unexplained 3/4
  factor from this session's PSP camera work does not exist on the PS2 path**,
  which computes its distance a visibly different way; and PS2 builds three
  separate lazily-constructed CRC tables plus a fourth static one, where PSP
  builds one.
- **`0x00213e58`, the PS2 LZSS stream constructor, is now the single
  highest-value RE target left** (confidence 55, decode loop not yet read) -
  the PS2 archives are the only corpus that exercises this project's LZSS
  reader at all, and it has so far only ever been validated against its own
  inverse.
- **Not yet attempted on PS2**: the collision walk, the front-end video/Movie
  widget, the main loop, and - most relevant to the M4 physics work happening
  elsewhere this pass - the `Ship_Update*` force terms themselves. Since the
  handling *parameter block* is now confirmed byte-identical in layout, its
  consumers reading those same offsets is the obvious next step, and would pull
  `engine.md`'s force law out of its current decompilation-only confidence cap.

One tooling fix landed alongside the docs: `apply-ghidra-names.py`'s evidence
check hardcoded `psp-pulse` as every row's base directory, which would have
silently failed every PS2 row. Evidence paths now resolve relative to the TSV
that carries them. Verified both ways: PSP's default set still applies clean
(433 applied, 0 failed), and a dry run of the new
[`ps2-pulse/names.tsv`](docs/ghidra/functions/ps2-pulse/names.tsv) applies all
48 with nothing failing. There is no `just` recipe for the PS2 set yet; see
`ps2-pulse/README.md` for the direct invocation.

**Left for later, out of this pass's scope:** `docs/formats/wad.md` should
record that its header/entry layout is now confirmed in a second binary, and
`docs/formats/handling-stats.md` should gain the `<Misc>` element.

## LZSS has its second binary now, and it agrees - with two new caveats

The same agent went on to read `SCES_547.48`'s LZSS decoder
(`Lzss_Decode`, `0x00214080`, plus `Lzss_ReadBits` at `0x00213f70`) end to end.
**No divergence.** Window `0x1fff` (8192), write cursor seeded to 1, a 13-bit
position used as a direct (not relative) ring index, length biased by two into
a loop that runs one iteration past the bias (3..18), flag bit 1 = literal /
0 = match, MSB-first bit reading with no byte alignment - every parameter
already in [`docs/formats/lzss.md`](docs/formats/lzss.md) is visible in the
PS2 code, under a different compiler and ISA. Committed as `23664bf` and a
follow-up doc amendment `a169521`.

**No live disc validation was possible** - `data/` doesn't exist in this
working tree for this agent (no image mounted). What it did instead:
transcribed a *second* Rust decoder directly from the PS2 disassembly, in the
PS2 code's own shape (resumable state machine) rather than the existing
PSP-derived decoder's shape, and diffed the two over 40,000 random streams, a
full ring wrap, and degenerate inputs - agreeing on every byte. It then
deliberately broke the reference eight different ways (length bias, flag
polarity, bit order, field order, mask reset point, cursor start,
absolute-vs-relative addressing, window size) to confirm the comparison
actually has teeth, after noticing its first sweep silently passed with a
no-op mutation.

**Confidence deliberately held at 94, not raised to 95**: the rubric's top band
needs a runtime trace *and* a second binary, and this only supplies the
second leg. Recorded explicitly so nobody "rounds up" later.

**Two new findings, neither reachable from the PSP side alone:**

1. **The PS2 decompression ring is never zeroed** - a fresh, unzeroed heap
   allocation per stream. Since ring positions are absolute rather than
   relative, a match *can* legally address a slot the stream hasn't written
   yet; the real console would read heap garbage there, our decoder returns
   zero. All 6,053 shipped streams happen to decode correctly under the
   zero assumption - so this is a fact about what the shipping encoder never
   emits, not a bug in the format reading. **A hand-built or fuzzed stream
   could tell the two apart where no real archive on either disc can.**
2. **A dead 98,316-byte encoder workspace** (`Lzss_AllocBuffers`/
   `Lzss_FreeBuffers`, allocated and freed, never read or written by anything
   else). `98,316 = 3 * (8192+1) * 4` is exactly Okumura's `lson`/`rson`/`dad`
   search-tree shape for an 8192 window - the *compressor's* structure,
   shipped compiled out of a binary that only ever decodes. Confirmed by a
   full instruction-operand scan (2 references to this workspace across
   361,812 instructions, versus 6 for the real ring, 3 of them inside
   `Lzss_Decode` itself) after a plain `get_xrefs_to` call under-reported and
   was caught rather than trusted.

10 new Ghidra symbols (58 total on `SCES_547.48` now), one below 70
(`g_lzss_encoder_tree`, 68, `_q`-suffixed). `cargo nextest -p oag-formats`:
192/192.

**Tooling note surfaced by this agent, worth the user's attention:** `sd`
(listed in the global CLAUDE.md as an available CLI tool) is **not installed**
in this environment, and whatever currently answers to that name **exits 0
while doing nothing** rather than failing loudly - it silently no-op'd this
agent's first negative-control mutation sweep and briefly made a broken test
read as passing. Worth installing `sd` for real, or removing it from the tool
list, before anyone relies on it for an edit that matters.

The PS2 Ghidra work paused here on purpose: the `0x00202d48`-`0x00203a80` XML
reader family (confidence 60, would unlock every XML-driven subsystem at once)
was flagged but deliberately not started, in favour of handing off one clean,
finished deliverable rather than a partial one.

## M1 is done: PS2 mesh batches decode too, and it's a different GPU's format

The last M1 gap - `oag-view --mesh` failing on PS2 vertex type `0x1b9` - is
closed. **The reason it looked unrecognisable is that it is a different
platform's format wearing the same bit fields.** PS2 has no GU: the vertex-type
word's position bits say `f32` instead of the PSP's hard-coded three-`s16`, and
that's not a wider PSP vertex format, it's a marker that the whole payload is a
**VIF command stream** (the PS2's DMA-driven equivalent of a GE display list) -
implemented in the new `oag_formats::vif` module and the PS2 path of
`vex::mesh_batches`, committed as `d9f14bb`.

**Confidence 94**, on invariants that can't come out by accident, checked over
**every PS2 `.vex` file**: 757 models, 98,243 batches, 11,767,670 vertices.
Three independent framing lengths (region header, DMA tag quadword count, and
where the command walk actually ends) all close exactly; every one of the 11.8
million decoded positions falls inside its own batch's bounding box with zero
slack; normals come out unit-length to `1.2e-7`; and the same model renders the
same size through both asset paths (PS2 `01_Track` radius 831.02 against the
PSP's 830.92; the Feisar ship at 6.45 on both).

Three things worth knowing before touching this code:

- **A strip split across VU1 memory repeats two vertices at the seam on
  purpose** - 44,967 of 89,302 strip batches split this way, and the repeat is
  what keeps winding parity, not a bug to dedupe.
- **Colour is a 0-128 range, not 0-255** (the GS's own convention for "full
  intensity" through the modulate path) - reading it as a normal 0-255 byte
  makes every PS2 model exactly half as bright, which reads as a lighting bug
  rather than a decode one. Confidence only 70 on this specific point, since
  nothing in the executable was actually read for it.
- **PS2 models still render untextured, and unlit too** - decoding the
  vertex/geometry format didn't touch the separate open question of where PS2
  textures live (the embedded block is empty, unchanged from before this
  session). Compounding it: every PS2 chunk happens to carry vertex colour,
  which `oag-view` already treats as "prelit, skip the light rig" for the PSP
  side - so a PS2 model comes out as a featureless white silhouette, geometry
  correct but with no visual parity to the PSP screenshot at all. **Say this
  precisely**: M1's exit criterion is met on geometry, not on appearance.
- **The PS2 disc also carries 6,080 ordinary PSP-format batches** alongside
  the VIF ones, so the PSP-vs-PS2 discriminator has to be per-batch (which it
  is), never per-disc.
- **The advisor caught a real gap before it shipped**: concatenating a split
  strip's chunks is only winding-correct if every non-final chunk has an
  *even* vertex count; the decoder now refuses odd ones outright rather than
  silently producing a wrong triangle. The disc-wide validation run (all
  98,243 batches) confirms no real file ever triggers the rejection - but the
  check stays, since nothing rules it out for a hypothetical or fuzzed file.

`docs/overview/roadmap.md`'s M1 exit criterion is updated to record this as
met on geometry, with the visual-parity gap stated explicitly rather than
implied.

## The PS2 XML reader is confirmed, and it answers two things from unrelated pages

The same agent that did the LZSS work went on to confirm the whole XML reader
family (`0x00202d48`-`0x00203a80`) - **35 more symbols, none below 70**, taking
`SCES_547.48` to 93 documented symbols total. It's a scanning parser with no
tree: a 160-byte cursor holds a begin/end pair into the loaded file buffer and
re-scans on every step, closing tags are found with a depth counter, and -
worth knowing before reimplementing anything against this data - **it edits
the buffer in place** (NUL over each closing quote, so attribute values are
bare pointers into the file) and **compares every name case-insensitively**.
Committed as `96a658a` and a follow-up correction `0cca5af`.

**Two findings land on pages this agent didn't write:**

1. **`docs/formats/fexml.md`'s 18-code attribute dictionary is now read
   straight off the parser, not inferred from data.** `Xml_OpenFile` stores a
   `code`-tagged element's attributes into an 18-slot table keyed by
   `name[0] - 'a'`, and two independent constants (`c - 'a' < 0x12` at both
   expansion sites, and `0x258 - 0x210 = 72 = 18 * 4` from the constructor)
   confirm the count the format page had only ever gotten by counting codes
   across `FEData.wad`. Also answers one of that page's open questions: PS2
   uses the same schema.
2. **The default float accessor is exponent-blind, and every handling
   parameter goes through it.** `Xml_AttributeAsFloat` skips any character
   that isn't a digit, `.` or `-` rather than stopping at it, so `1e-5` parses
   as `-15`. No shipped value uses exponent notation, so nothing is wrong
   today - but a tool that regenerates `HandlingStats.xml` must never emit
   one. Flagged on both `handling-xml.md` and `fexml.md`. This is PS2-only
   evidence; the PSP reader isn't located yet, so it doesn't transfer without
   checking.

Also fixed, from the same agent's own earlier work: `handling-xml.md` had
called two of `HandlingXml_ParseGlobal`'s five arrays "per-class scalars" when
they're actually `char*` strings. And it caught its own slip - it first wrote
"no PSP XML reader is documented" from a case-sensitivity mistake in its own
grep; `psp-pulse/frontend-video.md` already documents `Movie_ParseAttributes`
as a *consumer* of that reader (not the generic parser itself), which is now
the obvious anchor for locating the real one.

**Next targets it flagged itself, in the order it would pick them:** the PSP
XML reader (cheap now, and settles whether the exponent-blindness finding
applies to both builds); an `<FEGlobals>`/`<FEConst>` indirection at
`0x0017c630` that lets front-end XML reference a symbolic name where a number
is expected, not in `fexml.md` at all yet; and **the PS2 `Ship_Update*` force
terms** - directly relevant to the physics investigation elsewhere in this
file, since the handling *parameter block* is now confirmed identical in
layout, so a second reading of the force law could settle open questions
there (the roll-oscillator constants among them) the same way LZSS and the
handling loader just got settled.

## The roll-oscillator margin is now settled at instruction level, in a second binary - and the crate was already right

The same agent took its own suggestion and read the PS2 build's craft-update
and rigid-body path (`docs/ghidra/functions/ps2-pulse/craft-update.md`,
13 symbols, nothing below 78). **This agent hit a context limit
("Prompt is too long") right after finishing its work and could not send its
own final report** - its last edit landed uncommitted, was recovered and
committed on its behalf (`941a4ee` plus a follow-up), and is folded in here
instead of in its own words. Anyone picking up further PS2 Ghidra work should
start a fresh agent rather than trying to resume this one.

Three things settled, one deepened:

1. **The surface-alignment gain really is `-400.0`, confirmed at instruction
   level in both binaries.** `Ship_HoverFourCorner`/`Ship_HoverTwoPoint`
   materialise `0xC3C80000` as an immediate on PS2, same magnitude, same sign,
   same operand order as PSP. `docs/physics/README.md`'s standing suspicion
   that this was a transcription error is retired: two different compilers on
   two different ISAs producing the identical constant is not a coincidence.
2. **Roll damping is `-2.0`, confirmed exactly - but only outside "mode 0"**,
   which uses `-5.0` instead (`craft+0x2d4` selects; yaw damping is a hard
   `-5.0` always, pitch damping is per-ship). Whatever repro is used for this
   instability needs its ship in the right mode for `c = 2` to be the number
   that applies.
3. **Sub-stepping cannot rescue this instability, confirmed from the PS2's own
   `Body_Integrate` disassembly, not just inferred from the Rust integrator's
   contract**: the force/torque accumulators are filled once per frame and read
   *inside* the sub-step loop with nothing recomputing them between
   iterations - the original does this too, it isn't a simplification this
   project's reimplementation introduced. Redone properly (accounting for the
   real n-substep position/velocity update rather than a naive single Euler
   step), the required damping is `c >= k*H*(n+1)/(2n)`:

   | Sub-steps | `c` required | Against the actual `c = 2` |
   | ---: | ---: | ---: |
   | 1 | 6.67 | 3.33x short |
   | 3 | **4.44** | **2.22x short** |
   | infinite | 3.33 | 1.67x short |

   **The `n = 3` row is exactly what `crates/physics/src/hover.rs`'s own
   stability note already computes.** So the crate's math was already right;
   `docs/physics/README.md`'s older `h <= c/k` reading (which assumes forces
   are re-evaluated every sub-step, giving a falsely-reassuring 1.11x) is
   the thing that needs correcting, not the other way round. **This margin is
   real, it is confirmed in a second binary at instruction level, and it
   cannot be fixed by raising the sub-step count or by any of the constants
   just confirmed exact** - if a fix is needed, it has to come from somewhere
   this reading hasn't reached yet.
4. **The handedness question deepened rather than closed.** `Body_AddForceAtPoint`
   computes torque as `F x r`, not the physically-correct `r x F` - confirmed
   in raw VU assembly, on the one term whose correct sign is fixed by
   mechanics rather than by anyone's prose. That makes the anomaly
   **systematic across at least two terms**, corroborating `engine.md`'s
   standing "the sign lives downstream" hypothesis over "someone mistyped it"
   - but where it's compensated is still unlocated: `Body_AddTorqueWorld` and
   `Body_Integrate`'s rotation update are both plain `+=` with no negation on
   the path read so far. **A follow-up pass has since settled the operand
   convention against the Sony *VU User's Manual* v6.0 and a raw-encoding
   decode of the four instructions in question, confirming Ghidra prints them
   in the manual's mnemonic order rather than reversed** - so "maybe
   `vopmsub`'s convention is simply the reverse of what was assumed, and all
   three terms flip together" is **refuted**, not a live explanation. Two
   candidates remain, and this page picks neither: the compensation is in the
   unread inertia/rotation-matrix path at `body+0xc0`..`+0xf0`, or the force
   is already pre-negated before this call, uninspected component-by-component.
   See `docs/ghidra/functions/ps2-pulse/craft-update.md`'s "cross-product
   convention" section for the manual citation and the encoding table - a
   fresh agent picking this up should start from the `+0xc0` matrix rather
   than re-checking the operand convention.

One more unresolved difference, not yet chased: the PS2's four-corner
bank-to-yaw term uses `50.0` where the PSP uses `30`; neither value is
runtime-verified yet.

## The torque-sign mystery is closed: nothing was ever inverted

A fresh PS2 Ghidra agent (the previous instance died from context overflow
right after resolving the operand-convention question above) took the two
remaining candidates for where the `F x r`/`-400 * (up x n)` sign anomaly gets
compensated, and **both are dead, because there is nothing to compensate.**

**The engine integrates its rigid-body basis as `e' = e x w`, not the textbook
`e' = w x e`.** Read directly out of `Body_Integrate`'s skew-matrix
construction, whose three base constants were confirmed all-zero by reading
them out of memory: the built update gives
`d(row0) = h * (-z*row1 + y*row2)`, the negative of what `w x e0` would
produce. That means **`w_game = -w_physics`** - the engine's angular velocity
is the negative of the textbook pseudovector, consistently, throughout the
whole rigid-body formulation. Substitute that in and every apparent inversion
disappears: `tau = F x r` becomes exactly the textbook `r x F`, and
`-400 * (up x avgNormal)` becomes exactly the aligning `+400 * (up x n)` the
prose always said it was. **`docs/physics/README.md`'s prose and the
arithmetic agree after all - what was missing was the convention, not a
sign.**

Both remaining candidates from before are directly ruled out, not just
deprioritised: the hover spring's force argument at its call site is a plain
**positive** multiple of `up`, no negation anywhere in it; and the matrix at
`body+0xc0`..`+0xf0` is the basis's rotation inverse (built as the adjugate
scaled by `1/det`), which for an orthonormal basis is just the transpose -
determinant `+1`, structurally incapable of flipping a sign.

**This is explicitly not the handedness question**, and is independent of it.
The PS2's own basis orthonormaliser rebuilds `row0 = row1 x row2`, which is
the same statement as `engine.md`'s runtime-measured
`cross(row0, row1) = row2` on 200/200 PSP ticks - both agree the basis is
positively oriented under ordinary component-wise cross products. Handedness
was never the variable; the sign of `w` was, and the two had been conflated.

**Two independent legs support this**, one of which needs no assumption from
the other at all: the instruction-level integrator read, and separately,
`engine.md`'s own runtime steering measurement (accumulator sign in, observed
rotation sign out, bypassing every intermediate transform) and its weathervane
term (`cross(forward, velocity) * -0.1`, stated as turning the nose toward
travel - backwards under the textbook convention, correct under this one).
Confidence **88** in the PS2 build (raw disassembly, a vendor ISA manual,
memory-read constants, corroborated by a measurement on the *other* binary);
confidence **80** that the PSP build shares the convention, since the PSP's
own integrator has never been located.

**The actionable rule for a reimplementation, stated plainly by the finding
itself: apply exactly one flip, consistently.** Either keep the textbook
`e' = w x e` and negate every torque source read from the disassembly, or keep
the disassembly's torque expressions verbatim and integrate with `e' = e x w`.
**Doing both, or neither, is not a tuning problem - it's unconditional
divergence for the terms affected**, a different and much worse failure mode
than the mild growth-type instability the physics investigation elsewhere in
this file has been measuring. Since that investigation's own repro shows
growth rather than immediate blow-up, that's independent evidence its current
sign convention is already internally consistent - flagged to it as
reassuring context rather than a required change. Two things to note
regardless: **angular damping (`tau = -c*w`) is invariant under either
convention** and must not be "corrected"; and `Body_AddForceAtPoint`'s actual
argument order is `(body, force, point)` - its prologue's register shuffle
disguises this, and an earlier reading had force and point's roles described
backwards (never a behavioural bug, just a documentation one).

## `oag-trace` got its first real comparison, and it changes where the floor-bug fix should look

The agent that built `oag-trace` circled back to attempt a real capture, as
suggested once niri floated PPSSPP's window and the save profile was confirmed
present. **It got one, and it also hit a context limit ("Prompt is too long")
right after committing its work** - same failure mode as the PS2 Ghidra agent
above, and again the work itself landed cleanly (`75fc4dc`) before the process
died; there was nothing left uncommitted to recover this time.

**There is now a documented, reusable reference scenario**: Time Trial, Venom,
Talon's Junction White, Assegai - one ship, no weapons, no AI, 5,178 m, with the
exact menu key sequence from a cold boot recorded in
[`ppsspp-debugger.md`](docs/reverse-engineering/ppsspp-debugger.md#the-reference-scenario).
This is the scenario to reuse for anything that needs to compare against the
original, including the user's stated goal of eventually racing a full race on
both the original and the reimplementation on the same track.

**The first real trace-vs-simulation comparison found the ship falls through
the floor at the first step, at the original's own recorded starting
position** - not after driving for a while. `grounded` reads `1.0` on all 200
real ticks; the simulation's reads `0.0` on every tick but the seeded first
one. Everything downstream follows from that single fact: velocity is out at
tick 1 (max 211 units/s against a recorded 22.7), orientation.forward at tick
1 (max 1.52 rad), and position at tick 3, whose error keeps growing to a max
of 424 units by the last recorded tick (199) - not 423 units by tick 3, which
conflates when the field first crosses tolerance with its eventual maximum
and overstates the symptom's early severity. The control columns
(throttle/brake/steer/airbrakes) all match exactly, so input and scaling are
not the problem - **the entire divergence is the force law failing to find
ground contact where the original does, immediately.** This is a different,
and likely more primary, symptom than the roll-oscillator instability the
physics investigation elsewhere in this file has been focused on - that was
about a ship that eventually tumbles once already grounded; this is about a
ship never grounding at all from a correctly-seeded position. It also gives
the earlier "hover target height exceeds probe cast length" hypothesis its
first real numeric backing, rather than a static reading. Relayed directly to
the physics investigation.

**Two free findings from the same capture, one of which needed a correction:**

- **Handedness settled independently, by a method that needs no simulation at
  all.** The recorded ship travels along `+row 2`
  (`dot(velocity_hat, forward) = +0.997` every tick) and
  `cross(row0, up) = forward` holds 200/200 - so row 0 is confirmed as the
  **left** axis, corroborating the existing 84-confidence roadmap finding
  (which came from the sign of the yaw response to a held steer) by a
  completely independent route. Running the comparison the other way
  (`--basis right-up-back`) is worse on every field, which is the switch
  doing its job.
- **`speed_cached` is not the velocity's length - it is the previous tick's
  forward-projected speed, `dot(velocity, forward)`.** An earlier draft of
  this section called it "the length, one tick stale"; that was wrong, and
  worse, not even a new finding: `docs/ghidra/functions/psp-pulse/engine.md`
  already established the projection at confidence 95, and the error was a
  transcription slip into `psp-trace.py`'s own comment that briefly
  contradicted the project's own page. The capture corroborates `engine.md`
  rather than adding to it - residual mean `3.3e-6` against a speed of 24,
  indistinguishable from exact rather than merely inside tolerance - and
  further pins that the cache is written from **both** vectors' previous-tick
  values (not a mid-update mix): `dot(vel(t-1), fwd(t-1))` matches to `3.3e-6`
  against `1.3e-3`/`1.6e-2` for the two mixed-tick combinations. Scored **90**,
  not 95: the reference scenario is a straight, where the projection and the
  raw magnitude differ by only the 0.3% a `0.997` heading cosine allows, so a
  cornering capture (now on `oag-trace.md`'s "not yet" list) is what would
  separate the two readings for real. The live `speed` field (`+0x398`,
  distinct from `speed_cached`) still matches neither reading exactly (3.67%
  above the length, 4.02% above the projection) and stays an open,
  low-priority recording-model mismatch - not a physics bug, and **it now
  leads the comparison's first-divergence line at tick 0** purely because
  it's compared against the seeded row; read past that line to the tick-1
  group above for the actual physics finding.

## Eight of engine.md's force-law terms now have a second binary, before the agent stalled

Before going quiet without sending its own final report (stopped after
repeated idle checks with no reply - not a context-overflow crash like its
predecessors, just silence), the same agent that resolved the torque-sign
question kept going and identified **9 of `Ship_UpdateCraft`'s 16 PS2
callees**, checking each against `docs/ghidra/functions/psp-pulse/engine.md`
term by term. All the substance is already committed (`14f895d`, `5b3230b`,
`8596e07`, `8590e43`); this is recording it since the agent itself didn't get
to.

**Six terms reproduce in full** - engine, steering, pitch, lateral grip,
weathervane torque, and the still-unnamed track-section force - and two more
mostly do (quadratic drag, three of four coefficients; gravity, structure
only, two details differ). Each of these moves from the decompilation-only
84 cap to **88**, per the rubric's "second binary" leg. Three specific weak
points on `engine.md` got stronger evidence, not just corroboration:

- **The dead `Engine.gain`/`falloff` ramp** - flagged as the kind of thing an
  obvious reimplementation gets wrong - reproduces exactly: the PS2 also ramps,
  clamps, then throws the result away and recomputes thrust straight from the
  input. `85 -> 90`.
- **`slidegrip` as "percent of grip retained"** - previously an interpretation
  of a load-time scale factor - is a **literal expression** in the PS2 build:
  `airbrake * (0.01 - slidegrip) - 1.0`. `90 -> 93`.
- **The PSP's odd `S221` register-carry pattern**, previously scored 65 with a
  recommendation to read it as `accumulator.y = lift`: the PS2 builds an
  ordinary two-lane thrust/lift vector, which doesn't explain the PSP's
  codegen but does confirm the intended semantics. The recommendation moves to
  85; the PSP-specific explanation stays at 65.

**Four real cross-platform differences, none of them corrections to
`engine.md` - each needs the PSP side re-read before anyone decides which
build is the odd one out:**

- The PS2's drag term has **no `-0.9` mode-enum branch** at all.
- The PS2's gravity scale sits on **`flight_gravity`**, not `normal_gravity`
  as `engine.md` currently states for that term.
- **PS2 gravity is scaled by `(1 - magLockBlend)`** - `engine.md` records that
  factor for vertical damping and bank-to-yaw but not for gravity. Taken
  literally, a fully mag-locked PS2 ship has no gravity at all.
- **PS2 lateral grip multiplies both grip coefficients by `1.5`** under the
  same flag bit that gates the turbo add - not recorded on the PSP side.

Two structural notes worth knowing before porting any layout: the PS2's
controls pointer lives at `craft+0x98`, not `+0x78` - same member offsets
inside it, the struct just moved - and **the PS2 has no craft-side force
accumulators at all**: `Body_ClearAccumulators` zeroes the body's four
accumulators directly and every term writes the body, where the PSP keeps a
separate craft-side copy. Same four accumulators, one less layer of
indirection.

**Still not corroborated, and the concrete next step for whoever picks this
back up**: brakes, airbrakes, rolling resistance, and vertical damping remain
single-source at 84; and the fifteen-term *ordering* itself - which is what
the frame-stale-groundedness finding depends on - was never checked, since
every PS2 identification above came from matching callees rather than reading
the call sequence.

## The PSP has the same exponent-blind float parser - confirmed, not assumed

A fresh agent located the PSP's generic XML reader (`docs/ghidra/functions/psp-pulse/xml-reader.md`,
new page, anchored from `Movie_ParseAttributes`'s callees the same way the PS2
page anchored from `Handling_ParseStats`) and settled the one open
cross-platform question the PS2 work left behind: **`Xml_AttributeAsFloat`
(`0x0895379c`) is the identical hand-rolled, exponent-blind parser on PSP
too** - every character that isn't a digit, `.` or `-` is ignored rather than
ending the scan, so `1e-5` parses as `-15`. Confidence **95**: unambiguous
decompilation, a second binary agreeing instruction-for-instruction, and every
consumer traced rather than assumed - which is every handling parser
(`ParsePhysical`/`ParseAntigrav`/`ParseEngine`/etc.) and all five camera
parsers. `docs/formats/handling-stats.md` and `docs/formats/fexml.md`'s "no
scientific notation" caveat is no longer PS2-only evidence.

**The same split exists on both builds**: a second, correct accessor
(`Xml_AttributeAsFloatLibc`, a real `atof`/`strtod` wrapper) exists alongside
the blind one, used exclusively by the front-end widget layer while every
gameplay parser goes through the blind one. Neither Ghidra function existed
before this pass - both were only visible in raw disassembly.

## PS2 textures found, decoded, and rendering - M1's last real gap is closed

A fresh agent (replacing the one that decoded PS2 mesh geometry, after it hit
the same context-overflow failure noted above) went looking for where PS2
textures actually live, since the embedded block in a PS2 `.vex` file is
always length zero. **They are standalone WAD entries, never embedded**, and
a model's set is additionally bundled into a small nested WAD (same 8-byte
header, 16-byte entries as the outer archive) keyed by a hash that has not
been recovered - so `oag-view` takes the entry explicitly for now rather than
finding it automatically. Confidence **94**, on invariants checked across all
**5,348** real PS2 textures across both WADs: a dimension declared twice (a
packed log2 byte and explicit width/height words) agreeing on all 5,348;
both GS `GIFtag`s declaring their own payload size in a way that has to match
the dimensions' implied byte count, also 5,348 of 5,348; and the total blob
size closing by formula on 5,348 of 5,350 candidates.

**The format is a GS upload DMA packet, not a texture file** - `TRXPOS`/
`TRXREG`/`TRXDIR` register writes framing two GIF transfers (texels, then
palette), which is real PS2 hardware behaviour rather than anything specific
to this game. Two findings worth knowing if this code is ever touched:

- **The dimension words are stored height-first**, which only shows up because
  `TRXREG` is a fixed, asymmetric function of the true dimensions - reading
  width-first decodes every *square* texture correctly (most of them) and
  scrambles every other one silently.
- **Most textures (5,077 of 5,348) are 8-bit-indexed data pre-swizzled into
  PSMT8 order so it can be uploaded as PSMCT32**, the standard PS2 trick for a
  format the GS has no fast host-upload path for. The swizzle permutation is
  only well-defined for widths of 16 and up, and **the widths where it breaks
  down are exactly the widths the game switches to a direct linear transfer
  for (4 and 8)** - the arithmetic predicting the same fallback set the data
  actually uses is a corroboration, not a coincidence. The palette is in
  `CSM1` order (a bit-swapped index within each group of 32) and alpha runs
  0-128 rather than 0-255, the same GS convention already found for PS2 vertex
  colour.

**It renders.** `oag-view --mesh 'Data\Ships\Feisar\Ship.vex' --textures
0xfc3f75cf` against `WADS2.WAD` draws the Feisar ship in its actual blue and
yellow livery, team name legible, UVs landing on the right panels - not just
"doesn't crash," but recognisable, correctly-oriented artwork. `docs/overview/roadmap.md`'s
M1 exit criterion is updated from "met on geometry" to **met**, full stop, on
both platforms.

**Not determined, and not blocking**: the PSMT4 swizzle (5 of 5,348 textures,
refused rather than guessed at); what the two unexplained fields at `+0x08`/
`+0x0c` are (look like a GS memory allocation the file has no reason to fix,
not needed to decode); and the texture-set lookup mechanism above. Also
recorded as a live discrepancy: a PS2 `.vex`'s own `Texture` node payloads are
stale PSP-shaped leftovers (mip counts, sizes) that disagree with the actual
loaded texture entirely - the Feisar node claims 512x512 8bpp with 6 mips,
the real entry is 256x256 with one level and the PS2 blobs carry no mips at
all.

Also landed in the same commit: `scripts/psp-trace.py` gained `--ship`/`--craft`
selection, because a race breakpoints on `Ship_UpdateCraft` once per ship per
tick with a different craft each time - the previous capture logic stopped at
the second address, which is fine for a one-ship Time Trial but makes a race
capture meaningless without picking one craft to follow. Found in the process:
**the player's craft is the one whose throttle is exactly 0 or 100** (digital
input), where AI craft read fractional throttles; and only four of eight craft
were updated per tick in one race capture, for a reason not yet established.

## `oag-trace` exists now: M3's missing comparison side is built

A new crate, `crates/trace` (`oag-trace`, lib + bin, added to the workspace),
closes the gap [the roadmap](docs/overview/roadmap.md) and this file both
flagged as the top priority: **the instrument that reads a trace and a
simulation run and reports where they diverge, not whether they match.** 38
tests, all against hand-authored fixtures (real traces are derived game data
under `data/traces/`, gitignored, so nothing here can run in CI - see
[the tool's page](docs/tools/oag-trace.md) for why). Clippy, fmt, rustdoc and
`just check-docs` all clean; the full workspace gate was not run, deliberately,
per this pass's rule about not running `just` while other agents have crates
open.

`oag-trace show|run|compare`: `show` summarises a capture (tick count, dt
distribution in Hz, distance travelled - which is exactly the false-start
stall trap from earlier this session - and how well `cross(row0, up) == forward`
holds); `run` seeds a `World`/`Ship` from a recording's first row and replays it
through `oag_physics::step`/`oag_gameplay::ship_controls`, sampling **before**
each step to match where `psp-trace.py`'s breakpoint actually sits (craft
entry); `compare` diffs two traces on the 13 fields
[the protocol](docs/reverse-engineering/verification-protocol.md) specifies,
with the right tolerance per field (position 0.01 abs OR 1e-4 rel, orientation
1e-4 rad, velocity 1e-3 rel, `grounded` exact), and reports **the first
divergent tick, the field, the magnitude, everything else that moved on that
same tick, and a trend** - `exact`/`bounded`/`growing`/`shrinking` from a
least-squares slope plus a first-quarter-vs-last-quarter mean comparison - so a
bug that is small-but-growing doesn't hide behind a tolerance that only checks
magnitude.

**One thing worth knowing before trusting a `compare` run: a naive
tolerance check would have called a `NaN` a perfect match on every field**,
since every comparison against a non-finite value would read as "within
tolerance" by omission. The advisor that reviewed this crate's design caught
it; non-finite is now always reported as a divergence, never silently accepted.

**A sharper version of the handedness finding**, tightened while building the
replay path: the recording's own right/up/forward rows are a positively
oriented basis, while `Body`'s convention is not (`forward` is `-Z`), so the
mapping between them needs an **odd** number of sign flips, not one chosen
arbitrarily - flipping only row 0, as a naive port might, is a reflection that
`Quat::from_mat3` will silently turn into nonsense rather than error on.
`oag-trace run --basis left-up-forward` (measured, and the default) versus
`--basis right-up-back` exist as two concrete, opposing hypotheses; running
both against a real capture is a one-command way to settle
[the 84-confidence cross-product claim](docs/ghidra/functions/psp-pulse/engine.md)
that this session's earlier physics work left open. Nobody has run that
comparison yet - it needs a real capture, see below.

**No real emulator capture was attempted, on purpose, not because of a missing
piece.** `PPSSPPSDL`, a live display, and `data/images/pulse-psp-usa.chd` are
all present and usable; capturing one needs an interactive GUI session driving
Pulse's own menus on the user's actual desktop mid-race (and the SDL build
throttles when its window loses focus), which is a different kind of task from
everything else this pass did unattended. Doing that capture and then running
`oag-trace run`/`compare` against it is now genuinely one session's work away,
and it is the highest-leverage thing to do next: it would settle the handedness
question above and give the M4 physics work (see the fall-through-floor
investigation elsewhere in this file) its first real measurement instead of a
static reading.

**What the capture format still can't see, documented rather than faked**:
angular velocity, pitch, the landing/leap timer, and raw stick position aren't
columns `psp-trace.py` records, so `compare` can't diff them; a dropped tick is
invisible too, because the capture numbers rows by counting breakpoint hits,
not by reading a tick counter. There's no committed input script or save state
yet either, so every scenario run so far has been ad hoc rather than one of
[the protocol's seven](docs/reverse-engineering/verification-protocol.md).

**One coupling to know about before changing force-law code elsewhere**: the
crate's own coast-down self-test assumes rolling resistance and quadratic drag
are *forces*, using a large test mass to divide their contribution down to
near-zero. If a future physics change turns either term into an acceleration
instead, that test will start failing - correctly, as a signal that the
contract changed, not as a harness bug to silence.

`data/cache/pulse-psp-usa.iso` (457 MB) is the raw ISO the emulators want,
extracted with `just extract-iso`. It is gitignored twice over and it is a real
4 GB-class copy on `ecryptfs`, so keep it rather than re-extracting.

## The original is now scriptable, and it corrected four things

[`docs/reverse-engineering/ppsspp-debugger.md`](docs/reverse-engineering/ppsspp-debugger.md)
is the new load-bearing page. PPSSPP's websocket debugger gives breakpoints,
memory, registers, disassembly, threads and **scripted input**, and
`scripts/ppsspp_debugger.py` wraps it with the traps encoded rather than
described. `scripts/psp-trace.py` captures a per-tick CSV off a running race
(`just trace --ticks 300 --hold cross --warmup 4`).

**Four traps, each of which cost real time.** They are on that page in full, but
the two that will bite hardest:

1. **A breakpoint added while the CPU is running never fires.** It is accepted,
   it appears in `cpu.breakpoint.list`, and nothing happens - under the JIT and
   under `--ir` alike. Break first, then add, then resume.
2. **Changing the breakpoint list rebroadcasts `cpu.stepping` with the old pc**,
   which a naive client reads as an instant hit at 0.00 s. Three rounds of tests
   reported phantom hits before this was clear. Match on the pc.

And one that decides the architecture: **a memory read costs ~520 ms while the
CPU runs and ~0.3 ms while it steps.** Per-tick capture must be breakpoint-driven
with one batched read per hit. Polling a running CPU at 60 Hz is not slow, it is
impossible.

What that instrument then found:

- **`Ship_UpdateCraft`'s call site**, which engine.md's confidence was capped on
  and the roadmap listed as open: `0x0884ff70`, a virtual call through slot
  `0x70` of the vtable at `object+0x38`, inside a per-entity update loop at
  `0x0884f70c`. The enclosing function is deliberately **not renamed**.
- **Its register assignment was wrong.** The craft is `a0` and the body is `a1`,
  not `a1`/`a2`: `a0+0x1cc` holds exactly the value in `a1`, and the craft's `dt`,
  `grounded` and throttle all read sensibly off `a0` and as garbage off `a1`.
  Anything written against the old signature reads the wrong structure.
- **Delta time is variable, and its mean is `1/59.94`.** 72 distinct values over
  120 ticks, mean and median `0.016683`. That confirms ADR-0007's premise and the
  vblank-locked presentation finding in one measurement.
- **The three fixed-timestep globals are still zero after real racing**, closing
  frame-pacing.md's "a write through an unresolved pointer cannot be excluded"
  hedge as far as it can be closed. 88 -> 93.
- **`0x08b31784` is a pointer to the state machine, not the machine.** The state
  name at `+0x18c` is an inline buffer, proved by catching `"Show Logo\0election"`
  - a short name overwriting `"Language Selection"` in place. Reading it is a
  text-mode view of the front end and is how the menus were navigated.

### The false start, which reads exactly like a bug and is not

Most of the captures this pass were of a **stalled** ship, and the trap is worth
knowing before it costs someone else an hour. Pulse penalises a false start by
flooding the engine, and holding thrust through the countdown is what a script
does by default. The symptom: the throttle field reads the full 100 you are
sending, the cached speed climbs to 10 and resets, and the ship does not move at
all. That reads like a broken input path or a wrong position offset. It is
neither.

The cure is a restart with nothing held: `start`, `down` x4, `cross` for RESTART
RACE, then ~25 seconds of no input at all, then capture with thrust. That
produced the trace this pass measured from - 200 ticks, 73.85 units travelled,
speed steady around 22 - and it is what settled the cached-speed staleness
199/199. The scenario is Time Trial, Venom, Talon's Junction White, Assegai, and
the profile save now persists, so the first-boot walk never has to be repeated.

M4 is open and partly built. What exists now that did not this morning:

| Crate | What it is |
| --- | --- |
| `oag-physics` | Ship dynamics and collision queries. Depends on `oag-core` **only**. |
| `oag-gameplay` | `World`, `Ship`, `InputSnapshot`, spline spawning, and the two bridges from the on-disc schemas onto the simulation's types. |
| `oag-render` | The wgpu renderer, extracted out of `oag-view` so `oag-game` can use it. Owns no window. |
| `oag-input` | Devices onto the abstract button layer and the input snapshot. |

Plus `oag_formats::collision` and `oag_formats::handling`, and
[`docs/ghidra/functions/psp-pulse/engine.md`](docs/ghidra/functions/psp-pulse/engine.md),
which is the single most load-bearing new document in the tree.

Two modules **moved**, and the reason is worth keeping: `input.rs` went from
`oag-game` to `oag-gameplay` because the simulation owns the snapshot type, and
`keys.rs` went to `oag-input` because a device mapping is not a game concern.
Both are re-exported from `oag_game` so the front end's call sites read as they
did.

## There is a ship on a track now, and it cannot finish a lap

```sh
just play --race                       # arrows steer, X/Return thrust, Q/E airbrakes
just play --race --art --screenshot /tmp/race.png --ticks 45 --hold cross
just play --race --dry-run             # report what came off the disc, then exit
```

`oag-game --race` loads a track and a ship off the disc, spawns the ship on the
racing line, steps the simulation at a fixed 60 Hz from the keyboard and draws it
from the chase camera the ship's own data describes. `--art` draws the track's art
meshes instead of the diagnostic ribbon; the ship sits in the same place in both,
which demonstrates spline, collision and render geometry share one world space.

**What works.** The ship spawns the right way up on the racing line, the probes
find the collision soup, and thrust drives it forward along its own forward axis.
Over 120 ticks it is grounded on 83, peaks at 79.9 speed, and stays within 18.7 of
the spline against a 66.4 envelope. All 203 sampled downward probes along the
whole spline find geometry.

**What does not.** At about 100 ticks the ship loses its last probe contact,
pitches, is thrown clear, and is outside the envelope by tick 166. This is the
force law, not the composition, and three diagnostics say so: it reproduces on a
**flat infinite floor** with no track, spline or camera involved; it is **stable at
1/240 s**; and it is **stable at 60 Hz given a textbook box inertia** from
`<Misc>`. That is the `h <= c/k` arithmetic below, showing up exactly where it
predicted. **Nothing was tuned to hide it**, and
`the_ship_does_not_stay_on_the_track_yet` records it as a test whose failure
message says to delete it.

Three things it found that live in crates the race work did not own, all still
open:

1. **`ChaseParams::pos_length` is stored negative** in every
   `<ExternalCameraFar/Close>` on the disc, while `oag_render::camera::chase`
   documents it as a positive distance behind and negates it again - which puts
   the eye in front of the ship. `race.rs` negates at the adapter boundary,
   confidence 80. The convention probably belongs in `oag-render`. This is exactly
   the "cheap to check once there is a ship to look at" test that module wrote
   down for itself.

   **Settled since, against the original running** - see
   [camera.md](docs/ghidra/functions/psp-pulse/camera.md). The disc's
   `pos_length` is a **signed offset along the ship's forward axis** (negative =
   behind), on the same convention as `pos_height` and `lookat_length`, so
   `eye = position + forward * pos_length + up * pos_height` uses the file's
   value unchanged. Measured off a breakpoint in `Camera_UpdatePlayerView`:
   the eye's projection on the ship's forward row is **negative** for both
   external views and **positive** for the internal one, matching
   `<InternalCamera length>` exactly. So the current behaviour is right and the
   two negations must not both be removed; what is wrong is only `chase.rs`'s
   prose. **The Rust was deliberately not changed.** The minimal tidy-up is one
   sign flip in `oag_render::camera::chase::anchor` plus deleting
   `race::chase_pos_length`, with no behavioural difference; do it as one commit
   and check a screenshot is byte-identical across it.

   One new open question came out of the same measurement: the original puts the
   eye at exactly **3/4** of the authored external offset, at rest and at speed
   alike (seven samples over 541 world units, ratio `0.7500` every time), and
   **where that factor comes from was not found**. Do not hardcode it.
2. **`hover::target_height` exceeds `<Antigrav ride_height>`, which is also the
   probe cast length.** So there is no height at which the spring rests *and* the
   probes still see ground, and the oscillation is structural under the current
   reading. The culprit is the additive `antigrav_height_adjust`, which
   `oag_physics::hover` already flags as a confidence-50 guess with nothing found
   that writes it. Zeroing it makes target and reach equal. One line, but it is a
   physics decision and wants evidence, not a patch.
3. **Single-probe contact is the norm**: both probes touch on only 6 of 120 ticks,
   one on 83. An off-centre force every tick is a pitch torque every tick, which
   the missing inertia tensor (`Body::inertia` is still `(1,1,1)`) then amplifies.

Findings 2 and 3 together are probably the whole of why it cannot fly a lap, and
both are cheap to test. Start there before touching the integrator.

## `just play` now ends in a race, and getting there settled three conventions

The front end and the race were two disjoint entry paths chosen by `--race`, each
with its own window, event loop and wgpu device. They are one now: `oag-game` opens
one window, and the front end reaching **`Launch Game`** - the state picking a
language already fired - loads the track and hands the window over. `--race` is the
same race entered without booting the front end. There is no menu in between,
because there is no menu: the root XML's `MainMenu_Definition.xml` is unbuilt, so
`Launch Game` starts one race on whatever `--track/--team/--class` name.

Verified end to end three ways: a headless capture that follows the handoff
(`just play --screenshot ... --until "Launch Game" --press start,cross --ticks 60`),
a `#[ignore]`d ground-truth test that boots off the disc and flies what `Launch Game`
starts, and the real window driven by a synthetic keyboard through `/dev/uinput`
under niri, with the injection guarded on the compositor reporting the game window
focused. That last technique is worth keeping: it is the only way anything in
`main.rs` has ever been exercised.

**Three conventions were settled at the boundary, and one hypothesis was closed.**

1. **A `.vex` ship is authored nose along `+Z`** while `Body::forward` is `-Z`, so
   the ship was drawn facing backwards and the chase camera - correctly behind the
   body - showed it head-on. `race::MODEL_YAW` composes the half turn in
   `ship_model_matrix`, at the body-to-model boundary and **not** in
   `oag_render::mesh`, so `oag-view --mesh` still shows an asset in its own space.
   Confidence 85, measured: sliced along `z`, the hull is 5.3 units across at `-Z`
   with the model's full height and 1,175 of its 1,334 vertices, and tapers to 0.9
   across at `+Z`.
2. **The hypothesis that follows from it is false, and this is the useful part.**
   If any body-space offset from disc data were signed along `z`, the same flip
   would apply to it - which would have been a candidate explanation for finding 3
   above, single-probe contact. It is not: `oag_physics::hover::probe_offsets` is
   `[-half, +half]` about the centre from an **unsigned** `<Misc length>`, and a
   symmetric pair is invariant under a half turn. Grepping `oag-physics` and
   `oag-gameplay` for any other body-space offset built from handling data finds
   none. So the `+Z` finding has no consumer in the simulation today and explains
   nothing about the wobble. **Do not go looking for one.**
3. **The field of view is authored for one viewport shape and only one.** The PSP
   renders 480x272 and nothing else, so what a differently shaped window should show
   is a presentation choice, made in `oag_render::camera::fit_vertical_fov`: at the
   authored aspect and wider, the value is used unchanged; narrower, the vertical
   field opens up to hold the authored horizontal field. Without it a tiling
   compositor's portrait slot crops the track edges away and the camera reads as
   broken when the data is fine. It carries no confidence score because it is not a
   reading of the game, and it is constrained by a check rather than by trust: a
   capture at 1440x816 is **byte-identical** across the change. The window also asks
   for a fixed size now, which is what makes niri float it rather than tile it.

`--size WxH` renders a capture at any shape, which is how the above is checked with
no compositor involved.

**Still not wired.** No menu, no HUD, no lap counting, no restart and no pause: the
only way out of a race is Escape. The player route still draws the **diagnostic
ribbon** by default rather than the track's art meshes, and `--art` is
transformatively better to look at - that is a deliberate default from when `--race`
was a developer tool, and it is worth reconsidering now that `just play` leads here.

## The engine force law is recovered, and it corrected four things

M4 was blocked on something nobody had written down:
[`docs/physics/README.md`](docs/physics/README.md) specifies the air cushion, the
airbrakes, grip, drag and magstrips in detail, and says **nothing** about how
`<Engine>`, `<Brakes>`, `<Turning>` and `<pitch>` become force. A ship built from
that page hovers and slides but cannot drive.

That is now recovered from `BOOT.BIN` into
[engine.md](docs/ghidra/functions/psp-pulse/engine.md): the craft update frame
(`Ship_UpdateCraft`, `0x08849618`), every control term, the passive terms, and
**all 32 handling parameters placed** with the block's byte accounting closing
exactly - `6+3+5+3+7+4+4 = 32` floats = `0x80` = the observed per-class stride,
no gap and no overlap. Read from the XML loader itself, so name-to-offset is a
direct read rather than inference. 26 renames, nothing below confidence 70.

Four of its findings corrected work that had already been written:

1. **Four parameters are pre-scaled by the loader** (`Engine.amount` x0.001,
   `Brakes.amount` x-0.01, `Airbrake.amount` and `slidegrip` x1e-4). This answers
   the Units question [handling-stats.md](docs/formats/handling-stats.md) could
   not settle from the XML alone. The scaling is applied in
   `oag_gameplay::handling`, at the same point in the pipeline the original does
   it; `oag_formats::handling` deliberately returns the document's raw values and
   `oag_physics::Handling` holds the scaled form. **Applying it twice is the most
   likely bug in this area and it would be silent** - the ship would just be
   sluggish. `oag_gameplay::handling::SCALED_FIELDS` exists so the set is
   greppable, with a test that it has not grown.
2. **`Engine.gain`/`falloff` are dead.** The throttle state is ramped, stored,
   then overwritten with the raw input two instructions later. Verified in
   disassembly, not in the decompiler. `oag-physics` had faithfully implemented
   that ramp, in good faith, because the page said the idiom was documented.
3. **`ride_height` IS in the force law**, contradicting that page's "it never
   appears": it reaches the hover spring's target through `craft+0x2f0`.
4. **Groundedness is one frame stale for the control terms but not for grip.**
   Hover is step 8 of 15 and clears the contact flag on entry, so engine, brakes,
   pitch, drag and gravity read *last* frame's value while lateral grip and the
   weathervane read this frame's. The lag was known for the hover spring; it
   actually splits the force law in two. **A reimplementation that resolves
   contacts first diverges on every takeoff and landing frame.**

It also found real angular damping at `(-pitch_damping, -5, -2)`, which retires
an earlier "there is no angular damping at all" finding - that was true of the
page, not of the game.

## The best method lesson this session: two contradictions, and what they turned out to be

`docs/physics/README.md` recorded the surface-alignment torque as
`-400 * cross(up, avgNormal)` **and**, in the same sentence, as one "which levels
roll and yaw". Those cannot both hold. For unit `up`,
`(up x n) x up = n - up(n.up)`, the component of `n` perpendicular to `up`, so
`+k * cross(up, n)` aligns and `-k` diverges - and no magnitude, inertia tensor or
lever arm can change that. Simulation agreed: the literal sign tumbles a ship in
under a second.

**The interesting part is what happened next.** The weathervane torque recovered
from `engine.md` has the identical shape - `cross(forward, velocity) * -0.1`,
described two lines later as "what turns the nose toward the direction of
travel". Two independent transcription errors of one shape is a poor explanation
where a **systematic handedness difference** is a good one, and handedness is
listed as not determined on both pages. If that is what this is, both expressions
are correct in the original's own frame, nothing is a typo, and a reimplementation
in a right-handed frame must flip every cross product inherited from this path.

**That was a falsifiable prediction, and this pass falsified the mechanism behind
it.** Measured off a running race: `cross(row0, row1) = row2` exactly on 200 of
200 ticks, so the original's basis is positively oriented under the ordinary
component-wise cross product - the same arithmetic we use. There is no
frame-handedness difference available as the common cause of those two signs.

What the same measurements did settle is sharper and more useful. Holding left
gives `steer = -96` and `+1.51 rad/s` about row 1; holding right gives `+96` and
`-1.42`; 199/199 and 0/199 consistent, mirror-symmetric in sign and magnitude, so
it is steering rather than collision. Two consequences: **row 0 is the left
direction, not the right one** - it is named from use sites that fix the axis but
not its sign, and a reimplementation mapping it onto `+x = right` steers
backwards and puts the lateral grip term the wrong way with it - and any
reading of the angular accumulators now has a constraint to satisfy, namely that
`yaw = steer * Turning.amount` into local `.y` must come out as `-k * steer`
about row 1, `k` about `0.0155` at 22 units/s. If those two torque terms really
do need flipping, the sign lives in how accumulators are applied to the body, not
in the coordinate frame. See
[engine.md](docs/ghidra/functions/psp-pulse/engine.md).

Scored 84, not 85, and the reasoning is worth copying: the arithmetic admits no
third reading, but neither leg of the rubric's 85-94 band applies, because the
decompilation as recorded is self-contradictory, no call site was re-read, and
there is no multi-file data invariant.

### A number that turns "the ship wobbles" into a measurement

Roll is a damped oscillator with stiffness `k = 400` (alignment) and damping
`c = 2.0` (`Ship_ApplyAngularDamping`). Explicit Euler is stable exactly while
`h <= c/k = 0.005`, and the specified sub-step is `h = 1/180 = 0.00556` - about
**11 % too large**. So a ship sitting still on flat ground with a small roll grows
its oscillation by roughly a third per period and tumbles in a few seconds, which
`oag-physics` reproduces.

It is independent of the inertia tensor and of whether the angular accumulators
hold torque or angular acceleration, so neither open question tunes it away. One
of three things is true, and a trace distinguishes them in minutes: a magnitude is
wrong, the sub-step count is not three, or the original is marginally unstable in
that state and gets away with it because a racing ship is never in it. **This is
the first concrete thing M3 should measure.**

## Two formats decoded, both validated on both discs

**Collision geometry** ([collision.md](docs/formats/collision.md)) is an indexed
triangle soup in `.vex` nodes under five collision class IDs, separate from the
render mesh. The premise that it lives in the track `.vex` was unverified when the
work started and is now confirmed two ways: the class table, and the decoder used
as a detector firing on exactly those five IDs and no others across ~132,000
nodes. Validated over **319 nodes, 18,745 objects, 602,086 vertices, 594,615
triangles**, no exceptions. New findings:

- The "unknown `u16`" at chunk `+0x04` **is the element stride** (56,235/56,235).
- Chunk 3 is per-vertex, as predicted at confidence 88 (18,745/18,745).
- Every object stores exactly 3 chunks in order **1, 3, 2** - scalars before
  indices, which keeps the `f32` array 4-byte aligned when an odd triangle count
  ends the index array on a half-word.
- **All 602,086 vertex scalars are exactly `1.0`.** The field is authored and
  unused, so "what does the per-vertex scalar mean" is **not answerable from the
  assets** - only its consumer can say. The roadmap's open question is re-scoped.
- **Cage Collision is not dead content, it is the other SKU**: 6 nodes on PS2, 0
  on PSP.
- **Vertices are already world space** (identity transform, depth 1, 319/319). A
  consumer that composes the transform chain moves them twice.
- **The same format is in Wipeout Pure**, class IDs `0x36b`/`0x36c`/`0x37f`, 48/48
  closing exactly. Which is which is *not* determined (confidence 45), so nothing
  was named.
- **A live contradiction**: world extents reach 1,335.8 (PSP) and 1,554.1 (PS2),
  but the documented sweep-and-prune packing `((int)coord + 0x400) * 2` covers only
  about +/-1024. A straight transcription would silently drop geometry at the far
  end of a large track.

**Handling stats** are parsed and validated: 8 teams x 4 speed classes on **both**
discs, 174 attributes per file, every one required - a missing attribute is a
typed error, never a zero, because a `mass` that quietly arrived as `0.0` reads as
a tuning problem and gets looked for in the wrong place for a long time. `"nan"`
and `"inf"` are rejected too, since Rust's `f32` parser accepts them.

### Where the two readings disagreed, and which won

`engine.md` and `docs/physics/README.md` overlap and conflict in five places.
`engine.md` won each time, because it was read from the loader and the craft path
with addresses rather than summarised. Recorded so nobody re-litigates it:

| Question | Taken from | Note |
| --- | --- | --- |
| Bank-to-yaw: `+30 * right.y`, or that times `(1 - magLockBlend)` | `engine.md` | |
| Which speed the airbrake terms use | `engine.md` | The craft caches `abs(dot(v, forward))`; the brake uses `length(velocity)`, per its own section |
| Whether `ride_height` is in the force law | `engine.md` | It is. Ships now settle at a data-set height rather than an emergent one, which is the largest behavioural change of the session |
| Whether the engine ramp exists | `engine.md` | It does not |
| Two masses: spring reads `Physical::mass`, gravity reads `Body::mass` | both | Equal in practice; **keeping them equal is the integration layer's job** and nothing does it for you |

One interface note, because it looks like a 100x bug and is not:
`oag_physics::controls` works on the original's `0..=100` control scale, but
`ShipControls` remains normalised (`-1..=1` axes, `0..=1` triggers) and the
multiply by `CONTROL_RANGE` happens inside `oag-physics`. Callers must **not**
pre-scale.

## Traps and near-misses from this session

- **The PS2 `handlingstats.xml` files have a malformed prolog**:
  `<?xml version="1.0" encoding=utf-81"?>` - unquoted value, stray digit, **odd
  number of quotes**. Tracking quotes across the closing `>` put the scanner
  permanently out of phase and returned an *empty document with no error*. Two-line
  fix (end a processing instruction at its own `?>`), eight otherwise unreadable
  files. Wrong, silent, and it would have looked like a physics bug.
- **The collision walk came up 10 bytes short**, and the answer was 16-byte node
  padding - **not** the `u16`-chunk-count hypothesis the error message itself
  suggested. The arithmetic invariant beat a careful re-read again, and it beat a
  plausible hypothesis that the code was already nudging toward.
- **`docs/formats/handling-stats.md` was reproducing two real tuning values** in
  its Units note, an ADR-0006 violation that predated this session. Rewritten to
  describe the ratio. Worth a periodic sweep; the rule is easy to breach while
  making a legitimate point.
- **Do not run `just` or `cargo fmt --all` while background agents are editing.**
  A gate run taken mid-edit reported a physics test failure that no longer
  existed, and `cargo fmt --all` rewrites files an agent holds open, breaking its
  next edit. Scope to `-p <crate>` while work is in flight.
- **`ls` and `rg` intermittently fail on relative paths in this shell**
  (`permission denied`, `No such file or directory`) on directories that plainly
  exist; absolute paths work. Suspected `ecryptfs`. Use absolute paths in scripts.
- `data/` is on `ecryptfs`, so **`cp --reflink` does not work** - copying the
  three images is a real 4 GB copy, and there is no cheap clone.

### How to verify a renderer refactor with no "before" to compare against

This is reusable and it closed the one gap the `oag-render` extraction otherwise
had. `git worktree add <tmp> HEAD` gives a clean pre-change checkout; symlink
`data/images` into it (note the worktree already contains a tracked
`data/README.md`, so link `data/images`, not `data`), build `oag-view --release`
there, and capture screenshots. Then capture the same three from the working tree
and `cmp` them.

Result this session: `--mesh` on the Feisar ship, `--track` on `01_Track` and a
texture from `FE.wad` all came out **byte-identical** across the extraction. That
is what makes the refactor verified rather than only "behaviour-preserving by
construction".

## Verification status: what to lean on

**Verified against real data this session:** the collision layout (319/319 closure
on both discs), the handling schema (8 teams x 4 classes on both discs), the
`oag-render` extraction (byte-identical screenshots), and the three image hashes.

**Verified earlier and still good:** the WAD name hash and size-field ordering,
LZSS, texture decoding, `.vex` geometry and textures, the `section` count, the
whole `WO Track` layout, the junction slot ordering, the import stub names, the
PSP image base.

**Runtime-verified this pass, against the game actually running:** the variable
timestep and its `1/59.94` mean, the three dead fixed-step globals, the racing
`display+0x116c` flag, `secondsPerTick = 1e-6`, the `Ship_UpdateCraft` call site
and register assignment, and the state machine's pointer-and-inline-buffer
layout. This is the first evidence in the repository that came from execution
rather than from reading.

**Static reading, never executed:** *all* of the physics. `oag-physics` transcribes
`docs/physics/README.md` and `engine.md`, asserts structure, and **tunes nothing**.
The one exception is the two cross-product signs, settled by arithmetic - and even
there only the signs, not the magnitudes. Nothing in the force law has been
compared against the original running, which is what M3 is for. Do not raise a
confidence score on the strength of the simulation agreeing with itself; that was
exactly the RNG mistake this file already records.

## What is deliberately not done

| Not done | Why |
| --- | --- |
| Engine/brake/steer/pitch **verification** | The force law is recovered and implemented; nothing has been traced. M3. |
| The magstrip magnetic hold | Its force law was not decoded. Consequence: **an inverted ship falls off.** Nothing recovered pulls a ship toward a ceiling - gravity writes world `.y` only and the hover spring always pushes *away* from the surface it found. Expected, not a bug. |
| The four-corner hover variant | The selector is now known (`DAT_08ab07e3 == 0 && DAT_08b31048 == 6`, which also disables the brakes and swaps in an auto-speed law). The variant itself is not implemented. |
| Sweep and prune | A linear scan behind an AABB reject. Correct, and not the bottleneck. See the +/-1024 contradiction above before implementing it. |
| Quadratic drag and hover downforce magnitudes | Implemented as shape with the coefficient at `0.0`, because the pages record that the terms exist and not how large they are. |
| **The ship is rotationally unstable at rest** | Not a bug to patch. It is the `h <= c/k` arithmetic above showing through: a stationary ship with a small roll grows its oscillation and tumbles in a few seconds. Fixing it by changing a magnitude would destroy the measurement, which is currently M3's sharpest question. |
| **Quadratic drag adds energy while reversing** | `velocity * forwardSpeed * k` is dissipative only while `forwardSpeed > 0`. No prose contradicts the expression, unlike the cross-product terms, so it is transcribed literally and no test lets a ship reverse. A real hazard the moment anything can. |
| Sideshift | Recovered as a mechanism, but **which buttons fire it is not known**, so `oag_gameplay::controls` returns `Sideshift::None` always and a test pins the gap. |
| The game's own font | Unchanged: `.fnt` metrics decode, the atlas's pixel layout does not. |
| Front-end widget offsets, audio, batch list B, transparency sorting, mip levels, `section` payloads, the 29 unresolved import stubs | Unchanged from the previous handover. |

## Method that is not obvious

### Symbol names are reproducible

```sh
just apply-names        # 426 symbols into whichever program the bridge has open
```

120 come from [`names.tsv`](docs/ghidra/functions/psp-pulse/names.tsv) (up from 94)
and 306 are import stubs re-derived from the binary.
`scripts/apply-ghidra-names.py` refuses any row whose address and name are not
both still on its evidence page, so ADR-0005 is enforced rather than trusted.

**Two traps in the Ghidra bridge.** `dry_run` is *not* honoured by the rename
endpoints: it reports what it would do and does it anyway. And it rejects global
names lacking a Hungarian type prefix, which is not this project's convention, so
data addresses go through `create_label`.

`just check-docs` validates that links **resolve**, not that a page is
**reachable**. `engine.md` was an orphan when it landed. If you add a page, add
its row to the relevant `README.md` too.

### Naming imports is a check, not a guess

A PSP import NID is the first four bytes of `SHA-1(name)`, little-endian. Hash
candidates and keep the matches; a wrong name cannot produce the right NID. 306 of
335 resolve. See [imports.md](docs/ghidra/functions/psp-pulse/imports.md).

### Finding entries by name

WADs store only a CRC-32 of each name and most names are built at runtime, so
`scripts/mine-names.py` rebuilds the candidate list: `just mine-names <image>`,
292 of 1,142 in `Data.wad`. The step that matters is that **track directories are
numbered** (`01_Track`), so `Data\Plugins\PI001\Definition.xml`'s `location`
attributes plus the `%s\%strack%s.vex` template are what make track files
reachable at all.

### Validating a decoder

Every format decoded here had a self-check available, and using it has now caught
real errors nine times and confirmed a format outright three more. **Look for the
arithmetic that must hold before writing the parser, then put it in a test rather
than in prose.**

- **WAD**: the offset chain. 193/193 against 2/193 resolved a transposition.
- **`.vex` geometry**: each batch's declared bounding box, over 342,115 vertices.
- **`.vex` node tree**: child counts sum to one less than the node count. Exact as
  a `u16`; 111 million as a `u32`, which settled the width.
- **Embedded textures**: sizes sum to the declared block length.
- **Track**: exactly 64 `section` nodes, matching the 64-bit PVS mask.
- **`WO Track`**: the payload closes on 40 of 40 files.
- **LZSS**: where the *reader* stops - every one of 6,053 streams consumed to
  within one or two bytes of its end.
- **PMF**: header arithmetic, zero stray bytes, access-unit counts against
  duration.
- **Collision**: the walk closes on **319 of 319** nodes once 16-byte node padding
  is accounted for, which also pinned the chunk count at 32 bits and turned the
  unknown `u16` into the stride.
- **Handling stats**: completeness. 8 teams x 4 classes x every attribute, with a
  test that removes each of 174 attributes in turn and requires every removal to
  error.
- **The handling parameter block**: 32 floats = `0x80` = the observed per-class
  stride, closing with no gap or overlap, which is what placed the 13 previously
  unknown offsets.

### Running the game headlessly

`PPSSPPHeadless` boots straight from a raw ISO with no display:

```sh
chdman extractdvd -i data/images/pulse-psp-usa.chd -o /tmp/pulse-psp-usa.iso
PPSSPPHeadless /tmp/pulse-psp-usa.iso -l --timeout=8 > log.txt
```

`-l` logs the full HLE trace. `--debugger=PORT` opens a websocket debugger that
breaks at start; it **is** now used, and everything about it lives in
[ppsspp-debugger.md](docs/reverse-engineering/ppsspp-debugger.md). Two things
that page will save you rediscovering: headless cannot get past Pulse's
first-boot name entry (no persistent memory stick), so use `PPSSPPSDL` with
`RemoteDebuggerOnStartup`; and the SDL build throttles to a crawl when its window
is unfocused, which looks exactly like a hang.

### Ghidra

`BOOT.BIN` must be loaded with the **Allegrex** processor module, not stock
`MIPS:LE:32`, or the decompiler builds confident fictional C from VFPU
instructions. `just build-allegrex`, needs JDK 21.

Image base `0x08804000`, confirmed against PPSSPP's own loader log and against
`BOOT.BIN`'s ELF program headers. The Ghidra project still lives in the repository
root (`OpenAntiGrav.gpr`) rather than `data/ghidra/`; moving it is safe once
closed.

## Where I would go next

1. **`oag-trace`, the comparison side, which does not exist at all.** Capture
   works and records a moving ship; nothing reads a trace back and diffs it
   against a Rust run. The CSV's columns are already the ones
   [the verification protocol](docs/reverse-engineering/verification-protocol.md)
   asks for, and the output that matters is the first divergent tick and by how
   much, not a pass or fail. Note traces are derived game data: they live in
   `data/traces/` and are never committed, so this cannot be a CI test.
2. **Make the ship fly a lap, and try the two cheap explanations first.** The
   hover target exceeding the probe reach, and the missing inertia tensor, are
   findings 2 and 3 above; both are small, both are testable against
   `just play --race`, and neither needs an emulator. Only if those do not settle
   it is the integrator itself the suspect.
3. **The two numbers that cap confidence across the physics tree.** The
   integrator-stability arithmetic above (`h <= 0.005` vs `1/180`) and the
   handedness prediction are both falsifiable in minutes **once a trace has a
   moving ship in it**. The instrument is built; the measurement is not taken.
4. **Test the handedness prediction.** Recover one more cross-product term from
   the craft path. If it needs the same flip, the systematic-handedness
   explanation is confirmed and the coordinate-conventions open question falls out
   with it. Cheapest high-value RE task available, and it needs no emulator.
5. **Decode PS2 vertex type `0x1b9`**, still M1's exit criterion. `--track` now
   renders on both discs; `--mesh` gets past textures on PS2 and fails on this.
   Comparing the PSP and PS2 batch headers of the same ship is the obvious first
   move and needs no emulator.
6. **The `.fnt` atlas layout.** Unchanged and still the last thing between the
   front end and Pulse's own type. Four more transform families were ruled out two
   sessions ago; see [fnt.md](docs/formats/fnt.md) for what has been tried.
7. **LZSS is still tested only by its own inverse.** The stream-consumption check
   is the only independent oracle and it constrains the layout without pinning it.
   Untouched again this session.
8. **Check whether the determinism gate actually covers the new simulation.**
   `crates/core/tests/determinism.rs` compares a hash against a committed
   reference, and `CLAUDE.md` calls that gate load-bearing. It was written before
   `oag-physics` and `oag-gameplay` existed, and nobody checked this session
   whether any of their state feeds it. If it does not, the two crates that most
   need the guarantee are outside the gate. Worth an hour, and note the standing
   rule: **when that test fails, find the bug - never update the reference
   constants to make it pass.**
