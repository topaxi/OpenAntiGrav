# HD's shield hit-flash is amber, not Pulse's cyan - and the steady-state colour is a parameter this pass did not trace to its source

2026-09-16. The user, who plays the originals, reported from play that HD/Fury's
shield "renders and animates, it just does not look exactly like in the
original (shape/color)." The renderer was drawing HD's shield through Pulse's
own recovered law and constants, title-blind - the entry-name resolution
already picked HD's own per-team `shipshield.vex`
(`crates/livery/src/entry.rs::shield_entry_names`), but the colours and
swell/scale constants painting it were `oag_render::shield`'s Pulse ones on
every title.

**Read HD's own `Shield.cpp` on `/ps3-hdfury-eu/EBOOT.elf`.** Full account in
[`docs/ghidra/functions/ps3-hdfury-eu/shield.md`](../../docs/ghidra/functions/ps3-hdfury-eu/shield.md).
The law is the same one Pulse's page already recovered - fixed ~60 Hz substep,
per-channel colour lerp, swell lerp, `sin`-shaped alpha/scale flicker, the
external-camera-vs-cockpit-sphere branch - checked field for field against
[`shield-pickup.md`](../../docs/ghidra/functions/psp-pulse-usa/shield-pickup.md).
Two colours are confirmed to differ:

- **The hit flash is amber, `(1.0, 0.2, 0.0, 1.0)`**, where Pulse's is cyan,
  `(0, 1, 1, 1)`. Read off `ShipShield_InitColours`'s own literal stores into
  a runtime colour block, and confirmed as the value `ShipShield_Hit` copies
  on an absorbed hit.
- **The activation colour is a dim red, `(0.547, 0.1, 0.1, 0.0)`**, where
  Pulse's is transparent black. Alpha is still `0.0` on both, so the shell is
  still invisible on the activation frame - only the hue the fade-up starts
  from moves.

**The steady-state colour is a call argument on HD, not a hardcoded
constant.** Pulse's `ShipShield_Activate` writes white itself; HD's takes it
from its caller. The likely caller (`FUN_000d5610`, HD's probable
`Shield_Fire`) was not traced through this binary's TOC defect in the time
this pass had - see [`README.md`](../../docs/ghidra/functions/ps3-hdfury-eu/README.md)'s
"code xrefs are sound, data xrefs are not". A per-team `ShieldColour` config
key exists and is parsed elsewhere (`FUN_000dba30`), which is a live
hypothesis for what feeds that argument, not a confirmed one.

**Ported behind a title axis.** `oag_render::shield::Palette` (`activation`,
`target`, `hit`) is a new small `Copy` type; `PULSE_PALETTE` is exactly what
`ShipShield::new()` always drew (unchanged, every existing test still passes
unmodified) and `HD_PALETTE` carries the two measured colours plus
`TARGET_COLOUR` - Pulse's white, **chosen rather than measured**, labelled so
in the doc comment, because it is the least-invented default until the real
call site is read. `Setup::shield_palette` is resolved in
`crates/game/src/race/load.rs` from `craft_title.name == oag_hd::TITLE.name`,
the same title `shield_entry_names` already keys its own path resolution off,
and threaded into `Race::start` the same way `hd_trail` is. Two new render
tests pin the palette difference and the shared-target choice
(`crates/render/src/shield/tests.rs`).

**Not captured live.** The `--give shield` debug affordance never produced a
visible activation in repeated `--screenshot` runs across a wide tick range
(10 through 900) on `talons_junction` - `holding Shield` stayed the telemetry
line throughout, which only says the pickup slot is non-empty (kept refilled
by `--give`), not that `Shield_Fire`'s bit ever set. The wiring this pass
*did* exercise end to end is the existing unit-test suite -
`race::tests::weapons::a_fired_shield_arms_the_timer_for_its_authored_duration`,
`...the_cockpit_view_swaps_the_shield_shell_for_its_sphere`,
`...absorbing_a_shield_pays_the_shields_own_absorb` and the two new palette
tests all pass - so the law is verified against the recovered constants, but
nobody has watched HD's amber flash in a rendered frame yet. Whether that is a
timing quirk of `--give`/`--press` (the grant landing one frame later than the
edge it should coincide with) or something else is unresolved; a Weapon Pad
crossing in a real race, or a rival-contact hit while carrying one, would
sidestep it.

## Open

- The steady-state target colour's real source (`FUN_000d5610`'s call
  argument to `ShipShield_Activate`) is unread. If it turns out to be a
  per-team `ShieldColour` rather than a constant white, `HD_PALETTE.target`
  needs to become per-team too, which is a bigger change than this pass made.
- `FUN_00676fe8`, HD's flicker call, takes three arguments where Pulse's
  `sinf` takes one. Kept as `sin` in the port on literal-pool-proximity
  grounds; not decompiled.
- `FUN_00125d98`, the external-camera shell draw HD's `ShipShield_Update`
  dispatches to (the sibling of the decompiled `ShipShield_DrawCockpit_q`).
  Not read at all.
- The `--give shield` CLI recipe never demonstrably fired in this pass's
  screenshot attempts - worth a look from whoever next needs a headless
  shield capture, since it is also `weapons/visuals.rs`'s own documented
  debug affordance for other pickups.
- HD's per-team `<team>_shield.rcsmaterial` is still unread (a standing gap
  from `shield-pickup.md`, not new here): the shell draws untextured and
  additive regardless of which palette tints it, brighter and flatter than
  the disc's own material.

## Next Steps

1. Resolve `FUN_000d5610`'s TOC with `scripts/ps3-toc.py` and read what it
   passes as `ShipShield_Activate`'s target-colour argument. Settles whether
   `HD_PALETTE.target` should be per-team.
2. Get one live screenshot of the amber flash - a Weapon Pad crossing in a
   real `single_race` with `--autopilot`, or a rival collision while holding
   the pickup, rather than fighting `--give`'s timing.
3. Decompile `FUN_00125d98` for symmetry with the cockpit branch, and
   `FUN_00676fe8` to settle whether the flicker really is a bare `sin`.
