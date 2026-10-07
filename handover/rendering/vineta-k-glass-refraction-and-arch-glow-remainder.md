# Vineta K's glass refraction and arch glow: what is left

2026-10-07, `hd-vineta-ceiling`. The read and the fixes are in [rcsmaterial.md](../../docs/formats/rcsmaterial.md), "Vineta K's ceiling: the tunnel glass reads the screen, and the arch lights lost their glow".

## Open

- The refraction pass reads the grab at the pixel's own position; the original perturbs it by the normal map times `0x9fc59444`. A real scene-copy pass would draw it, and would also put transparent draws behind the glass (the sea foam) into the grab in the right order.
- The arch glow texture is sampled at the diffuse coordinate; the program samples it at `f[TC0].zw`.
- `accumulates`' per-lane taint reaches 20 of 28 circuit models (`hd_add_second_census.rs`); only Talon's Junction `03` has a reference. Take a frame of Modesto Heights or Amphiseum at a matched pose.
- The glass sits in front of `hd-sky-luma`'s sky: re-take `data/scratch/hd-vineta-ceiling/cmp_p03.png` after that lane merges, expecting teal.
- `cl_tunnelrefraction`'s own diffuse-colour scale (0.2549) and doubled grab are read off the disassembly, not pinned by a test.

## Next Steps

1. Re-render `--camera-pose=-861.568191,-145.613369,178.961006,-0.122993,-0.045426,0.991367,-0.100063,0.994429,0.033152 --camera-fov 66.9506` once the sky lane lands.
2. RPCS3-capture a second circuit that `hd_add_second_census` shows moved.
