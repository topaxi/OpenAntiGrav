# HD's pads author an emissive mask and a material of their own; nothing reads either

2026-09-03. Fell out of fixing the invented pad tint (`docs/rendering/pads.md`,
commit "fix(render): a pad draws the colour its own texture paints"). That fix
is complete and this file is only what it did **not** close.

## What is closed, so nobody re-derives it

- A speed pad is **never recoloured on any title**. `Pad_UpdateRefreshTimer`
  (`0x089265f0`) writes no colour, Pulse's gold and HD's blue are both in the
  texture, and `oag_render::speedup_pad` is deleted rather than gated.
- Pulse's `Weapon Pad` cycle (`pad+0x6c`, table `0x08ac00c8`) stays exactly as
  recovered on PSP/PS2 and does not run on HD. **HD runs its own cycle, and it
  is wired as of 2026-10-08 (`hd-weapon-pads`)**: the light bars' colour is
  `WeaponPad_UpdateRefreshTimer`'s per-pad cycle, red, written into the pad
  program's constant every frame - measured on RPCS3, `docs/rendering/pads.md`
  "The red is a cycle". `Title::weapon_pad_glow` carries it. The material's
  authored `W_Cycle` is not what the original draws.
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

- **Settled 2026-09-16, commit `91071e60` - do NOT re-derive.** The colour
  question above (2026-09-08) is closed and its "the weapon pad's value is
  wrong" framing is stale. `crates/render/examples/hd_pad_colour_census.rs`
  re-derives const slot 58 (`0xce5c4410`, named `W_Cycle`) directly out of
  `Material::parameters` across all 12 circuits that ship pad geometry, not
  through the register trace: it is each pad's own **per-instance authored**
  value, not a shared circuit tint and not a fixed per-type constant. `Weapon
  Pad` is red (`[1.0, 0.0, 0.0, 0.0]` on `talons_junction`, this project's
  default circuit) on `talons_junction`, `tech_de_ra`, `modesto_heights` and
  `15_anulpha_pass`, cyan/blue on the other 8; `Speedup Pad` is cyan (or
  near-white on `15_anulpha_pass`) everywhere it has geometry at all.
  Confidence 88. This matches the maintainer's play report on the circuits
  that are red - the aliasing concern that capped the register trace at 82
  was a red herring; the value itself was never wrong, only measured on a
  circuit (`12_sol_2`) where it happens to be cyan for both pad types. See
  `docs/rendering/pads.md`'s "Corrected 2026-09-16" section for the full
  table.
- **Vtable diff done, 2026-09-17 - `docs/ghidra/functions/ps3-hdfury-eu/pads.md`,
  do NOT re-diff.** `WeaponPad_Importer`'s per-frame update
  (`WeaponPad_UpdateRefreshTimer`, `0x002e02b8`, vtable slot 3) *is* a runtime
  colour cycle - a flat grey while cooling, a 6-entry keyframe cross-fade once
  armed, structurally identical to PSP's function of the same name. It writes
  the cycling colour to two fields on the pad's own C++ object
  (`this+0xf0`/`this+0x1b0`), not into the `.rcsmaterial` parameter found
  above - so there are genuinely two separate colour mechanisms on an HD
  weapon pad, not one. **What is still open**: whether either object field
  reaches the renderer at all (traced to nothing downstream this pass) and
  who writes the `.bss` global (`0x00aec2c0`) the cooling branch reads its
  neutral colour from. See Next Steps items 2-4 below for the detail.
- The maintainer's report that only the light bars change state, not the
  whole pad, is corroborated by the `_ne`-alpha finding (~7% of the texture,
  bars-only) - that half of the trace is independently confirmed and does
  not need re-checking.

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

   > **Corrected 2026-09-16, commit `91071e60`.** The paragraph above is
   > wrong about "cannot come from this material record" - see the `## Open`
   > section's first bullet. The `_ne`-gated term's own colour constant
   > (`0xce5c4410`, `W_Cycle`) *is* authored per material instance and *is*
   > red on `talons_junction`. The lit colour comes from exactly this
   > material record after all; what is still unrecovered is only whether
   > anything toggles the accumulate on and off at runtime.
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

   > **Reversed 2026-09-16, commit `91071e60`.** "Negative result" was wrong:
   > `0xce5c4410` (`W_Cycle`) is a material-authored tint after all, and its
   > value on `talons_junction` is `[1.0, 0.0, 0.0, 0.0]` - red. It reads at
   > 82 confidence through the register trace only because the trace was run
   > on `12_sol_2`, where the same parameter happens to author cyan; reading
   > the parameter directly (`Material::parameters`, no register trace) across
   > all 12 circuits with pad geometry gets 88 and shows it varies per
   > circuit. None of the three alternatives this bullet named is what
   > happened - the colour was always in this material record, just not on
   > the one circuit measured first. Only the *toggle* (does anything gate
   > the accumulate on/off at runtime) is still open.
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
   `oag_texture::gtf::Texture::to_rgba` and write the alpha channel out as
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
   `oag_mesh::mesh::rcs::skin::picks` (`crates/mesh/src/mesh/rcs/skin.rs`)
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
   offset or scroll-rate hash `oag_mesh::mesh::rcs::emissive` already reads,
   so a wired layer would run at `emissive`'s own defaults - tint
   `[1.0, 1.0, 1.0]`, no scroll. So the disc's own shader treats the light
   bars as a plain, unconditional additive layer, on every pad measured, with
   nothing that looks like a cooldown gate in the material record itself.

## Next Steps

0. **2026-10-05, open: the fragment decoder now keeps source negate (bit 17)
   and the condition field (commit `6428eae0`), and readings made before that
   may move.** The pad's "specular scalar" turned out to be EXP2 fog once the
   negate was honoured. Re-check, without re-deriving the pad itself, every
   earlier conclusion that read decoded HD fragment programs:
   `Program::specular_exponent` and `Program::dp3_feeding` (`crates/mesh/src/mesh.rs`,
   `crates/rcs/tests/specular_power_ground_truth.rs`) and the `hd_specular_*`,
   `hd_litex2_census` and `hd_dp3_feeding_lane_check` examples under
   `crates/render/examples/`. The gate stayed green across the change, so no
   asserted value moved; what is unchecked is the prose readings built on
   those examples' output.

In order of cost, cheapest first. Step 1 (finding the binding) is done; the
old step 0 (which alpha channel gates the accumulate) is done too, and
changed step 1's own cost estimate - see below. What is left is deciding
whether to spend on wiring it at all, and the two RE leads.

1. **~~Done 2026-10-05~~ - wired; see `docs/rendering/pads.md`, "Wired, 2026-10-05". The multiply this item feared was EXP2 fog (the decoder had dropped the negate bit), `_ne` is bound third beside the lightmap, and the disc's own normal/glow terms are in `mesh.wgsl`. Original-four speed pads: also wired 2026-10-05 (hd-speed-pads), routed by pad material in `build_scene` because their node hashes name no chunk; see pads.md "Wired, 2026-10-05, the four original circuits' speed pads". Still open: the runtime cooldown colour, and which node owns which speed-pad chunk.** Old text follows. Attempted 2026-09-25, stopped before any code change - do not
   re-attempt without first reading `docs/rendering/pads.md`'s "Wiring
   attempted and stopped" section.** The question this item's own "read the
   trap before touching it, twice over" text asked - does the `_ne`-gated
   term reach the program's output through ADDs alone, or through a multiply
   - is now settled by a register-level trace of both pad programs on
   `12_sol_2`, reproducible from `hd_pad_ne_tint_probe.rs`: **through a
   multiply.** The term is only ever added into the sum it shares with the
   diffuse-alpha-gated term (the earlier "Which alpha channel gates the
   accumulate" section's own equation was already right about that), but
   that whole sum is then multiplied by the program's specular scalar at the
   program's true final instruction, before the last additive term. Wiring
   it unscaled would not be the disc's real term.

   **What blocks reproducing the scalar is not effort, it is two opcodes
   this project has already looked at and declined to name.** Both chains
   (`Program::specular_exponent`'s own `LG2`/`MUL`/`EX2` idiom) pass through
   opcodes `0x3b`/`0x3d`, which `Program::name()` returns `None` for and
   `docs/formats/rcsmaterial.md` already records as genuinely absent from
   nouveau's opcode table, not merely unported - `renderer.md` reads `0x3b`
   as a normalise/rsq helper at confidence ~70 and deliberately does not
   apply that reading, per this project's own rename-confidence line. Naming
   it anyway to get a picture out of this pass would be exactly the guess
   `CLAUDE.md` asks not to make. A second, smaller blocker sits behind the
   first: the `N` in this chain's `N.H` is `_ne`'s own decoded tangent-space
   normal, and `mesh.wgsl` has no tangent frame to build it from yet
   (screen-space derivatives of `in.world`/`in.texcoord` are the cheaper of
   the two ways to add one, needing no new vertex attribute).

   **Nothing wired, nothing regressed** - `skin::picks`, `Pick`,
   `TextureSlots`, `material_bind_group_layout` and `mesh.wgsl` are all
   unchanged. The two traps this item already named (trading the lightmap
   away, and reproducing the wrong generic shape) are both still live for
   whoever unblocks the opcodes; a bind-group change with no correct combine
   to put in it would be a third failure of the same shape. The original
   text below is kept for the plumbing shape it still correctly describes,
   once the opcodes are named.

   > **The opcode blocker is cleared, 2026-09-25 - do not re-attempt naming
   > `0x3b`/`0x3d`.** RPCS3's own `FPOpcodes.h` (GPLv2, independently
   > reverse-engineered against real hardware) names both:
   > `RSX_FP_OPCODE_DIVSQ = 0x3B` (`a / sqrt(b)`) and `RSX_FP_OPCODE_FENCT =
   > 0x3D` ("Fence T?"). `DIVSQ` resolves the exact contradiction that kept
   > `op3B` below the rename line (it produces both the normalize idiom and
   > the same-register square-root idiom `renderer.md` had found
   > irreconcilable under a dedicated `NRM`), and a disc-wide census
   > (`crates/render/examples/hd_op3b_op3d_census.rs`, 1,632 files, 76,358
   > fragment blocks) confirms `0x3d` writes no real destination on every one
   > of 59,256 uses - an exact invariant, not a sampled rate. Confidence 84
   > (`0x3b`/`DIVSQ`) and 90 (`0x3d`/`FENCT`, "no data effect"). Both pad
   > programs' specular chains use only `0x3b`, never `0x3d` - traced with
   > `hd_pad_ne_tint_probe.rs`'s raw-opcode output. **What is still open,
   > unchanged by this**: the tangent frame `mesh.wgsl` needs to build `N` for
   > this chain (this item's own second, smaller blocker, still unaddressed),
   > and the bind-group/third-texture change below, which still depends on
   > that tangent frame landing first. See `docs/rendering/pads.md`'s "Item 1
   > cleared, 2026-09-25" section and `docs/formats/rcsmaterial.md` for the
   > full evidence. No code wired; `Program::dp3_feeding` is deliberately
   > left unrelaxed, a separate change for whoever picks this up next.

   Original text, still the plumbing shape once unblocked - and read the
   trap before touching it, twice over now. This is *not* the cheap step it
   looks like, for two separate reasons. First, the one already known: the
   naive fix - make
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
2. **Done, 2026-09-17 - `docs/ghidra/functions/ps3-hdfury-eu/pads.md`.** The
   vtable diff is done mechanically rather than assumed, and the "shares
   fourteen and differs at slots 0, 3 and 5" claim above is wrong: nine of
   seventeen slots match, eight differ (0, 3, 5, 12, 13, 14, 15, 16), and
   slot 3 is the class-specific per-frame update. It decompiles as
   `WeaponPad_UpdateRefreshTimer` (`0x002e02b8`) - PSP's function of the same
   name, reimplemented almost exactly: a refresh timer at `this+0x160`, the
   identical flat grey `0x3f3f3f` while cooling down, and a 6-entry keyframe
   cross-fade once armed, written to `this+0xf0` (packed RGBA) and
   `this+0x1b0` (float vector). `SpeedupPad_Importer`'s own slot 3 is an
   unrelated one-argument accessor with no colour logic at all. See that page
   for the full trace and the four reproducer scripts under `scripts/ghidra/`.
3. **Partly answered by the same pass.** `WeaponPad_Importer`'s `this+0x1b0`
   vector is not a spatial bounding-box value - `WeaponPad_UpdateRefreshTimer`
   writes the same cycling colour there (as a float vector) that it packs
   into `this+0xf0` (as bytes), so the constructor's own initial write to
   `this+0x1b0` is this field's cooling-down default, not box geometry. The
   narrower question - who writes the sixteen bytes at the `.bss` global
   `0x00aec2c0` itself (the neutral vector `WeaponPad_UpdateRefreshTimer`
   reads while cooling) - is still open: a whole-image literal scan finds
   exactly one TOC slot referencing it (`0x008b431c`) and five functions that
   *read* through that slot, none of which write to the global's own storage.
   A live read (`just build-rpcs3-watchpoints`) is still the way to find a
   writer, if one exists at all rather than the global staying at its `.bss`
   zero-fill.
4. **The framing here was backwards, and is corrected now rather than
   repeated.** "Do not add a cooldown grey by analogy with Pulse - the two
   titles' pads are already established to differ in mechanism" is wrong:
   they do not differ in mechanism. HD's `WeaponPad_UpdateRefreshTimer` runs
   the same grey-while-cooling / keyframe-cross-fade-when-armed shape as
   Pulse's, at the same magic constant and the same 6-entry table size - the
   two titles differ only in whether the pad's own texture needed recolouring
   in the first place (Pulse's `weapon_under.tga` is neutral; HD's
   `ds_weaponup_cs.gtf` is already painted). **What is still genuinely open,
   and is the real next step**: this function writes `this+0xf0` /
   `this+0x1b0` on the pad's own C++ object, and nothing in this pass traces
   where either field goes afterward - neither is `GpuVertex::colour` (flat
   `0,0,0` on every measured chunk) or a `.rcsmaterial` parameter. If a
   consumer patches a shader constant from one of these fields, that is the
   runtime mechanism the material's static `W_Cycle` colour (settled
   2026-09-16, commit `91071e60`) would ride on top of; if nothing reads
   them, this is a second "authored but unwired" gap, the same shape as the
   emissive mask itself. **Do not wire a cooldown grey by guessing which of
   these it is** - trace the consumer first.

**A play capture was attempted 2026-09-17 and did not land a weapon-pad
closeup.** The RPCS3 capture harness itself works now (`scripts/rpcs3-drive.py`'s
`open_session` fix landed 2026-09-17; screenshots are not black) - three
captures on `talons_junction` (6, 10 and 16 shots, 1-3 s apart, default
campaign walk) show cyan speed pads clearly (corroborating the material
finding) but never caught the ship crossing a weapon pad closely enough to
read its state, despite one capture showing a weapon (`Machine Gun`) get
picked up between frames. This is a sampling-luck gap, not a tooling one -
whoever picks this up next should either script tighter interval sampling
around a known pad position (`Data\Environments\Talons_Junction\track.vex`'s
own pad nodes would give the exact in-lap position/time to target) or accept
that the Ghidra-side answer above is cheaper than hunting for the right
frame.

## From the HANDOVER.md index (moved 2026-09-25)

**What binds the `_ne` file is now answered, asset-side, 2026-09-07**: the pad material is an inline `.rcsmodel` record, not a separate file, and it genuinely names `ds_speedup_ne.gtf`/`ds_weaponup_ne.gtf` as its own `second_texture` - but the record carries a *third* sampler entry (the lightmap), and `skin::picks`'s "the lightmap wins the second binding, wherever it sits" rule grabs that instead, so the `_ne` entry is never bound. Measured off the actual shader too: the `_ne` sampler binds fragment unit 1 and `Program::accumulates(1)` is `true` on 18 of 18 pad chunks on `12_sol_2`, unconditionally - no parameter patch gates it. `speedup_material.rcsmaterial` turned out to be an authored orphan on this circuit; the `Speedup Pad` chunks name a generic material instead. **Wiring it is not cheap**: the renderer has room for only one second texture per material, so the naive fix trades the lightmap away for the glow - a near-black regression the existing pixel-count guard would not catch, since an added glow layer adds lit pixels even while losing more of them to the dropped lightmap. A real fix needs a third texture slot reaching every model's bind group. **Maintainer report from play, 2026-09-07, more specific than before**: HD weapon pads light up red, go dark on grant, relight on refresh - only the light bars, not the whole pad. **Checked asset-side the same day, and it is a negative result**: the `_ne` file's RGB is a tangent-space normal map (whole-texture mean `[127,129,243]`/`[127,128,244]`, confirmed by the shader's own `x*2-1` normal-decode right after the sample), not paint, and neither pad's fragment program carries a tint anywhere - baked or parameter-driven. So nothing measured explains "red" for the light bars; that hypothesis is closed, not open. **Which alpha channel gates the accumulate is now answered too, same day**: `_ne`'s own alpha, not the diffuse's - traced with the fragment program's own swizzles and write masks, which the probe now prints. `_ne`'s raw alpha (untouched from its `TEX` sample to instruction 41/43) gates an additive term identically on both pad classes; the diffuse's alpha only multiplies that already-accumulated sum one step later. Neither channel runs the renderer's generic `mesh.wgsl` shape at all - the pad's own program runs two power-curve chains (one against the lightmap sample directly, one a genuine `N·H` specular exponent) as well as `_cs` and `_ne`; the "second texture" those curves read is unit 2, which is the material's own already-known lightmap (`LIGHTMAP_SAMPLER`), already bound today via `skin::picks` - not a fourth, unbound texture, so the slot count stays three. `Program::output_texels()` independently agrees with the hand trace on both programs' output-alpha lane, and a `Weapon Pad`-only register-aliasing caveat (a `TEX H0` sits between `R0`'s write and its use, unlike `Speedup Pad`'s `TEX H5`) is recorded rather than glossed over. The colour patched into the `_ne`-gated term is now read too: `[0.0, 0.768628, 0.992157, 0.0]`, a light cyan/blue, identical between the `Speedup Pad` and `Weapon Pad` material files - not red, and shared rather than pad-specific, which reads as a circuit rim tint. **Corrected 2026-09-08, from play**: that colour is right for the *speed* pad (cyan, confirmed) but wrong for the *weapon* pad, which the maintainer reports as red - the thread's own confidence-82 cap over a `Weapon Pad`-specific register-aliasing concern was the named suspect and now looks like the actual bug. What is still open is where the red actually comes from, which needs the Ghidra-side vtable diff this thread's Next Steps already named, not another asset probe - and wiring anything is now known to need reproducing that arithmetic, not just a third texture slot, since the disc's own formula is not the generic emissive shape either

## Update 2026-10-08 (`hd-weapon-pads`): the red is landed, what stays open

Closed: where the red comes from, and wired (`docs/rendering/pads.md`, "The red is a
cycle"; `docs/ghidra/functions/ps3-hdfury-eu/pads.md`). The "unmeasured runtime
state" candidate was the right one: the cycle position and the cooling vector are
pad-object state the engine copies into the fragment program's constant.

Open, in order:

1. **A ready pad on a second circuit, and a cooling pad on Vineta K.** Seen: two
   ready pads (Vineta K, two phases of one cycle) and one cooling pad (Sol 2).
   Cheapest: `place` with a pose clear of every pad on arrival, so the pad is not
   collected by the placement (the Sol 2 and the second Vineta captures were).
2. **The code that carries `pad+0x1b0` into the constant** (material parameter
   `W_Cycle`, `0xce5c4410`): no `lis 0xce5c` immediate exists, so it is indirect.
   The wiring rests on the numbers fitting, not on a traced store.
3. **The cycle's real rate against video**: 3 keys a second is read from
   `DAT_008b4324`, not timed on frames.
4. **The `_ne` normal-map lighting terms** of the pad program (the specular chain)
   are still the renderer's own `pad_normal`, not the program's; unchanged here.
