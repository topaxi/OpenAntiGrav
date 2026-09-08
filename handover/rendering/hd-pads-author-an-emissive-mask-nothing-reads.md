# HD's pads author an emissive mask and a material of their own; nothing reads either

2026-09-03. Fell out of fixing the invented pad tint (`docs/rendering/pads.md`,
commit "fix(render): a pad draws the colour its own texture paints"). That fix
is complete and this file is only what it did **not** close.

## What is closed, so nobody re-derives it

- A speed pad is **never recoloured on any title**. `Pad_UpdateRefreshTimer`
  (`0x089265f0`) writes no colour, Pulse's gold and HD's blue are both in the
  texture, and `oag_render::speedup_pad` is deleted rather than gated.
- Pulse's `Weapon Pad` cycle (`pad+0x6c`, table `0x08ac00c8`) stays exactly as
  recovered on PSP/PS2 and no longer runs on HD.
- On a PS3 chunk `GpuVertex::colour` is HD's baked per-vertex **light**, not a
  tint. `Drawable::tint_weapon_pads` gates on `Model::vertex_colour_is_light`
  for that reason, and
  `crates/game/tests/hd_pad_illumination_ground_truth.rs::hd_pads_keep_their_authored_light_and_are_never_recoloured`
  pins it.
- `uNumSpeedupPads` / `uNumWeaponPads` are **leaderboard stat fields**, not
  render state - their only reader is the `sceNpManagerGetOnlineName`
  serialiser at `0x0001d800`. A dead end, recorded as one.
- **What binds a pad's `_ne` file - answered, asset-side, 2026-09-07.** Every
  pad chunk's material record on `12_sol_2` is **inline**, not a separate file
  the chunk merely names, and it genuinely carries the `_ne` mask as its own
  `second_texture` (`+0x78`/`+0x60`) - the eight `Weapon Pad` chunks all name
  `materials/weapon_pads.rcsmaterial`, exactly and only them (8 material
  records disc-wide name it); the ten `Speedup Pad` chunks all share one
  material slot naming the generic `materials/diffuse_normal_specular_emmissive.rcsmaterial`
  instead, not `speedup_material.rcsmaterial` - which **zero** material
  records in this model name, so it reads as an authored orphan here and does
  not explain why the disc ships it standalone after all. See
  `docs/rendering/pads.md`'s "What binds the `_ne` file" section for the full
  measurement and `crates/render/examples/hd_pad_material_dump.rs` for the
  probe. **Do not re-derive this**; the next open question is whether it is
  worth wiring, not whether it exists.
- **The `_ne` file's RGB is a normal map, not paint - answered, asset-side,
  2026-09-07.** Not red, not white, not any flat colour: `ds_weaponup_ne.gtf`
  and `ds_speedup_ne.gtf` both have a whole-texture RGB mean of
  `[127, 129, 243]`/`[127, 128, 244]` - almost exactly the canonical
  tangent-space normal-map neutral `[128, 128, 255]` - over 32,770/35,360
  distinct RGBA values across ~1.05M texels each, a continuous gradient, not
  a handful of painted colours. The rendered PNG is an unmistakable embossed
  blue/purple bump map. Confirmed a second way, off the shader itself rather
  than the pixels: the instruction right after each pad's unit-1 `TEX` is
  `MAD R#, R#, {2.0, -1.0, ...}, ...`, the standard `x*2-1` unpack of a
  `[0,1]`-packed normal. **Neither pad's fragment program carries a tint
  either** - `emissive`'s tint parameter (`0xe8bcd7f5`) is absent from both
  programs' declared-parameter lists and patches zero code slots in either,
  so it never reaches the microcode at all; the only non-zero inline
  constants in either 50-instruction program are the normal-decode pair and
  a Blinn-Phong specular trick's literals. **So nothing measured, asset-side,
  explains "light up red" for the light bars.** A second, separate tension:
  `mesh.wgsl`'s already-implemented generic version of this accumulate shape
  gates by the *diffuse's* alpha, and `_cs`'s own alpha covers 93% of the
  texture (fully opaque only at the cross/chevron outline) - nothing like
  the ~7%, bars-only mask `_ne`'s alpha carries. Which alpha channel and
  which of {RGB, RGB×alpha, alpha-alone} actually governs a pad material is
  unresolved: the pad's own unit-1 sample lands in `R0`, gets unpacked as a
  normal and is threaded through a specular chain before the program's final
  blend, so register-tracing this pass did not confirm the canonical shape
  end to end for either pad. See `docs/rendering/pads.md`'s "The `_ne` file's
  RGB is a normal map, not paint" section for the full measurement,
  `crates/render/examples/hd_pad_ne_colour_probe.rs` and
  `hd_pad_ne_tint_probe.rs` for the probes. **Do not re-derive this** - the
  question left open is where the red the maintainer saw actually comes
  from, not whether this file's colour explains it.
- **Which alpha channel gates the accumulate - answered, asset-side,
  2026-09-07.** `_ne`'s own alpha, not the diffuse's. Extended
  `hd_pad_ne_tint_probe.rs` to print swizzles and write masks and traced
  `R0.w` (the raw `_ne` alpha, written once by the unit-1 `TEX` and never
  overwritten again) forward: it reaches an accumulate at instruction 41
  (`Speedup Pad`)/43 (`Weapon Pad`), `MAD dst.xyz, R0.wwww, C.xyzw,
  accumulated.xyzw`, identically shaped on both pad classes. The diffuse's
  own alpha (`H5.w`/`H0.w`) is read too, one step later, but as a
  **multiplier on the whole sum that already includes the `_ne`-gated
  term**, not a second independent gate - so the "generic shape would spread
  a glow across 93% of the plate" concern the previous pass raised does not
  apply: the pad's real formula is not that shape on either channel. Neither
  is it the generic `mesh.wgsl` shape at all - the pad's own program spends
  roughly forty instructions on two separate power-curve chains (a
  per-channel curve applied directly to unit 2's own sample, plus a genuine
  `N·H` specular exponent, both sharing the `LG2`/`MUL(32.0)`/`EX2` idiom
  `specular_exponent` already names elsewhere) between the `_ne` sample and
  the program's end, and the true final instruction adds a third,
  texture-free term built from constants and the specular scalar. **Unit 2
  is not an unbound fourth texture** - `program.declared.samplers` names it
  as `LIGHTMAP_SAMPLER`, the material's own already-known lightmap entry,
  the same one `skin::picks` already routes into the renderer's
  `aux`/`lightmap` binding today; the only genuinely unbound texture is
  `_ne` itself. **The colour patched into the `_ne`-gated term is identified and it is not
  red**: parameter hashes `0x7611a2d8` (`Speedup Pad`) and `0xce5c4410`
  (`Weapon Pad`) - previously flagged only as "not `emissive`'s tint" -
  both author the identical value on `12_sol_2`, `[0.0, 0.768628, 0.992157,
  0.0]` (RGB `(0, 196, 253)`, a light cyan/blue), shared between two
  otherwise-independent material files, which reads as a circuit rim/sky
  tint rather than a pad-specific glow colour. **This does not identify a
  cooldown state and does not need to**: the accumulate is still the same
  unconditional one already established, on 18 of 18 chunks. See
  `docs/rendering/pads.md`'s "Which alpha channel gates the accumulate"
  section for the full trace, including its `Weapon Pad`-only register-alias
  caveat (a `TEX H0` sits between `R0`'s write and its use there, where
  `Speedup Pad`'s equivalent fetch is `TEX H5` and has no such caveat) and
  the `Program::output_texels()` cross-check that agrees with the hand trace
  on both programs' output-alpha lane. **Do not re-derive this** - the
  question left open is still the same one: where the red the maintainer
  saw comes from, which needs the Ghidra-side vtable diff, not another
  asset probe.

## Open

- **The two pad types differ in colour, and this project measured them as
  identical - 2026-09-08, from play.** The maintainer reports **speed pads are
  cyan, weapon pads are red**. The register trace measured the patched colour
  as `(0, 196, 253)` cyan and reported it the same in both material files. Cyan
  is corroborated for the *speed* pad; the *weapon* pad's value is wrong.
  **The suspect is already named in `docs/rendering/pads.md`**: confidence was
  capped at 82 over a `Weapon Pad`-specific `R0` aliasing concern under NV40's
  H/R packing, and the weapon pad is exactly the one play says we read wrong.
  Re-derive const slot 58 straight out of the material record rather than
  through the register trace, and **do not hand-correct the value to red**.
- **The same report corroborates the finding that matters most**: only the
  light bars change state, which is the `_ne`'s own ~7% alpha and not the
  diffuse's 93%. That half of the trace is independently confirmed.

**Does an HD weapon pad look different while it is cooling down, and what
draws that? There is a state change - the maintainer reports it from play,
2026-09-07, and it is more specific than the recollection this row used to
carry.** Verbatim: HD weapon pads **light up red**, **go dark when they give a
weapon**, then **relight when arming** - and asked whether it is the whole pad
or only the bars, the maintainer's answer is **only the light bars**. That is a
report from play, not a
measurement off the original, and nothing may be tuned to match it - but it is
this project's most reliable class of lead and it names a trigger, which the
earlier "goes dark and then relights" did not.

Three things it changes:

0. **"Only the light bars" and "the emissive layer toggles" are still a
   plausible statement, but weaker than it looked before the colour check
   below.** The `_ne` mask's alpha is established as covering exactly the
   light bars, so a state change confined to the bars is *consistent with* a
   state change confined to what that mask selects. It is no longer the
   strongest read, though: the mask's own RGB is a normal map and neither
   pad's shader carries any tint at all (see "What is closed" above), so if
   the emissive layer is the whole mechanism, its lit colour cannot come from
   this material record or this file - it would have to come from somewhere
   else the runtime feeds in, which is exactly what steps 2-3 below would
   find.
1. **The trigger is the grant, and the relight is the refresh.** That maps onto
   a `WeaponPad_UpdateRefreshTimer`-shaped per-frame update, which is exactly
   what step 2's vtable diff is for. It also means the play capture the old
   version of this row asked for is no longer the cheapest way to establish
   *whether* there is a state change - there is one.
2. **"Red" is asset-side settled, and it is a negative result.** The `_ne`
   mask's alpha covers the light bars; its RGB is a tangent-space normal map,
   not paint, and no tint - baked in the microcode or driven by a material
   parameter - exists anywhere in either pad's own shader (see "What is
   closed" above for the numbers). So the lit state is **not** "the emissive
   layer measured below, drawing at `emissive`'s default white tint over red
   texels" - that hypothesis is closed. Either an unmeasured runtime state
   feeds a colour in from outside this material record (the vtable diff,
   step 2 below, is the way to find it), or the red the maintainer describes
   comes from a different mechanism than this emissive layer entirely, or -
   the possibility this project's own history warns to keep live - the
   recollection is of a different title's pad. Nothing here decides between
   those three.
3. **It does not contradict "a speed pad is never recoloured on any title".**
   That closed bullet is about *speed* pads and about *recolouring*. An
   emissive layer toggling on and off is a different mechanism, and the two can
   both be true. Do not reopen the closed bullet on the strength of this;
   settle which mechanism it is first.

Nothing read in the executable says so yet.

Three things the disc authors that this project does not touch:

1. **A per-circuit emissive mask.** `ds_speedup_ne.gtf` and
   `ds_weaponup_ne.gtf` sit beside the `_cs` colour files on all eleven
   circuits, and **the alpha channel of each is a mask over exactly the pad's
   light bars** - the little rectangles along the chevron and the cross, and
   nothing else. Reproduce: decode the `.gtf` through
   `oag_formats::gtf::Texture::to_rgba` and write the alpha channel out as
   grey.
2. **Two standalone pad materials per circuit**,
   `materials/speedup_material.rcsmaterial` and `materials/weapon_pads.rcsmaterial`,
   duplicated again under `materials_dlc/` in `DATA03.PSARC`. 110 pad-named
   entries across the disc. `weapon_pads` on `12_sol_2` is 105,776 bytes and
   70 variants.
3. **Neither reaches the frame, and now the exact reason is measured rather
   than open.** Every pad chunk on `12_sol_2` resolves to one surface role,
   `0x00000059`: second texture is the circuit's lightmap atlas
   (`lmaps/ile_mesh_combine13-lmap.gtf`), `ADD_SECOND` clear, `NO_AMBIENT`
   set. That much is still true, but "neither `_ne` file is among the second
   textures the pad model loads" undersold it: the material *names* the `_ne`
   file at `+0x78`/`+0x60`, it just is not what gets **bound**. Each pad
   material record carries **three** sampler entries, not the two this
   renderer's `Pick` has room for - `[0]` the `_cs` diffuse, `[1]` the `_ne`
   mask (the file's own `second_texture`), `[2]` the lightmap - and
   `oag_render::mesh::rcs::skin::picks` (`crates/render/src/mesh/rcs/skin.rs`)
   short-circuits on `Material::lightmap_entry().is_some()`: "the lightmap
   wins the second binding, wherever it sits" grabs entry `[2]` for the
   renderer's one `aux` slot and entry `[1]` is never looked at again. That
   short-circuit exists on purpose, to fix Talon's Junction's baked lighting
   landing on a non-first entry, and it is doing exactly what it was built to
   do here too - it just also drops a legitimate second thing the material
   author put in slot `[1]`.
   **The `_ne` binding is not idle microcode either.** Resolving each pad
   chunk's own shader variant the way `skin::variants` does (`Class::Static`,
   the ordinary lit-race pass) and reading its declared samplers: the `_ne`
   sampler's name hash (`0xa2d555b9`, shared by both pad types) binds
   fragment unit 1, and `Program::accumulates(1)` is `true` - on **18 of 18**
   pad chunks measured (10 `Speedup Pad` + 8 `Weapon Pad`). No parameter
   patch gates it; each material carries exactly one `parameters` entry and
   neither pad's hash (`0x7611a2d8` speedup, `0xce5c4410` weapon) is the tint,
   offset or scroll-rate hash `oag_render::mesh::rcs::emissive` already reads,
   so a wired layer would run at `emissive`'s own defaults - tint
   `[1.0, 1.0, 1.0]`, no scroll. So the disc's own shader treats the light
   bars as a plain, unconditional additive layer, on every pad measured, with
   nothing that looks like a cooldown gate in the material record itself.

## Next Steps

In order of cost, cheapest first. Step 1 (finding the binding) is done; the
old step 0 (which alpha channel gates the accumulate) is done too, and
changed step 1's own cost estimate - see below. What is left is deciding
whether to spend on wiring it at all, and the two RE leads.

1. **Wire the emissive layer - and read the trap before touching it, twice
   over now.** This is *not* the cheap step it looks like, for two separate
   reasons. First, the one already known: the naive fix - make
   `skin::picks`'s `aux` slot land on sampler entry `[1]` instead of `[2]`
   for a pad material - is a regression, not a fix: it drops the lightmap
   binding entirely, and `docs/rendering/pads.md` already establishes what
   that does to an HD pad lit only by its lightmap with `NO_AMBIENT` set -
   flat blue-looking-correct turns near-black. **The existing guard would
   not catch it either**:
   `crates/render/tests/pad_alpha_test_ground_truth.rs::an_hd_speedup_pad_draws_visible_pixels`
   counts *lit pixels*, and an added glow layer adds pixels even while the
   lightmap it silently traded away was worth more of them - a smaller,
   dimmer, wrongly-lit pad can still clear a lit-pixel floor. The correct
   shape needs **both** textures bound at once, which this renderer's
   architecture does not have room for: `Pick` carries exactly one `aux`
   entry, `skin()` returns exactly two `TextureSlots` (`textures`,
   `lightmaps`), and `mesh.wgsl` binds exactly two texture units per
   material. Wiring this for real is a third-texture-per-material change -
   `Pick`, `skin()`'s return shape, `Model`, `mesh_render::build`'s bind
   group, `mesh.wgsl`, and a new `slots` flag so the shader knows a material
   has one - touching the bind-group layout every model in the crate uses,
   not a pad-only change.

   **Second, and new as of 2026-09-07's channel trace: the pad's own
   fragment program is not `mesh.wgsl`'s generic `glow = second.rgb *
   tint.rgb * first.a` shape at all**, on either alpha channel - it reads
   `_ne`'s alpha to gate one additive term that is itself scaled by the
   diffuse's alpha and then by a specular scalar before reaching the
   output, plus two separate power-curve chains (one against the lightmap
   sample directly, one a genuine `N·H` specular exponent) - see
   `docs/rendering/pads.md`'s "Which alpha channel gates the accumulate"
   section. The "second texture" those curves read is **not a fourth,
   unbound texture** - it is unit 2, and unit 2 is the material's own
   already-known lightmap (`Material::lightmap_entry()`'s own
   `LIGHTMAP_SAMPLER` hash, confirmed against the program's declared
   samplers), the exact entry `skin::picks` already routes into the
   renderer's `aux`/`lightmap` binding today. So the texture-slot count is
   still three, not four: `_cs` (already bound), the lightmap (already
   bound, just via the existing `aux` slot), and `_ne` (the one genuinely
   missing binding). **What is bigger than step 1's original estimate is
   the arithmetic, not the binding count**: landing the generic `emissive`
   shape - even pointed at the right channel - would draw a materially
   different picture than the disc's own formula, which needs the
   lightmap-power curve and the specular-exponent chain reproduced in
   `mesh.wgsl`, not just a third texture read. **If this is undertaken, say
   explicitly which of the two (a generic third-slot glow, or the disc's
   real formula) is being built, and prove it with pictures at each step** -
   they are not the same feature, and building the first is not a step
   toward the second.
2. **Diff the `Pad_Importer` vtables.** `WeaponPad_Importer`'s object takes
   its vtable from `0x0086a730`, seventeen slots. The adjacent table at
   `0x0086a780` shares fourteen and differs at slots 0, 3 and 5. A
   class-specific per-frame update sits in one of those three - that is how
   `WeaponPad_UpdateRefreshTimer` was found on PSP. Both constructors are
   below `0x32d5e0`, so **Ghidra's TOC is the right one here** and data
   references can be read directly (`docs/ghidra/functions/ps3-hdfury-eu/memory.md`).
3. **Resolve `WeaponPad_Importer`'s `this+0x1b0` vector.** Its constructor
   (`0x002e0210`) loads it through TOC slot `0x008b431c`, whose value
   `0x00aec2c0` is past the image's last section (`0x009356ff`) - a `.bss`
   global with no bytes in the file. `SpeedupPad_Importer` (`0x002dd340`) has
   no such store, so it is class-specific. Finding its writer needs either a
   whole-image scan for the address or a live read; the patched RPCS3 with
   working write watchpoints (`just build-rpcs3-watchpoints`) is the tool for
   the second.
4. **Only then implement a state change, if 2-3 find one.** Until they do, an
   HD pad drawing the same in both states is the correct answer, not a
   placeholder to improve on. Do not add a cooldown grey by analogy with Pulse
   - the two titles' pads are already established to differ in mechanism,
   since Pulse's texture is neutral and HD's is painted. And per step 1's
   measurement: the material's own shader accumulates the light-bar layer
   unconditionally on every chunk read, with no parameter that looks like a
   gate, and (2026-09-07) neither its RGB nor any tint anywhere in either
   pad's shader is red - so if 2-3 do find a cooldown state, it is not
   sitting in this material record, this file or this shader, and it is not
   yet clear it is the same mechanism as the emissive layer at all. Wire
   whatever colour 2-3 find as its own thing rather than assuming it slots
   into `emissive`'s tint.

A play capture would settle the *observable* half quickly, but is not this
project's cheap step right now: this desktop has no working GUI windows and
RPCS3 screenshots are known to come back black here (see the memory note on
verifying headless), so a capture needs a machine with a working display -
recommend it to whoever picks this thread up next with one, rather than
attempting it blind. Race on `12_sol_2`, cross a weapon pad, and compare the
pad in the frame before and a second after: that answers "is there a state
change at all" without reading a single instruction, and decides whether
steps 2-3 are worth their cost.
