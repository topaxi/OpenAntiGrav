---
status: re-first
titles: [hd, 2048, omega]
updated: 2026-11-22
summary: "Detonator races 14 stages; mines and bombs bite; cannon, score, own hull, handling and models read, not wired; EMP, auto-speed open"
categories: [rendering]
---

# Detonator races its stages without mines, EMP or auto-speed

2026-11-13, lane `modes-zone-battle-detonator`. `oag_race::Mode::Detonator` races fourteen
stages, one per lap, on HD, 2048 and Omega (`docs/gameplay/race-modes.md`, "Detonator";
evidence `docs/ghidra/functions/ps3-hdfury-eu/detonator-mode.md`). A player sees the stage
grade escalate lap by lap and the race end after lap 14, and nothing to shoot.

## Open

- **Own hull, handling file and object models: read, not wired** (2026-11-22,
  `docs/ghidra/functions/ps3-hdfury-eu/detonator-hull.md`, HD static, 60-85 per piece; 2048
  `vita-2048-eu-v104/detonator-hull.md`, Omega `ps4-omega-eu/detonator-hull.md`: checked,
  applies). What the port must change:
  1. Hull: every craft flies `/data/ships/detonator/ship.vex` (82), with the livery swap
     `zoneship_zone` -> the team's `PI_TeamModel name="detonator"` `texturelocation`
     (`zoneship_<team>`, fallback `zoneship_faisar`; 78/65), under `data/ships/detonator/`. Only
     `detonator_ao.gtf` differs per team. Also from that folder: `shipwreck.vex` (an outline
     placeholder), `ship_deathshell.vex`, `locators.vex` and `engineflare.vex`. No LOD, leach or
     shield; the absorb shell is `absorbeffect.vex`. Today `ZoneCraft` applies only in
     `Mode::Zone` (`load/zone_livery.rs`, `load/skin.rs`, `load/roster/model_livery.rs`), so
     Detonator draws the team's race hull. Omega's key is `zoneship_team` (its Zone key).
  2. Handling: `/data/ships/detonator/handlingstats.xml` (82), not Zone's. It has the same
     engine, `Turning` 2.2/1200, `Airbrake` 6/1100/sideshift 650, `Tilt` 0.3, `Misc.width` 4.0
     and its own cameras. Today `load/sim.rs` applies `ZONE_HANDLING` to every `on_zone_mask`
     mode. It needs a Detonator entry on the title (HD `/data/ships/detonator/handlingstats.xml`;
     2048 and Omega `.../hdships/Detonator/handlingstats.xml`, the same bytes). Engine lights stay
     Zone's.
  3. Mine model: `/data/weapons/mine_rita_football` + `mine_rita_spikes` (both
     `detonator_mine_normal_alpha`) + `electric_visualiser` (82), in place of `hd_mine`
     (`weapons/visuals.rs` `mine_model_matrices` feeds Detonator mines to the race Mine).
  4. Bomb model: `/data/weapons/detonator_bomb_final` + `_final_equalisers` + `_glowedges` (82).
  5. Pads: `/data/new_mode_pads/detonator_pad/detonator_pad` on every speed pad and every weapon
     pad (70). Unread: whether the original pad still draws underneath. Omega takes the path from
     a data field.
  6. Cannon (when wired): `detonator_cannonbolt`, `hd_muzzleflash_detonator`, from the
     `cannon_flash_left`/`right` locators.
  Open: the substitution table's reader, the race-side write of `ship+0x6298`, the
  `Charge1`/`Charge2` block writer (`Ship_BindDetonatorHullParams`), the mine's music-driven
  binds. `detonator_pickup` and `detonator_mine_explosion_kaleidoscopic` ship, but no loader was
  found (70, negative).

- **Layout files decoded, placement and contact wired** (`oag_tables::detonator_layout`,
  `oag_weapons::detonator`, `oag_raceplay::detonator`): HD races lay their mines and bombs per
  stage and draw them with the ordinary Mine and Bomb models. Contact is wired
  (`oag_weapons::detonator::step_contacts`: damage, velocity cut, bomb push, removal, destroyed
  craft ends the race); the explosion effects and sounds, the Detonator-specific models, the survey's collision return codes (probes are
  chosen).

- **Contact, blast and destruction: read, wired in sim, not in effects** (2026-11-22,
  `docs/ghidra/functions/ps3-hdfury-eu/detonator-mine.md` and `detonator-bomb.md`, HD static,
  confidence 72-82 per law). Mine: sphere `trigger_radius` 3, player immune for 0.5 s after
  laying, damage 10 (halved when the weapons flag is off, 60), velocity x0.5 once, no push, never
  times out, explodes and leaves the tick after contact as `WO_MINE_EXPLO_DETONATOR_DEAD`. Bomb:
  sphere 6, same grace, damage 20 to the toucher, push `30 * (1 - d/40)` to every craft within
  40, detonates the tick of contact, `WO_BOMB_EXPLO_DETONATOR`; the stage step detonates the old
  bomb harmlessly (the one-bomb rule). Energy at 0 -> `Ship_SetState(4)` -> `+0x6a64 |= 0x1000`
  -> the race ends. Open: `points_per_metre`/`max_distance` (no reader found), the cannon's hit
  and score, who spends the pending push, the update order within a tick, the PVS-gated lane
  free. 2048/Omega: data and effect selector checked, applies; contact law not read there.

- **Pulse cannon, hits, score, destroyed count: read, not wired** (2026-11-22,
  `docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`, HD static, 55-85 per law; 31 names).
  The craft's fixed cannon: clip of 15 two-round volleys (left and right `cannon_flash`), one
  volley per 0.1 s while fire is held (`t -= dt; if !(t > 0) fire, t += 0.1`; released: `t = 0`).
  Round: speed 500 (launch tick `500 + 3.6 v`), life 1.0 s, probe 9.0 down the surface normal and
  ride 3.5 above a floor hit, one wall reflection `v - 2(v.n)n` then dies on the next wall
  (`WO_CANNON_SPARKS_DETONATOR`, `CANNONEXPLWALL`). Hit test: capsule of the tick's segment vs
  radius = the object's `trigger_radius` (mine 3, bomb 6), loops do not stop at a hit. Mine: one
  hit, harmless. Bomb: 4 hits, the 4th destroys it harmlessly and sets off mines within
  `sqrt(5000)`. Reload: empty + trigger released, or the absorb button: refill 15, locked 0.75 s.
  Score into `rm+0x2df0` from `/data/xml/DetonatorScoring.xml` (not the weapon table's points,
  which nothing reads): live mine `trunc(100 * m)`, `m += 1.0` per live mine in the clip (reset at
  reload); dark 50; bomb `2000 + 500 n`; chain live `100 * m2` (reset per bomb), dark 50; streaks
  of hitting volleys in a clip 5/10/15 pay 1000/3000/5000. Count `rm+0x6de3c`: direct cannon mine
  hits + EMP kills (not chain mines, not bombs). End: `pct = count / 1860 * 100` (f32). Top block
  corrected: HUD message 8 is a bomb-ahead warning, `FUN_00067120` is the HUD/medals/trophies.
  2048/Omega: scoring file identical, streak law and round constants read alike; hit test and
  score adds not read there. Open: the fire dispatcher and manager order in a tick, the bomb-ahead
  measure, the `+0x200000` clean-lap `+1000`.

- **End screen headline is wired** (`Headline::Detonator`, `ER_ZDETONATOR_COM`): it draws on
  2048 and Omega, nothing on HD (no HD archive authors the text). Score and stage rows open.

- **Weapon table decoded, not spent.** `WeaponStats::detonator()` reads the Detonator-only
  attributes (Mine scoring, Cannon reload/rebounds, Bomb shots and points, the `EMP` block,
  `energy_recharge_per_stage`) on HD, Omega and 2048 (`docs/formats/weapon-stats.md`,
  "Detonator's attributes"). Nothing in a race reads it; each field's meaning waits for the
  law that spends it.

- **Mines and bombs: the placement law is read, nothing is wired.**
  `docs/ghidra/functions/ps3-hdfury-eu/detonator-placement.md` (HD, confidence 70-88 per piece;
  Omega read line by line, 2048 structurally: both "checked, applies, not wired"). The data is
  two global files, not per-track: `/data/xml/DetonatorLayouts/PatternFile.xml` (18 `Pattern`
  groups of `ClusterN` rows) and `GamePlayValues.xml` (`MineCountStage1..14` = 80..200, lane
  width 3.5, the 20-section jump blackout); same content on all three titles. The per-track
  `%s.xml` path is formatted and never read. Phase 4 needs a reader for both (`oag-tables`,
  generic XML) and `oag_title::Detonator` entry names; Phase 5 the `DetonatorField` (200-mine
  pool) and the placer on the seeded `Rng`. Still unread: the collision return codes the lap
  survey tests (`0/2/4/5/127`; a stand-in "lane free when the downward probe hits track" is
  needed and must say so), the route-fork walk, what the mine/bomb inits do with the matrix and
  `sec+0x40`/`+0x60`, how a mine or bomb leaves its pool (the bomb pool is 2 and the spawn does
  not check it), `0x0012ae38`/`0x0012ae10`/mine vslot `+0x3c`. Models: see the hull bullet
  above (`DetonatorMine_Construct` `0x0013a7d8`, `DetonatorBomb_Construct`); effects `WO_BOMB_EXPLO_DETONATOR`,
  `WO_MINE_EXPLO_DETONATOR[_DEAD]`, `WO_CANNON_SPARKS_DETONATOR` (2048 `data2.psarc`), whose
  triggers need the hit read first.
- **EMP.** The EMP gauge law is read (page, "What `Detonator_UpdateRace`
  does that is not wired"); the EMP pads (`data/New_Mode_Pads/Detonator_Pad`, 2048), the
  charge flag's writer (`craft+0x6a64 & 0x8000000`) and the EMP fire (`Ship_UpdateCannonTrigger`'s
  input `+0x1c` arm, `EMPManager`) are not. The cannon and its reload are (above). `docs/gameplay/weapon-lineage.md`
  already lists EMP as unwired.
- **Auto-speed.** The manual: "Like Zone mode, acceleration is automatic and ever-increasing".
  No law read; the player throttles. Read what Detonator's engine branch does (HD's hover
  code gates modes 6, `0xd`, `0xe` together, `hover-target.md`). The handling file is answered:
  `/data/ships/detonator` (hull bullet above).
- **Score, end screen, HUD.** Score and percentage laws read (cannon bullet above), not wired;
  `ER_ZDETONATOR_COM` / `ER_DETONATOR_STAGE` / `PERCENT_MINES_CLEARED`;
  `detonator_hud.xml` (the race draws the time trial's HUD). Campaign cells (`mode="Detonator"`,
  14 on `DATA00`'s Fury grids) are kept raw and do not launch.
- **The out-of-race ending** (`+0x6a64 & 0x1000`) is not modelled. Its writer is read
  (`Ship_SetState` state 4, `detonator-mine.md`); Phase 7 wires it with the contact law.
- **Omega's grade** is not drawn (Omega draws no Zone grade either); Omega's camera scale is
  inherited from HD, its craft constructor unread. **2048's per-stage `.gxt` textures** are
  unwired, as Zone's are.
- **2048 `zone_1`..`zone_4` draw a black floor** in every mode (Time Trial too): not
  Detonator's, seen while capturing.

## Zone Battle (HD mode `0xd`, Omega also `0x15`; 2048) is unread

The brief's second priority. Nothing in `docs/` recovers its rules: the stage source
`*(rm + 0x2dfc + n*4)` (per viewport) has no recorded writer
(`zone-effectsettings-loader.md`, tenth pass); `race-hud.md` names MPNitro as the Zone Battle
manager and the campaign authors it as `mode="NitroBattle"` (`AICount="7"`, `laps="0"`, a real
`NitroElim*` target triple: `crates/hd/tests/campaign_grids_ground_truth.rs`). Read recipe:
`SPNitro`'s race-manager vtable (`race-manager.md` table) for its `Update`, then
`search_instructions` for `stw` at `0x2dfc(` (folded-bias trap: also scan with
`scripts/ps3-dform-scan.py` over the update's real extent) to find what steps a viewport's
stage and what the elimination targets count. 2048 and Omega checked against it per CLAUDE.md.

**The medal side is read, the wiring waits for the race.** A `NitroBattle` cell's medal is its
race position against the rung's `1`/`2`/`3` (`<=`, unflipped; confidence 70,
`docs/ghidra/functions/ps3-hdfury-eu/cell-medal-award.md`), and `Cell::evaluate_medal_for_difficulty`
already reads it that way for a title whose `counting_modes` omits the mode. Not wired:
`oag_game::campaign::race_mode_for_cell` refuses the mode, and `RaceStage::campaign_tier` has no
value arm for `Other("NitroBattle")`, so the day it launches that arm needs the race-position
value (the original passes `param_1[0xb22]` there, a `u16` off the race record, not read).

## 2048 Combat is unread

`GameMode_GetKindLabel` (`0x812b021a`) shows `FE_GAMEMODE_ELIM` as `COMBAT`, so 2048's Combat
label is its Elimination; but `TrackStartup_Read`'s Combat board test `FUN_812b0880(mode, 1)`
(`track-startup-sound-bank.md`) is unread, so which mode id picks `321Go_2048_Combat.vex` is
not known. Read `FUN_812b0880` before mapping the board to `Mode::Eliminator` on 2048.

## Next Steps

1. Phase 4: read `PatternFile.xml` and `GamePlayValues.xml` (census on HD `DATA00`, 2048
   `data.psarc`, Omega `data00.psarc`), name them on `oag_title::Detonator`.
2. Phase 7: wire the contact law from `detonator-mine.md`/`detonator-bomb.md` (damage, velocity
   cut, push, removal, the stage-step bomb detonation, the out-of-race end).
3. Phase 9: wire the pulse cannon from `detonator-cannon.md` ("What the simulation needs"): a
   Detonator-only fixed-weapon flag (pickups stay off), a 30-round pool, the trigger/clip/lockout,
   the round's track-skimming motion and one rebound, the capsule hit test, bomb hit counts and
   the chain reaction, the score into a Detonator score, the destroyed counters and the end
   percentage. Needs a reader for `/data/xml/DetonatorScoring.xml` in `oag-tables` and its entry
   name on `oag_title::Detonator` (HD `DATA00` `data/xml/detonatorscoring.xml`, 2048 `data.psarc`,
   Omega `data00.psarc`).
4. Read Detonator's engine branch for the auto-speed law.
5. Zone Battle: the read recipe above.
