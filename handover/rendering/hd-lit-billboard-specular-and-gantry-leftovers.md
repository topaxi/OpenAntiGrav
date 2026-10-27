---
status: ready-to-wire
titles: [hd]
updated: 2026-10-26
summary: "no-dot-power specular landed 2026-10-26; tv_glow.gtf second texture and the Omega card route are open"
categories: [rendering]
difficulty: medium (30)
---

# HD: lit materials with no `pow` chain still draw the stand-in specular; the card's remaining leftovers

2026-10-18, `gantry-hd-alpha`. The gantry blur lane is closed (card sampler wraps now,
`docs/rendering/start-gantry.md`, "the card sampler wraps"). What it left:

## Open

- ~~**No-chain programs draw a specular the original does not.**~~ Landed 2026-10-26 (`hd-top-right-band`,
  `docs/formats/rcsmaterial.md`, "A program with no `LG2` reading a dot product draws no specular"):
  `NO_SPECULAR` is now any program with no `LG2` reading a dot product (`Program::has_dot_power`), `no_specular`
  in `shade.wesl` no longer needs `colour_factor`. `billboarddiffuse` and `cf_vex_billboard` (no `LG2` at all) are
  in it. The census asked for here ran as a 13-circuit dump of every material's verdict; census of which
  drawn blocks carry an `LG2` is replaced by the dot-product test (the lightmap curve has an `LG2` too).
- **`tv_glow.gtf` on `02_track`.** `cf_vex_billboard` binds it as a second texture and it does
  not reach the shader (no `ADD_SECOND` bit, slots word `0x18`).
- **Omega.** `adverts: None`, so no card route; the shared lineage suggests the same quads.
  Not checkable on a sampler (no PS4 capture path).

## Next Steps

1. ~~`rg -c LG2` per drawn block over the HD materials that report `specular_exponent_unresolved`.~~ Done by
   `has_dot_power` (2026-10-26).
2. ~~Gate `NO_SPECULAR` per family.~~ Gated by program, not family; no golden moved.

## Open from `gantry-finish` and `hd-cards-unlit` (2026-10-18)

Landed: every HD card (slots 2, 3, 5, 7, 8) is drawn unlit (`Adverts::adverts_unlit`; `docs/rendering/start-gantry.md`, "every card is
unlit"). What is left:

- ~~**FINAL LAP panel residual.**~~ Closed 2026-10-19 (`hd-frame-brightness`): the 0.63-0.66 frame and 0.53-0.56 panel ratios were a `Write Color Buffers`-off reference. WCB-on re-shot: whole frame 1.06, panel within glyph flicker; no code change (`docs/rendering/start-gantry.md`, closing note in item 2).
- **The finish.** Static reading says the flag persists; no RPCS3 capture (the lap counter cannot be forced, branch `lap == 0`
  wins). A real three-lap finish on RPCS3 would settle it.
- Pulse's `JonsFinalLap:Final_Lap` against the PPSSPP software renderer, and Pulse's `Adverts::adverts_unlit` (`false`, unread): with
  `gantry-pulse` / `pulse-card-verify`.
- The advert slots on the posts: Campaign slot 7 is `Blitzed_board.vex` in the original and `fx350` in ours.
