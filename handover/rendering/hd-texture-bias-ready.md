---
status: ready-to-wire
titles: [hd]
updated: 2026-10-27
summary: "HD ours is 10-20% soft near; a material LOD bias of -0.75 to -1.0 (fitted, not the filter word's -1.5) closes it; wire it"
categories: [rendering]
size: S (60)
difficulty: easy (60)
---

# HD textures read soft because the material LOD bias is held off

2026-10-27, `texture-crispness`. A table value plus a 13-pair rescore; confidence in the cause 75.
Evidence and method: `docs/rendering/hd-track-tone.md`, "Texture crispness".

## Open

- On the cold HD pairs ours reads 0.81 (near) and 0.98 (far) of the original's fine-band energy at bias 0; -0.75 gives 1.00 and 1.10, -1.5 gives 1.11 and 1.19.
- `Looks::material_lod_bias` is `Rule::UNREAD` for HD; `MATERIAL_LOD_BIAS` is -1.5 (filter word `0x02063e80`). Best fit is between -0.75 and -1.0, not understood why (RPCS3's LOD compute, or our derivative scaling in `texture_lod.wesl`).
- Also missing, not blur: the warm light pool on the Vineta K kerb, the Tech de Ra glass grid.

## Next Steps

1. Rescore with `scripts/texture-sharpness.py` on all 13 pairs at -0.75, -1.0 and -1.5 (re-render ours into a scratch copy, never the page's `data/reference`). The five-pair table is in the doc.
2. Pick the value by that fit; make `MATERIAL_LOD_BIAS` per-title data if it is not -1.5, and mark the rule `Measured` with the number, labelled as the fitted value.
3. Repeat on a second boot of the original to firm the confidence; investigate the 0.5 level gap against `textureSampleBias`.
4. Mean grey rises 0.05-0.3 on a pair; that is detail, judge with the band metric.
