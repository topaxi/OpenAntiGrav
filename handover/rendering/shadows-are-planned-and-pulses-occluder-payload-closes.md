# Shadows are planned as four techniques, and Pulse's occluder payload closes at `0x50 + 32n + 16m`

2026-09-02. The design is [`docs/rendering/shadows.md`](../../docs/rendering/shadows.md) - the ladder, the per-title split, the six-step plan. This file keeps only what is not there.

**Nothing is built.** The page exists so the enum and the menu row are not written twice.

**Step 1 of the plan landed the same day**: [`crates/vex/tests/shadow_occluder_ground_truth.rs`](../../crates/vex/tests/shadow_occluder_ground_truth.rs), four `#[ignore]`d tests, all four passing under `just test-data`. Every number the design page states is asserted there. The throwaway example that produced them was deleted in favour of it rather than kept beside it - two copies of the same walk is how two surveys drift apart.

**2048 was measured on 2026-09-02 and the expected answer was wrong.** The sweep predicted zero occluders and the disc holds **six** - three weapons (`pulse_bomb`, `pulse_mine`, `pulse_shuriken`), each shipped twice under `data/Weapons/` and `data/Weapons2048/`. All six close at `0x50 + 32n + 16m` with the same two header constants, and their `(n, m, len)` triples match Pulse's own `BEData.wad` entries for the same three weapons. **The layout is therefore a two-title, two-platform result, not a Pulse quirk.** Its craft author none, so 2048's ship shadows are its own runtime, not this class. The executable string comparison behind that is in [`shadows.md`](../../docs/rendering/shadows.md); the short version is that 2048 has **none** of HD's four shadow-map job names and **none** of its seven shadow samplers, and instead has `track_proximity_shadow_vp`/`_fp`, a precomputed per-circuit ship-environment shadow block, and a `ShadowLight` of its own.

**A false negative got as far as a passing assertion before the file-count guard caught it.** `vex_files` originally filtered on `version == 6`, which is right for Pulse and walks **zero** files on Pure, whose `.vex` are version 4 with 15 version-3 among them. "Pure authors no shadow class" then passes for the wrong reason and looks like a finding. The guard that caught it is `assert_eq!(files.len(), PURE_VEX_FILES)` - **pin the corpus size, not just the result**, on any sweep whose interesting answer is zero. The same slip put 415 in `PULSE_VEX_FILES` when the version-6 count is 382, and that assertion caught it too.

**The trap that cost the most here, and it is a general one.** `vex::mesh_batches(payload, 0).is_ok()` returns `true` on all 129 occluder payloads and means **nothing**. The first pass read that as "the payload is mesh-shaped", which would have put a `Skycube`-style decode in the plan. The real closure test - material count at `+0x02` placing the array end exactly at `+0x04`'s geometry offset - **fails on 129 of 129**, because `+0x02` is not a material count and `+0x04` is a stale runtime pointer on this class. `skycube_ground_truth.rs` warns about exactly this in its own header and it still caught someone. **Never accept a decoder returning `Ok` as evidence of a layout; only closure is.**

**What the second reading found, and why it is trustworthy where the first was not.** `n` at `+0x00` and `m` at `+0x02` give `0x50 + 32n + 16m == payload.len()` on **129/129**, over twelve distinct `(n, m)` pairs and lengths 272..4432. Two constants ride along at `+0x24` (`u32` 5) and `+0x28` (`f32` 1.0), both 129/129, and the first three floats of every one of the `n` 32-byte records are unit length on 129/129. Layout confidence 88; the "planes and edges of a convex hull" reading is **60** and carries no name yet.

**Both occluder record arrays are decoded (2026-09-02).** `n` 32-byte **face** records (unit plane normal, then a `u32` vertex count at `+0x0c`, then indices) and `m` 16-byte **vertex** records, `(w, x, y, z)` with `w` first and `w == 1.0` on every record on both discs. `pulse_mine` is a tetrahedron; `pulse_bomb` a pentagonal frustum. Interpretation confidence 60 -> **85**. Two traps found while getting there, both now asserted: **`m` counts slots, not vertices** (150 spare origin slots disc-wide), and **the bounding box is authored, not derived** - `BEData.wad#20` is a flat hull declaring its `y` max as the denormal `0x00800000`, so the disc-wide assertion is containment, not equality.

**The runtime reader is found, 2026-09-03 - `Shadow_RenderOccluderVolume` at `0x089038c8`, confidence 84.** [`shadow-occluder.md`](../../docs/ghidra/functions/psp-pulse-usa/shadow-occluder.md) has the full read; the short version: it reads exactly the `n`/`m`/bbox fields this payload decode pinned (three independently-established struct offsets landing where the function reads them), derives its projection direction from the occluder's **own local axis transformed by its own world matrix** - never a light - and extrudes a silhouette-edge stencil shadow volume from the `n` faces and `m` vertices. **It turns out to be the same function `exhaust.md` already named from an unrelated angle** (tracing `g_craft_scale`) and called "a stencil shadow-volume pass over the craft" - one function renders both the craft's own drop shadow and every free-standing occluder hull, branching on whether it is attached to a parent object.

**The static class-id link is traced too, same day.** Neither `get_xrefs_to` nor `get_function_callers` finds `Shadow_RenderOccluderVolume` (same unrelocated-constant trap `autopilot.md` and `positional-audio.md` already document), but `search_instructions` on the unrelocated `jal` target for `Vex_RegisterClass` turns up 50 call sites, and one of them - `DynamicShadowOccluder_RegisterClass` (`0x0890446c`, confidence 84) - passes the literal `0x3c3` to it, no relocation ambiguity at all since a class id is a plain immediate. That function then installs a method table at `0x08ad1214`; byte offset `+0x44` from that table's base (absolute `0x08ad1258`) holds `Shadow_RenderOccluderVolume`'s address, byte for byte. Class id `0x3c3` -> registration call site -> method table -> `+0x44` -> function, no step inferred from shape alone - but it's one binary's own internal consistency, not corroborated across files or by a trace, so it caps at 84 (Probable) rather than reaching Confident. A **checked, not assumed** claim: `+0x44` is not "slot 7 of a fixed grid" - two more registration functions' own tables sit at deltas that are not multiples of `0x88`, so table size varies per class and there's no uniform stride to index into; what *does* corroborate the table being real is that one of those two (`FogCube_RegisterClass`) repeats the exact same value at the exact same other offset (`+0x0c`) that `DynamicShadowOccluder`'s table has, while a third (`Mesh_Register`) overrides it - the shared-base/selective-override shape `exhaust.md` already described for `Vex_RegisterClass`. One sub-lead (a second constant in the same registration function, hoped to be the class's name pointer) did not resolve to anything legible and is recorded as a dead end rather than dropped silently - see `shadow-occluder.md`.

**A correction the same day, and the mechanism worth carrying**: a first pass at the padded-box question (below) generalized a rule from the 32 mismatching nodes alone, never checked against the 97 matchers for a counterexample - and two matchers falsified it immediately once checked. Also caught the same day: manually relocating a table of raw pointer values by hand produced a wrong sum on every row but the one independently cross-checked. Neither error was subtle in hindsight; both survived a first pass. **Check a claim against the population it was supposed to hold on, not just the population it was read from - and do relocation arithmetic with a calculator, not by eye.**

**The padded box at `+0x30`/`+0x40` is read too, same day - and the first reading of it overclaimed a clean rule, corrected the same day once checked against the full corpus rather than just the 32 mismatchers it was read from.** Quantified precisely instead: of the 14 nodes with a positive header `min.y`, 12 get `+0x30`'s `min.y` floored to `0.0` and 2 keep their real value (both `shadow_lodShape`, same two files, same smallest value in the set, undiscriminated). Of the 54 nodes where the packed header's `max.y` is the denormal sentinel `0x00800000` and the true value isn't itself zero, 16 get `+0x40`'s `max.y` fixed to the true vertex-derived value and 38 get a hard `0.0` instead - neither node name nor the numeric value predicts which. (The `97 of 129` matcher count itself hides some of these: the test's `1e-6` tolerance can't distinguish a true copy from a hard-zero substitution against the `~1e-38` denormal, so an unknown slice of the 97 is the latter.) The honest summary: the padding leans toward the ground plane far more than away from it, consistent with being shadow-relevant rather than a plain geometry cache, but it is not a single deterministic rule. Full numbers in `shadow_occluder_ground_truth.rs`'s `PSP_OCCLUDERS_WITH_PADDED_BBOX` doc comment and `shadow-occluder.md`.

**A dead end worth not repeating**: the `507` repetition of 2048's Nova technique names is a **linker artifact**, not a material count. The full technique list is emitted once per translation unit that references it, interleaved with that unit's own debug strings (`Backend/General/MPTournament_RaceManager.cpp` and friends), at variable stride. An earlier version of `shadows.md` read it as "507 shader sets" and was wrong. The check that settles it: unrelated neighbours in the same table (`Tournament` 16, `Elimination` 3) do not repeat, so it is not a whole-pool duplication either.

**HD's own `LiveStencilShadow`/`shadow.stencilvolume` path is traced now too, 2026-09-04** - [`shadow-stencilvolume.md`](../../docs/ghidra/functions/ps3-hdfury-eu/shadow-stencilvolume.md). The shader technique is a real registered material with three named constants (`worldViewProj`, `lightDirection`, `extrusionDistance`, each individually resolved against its own instruction's TOC rather than trusted from Ghidra's symbol names) - `lightDirection` being named at all is new evidence HD's version may take a light as input, unlike Pulse's `Shadow_RenderOccluderVolume`, which reads none. A per-model flag bit (`self+0xe4`, bit `0x2`) builds a path ending in the literal leaf `shadow.stencilvolume`, then loads it through a `Crc32`-hashed find-or-load cache into a parsed record array plus a computed bounding box. The record shape rhymes with Pulse's `.vex` occluder payload (leading counts, a geometry array, a derived bbox) but is only partly understood (confidence 50 - a sub-call this pass didn't decompile sizes the parts that matter most) and not obviously byte-identical. **Two mechanisms I got wrong on the first pass, both caught before being written down as findings rather than after**: two string constants nearby in the same decompiled function (`TrailDriveLevel`, `uvOffset` and others) looked at first like they belonged to the same path-building block - they don't, they're unrelated material-parameter names hashed elsewhere in the same (very large, multi-purpose) function, caught by checking `scripts/ps3-toc.py resolve` against the exact instruction issuing each load rather than trusting proximity in the decompiled text. And a data cross-reference on two of the functions involved looked like a shared vtable/dispatch slot, implying reuse for other resource kinds - it's an ordinary PPC64 OPD descriptor (`scripts/ps3-toc.py u32` on it resolves to exactly `{that function's own address, the shared module TOC}`), which every function in this ELF has regardless of whether anything dispatches through it. Both are the same category of mistake this file's own "correction" paragraph above already warns about: a claim generalized from something read once, not checked against a population (the surrounding strings; every other function in the binary) that would have falsified it immediately.

**The draw call is found too, same day.** `Shadow_DrawOccluderStencilVolumes` (`0x003e6918`) walks the model-instance array and, for every instance flagged (a byte at `+0x140`, role unread) and shadow-volume-bearing, runs a colour-mask-bracketed two-sided depth-fail ("Carmack's Reverse") stencil shadow volume: colour off, `Shadow_AccumulateStencilVolume` (stencil write, cull disabled, `ALWAYS`/ref-1 func, `INCR_WRAP`/`DECR_WRAP` front/back ops), colour on (RGB, alpha off), a generic scene-chunk submit (`FUN_004053e0`, blend enabled) whose geometry is *not* confirmed - an earlier same-day draft guessed "the caster's own mesh" without checking and that guess is retracted - colour off again, then `Shadow_ClearStencilVolume` (stencil test `NOTEQUAL` 0, `(KEEP,KEEP,ZERO)` op, no colour output at all - a stencil reset, not a "resolve" the way an even-earlier draft named it). The colour-mask sequence is the tell: `FUN_004053e0` is the only one of the three draws with colour on and blending enabled, making it the real candidate for whatever visibly darkens the screen, while the two named stencil passes write no colour whatsoever. Both stencil passes upload the `LiveStencilShadow` technique's constants and draw the *same* `self+0x128` handle via an argument literal `5` that, if a GCM primitive type, is `TRIANGLES` (`gcm_enums.h` is 1-based, so `5` is not `TRIANGLE_STRIP` - an earlier same-day draft cited that wrong too, without checking the actual enum). Every RSX register identity here (`0x304`, `0x310`, `0x324`, `0x328`, `0x348`, `0x183c`, the func/op enum values) was checked against a local `rpcs3` checkout's own `Emu/RSX/gcm_enums.h`, not assumed from general OpenGL familiarity - the same move `material-state.md` already used. **This also revises the record-format reading**: the draw walks `self+0x128` as a `std::vector`-shaped object, one 16-byte entry pushed per *outer* iteration inside `Shadow_ParseStencilVolumeGeometry` (not per `n`-record), and reads each entry's index-count field as the primitive count. **Three claims corrected in the same pass they were made, all caught by checking rather than trusting an inference from shape**: what `FUN_004053e0` draws, which primitive `5` names, and what to call the resolve/clear pass - see `shadow-stencilvolume.md`.

**The record format closed the same day, once `Shadow_AllocateStencilVolumeBuffers` (`0x005ee2c0`) was decompiled.** It allocates an `n`-element, 24-byte-stride vertex buffer and a separate `m`-element index buffer - and the index buffer's own width computation (`FUN_005a8750`'s branchless format select, evaluated for the literal `param_2=0` this call site passes) comes out to exactly `4` bytes, matching `Shadow_ParseStencilVolumeGeometry`'s own `m << 2`-byte `memcpy` of the index blob without either function citing the other. That's an exact arithmetic invariant between two independently-read functions - the confidence rubric's own top band short of a runtime trace - and it settles what the first pass over this function left as a guess: **`n` is the vertex count, `m` is the drawn `u32` index count.** Same coarse shape as Pulse's `.vex` occluder payload (leading counts, a geometry array, a derived bbox), but a genuinely different topology encoding, not just different bytes: HD pairs an explicit vertex+index buffer, Pulse indexes its own face/vertex-slot arrays with no separate index list. `Resource_FindOrLoadByHashedPath` (the renamed `0x003ee398`) and `Shadow_ParseStencilVolumeGeometry` (the renamed `0x005ee7d0`) are both named now too - the former deliberately *not* `Shadow_`-prefixed, since nothing in it is shadow-specific and the path is entirely the caller's.

**The record format is byte-verified now, same-thread continuation on
2026-09-04: `scripts/psarc.py` finds all 39 real `shadow.stencilvolume`
files disc-wide (one per `data/ships/<name>/`) and every one matches
`16 + n*24 + m*4` exactly, with an identical `(n=24, m=108)` header on all
39 - not a sample. Decoding the vertex records closes what was still open:
they are `(x, y, z, nx, ny, nz)`, and the 24 vertices/108 indices form a
fixed, unwelded six-face box (4 verts/face, 12 face-quad triangles, plus 24
more triangles that turned out to be degenerate padding - see below) with
only vertex *positions* varying per ship to fit its own bounding footprint -
the topology itself is byte-identical across all 39. Record-format confidence raised 82 -> 92, the
"exact arithmetic invariant across many real files" band. This also forced
a correction: a previous pass here called the min/max reduction at
`param_1+0x20`/`+0x30` a "bounding box"; the offsets it actually reads
(`0xc`/`0x10`/`0x14`) are the *normal*, not the position, per this
byte-verified layout, and a min/max over a fixed box's six face normals is
a constant, not a spatial bound - the "bbox" framing is retracted from the
doc page, the plate comment, and here. The sealed-box shape is consistent
with per-vertex extrusion happening in `LiveStencilShadow_vp` rather than a
CPU-computed silhouette hull the way Pulse's `Shadow_RenderOccluderVolume`
builds one - a plausible mechanism reading, not a confirmed one (confidence
65; the shader's own bytecode is unread). Cross-title note worth keeping:
Pulse authors a distinct hull per weapon (`pulse_mine` 4/4, `pulse_bomb`
11/12); HD authors one topology once, disc-wide, and only refits vertex
positions per caster. One loose end found along the way, not resolved:
`data/ships/detonator/`'s box is ship-scale, similar order of magnitude to
`feisar`'s and `qirex`'s, despite `detonator-bomb.md` naming `DetonatorBomb`
as a weapon class - whether this is a distinct ship-sized entity sharing the
name, or a weapon shadow proxy authored at ship scale, is unresolved. Full
numbers and the reproduce command are in `shadow-stencilvolume.md`.

**The vertex shader is disassembled too, same-thread continuation on
2026-09-04.** `Shader_SetLiveStencilShadowTechniqueActive`'s
`ShaderRegistry_Register` calls resolve to real `"SHO\x08"` blocks built
into `EBOOT.elf` itself (`0x00929600` for `LiveStencilShadow_vp`,
`0x00929580` for `_fp`), and `scripts/ps3-microcode.py` disassembles both.
The vertex program's 8 instructions settle the mechanism the box topology
only suggested, and it turns out not to be what the "silhouette extrusion"
framing assumed: every vertex - not a subset selected by facing - gets the
*same* displacement, `lightDirection * extrusionDistance`, added to its
position before the `worldViewProj` transform. The vertex normal is dotted
with `lightDirection` into a register that is never read again in the
8-instruction program - genuinely unused, not merely unconfirmed. All three
parameter names are confirmed by exact hash preimage
(`~crc32("worldViewProj")`/`"lightDirection"`/`"extrusionDistance")`
matching the values `ps3-microcode.py` printed byte for byte. So: a
**rigid-body shift of the whole sealed box along a uniform direction**, not
a per-vertex silhouette extrusion the way Pulse's `Shadow_RenderOccluderVolume`
computes one - which explains *why* a fixed, disc-wide box template works
at all: there's nothing model-specific left for the vertex program to key
off of. The fragment program is trivial (`MOV H0, {1,0,0,0} | END`),
consistent with colour being masked off for both stencil passes anyway.
Confidence 84 (a direct, unambiguous disassembly with hash-confirmed
parameter identity, but one shader block rather than an invariant checked
across many files, and not runtime-traced - the same 84 ceiling the doc
page's own intro states). Full instruction listing in
`shadow-stencilvolume.md`.

**A same-day advisor check caught an assumption the vertex-shader finding
needed verified, not just written down: does the accumulate pass actually
draw the buffer twice (once shifted, once not) the way a classic near-cap/
far-cap shadow volume needs?** Decompiled both `Shadow_AccumulateStencilVolume`
and `Shadow_ClearStencilVolume` directly to check: neither re-uploads a
different `extrusionDistance` between draws, both pull the same cached
technique-constant block the same way and draw `self+0x128`'s geometry once
each. So there is no separate unshifted/shifted pair anywhere in this
path - both passes submit the *same*, once-shifted closed box. That's still
a legitimate use of two-sided depth-fail stencil (any already-closed
watertight solid can be tested this way without needing a near/far pair -
this shader builds the closed volume the other way, by shifting the whole
thing rigidly instead of extruding it), but it means what's actually being
tested is closer to **"does this pixel's depth sample fall inside a fixed
box template, shifted toward the light and positioned at the caster"** than
to a true silhouette-derived shadow volume. Confirmed by decompile, not
assumed - see `shadow-stencilvolume.md`.

**A second advisor check, same session, caught an overstated claim about
the mesh itself: "sealing the box into one closed manifold" was never
verified.** Checked directly across all 39 real files: the 24 "bridging"
triangles the previous pass named are **degenerate on every one of the 39
files** - each has two of its three vertices at the exact same 3D position
(two of a box corner's three per-face-normal copies coincide), so they
rasterize zero area. The box was already watertight from its 12 real
face-quad triangles alone (adjacent faces' corners already coincide in
position, just not by index); the other 24 triangles are inert padding, not
sealing geometry - the earlier "sealing... rather than 6 floating quads"
framing overstated their role and is retracted. Two headline labels
("a two-sided depth-fail stencil shadow volume") were also caught still
asserting what the body had already retracted, in the doc page's own
heading, `shadows.md`, and `HANDOVER.md` - fixed to name what was actually
confirmed: a stencil test over a rigidly-shifted box proxy.

**A third advisor pass connected the two loose ends from the last two
findings into one hypothesis, from facts already on the page.** The vertex
shader computes `dot(normal, lightDirection)` and never reads it; the
geometry has 12 real face triangles plus 24 degenerate corner-bridging
ones. Both are exactly what a real near-cap/side-wall silhouette-extrusion
volume needs and doesn't get: the dot product is exactly the term such a
shader would use to decide which corner copies to displace, and 12+24 is
exactly that construction's topology. Leading hypothesis, not confirmed:
this asset and shader were authored for a genuine silhouette extrusion, and
the shipped vertex program takes a simpler, uniform-shift path instead,
leaving the dead dot product and the degenerate triangles as vestiges of
the unused branch. Recorded as the leading Open item on
`shadow-stencilvolume.md` for whoever picks this up next; nothing here
explains *why* the branch went unused.

## Open

- **What 2048's `track_proximity_shadow_vp`/`_fp` pair actually does.** The name and the separate `Lighting.Debug.Draw Ship Shadows` / `Draw Ship Env Shadows` toggles are all the evidence there is; no function has been read. Same for the `PRECOMPUTED TRACK (SHIP ENV SHADOWS)` block - its budget line proves it is precomputed per circuit and sized in main and VRAM, and nothing says what it holds
- **Where 2048's `Lighting.ShadowLight direction` is authored.** It is a key in the executable; `effectsettings.md` records that 2048's per-stage blocks author no `Lighting.*` keys at all, so it is likely `.envsettings` - unchecked
- **Whether `mapped` is worth offering on HD or 2048 at all.** Both already draw authored shadows, so `mapped` buys receivers and cascades and *spends* art - HD's lightmap-alpha shadows, 2048's precomputed environment shadows. Nobody has put them side by side, and until someone does, "an improvement" is an assumption
- **Whether the 507 Nova blocks are one per material.** The count and the ~3.6 KB repeat spacing are measured; that a block corresponds to a material rather than to some other shader-set grouping is a reading at about 70, and 2048's own material-table count would settle it
- **What `Nova` is named after, and whether an `Ile`-era doc page should be retitled.** The `Ile*` tokens on `rcsmaterial.md` are documented as material flags with no system behind them; they are the HD generation of this pipeline
- **Whether 2048's directional bake is a radiosity-normal basis or a single dominant direction.** "DirectionalNova" and the `VL1/2/4/6` variants say directional; the basis is unread, and it is the part that would actually have to be reimplemented
- ~~The stencil-volume lead is not chased.~~ **Chased and confirmed 2026-09-03**: `Shadow_RenderOccluderVolume` is a runtime reader, not just a lineage argument - see above
- The `m` 16-byte records' vertex order/winding within a face's index list is undecoded beyond "vertex data": `Shadow_RenderOccluderVolume` reads them positionally but the face-to-vertex index mapping's exact byte layout was not worked out this pass
- ~~The static class-id link is still open.~~ **Traced 2026-09-03**: `DynamicShadowOccluder_RegisterClass` (`0x0890446c`) registers `0x3c3` and installs the method table whose `+0x44` holds `Shadow_RenderOccluderVolume`'s address, byte for byte - see above. What's left is only a runtime trace, per `shadow-occluder.md`'s own Open section
- ~~The padded `vec4` bbox copy at `+0x30`/`+0x40` matches the packed one at `+0x0c`/`+0x18` on **97 of 129**. What the other 32 carry there is unread.~~ **Read 2026-09-03, corrected same day**: quantified, not a clean rule - see the paragraph above. **File/build-version checked 2026-09-04 and ruled out**: no such field exists in the WAD directory, and sibling nodes in the same file entries disagree regardless (`shadow_padding_probe.rs`). The repeated exact vertex-derived value predicts kept-vs-hard-zero on 14 of 17 groups, not 3 - closer than "no discriminator" but still not a rule; what actually decides those 3 (and the 2-of-14 `min.y` exceptions) is unknown. **The 10 unnamed/track-side nodes are confirmed to behave differently, same pass**: two of them diverge on `x` (one also `z`) where every local/named node only ever diverges on `y` - see `shadow-occluder.md`
- ~~Where the light direction for a Pulse shadow comes from.~~ **Answered 2026-09-03**: nowhere. `Shadow_RenderOccluderVolume` derives it from the occluder's own local axis, transformed by its own world matrix - Pulse never reads a light for this, consistent with the disc having no light rig to read one from
- Whether the 10 world-space occluders are a track feature at all, or authored-and-inert the way `DirectionalLight` turned out to be. Their bbox extents run to 882 units, so they are not craft
- Whether `original` should be the default once it exists, on a title where the tier is not obviously better-looking than `blob`
- ~~HD's stencil-volume draw call is still unfound.~~ **Found 2026-09-04**: `Shadow_DrawOccluderStencilVolumes` (`0x003e6918`) walks the model-instance array for the flagged ones and runs a two-sided depth-fail stencil test, colour-mask bracketed - `Shadow_AccumulateStencilVolume`, a colour-on/blend-on submit (`FUN_004053e0`) whose geometry is unconfirmed, then `Shadow_ClearStencilVolume` (a stencil reset, not a "resolve") - RSX register identities cross-checked against a local `rpcs3`'s own `gcm_enums.h`, not assumed. **Later found (below) to be a test over a rigidly-shifted box proxy, not a true silhouette-derived volume.** **`FUN_004053e0` turns out not to draw immediately at all**: it compiles a small command stream and hands it to a shared, seven-caller SPU-job-submission primitive matching this project's own already-documented job pattern (`engine-trail.md`) - generic infrastructure, but through a path that would need SPU-job-level tracing to say what geometry it actually carries. See `shadow-stencilvolume.md`
- ~~No PSARC archive has been searched for an actual `shadow.stencilvolume` entry.~~ **Found and decoded 2026-09-04, all 39**: byte-verifies the `n`=vertex/`m`=index split (confidence 82 -> 92) and reveals a fixed sealed-box topology shared by every ship - see above and `shadow-stencilvolume.md`
- ~~Whether `LiveStencilShadow_vp` actually extrudes per-vertex along `lightDirection`/`extrusionDistance`.~~ **Disassembled 2026-09-04**: no - it's a uniform shift applied to every vertex regardless of facing, not a per-vertex silhouette extrusion (confidence 84) - see above
- **HD's shadow pipeline is read and measured, 2026-09-11** - [`shadow-model-maps.md`](../../docs/ghidra/functions/ps3-hdfury-eu/shadow-model-maps.md). `Shadow_RenderModelShadowMaps` (`0x003ed810`) renders one map per ship from 70 units up `Lighting.Sun direction`, box fitted to the ship's bbox, near 1 / far 140; `RenderModelShadowsOnTrack` is the stencil pass plus a 50-unit track-chunk redraw under `SRC_ALPHA/ONE_MINUS_SRC_ALPHA`; `RenderModelAmbientShadowsOnTrack` is empty and its compiler unreferenced, so HD never draws `ambient_shadow.gtf`. Live in RPCS3 mid-race: the global env block holds the circuit's sun unchanged, every craft matrix carries the `0.75` scale, and recorded craft heights raycast against our track at `3.9` on the straight (ours `4.0`). The wrong-side shadow the thread was opened for was this project's receiver lookup, mirrored since 2026-09-04 - fixed in `mesh.wgsl`, pinned by an off-centre caster in `shadow_map_coverage.rs` on both axes; a plan-view render now shows the shadow where that sun puts it. **Still open from it**: (a) the original's per-ship fit against our one grid-wide fit is a texel-density difference worth a screenshot pair on the same team and circuit; (b) `mapped` darkens the whole road uniformly to a `0.797` luminance ratio against `off` while scenery stays at `1.000` (Talon's Junction, autopilot tick 2100, regional medians) - two of its four depth taps reading shadowed everywhere (`1 - 0.5 * MAPPED_STRENGTH`), identical before and after the mirror fix, so a bias/tap problem of that tier's own; the first thing to look at is the half-texel tap offsets against the rasterizer's slope-scaled bias in `shadow::map::Map::new`; (c) how dark the original's redraw makes the road is still the `ShadowToAlpha` variant's unread colour - `MAP_STRENGTH` stays ours.
- **Whether other model classes (props, track pieces, other weapons) share HD's fixed 24-vertex box template**, or use a different one - only `data/ships/*` was checked
- **Why `data/ships/detonator/`'s box is ship-scale** despite `detonator-bomb.md` naming a `DetonatorBomb` weapon class - not resolved
- ~~What `0x005ee7d0`'s sub-call, `0x005ee2c0`, does with `(n, m)`.~~ **Decompiled 2026-09-04**: it allocates the vertex and index buffers, and its index-width computation gives the exact arithmetic invariant that closed the `n`/`m` question - see above. What's left of the record format: the other three of six floats per vertex, and the real on-disc byte layout (still unverified against a captured file)

**`Ile` and `Nova` are two generations of Studio Liverpool's baked-lighting pipeline, and 2048 replaced the first with the second.** HD's lightmaps are `ile_mesh_combine_*-lmap.gtf`; 2048's are `nova_mesh_combine_*-lmap.gxt`, **941 of 941**, the thirteen ported HD/Fury DLC circuits included - not one HD lightmap survived. HD's executable has no `nova` string and 2048's has no `Ile` string, so the two never coexist. The material flags map one to one: HD's `IleLightmap`/`IleVertex` is 2048's `DirectionalNovaTexture`/`DirectionalNovaVertex` - **which means [`rcsmaterial.md`](../../docs/formats/rcsmaterial.md)'s already-documented `Ile*` tokens now have a name for what they belong to.** Every ported circuit was baked twice, forward and reversed. Confidence 90. What Nova actually *computes* is unread and unassessed.

**A measurement error worth not repeating**: filtering lightmaps with a substring match on `lmap` also matches `NormalMap.gxt` (`Norma`+`lMap`). It inflated the first pass on both titles - 2048 955/870.1 MiB and HD 818/934.2 MiB, against the corrected 941/867.1 and 793/925.2. The conclusion survived; the numbers did not. Filter on `-lmap.gtf$`/`-lmap.gxt$`.

**The "2048 looks better than HD" question was raised from play and checked against both discs.** The intuitive answer - 2048 prebakes more - is **wrong**: HD ships **934.2 MiB** of lightmaps across 818 entries against 2048's **870.1 MiB** across 955. What actually differs is coverage and kind. 2048's bake is *directional* and universal (the Nova system, 16 permutation names, lightmaps named `nova_mesh_combine_track_N-lmap.gxt`), while HD leaves **305 of 983 chunks with neither a lightmap nor a colour set** and makes the difference up with a runtime rig - dynamic lights, SPU vertex lights, light volumes, spotlights - that 2048's vocabulary does not contain at all. Full working in [`shadows.md`](../../docs/rendering/shadows.md). **Not controlled for**: 2048 runs 30 Hz at 960x544 against HD's 60 Hz at 1280x720, ~3.5x the GPU time per pixel, and art direction is not controlled for either.

**One follow-up is blocked on tooling rather than on evidence.** `track_proximity_shadow_fp` is PS Vita **GXP** microcode and this repo has only a PS3 decoder (`scripts/ps3-microcode.py`); a GXP disassembler is its own piece of work. (The `0x3c3` runtime reader no longer belongs on this list - a live Ghidra bridge was available 2026-09-03 and both it and its registration were traced. HD's `LiveStencilShadow`/`shadow.stencilvolume` path still needs the same kind of pass on the HD binary specifically, not attempted this thread.) **The blocker moved, not closed, 2026-09-04**: `scripts/vita-gxp.py opcodes` now locates the USSE opcode field corpus-wide (a variable-width prefix, mostly 5-6 bits, plus secondary's end bit at bit 50) - see `handover/tooling/2048s-gxp-containers-decode-the-usse-stream.md` and `docs/formats/gxp.md`. No opcode is named yet, so `track_proximity_shadow_fp` still can't be read; the next step for *that* is naming an op1 value, tracked on the GXP thread rather than here.
- **Does the original's `original` shadow blink the way ours did before 2026-10-05?** The hull-on-one-plane blink is fixed ([the section](../../docs/rendering/shadows.md#the-shadow-blinked-2026-10-05-a-flat-polygon-on-a-road-that-is-not-flat)) but never compared with a PPSSPP software-renderer capture of a Zone race over consecutive frames. The visible road stands proud of the collision floor by up to roughly `LIFT` 0.15 on that circuit; where that gap comes from (a raised art layer, the ribbon's own offset) is unread. `LIFT` 0.15 is chosen, not measured. `Race::floor_above` is about 300 casts per craft per frame at `original`, unprofiled.

## Next Steps

**The RE half of this thread (steps 1 and 4) is done.** Building anything on
it - the setting, `blob`, Pulse's `original` tier - is a rendering task now,
not a reverse-engineering one, and lives in its own thread:
[original-on-pulse-is-the-tier-still-unbuilt.md](original-on-pulse-is-the-tier-still-unbuilt.md).
What's left here is the RE work that thread doesn't need and isn't
unblocked by it:

- ~~HD's `LiveStencilShadow`/`shadow.stencilvolume` path needs the same kind
  of Ghidra pass this thread just gave Pulse's `0x3c3`, done on the HD binary
  specifically - not attempted here.~~ **Traced 2026-09-04**: the shader
  technique (`Shader_ResolveLiveStencilShadowConstants`,
  `Shader_SetLiveStencilShadowTechniqueActive`) and the per-model trigger (a
  flag bit builds a fixed-named `shadow.stencilvolume` sibling path, loaded
  through a hashed resource cache into a record array plus a computed
  bounding box) are both found - see
  [shadow-stencilvolume.md](../../docs/ghidra/functions/ps3-hdfury-eu/shadow-stencilvolume.md).
  **The draw call is traced too, same day**: `Shadow_DrawOccluderStencilVolumes`
  runs a two-sided depth-fail stencil test, colour-mask bracketed, RSX
  register identities cross-checked against a local `rpcs3`'s own source
  (later found to be over a rigidly-shifted box proxy, not a true
  silhouette-derived volume - see below). **The record format is closed and byte-verified, same day**: an
  `n`-vertex, `m`-index (`u32`) buffer pair, settled first by an exact
  arithmetic invariant between two independently-read functions, then
  confirmed against all 39 real `shadow.stencilvolume` files disc-wide -
  every one an identical fixed sealed-box topology, only vertex positions
  varying per ship - see above (confidence 92). **The vertex shader is
  disassembled too, same day**: a uniform rigid-body shift along
  `lightDirection * extrusionDistance`, not a per-vertex silhouette
  extrusion - the normal is read but its dot product is never used
  (confidence 84). **What's left, not attempted this pass**: what
  `FUN_004053e0` (the actual visible-effect candidate) draws
- **Write a GXP fragment-program decoder** (`scripts/vita-gxp.py`, on the model of `scripts/ps3-microcode.py`). It unblocks `track_proximity_shadow_fp` and `NovaShipOcclusion`, and beyond shadow it is the only way to read any of 2048's 67 shader programs
- Once the GXP decoder exists: **what `track_proximity_shadow_vp`/`_fp` actually does**, **where `Lighting.ShadowLight direction` is authored** (`.envsettings` is the leading guess), and **whether 2048's directional bake is a radiosity-normal basis or a single dominant direction**
- **A PPSSPP watchpoint on `self+0x50`** during a lap past a known occluder circuit would move `Shadow_RenderOccluderVolume` and `DynamicShadowOccluder_RegisterClass` past confidence 84 - optional, not blocking anything
- **What actually decides the padded-bbox field's remaining exceptions** (the 3-of-17 value-groups and the 2-of-14 `min.y` nodes) - file/build-version, node name and specific value are all now ruled out (see above), so the driver is something not yet identified; likely needs the export tool's own behaviour, not more disc-side sweeping

## From the HANDOVER.md index (moved 2026-09-25)

RE only; the implementation half has its own thread, below. Pulse's `0x3c3` occluder, HD's `LiveStencilShadow`/`shadow.stencilvolume` path (all 39 real files decoded, vertex shader disassembled) and 2048's `Nova` bake are all traced and closed at confidence 82-92; what is left is 2048's `track_proximity_shadow_vp`/`_fp` pair (unread), where `Lighting.ShadowLight direction` is authored, and one ship-scale outlier (`detonator`) whose box does not match its weapon class - see the thread for the full trail.
