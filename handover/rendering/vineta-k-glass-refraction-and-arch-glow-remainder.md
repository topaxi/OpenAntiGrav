---
status: re-first
titles: [hd, 2048, omega]
updated: 2026-11-01
summary: "Vineta K: rock-to-sand mask blend (j_rockblend5) unread; the sea sheet ripple needs the paraboloid probe"
---

# Vineta K's glass refraction and arch glow: what is left

2026-10-07, `hd-vineta-ceiling`, then `hd-glass-opus` and `hd-behind-glass`. The reads and the fixes are in
[rcsmaterial.md](../../docs/formats/rcsmaterial.md), "Vineta K's ceiling: the tunnel glass reads the screen, and the
arch lights lost their glow", its section 3 "The behind-the-glass target is drawn", and
[visibility.md](../../docs/ghidra/functions/ps3-hdfury-eu/visibility.md), 2026-10-07.

**Landed (`hd-behind-glass`):** the behind-the-glass target is drawn (640x360, the sky plus the `0x10` chunks, 4/3
wider tangent, the `0x20` alternate fog, single-sided), and the glass is one opaque draw reading it. Pose A's right
ceiling panes: reference `(4, 87, 87)`, before `(135, 176, 57)`, after `(14, 72, 72)`. Lane scratch:
a scratch directory, not kept (`report.md`, `shots/`, `sweep/`).

## Open

- **2048 and Omega Vineta K: no file field found for HD's behind-the-glass bit (negative result, 2026-11-01, `psp2-behind-glass`).** The maintainer sees extra meshes in the 2048 and Omega tunnels; HD's `0x10` bit lives in the chunk render block (`Mesh::render_flags`), a different record from the material render-state word whose bit 4 is the cull bit. The psp2 container has no located render block. Census over 2048 DLC1 Vineta_K `track.rcsmodel` (1,822 mesh objects) and Omega `01_vineta_k/track.final.rcsmodel` (2,878): mesh object `+0x0a` flags take 0x0101/0x0201 only (266 `0x0201` are all `trackpart_animations*`); every bit of the 64 words around each first submesh record, the mesh object's 16 words and the material state (6 values on 2048, 8 on Omega) were scored against the Maya export tags `refract` and `secondaryfog` in the names (273 meshes on 2048, 227 on Omega, the only oracle for HD's `0x10`/`0x20`): none splits them (best bit 70 errors against a 273 baseline, and that word is pointer or string bytes). Material state bit 4 (the cull lane's bit) is set on all six 2048 values, so it partitions nothing here. HD flagged chunks barely twin: 24 of 210 flagged HD chunks have a 2048 bbox twin, because 2048 re-splits them (HD chunk 1574, 2,488 vertices, is covered by ~170 small `tunnel_understruts*` pieces of 35 and 218 vertices that carry no tag). Nothing was wired and no name list used. **Next read recipe (opus RE):** find in the Vita `RcsModel_Load` object graph (and Omega's counterpart) what the render passes read per mesh to choose the glass-target pass, starting from the draw routing that consumes `tunnel_understruts*` meshes; or capture Vita3K draws into the glass target and tie them to submeshes by vertex buffer offset, as HD's RPCS3 census did.
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
- Talon's Junction's frame moved by 1/255 in one pixel with this change (Pulse is byte-identical); loader report and
  draw counts are unchanged and neither fragment entry point's old code removes it. Cause not isolated.
- `cl_tunnelrefraction`'s own diffuse-colour scale (0.2549) and doubled grab are read off the disassembly, not pinned
  by a test.
- **Tech de Ra's glass road: ours is too wide a rainbow, not wrongly rainbow** (2026-10-21, `hd-rainbow-glass`). A cold-boot original is lilac and
  blue with a pink band; ours adds a yellow-green far band and a rainbow wedge. Read `rpcs3-capture.md`, "Tech de Ra's glass road is iridescent
  in the original too", for what is ruled out (the ramp sign). Not yet read: the bias/light scaling of the ramp, the undrawn `paraboloidReflectionTex`
  tint, fog at distance. Talon's far track has no cold-boot capture yet (`rpcs3-hd-cold-pair.sh <lane> 8 <out>`).

## Next Steps

1. Read `and_rocktosand`'s mask blend (`j_rockblend5` red channel between `and_sand_sand` and `and_rock4`) and draw
   it; re-dump the target at pose A (`OAG_DUMP_BEHIND_GLASS`) against `target_be.png` in the `hd-glass-opus` scratch.
2. Measure whether the 4/3 holds at speed (`place --speed`, dump `0x00c49240..0x00c492b0` and the vertex constants in
   the hook's last round), and capture a second pose with the glass in view to re-check the coordinate law.
3. RPCS3-capture a second circuit that `hd_add_second_census` shows moved (`place --hook`, same recipe).

- **Glass sweep, 2026-10-21 (`hd-glass-sweep`, then `hd-glass-fable`): the grid's coordinate is right, its mip level is the structure, `c` is on the road.** `rpcs3-capture.md`, "The grid's coordinate is right and its mip level is the structure". The "`in.texcoord` about 27" reading is withdrawn: the road samples the grid at `Uv1` as the program does, at derivative level 3 to 6, where the original reads 2 to 2.5 levels finer (its `-1.5` bias plus the stored frame's scale-150 supersampling); the fix is the held `material_lod_bias` rule, re-judged on the scale-100 originals. `emissive` no longer claims sheen slots, so `glass_sheen::bias` writes `c` on Tech de Ra's road: saturation 0.31 to 0.26 (original 0.24), far G/R 1.24 to 1.02 (1.01); the pair's mean grey rises 0.41, and the excess is the ramp's coordinate on the mid and far road (the original's pixel implies `u` about 0.36 to 0.5 where ours reads 0.10 to 0.20). Next: dump draw 354's vertex constants (`c[209]`, `0x3466fc0e`) at the cold pair's tick against the view matrix's eye, then the chunk's packed normal.
