---
status: re-first
titles: [hd, 2048, omega]
updated: 2026-11-22
summary: "Detonator: stages, mines, bombs, hull, cannon rounds, explosions and their sounds on HD; EMP, auto-speed, HUD, end screen, campaign open"
categories: [rendering]
---

# Detonator: the plan, what is landed, and what to do next

Written for the next agent. `oag_race::Mode::Detonator` races fourteen stages (one per lap) on
HD, 2048 and Omega (`docs/gameplay/race-modes.md`, "Detonator"). The work is a 15-phase plan;
phases 1-7 plus the RE of phases 8 and "8b" (hull) are committed on `main`, nothing is
uncommitted. Read this file, then `docs/ghidra/functions/ps3-hdfury-eu/detonator-*.md` (the
evidence), then start at "Next Steps".

## The plan and its status

| # | Phase | Status | Commit |
| ---: | --- | --- | --- |
| 1 | Decode the Detonator-only weapon-table attributes (`WeaponStats::detonator()`, `oag_title::Weapons::detonator`) | done | `8f008cf7b` |
| 2 | End-screen headline `ER_ZDETONATOR_COM` (`Headline::Detonator`; draws on 2048/Omega, nothing on HD: no HD archive has the id) | done | `d890b438a` |
| 3 | RE: stage placement of mines and bombs (`detonator-placement.md`) | done | `543e8953b` |
| 4 | Read `PatternFile.xml` / `GamePlayValues.xml` (`oag_tables::detonator_layout`, `oag_title::DetonatorLayouts`, `docs/formats/detonator-layouts.md`) | done | `8a6918978` |
| 5 | Place mines and bombs (`oag_weapons::detonator::place_stage`, `World::detonator`, `oag_raceplay::detonator`, drawn with the ordinary Mine/Bomb models) | done | `dbb59fd37` |
| 6 | RE: mine/bomb contact, blast, destruction (`detonator-mine.md`, `detonator-bomb.md`) | done | `57d0847fb` |
| 7 | Hazards bite: `step_contacts` (damage, velocity cut, bomb push, removal), destroyed craft ends the race; the race loads the Detonator weapon table | done | `6a4c6be71` |
| 8 | RE: pulse cannon, hits, score, destroyed count, percentage (`detonator-cannon.md`) | done | `0db7e825c` |
| 8b-RE | RE: own hull, own handling file, own mine/bomb/pad models (`detonator-hull.md`) | done | `bed94feed` |
| 8b | Wire the hull, handling, mine, bomb and pad models | done (wreck, death shell, colour binds open) | `4f4b70173` |
| 9 | Wire the cannon, score, destroyed count and percentage | simulation done; rounds, flash, explosion effects and the cannon's cues drawn on HD; no HUD, no end screen | `59215fc52`, the `det-fx` commits |
| 10 | EMP: RE (pads, charge flag writer, `EMPManager`) then wire | open | |
| 11 | Auto-speed: RE Detonator's engine branch, then wire | open | |
| 12 | Detonator HUD (`detonator_hud.xml`, `hud_detonatorweapons.xml`, `detonator_warnings.xml`) | open | |
| 13 | End screen proper (score, percentage, stage rows) | open | |
| 14 | Campaign launch (`race_mode_for_cell` refuses `Other("Detonator")`) | open; do not ship before 9 | |
| 15 | Omega and 2048 parity leftovers | open | |

A race can already be launched from the command line:
`oag-game --race --mode detonator [--autopilot --ticks N] data/images/hdfury-ps3-eu-dec.iso`
(HD EU disc in `data/images/`). Menu launch is phase 14.

## What a player gets today (HD)

14 stages; each stage lays up to its `MineCountStage` mines and at most one bomb; a mine costs 10
energy and half the speed, a bomb 20 and a push; energy at zero ends the race. Every craft flies
the Detonator hull in its team livery on Detonator's own handling file; mines, bombs and pads draw
the Detonator models (the mine football draws plain white: its `MineColour`/`MineBrightness` binds
and the bomb's `BombColour` are unread). Square fires the pulse cannon and Circle reloads; rounds
destroy mines and bombs and the race keeps a score, a destroyed count and a percentage, but no
the rounds and muzzle flashes are drawn and a shot mine, a bomb and a round on a wall play their disc effects (HD); the HUD is the time trial's. No EMP, no auto-speed.

Open from 8b: no wreck or death shell (HD's PS3 hull branch loads no wreck in any mode; the
Detonator `shipwreck.vex` is an outline placeholder); the circuit's own pad still draws under the
Detonator pad (unread whether the original hides it; the model's origin at the pad transform is
chosen); 2048 reads the object models from HD's `Data\Weapons` prefix (its loaders' prefix unread).

## Working rules learned the hard way

- Docs-only RE phases go to a `worker` subagent (`model anthropic/claude-opus-5-5`, sync) with a
  self-contained brief; check `just check-docs check-names check-handover check-status` (run
  `just gen-status` after new `names.tsv` rows), then commit docs separately from code.
- Each code phase: `cargo fmt --all`, `just lint`, `just gate-affected`, then
  `just test-data-affected`. **Three `portuguese_br_ground_truth` tests in `oag-raceplay` fail on
  a clean checkout; ignore them.** Run ignored ground-truth tests with
  `cargo test -p oag-raceplay --profile quick --test it_a_g -- --ignored detonator_ground_truth`.
- `oag-tables` integration tests live under `tests/it.rs` (add modules, not files).
- **`World` size ratchet** (`crates/gameplay/src/world.rs`, `the_world_snapshot_does_not_grow`):
  raise `LIMIT` with a comment line naming the field. `World::detonator` hashes only when
  non-empty (`hash.rs::write_detonator`), so determinism references do not move: keep any new
  Detonator state empty/zero outside the mode, and never edit the reference constants.
- Mark a stand-in "chosen, not measured"; every RE claim needs a confidence (CLAUDE.md).
- The autopilot flies through mines and dies or crawls: ground-truth tests about stages use
  `dodge`/`keep_alive` in `crates/raceplay/tests/detonator_ground_truth.rs`.
- Scratch `#[ignore]` tests must be removed before committing.

## Where the code is

- `crates/tables/src/weapons/detonator.rs`, `detonator_layout.rs`: tables.
- `crates/title/src/{weapons,race}.rs`: `Weapons::detonator`, `Detonator`, `DetonatorLayouts`
  (entries wired in `crates/{hd,2048,omega}/src/race.rs`).
- `crates/weapons/src/detonator.rs` (+ `detonator/tests.rs`): `Field`, `Placed` (an entry index
  and a lane; position comes from `Survey::point`), `place_stage`, `Contact`, `step_contacts`.
- `crates/raceplay/src/detonator.rs`: the lap survey (collision probes, **chosen**),
  `advance_detonator` (contact, then stage laying; called after `update_standings` in `tick.rs`),
  `detonator_mine_position` (tests); `assets.rs::{load_weapons, load_detonator_layout}`;
  `weapons/visuals.rs` (`detonator_matrices`, `detonator_matrix`); `scene/weapon_models.rs`.
- `crates/gameplay/src/{world,hash}.rs`: `World::detonator`.

## Next Steps

### Phase 8b: hull, handling, models (small, all data read)

Source of truth: `detonator-hull.md` (and the "Own hull..." bullet under Open). Do in this order:

1. **Handling.** `/data/ships/detonator/handlingstats.xml` (2048/Omega
   `.../hdships/Detonator/handlingstats.xml`, same bytes) instead of Zone's for `Mode::Detonator`.
   Today `load/sim.rs` applies `ZONE_HANDLING` to every `Mode::on_zone_mask` mode (~l.159,
   ~l.502); add a Detonator entry on the title data (a `Title` axis, ADR-0058: no
   `title.name ==` branching, `just check-title-branching`). The hover rig and no brakes stay the
   mask's; engine lights stay Zone's.
2. **Hull and livery.** `ZoneCraft` (`oag_title::race::zone_circuit`/`race.rs`,
   `ShipPaths::hull_entry_name(team, zone: bool, ..)`) is Zone-only and `zone: bool` is threaded
   through `raceplay/src/load/{zone_livery,skin,muzzle,roster/model_livery,view}.rs`,
   `oag_livery` (`zone_livery::swap`, `entry.rs`, `lib.rs` mode field) and `oag_game::preview`.
   Generalise it to a per-mode craft (`Zone` | `Detonator`): hull
   `/data/ships/detonator/ship.vex`, livery swap `zoneship_zone` -> the team's
   `PI_TeamModel name="detonator"` `texturelocation`, fallback `zoneship_faisar`; wreck, death
   shell, locators, engine flare, `absorbeffect` from that folder; no LOD/leach/shield. Omega's
   key is `zoneship_team`. Start by reading how `Mode::Zone` reaches each of those callers
   (`rg 'Mode::Zone' crates/raceplay/src/load crates/livery crates/game/src/preview*`).
3. **Models.** Mine = `/data/weapons/mine_rita_football` + `mine_rita_spikes` +
   `electric_visualiser` (the visualiser is music-driven: draw the first two, say so);
   Bomb = `detonator_bomb_final` + `_final_equalisers` + `_glowedges`;
   pads = `/data/new_mode_pads/detonator_pad/detonator_pad` on every speed and weapon pad.
   `weapons/visuals.rs::{mine,bomb}_model_matrices` feed the race Mine/Bomb pools today; give
   Detonator objects their own model pools in `scene/weapon_models.rs`.
4. Ground truth on HD (hull entry resolves, handling parses, models load); 2048/Omega entry
   resolve checks like `the_detonator_layout_entries_resolve` in `crates/2048/tests/patch_ground_truth.rs`
   and `crates/omega/tests/omega_title_ground_truth.rs`.

### Phase 9: the cannon, score, percentage (design worked out, nothing written)

Law: `detonator-cannon.md` ("What the simulation needs"). Decisions already made:

- **Tables:** add `Scoring` + `parse_scoring` to `oag_tables::detonator_layout` for
  `Data\XML\DetonatorScoring.xml` (`<Bullet multiplier new_mine_score old_mine_score
  bomb_hit_score bomb_score bomb_increment>`, `<Emp new_mine_score old_mine_score bomb_score>`;
  disc: 1.0,100,50,0,2000,500 / 50,25,1000), entry name `DetonatorLayouts::scoring` on HD, 2048,
  Omega (same file on all three; resolve-check like the layout files), loaded in
  `assets::load_detonator_layout` into `oag_weapons::detonator::Layout`. Note
  `GamePlayValues::default()` has `cannon_lockout_*` and `bomb_chain_reaction_distance` at 0.0;
  the disc authors 0.75 / 0.75 / 5000 (the chain radius is `sqrt(5000)`, a squared distance).
- **Placed gains `live: bool`** (laid this stage; fits the 4 bytes) and `place_stage` darkens
  every surviving mine first (`MineManager_DarkenAll`): a shot live mine scores `100 * m`, dark 50.
- **State** in `Field` (stay `Copy`, `const fn new`, hash when non-empty, add an `armed` flag
  so a non-Detonator `Field` stays empty): clip (15), trigger timer, ready, lockout and its
  total, reload-pending streak reset, ring index and `hits[30]`, streak award latches, 30 rounds
  (`pos, prev, vel, up, normal, age, rebounds, slot, dead`), per-bomb hit counts (swap with the
  bomb array on removal), score (i32), `m_mine`, `m_chain`, bombs-shot `n`, mines-destroyed and
  bombs-destroyed counters, trigger-held ticks. Raise the `World` ratchet (~2 KiB).
- **Inputs:** fire = `Button::Square` held, reload = `Button::Circle` (pressed; edge-vs-level
  unread, **chosen**), slot 0 only; pass the snapshot into `advance_detonator`
  (`tick.rs` ~l.557 has `inputs`). Use weapons-on damage rules as `step_contacts` does. A
  Detonator-only fixed-weapon gate: do not flip `Mode::weapons_enabled()` (asserted false).
- **Rounds:** `step_cannon` generic over `Craft` with a sweep closure
  `Fn(Vec3, Vec3) -> Sweep {None, Floor{point,normal}, Wall{point,normal}}`; raceplay builds it
  from `CollisionWorld::raycast` (`Surface::Wall` = wall, `Floor`/`MagFloor` = floor, `Reset`
  ignored). Probe 9.0 down the surface normal and ride 3.5; velocity rebuilt to
  `unit(forward perp up) * 500`; one wall reflection (`-2.0`, `+3.0` out); life 1.0 s; capsule
  hit radius = the object's `trigger_radius`; mine/bomb loops do not stop at a hit; a bomb takes
  `number_of_shots_to_destroy` (4) hits, the 4th removes it harmlessly and chains mines within
  `sqrt(5000)` (scored, **not** counted). Count `destroyed_mines` for direct mine hits only.
- **Muzzles:** from `locators.vex`'s `cannon_flash_left/right` in the Detonator hull folder
  once phase 8b lands (do 8b first); until then offset from the craft by its dimensions
  (**chosen**).
- **Percentage:** `destroyed_mines / sum(MineCountStage1..14) * 100` in f32 (`1860` on the disc),
  exposed from raceplay for the end screen; streaks 5/10/15 pay 1000/3000/5000 per
  `detonator-cannon.md`.
- Unread, keep as noted: `EnergyRechargePerStage` (10 on the disc; spent each stage step,
  `detonator-placement.md`, 60: not wired), the 0.5 s owner grace, the `+0x200000` clean-lap
  `+1000`, the bomb-ahead warning (HUD message 8).
- Tests: unit (shots to destroy, score adds, streaks, percentage, rebound, reload) in
  `crates/weapons/src/detonator/tests.rs` (extract a `cannon` submodule; the file is ~600 lines,
  the limit is 1,500, inline test mods 500); an HD ground-truth test (fire while standing in a
  field: score monotonic, destroyed count > 0); a hash test that a fired round moves the hash.
  Draw the rounds with `detonator_cannonbolt` once 8b has the models.

### Phases 10-15 (from the original plan)

- **10 EMP.** RE first (docs-only): the writer of `craft+0x6a64 & 0x8000000`, the EMP pad source
  (HD `data/new_mode_pads/detonator_pad`, 2048 `Detonator_Pad`), `table+0x1bc` (an `EmpStats`
  field), `EMPManager` update (what firing clears and scores: `Emp` score values are read), the
  gauge `craft+0x7da0` (law read, 70: `(n - (hi - lo)) / (hi - lo)` clamped to 1, `EMPREADY` /
  `EMPFULL` edge cues). Then wire pad detection (`raceplay/src/pads.rs`), gauge, fire input
  (`in+0x1c`), cues as per-tick outputs (ADR-0018). `MineManager_DestroyAtLane` /
  `BombManager_DestroyAtLane` are the kill callers. Gates add `check-weapon-lineage`.
- **11 Auto-speed.** RE what mode `0xe` does under mask `0x206040` in `Craft_Update` (does it use
  Zone's start/increment law keyed on the stage?). Do not borrow Zone's law without the read.
  Wire through a `Title` data axis (`start.rs` ~l.668, `player_step.rs` ~l.228,
  `pilot_assist.rs` is Zone-gated: check). Ground-truth: a coasting craft reaches speed.
- **12 HUD.** RE the widget bindings for mode 14, then `HudLayouts` gains a Detonator layout
  (HD/Omega `/data/xml/detonator_hud.xml`, 2048 `Data\XML\Detonator_HUD.xml`), bound to the state
  from phases 5-10; atlas `detonator_hud2` (`raceplay/src/hud.rs`). Files:
  `crates/title/src/hud.rs`, `crates/{hd,2048,omega}/src/hud.rs`, `crates/hud`.
- **13 End screen.** Read the mode-14 results branch (`ER_DETONATOR_SCORE`,
  `PERCENT_MINES_CLEARED`, stage reached); fill the grid in
  `crates/game/src/app/race_stage/endrace.rs` and `crates/ui-screens`; `just check-strings`.
- **14 Campaign.** `crates/game/src/campaign.rs::race_mode_for_cell` maps `Other("Detonator")` to
  `Mode::Detonator` (HD Fury grids `grid_08`..`grid_15`, 14 cells; Omega; 2048 DLC), the medal
  value arm in `RaceStage::campaign_tier` is the Detonator score (counting mode, `>=`); update the
  campaign ground-truth tests and `docs/formats/race-campaign.md`.
- **15 Parity.** Omega race-start placement call, stage cap into `craft+0x5e4`, grade drawing;
  2048 per-stage `.gxt` textures.

Overall risks: most of EMP, auto-speed and the HUD is unread (RE-first, may need Ghidra
AltiVec workarounds: `llvm-objdump -d --mattr=+altivec,+vsx` and `scripts/ps3-toc.py`); the
placement RNG cannot match the original's `Libc_Rand` (a departure, documented).

## Open

- **Own hull, handling file and object models: wired** (2026-11-22; the wreck and death shell
  (HD's hull build loads no wreck in any mode), the mine/bomb colour binds (`MineColour`,
  `BombColour` ...), the pad's original geometry underneath and 2048's object models stay open;
  `detonator-hull.md`, "Wired"). Read as follows (2026-11-22,
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
  6. Cannon: wired (`det-fx`): `detonator_cannonbolt` and `hd_muzzleflash_detonator` from the
     `cannon_flash_left`/`right` locators, which the simulation fires from too
     (`load::muzzle::cannon_flash`).
  Open: the substitution table's reader, the race-side write of `ship+0x6298`, the
  `Charge1`/`Charge2` block writer (`Ship_BindDetonatorHullParams`), the mine's music-driven
  binds. `detonator_pickup` and `detonator_mine_explosion_kaleidoscopic` ship, but no loader was
  found (70, negative).

- **Layout files decoded, placement and contact wired** (`oag_tables::detonator_layout`,
  `oag_weapons::detonator`, `oag_raceplay::detonator`): HD races lay their mines and bombs per
  stage and draw them with the ordinary Mine and Bomb models. Contact is wired
  (`oag_weapons::detonator::step_contacts`: damage, velocity cut, bomb push, removal, destroyed
  craft ends the race); the survey's collision return codes (probes are
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

- **Pulse cannon, score, destroyed count: wired, rounds and effects drawn on HD** (2026-11-22,
  `oag_weapons::detonator::cannon`, `oag_raceplay::{detonator_fx, RaceSim::detonator_score,
  detonator_destroyed_mines, detonator_percentage}`; Square fires, Circle reloads, slot 0). Muzzles
  are the hull's `cannon_flash` locators; the tick's `oag_weapons::detonator::Event` list drives
  `WO_MINE_EXPLO_DETONATOR[_DEAD]`, `WO_BOMB_EXPLO_DETONATOR` and `WO_CANNON_SPARKS_DETONATOR`
  (HD's triggers; 2048 and Omega ship and parse them, **checked, applies, not wired**), plus `CANNON`
  and `CANNONEXPLWALL`, `MINEEXPL` and `BOMBEXPL` (`Cue::MineExplosion`/`BombExplosion`, HD only; `BOMBEXPL_PC` never). Open: the round's draw (`0x00121e68`) and its flash window are HD's Cannon's
  (chosen); the effects use the procedural profile (no sprite list, no matched capture); no
  the Detonator weapon bank (`weapons_det.bnk`?) is unread; the visibility gate on the effects; the `CANNONRELOAD`/`BONUS_*` cues, the HUD (clip, reload gauge, score, `x%02d`),
  the end screen rows, the 0.5 s owner grace, the clean-lap `+1000`, the bomb-ahead warning,
  `EnergyRechargePerStage`. 2048 and Omega fire it too and draw its two models (tests in
  `detonator_ground_truth.rs`). The record below is the read:
- **Pulse cannon, hits, score, destroyed count: read** (2026-11-22,
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
