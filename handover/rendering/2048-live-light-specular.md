---
status: re-first
titles: [2048, omega]
updated: 2026-11-19
summary: "Altima's violet run-off band is fc18's specular lobe from a liveLighting0 that is not the sun; trace the param-table writer"
size: M (20)
difficulty: medium (60)
---

# 2048's `liveLighting0` is not always the sun: the violet band on Altima

2026-11-19, lane `2048-leftovers`. Evidence and numbers:
[gxp.md](../../docs/formats/gxp.md), "`fc16_phong_alphaspec_normal_emissive` adds its emissive
map", last paragraph; [water.md](../../docs/rendering/water.md), "The 'rim' is not the edge".

## Open

- Altima's violet band (box `533,172,541,196` at the `altima2c` pose, original 35.4 / 34.7 / 60.4,
  ours 16.3 / 16.3 / 18.0) and the violet dotted line along the far edge. Read as fc18's specular
  `SpecTint * liveLighting0specular * D.a * lm.a * sat(R.L)^70` on the run-off picture's alpha
  patch: the band's shape is that patch. With `liveLighting0` = the `.envsettings` sun, the lobe is
  zero over the whole frame in ours, so the original binds another light there (confidence 45).
- The fc16/fc18 specular and the `s.w * sat(R.L) * lm^2` term are not wired for 2048 at all
  (ours has no 2048 specular; `light.rs` leaves `specular_scale` 0 when the key is absent).
- The orange marks on Altima's road at the same pose are not read (candidates: a decal family,
  `fc18_Blend_TrackSurface_Decal_Lerp`, or the pads).
- Omega's fc16 emissive add is **checked, applies, not wired**: its lightmapped pixel program
  (`data00.psarc`, `gcn_dump --at 0xad44`) ends `E + light * D + spec` (E unsquared), but a PS4
  fc16 submesh binds its normal map third (`psp2::normal_map`), so `E` needs a binding of its own
  before `psp2::emissive_add` can admit the file on PS4.
- Omega: its fc16 programs read the same uniforms; whether Omega binds the same light is not
  checkable without a readout.

## Next Steps

1. Ghidra, `/2048/eboot-vita-2048-eu-v104.elf`: `FUN_8101d068` registers the engine parameter
   table (`liveLighting0direction` slot `0xe`, `liveLighting0specular` `0xf`,
   `liveLighting0diffuse` `0x10`, `shadowLightSpecularDiffuse` `0x12`). Find the per-draw writer of
   those slots (xrefs to the table object `DAT_815cd380` and its set-by-index call) and what light
   it picks (nearest track light? a light list in the circuit's files?).
2. If it is a per-region or per-object light, find its data on the disc (a light list in
   `altima/` or the `.envsettings`), then wire the lobe with that colour and direction.
3. Measure `s.w` (`shadowLightSpecularDiffuse.w`): candidate `Lighting.ShadowLight diffuse nova
   scale` (0.3 on Altima), unconfirmed.
4. A Vita3K pair at the pose is optional (three boots hung on 2026-11-19 at load average 22-26);
   water.md's box means are the original's numbers; no original frame of the pose is kept under
   `data/reference/` yet (adopting one is the lead's step).
