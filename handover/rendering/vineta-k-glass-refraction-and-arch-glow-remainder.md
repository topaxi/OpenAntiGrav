---
status: re-first
titles: [hd, 2048, omega]
updated: 2026-11-01
summary: "Vineta K: HD rock-to-sand blend unread, sea ripple needs the paraboloid probe; 2048/Omega target drawn, rt_icetunnel (Sebenco) glass law unread"
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

- **2048 Vineta K: the main view leaves out the meshes named `wohdmeshrefract` (wired 2026-11-01, `psp2-glass-re`).** It is not a file field. `Environment_Load` substring-matches each track mesh object's name and sets instance flag `0x20`. `TrackPass_FilterSubmesh` (`0x813b9670`) rejects that flag in the main view and draws flags `0x30` only into the half-size `screenSpaceRefractionTex` target. Confidence 82, evidence in `docs/ghidra/functions/vita-2048-eu-v104/refraction-pass.md`. There are 238 such meshes on Vineta K. The `tunnel_understruts*` are **not** among them, so the original 2048 draws them in its main view (HD's chunk 1574 has no 2048 counterpart in this rule). Wired for 2048 and Omega (`psp2::refraction_only`, `psp2_refraction_only_ground_truth`).
  - **Omega: ported (2026-11-01, `omega-glass-re`).** `TrackPass_FilterSubmesh` is at `0x01605760` with the same kind-0 `0x20` and kind-1 `0x30` tests (confidence 80, `docs/ghidra/functions/ps4-omega-eu/refraction-pass.md`). The `is_ps4` guard is gone. Omega skips 212 meshes on Vineta K, plus one on **Sebenco Climb** (`ice_tunnel_wohdmeshrefract2Shape`; the Vita's Sebenco has none).
  - **Zone keeps them (wired).** Zone pass kinds never test `0x20`: kinds 2 to 4 on the Vita, 3 to 5 on Omega. So the skip runs outside Zone only.
  - **Sebenco Climb moved.** Its icy rock sheet above the track (tick 1800 of an autopilot lap) is gone from the main view, and our sky shows black there until `rt_icetunnel` samples the target (below). Compare against an original Omega frame at that spot (the wipeout_pc native port).
  - The render target of Omega's kind-1 list `0x01f8f310` is not traced (`FUN_016304d0(5, ...)` in `FUN_015c63a0`).
  - **The refraction target is drawn (2026-11-01, `psp2-refraction-target`).** The left-out meshes go into a half-surface target (2048 480x272 measured, Omega 960x540 inherited) and `mt_tunnelrefraction` (2048, 26 submeshes) and Omega's `mt_`/`cl_tunnelrefraction` sample it: law `mix(fog, W * 2.0039 * D.a * G^2 + spec + lit * D^2, vis)` read off Vita3K's cached fragment SPIR-V (80). Test: `psp2_refraction_only_ground_truth` (target set == Zone's draws of the `0x30` meshes, main view == Zone minus refract, Zone has no target). **Inherited from HD, unread on the Vita:** the sample coordinate (the vertex program has no cached permutation), the 0.75 clip narrowing, single-sided drawing. **No alternate fog pair** exists in 2048/Omega `.envsettings`, so one primary-fog group; what the Vita does with flag `0x1000` (`wohdsecondaryfog`, 236 of 238) is unread. No Vita3K frame of a Vineta K tunnel exists to score the panes against.
  - **Open: Sebenco Climb's `rt_icetunnel`** (one submesh, Omega only, `screenSpaceRefractionTex` declared, five samplers, thirteen parameters, no `0x78575769`): its GCN pixel program is unread, so the rock sheet sits in the target and nothing samples it; tick 1800's black stays. Next: read it with `oag_rcs::gcn` and add its law beside `psp2::refraction`.
  - Next: Vita3K capture at a tunnel pose to score the panes and the sample coordinate; read the Vita's target camera and flag `0x1000`.
  - A Vita3K capture of the target's draws at the tunnel would take the confidence from 82 to 90+.
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
