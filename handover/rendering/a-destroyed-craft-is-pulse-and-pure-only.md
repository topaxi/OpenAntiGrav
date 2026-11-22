---
status: ready-to-wire
titles: [hd, 2048, omega, pulse]
updated: 2026-11-23
summary: "Destroyed craft wired on Pulse, Pure, HD, 2048 and Omega (HD sequence); open: sound, camera mode 8/9, collider and kick, Zone end"
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
- **HD/Fury: wired (rows 1, 2, 9, 10, 11 partly, 12), 2026-11-22.** `Title::destroyed` carries
  the timers and the two edges (`oag_title::destroyed`), `Looks::hull_wreck` loads
  `shipwreck.vex` for every team, `WO_SHIP_EXPLOSION` is thrown on state 4 and the death plume
  on state 5, a settled wreck leaves the pair test at 6 s and the world with it. Not wired: row 3
  (sound), row 6 (collider, kick), row 5 (HD's own camera), rows 4, 7 and 14, the `body+0x210`
  half of row 11, a live RPCS3 pair. Table: `ship-destroy.md`, "Wired on HD". The text below is
  what was read.
- **HD/Fury, as read before wiring.** `docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`
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
- **2048 and Omega: wired with HD's data, 2026-11-23.** `Title::destroyed` is `DestroyedCraft::HD`
  (inherited), `hull_wreck` is on, `WO_SHIP_EXPLOSION` and the death plume are named; every team and
  Omega's 2048-era ships load a wreck. Our blast frames show fire (Altima, Modesto Heights). Not
  wired, as on HD: sound (Omega's is Wwise `ShipDestroyed_*`; 2048 has no rumble), camera mode 9
  (unread, HD's inherited camera runs), collider and kick, the `_n1` set's `WO_NITRO_SHIP_DEATH`
  (modes 13 and 21), the second ship family's `WO_SHIP_EXPLOSION_PLAYER` (unread).

## Open

- **The HD blast frame reads as a soft black ball with sparks and no fire** (ours, `--force-wreck 60:7` on
  Talon's Junction, 2026-11-22, `hd-wreck-wire`). Pulse's `WO_SHIP_EXPLOSION` draws an orange textured
  fireball. Check HD's `WO_SHIP_EXPLOSION` against an original frame (or its `.pob` templates' blend and
  sprites) before trusting the picture: a black fireball is the shape a wrong blend mode or missing sprite takes.

1. ~~HD/Fury: read `Ship_SetState`'s destroy states~~ Read 2026-11-22
   (`ship-destroy.md`). Still open there: the death kick's formula, the wreck collider's child
   shapes, camera mode 8's view against Pulse's mode 5, the `+0x7e40` call on the 4 -> 5 edge,
   and a live run of the state timings.
2. ~~2048 and Omega wired~~. Their second ship family (`WO_SHIP_EXPLOSION_PLAYER`,
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

1. ~~HD wire lane~~ landed (above). Left on HD: rows 3 and 6 (needs the death kick formula and
   collider read), row 5, a frame pair against RPCS3 once a build boots the HD states.
2. ~~2048/Omega wired~~ (above). Left there: sound (`hd-wreck-sound` lane), mode 9's view.
3. Then a Zone lane (item 4): an `oag-re` measure lane on HD's Zone end first (it reuses
   step 1's `Ship_SetState` reading), checked against 2048 and Omega, then wire it with a
   frame pair of a Zone end against RPCS3.
