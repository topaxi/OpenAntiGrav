---
status: measure
titles: [pulse]
updated: 2026-10-12
summary: "Cloud mask by difference not obtained; the original's visible-set rule differs from ours"
categories: [rendering]
---

# Pulse's cloud field: what is left after the group gate and the matched frame

2026-10-12, `pulse-clouds`. The field is built and reproduces four boots' RAM
exactly from their seeds. Two things closed today, both in
`docs/ghidra/functions/psp-pulse-usa/clouds.md`:

- the far-camera skip is a PVS gate on the group's parent `Section` node (a
  group is neither drawn nor advanced when its section's bit is clear in the
  view section's row), implemented in `oag_fx::cloud::Layer`;
- a matched frame against PPSSPP's software renderer: the same pose and the
  same seeds put every puff on the same pixels (RMSE 0.0053 over the cloud
  patch).

## Open

- **The original's cloud mask by difference was not obtained.** Zeroing the
  vertex colour word, or writing solid green into it at the `Gu_CallList`
  breakpoint, did not change the picture. A mask would pin the texel-by-colour
  combine (`FUN_0891e988(0xfdb2, 0x3e9)`) arithmetically rather than by tone.
- **Our visible set is the craft and camera rows with adjacency padding; the
  original uses the one row of the craft's section.** Clouds use our set's
  mask. They differ only at section edges.
- **The original advances `phase` per rendered frame, ours per simulation
  tick.**

## Next Steps

1. If the combine matters: capture the original's cloud-free frame at the
   same state (a save state taken just before a group's draw, loaded twice)
   and difference it against a normal one.
