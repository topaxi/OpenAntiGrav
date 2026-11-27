---
status: re-first
titles: [omega, 2048]
updated: 2026-11-27
summary: "Omega/2048 shell, colour and lattice animation wired; open: ambient and lit variants, Zone shell, 2048-content modes, brightness unjudged"
size: M (65)
difficulty: medium (60)
---

# Omega and 2048 draw HD's shield palette; the real values and the shell build are read

Evidence: [`ps4-omega-eu/shield.md`](../../docs/ghidra/functions/ps4-omega-eu/shield.md) and
[`vita-2048-eu-v104/shield.md`](../../docs/ghidra/functions/vita-2048-eu-v104/shield.md) (static reads,
confidence 85 at most). HD's wire is merged; this thread carries the same wire to the two titles whose
`.rcsmodel` is the Vita/PS4 container.

## Open

Landed (all three titles; evidence in the shield pages of `ps3-hdfury-eu`, `vita-2048-eu-v104` and
`ps4-omega-eu`, "The lattice scrolls"): shell build, per-team colour, the lightning layer on the HD-content
shells and cockpit of both consoles, the magnetic shell's `u + time * Speed_multiplier`, and (2026-11-26) the
shell's scene clock, without which none of the shader terms ever moved.

1. **The colour rate depends on the mode.** `0.15` for HD-content modes (id `< 0x17`), `0.05` for 2048-content
   modes. Unreachable: nothing launches a mode `>= 0x17`; `Palette::colour_rate` and `CONTENT_2048_COLOUR_RATE`
   are ready. The same mode picks `Data/Weapons2048/vr_shield_cockpit` over `Data/Weapons/`.
2. `constantAmbientColour` (implicit 1), the lit variants' light terms, and the fog order (ours adds the
   lightning after fog) are unread or unmatched.
3. Zone's `zonebattle_shield` (HD, Vita, Omega) is unread; no mode of ours raises it.
4. Omega's PS4 program does not square the lightning tap where the Vita's does; whether `cf_plasma` is sampled
   through an sRGB view decides whether Omega's term should be decoded (ours takes it raw).
5. The cockpit sphere's blend on 2048 and Omega stays the chosen alpha-over.
6. No original frame of a raised shield has judged brightness (wipeout_pc for Omega, Vita3K for 2048).
7. Their shell materials carry the back-face cull bit (`0x3d`, magnetic `0x39`) and ours draws both faces;
   HD's live draw culls (`ps3-hdfury-eu/shield.md`, "The shell's draw state"). Checked, applies, not wired.

## Next Steps

1. Capture a raised shield on wipeout_pc (the maintainer launches it) and on Vita3K; judge brightness, the
   lightning decode (item 4) and the cockpit blend.
2. Read `zonebattle_shield` only if Zone Battle gets a `Mode`.
