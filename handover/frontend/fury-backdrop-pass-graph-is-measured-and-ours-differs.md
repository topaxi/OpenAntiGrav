---
status: ready-to-wire
titles: [hd]
updated: 2026-11-05
summary: "Fury backdrop's level gap was RPCS3's 16-bit quad index wrap; chain and encoded blends wired; equaliser, mode 11 and a fresh-particle residual remain"
categories: [frontend, rendering]
---

# The Fury backdrop's pass graph is wired; what is left is the equaliser, mode 11 and a small residual

The laws and the numbers are on `docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md`: "The pass graph, wired", "The
final composite adapts", "The Fury references carry an RPCS3 defect". Read the last one first: **every RPCS3 frame of this
backdrop draws records `0..16383` three or four times and the rest never**, so no stored pair scores this backdrop. Wrapped
the same way in a scratch build, ours reads R mean `46.6` against the reference's `48.3` with nothing fitted; as shipped, every
record once, `13.9` on a silent frame. Do not fit towards the reference. The clean reference is
`handover/tooling/rpcs3-quads-wrap-at-65536-so-fury-references-are-wrong.md`.

## Open

1. **The equaliser tap** (a separate lane; nothing here stands in for it). Bands `+0x1b4..+0x1f0`, banks `+0x19c`
   and `+0x1dc`, threshold `g+0x1708`. Shipped silent: pulse `0.23`, glow at its floor, pivot off by `-0.05`. This is now
   the largest thing between the shipped picture and a player's.
2. **Mode 11's mip chain and quads**: only once morph paths draw at all. 250 quads at 720p, 501 at 1080p.
3. **Fresh particles run `14` to `25` percent over**: defect-free slice R mean `7.3` against `6.4`, q99 `107` against `81`;
   wrapped ladder `21.8` against `17.5`. Unexplained. The `0.7` percent narrower field of view (ours `2.1148` against the ring's
   `2.0993` in `x`, an aspect of `1.7647`) moves dots but should not move level; check it first.
4. **The half texel sits ahead of the chain** in our present pass, where the original's ladder reads the field directly, so
   the gate sees two half-dark rows. Move it into the resolve when `handover/rendering/hd-race-final-quad-half-texel-unread.md`
   settles whether the chain wants it for a race too.
5. **HD-style (non-Fury) menus end in the same present path** but `oag_game::render::SceneBackdrop` does not run this chain.
   Not measured; check the scene backdrop's exposure against a reference.

## Next Steps

1. Pick up the equaliser with a clean reference in hand if there is one; it can land without.
2. Not to wire: the index wrap, a `40%` fade (20 dumped frames and both builds' float scans find none), the 720p field's
   2x1 AA as anything but MSAA shaded once, and RPCS3's `sampleTexture2DMS` bug.
3. The `3` pixel sideways residual belongs to the dynamic-width thread.
