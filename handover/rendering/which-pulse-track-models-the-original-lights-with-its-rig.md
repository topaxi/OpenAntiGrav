---
status: measure
titles: [pulse, pure]
updated: 2026-11-19
summary: "Pulse PSP: only the start laser and hull take the circuit GE rig; which other lit scenery does is unmapped (3 pairs better, 2 worse)"
categories: [rendering]
---

# Which Pulse track models the original lights with its rig

The start laser's two textures take the circuit's rig (`docs/ghidra/functions/psp-pulse-usa/scene-light.md`,
"The start laser is lit by the rig too"); every other colourless lit track vertex keeps the stand-in.

## Open

- Giving every colourless lit track vertex the rig moved `amphiseum-black` 3.93 to 3.36, `outpost-7-white` 3.79 to 3.62 and
  `talons-boost` 9.13 to 9.00 and moved `de-konstruct-black` 3.39 to 3.46 and `outpost-7-black` 3.30 to 3.34 the wrong way:
  some lit scenery is rebuilt (rig), some is the raw texel (the 97 models whose list stayed `RET`).
- Basilico's second lit scenery strip (vertex type `0x123`, chunks 2730 and 2778 of the dump) is under the rig and is not marked.

## Next Steps

1. Read `Vex_UpdateLightLists` and what sets `model + 0xac` (the selector); mark by that, not by texture name.
2. Until then, one GE dump per circuit's grid lists the lit draws that carry a list (`psp-ge-dump.py census`, `lighting` 1); add their textures to
   `RIG_LIT_TEXTURES` in `crates/raceplay/src/load/pulse_psp.rs` only with a dump behind each.
3. The unlit raw texel for the 97 (the stand-in is an invention there): a separate change, measured per circuit.
