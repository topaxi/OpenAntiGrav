# oag-post

Post-processing between a scene and the surface: bloom, tonemap, motion blur,
SMAA, FXAA and FSR 1 and 3. Each stage reads a finished offscreen frame and writes
another, with no knowledge of what drew it.

Depends on `oag-gpu` and `oag-core` only among workspace crates, and not on
`oag-mesh`.
