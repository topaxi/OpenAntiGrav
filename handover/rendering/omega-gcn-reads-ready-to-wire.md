---
status: ready-to-wire
titles: [omega]
updated: 2026-10-27
summary: "Omega GCN reads: tangent w is ±1 raw, hologram is HD law without /2, 1.07 composite adds ColorScale; three wiring lanes"
difficulty: medium (40)
---

# Omega's shader reads are in; three are ready to wire

2026-10-27, `gcn-operand-decoder`. `oag_rcs::gcn` decodes every GCN program Omega ships
([gcn.md](../../docs/formats/gcn.md); `cargo run -p oag-rcs --example gcn_dump`). Three HD findings that sat as
"Omega: not checkable" were read with it. None is wired.

## Open

1. **Omega tangent frame** (reader DONE 2026-10-27: `psp2` reads type `0xe`, `|w| = 1` on 36.4M vertices; **left open: the draw**. The `psp2` route has no normal-map binding at all and no PS4 normal-sampler rule. Census: tangent-carrying submeshes name `Normal`, `NormalMap`, `NormalTexture` or unnamed `Texture1,Texture2` (diffuse_normal_specular, weapon_pads). Next: read from the GCN pixel program which sampler is decoded `2x-1` and feeds N, bind it third, pack `s = w = +-1` as bytes 0/255 through a pack helper shared with `baked_tangent`; PS4 only, 2048 stays unbound. Omega pairs on the compare page are only Anulpha and Vineta K.) (mesh, `oag_rcs::rcsmodel::psp2`). Omega's tangent is four signed bytes, type nibble `0xe`
   (the same as `normal`'s), and `w` is exactly ±127 on every vertex measured; every Omega vertex program reads `w`
   raw. `psp2.rs`'s tangent reader accepts only the Vita's type `5`, so **no PS4 tangent is read** and Omega's
   tangent-space normal maps draw without a frame. Wire: accept type `14` on a PS4 file (signed bytes, `/127`), bake
   the scale as signed (the `TangentFrame::Signed` path HD's wiring added), on **every** Omega circuit. Evidence:
   [hd-track-tone.md](../../docs/rendering/hd-track-tone.md), "Tech de Ra's near-left wall", Omega paragraph. Check
   against an Omega frame of Tech de Ra (shadPS4 race shots are untrusted for tone, so judge the bump's direction,
   not brightness).
2. **DONE 2026-10-27** - **Amphiseum hologram board on Omega** (mesh). Same law, units and constants as HD's mode 7
   (`PROGRAM_STATIC_HOLOGRAM`), **without the `/2`**; Omega's draw goes through the `psp2` material route, which does
   not run HD's mode 7 today. Evidence: hd-track-tone.md, "The Amphiseum hologram board", Omega paragraph. Before
   porting, settle whether HD's `/2` is HD's `output_scale` (that section's "The `/2`") so Omega's port drops exactly
   that and nothing else.
3. **The 1.07 composite** (post, `oag_post::omega_tonemap`). **Ready to wire with `ColorScale = (1, 1, 1)`** (the
   patch's source global is initialised `(1,1,1,1)` and rests there; confidence 65; only a fade drives it lower, gate and
   trigger unread). The patch moves `ScreenTint` outside the vignette and multiplies `ColorScale` in. Evidence:
   [colorscale.md](../../docs/ghidra/functions/ps4-omega-eu/colorscale.md),
   [tonemap.md](../../docs/ghidra/functions/ps4-omega-eu/tonemap.md), "Read with the decoder". Only matters once
   bloom, tint and vignette are drawn at all.

## Later

- Name the PS4 Pro opcodes the Sea Islands tables do not list (VOP1 84-91, VOP2 50-58, VOPC 201-206); they occur only in
  `_1`/`_2` (Pro) materials, which no model names, so no wiring needs them.
- What selects `_1` against `_2` at run time (eboot).
- 2048's tangent `.w` and hologram (`hologram.rcsmaterial`) programs: needs USSE operands (`gxp.md`).

## Next Steps

1. Lane 1 first: it touches every Omega circuit's normal-mapped surfaces.
