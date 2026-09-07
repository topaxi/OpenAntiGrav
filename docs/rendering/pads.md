# Speed pads and weapon pads: what colours one

Two `.vex` classes, `Speedup Pad` `0x3bd` and `Weapon Pad` `0x3be`, and one
question this page exists to answer for both on every title: **where does a
pad's colour come from?**

The trigger side is [`pads.md`](../ghidra/functions/psp-pulse-usa/pads.md); the
payload layout is [`formats/pads.md`](../formats/pads.md); how a pad's geometry
gets into a model at all is `oag_render::mesh::build_pads` on PSP/PS2 and
`oag_render::mesh::rcs::pads` on PS3. This page is only about pixels.

## The answer, per title and per class

| Title | Class | Colour comes from | Recovered |
| --- | --- | --- | --- |
| Pulse (PSP/PS2), Pure | `Speedup Pad` | the texture, unmodified | 90 |
| Pulse (PSP/PS2), Pure | `Weapon Pad` | `pad+0x6c`, a per-tick runtime write | 85 |
| HD / Fury | `Speedup Pad` | the texture, unmodified | 80 |
| HD / Fury | `Weapon Pad` | the texture; any armed/cooling state is **unrecovered** | 80 |

Three of those four rows say "the texture", and that is the finding worth
carrying: **on every title measured, a pad's colour is painted by the artists
and the engine leaves it alone.** The one exception is Pulse's `Weapon Pad`,
and it is exceptional because its texture is deliberately neutral so that the
runtime write has something to colour.

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
| Pure | `speedup_GLOW_KEY.tga` | the same, and see [`HANDOVER.md`](../../HANDOVER.md)'s alpha-test note |
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
`oag_render::mesh::rcs::skin::picks` (`crates/render/src/mesh/rcs/skin.rs`)
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
scroll-rate hashes `oag_render::mesh::rcs::emissive` already reads for other
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
