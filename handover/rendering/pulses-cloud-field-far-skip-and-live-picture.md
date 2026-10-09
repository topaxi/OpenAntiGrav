---
categories: [rendering]
---

# Pulse's cloud field: the far-camera skip and a live A/B picture

2026-10-04, `pulse-clouds`. The field itself is done. `oag_fx::cloud`
builds each `cloudGroup`'s records the way `CloudGroup_BuildField`
(`0x08933c1c`) does: scatter, `Overlap` cull, height ramp, atlas cell. Fed a
boot's seeds, the build reproduces that boot's RAM exactly (four boots, both
`05_Track` layouts). Evidence: `docs/ghidra/functions/psp-pulse-usa/clouds.md`,
`crates/fx/tests/cloud_field_ground_truth.rs`, ADR-0056.

## Open

- **The far-camera draw skip is not read.** The original does not draw or
  advance a group when the camera is far away. Ours draws every group every
  frame.
- **No matched frame against PPSSPP's software renderer.** `psp-drive.py
  place` near track index ~3000 (path 2, section 32-34) was either undone by
  the game's reset or threw the craft off the track, on two boots. Only the
  first placement in this lane, at index 2054 with world up, settled. So the
  texel/vertex-colour combine (`FUN_0891e988(0xfdb2, 0x3e9)`) and the cell's
  V orientation are checked against RAM, not against pixels.

## Next Steps

1. Read who calls a `cloudGroup`'s `draw` slot (`0x08ad2a68` in its method
   table) and what distance or visibility test gates it. Implement it in
   `oag_fx::cloud::Layer`.
2. For the picture: drive to the clouds rather than placing there, for
   example by letting the craft run from a placement that settles (index
   ~2054). Then pause with the debugger, zero the colour word at vertex
   `+4` of `*(group+0x194)` for every kept sprite, resume three frames, grab
   `import -window root` on the Xvfb, and restore. The diff is the
   original's own cloud mask. Read the group seeds from RAM (`group+0x70`),
   build ours with them (patch `CHOSEN_SEEDS` in a scratch build), and
   render with `--pose-from` a `psp-trace.py --camera` row of the same tick
   and `--team` matching. Scripts: a throwaway script, not kept,
   `scan.py`, a throwaway script, not kept (scratch, not permanent).
