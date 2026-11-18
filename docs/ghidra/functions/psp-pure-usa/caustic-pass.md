# Pure's caustic pass: Pulse's, call for call, with the frames inside the circuit's `.vex`

**Binary:** `psp-pure-usa` `BOOT.BIN`, image base `0x08804000`, Ghidra `/pure/BOOT-psp-pure-usa.BIN`.
Read 2026-11-18 (lane `pure-caustics`): decompiles and a byte search for Pulse's constants, plus a
census of the EU disc's `.vex` files and a headless frame of ours. No PPSSPP run. Twin of
[Pulse's page](../psp-pulse-usa/caustic-pass.md), which gives the law step by step; the cross-title
picture is on [water.md](../../../rendering/water.md).

## Answer

Pure runs Pulse's caustic pass unchanged, on one circuit. **Vineta K's `track.vex` flags 352 of its
1,335 batches** (header byte 3 bit `0x04`, read with Pure's own mesh class `0x11e`); no other Pure
`.vex` flags any (Blue Ridge, Modesto Heights and the weapon and billboard files: 0). Reproduce:
`OAG_SWEEP=1 cargo nextest run -p oag-vex --run-ignored all --no-capture -E
'test(/water_caustic_sweep::/)'`, and `only_vineta_k_carries_the_caustic_bit_on_pure`
(`crates/vex/tests/caustic_pass_ground_truth.rs`). The sweep once read `0/0` on Pure because it walked
version 6's mesh class (`0x125`); it now picks the class by the file's version.

## Functions and data

| Address | Name | Pulse twin | Confidence | Evidence |
| --- | --- | --- | ---: | --- |
| `0x0886e460` | `Mesh_BeginCausticPass` | `0x0890dc64` | 80 | the same call sequence: `Gu_TexProjMapMode(0)`, `Gu_TexMapMode(1,0,0)`, `Gu_TexWrap(0,0)`, `Gu_Enable(10)`, `Gu_Disable(0xb..0xe)`, white colour, `Gu_AlphaFunc(1,0,0xff)`, `Gu_Disable(0)` |
| `0x0886e5d0` | `Mesh_SetCausticNodeState` | `0x0890ddd8` | 80 | `byte[mesh+0x0e]` times the three tint floats into `Gu_Ambient`; `Gfx_BuildBatchStateList(.., 0x202, 0, ..)`; pixel mask `0xff000000`; fog by `byte[mesh+0x0c] & 8`; the rotation literal `0x3fc90fdb`; texture matrix slot 3 |
| `0x0886e91c` | `Mesh_EndCausticPass` | `0x0890e0dc` | 80 | `Gu_TexMapMode(0,0,0)`, `Gu_DepthFunc(6)` |
| `0x0886e884` | `Mesh_SetCausticBatchState` | the loop body in `FUN_089307b4` | 75 | depth mask and function by `pass_mask & 2` (write and `6`, else no write and `2`) |
| `0x0886dfd4` | `Mesh_DrawCausticNode` | the group-`8` loop in `FUN_089307b4` | 78 | begin, per-node state, then every batch whose byte 3 `& 4` (or whose mesh `+0x124 != 0`), end |
| `0x0886d414` | `Texture_LoadEffectSurfaces` | `Texture_LoadEffectSurfaces` | 78 | sixteen `FUN_0886595c(.., 0x2453f0 + 0x20 * n)` lookups of the names at `0x08a493f0` ... `0x08a495d0`, `Data\Tex\caustics\save.01.tga` to `save.31.tga` (the odd numbers), the Pulse table's twin |
| `0x08a8f500` | `g_caustic_tint` | `0x08abf494` | 75 | bytes `cdcc4c3e cdcc4c3f 0000803f`: `0.2, 0.8, 1.0` |
| `0x08a8f50c` | `g_caustic_uv_scale` | `0x08abf4a0` | 80 | `0x3c656042` = `0.014`, read once, by `0x0886e5d0` (`0x0886e76c`); the only occurrence of the word in the binary |

Callers: `0x0886dfd4` from `0x0886f9ec` and `0x08870634`; `0x0886e5d0` from `0x0886dfd4` and
`0x0888473c`. The tint's channel order is Pulse's (`0.2` red, `1.0` green, `0.8` blue in the packed
word, read from the decompile). Not located: the `(int)(track_time * 30) & 0xf` frame binder (the 30
and the mask are Pulse's; the same sixteen frames and the same loader shape make it the same call).

## Where the frames are

Not in `Data.wad`: neither `Data\Tex\caustics\save.01.tga` (`0x94c3bc56`), the `.mip` name nor the
`.pct` name has an entry on the Pure EU disc. Vineta K's `.vex` embeds them as textures named
`Z:\data\Tex\caustics\save.NN.tga`, the odd `01` to `31`, and the loader's lookup by name finds
them there. Ours reads the sixteen from the circuit's own embedded textures
(`oag_raceplay::load::caustic`), by the name's last component.

## What ours does

`oag_pure::effects::LOOKS.track_caustic` is on for the PSP, `Origin::Measured`, and Pure's
`water.original_differs` is `true`: `graphics.water = off` draws Vineta K's seabed plain,
`original` draws the moving ripple web. Of the 352 flagged batches 279 are drawn and 73 (under an
animated node) are left out, as on Pulse. Test: `vineta_ks_seabed_draws_its_embedded_caustics_and_
they_follow_the_clock` (`crates/game/tests/caustic_pass_ground_truth.rs`).

## Confidence

- **85** that Pure's pass is Pulse's law: four functions with the same call sequence and the same
  constants, one flagged circuit, the frame table the same sixteen names.
- **70** on the picture (a decompile and the census, no PPSSPP GE list of Vineta's seabed taken):
  the look of ours is Pulse's law applied; a recorded list is the check that would raise it.

## 2048 and Omega

Not applicable: Pure's pass is the PSP engine's, and 2048 and Omega share HD's lineage, which has no
caustic pass (`data/tex/caustics/` ships on HD with no consumer found,
[water.md](../../../rendering/water.md)). Checked, differs.
