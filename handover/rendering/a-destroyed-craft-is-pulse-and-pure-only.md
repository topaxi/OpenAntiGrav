---
status: re-first
titles: [hd, 2048, omega, pulse]
updated: 2026-11-22
summary: "A destroyed craft's wreck model, effects and states are wired on Pulse (PSP, PS2 carried) and Pure (inherited); HD/Fury, 2048 and Omega draw no wreck"
---

# A destroyed craft: wreck model, effects and states beyond Pulse

Asked for by the maintainer, 2026-11-22: every title should show its own destroyed
craft (wreck model, explosion, smoke, the states a wreck goes through), as Pulse does.

## Where it stands

- **Pulse PSP: measured and wired.** `Ship_SetState` case 5 swaps to `shipwreck.vex`
  (`oag_livery::wreck`, `oag_raceplay::scene::wreck`), `race::wreck_fx` throws the node
  blast, death sparks and the big explosion 1.5 s on, the destroy camera cuts to the
  circuit's own camera, states 6 and 8 are measured
  (`docs/ghidra/functions/psp-pulse-usa/ship-wreck-model.md`, `wreck-collision.md`,
  `camera.md` "The destroy camera").
- **Pulse PS2: carried over.** The disc ships `shipwreck.vex` per team; the port's
  `Ship_SetState` is unread (`crates/pulse/src/effects.rs`, `hull_wreck`).
- **Pure: wired by inheritance.** `Shipwreck.vex` and `Phantom_shipwreck.vex` are named
  (`crates/pure/src/race.rs`), the swap and the node effects ride Pulse's rule
  (`crates/pure/src/effects.rs`, `hull_wreck`, `Origin::InheritedFrom`); the big blast is
  Pure's own read (`docs/ghidra/functions/psp-pure-usa/wreck-big-blast.md`). Pure's own
  `Ship_SetState` and death set (`WO_SHIP_EXPLO_SMOKE`, `WO_SHIP_EXPLOSION_DEBRIS`, ...)
  are not read.
- **HD/Fury: unread, nothing drawn.** `Looks::hull_wreck` is `Rule::UNREAD`; the wreck
  trio is Pulse-gated and does not fire on HD (`docs/overview/status.md`, Particles).
  Leads: `Ship_SetState` `0x000e1e20` state 4 plays `~SHIP_ONFIRE` after `0x000d96c8`
  (the only function naming `~SHIP_DESTROYED`), state 5 calls `ShipDamageFx_Update`
  (`docs/ghidra/functions/ps3-hdfury-eu/particle-triggers.md`, item 5). Which model
  file a destroyed HD craft draws is unread.
- **2048 and Omega: unread.** The Zone wreck livery is a separate thread's item.

## Open

1. HD/Fury: read `Ship_SetState`'s destroy states (4 to 8), the wreck model (file per
   team, or the hull with a material change), the effects each state fires and where,
   the camera, and whether a wreck still collides (the per-body enable byte in
   `Collision_ProcessPairs`). Census the PSARCs for wreck models first.
2. 2048 and Omega: the same, checked against HD (lineage rule); 2048 first, Omega's
   `eboot.bin` second.
3. Pure's own `Ship_SetState` against the inherited Pulse rule; Pulse PS2's port.
4. **The Zone destruction sequence on every title** (maintainer, 2026-11-22). Pulse's is
   wired (`zonewreck.vex`, case 4's `~BLOWUP` for the local player, the death camera mode 5,
   the HUD hide on `DAT_08ab0838`; `docs/ghidra/functions/psp-pulse-usa/zone-mode.md`,
   `zone-rest.md`); its open reads (`0x08ab10e8`, `0x08ab2120`) are in
   `handover/gameplay/zone-ends-now-what-is-missing-is-the.md`. HD/Fury, 2048 and Omega:
   which model a Zone craft becomes (2048 maps a Zone death shell,
   `docs/ghidra/functions/vita-2048-eu-v104/zone-craft.md`, confidence 80), the effects,
   camera, HUD and sound at the Zone's end, are unread. Pure's Zone wreck rides Pulse's rule.
   The Zone wreck's livery is `zone-livery-wreck-lods.md`'s item.

## Next Steps

1. An `oag-re` (opus) measure lane on HD/Fury's destroy sequence (item 1), ending with
   a ready-to-wire table like `hd-contact-sparks-follow-the-ring-and-the-smoke-is-killed.md`'s,
   and a "checked against 2048/Omega" line per row.
2. Then an `oag-wire` lane: `Looks::hull_wreck` and the wreck effect triggers for HD,
   with a frame pair of a destroyed craft against RPCS3.
3. Then 2048/Omega (item 2), wired together per the backport rule.
4. Then a Zone lane (item 4): an `oag-re` measure lane on HD's Zone end first (it reuses
   step 1's `Ship_SetState` reading), checked against 2048 and Omega, then wire it with a
   frame pair of a Zone end against RPCS3.
