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

- Dark pairs, by surface: Sebenco Climb (-22.3 signed bias) sky and far scenery (snow -65, far building -41, floor and wall
  -13 to -17); Modesto Heights (+0.9) the sky only (-36); Moa Therma (-14.2) the stands (-16.1) and stripe road (-14.6),
  sky level. The Amphiseum (+5.0) has no brightness gap (ceiling and stands are brighter in ours; its 49.1 is
  decorrelation). Moa's stands belong to the queued crowd-lighting lane.
- Not known: whether the original's scene luma on the same frame is ours' (Sebenco 0.1756, Moa 0.3851) or x1.6 brighter.
- No texel of Sebenco's luma image reaches 0.97, where the other circuits reach 1.6 to 8: not located, so whether the HDR
  fog colour (`0.646 1.18 1.85`) is clamped on far geometry, or the sky draws at its texel with no brightness term, is open.
- `Lighting.Sky brightness` does not predict the sign across five circuits, so a plain multiplier is refuted. The sky and
  fog consumers in the 1.07 eboot are unread (the 2048-mode lane found the registrar `FUN_015eda20`, not a consumer).

## Next Steps

1. Read the original's luma ring: native port, `SIGSTOP` after the pair grab, guest pointer `0x021dbdb8`, ring at the object's
   `+0x1d8` (N-1 slots; offset unconfirmed, `ToneMap_UpdateCoefficients` is unresolved on 1.07; raw luma versus past `LAvg`
   not pinned); one boot per circuit. Needs a pad seat and the game copy.
2. If the ratio is near 1: the gap is shading of the sky and far surfaces; read the sky and fog consumers. If near 1.6: look
   for a scene-wide light term the loader does not read.
3. Locate Sebenco's brightest luma texels (256 to 1920 scaling) to split the sky from clamped fog.
