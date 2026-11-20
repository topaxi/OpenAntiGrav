---
status: re-first
titles: [omega]
updated: 2026-11-20
summary: "Omega HD-mode: original's scene luma read live: 1.23-1.64x ours on Sebenco/Moa/Modesto, equal on Amphiseum, by surface; read sky/fog consumers"
difficulty: medium (45)
---

# Omega's HD-mode frames: the scene's light level differs by surface, exposure does not

2026-11-20, `omega-hd-exposure` then `omega-luma-read`. Evidence, boxes, the ring layout and the per-pair numbers:
[`omega-status.md`](../../docs/formats/omega-status.md), "HD-mode frame brightness: the tone map is not the gap" and "the
original's luma ring, read".

## Open

- **Settled by `omega-luma-read` (native port, eight boots, ring read through `/proc/<pid>/mem`):** the original's whole-frame scene luma
  (`LAvg`, settled at the grab) is Sebenco 0.2290/0.2299, Moa 0.4720/0.4740, Modesto 0.4197/0.4299, Amphiseum 0.1324/0.1325 against ours'
  0.1756, 0.3851, 0.2621, 0.1333: ratios 1.30, 1.23, 1.60 to 1.64, 0.99. `E` equals ours on Sebenco, Moa and Amphiseum (pinned) and is
  0.60 against ours' 0.868 on Modesto. Neither "equal" nor "1.6 times" holds across the four.
- Inverting the original's frames with its own `E`, `t1` and curve puts the excess in surfaces: Sebenco sky and far building about 2.3 to
  2.5 times ours, floor and wall 1.1 to 1.4; Modesto 1.4 to 2.2 everywhere (sky most); Moa sky and floor 1.0, stands 1.4, stripe road 1.6;
  Amphiseum ceiling and stands 0.67 to 0.73 (ours brighter). Bloom (ours draws none) inflates the bright rows, saturation deflates them.
- Not known: which scene term carries it. The sky and fog consumers in the 1.07 eboot are unread (`FUN_015eda20` is only the registrar);
  `Lighting.Sky brightness` does not predict the sign; no texel of Sebenco's luma image reaches 0.97 where the other circuits reach 1.6 to 8
  (whether the HDR fog colour `0.646 1.18 1.85` is clamped on far geometry, or the sky draws with no brightness term, is open).
- Moa's stands belong to the queued crowd-lighting lane.

## Next Steps

1. RE: find the consumers of `Lighting.Sky brightness`, `Lighting.Fog colour` and the sky material in the 1.07 eboot (patch program
   `/omega/eboot-ps4-omega-eu-v107.bin`) and read what multiplies the sky and far geometry; the Sebenco snow sky (2.3 to 2.5 times ours)
   is the test case, Moa's sky (1.0) the control.
2. Acceptance numbers for any wiring: ours' `LAvg` on the four pairs should reach 0.229, 0.472, 0.42 and 0.132 (the readback patch
   is not kept; `omega_tonemap`'s `frameL` is the quantity), and Modesto's `E` should fall to about 0.6.
3. Re-read the ring on Vineta K, Metropia and the other seven HD-mode circuits if the wiring is to be checked beyond these four (one boot
   is 80 s; the recipe is in `omega-status.md`).
