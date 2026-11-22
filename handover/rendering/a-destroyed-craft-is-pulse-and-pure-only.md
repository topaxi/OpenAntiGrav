---
status: ready-to-wire
titles: [hd, 2048, omega, pulse]
updated: 2026-11-22
summary: "Destroyed craft wired on Pulse and Pure; HD/Fury states 4 to 8 read and ready to wire (ship-destroy.md), 2048/Omega twins read; Zone end unread"
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
- **HD/Fury: read, ready to wire; nothing drawn yet.** `docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`
  has the table (16 rows, static, 2026-11-22): `shipwreck.vex` replaces the hull **at state 4**
  (`Ship_SelectWreckModel` `0x000db400`), not at Pulse's state 5; one `WO_SHIP_EXPLOSION` at the
  live model's node, 4.0 back along its Y row; `EXPLBIG_PC`/`~SHIP_DESTROYED`, `~SHIP_ONFIRE`;
  the destroy camera (`Camera_SetMode` 8); the tumble on the compound collider `+0x694c` with
  `Body_SetBoxInertia(1, 12, 6, 12)` and a random kick; state 5 after 0.3 s with
  `WO_SHIP_DEATH_DAMAGE_PLUME` at the flare anchor; state 6 once the wreck has lain still for
  6 s (or after 20 s), which takes it out of the physics world and clears the `+0x70` pair
  byte, the wreck still drawn. Every ship directory ships `shipwreck.vex`. Not measured live:
  the host's packaged RPCS3 0.0.42 refuses the HD save states and a cold boot stalled at `boot`
  (the page's "Measured on RPCS3: not run" has the exact failures and the read recipe).
- **2048 and Omega: read as twins, not wired.** `Ship_SetState` is `0x811c711e` (2048) and
  `0x012ff700` (Omega) with HD's ten cases, timers and masks; both ship `shipwreck` per team
  (HD set plus their own `<team>2048/<n>`). Differences: the destroy camera is mode 9 on both,
  2048 has no rumble, Omega's death sounds are Wwise `ShipDestroyed_Player`/`_NPC`. Both carry
  a second ship family naming `WO_SHIP_EXPLOSION_PLAYER`, unread. The Zone wreck livery is a
  separate thread's item.

## Open

1. ~~HD/Fury: read `Ship_SetState`'s destroy states~~ Read 2026-11-22
   (`ship-destroy.md`). Still open there: the death kick's formula, the wreck collider's child
   shapes, camera mode 8's view against Pulse's mode 5, the `+0x7e40` call on the 4 -> 5 edge,
   and a live run of the state timings.
2. 2048 and Omega: the twins are read (`ship-destroy.md`, "Twins named here"); wire them with
   HD, by the backport rule. Their second ship family (`WO_SHIP_EXPLOSION_PLAYER`,
   2048 `0x812c7e2c`, Omega `0x0176fe30`) is unread.
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

1. **Ready to wire**: an `oag-wire` lane on `ship-destroy.md`'s table for HD: `Looks::hull_wreck`
   swapped in at the state-4 edge, `WO_SHIP_EXPLOSION` and `WO_SHIP_DEATH_DAMAGE_PLUME`, the
   0.3 s / settle / 20 s chain, the pair byte cleared at state 6; a frame pair against RPCS3 once a
   build that boots the HD states is back.
2. Then 2048/Omega (item 2), wired together per the backport rule.
3. Then a Zone lane (item 4): an `oag-re` measure lane on HD's Zone end first (it reuses
   step 1's `Ship_SetState` reading), checked against 2048 and Omega, then wire it with a
   frame pair of a Zone end against RPCS3.
