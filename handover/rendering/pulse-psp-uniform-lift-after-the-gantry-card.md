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
  mean blue bias at +0.77 against +0.94 and moves red and green the other way, vertica-white and talons-black have no fog); a global grade
  (the HUD's flat cyan bar is identical to 0.03).

## Next Steps

1. Mask the plume box on the six pairs and read its share per channel; compare ours' `engineflare` colour with a GE dump's blend state.
2. Read the quad's lighting and sampler for the green +2 on the gantry panel.
