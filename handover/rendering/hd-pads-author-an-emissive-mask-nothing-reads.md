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

## Open

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

0. **"Only the light bars" and "the emissive layer toggles" are the same
   statement.** The `_ne` mask's alpha is already established as covering
   exactly the light bars and nothing else, so a state change confined to the
   bars is a state change confined to what that mask selects. This is the
   strongest evidence yet that the emissive layer measured below is the whole
   mechanism, and it is why the questions below are worth their cost.
1. **The trigger is the grant, and the relight is the refresh.** That maps onto
   a `WeaponPad_UpdateRefreshTimer`-shaped per-frame update, which is exactly
   what step 2's vtable diff is for. It also means the play capture the old
   version of this row asked for is no longer the cheapest way to establish
   *whether* there is a state change - there is one.
2. **"Red" is checkable asset-side, right now.** The `_ne` mask's **alpha** is
   established as covering the light bars; **its RGB has never been looked
   at**. If the bars are red in the `_ne` file's own colour channels, the lit
   state is just the emissive layer measured above, drawing at `emissive`'s
   default white tint over red texels - and the dark state is that layer being
   switched off, not a recolour.
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

In order of cost, cheapest first. Step 1 (finding the binding) is done; what
is left is deciding whether to spend on wiring it, and the two RE leads.

1. **Wire the emissive layer - and read the trap before touching it.** This is
   *not* the cheap step it looks like. The naive fix - make `skin::picks`'s
   `aux` slot land on sampler entry `[1]` instead of `[2]` for a pad material -
   is a regression, not a fix: it drops the lightmap binding entirely, and
   `docs/rendering/pads.md` already establishes what that does to an HD pad
   lit only by its lightmap with `NO_AMBIENT` set - flat blue-looking-correct
   turns near-black. **The existing guard would not catch it either**:
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
   not a pad-only change. Do it with a picture check at each step, not just a
   pixel count: an isolated pad plate through `capture::capture_from`, at
   player framing, before and after.
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
   gate - so if 2-3 do find a cooldown state, it is not sitting in this
   material record and the light bars wired per step 1 would need to run
   always-on regardless, with whatever 2-3 find layered on top rather than
   replacing it.

A play capture would settle the *observable* half quickly and is worth doing
first if RPCS3 is already up: race on `12_sol_2`, cross a weapon pad, and
compare the pad in the frame before and a second after. That answers "is there
a state change at all" without reading a single instruction, and it decides
whether steps 2-3 are worth their cost.
