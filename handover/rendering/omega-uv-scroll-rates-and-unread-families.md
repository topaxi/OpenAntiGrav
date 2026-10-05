# Omega's glow layers scroll off 2048's rule; rates are chosen and several families are still

2026-10-05. `oag_rcs::rcsmodel::psp2::material::read_ps4` left `params` and
`samplers` empty, so nothing on Omega scrolled. The PS4 instance table is now
read (see `docs/formats/omega-status.md`, "The material uniform and sampler
table") and 317 glow-layer and 19 plain-scroll materials reach the existing
2048 glow plan unchanged.

## Open

- Scroll rates and the plain scroll's sign are **chosen, not measured**, as on
  2048: no GCN microcode of a scroll shader has been read (the lightmap law was,
  `docs/ghidra/functions/ps4-omega-eu/lightmap-prelit.md`).
- Families no consumer reads: `time` alone (`cf_uvanim_emssive*`,
  `mr_uvanim_em_*`, `scroller_glow_v3`), `TimeScaler` alone (`videoscreen_*`),
  HD-named scrolls (`scrollingalpha` `V_Offset`, `basic_uv_scroll` `VSpeed`),
  `lambert_spec_mult_*scroll`. They draw still.
- Vineta K's motion is subtle at race size on the panel inspected; no circuit
  was compared with a recording of the original.

## Next Steps

1. Read a scroll shader's GCN (the Omega eboot is `ps4-omega-eu`) to settle
   `TimeScaler` versus `time` and the sign, which would measure the rates for
   2048 as well.
2. Give `scrollingalpha` and `basic_uv_scroll` a V track from `V_Offset` and
   `VSpeed` once HD's meaning is checked on a PS3 capture.

## 2026-10-05, `transparent-floors`: Omega and 2048 transparency

Omega (header `+0x22`) and 2048 (`+0x12`) author HD's state word; blended and alpha-tested draws now leave the
opaque list (see `docs/formats/rcsmaterial.md`, "Omega and 2048 draw their see-through materials off the state
word"). **Open:** where Omega's per-material blend equation lives (header has no factor pair; alpha-over is
chosen, additive materials draw dim); read the GCN pixel programs for the real alpha source; use the priority
bits (9 to 11) for draw order; Omega's `etched_glass_tech` sheen and `Transparency` param; HD retune not done
(no HD code touched, no comparison taken).
