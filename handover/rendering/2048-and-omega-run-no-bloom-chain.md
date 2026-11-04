---
status: re-first
titles: [2048, omega]
updated: 2026-11-04
summary: "Omega's bloom draws since 2026-11-04 with its mask absent; the mask is per-material m_Alpha; 2048's wo_* bloom reads none"
categories: [rendering]
---

# 2048 and Omega run no bloom chain, so a glow mask has no reader

2026-10-27, lane `psp2-glow-mask` (stopped before code). The claim was that the
psp2 path stamps no glow mask, so the start-grid laser
(`fc01_effects_vscroll_emissive_alpha_glow`, additive by equivalence with HD's
`cf_startbeam_glow`) draws without HD's and Pulse's halo. Stamping would change no pixel.

## Findings

- **No bloom reads a mask on these titles.** `crates/raceplay/src/load/environment.rs:722`
  builds `hd_bloom` only when `ps3_geometry && !wo_title`. Vita 2048 is not PS3 geometry and
  Omega is `wo_title` (`looks.wo_composite`). `Scene::has_hd_bloom` is `self.hd.is_some()`.
  The only mask readers are `oag_post::hd_bloom`, `ps2_bloom` and the Pulse PSP bloom; none runs here.
  Docs agree: `2048-rcsmodel.md` ("without bloom"), `2048-status.md` (bloom and exposure unread),
  `2048-material-params.md` (`BloomFactor`/`BloomGate` authored, unread), `omega-status.md`
  (`Tonemap.*` replaces HD's `HDR and Bloom.*`).
- **HD's rule does not stamp blended draws.** `GlowMask::StampedByProgram` has
  `stamps_blended()` false and `blend_writes()` colour only; `hd-glow-mask.md` measured 53 of 59
  blended draws alpha-clear (5 write it). The start laser is blended (state `0x3d` on HD,
  `0x39`/`0x3d` on 2048), so by HD's rule it may not stamp. The Pulse-style second stamp draw is
  Pulse's rule, not HD's.
- **HD draw 551 (`cf_startbeam_glow`) has no recorded colour mask**
  (`docs/ghidra/functions/ps3-hdfury-eu/material-state.md` lists state and blend only). Whether
  HD's laser is one of the 5 stamping blended draws is unread.
- **Setting `glow_from_program` on psp2 models** (`psp2.rs` hard-codes `GpuVertex::glow: 0.0`)
  also moves the cutout targets and every opaque draw's alpha: shared-HD-adjacent, so not done.

## Update 2026-11-04, `omega-bloom-read`

Omega now runs its own chain (`oag_post::omega_tonemap::bloom`), so this thread's option A is done for Omega, and its `Tonemap.*` guess is replaced: the keys are
`Bloom.*`. Omega's bright pass does read a mask, a half-size copy of the scene's second target's `m_Alpha`, so option B is now real for Omega: without it, stamping
stays unread, and the start laser is a material whose program writes its texture alpha there. Recipe: `docs/ghidra/functions/ps4-omega-eu/tonemap.md`, "The bloom chain".
2048's `wo_*` chain has a gate and two blurs, no mask (checked, differs).

## Open

- Option A, a psp2 bloom: 2048 from `BloomFactor`/`BloomGate` or HD's chain inherited
  (labelled inherited from HD). Omega's is done (see the update above). Touches `environment.rs`
  and `oag_post` wiring. Needs: the keys' meaning (RE or inheritance), a render target the
  chain can read, a Deck CPU cost check.
- Option B, after A: mask wiring. `glow_from_program = true` on psp2 models, `GpuVertex::glow`
  per material from a table of HD `Program::output_alpha` values keyed by name and
  `lineage_blend::EQUIVALENT`, re-derived by a ground-truth test beside
  `hd_lineage_blend_ground_truth.rs`. Needs first: an `hd-draw-dump` of an HD start line to read
  draw 551's colour mask; a check that draws with no inherited value keep the alpha they have today.

## Next Steps

1. Read draw 551's colour mask (`scripts/hd-draw-dump.py`, Anulpha or Metropia start).
2. Decide A with the maintainer; B follows.
