# Speed pads and weapon pads: what colours one

Two `.vex` classes, `Speedup Pad` `0x3bd` and `Weapon Pad` `0x3be`, and one
question this page exists to answer for both on every title: **where does a
pad's colour come from?**

The trigger side is [`pads.md`](../ghidra/functions/psp-pulse-usa/pads.md); the
payload layout is [`formats/pads.md`](../formats/pads.md); how a pad's geometry
gets into a model at all is `oag_mesh::mesh::build_pads` on PSP/PS2 and
`oag_mesh::mesh::rcs::pads` on PS3. This page is only about pixels.

## The answer, per title and per class

| Title | Class | Colour comes from | Recovered |
| --- | --- | --- | --- |
| Pulse (PSP/PS2), Pure | `Speedup Pad` | the texture, unmodified | 90 |
| Pulse (PSP/PS2), Pure | `Weapon Pad` | `pad+0x6c`, a per-tick runtime write | 85 |
| HD / Fury | `Speedup Pad` | the texture, plus a per-circuit-authored additive glow tint (cyan on every circuit measured) | 88 |
| HD / Fury | `Weapon Pad` | the texture, plus a per-circuit-authored additive glow tint - red on `talons_junction`, `tech_de_ra`, `modesto_heights`, `15_anulpha_pass`, cyan/blue on the other 8; any armed/cooling state change is **unrecovered** | 88 |

All four rows agree on the finding worth carrying: **on every title measured,
a pad's colour is authored by the artists and the engine leaves it alone -
Pulse's runtime write included, since even that reads a fixed keyframe table
rather than computing anything.** HD's two rows are authored in two places at
once, not one: the diffuse texture paints a fixed cross/chevron outline, and
the `_ne`-alpha-gated additive glow (the light bars) is tinted by a per-material
parameter authored per circuit - see "Corrected 2026-09-16" below for the
per-circuit table. Nothing on HD is a per-tick write like Pulse's `Weapon Pad`;
"per-circuit-authored" means baked into the `.rcsmaterial` file, read once.

## A speed pad is never recoloured, on any title

`Speedup Pad`'s own class-table `update` slot is `Pad_UpdateRefreshTimer`
(`0x089265f0`, PSP Pulse). It reads, in full,
`pad->0x1a0 = max(0.0, pad->0x1a0 - dt)` - and nothing anywhere in the
executable ever writes a `Speedup Pad`'s `+0x1a0` non-zero, so even that is
inert. There is no colour write, no ready state and no cooldown state on the
class.

What the texture carries instead, measured on the discs:

| Title | Entry | What it paints |
| --- | --- | --- |
| Pulse | `flicker1nonalpha_GLOW.tga`, over `whitenonalpha.tga` | the gold chevron |
| Pure | `speedup_GLOW_KEY.tga` | the same; its batches ask for alpha reference `0`, which is what keeps the glow visible - see [`vex.md`](../formats/vex.md#a-batchs-own-attributes-decide-its-pipeline-not-which-list-it-came-from) |
| HD / Fury | `ds_speedup_cs.gtf`, 1024x1024 | a grey plate with a **blue** chevron outline |

Pulse's pad vertices carry a flat white (`255,255,255,255` on PSP, `253` on
PS2 - the PS2 exporter's own quantisation) plus one dark warm accent
(`34,34,17`), so the file's own vertex colour passes the texture through
rather than tinting it. HD's carry a flat `0,0,0`, which on that title is not
a tint at all - see below.

## What Pulse's `Weapon Pad` does instead, and why it is the exception

`WeaponPad_UpdateRefreshTimer` (`0x0892c034`) packs a flat `0x3f3f3f` grey into
`pad+0x6c` while the pad is cooling down, and cross-fades that slot through a
6-entry keyframe table at `0x08ac00c8` at three keys a second once the refresh
timer reaches zero. `pad+0x6c` is a full colour *replacement*: `Pad_Bind` calls
`Mesh_Bind` first, which builds the GE colour commands from the file's own
materials, and the timer overwrites that slot every tick regardless.

That is reproduced in `oag_render::weapon_pad`, which carries the table, and
applied by `Drawable::tint_weapon_pads`. The pad's own texture,
`weapon_under.tga`, is neutral, which is what makes a full replacement the
right shape here and the wrong shape everywhere else on this page.

## On Wipeout HD, `colour` is a light, not a tint

**This is the trap, and it is worth stating plainly because it already cost
one shipped regression.**

On a PS3 `.rcsmodel` chunk, `GpuVertex::colour` holds HD's baked per-vertex
light - the `f[TC1]` term the fragment program **adds** into its authored
lighting sum before multiplying the albedo. `Model::vertex_colour_is_light`
is the flag that says so, and `mesh.wgsl` reads it. Writing a palette entry
into that slot is not "the wrong colour"; it is a different physical quantity.

`12_sol_2` authors ten `Speedup Pad` and eight `Weapon Pad` chunks and every
vertex of all eighteen carries `0,0,0` there - HD's "this chunk is lightmapped,
it has no vertex light of its own" value. The colour is in the texture:

| Entry | What it paints |
| --- | --- |
| `ds_speedup_cs.gtf` | a grey plate, blue chevron outline; most saturated texel `58,113,99` |
| `ds_weaponup_cs.gtf` | a grey plate, **red** cross outline; most saturated texel `167,54,60` |

So an HD weapon pad is red because the artists painted it red, not because
anything cycles it.

### And every circuit authors a second pad texture, and a pad material of its own

Beside each `_cs` file sits a `ds_speedup_ne.gtf` / `ds_weaponup_ne.gtf`, and
**its alpha channel is a mask over exactly the pad's light bars** - the little
rectangles laid out along the chevron and the cross, and nothing else. That is
an emissive mask, authored per circuit, for precisely the elements that would
light up and go out.

Each circuit also ships two standalone material files carrying a pad name:

```
/data/environments/<circuit>/materials/speedup_material.rcsmaterial
/data/environments/<circuit>/materials/weapon_pads.rcsmaterial
```

Both are duplicated again under `materials_dlc/` in `DATA03.PSARC`. 110 entries
across the disc carry a pad name, on all eleven circuits. **Only one of the two
is what a pad chunk actually names, on `12_sol_2` at least - measured, not
assumed, 2026-09-07.** `weapon_pads.rcsmaterial` is exclusive to the `Weapon
Pad` chunks: exactly 8 material records in the whole model name it, and
`12_sol_2` authors exactly 8 `Weapon Pad` chunks. `speedup_material.rcsmaterial`
is named by **zero** material records in this model; the ten `Speedup Pad`
chunks all share one slot naming the generic
`materials/diffuse_normal_specular_emmissive.rcsmaterial` instead - a name
that says nothing pad-specific and is presumably reused elsewhere on the
circuit for ordinary geometry. So `speedup_material.rcsmaterial` reads as an
authored orphan here, at least on this circuit, and does not explain its own
existence the way it looked like it might.

**None of that reaches the frame yet, and the reason is now the precise
mechanism, not an open question.** Every pad chunk on `12_sol_2` resolves to
one surface role, `0x00000059`: second texture is the circuit's **lightmap**
atlas (`lmaps/ile_mesh_combine13-lmap.gtf`), `ADD_SECOND` clear, `NO_AMBIENT`
set. That much still stands, but "neither `_ne` file is among the second
textures the pad model loads" undersold it - see "What binds the `_ne` file"
below, which replaces that framing with the actual answer.

### The regression this replaced

Between 2026-09-02 and 2026-09-03 this project drew a flat, invented blue over
every speed pad on every title, and drew Pulse's recovered `Weapon Pad`
keyframe cycle on HD's weapon pads too. Both were reported from play: PS2
Pulse's speed pads had stopped being gold, and HD's weapon pads had started
cycling through colours they never cycle through.

The blue was documented at the time as a deliberate gameplay-clarity choice
with no disc evidence behind it. It was also, on HD, only *visible* because
the pad builder took both pad models off the authored lighting path on purpose
- `vertex_colour_is_light` forced `false` and every pad vertex's `lit` forced
`0.0`, which routes `mesh.wgsl` to its stand-in shading branch, where vertex
colour does multiply. A pad lifted out of the circuit's own light rig and
repainted flat is exactly the "plausible-looking stand-in" failure
[`CLAUDE.md`](../../CLAUDE.md) names, and the tell was there in the code: the
comment recording that the tint had been *invisible* until the lighting path
was changed to make room for it.

Both are gone. `oag_render::speedup_pad` is deleted rather than gated,
`Drawable::tint_weapon_pads` returns early on any model with
`vertex_colour_is_light`, and
`crates/game/tests/hd_pad_illumination_ground_truth.rs::hd_pads_keep_their_authored_light_and_are_never_recoloured`
pins all three facts against the disc.

### And putting HD's pads back on the lit path had to be checked as a picture

Restoring `vertex_colour_is_light` is not free: a pad's brightness is then
*entirely* albedo times lightmap, with no ambient floor - vertex light is a
flat `0,0,0` and `NO_AMBIENT` is set. Had the circuit's lightmap not reached
those chunks, or had their `lightmap_texcoord` fallen back to `[0.0, 0.0]`
(which `mesh::rcs::emit` does silently), HD's pads would have gone from flat
blue to **near-black**, and every attribute assertion above would still have
passed. That is the same shape as the failure
[`HANDOVER.md`](../../HANDOVER.md) already records for Pure's pads: a correct
triangle count and an empty picture.

Checked as a picture, offscreen, no window: one isolated plate per title
through `capture::capture_from`, plus
`crates/render/tests/pad_alpha_test_ground_truth.rs::an_hd_speedup_pad_draws_visible_pixels`
as the standing guard (559 lit pixels for `12_sol_2`'s ten plates, against
Pure's 321 for fourteen). What the plates show:

| Title | Speed pad | Weapon pad |
| --- | --- | --- |
| PS2 / PSP Pulse | gold chevron | neutral white cross, coloured by `pad+0x6c` in a race |
| HD / Fury | grey plate, blue chevron outline | grey plate, red cross outline |

An HD pad draws, and it draws the picture the artists painted. What it does
not draw is the light bars - see the open question below.

## The one open question

**Does an HD weapon pad look different while it is cooling down?** Pulse's does
- flat `0x3f3f3f` grey - and HD keeps Pulse's whole importer-per-class layout
([`renderer.md`](../ghidra/functions/ps3-hdfury-eu/renderer.md)), so a
counterpart is plausible. Nothing has been read that says it exists.

The maintainer's own recollection from play is that an HD weapon pad **does**
go dark and then relight, offered explicitly as memory rather than as a
measurement, so it is a lead rather than evidence. What the disc shows is
consistent with it and does not establish it: a per-circuit emissive mask
covering exactly the light bars is what a thing that lights up and goes out
would be authored as.

What is established so far, on the executable side:

- `SpeedupPad_Importer.cpp` and `WeaponPad_Importer.cpp` are both real
  translation units, sharing a `Pad_Importer.h` base, under
  `Code/System/Render`. Their constructors are `0x002dd2a0`/`0x002dd340` and
  `0x002e0168`/`0x002e0210`; both are below `0x32d5e0`, so Ghidra's TOC is the
  right one for them and their data references can be read directly (see
  [`memory.md`](../ghidra/functions/ps3-hdfury-eu/memory.md)).
- `WeaponPad_Importer`'s constructor stores a 128-bit vector at `this+0x1b0` -
  the same offset PSP's `Pad_Bind` writes its box minimum to - loaded through
  a TOC slot at `0x008b431c` whose value, `0x00aec2c0`, is past the image's
  last section (`0x009356ff`). It is a `.bss` global, so the bytes are not in
  the file and something else initialises them. `SpeedupPad_Importer`'s
  constructor has no such store, so the vector is class-specific.
- `uNumSpeedupPads`/`uNumWeaponPads` are **leaderboard stat fields**, not
  render state: their only reader is the `sceNpManagerGetOnlineName`
  serialiser at `0x0001d800`. Recorded so nobody follows them again.
- **No colour write and no emissive gate has been read on either class.**
  Nothing above is evidence of one.

### What binds the `_ne` file - answered, asset-side, 2026-09-07

**Nothing binds it, and now the reason is a measured mechanism rather than a
question.** The pad material is not a separate file the chunk merely names -
it is an ordinary **inline** `.rcsmodel` material record, and it genuinely
carries the `_ne` mask as the file's own `second_texture`
(`Material::second_texture`/`second_texture_sampler`, `+0x78`/`+0x60`). What
drops it is that the record carries a **third** sampler entry beyond the two
this renderer reads positionally - `[0]` the `_cs` diffuse, `[1]` the `_ne`
mask, `[2]` the circuit's lightmap - and
`oag_mesh::mesh::rcs::skin::picks` (`crates/mesh/src/mesh/rcs/skin.rs`)
resolves a material's one `aux` binding by "the lightmap wins the second
binding, wherever it sits" whenever `Material::lightmap_entry()` finds one
anywhere in the sampler list. That rule exists on purpose - it is what fixed a
third of Talon's Junction's baked lighting landing on a non-first entry - and
it does exactly what it was built to do on a pad material too. It just also
means entry `[1]`, the material's own designated second texture, is never
looked at again once a lightmap is found in the same list.

**The binding is not idle microcode either - measured off the actual shader.**
Resolving each pad chunk's shader variant the way `skin::variants` does
(`Class::Static`, the ordinary lit-race pass) and reading its declared
samplers and fragment program: the `_ne` sampler's name hash (`0xa2d555b9`,
shared by both `Speedup Pad` and `Weapon Pad`) binds fragment unit 1, and
`Program::accumulates(1)` is `true` on **18 of 18** pad chunks on `12_sol_2`
(10 `Speedup Pad`, 8 `Weapon Pad`). No parameter patch gates it - each
material carries exactly one `parameters` entry, and neither pad's hash
(`0x7611a2d8` speedup, `0xce5c4410` weapon) matches the tint, offset or
scroll-rate hashes `oag_mesh::mesh::rcs::emissive` already reads for other
circuits' glow materials, so a wired layer runs at that module's own
defaults - tint `1,1,1`, no scroll. **The disc's own shader treats the light
bars as a plain, unconditional additive layer on every chunk measured**, with
nothing in the material record that looks like a cooldown gate.
Reproduce with `crates/render/examples/hd_pad_material_dump.rs`.

**This is not the cheap wiring step it looks like, and that is worth stating
before anyone tries it.** The renderer's own architecture has room for only
one second texture per material - `Pick::aux` is a single `Option<usize>`,
`skin()` returns exactly two `TextureSlots`, `mesh.wgsl` binds exactly two
texture units per material - so making `aux` land on entry `[1]` instead of
entry `[2]` does not add the glow, it **trades the lightmap away for it**. On
a pad chunk with `NO_AMBIENT` set and a flat `0,0,0` vertex light, that is the
near-black failure this page's "checked as a picture" section above already
demonstrated the cost of - and the existing pixel-count guard
(`an_hd_speedup_pad_draws_visible_pixels`) would not catch it, because an
added glow layer *adds* lit pixels even while silently trading away a larger
number the lightmap was worth. Wiring this correctly needs a genuine third
texture slot per material, which is a bind-group-layout change reaching every
model the crate draws, not a pad-only fix.

Until the third-texture-slot work lands, an HD pad draws its authored texture
under the circuit's own lightmap in both states, unchanged by this finding.
That is a visible absence - a collected pad looks the same as an uncollected
one - and it is deliberately preferred to inventing a cooldown grey or a
regression dressed as a fix, per [`CLAUDE.md`](../../CLAUDE.md)'s "never
invent what the assets already author".

### The `_ne` file's RGB is a normal map, not paint - measured, 2026-09-07

A maintainer report from play (verbatim): HD weapon pads light up red, go
dark when they give a weapon, then relight when arming - and only the light
bars, not the whole pad. Since the `_ne` mask's alpha is already established
to cover exactly the bars, the cheapest open question was whether the same
file's RGB is red - which would make the lit state just this additive layer
at a red texel, drawn at the module's own white default tint. **It is not
red.** Decoded `12_sol_2`'s `ds_weaponup_ne.gtf` and `ds_speedup_ne.gtf`
through `oag_texture::gtf::Texture::to_rgba`
(`crates/render/examples/hd_pad_ne_colour_probe.rs`):

| File | Whole-texture RGB mean | Alpha-selected (bars) RGB mean | Alpha>0 coverage |
| --- | --- | --- | --- |
| `ds_weaponup_ne.gtf` | `[127, 129, 243]` | `[128, 71, 235]` | 6.8% of texels, 92% of those at alpha==255 |
| `ds_speedup_ne.gtf` | `[127, 128, 244]` | `[126, 63, 230]` | 7.3% of texels, 92% of those at alpha==255 |

Both files are blue-dominant everywhere (whole-texture blue channel never
drops below 99 of 255) with 32,770 and 35,360 distinct RGBA values across
~1.05M texels each - a continuous gradient, not a handful of flat painted
colours. That, the whole-texture mean sitting almost exactly on
`[128, 128, 255]` - the canonical tangent-space normal-map neutral - and the
rendered PNG's unmistakable embossed blue/purple look (rivets, panel edges
and the bars themselves in bas-relief) together read as a tangent-space
normal map, not a colour mask. **Independently confirmed from the shader's
own use, not just the pixel statistics**: disassembling both pads' fragment
programs (`crates/render/examples/hd_pad_ne_tint_probe.rs`) shows the very
first instruction after the unit-1 `TEX` is `MAD R#, R#, {2.0, -1.0, ...},
...` on both pads - the standard `x*2-1` decode that turns a `[0,1]`-packed
texture sample into a signed vector, applied to the same register the `_ne`
sample landed in. The disc's own program treats this texture's colour as a
normal to unpack, not a value to add.

**This also rules out DXT5nm** (the "green-and-alpha-only" normal-map
packing some engines use, where a texture's own RGB is throwaway and the
real X channel lives in alpha): alpha here is not a smooth per-texel channel
carrying geometry, it is binary-ish and sparse - 6.8-7.3% of texels non-zero,
92% of *those* exactly 255 - which is a mask, matching what was already
established (the mask covers exactly the light bars) and nothing else.

**Neither pad's fragment program carries a baked-in tint either**, red or
otherwise. `emissive`'s tint parameter (hash `0xe8bcd7f5`) does not appear
in either program's declared-parameter list, and `Program::patches` finds it
patching zero code slots in either - so it never reaches the microcode at
all, not even as a draw-time overwrite of a placeholder literal. The only
non-`[0,0,0,0]` inline constants in either 50-instruction program are the
normal-decode pair (`2.0, -1.0`), the `1/ln2` and specular-exponent (`32.0`)
literals a Blinn-Phong log/exp trick needs, and the placeholder zero
constants sun colour/direction and ambient patch at draw time - none of
which is a colour that could read as red.

**So, asset-side, nothing measured explains "light up red" for the light
bars specifically.** The `_ne` file's own colour channels are a normal map,
not paint; no tint - baked or parameter-driven - exists anywhere in either
pad's own shader; and the disc's declared default for this accumulate shape
is white. What the maintainer describes from play remains a lead this pass
could not close: settling it needs a Ghidra-side read of the pad importer's
per-frame update, outside this pass's tools (no Ghidra bridge held here).

**A second tension, found reading `mesh.wgsl` rather than the pad's own
microcode.** The renderer already implements a generic version of this
accumulate shape, for other materials on the disc: `glow = second.rgb *
tint.rgb * first.a` - unit 1's own **RGB**, times a tint, gated by the
**diffuse's** alpha (`first.a`, unit 0's fourth channel), not unit 1's own
alpha. Measured what that gate would actually select here: `ds_weaponup_cs.gtf`
and `ds_speedup_cs.gtf`'s own alpha is non-zero over **93%** of the texture,
fully opaque (`255`) only at the red cross/blue chevron outline (4% of
texels) - nothing like the tight ~7%, bars-only mask `_ne`'s alpha carries.
**If that generic mechanism is what actually runs for a pad material, wiring
it would spread a normal-map-coloured glow across nearly the whole plate**,
gated by the diffuse's own opacity, not confined to the light bars at all -
which would contradict the maintainer's "only the light bars" report even
more directly than the tint question above, and would mean `_ne`'s own
alpha (established to cover exactly the bars) is never even sampled by this
path. **Which of the two actually governs a pad material is unresolved by
this pass.** The generic shape is confirmed disc-wide for other materials,
not specifically disassembled end-to-end for either pad: the pad's own unit-1
sample lands in `R0`, gets unpacked as a normal (`x*2-1`) and is then
threaded through a Blinn-Phong specular chain before the program's final
`MAD`s, so by the time the program ends the register holding "unit 1's
value" no longer holds the raw sample - whether the final blend is really
this generic `second.rgb * tint * first.a` shape, some other shape reading
`_ne`'s own alpha instead, or the specular contribution alone with no
separate glow term at all, was not settled by tracing registers this pass.

**One picture, chosen and not measured, to make one candidate shape
concrete** - and, given the tension just found, this is not even the
best-supported of the candidates: `crates/render/examples/hd_pad_emissive_composite_probe.rs`
composites `_cs` (diffuse) plus white scaled by `_ne`'s own alpha, applied
offline, entirely outside the renderer (no change to `Pick`, `skin()` or
`mesh.wgsl`). The tint is `emissive`'s own fallback default, `[1,1,1]` -
**not a disc measurement**; the disc authors no tint for this material at
all, measured above. At that composite the bars turn white against a plate
whose border and cross are already red at rest; they do not turn red. This
is one candidate reading of at least four the microcode leaves open -
`accumulates(1) == true` says a value from unit 1 reaches the output, not
which of sampled RGB, RGB×alpha or alpha-alone it is, gated by which of two
different alpha channels this page has now found in the disc's own code -
and this pass did not resolve the accumulate instruction's own swizzle/mask
to settle any of it. Do not treat the PNG as measured; the measured half is
only the fallback tint value and each file's own alpha coverage.

### Which alpha channel gates the accumulate - settled, asset-side, 2026-09-07

> **PARTLY CONTRADICTED BY PLAY, 2026-09-08, and it lands exactly on this
> section's own stated weakness.** The maintainer reports from playing the
> original: **speed pads are cyan, weapon pads are red.** This trace measured
> the patched colour as light cyan/blue `(0, 196, 253)` and reported it
> **identical between the Speedup and the Weapon Pad material files** - and it
> is that *identity*, not the cyan, that the play report refutes. Cyan is
> right for the Speedup Pad and confirmed independently by play.
>
> **The suspect is the aliasing caveat this section already raised.**
> Confidence was capped at 82 precisely because of a `Weapon Pad`-specific
> concern: its diffuse fetch may alias `R0` under NV40's H/R packing, so
> `R0.w` at the weapon pad's `MAD H2, R0.wwww, ...` (const slot 58) may not be
> pristine `_ne` alpha. The one material the caveat singles out is the one
> material play says we read wrong. That is a strong hint the two are the same
> fault rather than a coincidence.
>
> **What to do:** re-derive the Weapon Pad's constant independently of the
> `R0` assumption - read const slot 58's own value out of the material record
> directly rather than inferring it through the register trace, and compare
> against slot 55's for the Speedup Pad. If slot 58 is red, the trace was
> right about the mechanism and wrong only about which constant reaches it.
> **Do not "correct" the value to red by hand** - measure it.
>
> Everything else in this section stands, including the finding that matters
> most: the accumulate is gated by the `_ne`'s own ~7% bars-only alpha, which
> the same play report independently corroborates ("only the light bars").

The open question above is answered by tracing register writes with their
own swizzle and write mask (`crates/render/examples/hd_pad_ne_tint_probe.rs`,
extended this pass to print both - it previously showed only the opcode and
the bare source list), cross-checked against `Program::output_texels()`, the
codebase's own already-calibrated register-taint reader for exactly this
question. **Confidence 82** on the hand trace itself: mechanical over a
50-instruction straight-line program with no branches, reproducible from the
probe's own output - capped below `output_texels()`'s own documented ceiling
because both readings share one unproven assumption (see the aliasing note
below), not because the arithmetic is in doubt.

**Neither of the two named candidates is what the pad's own microcode runs.**
It is not `mesh.wgsl`'s already-implemented generic shape
(`glow = second.rgb * tint.rgb * first.a`, gated by the diffuse's alpha) -
that shape is a single `MAD` and the pad's own program spends roughly forty
instructions between the unit-1 sample and the program's end computing two
separate curves against the material's other two textures. It is also not a
clean "`_ne`'s alpha alone, gating an add of `_ne`'s own RGB" shape either,
because `_ne`'s RGB is a normal map and is never used as colour (see above) -
it is consumed once, at the very first instruction after the `TEX`, and
never read again.

**The "second texture" the specular-shaped chain reads is not a fourth,
unbound one - it is unit 2, and unit 2 is the material's own already-known
lightmap.** `program.declared.samplers` names it directly:
`(0x37b5db58, unit 2)`, and `0x37b5db58` is `rcsmaterial::LIGHTMAP_SAMPLER` -
the exact hash `Material::lightmap_entry()` already resolves to
`lmaps/ile_mesh_combine13-lmap.gtf` for this material. So the renderer
already samples the one non-`_cs`, non-`_ne` texture a pad's own shader
reads: `skin::picks`'s "the lightmap wins the second binding" rule (see
above) is exactly what routes that same entry into the `aux`/`lightmap`
binding today. **There are two distinct power-shaped chains here, not one**,
easy to conflate because both share the `LG2`/`MUL`/`EX2` idiom
`Program::specular_exponent` already names elsewhere on this disc:

- A **per-channel curve applied directly to the lightmap sample itself**
  (`H6`/`H4`, the raw `TEX unit=2` result, each channel independently run
  through its own `LG2`→`MUL`→`EX2`) - the same `pow(lightmap.rgb, k)` shape
  `specular_exponent`'s own doc comment already records for `track_surface`,
  not a specular term at all.
- A **genuine `N·H` specular exponent**, a saturated `DP3` between two
  vectors built from the interpolated input and the unpacked `_ne` normal,
  fed through the identical `LG2`/`MUL(32.0)`/`EX2` idiom, landing in the
  scalar this page's trace below calls the "specular scalar."

**What the trace actually shows, identically shaped on both `Speedup Pad`
and `Weapon Pad`:**

- The unit-1 `TEX`'s destination register (`R0`) is written in full at
  instruction 0, then only its `.xyz` lanes are touched again (by the
  normal-unpack and the tangent-space chain that follows). **`R0.w` - the
  raw `_ne` alpha, exactly the file's own light-bar mask - is never
  overwritten for the rest of the program.** It survives, untouched, to
  instruction 41 (`Speedup Pad`) / 43 (`Weapon Pad`):
  ```text
  MAD H4, R0.wwww, C.xyzw, H4.xyzw   ; speedup, const_slot 55
  MAD H2, R0.wwww, C.xyzw, H0.xyzw   ; weapon,  const_slot 58
  ```
  an accumulate whose first operand is `_ne`'s own alpha, broadcast, times a
  material constant, added to a term that is itself `diffuse.rgb ×` the
  lightmap-power curve above. **This is the channel that gates the additive
  term** - `_ne`'s own alpha, the bars mask, not the diffuse's.
  **Aliasing caveat, `Weapon Pad` only**: this reading treats `H`- and
  `R`-indexed registers as independent storage, the same design choice
  `Program::output_texels()` makes (validated once, on a shipped `H2`/`R2`
  pair, per that method's own doc comment - not proven disc-wide). NV40
  packs `H[2i]`/`H[2i+1]` into `R[i]`, so `H0` packs into `R0` - and
  `Weapon Pad`'s diffuse sample is `TEX H0` (not `H5`, unlike `Speedup
  Pad`'s), landing between `R0`'s first write and this use. If `H0` and `R0`
  physically alias on real hardware *and* the compiler let a live `R0` value
  cross that write, `R0.w` here would not be pristine `_ne` alpha. Two
  things weigh against that: a compiler correctness argument (a live value
  colliding with a same-cycle full-mask `TEX` write into aliased storage is
  a basic register-allocation bug, not a plausible compiled shape), and a
  direct cross-check - `program.output_texels()` independently reports the
  `Weapon Pad` program's alpha lane as `Unit { unit: 1, channel: Some(3) }`,
  agreeing with this hand trace exactly, channel for channel. Both readings
  share the same underlying assumption, so this is not independent proof of
  the hardware's real packing - but it does rule out an arithmetic slip in
  the hand trace, and the `Speedup Pad` reading has no such caveat at all:
  its diffuse fetch is `TEX H5`, which packs into `R2`, disjoint from `R0`
  under the very same rule.
- The diffuse's own alpha (`H5.w`/`H0.w`, from the unit-0 `TEX` at
  instruction 36/39, likewise never overwritten before use) is read too, but
  one step later and as a **multiplier on the whole accumulated sum**, not a
  second independent gate: `H1.xyz = H1.xyz * diffuse_alpha + H4.xyz`, where
  `H4.xyz` already carries the `_ne`-alpha-gated term above. So a pad pixel
  where the diffuse is fully transparent would zero out *everything*,
  including the bars' own contribution - consistent with this being an
  alpha-tested surface (`pad_alpha_test_ground_truth.rs`) where a fragment
  that fails the test never reaches this arithmetic at all, rather than a
  second gate meaningfully narrowing the ~93% diffuse-alpha region down to
  the ~7% bars.
- The whole sum is then scaled again by the program's own specular scalar
  (the `EX2(log2(N·H) * 32)` chain, landing in `R0.x` on `Speedup Pad` /
  `R0.z` on `Weapon Pad`) and added to a third term, at the program's true
  final (`end`-flagged) instruction, built entirely from constants and that
  specular scalar - no texture read reaches it at all.
- **The two pad types' own output alpha differs, and `output_texels()`
  agrees with the hand trace on both.** `Weapon Pad`'s final `H0.w` is a
  bare `MOV` from `R0.w` - `_ne`'s own raw alpha becomes the fragment's
  output alpha directly, so the light-bar mask is what the alpha-test/blend
  stage itself sees; `program.output_texels()`'s own fourth lane reports
  exactly `Unit { unit: 1, channel: Some(3) }` for this program. `Speedup
  Pad`'s final `H0.w` is a `MOV` from a patched constant, unrelated to
  either texture; `output_texels()`'s fourth lane for that program is
  `Untraced` - no texture unit reaches it at all, which is what a
  constant-sourced alpha should report.

**The colour patched into the `_ne`-alpha-gated term is now identified, and
it is not red.** The constant at code slot 55 (`Speedup Pad`) is driven by
parameter hash `0x7611a2d8`; slot 58 (`Weapon Pad`) by `0xce5c4410` - the
same two per-material hashes `pads.md`'s earlier pass already found were
*not* `emissive`'s tint/offset/scroll, but did not go on to read what they
actually carry. Both materials' one declared parameter authors the identical
value on `12_sol_2`: `[0.0, 0.768628, 0.992157, 0.0]` - RGB `(0, 196, 253)`,
a light cyan/sky-blue, not red, and **identical between the two otherwise
separate material files**, which reads more like a shared circuit rim/sky
tint than a pad-specific "this is my glow colour." Confidence 85 for the
value itself (`Material::parameters` is a direct field read); confidence 55,
chosen rather than measured, for reading it as a shared rim tint rather than
a coincidence - nothing pins down the parameter's *name*, only its hash and
its authored value.

**So, settled:** of the two channels this page's earlier pass left open,
`_ne`'s own alpha - not the diffuse's - is the one genuinely wired into an
additive term in the pad's own shader, on both pad classes, at the identical
structural position. That is consistent with (though it does not explain
the colour of) the maintainer's "only the light bars" report, since `_ne`'s
alpha is exactly that mask. It does **not** reopen "light up red": the
colour this channel actually gates is measured, authored blue, and shared
between two otherwise-independent material files, which is a second,
independent reason (beyond the already-closed "no tint anywhere in either
program") to doubt this material record is the source of what the
maintainer described. What gates the diffuse's own alpha is a straight
multiplier on the sum that already includes the `_ne`-gated term, not a
second selection between two different light-bar masks - so the "the
generic shape would spread a normal-map-coloured glow across 93% of the
plate" concern that opened this section does not apply either: the pad's
real formula is not that shape at all, on either channel.

This still does not identify a cooldown state, and does not need to: the
`_ne`-alpha-gated term above runs through the same unconditional accumulate
already established (no parameter patch gates it, 18 of 18 chunks), so
whatever the maintainer describes from play is not sitting in this material
record any more than the earlier tint check found it there. Steps 2-3 of the
handover thread's Next Steps - the Ghidra-side vtable diff - remain the way
to find it, unchanged by this section.

### Corrected 2026-09-16: it is each pad's own colour, and it is red on most circuits

**The "identical, cyan, shared rim tint" reading above rests on one circuit
and does not generalise - the value varies per circuit, and the hash the
paragraph above calls unidentified was already named.** Two independent
misses compound here: `0xce5c4410` (`Weapon Pad`'s parameter) is
`~crc32("W_Cycle")`, recovered by `docs/formats/rcsmaterial.md`'s own
parameter-preimage sweep on 2026-08-31 and sitting in that page's table
since - two weeks before this section called it unidentified. `0x02ab9f07`
(`Speedup Pad`'s parameter on most circuits) is likewise already
`~crc32("Colour")` on the same table. Neither page linked to the other's
finding; this is the fix.

**The re-derivation the section above asked for**, straight out of
`Material::parameters` on every circuit that authors pads
(`crates/render/examples/hd_pad_colour_census.rs`, twelve circuits with pad
geometry out of sixteen `track.vex` on the disc - the four Zone circuits
author none):

| Circuit | Speedup Pad hash | value | Weapon Pad hash | value |
| --- | --- | --- | --- | --- |
| `01_vineta_k` | `Colour` | cyan | `W_Cycle` | cyan |
| `02_track` | `W_Cycle` | cyan | `W_Cycle` | cyan |
| `03_track` | `Colour` | cyan | `W_Cycle` | cyan |
| `04_chenghou_project` | `Colour` | cyan | `W_Cycle` | cyan (darker) |
| `05_ubermall` | `Colour` | cyan | `W_Cycle` | cyan |
| `10_sebenco_climb` | `Colour` | cyan | `W_Cycle` | cyan |
| `12_sol_2` | `0x7611a2d8` (unnamed) | cyan | `W_Cycle` | cyan |
| `15_anulpha_pass` | `0x7611a2d8` (unnamed) | near-white cyan | `W_Cycle` | **red** |
| `amphiseum` | chunks on a pad material, nodes unresolved | cyan (`speedup_material`) | `W_Cycle` | cyan |
| `modesto_heights` | chunks on a pad material, nodes unresolved | cyan (`weapon_pads`) | `W_Cycle` | **red** |
| `talons_junction` | chunks on a pad material, nodes unresolved | cyan (`weapon_pads`) | `W_Cycle` | **red** |
| `tech_de_ra` | chunks on a pad material, nodes unresolved | cyan (`weapon_pads`) | `W_Cycle` | **red** |

Every pad surface on every one of these circuits carries exactly one
parameter (`multi_param` in the tool's own output is empty), so "the value
is X" is not hiding a second, unread parameter.

**This settles the question the discriminating target was chosen for, and
the answer is neither of the two readings on the table.** It is not a shared
circuit/rim tint (the value is not constant - it is red on four circuits and
cyan/blue on eight) and the hash is not fixed per pad type either (`Speedup
Pad`'s own parameter is named `Colour` on six circuits, is `W_Cycle` itself
on `02_track` - the same hash `Weapon Pad` uses everywhere - and is the
still-unnamed `0x7611a2d8` on two more). What *is* fixed: **`Weapon Pad`'s
authored colour is red on `talons_junction`, `tech_de_ra`, `modesto_heights`
and `15_anulpha_pass`, and cyan/blue on the other eight; `Speedup Pad`'s is
cyan (or near-white on `15_anulpha_pass`) on every circuit that has one at
all.** `talons_junction` - this project's default circuit - is one of the
red ones, which is exactly the maintainer's play report. Confidence **88**:
a direct field read (`Material::parameters`) across every circuit that ships
the geometry, not an inference through the register trace, and not a hand
correction - the measurement came back red on its own.

**One authoring oddity worth recording rather than smoothing over**:
`02_track`'s `Speedup Pad` node resolves to a material record whose *file*
is still named `materials/weapon_pads.rcsmaterial`, textures swapped to
`ds_speedup_cs.gtf`/`ds_speedup_ne.gtf`. The file name is not a reliable pad
type identifier - only the `.vex` node's own class (`Speedup Pad` vs `Weapon
Pad`) and the material's own authored value are - which is also why naming
the parameter hash was never going to settle this on its own: `W_Cycle`
being "the weapon pad's own parameter" was itself an artifact of `hd_param_
names.rs`'s scan only ever meeting that hash on files still called
`weapon_pads.rcsmaterial`.

**Answered 2026-10-05 (hd-speed-pads):** `amphiseum`, `modesto_heights`, `talons_junction` and `tech_de_ra` do author speed-pad geometry; the node class is the same, but the nodes cannot find it. Their `Speedup Pad` nodes (18, 16, 17 and 15) carry a `+0x30` hash that matches **no** `.rcsmodel` chunk, while the models carry exactly 18, 16, 17 and 15 chunks on a pad material (`talons_junction` slots 360-377 and `tech_de_ra` 356-370 and `modesto_heights` 676-692 under `weapon_pads.rcsmaterial` with `ds_speedup_cs/ne.gtf`; `amphiseum` one slot, 489, shared by 16 chunks, under `speedup_material.rcsmaterial`). So the chunks were never excluded and drew in the scene's unreferenced pass as plain geometry, with the `_ne` bars unlit (black slots on `talons_junction`). Each material's authored value is cyan `[0.0, 0.768628, 0.992157]` (`hd_pad_ne_census` reads it). Probes: `hd_speed_pad_census`, `hd_speed_pad_hashes`, `hd_speed_pad_scene`.

**What this means for `oag_render`'s pad path**: nothing wires yet, and this
section does not change that on its own - see "One picture, chosen and not
measured" above for why a composite needs the accumulate's own swizzle/mask
resolved first, not only the colour. What it does settle is that a future
wiring should read this parameter **per material instance**, never assume
one shared constant for "the pad glow colour" the way the retracted reading
would have.

### Wiring attempted and stopped, 2026-09-25: the glow term reaches output through a multiply, and that multiply's own chain is undecoded

**Handover's Next Steps item 1 asked, before touching anything, whether the
`_ne`-gated term reaches the program's output through ADDs alone (the disc's
real term, safe to wire) or through a multiply by the specular scalar (needs
a chain this renderer cannot reproduce).** The page above already states the
answer in prose ("scaled again by the program's own specular scalar") but had
not traced it register-by-register to confirm it, and one sentence nearby
reads as if the diffuse alpha gates the accumulate itself rather than a
sibling term - worth resolving properly before writing a bind-group change on
top of it.

Traced both pad programs to their true final instruction with
`crates/render/examples/hd_pad_ne_tint_probe.rs` against
`12_sol_2/track.vex` (`DATA02.PSARC` on the EU disc), by hand, register by
register:

```text
Speedup Pad:
  [41] MAD H4, R0.wwww, C(0x7611a2d8), H4        ; the _ne-gated term itself
  [48] MAD H1, H1, H5.wwww, H4                    ; H4 only ADDED here
  [49] MAD H0, R0.xxxx, H1, R1                     ; END - H1 (carrying H4) MULTIPLIED by R0.x

Weapon Pad:
  [43] MAD H2, R0.wwww, C(0xce5c4410), H0          ; the _ne-gated term itself
  [46] MAD H0, H0, H0.wwww, H2                     ; H2 only ADDED here
  [49] MAD H0, R0.zzzz, H0, R1                      ; END - H0 (carrying H2) MULTIPLIED by R0.z
```

So the earlier "Which alpha channel gates the accumulate" section's own
`H1.xyz = H1.xyz * diffuse_alpha + H4.xyz` equation is right as written - the
`_ne`-gated term is only ever *added*, never itself multiplied by the diffuse
alpha - and the "scaled by the specular scalar" sentence is also right: the
**sum it lands in** (`H1`/`H0`, diffuse-alpha term plus `_ne`-gated term
together) is multiplied by `R0.x`/`R0.z` at the program's true final
instruction, before the last additive term. Both were true at once; neither
sentence was wrong, they were about two different multiplies. **This settles
Next Steps item 1's own question: no, wiring the `_ne`-gated term unscaled is
not the disc's real term - it omits a multiply that measurably still applies
to it.** Confidence 82, the same mechanical-trace ceiling the rest of this
page's hand traces carry, for the same reason: a straight-line 50-instruction
program with no branches, reproducible from the probe's own output.

**What blocks reproducing `R0.x`/`R0.z` is not the arithmetic itself but two
of the opcodes that build it.** Tracing back from the final `EX2 R0_sat,
R1.wwww -> .x` (speedup) / `EX2 R0_sat, R1.wwww -> .z` (weapon) through the
`LG2`/`MUL(1/ln2)`/`EX2` idiom this page already names
`specular_exponent`'s own shape, both chains pass through instructions the
probe prints as `???` - `Program::name()` (`crates/rcs/src/rcsmaterial/fragment.rs`)
returns `None` for opcodes `0x3b` and `0x3d`, the two nouveau's own opcode
table has no entry for and `docs/formats/rcsmaterial.md` already records as
genuinely unnamed disc-wide (86,664 and 28,634 uses respectively), not merely
unported. Three fall inside Speedup Pad's own chain (probe instructions 15,
23, 27) and three inside Weapon Pad's (19, 21, 31), all between the `_ne`
sample and the `EX2` that produces the scalar - `renderer.md` already reads
`0x3b` as a normalise/rsq helper "from its position in the stream", at
confidence ~70, and **deliberately does not apply that reading**, per
`docs/formats/rcsmaterial.md`'s own account of why it stays below this
project's rename line.

**So this is not a cost question any more, and stopping here is the correct
call under this project's own rule, not a shortcut around it.** Naming
`0x3b`/`0x3d` a value good enough to build a lit pixel from, at a confidence
this project has already looked at and declined to rename with, is exactly
the guess `CLAUDE.md` and this thread's own handover both ask not to make -
"below 50 confidence, do not rename at all... a guess dressed as a name stops
other people from looking" applies as much to a shader opcode as to a Ghidra
function. **Separately, and only relevant once the opcodes are named**: `N`
in this chain's own `N.H` is the tangent-space normal `_ne` itself decodes to
per pixel, not the vertex normal `mesh.wgsl`'s existing specular term already
uses - `crates/mesh/src/mesh.wgsl`'s `GpuVertex` carries no tangent
attribute, so even a correctly-named chain would need a per-pixel tangent
frame this renderer does not build yet, either from authored data or from
screen-space derivatives of `in.world`/`in.texcoord`. That is real, separate
work, but it is not what is blocking right now - the opcode names are.

**Nothing was wired.** `skin::picks`, `Pick`, `TextureSlots`,
`material_bind_group_layout` and `mesh.wgsl` are all unchanged by this pass:
adding a bind-group entry with no correct combine to put in it would draw a
picture with the right texture in the wrong place in the equation, worse than
the honest absence this page already documents. The two traps this project's
own history already named for this step - dropping the lightmap by reusing
its binding slot, and reproducing `mesh.wgsl`'s generic `glow = second.rgb *
tint.rgb * first.a` shape where the disc's own program does neither - are
both still live for whoever unblocks the opcodes and picks this up; a naive
"just add a third texture and gate by `_ne`'s alpha, undecoded scalar and
all" would be a third failure of the same shape, drawn instead of guessed
away, but still not what the disc's own microcode computes.

**What actually unblocks this, cheapest first:**

1. Get `0x3b`/`0x3d` past this project's own confidence line - independent
   corroboration for the existing normalise/rsq hypothesis (a second, unrelated
   usage shape, a primary-source opcode table, or a live GPU trace), not a
   second guess at the same confidence. `docs/formats/rcsmaterial.md`'s "A
   fourth, `0x3c`" paragraph is the template: it moved one opcode from
   unnamed to confirmed against Mesa's own header, on real primary-source
   evidence, not a stronger hunch.
2. Add a per-pixel tangent frame to `mesh.wgsl` for the one shading path that
   needs it - screen-space derivatives are the cheaper of the two options
   named above, since they need no new vertex attribute or `Model` field.
3. Only then does the bind-group change from the "Next Steps" plan below
   become worth making: a fourth (`_ne`) texture in `material_bind_group_layout`,
   a `Model::pad_mask`-shaped field populated only by `mesh::rcs::pads` (never
   the general scene pass, so this stays scoped to pad geometry the way the
   evidence is), and the disc's own `MAD`/`EX2` chain reproduced in
   `mesh.wgsl` rather than approximated.

### Item 1 cleared, 2026-09-25: `0x3b`/`0x3d` are named, off RPCS3's opcode table

**Not the existing normalise/rsq hypothesis strengthened - a different
primary source, found instead.** Mesa's `nvfx_shader.h` (this project's own
decoder's usual reference) has no entry for either opcode; RPCS3's own
`rpcs3/Emu/RSX/Program/Assembler/FPOpcodes.h` (GPLv2, independently
reverse-engineered against real hardware and shipping PS3 games) does:
`RSX_FP_OPCODE_DIVSQ = 0x3B` ("Divide by Square Root", `a / sqrt(b)`) and
`RSX_FP_OPCODE_FENCT = 0x3D` ("Fence T?" - RPCS3's own hedge). `DIVSQ`
resolves the specific contradiction that kept `op3B` below the rename line
(`renderer.md`'s "second usage shape" paragraph): a generic `a / sqrt(b)`
produces both the `DP3`-then-`op3B` normalize idiom (`v * rsqrt(d)`) *and*
the same-register `op3B(x, x)` shape (`x / sqrt(x) = sqrt(x)`, the standard
one-instruction square root on hardware with no native `SQRT`) from one
formula, where a dedicated `NRM` could only explain the first.

**Checked disc-wide, not asserted from the two hand-read examples that
motivated it**: `crates/render/examples/hd_op3b_op3d_census.rs`, all seven
archives, 1,632 `.rcsmaterial` files, 76,358 fragment blocks. `0x3b`:
183,623 uses, splitting cleanly into the two shapes above (105,800 /
38,284) plus 39,539 taking an `Input`/`Constant` operand (expected variety,
not a counter-example). Confidence 84. `0x3d`: 59,256 uses, **every single
one** writing destination register 63 (the 6-bit field's all-ones value) -
independently confirmed by hand on a sample to also carry `nvfx_shader.h`'s
own `NV40_FP_OP_OUT_NONE` bit set, matching Mesa's documented meaning for
that bit rather than merely correlating with it. Zero counterexamples: an
exact disc-wide invariant, not a sampled rate. Confidence 90 on "writes no
real destination" - consistent with "fence", inconsistent with any real
arithmetic contribution to a shading result. Both now named in `fragment.rs`
and `scripts/ps3-microcode.py`. Full evidence in
`docs/formats/rcsmaterial.md` and `docs/ghidra/functions/ps3-hdfury-eu/
renderer.md`'s "op3B resolves" section.

**What this clears for the pad glow specifically: item 1, fully.** Re-running
`hd_pad_ne_tint_probe.rs` with raw opcode numbers shown confirms all six
chain instructions the hand trace above found (Speedup Pad instructions 15,
23, 27; Weapon Pad 19, 21, 31) are `0x3b`/`DIVSQ` - **`0x3d` does not appear
in either pad's specular chain at all**, so both pad programs' specular
scalar is now fully nameable arithmetic: an `LG2`/`MUL(32.0)`/`EX2` power
curve fed, at its base, by a `DIVSQ`-built normalize.

**What is not cleared: item 2, still open, and nothing here touches it.**
The pad's `N.H` still needs a per-pixel tangent frame `mesh.wgsl` does not
build yet - naming the opcodes that compute the scalar does not supply the
normal that scalar's own chain needs as an input. **Nothing was wired in this
pass** - `fragment.rs`'s `Program::dp3_feeding`/`specular_exponent` gates are
unchanged (deliberately - see `renderer.md`'s own note on why relaxing
`dp3_feeding` to accept a `DP3`-then-`DIVSQ` idiom is separate work), and
`skin::picks`, `Pick`, `material_bind_group_layout` and `mesh.wgsl` are all
untouched. Item 2 (the tangent frame) and item 3 (the bind-group change) are
exactly as before.

### Wired, 2026-10-05: the pad light bars glow, and the "specular scalar" was fog

**Correction first.** The "wiring attempted and stopped" section above reads the multiplier `R0.x`/`R0.z` on the pad programs' final `MAD` as a specular scalar. It is **EXP2 fog**. `fragment.rs` dropped every source's negate bit (bit 17 of its own word) until this change; with it decoded, instruction 19 (Speedup) / 15 (Weapon) is `MUL R.w, -R1.w, R1.w` - the negated square of `TC3.w * fogColour.w` (`TC3.w` is clip-space w, the view depth) - then `MUL log2(e)` and `EX2_SAT`: `f = exp(-(k d)^2)`. The tail `MAD R1.xyz, -K, f, K` is `K (1 - f)` with `K = fogColour.rgb` (parameter `0x3dc31258`, which also patches the `.w` coefficient), and the final `MAD H0, f, colour, R1` is `f * colour + (1 - f) * fog`. Confidence 85 (a straight-line program, the same shape `scripts/ps3-microcode.py` documents for every fogged variant). So the `_ne` term is **not** multiplied by anything but fog, and `mesh.wgsl`'s `fogged()` already does that part.

The whole Speedup program, read with the negates (`hd_pad_ne_tint_probe`, now printing `IN<n>` and `-`):

```text
N      = nx*TC3 + ny*TC0 + nz*TC2          ; _ne.xyz*2-1; TC3 tangent, TC0 bitangent, TC2 normal
lit    = pow(lightmap.rgb, C(0x002c73e8)) * C(0x8670f0be) + lightmap.a * (N.L) * C(0x2dba643d)
spec   = (sat(N.Hhat))^32 * sat(N.L) * lightmap.a      ; H = normalize(TC1) + L, TC1 the eye vector
colour = diffuse * lit + _ne.a * W + diffuse.a * spec_colour * spec
out    = f * colour + (1 - f) * fogColour.rgb
```

Everything in it except the normal and the `_ne.a * W` term is what `mesh.wgsl`'s authored path already computed for every HD circuit chunk.

**What is built** (the disc's formula, not the generic `second.rgb * tint * first.a` shape):

- `_ne` bound as a **third** texture, `Model::pad_masks`, bind group 1 binding 3. The lightmap keeps binding 2. `slots::PAD_NE` (bit 14) is set by `mesh::rcs::pad_ne`, called from `mesh::rcs::pads` only.
- The `_ne` texel is decoded `x * 2 - 1` into the normal for `N.L` and `N.H`: `N = nx*T + nz*N0`.
- `_ne.a * W` is added after the light, ungated by the diffuse alpha. `W` is the parameter the program's inline constant is patched by, found by `Program::alpha_gated_parameter` (the `MAD dst, T.wwww, C, acc` after the `_ne` fetch), read per material instance: red on `talons_junction`'s `Weapon Pad`, cyan on `12_sol_2`'s both (`hd_pad_ne_census`). It rides in the material's `Model::emissive` tint.

**Measured, from the vertex program** (`vertex block #7` of `weapon_pads.rcsmaterial`, `ps3-microcode.py vp-file`): attribute `0xdbe5f417` (4 x ubyte, offset 10) is read `v[2].xyz*2-1` into `o[TC3]` (the tangent) and `v[2].w` multiplies `cross(N, T)` into `o[TC0]`. That `w` byte is `0` on 232 of 232 vertices of `12_sol_2`'s `Weapon Pad` chunks, 839 and 836 of 840 on `talons_junction`'s, 209 to 213 of 216 on `12_sol_2`'s `Speedup Pad`: **the bitangent is the zero vector and `ny` never reaches `N`**. The authored tangent points along `dP/du` (mean dot 0.94 to 0.99, `hd_pad_tangent_probe`). The earlier note that `+10` is an 11-11-10 tangent on stride 22 is not what the pad vertex program reads; the pad program reads it as ubytes.

**Chosen, not measured:** the tangent is derived per pixel from screen-space derivatives of world position and texture coordinate rather than decoded from the stream (no new vertex attribute); the diffuse `N.L` is clamped as the generic path clamps it where the pad program does not clamp it; `talons_junction`'s program is a different compile (64 instructions, `_ne` uv from `TC3.w`/`TC4.w`) and was read only for the structure the shader needs (the alpha-gated `MAD`), not traced end to end.

**Closed 2026-10-05:** the four original circuits' speed pads glow, see "Wired, 2026-10-05, the four original circuits" below. **Open:** the runtime cooldown colour (`WeaponPad_UpdateRefreshTimer`) is still unwired.

Pictures (`data/scratch/hd-pad-emissive/`, 1920x1080 `oag-game --race --mode single_race`): `ta_pair.png` and `tb_pair.png` (`talons_junction`, red), `sa_pair.png` and `sb_pair.png` (`12_sol_2`, cyan); top is before, bottom after. Pulse PSP's `16_Track` pads (`pulse_pw_*.png`, `pulse_ps_*.png`) are byte-identical before and after. Guards: `tests/hd_pad_ne_lit_path.rs` (authored rig; fails if the lightmap, the normal decode or the glow is dropped) and `tests/hd_pad_ne_ground_truth.rs` (disc-backed).

### Wired, 2026-10-05, the four original circuits' speed pads
`build_scene` now calls `pad_ne` for the pad materials (`weapon_pads`, `speedup_material`) that its unreferenced-chunk pass uses, so the 18, 16, 17 and 15 speed-pad chunks of `talons_junction`, `amphiseum`, `modesto_heights` and `tech_de_ra` bind their `_ne` mask and glow cyan (7,488, 6,656, 7,072 and 6,240 scene vertices, one colour each). Routing is by material, not node, because the nodes' hash names no chunk; **which node owns which chunk is still unknown** (not needed: a speed pad has no cooldown state). The filter matters: an unfiltered call also binds `ds_rail`, `ds_sf` and `ds_sfline_trench` (`diffuse_normal_specular_emmissive`, a different program) and put 5,296 red and 26,536 mixed glow vertices on `12_sol_2` and `01_vineta_k`. The 12 other circuits bind zero in the scene. Guard: `tests/hd_original_speed_pad_ground_truth.rs`. Pictures: `data/scratch/hd-speed-pads/<circuit>_{before,after}.png`. No RPCS3 reference frame was taken. Pulse is untouched: the change is under `mesh/rcs`, the PS3 builder, and no PSP path calls it.

## The load report said the speed pads were undrawn; they were not - 2026-10-06

On Talon's Junction (and the three other original circuits, 18, 16, 17 and 15
nodes) every `Speedup Pad` node names a hash no chunk of the `.rcsmodel`
carries, so `mesh::rcs::build_pads` - which draws a chunk through its node -
drew nothing, and the loader printed `speedup pads: 0 of 18 mesh node(s)
drawn from the .rcsmodel (0 triangle(s)); 18 addressed no chunk`, a warning
for a picture that was on screen. The chunks are the 18 on a pad material that
no node names, which `build_scene`'s world-space pass draws
(`bind_scene_pad_masks` binds their `_ne` mask by material). Seen in the
frame (`OAG_SKIP_MATERIAL=weapon_pads` removes the blue chevrons at tick 900,
`--hold cross`), and the report now counts them: 18 chunks, 5,382 triangles,
`Report::routed_chunks`/`routed_triangles`, pinned by
`crates/render/tests/hd_weapon_pad_split_ground_truth.rs`. What is *not* drawn
for them is the per-node ready/cooling state, which HD never recolours anyway
(above). Which chunk belongs to which trigger volume is unmeasured, so
nothing matches them by position. **Omega: not checkable** - its race
assets follow 2048 and its race is incomplete (`omega-status.md`), so there is
no HD-shaped circuit to read the same report on.
