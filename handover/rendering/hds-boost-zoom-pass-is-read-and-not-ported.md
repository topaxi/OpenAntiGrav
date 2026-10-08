---
categories: [rendering]
---

# HD has no motion blur; its boost and damage zoom-streak ring is read, not ported

2026-10-08, lane `hd-motion-blur`, answering the maintainer's question "does HD have motion
blur or a speed or boost blur". Evidence and law:
[funklayer-zoom.md](../../docs/ghidra/functions/ps3-hdfury-eu/funklayer-zoom.md). The short
version: no velocity or camera blur; a `FunkLayerZoom` ring pass that an event switches on
(a Turbo for about 1.2 s, damage for 0.6 s) and that does not run on speed alone. Nothing was
ported because three inputs to a faithful port are open. The motion blur setting's
`original` value is not offered on HD (it is not a motion blur).

## Open

- **The ring's accumulation weight.** The bloom chain keeps a ping-pong pair
  (`0x02300000`, `0x022c0000`) and draws the previous one over the current quarter-resolution
  scene each frame. Solving `X = (1 - a) * cur + a * Y` on dumped buffers gave `a = 0` at 438
  km/h (rms 0.29, exact) and `a = 0.048` at `E = 0.339`, so the weight looks tied to the
  boost pulse (about `0.15 * E`), which would make the boost smear carry real previous-frame
  ghosting. Two reads so far (see the page); more `(E, a)` pairs are in
  `data/scratch/hd-motion-blur/run8` when it finished.
- **What starts the boost pulse** (`FUN_0029ef40`, a virtual call with no static reference).
  A Turbo does. Speed pads, the start boost and rolls are untested.
- **The damage tint** `(1 - 0.9 P, 1 - 0.08 P, 1)` was read from the CPU loop, not seen on a
  frame with a large `P` and its mesh.
- **`oag_post::hd_bloom` models none of it**: not the ring, not the accumulation, not the
  pulse. The chain is otherwise measured against the reference at rest.
- Radial bloom (`zone_1` only) and the seven DoF programs: not read; neither ran in a race
  frame here.

## Next Steps

1. Read the open `(E, a)` pairs, then decide the accumulation weight: if `a = 0.15 * E`
   holds at four points, take it as measured. About 30 minutes with
   `scripts/rpcs3-hd-postchain.py --plan boost:0.2 ...` and `data/scratch/hd-motion-blur/ema.py`
   (promote it to `scripts/` when used again).
2. Find what fires `FUN_0029ef40`: break on a write to `0x00ad7880` (the glow state array)
   with `scripts/rpcs3_debugger.py`, then press a speed pad, roll, and take the start boost.
3. Take one frame with `P` above 0.5 (a hard wall hit during a Turbo) with
   `HOOK_TGT=cc0000` and `HOOK_LIGHT=0` for the colours.
4. Wire it as HD's own pass in `crates/post` (HD only through `Title` data, ADR-0058): a
   24-quad ring (inner radius 0.62, outer 1.5, 15 degree steps), vertex alpha 0 inside and
   `max(E, P)` outside, outer UVs pulled toward the inner vertex of the same angle by 0.15
   (tap 0) and 0.95 (tap 1), the two-tap fragment arithmetic and the blend in the page, the
   `E` pulse law, size jitter and `P` decay, matched against `run*` frames at the same `E`.
   An effect the original has needs its own setting value only if it is a blur; it is not.
5. Pulse and the other titles: the thread
   `do-the-originals-have-motion-blur.md` keeps Pulse PSP/PS2, Pure, 2048 and Omega.
