# Vineta K's glass refraction and arch glow: what is left

2026-10-07, `hd-vineta-ceiling`. The read and the fixes are in [rcsmaterial.md](../../docs/formats/rcsmaterial.md), "Vineta K's ceiling: the tunnel glass reads the screen, and the arch lights lost their glow".

## Open

- The refraction pass reads the grab at the pixel's own position; the original perturbs it by the normal map times `0x9fc59444`. A real scene-copy pass would draw it, and would also put transparent draws behind the glass (the sea foam) into the grab in the right order.
- The arch glow texture is sampled at the diffuse coordinate; the program samples it at `f[TC0].zw`.
- `accumulates`' per-lane taint reaches 20 of 28 circuit models (`hd_add_second_census.rs`); only Talon's Junction `03` has a reference. Take a frame of Modesto Heights or Amphiseum at a matched pose.
- **Two measured facts about the scenery beyond the glass (2026-10-07, `vineta-k-fidelity`; rcsmaterial.md, "Vineta K against a
  draw capture", section 3):** (a) the original draws one group of its draws (36 of 273 at the tunnel pose) with
  `Fog.Alternate Fog Color/Density` (`0 0.031373 0.031373`, `0.0045`), which this build never reads; what selects that group
  is open (not position, not material family, not the PVS cell). (b) Ours draws `and_girder3` (chunk 1320), `and_metalstruts_pt2`
  (1305), `and_dome_vent` (1286), `and_darkstrip` (1311), `j_strip_lights` (1315) near the tunnel and the original's frame binds
  none of them; the nearest cell's bitmap allows all five, so the cull is unexplained. Chunks 1286 and 1305 have origin-centred
  bounds (node-placed): read whether their `.vex` nodes sit under an LOD, switch or animation parent. Test any rule against
  `data/scratch/vineta-k-fidelity/out/boot9` and `out/boot6`.
- **The sea sheet** (`water_test_2`, `y = -50.8`, seen from below) is now `vertex colour * (ambient + sun * N.L)` (it drew white).
  `paraboloidReflectionTex` is a **runtime 512x256 dual-paraboloid render of the environment** (sky above the middle row, teal
  sea below; `probe11.png`), not `skyreflect.gtf`; its lower half, the part this view reads, has no disc source found. Not drawn.
  The panes' teal is the glass's `W` times a near-white grab, so it also needs the sky tint (`hd-sky-luma`'s open item).
- `cl_tunnelrefraction`'s own diffuse-colour scale (0.2549) and doubled grab are read off the disassembly, not pinned by a test.

## Next Steps

1. Find the rule that picks the alternate-fog draws and the node parent that hides the five absent chunks: read who writes `fogColour` per draw group in `Scene_PrepareFrame`
   (`0x003aa888`, fog offsets `+0x4f0`/`+0x504` in the `.envsettings` object), or fit a rule to the 36 captured draws'
   chunks (`rcs.render_flags`, `Water.Water plane height` 100, the chunk's node). Then draw the group with the alternate
   pair and re-compare `pairA.png`-style at the maintainer pose.
2. RPCS3-capture a second circuit that `hd_add_second_census` shows moved (`place --hook`, same recipe).
3. A real scene-copy pass for the glass (the grab offset by the normal map) once the fog group is right.
