---
categories: [rendering]
---

# Pulse's clouds are one 4-unit sprite per cube; the original draws a field of large ones

2026-10-04, `pulse-bloom-roll`. Found while verifying the clouds' roll
counter-rotation live, which is done: `CloudGroup_Draw` (`0x0893280c`, 92)
turns each quad by `g_camera_roll - phase`, and `oag_render::cloud` does the
same. See `docs/ghidra/functions/psp-pulse-usa/clouds.md`.

## Open

- **Ours draws far fewer and far smaller sprites.** In the original, live on
  de Konstruct Black (`05_Track` forward), the two `cloudGroup` instances near
  section 34 (`0x09790220`, `0x097903e0`) each hold 46 authored records
  (`+0x184`), culled by `Overlap` to 14 drawn (`+0x188`). The records are
  scattered through a box about 175 units across, the group's `+0x100`/
  `+0x110`. Their half-sizes (`record+0x10`) run from 32 to 47. Ours draws one
  sprite per `cloudCube`, at its centre, with half-size `SpriteRadius` = 4:
  five sprites on the whole track. That is why the clouds barely show in our
  frames, and why the roll fix moves no pixel at a matched pose.
- The half-sizes fit `4 * ~9.4 * (1 +/- 0.2)`; the factor's source is not read.
- Ours also has four cubes near x = -1000 to -1120; the original's RAM held
  only these two groups with drawn records. Not checked further.
- The original does not run the draw when the camera is far away (start line,
  section 29: the phases froze and the vertex buffer was never written). The
  test that decides this is not read.

## Next Steps

1. Read `CloudGroup_Init` (`0x08933048`) and the slots it calls for where the
   46 records, their positions and their half-sizes come from (the cube's
   transform scale is the first candidate). Check the result against the
   live records: the RAM dump is `data/scratch/pulse-bloom-roll/ram0.bin`,
   base `0x08800000`, but `data/` scratch is not permanent, so re-read them
   with `data/scratch/pulse-bloom-roll/roll_check.py`-style sampling if gone.
2. Then apply `CloudGroup_CullOverlappingSprites` (`clouds.md`), which only
   has work to do once there is a field of sprites.
3. Compare at a matched pose: `psp-drive.py place` at track index ~3060
   (path 2, section 34) of `oag-trace track --track 'Data\Environments\05_Track\track.vex'`,
   `psp-trace.py --camera`, then `oag-game --pose-from` with `--camera-fov`.
