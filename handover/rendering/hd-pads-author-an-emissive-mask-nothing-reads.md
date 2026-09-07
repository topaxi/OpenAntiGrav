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

## Open

**Does an HD weapon pad look different while it is cooling down, and what
draws that?** The maintainer's recollection from play is that it goes dark and
then relights, offered explicitly as memory rather than measurement. Nothing
read in the executable says so yet.

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
3. **Neither reaches the frame.** Every pad chunk on `12_sol_2` resolves to
   one surface role, `0x00000059`: second texture is the circuit's lightmap
   atlas (`lmaps/ile_mesh_combine13-lmap.gtf`), `ADD_SECOND` clear,
   `NO_AMBIENT` set. Neither `_ne` file is among the fifteen second textures
   the pad model loads at all. So the pads currently draw albedo times
   lightmap with no additive layer - honest, and missing the light bars.

## Next Steps

In order of cost, cheapest first:

1. **Find what binds a pad's `_ne` file.** Asset-side only, no emulator.
   `oag_render::mesh::rcs::skin::picks` decides which sampler entry a
   material's second texture comes from; the pad chunk's material is picking
   the lightmap instead. Start by dumping the pad chunk's own material record
   out of `12_sol_2/track.rcsmodel` and comparing its sampler list against
   `weapon_pads.rcsmaterial`'s 70 variants. If the pad material is a *separate
   file* the chunk names rather than an inline record, that alone is the gap -
   and it would explain why the disc ships `speedup_material.rcsmaterial`
   standalone at all.
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
4. **Only then implement.** Until one of 1-3 lands, an HD pad drawing the same
   in both states is the correct answer, not a placeholder to improve on. Do
   not add a cooldown grey by analogy with Pulse - the two titles' pads are
   already established to differ in mechanism, since Pulse's texture is
   neutral and HD's is painted.

A play capture would settle the *observable* half quickly and is worth doing
first if RPCS3 is already up: race on `12_sol_2`, cross a weapon pad, and
compare the pad in the frame before and a second after. That answers "is there
a state change at all" without reading a single instruction, and it decides
whether steps 1-3 are worth their cost.
