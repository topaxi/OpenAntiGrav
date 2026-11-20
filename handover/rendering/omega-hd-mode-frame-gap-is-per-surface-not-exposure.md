---
status: measure
titles: [omega]
updated: 2026-11-20
summary: "Omega HD-mode frames: tone map is not the gap; Sebenco and Moa are dark in sky and far scenery; read the original luma ring next"
difficulty: medium (45)
---

# Omega's HD-mode frames: the gap is per-surface light, not exposure

2026-11-20, `omega-hd-exposure`. Evidence, boxes and the read recipe:
[`omega-status.md`](../../docs/formats/omega-status.md), "HD-mode frame brightness: the tone map is not the gap".

## Open

- Sebenco Climb (-22.3 signed bias) and Moa Therma (-14.2) are dark, with the deficit in the sky and far scenery
  (Sebenco snow -65, far building -41, floor and wall -13 to -17). The Amphiseum (+5.0) and Modesto Heights (+0.9) have no
  brightness gap: their mean absolute grey (49.1, 40.8) is decorrelation. Correct the gallery note on The Amphiseum.
- Not known: whether the original's scene luma on the same frame is ours' (Sebenco 0.1756, Moa 0.3851) or x1.6 brighter.
- Ours' sky has no brightness term; `Lighting.Sky brightness` does not predict the sign across five circuits, so a plain
  multiplier is refuted. The sky and fog consumers in the 1.07 eboot are unread (the 2048-mode lane found its registrar,
  `FUN_015eda20`, not its consumer).

## Next Steps

1. Read the original's luma ring: native port, `SIGSTOP` after the pair grab, pointer at guest `0x021dbdb8`, ring at the
   object's `+0x1d8`; one boot per circuit. Needs a pad seat and the game copy.
2. If the ratio is near 1: the gap is shading of the sky and far surfaces (fog colour is HDR, `0.646 1.18 1.85` on Sebenco);
   read the sky and fog consumers. If near 1.6: look for a scene-wide light term the loader does not read.
3. Leave the crowd and road programs to their lanes.
