---
status: re-first
titles: [hd]
updated: 2026-11-05
summary: "Fury backdrop's level gap was RPCS3's 16-bit quad index wrap; chain, encoded blends and equaliser wired; mode 11 and a fresh-particle residual remain"
categories: [frontend, rendering]
---

# The Fury backdrop's pass graph is wired; what is left is mode 11 and a small residual

The laws and the numbers are on `docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md`: "The pass graph, wired", "The
final composite adapts", "The Fury references carry an RPCS3 defect". Read the last one first: **every RPCS3 frame of this
backdrop draws records `0..16383` three or four times and the rest never**, so no stored pair scores this backdrop. Wrapped
the same way in a scratch build, ours reads R mean `46.6` against the reference's `48.3` with nothing fitted; as shipped, every
record once, `13.9` on a silent frame. Do not fit towards the reference. The clean reference is
`handover/tooling/rpcs3-quads-wrap-at-65536-so-fury-references-are-wrong.md`.

## Open

1. **The screen change's `OnEnable` pulse** is unwired: `Fury` has no screen-change signal and its amplitude is unread,
   and it is what stands on top of `musicPulse` now the tap is in. Page: "`BackgroundAnimFury_OnEnable` fires on every screen change".
2. **Mode 11's mip chain and quads**: only once morph paths draw at all. 250 quads at 720p, 501 at 1080p.
3. **Fresh particles run `14` to `25` percent over**: defect-free slice R mean `7.3` against `6.4`, q99 `107` against `81`;
   wrapped ladder `21.8` against `17.5`. Unexplained. Ours reads an aspect of `1.7647`, exactly `30 / 17`, the shape of 480x272
   or 1920x1088, against the ring's 16:9 (`proj x 2.1148` against `2.0993`). `boot/backdrop.rs` passes `w / h` of
   `oag_display::display::viewport`; check whether that viewport or a PSP-shaped layout space is the source. It moves dots, not level,
   and may be the `3` pixel sideways residual.
4. **Lineage of the final quad's half texel, 2048 only.** Omega is `checked, differs` (static quad, UV exactly 0 and 1; `menu-backdrop.md`, "Lineage"). 2048's composite vertex program is `uv * mainScaleBias.scale + bias`; the bias carries a conditional `0.25 / dim` term (`PostFx_RenderChain` `0x810393a6`-`0x810393ee`, flag `sp+0x35c`) and the composite's own quad is unread. Recipe on the page.
5. **`[graphics] bloom` does not reach the menu**: the backdrop hard-codes `Glow::Drawn`.
6. **HD-style (non-Fury) menus end in the same present path** but `oag_game::render::SceneBackdrop` does not run this chain.
   Not measured; check the scene backdrop's exposure against a reference.

## Next Steps

1. Whether the bands hear more than music: find what else reaches MultiStream bus `0x00080003` (`zone-visualiser.md`, "Still open"), or
   fire a menu effect with a sound on RPCS3, compare a loop apart. `Mixer::music_tap` assumes music alone.
2. Not to wire: the index wrap, a `40%` fade (20 dumped frames and both builds' float scans find none), the 720p field's
   2x1 AA as anything but MSAA shaded once, and RPCS3's `sampleTexture2DMS` bug.
3. The `3` pixel sideways residual belongs to the dynamic-width thread.
