# `viewProj` is located; the capture harness still reports the wrong camera

2026-09-05, successor to the RPCS3 capture-harness thread, which closed when
its one open question was answered. The finding and its evidence are in
[rpcs3-capture.md](../docs/reverse-engineering/rpcs3-capture.md) ("`viewProj`
is in the pushbuffer, and here is where") and
[renderer.md](../docs/ghidra/functions/ps3-hdfury-eu/renderer.md)
(`Rsx_UploadVertexConstantBlock`, `0x005c18d8`). **What is left is
implementation, deliberately not attempted in the same sitting as the RE.**

`scripts/rpcs3-drive.py capture` boots HD headless, walks into a race, loops
poses in one debugger connection and pairs each frame with a camera - and the
camera it writes into `NN.json` is **wrong**, and has been for every capture in
`data/reference/` so far. `ps3_pose.candidates` slides over raw bytes looking
for sixteen consecutive big-endian floats. That finds a static cube face, or a
run of denormals with a lone `1.0` in the `w` slot
(`data/reference/hd-capture/talons/00.json` is exactly that), and against the
pushbuffer it finds an eye on the X axis at 11.0 units with a `fov` of 39.

The real matrix is the operand of a **count-17**
`NV4097_SET_TRANSFORM_CONSTANT_LOAD` - header `0x00441efc`, then one register
index, then the sixteen floats - loading `c[256]` and `c[260]`. Measured live
at `0x40077308` in two of three shots of a Talon's Junction race, with `aspect`
1.777778, unit error 7e-08, `fov_y` 60.0001 and an eye that moves 232 then 430
units over two six-second intervals.

## Open

- `ps3_pose` has no notion of a packet, so nothing in the harness can find the
  camera that was located. Every `NN.json` under `data/reference/hd-capture/`
  carries a wrong pose and should be regarded as unpopulated, not as data.
- The discriminator that actually picks the camera out is **multiplicity**, not
  a score: `c[256]` takes ~100 *distinct* values a frame (it is a per-object
  `worldViewProj`), and the camera is the one value that is single-valued
  within a frame and different between frames. `ps3_pose.score` is
  corroboration after that filter, not the filter. Whether to encode the
  cross-frame half in the harness - which needs two frames' dumps in hand - or
  to settle for the score alone on one frame is an open design question.
- Whether `c[256]` and `c[260]` really are `viewProj` and something else, or
  the same matrix under two names, is not read. Both receive the identical
  matrix eight or nine times a frame.
- The count-5 emitter (`Rsx_UploadVertexConstants`) writes a `float4x4` as four
  rows 24 bytes apart. Nothing needs it yet, but a finder that only handles the
  count-17 form should say so rather than look general.
- Not verified end to end: no frame has yet been rendered from the recovered
  pose and laid over the captured PNG. That is the test that would settle the
  handedness and the `N + 256` register mapping at once.

## Next Steps

- Teach `scripts/ps3_pose.py` a packet-aware finder: scan for the `0x00441efc`
  header, take the index and the sixteen floats that follow, and score those
  candidates rather than every byte offset. Keep the byte-sliding finder for
  non-pushbuffer regions. **This is a `scripts/` change, so it needs the full
  `just` gate, not the docs-only carve-out.**
- Have `cmd_capture` default its `--region` set to the pushbuffer
  (`0x40010000:0x4000`, `0x40060000:0x20000`, `0x40080000:0x20000`) rather than
  the EBOOT data segment, which is measured to hold nothing but static cube
  faces.
- Then re-capture Talon's Junction and render `oag-game --camera-pose ...
  --screenshot` from the recovered pose, and compare against the captured PNG.
  A frame that lines up settles the convention; one that is mirrored or
  inverted says which of transpose/handedness is wrong, and that is a cheap
  thing to bisect once the two images exist.
- Re-derive the **`camera` field** in every `data/reference/hd-capture/*/NN.json`
  once the finder works. **Only that field is wrong**: the `track` name, the
  pairing with `NN.png` and the PNGs themselves are load-bearing and must stay -
  `talons/`'s barrier walls are the independent confirmation of the flip fix,
  and `hd-talons-glass/` is what
  [talons-junctions-missing-floor-is-a-glass-floor.md](talons-junctions-missing-floor-is-a-glass-floor.md)
  compares against. They are gitignored, so this is housekeeping, not a commit.
- When the overlay is taken, **crop to the real RPCS3 window rectangle first**.
  The shared Xvfb on `:77` never repaints, so a previous boot's window survives
  the black trim and reads as part of the frame - see
  [rpcs3-capture.md](../docs/reverse-engineering/rpcs3-capture.md)'s 2026-09-01
  ghosting trap. Three boots landed on `:77` on 2026-09-05 alone. A misaligned
  overlay is a ghost until that is ruled out, not a handedness bug.
