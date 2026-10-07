# Vineta K's glass refraction and arch glow: what is left

2026-10-07, `hd-vineta-ceiling`, then `hd-glass-opus` and `hd-behind-glass`. The reads and the fixes are in
[rcsmaterial.md](../../docs/formats/rcsmaterial.md), "Vineta K's ceiling: the tunnel glass reads the screen, and the
arch lights lost their glow", its section 3 "The behind-the-glass target is drawn", and
[visibility.md](../../docs/ghidra/functions/ps3-hdfury-eu/visibility.md), 2026-10-07.

**Landed (`hd-behind-glass`):** the behind-the-glass target is drawn (640x360, the sky plus the `0x10` chunks, 4/3
wider tangent, the `0x20` alternate fog, single-sided), and the glass is one opaque draw reading it. Pose A's right
ceiling panes: reference `(4, 87, 87)`, before `(135, 176, 57)`, after `(14, 72, 72)`. Lane scratch:
`data/scratch/hd-behind-glass/` (`report.md`, `shots/`, `sweep/`).

## Open

- **The dunes inside the target are rock, the capture's are sand.** `and_rocktosand`'s red-channel blend mask
  (`j_rockblend5`) is read as the rock picture alone (rcsmaterial.md's table, "the red shape behind the glass"). It
  now shows through every pane that looks at the terrain.
- **The sea sheet is a smooth gradient where the capture's is rippled**: its `TC1 * R` term, the runtime 512x256
  dual-paraboloid `paraboloidReflectionTex`, is not drawn; its lower half has no disc source found (rcsmaterial.md
  section 2). This is now the largest difference at the top of the target.
- **Why the glass's literal coordinate is twice the target's.** `refractProject` and `distortion` as read give
  `(1 + x/w, 1 - y/w)`; the picture agrees with `0.5 + 0.5 (x/w, -y/w)` (90 % against 0 % on 7,341 pane pixels).
  A texture normalisation rule of the linear `R5G6B5` target is the likely reason; unread.
- The normal map's offset of the glass's projected point is not drawn (vertex normal, one unit).
- The 5 main-view chunks with flag `0x20` keep the primary fog.
- Ours draws every PVS-allowed node-placed chunk (197) without a frustum test; the original tests each one's runtime
  world sphere. The writer of those spheres is not found.
- The arch glow texture is sampled at the diffuse coordinate; the program samples it at `f[TC0].zw`.
- `accumulates`' per-lane taint reaches 20 of 28 circuit models (`hd_add_second_census.rs`); only Talon's Junction
  `03` has a reference. Take a frame of Modesto Heights or Amphiseum at a matched pose.
- `cl_tunnelrefraction`'s own diffuse-colour scale (0.2549) and doubled grab are read off the disassembly, not pinned
  by a test.

## Next Steps

1. Read `and_rocktosand`'s mask blend (`j_rockblend5` red channel between `and_sand_sand` and `and_rock4`) and draw
   it; re-dump the target at pose A (`OAG_DUMP_BEHIND_GLASS`) against `target_be.png` in the `hd-glass-opus` scratch.
2. Measure whether the 4/3 holds at speed (`place --speed`, dump `0x00c49240..0x00c492b0` and the vertex constants in
   the hook's last round), and capture a second pose with the glass in view to re-check the coordinate law.
3. RPCS3-capture a second circuit that `hd_add_second_census` shows moved (`place --hook`, same recipe).
