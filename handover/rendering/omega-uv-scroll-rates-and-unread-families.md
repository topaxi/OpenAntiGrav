# Omega's glow layers scroll off 2048's rule; rates are chosen and several families are still

2026-10-05. `oag_rcs::rcsmodel::psp2::material::read_ps4` left `params` and
`samplers` empty, so nothing on Omega scrolled. The PS4 instance table is now
read (see `docs/formats/omega-status.md`, "The material uniform and sampler
table") and 317 glow-layer and 19 plain-scroll materials reach the existing
2048 glow plan unchanged.

## 2026-10-06, `psp2-scroll`: what moved

The glow layer's rate is now `TimeScaler` where authored (still chosen) else
`1.0`, HD's engine clock (inherited: HD's identical records play it); the authored
`time` is no longer read. HD's vertex scroll is ported by name and authored rate
hashes (`vertex_scroll::inherited_rate`), 2048 and Omega both, labelled inherited
in the load report. Evidence and per-family reasons:
`docs/formats/2048-material-params.md`, "Inherited from HD". `basic_uv_scroll`
`VSpeed` is wired (Next Steps 2 below is done); `scrollingalpha` `V_Offset` is on
no circuit material. Still open below: a GCN read, the plain scroll's sign,
`TimeScaler`-alone, flipbook, inline-`time`, and a frame where an inherited
surface is isolated (the one candidate on Anulpha Pass is confounded by node
animation; the plain scroll on Altima's "WELCOME" ring is proven by a static
camera diff, `data/scratch/psp2-scroll/shots/sign_diff.png`).

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
word"). **Open:** where Omega's and 2048's own per-material blend equation lives (neither authors a factor pair; 2026-10-05
`omega-2048-materials` inherits HD's pair by name for 70 names and draws the rest alpha-over, chosen; the
2048 model-material pass that calls `sceGxmShaderPatcherCreateFragmentProgram` is not located, 22 callers of
`FUN_812f6bee` are not it); read the GCN pixel programs for the real alpha source; 16 2048 materials carry state
mode 3 (`fc06_lambert_alpha`) and draw opaque; 2048 Tower's lit floor still saturates (`Lighting.Exposure*`
authored, no consumer found); use the priority
bits (9 to 11) for draw order; Omega's `etched_glass_tech` sheen and `Transparency` param; HD retune not done
(no HD code touched, no comparison taken).
