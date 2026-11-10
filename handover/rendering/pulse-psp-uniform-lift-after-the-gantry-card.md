---
status: measure
titles: [pulse]
updated: 2026-11-10
summary: "Gantry card format wired (5-6-5 truncated); a uniform +0.7 grey on R, G, B remains, largest near the craft plume"
categories: [rendering]
---

# Pulse PSP: what is left of the "systematic blue lift" after the gantry card

Surveyed on the six best Pulse PSP race pairs (ours minus theirs, mean R/G/B +0.4/+0.6/+0.9, B stddev -0.6). Decomposition on
the pair copies, and the one law that moved it, are in `docs/rendering/start-gantry.md`, "Pulse's card is 5-6-5, truncated".

## Open

- Uniform +0.7 on R, G and B (GO board, HUD blocks and plume box masked, six pairs). Candidates by area: the plume and hull glow near the
  craft (white blobs in the difference image on four pairs; ours' plume brighter on vertica-white and metropia-white, dimmer on
  tech-de-ra-black), structured texture detail. Flat-in-both pixels agree exactly (63 to 65 % differ by 0), so there is no whole-frame
  rounding or grade term to recover.
- The panel's green reads +2 against the original after the card change, and the panel is drawn through the blend pipeline
  (texel alpha ~0.97 scales it) where the original's panel prims have blending off. Read the quad's own colour path next.
- The original's panel has a 4 to 5 px pattern ours lacks; the `GO` letters' brightness differs by the captured tick's fade phase.
- Hypotheses ruled out with numbers: a 16-bit main framebuffer or dither (the main target is 8888, `theirs.png` has 255 or 256 levels per channel,
  the dither matrix registers are zero and the 4x4-phase residual is flat to 0.05); distance fog (turning it off leaves the six pairs'
  mean blue bias at +0.77 against +0.94 and moves red and green the other way, vertica-white and talons-black have no fog); a whole-frame grade
  (63 to 65 % of the scene pixels flat in both frames differ by exactly 0).

- `billboards.md` names `FUN_0891fe14` as the 128 x 128 render-target creator, but in the USA `BOOT.BIN` as opened in Ghidra it is `jr ra; nop`:
  the format was read from the GE state instead; find the real creator and confirm the `FRAMEBUFPIXFORMAT` it sets.
- Texture decode (`CLUTFORMAT` `0xC5` in the dump) was not tested.
- Whether the original's fine hatch on the `GO` letters and on non-gantry advert panels (visible at 3x, absent in ours) is the card's own texel pattern or a
  write-time effect is not read.

- **Not the hull, 2026-11-10 (`pulse-hull-sheen`).** The hull's extra pass was 40 % too strong on
  Metropia (a law, wired: `mesh-draw.md`, "The extra pass's colour is the light the hull stands
  in"); fixing it took the whole-frame signed grey from +0.97 to +0.90 on Metropia White and +0.76
  to +0.67 on Black and moved nothing on the other four best pairs, whose frames are bit-identical.
  On craft-box pixels the pass does not touch, the original minus ours reads -2.2 to +0.5 levels
  across the six pairs (five of six negative), about the frame's lift, so the lift is on the
  hull's base colour too and is not a sheen term.

## Next Steps

1. Mask the plume box on the six pairs and read its share per channel; compare ours' `engineflare` colour with a GE dump's blend state.
2. Read the quad's lighting and sampler for the green +2 on the gantry panel.
