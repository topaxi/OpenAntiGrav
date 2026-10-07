# Vineta K's glass refraction and arch glow: what is left

2026-10-07, `hd-vineta-ceiling`. The read and the fixes are in [rcsmaterial.md](../../docs/formats/rcsmaterial.md), "Vineta K's ceiling: the tunnel glass reads the screen, and the arch lights lost their glow".

## Open

- The refraction pass reads the grab at the pixel's own position; the original perturbs it by the normal map times `0x9fc59444`. A real scene-copy pass would draw it, and would also put transparent draws behind the glass (the sea foam) into the grab in the right order.
- The arch glow texture is sampled at the diffuse coordinate; the program samples it at `f[TC0].zw`.
- `accumulates`' per-lane taint reaches 20 of 28 circuit models (`hd_add_second_census.rs`); only Talon's Junction `03` has a reference. Take a frame of Modesto Heights or Amphiseum at a matched pose.
- **The scenery beyond the glass, measured further (2026-10-07, `hd-glass-opus`; visibility.md and rcsmaterial.md
  section 3):** (a) the alternate fog is selected per chunk by render flag `0x20` (35/35, 76/76), **not wired**;
  (b) the ceiling "extra meshes" are kind-1 chunks 16-18, 55, 1315, 1317-1321, which the original culls with a frustum
  built from the authored fov 60 while it draws the picture at 4/3 the tangent (75.2 degrees). Ours draws HD at 60
  degrees. Closing it needs HD to draw 4/3 wider and cull at the authored fov, which is a camera change (go/no-go
  asked of the lead). Also: ours draws every PVS-allowed node-placed chunk (197) without a frustum test; the
  original tests each one's runtime world sphere.
- **The sea sheet** (`water_test_2`, `y = -50.8`, seen from below) is now `vertex colour * (ambient + sun * N.L)` (it drew white).
  `paraboloidReflectionTex` is a **runtime 512x256 dual-paraboloid render of the environment** (sky above the middle row, teal
  sea below; `probe11.png`), not `skyreflect.gtf`; its lower half, the part this view reads, has no disc source found. Not drawn.
  The panes' teal is the glass's `W` times a near-white grab, so it also needs the sky tint (`hd-sky-luma`'s open item).
- `cl_tunnelrefraction`'s own diffuse-colour scale (0.2549) and doubled grab are read off the disassembly, not pinned by a test.

## Next Steps

1. Wire the alternate fog for `render_flags & 0x20` chunks and re-compare at the maintainer pose. Measure whether
   the 4/3 draw-versus-cull ratio holds at speed (`place --speed`, dump `0x00c49240..0x00c492b0` and the vertex
   constants in the hook's last round).
2. RPCS3-capture a second circuit that `hd_add_second_census` shows moved (`place --hook`, same recipe).
3. A real scene-copy pass for the glass (the grab offset by the normal map) once the fog group is right.
