---
status: ready-to-wire
titles: [hd, 2048, omega, pulse]
updated: 2026-11-25
summary: "Destroyed craft wired on all titles; Zone end wired on HD, 2048, Omega; open: camera wiring, collider, Pure diffs, Zone look and velocity, Omega sound"
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
  `Ship_SetState` is read against Pulse's (`docs/ghidra/functions/psp-pure-usa/ship-destroy.md`, item 3
  below): mostly the same, four differences ready to wire.
- **HD/Fury: wired (rows 1, 2, 9, 10, 11 partly, 12), 2026-11-22.** `Title::destroyed` carries
  the timers and the two edges (`oag_title::destroyed`), `Looks::hull_wreck` loads
  `shipwreck.vex` for every team, `WO_SHIP_EXPLOSION` is thrown on state 4 and the death plume
  on state 5, a settled wreck leaves the pair test at 6 s and the world with it. Sound wired 2026-11-23 (`ship-destroy.md`,
  "Sound on HD"). Not wired: row 6 (collider, kick), row 5's camera is wired (`destroy-camera.md`, "Wired"), rows 4, 7 and 14, the `body+0x210`
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
  wired, as on HD: sound (Omega's is Wwise `ShipDestroyed_*` and no `.bnk` carries it; 2048's bank resolution is unread, both keep `~BLOWUP` via `DestroyedCraft::HD_BLOWUP`; 2048 has no rumble), camera mode 9
  (wired 2026-11-23, `destroy-camera.md` "Wired"), collider and kick, the `_n1` set's `WO_NITRO_SHIP_DEATH`
  (modes 13 and 21), the second ship family's `WO_SHIP_EXPLOSION_PLAYER` (unread).

## Open

- **The Zone end's debris fills the screen** (ours, HD, `--force-wreck 400:0`, frames 2026-11-23): the Zone end's camera is wired (the close orbit, `destroy-camera.md` Z2a to Z2f) and the throw velocity is chosen, so at 2.5 s the 112
  `zoneship_dead.vex` pieces cover the whole frame around the camera. Wire Z2's orbit and read Z11's throw speed
  before judging the picture against the original.
  **Update 2026-11-24 (`zone-orbit-clip`):** the orbit's start distance was wrong (0.5 off the craft, inside the hull); it was then given a fitted eye ease, and on 2026-11-25 (`zone-orbit-eye-read`) the render camera was read live: the eye is `focus + dir x (5 + dist)` with no smoothing, now wired, and the hull push-out is gone (`destroy-camera.md`, "The orbit eye"). Still open: the debris's throw speed (Z11) and the white shell's wire showing from the first frame where the original shows it from about 1 s.

- **The HD blast frame reads as a soft black ball with sparks and no fire** (ours, `--force-wreck 60:7` on
  Talon's Junction, 2026-11-22, `hd-wreck-wire`). **Cause found 2026-11-23 (`hd-black-blast`, `ship-destroy.md`,
  "The black blast"):** the root emitter of `WO_SHIP_EXPLOSION`, a fire flipbook (`smoke_new4_8x4`) under a
  `colour_mode` 3 table that is black for entries 1 to 31, multiplied; 2048 authors the same table. Not a decode,
  blend or missing-texture bug, and nothing is fixed. ~~**Open, RE:** what `colour_mode` 3 does~~ Read
  2026-11-23 (`hd-colour-mode3-re`, `ship-destroy.md`, "`colour_mode` 3 is a blend toward the scene light",
  84): not a multiply. Each byte becomes `trunc(E0 + (L - E0) * table[idx].R / 255)`, where `L` is
  `RibbonEffects_SampleLighting` at the instance. The root is white (entry 0) from life 1/256 to 1/8,
  then `L` from 0.16. **Wired (`pob-colour-mode-wire`):** `oag_title::Effects::colour_walk`
  (`LightBlend` on HD, `Reversed` on 2048 and Omega's 2048 set), applied at load by
  `oag_fx::psys::Effect::with_colour_law`. `L` is `oag_fx::psys::LIGHT` = the ribbon's white, **chosen, not
  measured**, until a live `ctx+0xb04` read (recipe on the page) gives it, so the blast tint is white for now.
  `WO_SHIP_EXPLOSION` is listed in `oag_hd::effects::SPRITES` and draws the orange fireball (frames on the page). Omega applies the reversed walk to `Data\particles2048` only.
  2048 plays modes 1 and 3 as the reversed walk (white, then black for the last eighth), and
  Omega's 2048 reader matches. Ours walks forward on every title, so on 2048 and Omega every
  mode 1 or 3 emitter is affected, not only the blast. HD's mode 1 is the plain forward walk. Still open: HD's frame-advance sources (count, rate, `[lo, hi]`; partly read on the
  same page), so listing the effect in `oag_hd::effects::SPRITES` may still give a white blowout.

1. ~~HD/Fury: read `Ship_SetState`'s destroy states~~ Read 2026-11-22
   (`ship-destroy.md`). Still open there: the death kick's formula, the wreck collider's child
   shapes, ~~camera mode 8's view against Pulse's mode 5~~ (read 2026-11-23, `destroy-camera.md`), ~~the `+0x7e40` call on the 4 -> 5 edge~~ (read and wired 2026-11-24, `ship-destroy.md`, "The debris throw"; open there: the nitro variant `0x00105450` (32 pieces), `Debris_Update`'s body-state branches, the torque's size, how gravity is applied, 2048 and Omega's piece paths),
   and a live run of the state timings.
2. ~~2048 and Omega wired~~. Their second ship family (`WO_SHIP_EXPLOSION_PLAYER`,
   2048 `0x812c7e2c`, Omega `0x0176fe30`) is unread.
3. ~~Pure's own `Ship_SetState` against the inherited Pulse rule~~ read 2026-11-23, static, 28 rows. **Wired 2026-11-23 (`pure-wreck-wire`)**: rules measured, gaseous flash, `EXPLSMALL`/`EXPLBIG`, the wreck hidden from the big blast on Pulse and Pure (`ship-destroy.md`, "Wired"); **left: the Phantom wreck (row 12, no Phantom hull ever loads in ours)** and the unread rows below. The paragraph that follows is what was read, before the wiring
   (`docs/ghidra/functions/psp-pure-usa/ship-destroy.md`). **Same as Pulse, so the wire lane flips
   `crates/pure/src/effects.rs` from `Origin::InheritedFrom("Wipeout Pulse")` to `Origin::Measured`:**
   `hull_wreck` (the swap on the state 5 edge, row 11), `Trigger::WreckNode` and `Trigger::WreckSparks`
   (rows 15, every Pure wreck has 3 to 7 locators), and the timers and destroy camera rows 1, 6, 7, 10, 24, 25.
   **Differs, ready to wire on Pure:** the `explosion_gaseous.vex` flash (`ShipGaseousFlash_Construct`
   `0x0885a7d8`) at the 4 -> 5 edge and again at the big blast in place of Pulse's shockwave ring (rows 8, 21);
   `Phantom_shipwreck.vex` for a Phantom craft (row 12, after a Phantom hull loads at all); `EXPLSMALL`/`EXPLBIG`
   for every craft with no `_PC` variant and no `SHIP_DEST` (rows 14, 18). **Same on Pulse and Pure, missing from ours on
   every title that uses Pulse's rule:** the wreck is hidden from the big blast on (its matrix is squashed `1e-4` and
   the per-state update skips the live model's matrix in state 6; Pure live, Pulse static at `0x08841e50`), where
   `oag_raceplay::scene::wreck` draws it for all of `Eliminated` (row 22). **Unread:** row 16 (`90.0` on the
   death sparks' instance), row 19 (big flash kind 6 against Pulse's 7), row 28 (state 7's callers). The state timings
   (0.5, 1.5, 2.0 s), the swap and row 22 (the wreck stays squashed in state 6) are measured live on PPSSPP
   (`scripts/psp-pure-wreck-states.py`). Still open: Pulse PS2's port.
4. **The Zone destruction sequence on every title** (maintainer, 2026-11-22). Pulse's is
   wired (`zonewreck.vex`, case 4's `~BLOWUP` for the local player, the death camera mode 5,
   the HUD hide on `DAT_08ab0838`; `docs/ghidra/functions/psp-pulse-usa/zone-mode.md`,
   `zone-rest.md`); its open reads (`0x08ab10e8`, `0x08ab2120`) are in
   `handover/gameplay/zone-ends-now-what-is-missing-is-the.md`. Still unread for the Zone end: 2048's
   `Ship_UpdateWrecked` arm (the shell animation) and both twins' race-manager side; everything
   else below is read on all three titles. Pure's Zone wreck rides Pulse's rule.
   The Zone wreck's livery is `zone-livery-wreck-lods.md`'s item.
   **HD/Fury: read 2026-11-23, wired 2026-11-23 (`zone-destroy-wire`: shell, hide, debris, `shipdestroy` on HD, HUD hide, results at 5.5 s; 2048 the same with its own cue, Omega without it; `zone-destroy.md`, "Wired")** (`docs/ghidra/functions/ps3-hdfury-eu/zone-destroy.md`,
   rows Z1-Z12, static, 2048 and Omega lines per row). Mode 6 is Zone, mode 14 Detonator. A Zone
   craft never becomes a wreck (`shipwreck.vex` is never selected in Zone, so the Zone wreck livery
   is unused there): `shipdestroy` from the environment bank after every cue is stopped, camera
   mode 2 on the craft, state 4 for one tick, the white shell `zoneship_dead_whiteshell.vex`
   (drawn over the hull all race) animated `1 - max(0.4 t, 0)` over 2.5 s, then hull, flare and
   shell hidden and 112 `zoneship_dead.vex` debris pieces thrown with the body's velocity (no
   fire); HUD hidden on the blast, `EndRace Results` 1.0 + 4.5 s later. 2048 and Omega: checked,
   applies, not wired for every row but no rumble on 2048, Omega's extra camera-2 condition and
   shell visibility, and Omega's `shipdestroy` event container (not checkable by bank census).
   Unread, each with its address on the page: ~~camera mode 2's per-frame law~~ (read, `destroy-camera.md`), the debris launch
   velocity, the `0x003f5878` pool, 2048's `Ship_UpdateWrecked` arm, the `+0x5f9c` branch.

## Next Steps

1. ~~HD wire lane~~ landed (above). Left on HD: rows 3 and 6 (needs the death kick formula and
   collider read), row 5 (wire `destroy-camera.md`'s "Ready to wire" table onto `oag_render::camera::destroy`: per-title stations, substep easing, random rate factors, wobble, the 8 s hand-back), a frame pair against RPCS3 once a build boots the HD states.
2. ~~2048/Omega wired~~ (above). Left there: sound (`hd-wreck-sound` did HD; 2048's banks and Omega's Wwise events are unread), mode 9's view (read, `destroy-camera.md`: HD's mode 8 plus the hold mode 8 rule, W8).
3. Zone (item 4): ~~read~~ ~~wired on HD, 2048 and Omega~~. Left: a frame pair of a Zone end against
   RPCS3 with the page's read recipe (the debris velocity and the shell's look are chosen or inherited
   until then; camera mode 2's orbit is read on HD, 2048 and Omega), 2048's and Omega's `shipdestroy`.
