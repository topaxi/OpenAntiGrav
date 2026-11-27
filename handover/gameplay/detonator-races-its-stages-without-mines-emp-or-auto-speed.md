---
status: ready-to-wire
titles: [hd, 2048, omega]
updated: 2026-11-27
summary: "Detonator open: floor visualiser second factor and 2048/Omega strip decode, upper-right screen, HD end-screen delay, Omega EndRace Menu, medal tiers"
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
| 2 | End-screen headline `ER_ZDETONATOR_COM` (`Headline::Detonator`; the id is in `DATA06`'s table on HD, filled into the served table at boot) | done | `d890b438a`, `det-end` |
| 3 | RE: stage placement of mines and bombs (`detonator-placement.md`) | done | `543e8953b` |
| 4 | Read `PatternFile.xml` / `GamePlayValues.xml` (`oag_tables::detonator_layout`, `oag_title::DetonatorLayouts`, `docs/formats/detonator-layouts.md`) | done | `8a6918978` |
| 5 | Place mines and bombs (`oag_weapons::detonator::place_stage`, `World::detonator`, `oag_raceplay::detonator`, drawn with the ordinary Mine/Bomb models) | done | `dbb59fd37` |
| 6 | RE: mine/bomb contact, blast, destruction (`detonator-mine.md`, `detonator-bomb.md`) | done | `57d0847fb` |
| 7 | Hazards bite: `step_contacts` (damage, velocity cut, bomb push, removal), destroyed craft ends the race; the race loads the Detonator weapon table | done | `6a4c6be71` |
| 8 | RE: pulse cannon, hits, score, destroyed count, percentage (`detonator-cannon.md`) | done | `0db7e825c` |
| 8b-RE | RE: own hull, own handling file, own mine/bomb/pad models (`detonator-hull.md`) | done | `bed94feed` |
| 8b | Wire the hull, handling, mine, bomb and pad models | done (wreck and death shell open); mine and bomb colour binds wired (`det-colour`) | `4f4b70173` |
| 9 | Wire the cannon, score, destroyed count and percentage | simulation done; rounds, flash, explosion effects and the cannon's cues drawn on HD; the HUD is row 12's, no end screen | `59215fc52`, the `det-fx` commits |
| 10 | EMP: RE (pads, charge flag writer, `EMPManager`) then wire | done on HD and Omega (`detonator-emp.md`, "Wired"); the wave's reach is chosen (`emp::SECTIONS_PER_EXTENT`); 2048's pads read and wired 2026-11-25 (`vita-2048-eu-v104/detonator-pads.md`) | `det-emp-wire` |
| 11 | Auto-speed: RE Detonator's engine branch, then wire | done on HD, 2048, Omega (`detonator-speed.md`, "Wired"), ramp included | `det-emp-wire` |
| 12 | Detonator HUD (`detonator_hud.xml`, `hud_detonatorweapons.xml`, `detonator_warnings.xml`) | wired on HD, Omega and 2048, the EMP pips and `EmpReady` included; the three medal models draw on HD and Omega (`det-medals`, 2026-11-23; 2048 left); open: their tier colour, `EmpBarEffect`, 2048's `EmpCharge`/`EmpFull`, bomb-ahead, score feedback (below) | `det-medals` |
| 13 | End screen proper (stage, mines, bombs, three chain counts, score, new-record banner) | HD wired and checked live (`detonator-endrace.md`): rows, score line, captions, record notice and kept best; open: the destruction before the panel, end-screen text size, Omega (no EndRace screens on main) and 2048 (no page to draw on) | `det-end` |
| 14 | Campaign launch (`race_mode_for_cell` maps `Other("Detonator")`; medal = the cannon score, `>=`) | done: 12 cells on HD, Omega, 2048; see `git log` (`Detonator campaign`) | see `git log` |
| 15 | Omega and 2048 parity leftovers | done: Omega's opening placement and stage cap are HD's law (pinned, `detonator-mode.md`); 2048's per-stage `.gxt` sets wired on its fallback; Omega's grade wired on its HD-lineage circuits (`det-parity`, 2026-11-23) | `det-parity` |

A race can already be launched from the command line:
`oag-game --race --mode detonator [--autopilot --ticks N] data/images/hdfury-ps3-eu-dec.iso`
(HD EU disc in `data/images/`). The menu launches it from a campaign cell (phase 14, done).

## What a player gets today (HD)

14 stages; each stage lays up to its `MineCountStage` mines and at most one bomb; a mine costs 10
energy and half the speed, a bomb 20 and a push; energy at zero ends the race. Every craft flies
the Detonator hull in its team livery on Detonator's own handling file; mines, bombs and pads draw
the Detonator models (a mine draws as its material's rim glow in the stage's colour and the music's
envelope, a bomb in the stage's inner and outer colours; see "Mine and bomb colour binds"). Square fires the pulse cannon and Circle reloads; rounds
destroy mines and bombs and the race keeps a score, a destroyed count and a percentage, but no
the rounds and muzzle flashes are drawn and a shot mine, a bomb and a round on a wall play their disc effects (HD); the HUD is row 12's. The craft drives itself (Zone's
auto-speed on the stage); pads charge the EMP instead of boosting, L1 fires it and its wave
clears the mines ahead (wave reach chosen).

Open from 8b: no wreck or death shell (HD's PS3 hull branch loads no wreck in any mode; the
Detonator `shipwreck.vex` is an outline placeholder); the circuit's own pad is hidden under the
Detonator pad on HD, Omega and 2048 (`DetonatorPads::replaces_circuit_pads`; HD 80 from a live frame,
Omega and 2048 65 from the same code shape, no original frame of either: `detonator-emp.md`, "Do the circuit's own pads still draw?"); Zone Battle (modes 13, 21) runs the same destroy loop and is not wired; the model's origin at the pad transform is
chosen; 2048 reads the object models from HD's `Data\Weapons` prefix (its loaders' prefix unread).

## Working rules learned the hard way

- Docs-only RE phases go to a `worker` subagent (`model anthropic/claude-opus-5-5`, sync) with a
  self-contained brief; check `just check-docs check-names check-handover check-status` (run
  `just gen-status` after new `names.tsv` rows), then commit docs separately from code.
- Each code phase: `cargo fmt --all`, `just lint`, `just gate-affected`, then
  `just test-data-affected`. Run ignored ground-truth tests with
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

Muzzle flash (Render accuracy row 6): the per-tick roll (rotation, size, stretch) and the two point lights are read and wired (`detonator-cannon.md`, "The flash's per-tick roll"; the earlier "no roll" paragraph was retracted). The blue wash across the scene is read and wired (2026-11-24, `det-flash-wash-re`): the round's light on the scenery, which the Zone programs' `SVC1` twins add after the fog and outside the surface multiply (`detonator-cannon.md`, "The blue wash of a volley"). Open: the `+0x130` alpha word's reader, a watchpoint on the flash material's `V_Offset` (inferred never written), and the 2048 and Omega `PulseBullet` updates.

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
- **Inputs:** fire = `Button::Square` held, reload = `Button::Circle` (pressed; the buttons are read, `detonator-cannon.md`
  "The buttons", 80; edge-vs-level of the reload unread, **chosen**), slot 0 only; pass the snapshot into `advance_detonator`
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

- **10 EMP: done** (HD, Omega; 2048's pads unread). What stays open is under "Open" ("EMP:
  wired"); the law and the wiring table are `detonator-emp.md`, "Wired".
- **11 Auto-speed: done** (HD, 2048, Omega). Open: under "Open" ("Auto-speed: wired");
  `detonator-speed.md`, "Wired". **Also found** in the RE: HD's own Zone runs the same branch
  with the ramp (`psp-pulse-usa/zone-start.md` says otherwise; that page and the weapon-lineage
  Autopilot row are not edited).
- **12 HUD: wired** (`docs/ghidra/functions/ps3-hdfury-eu/detonator-hud.md`, "Wired").
  `HudLayouts::detonator` names each title's root; score, clip, stage, bullet icons, volley pips,
  the streak line and the energy readout (white in Detonator) read the live cannon
  (`oag_hud::Detonator`, `oag_raceplay::detonator_hud`, test
  `detonator_hud_ground_truth.rs`). **Medals: wired** (`det-medals`: `oag_hud::ImageModel` reader,
  `Race::set_medal_targets` from `medal_watch::tick`, `oag_game::hud_medal::Medals` through the model preview,
  `Detonator::shows_medals` HD and Omega on, 2048 off; `detonator-hud.md`, "The medals"). **Open for the
  medals:** the tiers draw the same grey (the model's own diffuse, specular and exponent parameters are
  authored apart, the program reflects the paraboloid probe, which the mesh reader classes as glint
  water and a preview has no probe for), the camera (`ModelZ -87`'s projection is unread), when each
  widget enables and whether a lower tier stays under a higher (best tier only here), 2048's
  `ImageX 720` skin, and HD's end-race `MedalModel*` (same models, not drawn there). **Still open:** the EMP
  widgets (phase 10), the bomb-ahead warning (needs an event stream), `ScoreFeedBack0..15` on Omega and 2048 (HD's is wired),
  HD's streak line (its language table has no `HUD_CHAIN_n`), the bullet icons' per-state art and
  the per-volley fired words. The brief that follows is what was read, kept for those:
  (`docs/ghidra/functions/ps3-hdfury-eu/detonator-hud.md`,
  HD static, 50-82 per widget; Omega checked: applies; 2048 checked: differs, own skin and EMP widgets).
  Wire it as a Detonator `HudLayouts` entry (`oag_title::HudLayouts` has no Detonator field today;
  `oag_hd::hud::layouts::DETONATOR` and the `detonator_hud2.gtf` atlas row already exist,
  `crates/hd/src/hud.rs`) and drop the "Detonator draws the time trial's" arm of
  `oag_raceplay::hud::hud_layout` (`crates/raceplay/src/hud.rs` ~l.111). 2048 reads
  `Data\XML\2048_hud\Detonator_HUD_HD.xml` only (the base `Detonator_HUD.xml` it ships is never
  named by its executable): a text-only score/stage skin on `2048_hud/DetonatorWeapons.xml`, whose
  EMP widgets are `EmpCharge`/`EmpFull`, not HD's `Emp_%d`/`EmpReady` (`crates/2048/src/hud.rs`
  ~l.560 lists the base path today); Omega names `Data\XML\Detonator_HUD.xml` like HD, and
  its `hud_detonatorweapons.xml` has an `<aImageModel>` typo on `EmpBarEffect`. Per widget, with the
  state that exists on main (`C` = `oag_weapons::detonator::cannon::Cannon`, reached as
  `World::detonator.cannon`; `RaceSim` accessors in `crates/raceplay/src/detonator.rs`):

  | Widget | Shows | Format | Source on main | Status |
  | --- | --- | --- | --- | --- |
  | `DetonatorScore` | the score | `%07d`, white | `C.score` (`RaceSim::detonator_score`) | ready |
  | `PulseBullets` | clip count | `%02d` | `C.clip` (0..15) | ready |
  | `Bullet00`..`14` | `Bullet(14-j)` spent once volley `j` has fired | sprite state | `15 - C.clip` spent icons from the right (the original reads 15 per-volley words next to `hits`, writer unread, 55); `C.hits[2j] != 0` is ring-indexed and diverges after a mid-clip manual reload | ready |
  | `Multi00`..`14` | `Multi(14-i)` lit once volley `i` hit; a streak award recolours its last 5/10/15 pips `0xff00eeff` | pip | `C.hits[2i] > 1 \|\| C.hits[2i+1] > 1`; awards from `C.awards` | ready |
  | `PulseBonus` | the streak line `HUD_CHAIN_n - amount` (1000/3000/5000), held 2.0 s then faded 1.0 s | `%s - %d` | edge-detect `C.awards` (`first_five`, `second_five`, `ten`, `fifteen` go false to true); the timers are view state | ready (needs the language ids `HUD_CHAIN_5/10/15`) |
  | `DetonatorStage` | HUD stage over 14 | `%d/%d` | `World::detonator.stages_laid.min(14)`: the original's `rm+0x2dee` starts at 1 (the constructors store it) and counts start-line crossings, capped 14 (72) | ready |
  | `DamageBar`, `DamageBarBg`, `ShieldBarText` | energy percent (white in mode 14) | `(int)pct` | the generic shield readout (`oag_hud::shield_bar`, `Readout::shield`) fed with the craft's energy pool | ready once the layout loads |
  | `ScoreFeedBack0`..`15` | a floating `+N` per score add (`+%3.0f`) | text | `Event::points` | wired on HD 2026-11-27 (`detonator-hud.md`, "The floating score"); Omega and 2048 author the widgets, their caller unread |
  | `DetonatorGold/Silver/BronzeMedal` | model when the score passes the cell's thresholds | model | `crate::medal_watch::detonator_value` + the cell | wired (`det-medals`); best tier only, chosen |
  | `ForwardWarning*` | bomb-ahead warning, once per stage, 2.0 s | icons | **missing**: `Detonator_IsBombAhead` (500.0 track measure + straight distance, `0xd2a30`/`0x588f0` unread) and the per-stage latch. On HD message 8 selects `ForwardWarningMineIcon`, which `Detonator_warnings.xml` does not author, so HD probably draws no bomb icon (55, `detonator-hud.md`) | needs RE + a capture |
  | `Emp_0`..`7`, `EmpReady`, `EmpBarEffect` | `n = rm+0x2dd8` pips lit while `n > i`, `EmpReady` at `table+0x1bc`, gauge `min(n/8, 1)` | pips | `RaceSim::detonator_emp`, `detonator_emp_gauge` | pips and `EmpReady` wired (HD, Omega); `EmpBarEffect` not drawn (an `<ImageModel>`); 2048's `EmpCharge`/`EmpFull` open |
  | `PulseScore`, `PulseMultiplier`, `EmpScore`, `PulseBankedScore` | **nothing on HD**: the controller never binds the first two, never reads the third and its banked-score text setter has no caller | - | do not draw `x%02d` or the `%04d` counter | skip |

  Also from the page: the reload gauge `(1 - lockout/total) * 15` that `Detonator_UpdateHud`
  computes feeds `0x0029d578`, a shader constant of the craft's own
  `detonator_bulletgaugesegments` material, not a HUD widget (**wired 2026-11-25, `det-ammo-gauge`**:
  `detonator-ammo-gauge.md`; 2048 and Omega ship the material, programs unread, not wired); the
  HUD's `PulseBullets` reads `15` through the whole reload lockout. Open for this phase: the
  `EmpBarEffect` shader constants, the bullet icons' per-state art (`0x0007f0a0`), and what
  `Detonator_UpdateHud` raises into the generic message lines.
- **13 End screen: HD wired** (`det-end`; law `docs/ghidra/functions/ps3-hdfury-eu/detonator-endrace.md`,
  "Wired"). `FieldResults::detonator` (`oag_ui_screens::endrace::DetonatorResults`) draws the six
  rows and the score line in place of the field grid, laid out beside `RaceGrid`
  (`endrace/hd/detonator.rs`); `oag_game::results_tables::detonator_results` fills it from the sim and
  the scoreboard (the stage row is the scoreboard's laps completed, so a run destroyed in stage 5 reads
  `4 / 14` and a finish `14 / 14`); the boot fills HD's served table with the eight ids from `DATA06`'s
  language table (`strings.fill`). The sim keeps race-long 5/10/15 chain counts and counts an EMP's
  bomb kills. Loyalty now pays on those laps (a destroyed run used to bank the full 14 stages).
  **Open:**
  - **Checked against two RPCS3 frames** (`detonator-endrace.md`, "Checked against the original"):
    the record notice, the captions and the kept best score landed; open on HD: the **5.4 s of
    destruction before the panel** (ours puts it up the tick the craft dies; what plays behind it afterwards is unchecked against
    the original's glow, particle burst and wipe-in), the **end-screen text size** (rows 0.6x, headline 0.7x; the screens draw `default`
    face as the scaled menu face, the campaign screens use the `Default` atlas), the
    `CONFIRM` glyph (the HD FE style now follows `display.front_end_style`,
    `docs/ui/endrace-screens.md`).
  - **Omega**: same filler (`FUN_0155bc90`) and ids; but Omega's `endrace_entry` is `None` (a finished
    race keeps the built-in scoreboard; wiring HD's reader leaves `EndRace Menu` with no options,
    `docs/formats/omega-status.md`), so no Omega Detonator screen draws until that is solved (a
    separate follow-up: Omega's `EndRace Menu`). Its served table carries all the ids (tested).
  - **2048**: its four-row list (stage = the HUD stage with no `/ 14`, mines `%.1f%%`, bombs `%d`,
    `IG_HUD_SCORE`, no chain rows) is modelled and worded (`Detonator2048Results`, tested) but not
    drawn: it belongs to its `Results` page, a named gap (`docs/ui/endrace-2048.md`); the touch
    Summary page has no widget for it. No layout is invented.
- **14 Campaign: done.** `race_mode_for_cell` maps `Other("Detonator")`; `RaceStage::campaign_tier`
  takes the score through `medal_watch::detonator_value` (finished races only, `>=`). 12 cells per
  title, not 14 (`docs/formats/race-campaign.md`, "Detonator cells launch"). The value handed to the
  compare is the score, read (`0x0005c3c0`, 82; `detonator-endrace.md`). A destroyed run records
  too (both endings call the builder with `force = 1`).
- **15 Parity: done** (`det-parity`). Omega's race-start placement call and the stage cap (v1.00
  `+0x5e4` = v1.07 `+0x5f4`, the auto-speed's field, not a second write) are HD's law: ported, no `Title` field
  (`detonator-mode.md`, "Omega's race-start placement and stage cap"); 2048's per-stage `.gxt` sets ride
  `Detonator::fallback_stage_textures` on its fallback circuits only; Omega's grade is HD/Fury's Detonator pair
  and `detonatorModeTrack<n>.gnf` on its HD-lineage circuits (`Environments2048` held back); see
  `detonator-look.md`, "Cross-title".

Overall risks: most of EMP, auto-speed and the HUD is unread (RE-first, may need Ghidra
AltiVec workarounds: `llvm-objdump -d --mattr=+altivec,+vsx` and `scripts/ps3-toc.py`); the
placement RNG cannot match the original's `Libc_Rand` (a departure, documented).

## Review fixes

- The auto-speed ramp is in the state hash in every Detonator race, not only where an EMP armed (`hash::write_detonator`; `Field::is_empty` now tests it). Detonator hashes move, other modes do not.
- The cannon, the EMP fire and the EMP charge all key on `primary_slot()` (`cannon_handed(slot)`); `detonator_ground_truth::an_hd_detonator_cannon_and_emp_work_off_the_primary_slot`.

## Open

- **Cannon bolt (`det-cannon-bolt-look`)**: the bolt's program and the half turn are wired on HD and the loop ride is
  confirmed live (`detonator-cannon.md`, "The bolt's look and the loop"). ~~(1) the spark's frame~~ read live and wired (`det-cannon-spark`, `detonator-cannon.md`, "The spark's matrix, read
  live"); left of it: the `-0.3` along the wall normal, size, brightness and timing (a camera-matched pair at a close wall
  hit: read the view matrix in the death pause, fire ours through the simulation, render with `--camera-pose`).
  ~~2048's and Omega's trigger~~ and ~~(2) their bolt programs~~ wired (`det-cannon-bolt-2048-omega`); neither has an
  original frame. Open: (3) the 0.05 to 4.8 unit sideways drift through the loop, 60 Hz
  against 30 Hz suspected; (4) no side-on capture of the original's bolt.

- **Mine look (`det-mine-lit`)**: the normal map is read now (the chunk has one coordinate set), so hubs and holes show.
  Open: (1) a pair's mines stay dark because the envelope has no history under raw bands below 0.3; carrying each mine's
  `m+0xf4`/`m+0xf8` out of the original's RAM (`ship/mines[].address`) into a flag that seeds `MineEnvelopes` needs an
  RPCS3 read; (2) with the envelope high, the ball's centre is dark in ours and bright in the original. **Not the eye**
  (`det-mine-eye-centre`, `detonator-mine-material.md`, "The lit draw, read live"): the lit pass is block `#15` and `c[209]`
  is the world camera, as ours feeds; the program, vertex decode, map, alpha test and cull all match, and the formula gives
  dark there in RPCS3's own shaders. A live A/B (`K[0x3c]` 0) turns the original's body dark with bright rings, so its
  body rim is about 0.4-0.7, not 0: its tangent-space `V` is off-normal. ~~Next: patch the football's fragment program in
  VRAM to output `normalize(f[TC1])`~~ done (`det-mine-body-white`, 2026-11-26, "The white is before post, and the program as read
  reproduces it"): read at one held pose, the program fed the frame's live constants matches the original's body rim at the edge
  and is 0.1-0.15 high inside, where the camera-facing spikes overlap; no input term is missing. ~~Next: ours' spikes~~ done
  (`det-mine-spikes`, `detonator-mine-draw.md`, "The spikes in a pair"): the old `mines-ahead` pair carried no envelopes;
  re-grabbed with them, ours' near mines are lit and spiked at the original's size (same theirs 15.44 -> 13.93). ~~Re-grab
  `cannon-volley` with envelopes~~ done (`det-cannon-regrab`, 14.32 -> 13.35); `pad`'s floor is music-lit differently on a re-grab.
  The pair now takes each mine's envelope at the picture's tick (`detonator-mine-draw.md`, "The envelope at the picture's
  tick"); near-ball boxes within 20 grey. ~~The mine spin~~ wired (`det-mine-spin`, `detonator-mine-draw.md`, "The spin,
  wired"): `mines-ahead` 13.17 -> 12.51 with the grab's ages pinned. ~~Ours' `PitchVolume` band~~ read
  (`det-mine-pitch-volume`, `detonator-mine-material.md`, "The visualiser's level"): both volumes are one whole-spectrum
  level `L` (`clamp01(5 L)`, `clamp(L, 0, 0.4)`); a wiring (commit `5fac71320`, reverted) gave `mines-ahead` 12.51 -> 12.52
  and `cannon-volley-hot` +0.04, so not kept. `PitchVolume` is worth 0.01 grey; `MasterVolume` 0.25. To land it: re-grab
  `cannon-volley-hot` with the volume words so no pair falls back to `L = 0`. Next: the spin on 2048 and Omega (their
  animate functions are not located). ~~The ring's own radius~~ measured (`det-mine-ring-radius`,
  `detonator-mine-draw.md`, "The ring's radius"): ours already draws it at the original's (57.5 against 55.5 px on the middle
  near mine; the side mines on the law's 74 and 60 px circles in both frames), and the three near mines' boxes carry 2.49 of
  `mines-ahead`'s 12.5. ~~The upper-right scenery~~ fixed (`det-dark-scenery`: Talon's `scanlinebillboard`
  hologram drew its advert in a Zone frame; `mines-ahead` 12.46 to 10.63). Next for that pair: the score panel (2.10, HUD).
  (3) ~~Ours builds the football and spikes with no tangent~~: wired (`Mesh::inline_tangents`, `+10`); the dark centre is
  gone, ours' core 143 -> 157.5 against the original's 188, so the body is still darker than the original's white.

**What remains open for Detonator (2026-11-23, after phase 15).** Phases 1-15 are in; what a player still
misses is, by title. *All three:* the medal tier colours (the tiers draw one grey), the bomb flash writer
(`b+0x154`, unread: brightness stays `0.2`/`0.8`), ~~the EMP wave's reach (chosen)~~ (read and drawn on HD, `det-emp-look-re`) and the radial bloom
(`oag-post` has no pass, Zone included), the floor spectrum feed (the pairs stay black under `--no-audio`), the
hull strip and wreck. *HD:* the end screen's destruction hold and text size.
*Omega:* the end-race screen (its `endrace_entry` is `None`, wiring HD's reader leaves `EndRace Menu` with no
options: Omega's `EndRace Menu`), the sky (`zone_sky: None`), the announcer events and the stage-10 music change
in its state `2`. *2048:* the `Results` page (the four-row list is modelled, no page to draw on), its pads'
EMP charge, `EmpCharge`/`EmpFull`, the medals' `ImageX 720` skin, a black floor on its ported circuits, and no
original frame to compare. The detail is below.

- **Mine and bomb colour binds: mine wired, bomb partly** (`det-colour`, 2026-11-23;
  `docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-material.md`, "The mine's fragment program",
  80, and "Wired"). A mine draws as its material's own unlit rim (`rim^(4 sat(1 - B) + 1) / (1 - 0.9 C)`,
  `PROGRAM_MINE_RIM`) in the committed stage's `Detonator Mine Colour` with the music-driven
  `MineBrightness` (`oag_raceplay::detonator_look`, per mine, keyed by survey entry and lane); a
  dark mine takes the literals and brightness 0; a bomb body and its equalisers draw as colour times
  brightness in the stage's inner and outer colours (`PROGRAM_BOMB_TINT`, chosen). Open:
  the bomb programs' real combine (overlay, environment picture), the flash `b+0x154` writer
  (unread, no trigger invented, lead's decision 2026-11-23: brightness stays `0.2`/`0.8`), the equalisers' rays and the glow edges' own colour binds
  (generic names `Colour`/`Brightness`/`RainbowColour`), the mine's size breathing (`scale`,
  `spikeScale`), the `Electric_Visualiser`, a dark mine's negative output on the original's target
  (drawn black here, unseen), the band source (the visualiser's held levels, not the raw array) and
  the call rate of the envelope (per tick here). Re-render the pairs before judging the mines
  (the pair flags pass `--no-audio`, so brightness is `0` there: a thin rim). 2048: checked, applies
  (byte-identical effect-settings file, same bind strings), not wired. Omega: checked, applies
  (same file; binds by hash constants), not wired.
- **Own hull, handling file and object models: wired** (2026-11-22; the wreck and death shell
  (HD's hull build loads no wreck in any mode), the mine/bomb colour binds (wired, see the
  "Mine and bomb colour binds" bullet above) and 2048's object models stay open (the circuit's own pads are now hidden);
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
  live capture of `Charge1`/`Charge2` (wired 2026-11-25, `det-hull-charge`: they are the alpha of the
  two plasma-shell materials, not the fuselage's; `detonator-ammo-gauge.md`; unread: `0x002a3718`'s body,
  so whether the strip lights at exactly the ready count, and the block live at EMP not ready, ready and
  full), the mine's music-driven binds. `detonator_pickup` and `detonator_mine_explosion_kaleidoscopic` ship, but no loader was
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
  needed and must say so), ~~the route-fork walk~~ (read and wired 2026-11-25, `detonator-placement.md` "The fork walk"; open: a live read of the lap array on a forked circuit), what the mine/bomb inits do with the matrix and
  `sec+0x40`/`+0x60`, how a mine or bomb leaves its pool (the bomb pool is 2 and the spawn does
  not check it), `0x0012ae38`/`0x0012ae10`/mine vslot `+0x3c`. Models: see the hull bullet
  above (`DetonatorMine_Construct` `0x0013a7d8`, `DetonatorBomb_Construct`); effects `WO_BOMB_EXPLO_DETONATOR`,
  `WO_MINE_EXPLO_DETONATOR[_DEAD]` (2048 `data2.psarc`), whose
  triggers need the hit read first (the cannon's wall spark is wired on 2048 and Omega).
- **EMP: wired** (`det-emp-wire`, `detonator-emp.md` "Wired"). ~~The wave's reach and speed, the kill's
  effect and sound~~: read live and drawn on HD (`det-emp-look-re`, 2026-11-24, `detonator-emp.md`
  "What HD draws for a fired EMP": one survey entry a tick for `extent` seconds, `WO_QUAKE_DETONATOR`
  on the front with a `(0, 5, 10)` light; the stepper is the Repulser's, copied). The particle rows
  were `SPARKS` without their noise force, fixed (same page, "The grey rows were sparks that
  never moved"); 13 other HD emitters author that force, unlisted. The in-run fork and the launch-time fork are wired (`detonator-emp.md`, "The launch-time fork"). The other route's record lookup is wired. Open: `~MAGPULSETRAVEL` unwired (all three titles). 2048 and Omega draw the wave and submit its light since 2026-11-26 (`detonator-emp.md`, "2048 and Omega draw it"); **open: the light changes no scenery pixels in ours on either** - read each title's Zone shader combine (GCN, GXP) for the vertex or per-draw light, then wire (HD's `lit_by_stage_alone` is HD-only, do not just turn it on). Their mine and bomb explosions are still unwired, so the wave's kills pop nothing there. Still open:
  whether the wave hurts the player,
  `EmpBarEffect` (an `<ImageModel>` the HUD layout reader does not model; the gauge is exposed),
  the ready pulse and post-fire flash on `EmpReady`, 2048's
  pad arms (unread: its pads keep boost and pickup, so its EMP never charges) and its
  `EmpCharge`/`EmpFull` widgets, Omega's EMP announcer lines (Wwise, not located).
- **Auto-speed: wired** (`det-emp-wire`, `detonator-speed.md` "Wired"). Open: a player-input speed
  curve on a clean lap against the original; HD's own Zone ramp (this build's Zone has none).
- **Score, end screen, HUD.** Score and percentage are simulated and shown: the HUD is phase 12's,
  the end screen's rows phase 13's (HD wired, above). Campaign cells (`mode="Detonator"`, 12 on the
  Fury grids of HD, Omega and 2048's pack) launch and are medalled on the score (phase 14);
  the mid-race medal models are phase 12's (wired, `det-medals`); their cues and `ER_*MA` lines are not (nothing measured them for Detonator).
- **The out-of-race ending** (`+0x6a64 & 0x1000`) is not modelled. Its writer is read
  (`Ship_SetState` state 4, `detonator-mine.md`); Phase 7 wires it with the contact law.
- **Omega's grade** is drawn on its HD-lineage circuits (phase 15); Omega's camera scale is
  inherited from HD, its craft constructor unread. **2048's per-stage `.gxt` textures** are wired on
  its fallback circuits (phase 15); an `Environments2048` circuit's own `ZoneMode2048` table in Detonator is
  unread on both titles.
- **2048 `zone_1`..`zone_4` draw a black floor** in every mode (Time Trial too): not
  Detonator's, seen while capturing.

## Render accuracy

Measured 2026-11-22 (`det-pairs`): the original (HD/Fury EU on RPCS3, Racebox RACE TYPE Detonator, `GetMode()==14`, Talon's
Junction, Feisar, no stick, no throttle) against ours at the original's own pose, camera, fov, lap clock and world clock. Three
pairs, `data/reference/compare-page/hd/detonator-talons-{mines-ahead,cannon-volley,pad}` (manifest `detonator.toml`, also in
`curated/`); method in `docs/tools/compare-page.md`, "Detonator pairs". Each was seen on one boot; the same scene (hull, mines
ahead, HUD, the pad at the right edge) was seen on four boots between lap clock 2.0 and 2.7 s. Whole-frame RMSE 0.26-0.30, edge
correlation 0.08-0.12, mean grey 28-41 (all three red on `compare_env.METRIC_BANDS`). Ours' craft is held at rest on the original's pose (`--pose-hold-ticks`): a first version let ours fly 130-190 units past the pose before the frame and was re-made, so rounds, mines and energy in the table are from the held version only. Region numbers below are RMSE (0-1) on the
three pairs and the region's share of the 1920x1080 frame; "area x error" is the product in percent, the order to fix in. Mine
positions differ by construction (ours places with its own seeded `Rng`), so a mine-only crop compares looks, not places.

| # | Region | Area | RMSE | Area x error | What differs (confidence 70 unless stated) |
| ---: | --- | ---: | ---: | ---: | --- |
| 1 | Sky and background (upper band, y < 340) | 31 % | 0.38-0.41 | 12-13 | Ours draws the circuit's day sky (clouds, blue); the original's frame is near black (mean `(18, 10, 11)` over `x 200..900, y 60..300`, ours `(96, 111, 121)`). **Cause read (80):** the original swaps in `ZoneSky.gtf` and draws the gradient dome on `g_ZoneEffectsActive`, which mode 14 sets; Detonator stage 1 authors `Sky horizon colour` and `Sky zenith colour` `0 0 0`, so the dome is black. Ours gates the sky swap and dome on `Mode::Zone` (`load/environment.rs:71`). `docs/ghidra/functions/ps3-hdfury-eu/detonator-look.md`. |
| 2 | Mid band (far track, mines, pad, y 340-640) | 28 % | 0.18-0.36 | 5-10 | Mines (row 4), pad (row 5), the original's bluish-purple ambient on gantries against ours' neutral grey. The gantry violet is likely the stage's `EQ colour tint` `171 9 255` on the billboards, which ours leaves off (the shared stage target is `Mode::Zone` only, `load/gantry_visibility.rs:25`; the original's gate is the byte, 80). |
| 3 | Lower band (near track, hull, y > 640) | 41 % | 0.14-0.22 | 6-9 | The original's track is a saturated blue-violet with red edge strips; ours is near black with the same red strips. Brighter by 10-19 grey in the original. Ours already applies Detonator's grade (its file, texture sets, Track recolour `[0.97, 0, 0]`); **Cause read (85, `det-track-look`, `detonator-look.md`, "The floor's violet"): the floor is the `zoneTexVis` visualiser, which is spectrum-driven and wholly black under the pairs' `--no-audio`.** 87 % of Detonator's track texture indexes smooth slot 164 (band 3), which takes `EQ analogue colour tint` `14 1 55` times the band's level; the dashes are bar segments (3.5 % of the texels) in `EQ colour tint` `171 9 255`. Ours wrote the bar tint into the smooth slots too (hue wrong at every level); now the analogue tint (`EqTints`). Floor boxes under `--zone-spectrum-test`, ours before to after against the original: A `(33.7, 5.0, 41.3)` to `(9.5, 0.0, 21.7)` against `(5.9, 0.0, 18.0)`, B `(65.7, 10.8, 80.1)` to `(19.8, 0.0, 42.9)` against `(9.0, 0.0, 31.5)`. **The pairs now record the original's held bands and ours is fed them (2026-11-23, `det-pair-bands`, next paragraph): the floor is lit, but ours is 2.2 to 3 times too bright on blue.** 0.22 on the volley pair (the ground ahead of the hull lit by the flash in the original). |
| 4 | Mines | 1-3 % (by eye) | not cropped | 1-2 (estimate) | Original: a spiky purple-white sphere with a lit shell, grows large as it nears. Ours: a small hard white ball before `det-colour`; since then a stage-coloured rim sphere (the colour and brightness binds are wired; the spikes still barely show at a distance, and the mine's size breathing is wired since `det-mines`: football `clamp(2.3 s, 1, 2.3)`, spikes `clamp(4 t, 1, football + 0.95)`, long spikes and a bigger shell; a mine's own `s` is not carried, so ours is the original's size only by chance). |
| 5 | Detonator pad | 2-3 % (by eye) | pad-box mean grey original 45.8, ours 32.6 to 50.0 | under 1 | **Fixed (2026-11-24, `det-pad-glass`).** Original: grey slab, white walls, red squares, matt. Ours was lavender with black squares and no walls. Causes read (85): the pad's `diffuse_normal_specular_emmissive_alpha` never reached `pad_ne` on an object model, so `emissive()` added the `_ne` normal map as a glow (lavender) and its index ORed onto the pad colour's (a missing entry, black squares); and the walls (`emissivecolourchange`, white, 0.76 below the model origin) sank under the road. Now `pad_ne` binds the mask and its red `0x7611a2d8`, and each pad matrix is raised by the model's own depth (**chosen, not measured**: the original's binding loop is unread; the crop agrees to about 1 px). `pad` pair 8.43 to 8.17; `mines-ahead` 14.65 to 14.65. |
| 6 | Cannon muzzle flash and rounds | under 1 % (by eye) | cropped: ours' flash is flat, see the row | under 1 (estimate) | Original: two blue-white flashes at the wing tips and blue-white rounds with a trail, still lit 6 ticks after the hold. Ours (craft held at rest, volley of 16 ticks, frame 4 ticks after it): no flash at the wing tips, and a white burst far ahead where a round hit one of ours' mines (score 0000100). Confidence 55: ours' flash window and round colour were not read, only the one frame compared. Re-read 2026-11-24 over four grabbed frames of the flash (lap 2.000 to 2.050): the original's two bursts change size and outline every tick and differ between the sides, translucent blue-white, with a blue tint on the scene while they last; ours is one identical blown-out white frame for its 5 ticks (crop mean grey 76.7 each tick against 60.3, 79.4, 55.4, 54.9). Read and wired 2026-11-24 (`det-muzzle-flash-re`): `PulseBullet_Update` rolls the flash every tick of its `0.1 s` window as the ordinary Cannon's does (a turn about its forward, a size of 0.25 to 1.0 on all axes, a stretch of 0.7 to 1.3 on forward; the render queue only places it), and the queue submits two vertex lights, a blue `(0, 30, 40)` one of range 2.7 at each muzzle and a faint one on each round (`SpuLight_AddCandidate`, polled live on two boots). Ours now rolls and lights the same: crop mean grey 63.8, 54.6, 52.6, 58.9, 50.0 and 56.6, 58.4, 53.7, 63.3, 50.5 over two volleys (original 55 to 79), bright share 10 to 17 % (original 11 to 22 %), left and right differ; the `cannon-volley` pair 14.99 to 15.00 (unchanged: lights off 15.00 too). The blue wash is wired 2026-11-24 (`det-flash-wash-re`): ours' top-left quarter at the volley goes B 11.9 to 29.2 (original 7.3 to 32.6), the pair's frame is unchanged (its rounds are out of range by then). Still open: the frame strip (`V_Offset` has no bind, inferred never advanced). |
| 7 | Hull | 6.6 % | 0.25-0.29 | 1.6-1.9 | Pose agrees (0.000 units, 0.9 degrees). Original: dark gunmetal body, blue strip lit, cyan wing edges. Ours was a pale silver body with white wing edges. **Cause read and fixed (90, `det-hull-look`, `detonator-look.md`, "The hull's paint"): `emissive` added the hull's second texture as a glow** (`detonator_stripe.gtf`, a facing ramp, on the panels; `detonator_ao.gtf`, the picture itself, on the body), and the panels' iridescent overlay `2 * Colour * AO * stripe(N.V)` is now drawn (`mesh::rcs::hull_overlay`, 85). Side-panel box mean RGB ours `(109.4, 97.4, 98.1)` to `(67.7, 50.1, 50.6)` (`(54.1, 30.9, 32.0)` with the glow refusal alone), original `(60.5, 44.2, 44.5)`; ours is now 6 grey above it, cause open; the other HD pairs unchanged. **Still open:** the fuselage's own overlay (`rich_iridescent`, the iridescence ramp at another coordinate), and the lit blue strip and cyan wing edges (the `Charge1`/`Charge2` block's writer, `detonator-hull.md`). **Cockpit glass fixed (2026-11-24, `det-pad-glass`):** `emissive_constant` (state `0x39`, `SrcAlpha`/`One`) drew black in Detonator's rig; drawn unlit the dome box reads (25, 56, 74) against the original's (34, 70, 91), lit (6, 0, 6) (55, chosen from the frame). The blue segmented gauge on the back is not this lane. |
| 8 | HUD score panel | 5.7 % | 0.46-0.59 | 2.6-3.4 | The numbers, layout and clip pips agree; the translucent red panel in ours lets the bright sky through (the "red translucent layer"), the original's panel sits on a dark scene. Mostly row 1 bleeding through, not a HUD error; panel alpha not separately measured. |
| 9 | HUD stage, energy, clip | 8 % | 0.05-0.14 | under 1 | Close: `1/14`, `100` (both, with ours held at rest) and the `15`/`12` clip match, including the clip falling to 12 after three volleys; the score reads 0 in the original and 100 in ours after its round hit a mine, ours' mines being elsewhere. |

**The volley pair split (2026-11-26, `det-cannon-hot-gap`; `docs/tools/compare-page.md`, "The volley pair's difference, split").** Its
capture state is partly fixed (picture lag off the static scene, ours at the picture's tick, mine ages; 11.47 to 10.32). **Open draw gaps, on every Detonator
pair:** ~~bright striped grilles ours draws where the original is black~~ (fixed 2026-11-26, `det-dark-scenery`: the `scanlinebillboard` hologram's Zone variants draw the Zone surface, `docs/rendering/hd-zone-recolour.md`, "Holographic billboards in a Zone frame"; -hot 10.32 to 8.96, pad 7.60 to 5.88), ~~the score box's white-striped wings (1.2), HUD digits
and `1/14` too wide (1.2)~~ (fixed, `det-hud-score-box`: `docs/formats/hd-hud.md`, "Detonator's score box" and "Text scales on two axes"), hull too bright (0.7). ~~Open capture state: input latency before ours' fire, the volley pair's mine
envelopes~~ carried (`det-cannon-regrab`, "The volley clocks and the picture-clock reads"). **Open from that grab:** ~~ours' mine blast
draws white puffs where the original shows small shards~~ (2026-11-26, `det-mine-blast`: the mine effects draw their disc sprites, the
`Debris` atlas is the shards, `detonator-mine.md`, "The shards, against the original"). ~~Still open there: the destroyed mine is not
carried~~ (2026-11-26, `det-mine-ribbon`: carried, the pair script now keeps a mine that left the pool during the shot; `cannon-volley`
10.80 to 10.53, score 100 on both; `detonator-mine.md`, "Carrying the destroyed mine"). ~~The core near-white where
the original's is dim pink~~ (2026-11-26, `det-mine-blast-glow`: the whole live blast takes the stage's mine colour, read 85 and wired,
`detonator-mine.md`, "The blast's colour"; carried pair 12.31 to 12.12; 2048's `FUN_811e6ebc` and Omega's `FUN_0133ac10` carry the same
law, checked, applies, not wired until those titles answer the mine triggers; the blast takes `detonator_stage(laps)`, not a pinned
grade's stage). Muting the root's glows had no measured effect in that frame (12.12); their draw law is unread.
~~What is left is the `Trail_Ribbon` dot chains~~ (not the whole of it: "`cannon-volley` split by region" below). The ribbon (on the carried pair, muted: 10.22 against 10.53, so even a perfect ribbon leaves it above 10): ours draws each speed-0 ribbon particle as a dot,
the original shows a few thin lines; the far bar (12.41) and the root's noise force (12.15) are both worse. Next: one RPCS3 capture of the
blast's particle quads a few ticks after a hit, and who reads the child instance's `+0x130`/`+0x140` (same page, "The ribbon, open").
The `+100` is drawn (2026-11-27), 15 px above the original's because ours' shot is 9 ticks older at the picture. The craft-speed carry on the rounds is right physics: put back by `v` times their age, ours hits the mine at round age 8 against
the original's 10 (without the carry, 1). But it raised the carried pair to 13.75 (12.98 with the recolour), because the younger blast shows more of those gaps. Re-try it once
the glows are right; it is about 40 lines in `oag_raceplay::detonator` behind a `--detonator-cannon-craft-speed` flag.

**`cannon-volley` split by region (2026-11-26, `det-volley-split`; `docs/tools/compare-page.md`, "The `cannon-volley` difference, split by
region").** 10.53 = mines 3.11 (audio-driven), floor 2.58, right wall 1.01, upper-right screen 0.94, HUD 0.70, far scenery 0.66, hull 0.58,
ribbon 0.30, shot-specific rest under 0.2 each. ~~Next, ready to wire: the visualiser strip's sRGB decode~~ wired on HD (2026-11-27, `det-vis-srgb`; `zone-vis-gamma.md`,
"Wired"): `cannon-volley` **9.11**, all four race pairs fall. **Next:** read the upper-right screen's grey smear
(also on `mines-ahead`). ~~The right wall's missing window lights~~ wired (2026-11-27, `det-volley-wall`; `hd-zone-recolour.md`, "Window lights
on a plain chunk"): the lights draw, the wall row stays at 1.03 (registration); Omega's `tunnel_fx_noAlpha` is checked, applies, not wired. The ribbon is
0.31 of it. **Open:** (1) the floor is now darker than the original's on three of four pairs (second factor, `zone-vis-gamma.md`,
"Still unread"); (2) Omega: its `zoneTexVis` is an sRGB texture too, ours draws its strip raw (`oag_omega` has no `ZoneFrame`);
(3) 2048's HD-lineage circuits author the EQ tints through HD's tables and ours draws the strip raw: read whether their
Zone GXP squares the `zoneTexVis` tap and the strip's gamma mode (`zone-vis-gamma.md`, "Cross-title"), then port or record differs.

**Hull camera and size, re-based (2026-11-23, `det-camera`).** The maintainer's "ours is far, the original is close" was true and the
camera flags were not the cause: the pair fed the RAM body (2 to 4 ticks ahead of the presented frame) with the push-buffer camera, and
ours drew the Detonator hull at 0.75 where the original draws 0.63. Both fixed (`docs/tools/compare-page.md`, "The hull's frame pose in a
Detonator pair"; `ps3-hdfury-eu/camera.md`, "The drawn hull in Detonator is 0.63"). Hull rim box now within 9 px of the original on
top, left, right on all three new grabs (before: 229x134 against 266x248); the bottom is 18-24 px short (nozzles, exhaust flare; ours'
hull rear is lit blue where the original's is dark metal). Whole-frame mean absolute grey difference / RMSE, before then after, against
the same new `theirs.png`: `mines-ahead` 19.81 to 19.81 / 0.174 to 0.175, `cannon-volley` 18.60 to 18.74 / 0.167 to 0.169, `pad` 16.85 to
16.90 / 0.149 to 0.150: **the whole-frame score does not move with the hull** (6 percent of the frame), so the earlier 18.58 / 18.28 / 12.74
stand as the floor-and-sky scores of the old grabs and are not comparable with these (new boots, new music phase). **Open:** 2048 and
Omega draw the hull at the model's 0.75 (`hull_scale: None`) until a live frame reads their hull matrix; the physics and camera-rig use
of the `0.7` scale is wired for the rig only; `--field` still feeds the RAM body (the Detonator pairs do not use it).

**The three pairs re-grabbed and published (2026-11-23, `det-regrab`).** The new grabs of `det-camera` had no world clock and read 1.4 to
1.9 degrees of heading. Both are explained (`docs/tools/compare-page.md`, "Detonator pairs re-grabbed"): the clock read refuses a world clock
under 13.3 s (`GANTRY_WINDOW_END`, a guard that keeps a gantry node from passing for scenery) and a state saved at the grid prompt a few
seconds in restores to a world clock of about 7 at the lap clock's start; a state saved 60 s into the prompt
(`hd-detonator-talons-feisar-grid-late`, world clock 54 to 59 s at the pair) reads 24 nodes. The heading is pitch: `--pose` carries a yaw,
ours' craft pitches 1.6 to 2.0 degrees where the original's pitches 1.9 to 3.2, and the 1.0 degree tolerance stays. Result, on `main` `3f8f627`
(whole-frame mean grey / RMSE of the page's own scorer, two boots each, heading degrees, clock offset s):

| Pair | Mean grey (boot 1, boot 2) | RMSE | Heading | Clock offset | Hull rim box original / ours |
| --- | --- | --- | --- | --- | --- |
| `pad` (lap 2.0) | 10.48, 11.80 | 0.137, 0.147 | 0.35, 0.15 | 54.12, 52.90 | 826,705-1093,943 / 827,706-1094,939 |
| `mines-ahead` (lap 2.67) | 18.53, 19.39 | 0.195, 0.212 | 0.17, 0.79 | 54.12, 52.90 | 826,703-1091,932 / 826,703-1091,935 |
| `cannon-volley` (lap 2.63, 0.75 s after the volley) | 17.42, 20.10 | 0.187, 0.207 | 0.70, 0.85 | 54.72, 54.93 | 823,707-1087,945 / 824,708-1088,943 |

The hull is within 1 to 4 px on every edge but the bottom (3 to 14 px, the exhaust flare). **No pair is under 10 mean grey: `pad` is closest
(10.48, 11.80), the campaign goal is not reached.** Biggest remaining difference, 3 x 3 grid mean absolute grey difference of the frame (grey 0-255, signed ours minus original): the
**upper-middle third on all three** (31 to 34, ours 12 to 14 brighter: ours draws the lit billboard wall and gantry above the track where the
original is near black), then on `mines-ahead` and `cannon-volley` the **middle right third** (25 to 37, ours 19 to 22 darker: the original's
lit mines and, on the volley, debris, where ours' mines are elsewhere and its rims dark). `pad` has no mine in the right third and its next
region is the upper left (11, ours 9 brighter). **`cannon-volley` was re-framed:** at lap 2.21 (0.23 s after the release) the heading
check refused on 3 of 3 boots (1.06, 1.54, 1.10 degrees); the refusal is pitch (`--pose` carries yaw only) and the cause of the extra pitch at 2.2 s is undetermined: unfired frames at lap 2.6 sit inside the
fired range, and no unfired frame exists at 2.21 (open: one no-fire boot at `--shot 2.21`); the fired frames at 2.6 and 3.0 s passed on every boot
(0.70 to 0.89 degrees), and one more boot's volley never fired (the press landed with no tick running), so the pair is the 2.6 s frame. Hull crops read: the fuselage's lit blue strip is missing in ours and its rear is lit pale blue where the original's is dark with a magenta flare.

**The original's bands in the pairs (2026-11-23, `det-pair-bands`).** `--zone-bands` / `--zone-bands-raw` feed the original's held
(`+0x32f8`) and raw (`+0x32b8`) bands, read in the frame's own pause, to the floor visualiser and the mine envelope
(`docs/tools/compare-page.md`, "The original's music bands in a Detonator pair"). The three pairs were re-grabbed (the old ones recorded
no bands; the music phase follows the START RACE tap), so "before" is ours silent against the new `theirs.png`. Whole-frame RMSE
silent to fed: `pad` 0.154 to 0.158, `mines-ahead` 0.206 to 0.200, `cannon-volley` 0.201 to 0.198 (a repeat boot: 0.150 to 0.169 and
0.246 to 0.238). Floor box A mean RGB, original / silent / fed: `pad` `(6.7, 0, 19.1)` / `(0.1, 0, 0)` / `(21.5, 4.3, 45.5)`,
`mines-ahead` `(5.1, 0, 14.0)` / `(0, 0, 0)` / `(15.1, 2.9, 33.1)`, `cannon-volley` `(6.2, 0, 18.3)` / `(0.1, 0, 0)` / `(25.8, 3.7, 51.7)`; box B
`(9.5, 0, 29.1)` / `(0.7, 0, 0)` / `(36.3, 8.0, 74.7)`, `(7.4, 0, 24.9)` / `(0.6, 0, 0)` / `(31.0, 6.6, 65.3)`, `(8.5, 0, 28.2)` / `(0.7, 0, 0)` /
`(44.8, 7.1, 88.2)`. **Result: the held bands are live under the muted emulator and non-zero at the grab (band 3 between 0.51 and 0.93
on 5 frames), the floor lights in the right hue, and ours' blue is 2.2 to 3.1 times the original's (5 times on one repeat) with RMSE barely
moving.** Ours' box blue against band 3 alone is gamma-shaped (0.29: 33.8, 0.58: 45.5, 1.0: 58.0, about level^0.43), so matching the
original's 19 would take a presented band 3 near 0.07. Whether the original's box tracks band 3 is **undetermined**: the frame is 0 to 3
ticks older than the RAM read and on three of five frames raw equals held (rising at the read); the two falling frames order as band 3
predicts. **Open:** grabs where raw is below held, or a lag-scanned frame; the visualiser's texel-to-pixel gain in ours; the dashes (faint
in ours, bright violet in the original, not measured); the mine rims (a one-frame capture gives the envelope no history, and ours picks its
own band and places its own mines).

**Floor visualiser, re-measured (2026-11-23, `det-floor`).** The three pairs re-rendered on `main` with the recorded bands (`--zone-bands`),
then with the glow's depth factor on the original's near plane (`zone-shader.md`, "The glow's depth factor": near 0.4773-0.4805 on the
Detonator pairs, 0.5417 on circuits, off `view_proj`). Whole-frame mean grey / RMSE, floor box A `x250..800 y780..850` and B `x1090..1230 y780..1000`
mean RGB ours / original:

| Pair | Mean grey main to after | RMSE | A ours before to after / original | B ours before to after / original |
| --- | --- | --- | --- | --- |
| `mines-ahead` | 19.96 to 18.58 | 0.200 to 0.198 | `(15.1, 3.1, 33.2)` to `(9.8, 1.5, 23.3)` / `(5.1, 0, 14.0)` | `(31.0, 6.8, 65.4)` to `(20.4, 3.3, 46.4)` / `(7.4, 0, 24.9)` |
| `cannon-volley` | 19.60 to 18.28 | 0.198 to 0.192 | `(25.8, 3.6, 51.7)` to `(17.6, 1.7, 37.3)` / `(6.2, 0, 18.3)` | `(44.9, 7.1, 88.2)` to `(31.2, 3.7, 64.1)` / `(8.5, 0, 28.2)` |
| `pad` | 14.52 to 12.74 | 0.158 to 0.149 | `(21.5, 4.2, 45.5)` to `(14.2, 1.8, 32.2)` / `(6.7, 0, 19.1)` | `(36.3, 7.9, 74.7)` to `(24.6, 3.8, 53.4)` / `(9.5, 0, 29.1)` |

Ours' blue is still about 1.7 to 2.3 times the original's (a constant of about 2 in linear light is unread: the vis texel's gamma flags at
`0x003d6dc8`, and the original's floor blue is nearly flat across held band 3 0.51 to 0.93, so the texel the frame sampled is unmeasured; recipe in
`zone-shader.md`). Where the rest of the mean grey sits (6 x 8 grid, signed ours minus original, any pair): the mid-distance red band (ours 30 to 55
darker: the Track Fog `1.8 0.141 0.007` is not drawn), the upper right (ours 15 to 40 brighter), the floor (ours 10 to 20 brighter, now about 5 to 10),
the mines and pad. Ours' floor also carries more red than the original's (box A red 9.8 against 5.1).

**Track fog and the glass floor, wired (2026-11-24, `det-mines-ahead-structure`).** The mid-distance red band's cause is wired: a track chunk
takes the stage's Track Fog (`mesh_render::TrackFog`), and `etched_glass_tech`'s Zone variants run no facing-ramp sheen (the "grey-green
structure" on `mines-ahead` was this glass floor, present in the original, dark). `mines-ahead` 14.65 to 13.74, `pad` 8.17 to 7.68,
`cannon-volley` 15.00 to 14.18; evidence on `detonator-look.md`, "Track fog and the glass floor". The far-curve box's "8 low" (`det-glass-box-colour`, 2026-11-25) is the
mine drawn black in ours and pale violet in the original: without it the box is within 1.8 grey per channel (`detonator-look.md`, "The \"8 low\" was the mine"); the mine's lit look is `det-colour`'s. Open: the Zone variant's grid and paraboloid reflection; 2048/Omega Track fog (checked, applies, not wired: no original frame).

**The vis texel's gamma flag, read (2026-11-23, `det-floor-gain-re`, `docs/ghidra/functions/ps3-hdfury-eu/zone-vis-gamma.md`).** `zoneTexVis`'s address
word is `0x60730303` (gamma nibble 7, confidence 75): the original sRGB-decodes the strip, ours samples it raw into the linear sum. Applied alone (scratch,
reverted) it overshoots: box blue ours over original goes from 1.7-2.3 to 0.38-0.89. ~~Ready to wire on its own~~ wired (the page, "Wired"). Open, in
order: the viewport z mapping (`SET_VIEWPORT_SCALE`/`OFFSET` z; a doubled depth factor is about 1.33x after the encode), the band-independent floor term
(the original's floor red 5-9 is not the strip: Track Fog, the Detonator radial bloom), and one RPCS3 frame dump confirming `0x60730303` on the vis unit.

**The upper-middle third, partly found (2026-11-23, `det-scene`).** Cause: the stage's zero `Fog density` was read as "keep the circuit's
fog", so Talon's Junction's lilac fog (`0.26 0.22 0.38`, `0.001`) lay over the far scenery where the original's frame is black. Now a
stage that authors density 0 has no fog (`ZoneGrade::fog`; `detonator-look.md`, "Fog"). Cell (3 x 3 grid, mean absolute grey
difference, signed ours minus original) and whole-frame mean grey, before to after: `pad` 31.0 (+12.3) to 21.7 (+2.3) and 10.48 to
**8.81**; `mines-ahead` 32.3 to 24.7 and 18.53 to 16.23; `cannon-volley` 33.5 to 25.7 and 17.42 to 14.88. `pad` is under 10, the other
two are not. "Well under half" of the cell is not reached: what is left there is a white-grey ribbed slab over the track at the right
of the cell (near black in the original; not the ambient, sky reflection, bands, shadow tier, track shine or the scene-class surface,
which reads black at the slab; its draw is unfound) and the HUD panel (about 45 % of the cell; the cell without it is 10.5, 15.0 and
14.1). **Zone stages 0, 13 and 14 now draw no circuit fog too** (they author density 0); no original frame of those stages is held.
Next biggest difference per pair: the middle right third on `mines-ahead` (36.4, ours 20.7 darker) and `cannon-volley` (26.2, ours 17.8
darker), the lit mines and debris; `pad`: the upper-middle cell (21.7) then the lower-middle (14.4, ours 7.3 brighter).

Cheapest first was row 1, then row 2's gantries, then the bloom arm (all three wired, below), then rows 7 and 4.

**Zone's look in Detonator (read 2026-11-22, `det-look-re`, `detonator-look.md`).** Mode 14 is in the effect gate's mask
(`0x00206040`: modes 6, 13, 14, 21), so the Zone pieces that test the byte and not the mode run in Detonator: the
`ZoneSky.gtf` swap and gradient dome, the stage block's publication, the billboards' shared stage target. The mode-keyed
pieces have Detonator arms of their own: the settings file (`DetonatorMode[DLC3].effectSettings`), the texture sets, the
stage source (`RaceManager+0x2e10`) and the bloom block. Stage 1 authors Sky horizon and zenith `0 0 0`, Sun `0`, Fog
`0`, Scene colours `0`, Track Texture Colour `0.97 0 0`, EQ tint `171 9 255` (full table on the page).

**How ours applies it today.** `zone_grade` (`crates/raceplay/src/load/environment.rs`) already builds Detonator's grade
from `oag_hd::race::DETONATOR`: its file and DLC3 split, both texture sets, the lap-counter stage; fog, light rig, Track
and Scene recolour, hull sun and probe tint follow it per frame. So the track and scene colours are the file's.

**Wired 2026-11-22 (`det-look`).** 1. Sky: `hd_sky_model` takes `mode.on_zone_mask()`; Detonator on Talon's Junction loads
`ZoneSky.gtf`, draws the dome, and stage 1's horizon and zenith are `(0, 0, 0)` (`zone_sky_ground_truth::a_detonator_race_draws_the_zone_dome_in_its_own_stage_colours`).
2. Billboards: `zone_shares_texture` reads the new `Adverts::detonator_shares_one_texture` (HD `true`; Omega and 2048 `false`,
their Detonator frames are unread), so slots 1 to 7 show the stage's equaliser target in Detonator on HD only. 3. Bloom:
`bloom::for_mode` takes the table's feedback, sizes, frame exponent and alpha for Detonator and keeps the circuit's frame
contribution and adaption boost (`ZoneFrame::detonator_arm`, HD only). Upper band `x 200..900, y 60..300`, ours before then after
against the original `(17, 10, 10)` on mines-ahead: `(96, 111, 121)` to `(16, 8, 8)`; pair RMSE 0.289 / 0.297 / 0.264 to
0.205 / 0.223 / 0.159 (cannon-volley / mines-ahead / pad), mean grey 35.9 / 41.5 / 28.3 to 20.6 / 20.9 / 12.6.

**Still open from this list.** The bloom arm's radial values (contribution `0.1`, exponent `0.1`, distance scale `0.6`, enabled)
are not applied: `oag-post` has no radial bloom pass, in Zone either (`zone-frame.md`, "Open"). The slot 8 gate in
`gantry_visibility.rs` (`mode == Mode::Zone`, the card's Zone-grade exemption) was tried on `on_zone_mask()` and drew a
pixel-identical Detonator start frame (whether the exemption fires there was not separated), so it is left. **The pairs' violet
billboards need a spectrum:** the re-render runs `--no-audio`, whose held bands are zero, so the equaliser target is a black grid
on black and the pairs show no violet panel; with `--zone-spectrum-test` the same pad frame draws violet bars on the billboards
where the original's are (checked by eye, upper right). Row 3's floor violet is the spectrum-driven `zoneTexVis` (row 3), not a missing term; row 2's gantry violet and row 8 are not fixed by
these three: the frames still show a lit grey scenery band at upper right where the
original is dark (Scene colours authored black), so the grade's Scene recolour is the next suspect. Re-measure
the hull (row 7) and the mine and bomb colours (row 4): the colours' binds are `det-colour-re`'s reading.

What stays as it is: palette, textures, stage source, hull model, handling. 2048 and Omega: Omega's `zone_sky` is `None`
(sky source unread), 2048's Detonator sky is unread and has no frame pair; nothing to wire there yet.

**The cannon button (read 2026-11-22, `detonator-cannon.md`, "The buttons", 80): ours is right.** The cannon reads the
controls record's `+0x1a` (the ordinary *use* hold), the reload `+0x1b` (the *absorb* button), the EMP `+0x1c` (L1). From
`Pad_Update`'s index table, use is SQUARE, absorb is CIRCLE and change camera is TRIANGLE. The pairs lane's "TRIANGLE
fires, SQUARE changes the camera" is the harness: `rpcs3_pad.py` sends `triangle` as `BTN_NORTH` and `square` as
`BTN_WEST`, which RPCS3's evdev default reads as Square and Triangle, so the two reach the game swapped (swap
mechanism 70; the live result and the static read agree independently). Ours' Square fires and Circle reloads: no change
(the reload is a press edge in ours, chosen; the original's level-or-edge is unread). Also measured: the craft does not
steer itself (auto-speed is speed only);
with no stick it keeps the line for about 6 s on the first straight, and it flies into the first laid mine near 3 s (original: energy 100 to 90, an
orange screen flash, grey shards and a blue shield glow, scratch capture, no pair).
A START RACE tap before the prompt is up is held into the race and sends the craft off the track within 4 s (the capture script
waits 30 s after a restore).

The pair scripts' `--press`/`--hold` names are the harness's: its `triangle` is the game's SQUARE (fire) and its `square` the game's TRIANGLE (camera). A script that fires the original's cannon uses `triangle`; ours' fires on its own Square.

Not reached: **the EMP and the pad pair's contact.** One steered attempt (stick right 0.5 s from lap clock 1.5 s) moved the craft
toward the pad but into a mine first; the pad is at the right edge about 2.0-2.3 s into the lap and needs a steered line past the
first mines. The EMP pips, L1 and the wave were not captured. `rpcs3-hd-detonator-shots.py --steer` and `--emp-clock` exist for a
next try (tune the stick window; mines near 3 s may need the cannon fired first).

**The original's mines (2026-11-23, `det-mines`; `docs/tools/compare-page.md`, "The original's mines in a Detonator pair"; `detonator-mine.md`, "Where a mine stands").**
Claim: the middle-right cell of `mines-ahead` (36.4) and `cannon-volley` (26.2) falls well below its value and the frame toward under 10.
**False as it stands.** *Placement is carried exactly*: the pair reads the 80 `DetonatorMine` objects (heap scan, vtable `0x008642f0`) and
ours lays each on the survey lane nearest its position (`--detonator-mines`, `RaceSim::carry_detonator_mines`); the layout is the same to the bit on
three boots, all 80 within 0.47 units across the section, and the objects stand **3.0 above** ours' lane point (`MINE_LIFT`, now the mine's draw,
contact, cannon and EMP point). But with the mines in the original's places the cell stays at 35 to 46 (seeded 36.4): **the look dominates, not
the placement** (ours was a small dark rim disc, the original a lit white-violet shell with its holes and long spikes about twice the size). Size
breathing is wired (the original's `m+0xd8 = 2.3 s` read back to the digit), and the frames get *worse* (`mines-ahead` 16.23 to 18.59, `cannon-volley`
14.88 to 15.24, `pad` 8.81 to 8.80) because **the original draws its near mines about 30 % closer to the screen centre than their objects**
(`mines-ahead`; 0.87 on the volley; none at 83 to 89 units in `pad`; the far mines exact): undetermined, and ours' mines are 7 units nearer. Per pair
cell MAD (signed ours - original) seeded to carried+lift+size: `pad` 6.3 (-2.1) to 6.3 (-2.1), `mines-ahead` 36.4 (-20.7) to 46.4 (-12.7), `cannon-volley`
26.2 (-17.8) to 27.6 (-17.0). **Next:** (1) the displacement: read it from a third frame at another depth (a `--shot` at 2.3 s puts the three near mines
at 30 to 40 units), or have the maintainer say whether the original's mines are drawn ahead of their objects; (2) carry each mine's `s`/`t`
(`+0xf4`, `+0xf8`, read in the same pass with `OAG_MINE_RAW=1`) so a pair's mine is the original's brightness and size, not a history-less envelope;
(3) the pairs stay unpublished with the flag until (1).

**The displacement is the pair's camera (2026-11-24, `det-mine-look-re`; `detonator-mine-draw.md`).** The original draws each mine part at its
object's position (`SceneNode_SetLocalMatrix` `0x00327500`, no view term); the pair's push-buffer camera is 1.5 to 4 ticks of travel newer than
the picture (`mines-ahead` 3.55, `cannon-volley` 2.30, `pad` 1.55, fitted on the near mines, one cross-track scenery edge agrees; 70, not
reconciled with `hd-pair-lag-scan.py`'s 0 to 3 ticks: settle that first). The "twice the size" reads as the ring and the long spikes around an
ordinary football (by eye on one mine, 55): the ring is the `Electric_Visualiser` (bounding radius 2.45), a camera-facing billboard ours does not
draw (`DetonatorMine_PoseVisualiser` `0x00139308`, scale `1 + 0.4 (m+0xd8 - 1)` live, `0.9` dark; 65). **Ready to wire, in order:** (1) pair
tooling: render ours from the picture's camera, moving `--camera-pose` and `--pose` back by a per-grab fitted `k` (`compare-page.md`, "The picture
is older than the push buffer's camera"); likely moves every HD pair with near scenery; (2) draw the visualiser billboard; (3) carry `s`/`t` per
mine (above). Superseded: Next (1) of the paragraph above.

**Row (1) is wired for Detonator (2026-11-24, `det-pair-lag`; `compare-page.md`, "The presented lag of a Detonator grab").** Reconciled: the picture is 6.0 ticks behind RAM on 15 grabs (85), which is 2 or 4 behind the push buffer as the hull's skew is 4 or 2; `hd-pair-lag-scan.py`'s 0 to 3 was a race-mode whole-frame score. `rpcs3-hd-pair.py --detonator` fits `k` per grab from the mines (`hd_picture_lag.py`) and moves ours back; mean grey 25.3 to 17.5, 20.3 to 17.2, 23.5 to 17.1 on three pairs, unchanged where no mine is 8 to 50 units ahead (`pad`, `cannon-volley` at 2.2 s). **Next:** grab `pad` and `cannon-volley` at 2.6 to 2.7 s so a fit exists; a stop within 8 ticks of the previous one reads 4.5 to 5.5 and fits worse (50, mechanism unknown); race-mode lag is unmeasured, so `--field` stays uncorrected; the heading check refuses most fresh Detonator pairs at 1.0 to 1.7 degrees (open pitch).

**Pitch found, mines-ahead re-grabbed with the fit (2026-11-24, `det-regrab-fitted`).** The heading refusals were ours: `--pose` plus
`--pose-hold-ticks` held the spline attitude (forward y 0.030) where the original body and ours' own free flight read 0.041 to 0.047. The pair now
feeds the push buffer's hull attitude as an exact pose; heading reads 0.000 on every pair (`docs/tools/compare-page.md`, "The pitch the heading
check refused"). `mines-ahead` re-grabbed as the first stop of a boot: fit 6.03 and 6.15 ticks, mean grey 16.40 published to 14.65 and 15.79
(fit off 15.75 and 17.23); `pad` 8.82 to 8.43; `cannon-volley` 15.33 to 14.99 (the 2.2 s frame, refused before, now makes a pair at 12.95).
Goal under 10 reached on `pad` only. **Next:** the mine's body (dark ball with ring where the original's is a bright spiked sphere), the lit billboard
wall, the hull's lit strip; a cannon-volley grab at 2.7 s with the fit (the stored `b6/01` read 16.65 fitted) only if the mine body closes first.

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

**The visualiser ring is drawn on HD (2026-11-24, `det-mine-visualiser`; `detonator-mine-draw.md`, "The visualiser, wired").** Row (2) above: the
ring is posed camera-facing and drawn with `electricity.rcsmaterial`'s own program. Published pairs (no picture-lag fit): `pad` 8.82 -> 8.82,
`mines-ahead` 16.38 -> 16.40, `cannon-volley` 15.32 -> 15.33; middle-right cell MAD 36.79 -> 36.87 and 26.42 -> 26.46: the ring lands on the wrong pixels
because ours' near mines are not where the picture has them (re-grab the pairs with the fitted lag, then re-measure). The original's ring is two smooth
circles; ours a ragged band at the disc's rim: the ring's clock `t`, the unread level readers and the roll are the candidates. **Omega draws it too (2026-11-25, `det-visualiser-2048-omega`;** its GCN program is HD's law, `detonator-mine-draw.md`). **2048 draws it too (2026-11-25,
`det-visualiser-2048`):** its USSE program is HD's law with two prefetched taps (five fetches, not three) and the colour-bearing taps squared
in code (`detonator-mine-draw.md`). **Open:** the original's ring look against ours on a lag-fitted pair (HD; no 2048 or Omega capture of the ring).
