# HD, 2048 and Omega render to an HDR target and tone-map it; we draw into an 8-bit one

2026-10-05, queued by the maintainer. No work started. Three questions, one
thread, because they share one stage: what the original's colour pipeline is
between "shader output" and "pixel on screen", and whether ours matches it.

What is already established, do not re-derive:

- **No title's textures are 10-bit or HDR.** HD's 7,333 `.gtf` are 7,154
  DXT1/3/5 plus 8-bit RGBA (`docs/formats/gtf.md`); Omega's 1,407 `.gnf` are
  1,378 BC7 sRGB, 11 BC4, 4 BC1, with no BC6H (`docs/formats/gnf.md`). The high
  range comes from lighting (lightmap curve, emissive, bloom, Omega's prelit
  scale of 2.5), not from texels.
- **Omega renders HDR**: its pixel shaders export fp16 (`v_cvt_pkrtz_f16_f32`),
  and every circuit's `.envsettings` carries a `Tonemap.*`/`TonemapHDR.*` block
  (exposure minimum/maximum/response/time, luminance and source-colour
  coefficients). `oag_tables::envsettings::Tonemap` reads it; **nothing applies
  it**, because its consumer is not located (next address: the
  `wo_composite_*` registry at `0x01623500`). See `docs/formats/omega-status.md`
  around "The `Tonemap.*` block is read".
- **HD** carries an `HDR and Bloom.*` block in its `.envsettings`, read for the
  bloom chain (`handover/rendering/hds-frame-was-too-bright-and-too-bloomy.md`).
  HD's own render-target format and any tone curve are **not recorded**.
- **2048** carries `1 0 1 0` where Omega has the prelit scale/bias/power; its
  target format is not recorded either.
- Ours draws every title into a saturating 8-bit target
  (`docs/rendering/fsr3.md`, "Auto-exposure is off": "This renderer draws into
  a low-dynamic-range target").

## Open

- ~~**Omega: HDR target plus its tone-map.**~~ **Closed 2026-10-05
  (`omega-tonemap`)**: consumer `ToneMap_ApplyEnvSettings` (`0x01620980`), law read
  and wired as `oag_post::omega_tonemap`; see
  `docs/ghidra/functions/ps4-omega-eu/tonemap.md` and `docs/formats/omega-status.md`.
  Left open from it:
  - **Altima's road is a flat colour** with the curve on (pink, untextured; it
    clipped to white before). Some road surface on that 2048-heritage circuit is
    lit far past 1.0 or drawn without its texture. Check its material and
    lightmap binding before blaming the curve:
    `altima-t300-before-after.png`. The 3.08 %
    still clipped at tick 600 (`altima-t600-before-after.png`) is that same
    road, not a hot light.
  - The scanout format (sRGB or UNORM) is unread, so the final `pow(1/2.2)` is
    chosen. Next address: the SDR copy `FUN_0122d760` called from `FUN_01629ce0`,
    and the display buffer's creation.
  - Which level of the 256x256 luma chain the coefficient pass samples, and what
    image the chain starts from (`ToneMap_UpdateCoefficients`, `0x0178c490`).
  - The Pro checkerboard resolve `0x19685a0` (same `m_abcd` constants), unread.
  - Omega's bloom and its low-resolution additive layer go on after the curve in
    the original. Neither is drawn here yet.
- **HD and 2048: do they do the same?** Read each executable's final composite
  and its target format before assuming Omega's design. A title that genuinely
  renders LDR keeps our 8-bit path.
- **"HD's track textures feel a bit off"**, maintainer report, 2026-10-05.
  Make it a measurement: same team and circuit as an RPCS3 capture (software
  renderer not applicable on PS3; use RPCS3's own frame dump), compare
  luminance and saturation of the road and walls region by region. A missing
  tone curve shows as a *systematic* curve between ours and the original's
  (a remap that fits every region at once); a lighting or texture defect shows
  as a regional one. That is the discriminating test. Do not tune to match.
- **Should a craft take on a surface's local colour?** Maintainer question,
  2026-10-05: some circuit parts are "dyed" red - should a ship passing through
  light up red? Nothing in the docs answers it. What is known: HD sums a
  per-frame SPU point-light list into vertex colours; one producer is wired
  (each craft's engine light), and **23 other producers are catalogued and
  unwired** (`docs/rendering/README.md`, "SPU vertex lights"; `renderer.md`).
  Any track-placed coloured light would arrive through that list or a light
  volume (`Lighting.Debug.Draw light volumes` exists as a setting). The
  falsifier: an RPCS3 capture of a craft driving through a red section, hull
  RGB sampled before, inside and after. A hull that shifts red means a local
  light reaches it and its producer is to be found; a hull that stays put
  means the red is baked into the track's lightmap only, and ours is right not
  to tint the ship. Check the same on Omega and 2048 only if HD says yes.

## Next Steps

1. ~~Omega's tone-map consumer and law, then the fp16 target and the curve.~~
   Done 2026-10-05; the Altima road item above is the follow-up.
2. One RPCS3 capture session on HD that answers both the "feels off" curve
   test and the red-section hull test, same circuit, same team.
3. HD's and 2048's final composite read, only if step 2 shows a systematic
   curve.
