---
status: ready-to-wire
titles: [hd, 2048, omega]
updated: 2026-11-23
summary: "Detonator: mines, bombs, hull, cannon, HUD, EMP, auto-speed, end screen, Zone scene wired; radial bloom, floor violet, hull strip, mine colour open"
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
| 8b | Wire the hull, handling, mine, bomb and pad models | done (wreck, death shell, colour binds open) | `4f4b70173` |
| 9 | Wire the cannon, score, destroyed count and percentage | simulation done; rounds, flash, explosion effects and the cannon's cues drawn on HD; the HUD is row 12's, no end screen | `59215fc52`, the `det-fx` commits |
| 10 | EMP: RE (pads, charge flag writer, `EMPManager`) then wire | done on HD and Omega (`detonator-emp.md`, "Wired"); the wave's reach is chosen (`emp::SECTIONS_PER_EXTENT`); 2048's pads unread, so its EMP never charges | `det-emp-wire` |
| 11 | Auto-speed: RE Detonator's engine branch, then wire | done on HD, 2048, Omega (`detonator-speed.md`, "Wired"), ramp included | `det-emp-wire` |
| 12 | Detonator HUD (`detonator_hud.xml`, `hud_detonatorweapons.xml`, `detonator_warnings.xml`) | wired on HD, Omega and 2048, the EMP pips and `EmpReady` included; open: `EmpBarEffect`, 2048's `EmpCharge`/`EmpFull`, bomb-ahead, score feedback and the medal models (below) | |
| 13 | End screen proper (stage, mines, bombs, three chain counts, score, new-record banner) | HD wired (`det-end`, `detonator-endrace.md`, "Wired"): the six rows and the score line, live; open: the new-record banner, Omega (no EndRace screens on main) and 2048 (its four-row list has no page to draw on) | `det-end` |
| 14 | Campaign launch (`race_mode_for_cell` maps `Other("Detonator")`; medal = the cannon score, `>=`) | done: 12 cells on HD, Omega, 2048; see `git log` (`Detonator campaign`) | see `git log` |
| 15 | Omega and 2048 parity leftovers | open | |

A race can already be launched from the command line:
`oag-game --race --mode detonator [--autopilot --ticks N] data/images/hdfury-ps3-eu-dec.iso`
(HD EU disc in `data/images/`). The menu launches it from a campaign cell (phase 14, done).

## What a player gets today (HD)

14 stages; each stage lays up to its `MineCountStage` mines and at most one bomb; a mine costs 10
energy and half the speed, a bomb 20 and a push; energy at zero ends the race. Every craft flies
the Detonator hull in its team livery on Detonator's own handling file; mines, bombs and pads draw
the Detonator models (the mine football draws plain white: its `MineColour`/`MineBrightness` binds
and the bomb's `BombColour` are unread). Square fires the pulse cannon and Circle reloads; rounds
destroy mines and bombs and the race keeps a score, a destroyed count and a percentage, but no
the rounds and muzzle flashes are drawn and a shot mine, a bomb and a round on a wall play their disc effects (HD); the HUD is row 12's. The craft drives itself (Zone's
auto-speed on the stage); pads charge the EMP instead of boosting, L1 fires it and its wave
clears the mines ahead (wave reach chosen).

Open from 8b: no wreck or death shell (HD's PS3 hull branch loads no wreck in any mode; the
Detonator `shipwreck.vex` is an outline placeholder); the circuit's own pad is hidden under the
Detonator pad (live frames, 60; `detonator-emp.md`); the model's origin at the pad transform is
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
  `detonator_hud_ground_truth.rs`). **Still open:** the three medal models (HD `<ImageModel>`
  `.vex`; no HUD path draws one, and the cell's thresholds are not passed into the race), the EMP
  widgets (phase 10), the bomb-ahead warning and `ScoreFeedBack0..15` (need an event stream),
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
  | `ScoreFeedBack0`..`15` | a floating `+N` per score add (`+%3.0f`) | text | **missing**: the sim exposes only the running score; add a per-tick score-add event list (det-fx's per-tick event stream is the natural carrier), or approximate with the score delta | needs an event |
  | `DetonatorGold/Silver/BronzeMedal` | model when the score passes the cell's thresholds | model | `crate::medal_watch::detonator_value` + the cell | ready (phase 14's value) |
  | `ForwardWarning*` | bomb-ahead warning, once per stage, 2.0 s | icons | **missing**: `Detonator_IsBombAhead` (500.0 track measure + straight distance, `0xd2a30`/`0x588f0` unread) and the per-stage latch. On HD message 8 selects `ForwardWarningMineIcon`, which `Detonator_warnings.xml` does not author, so HD probably draws no bomb icon (55, `detonator-hud.md`) | needs RE + a capture |
  | `Emp_0`..`7`, `EmpReady`, `EmpBarEffect` | `n = rm+0x2dd8` pips lit while `n > i`, `EmpReady` at `table+0x1bc`, gauge `min(n/8, 1)` | pips | `RaceSim::detonator_emp`, `detonator_emp_gauge` | pips and `EmpReady` wired (HD, Omega); `EmpBarEffect` not drawn (an `<ImageModel>`); 2048's `EmpCharge`/`EmpFull` open |
  | `PulseScore`, `PulseMultiplier`, `EmpScore`, `PulseBankedScore` | **nothing on HD**: the controller never binds the first two, never reads the third and its banked-score text setter has no caller | - | do not draw `x%02d` or the `%04d` counter | skip |

  Also from the page: the reload gauge `(1 - lockout/total) * 15` that `Detonator_UpdateHud`
  computes feeds `0x0029d578`, a shader constant of the craft's own
  `detonator_bulletgaugesegments` material (an ammo gauge on the hull, 55), not a HUD widget; the
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
  - **The new-record banner** (`RecordNotifyBlock`, `RecBlockBlue`/`RecBlockGrey`, `RecValue` =
    `ER_NSR` + score): needs the best-score table `0x0001ef28` returns (unread, 72) and a Detonator
    best-score store (none on main); drawn off, `RecBlockGrey` included.
  - **Omega**: same filler (`FUN_0155bc90`) and ids; but Omega's `endrace_entry` is `None` (a finished
    race keeps the built-in scoreboard; wiring HD's reader leaves `EndRace Menu` with no options,
    `docs/formats/omega-status.md`), so no Omega Detonator screen draws until that is solved (a
    separate follow-up: Omega's `EndRace Menu`). Its served table carries all the ids (tested).
  - **2048**: its four-row list (stage = the HUD stage with no `/ 14`, mines `%.1f%%`, bombs `%d`,
    `IG_HUD_SCORE`, no chain rows) is modelled and worded (`Detonator2048Results`, tested) but not
    drawn: it belongs to its `Results` page, a named gap (`docs/ui/endrace-2048.md`); the touch
    Summary page has no widget for it. No layout is invented.
  - **No live frame of the original** (the grey ink, the `x = 590` left edge, the `RecLogoD` art,
    whether `MiniText` `ER_FINALPOS` is hidden: the Detonator branch writes none of it).
- **14 Campaign: done.** `race_mode_for_cell` maps `Other("Detonator")`; `RaceStage::campaign_tier`
  takes the score through `medal_watch::detonator_value` (finished races only, `>=`). 12 cells per
  title, not 14 (`docs/formats/race-campaign.md`, "Detonator cells launch"). The value handed to the
  compare is the score, read (`0x0005c3c0`, 82; `detonator-endrace.md`). A destroyed run records
  too (both endings call the builder with `force = 1`).
- **15 Parity.** Omega race-start placement call, stage cap into `craft+0x5e4`, grade drawing;
  2048 per-stage `.gxt` textures.

Overall risks: most of EMP, auto-speed and the HUD is unread (RE-first, may need Ghidra
AltiVec workarounds: `llvm-objdump -d --mattr=+altivec,+vsx` and `scripts/ps3-toc.py`); the
placement RNG cannot match the original's `Libc_Rand` (a departure, documented).

## Open

- **Mine and bomb colour binds: RE done, ready to wire except the shader combine** (2026-11-22,
  `docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-material.md`, 66-78, static; colour channel order inferred). Laws: the
  mine's `MineColour` is the committed stage's `Detonator Mine Colour` bytes / 255 on football and
  spikes (a dark mine: `(2,2,2)` / `(10,10,10)`, `MineBrightness` 0), `MineBrightness` is `2 * s`
  with `s` an attack-over-0.3 / decay-x0.95 envelope of one band (football: random band 0..3 per
  mine, spikes: that band + 11); the bomb's `BombColour` is `Detonator Bomb Inner Colour`,
  `BombEQColour`/`Colour`/`RainbowColour` the `Outer Colour`, brightness `0.2 + 2f` / `0.8 + 2f`
  from a decaying flash. The table is `detonatormode`/`detonatormodedlc3.effectsettings`, 15 stages,
  through `EffectSettings::stage_rgba8` (four `key::` constants to add); the stage is
  `ZoneGrade`'s lap-stage counter and the colour snaps on a step. The audio input is
  `oag_audio::spectrum::Spectrum::levels()` (16 bands, as `SoundSystem_GetBandLevel`); render side
  only. **No per-draw named-constant path exists** (`crates/mesh/src/mesh/rcs/*` read authored
  parameters once at load): the wiring is a per-instance override of two or three `float4` slots on
  the mine, spikes, bomb, equalisers and glow-edge pools in
  `crates/raceplay/src/detonator_models.rs` and `scene/weapon_models.rs` (several hours). **Read
  first:** the `detonator_mine_normal_alpha` fragment program's use of `MineColour` and
  `MineBrightness` (`oag_rcs::rcsmaterial::fragment::Program`), unread, so the dark mine's look is
  not yet a picture. 2048: checked, applies (byte-identical effect-settings file, same bind
  strings), not wired. Omega: checked, applies (same file; binds by hash constants), not wired.
- **Own hull, handling file and object models: wired** (2026-11-22; the wreck and death shell
  (HD's hull build loads no wreck in any mode), the mine/bomb colour binds (read, see the
  "Mine and bomb colour binds" bullet below) and 2048's object models stay open (the circuit's own pads are now hidden);
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
- **EMP: wired** (`det-emp-wire`, `detonator-emp.md` "Wired"). Open: the wave's reach and speed
  (chosen; recipe on the page), the kill's effect and sound, whether the wave hurts the player,
  `EmpBarEffect` (an `<ImageModel>` the HUD layout reader does not model; the gauge is exposed),
  the ready pulse and post-fire flash on `EmpReady`, 2048's
  pad arms (unread: its pads keep boost and pickup, so its EMP never charges) and its
  `EmpCharge`/`EmpFull` widgets, Omega's EMP announcer lines (Wwise, not located).
- **Auto-speed: wired** (`det-emp-wire`, `detonator-speed.md` "Wired"). Open: a player-input speed
  curve on a clean lap against the original; HD's own Zone ramp (this build's Zone has none).
- **Score, end screen, HUD.** Score and percentage are simulated and shown: the HUD is phase 12's,
  the end screen's rows phase 13's (HD wired, above). Campaign cells (`mode="Detonator"`, 12 on the
  Fury grids of HD, Omega and 2048's pack) launch and are medalled on the score (phase 14);
  the mid-race medal cues are phase 12's.
- **The out-of-race ending** (`+0x6a64 & 0x1000`) is not modelled. Its writer is read
  (`Ship_SetState` state 4, `detonator-mine.md`); Phase 7 wires it with the contact law.
- **Omega's grade** is not drawn (Omega draws no Zone grade either); Omega's camera scale is
  inherited from HD, its craft constructor unread. **2048's per-stage `.gxt` textures** are
  unwired, as Zone's are.
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
| 3 | Lower band (near track, hull, y > 640) | 41 % | 0.14-0.22 | 6-9 | The original's track is a saturated blue-violet with red edge strips; ours is near black with the same red strips. Brighter by 10-19 grey in the original. Ours already applies Detonator's grade (its file, texture sets, Track recolour `[0.97, 0, 0]`); what is missing is the violet: the floor's dashes read `(132..143, 6..7, 199..211)`, the hue of `EQ colour tint` (60, one frame), and the bloom's Detonator arm (below). 0.22 on the volley pair (the ground ahead of the hull lit by the flash in the original). |
| 4 | Mines | 1-3 % (by eye) | not cropped | 1-2 (estimate) | Original: a spiky purple-white sphere with a lit shell, grows large as it nears. Ours: a small hard white ball (`MineColour`/`MineBrightness` binds unread, known); ours also shows no spikes at a distance. |
| 5 | Detonator pad | 2-3 % (by eye) | not cropped | 1 (estimate) | Original: a white slab with red squares, matt. Ours: a purple glassy slab with cut-out squares. Different material and colour; position and size agree. |
| 6 | Cannon muzzle flash and rounds | under 1 % (by eye) | not cropped | under 1 (estimate) | Original: two blue-white flashes at the wing tips and blue-white rounds with a trail, still lit 6 ticks after the hold. Ours (craft held at rest, volley of 16 ticks, frame 4 ticks after it): no flash at the wing tips, and a white burst far ahead where a round hit one of ours' mines (score 0000100). Confidence 55: ours' flash window and round colour were not read, only the one frame compared. |
| 7 | Hull | 6.6 % | 0.25-0.29 | 1.6-1.9 | Pose agrees (0.000 units, 0.9 degrees). Original: dark gunmetal body, blue strip lit, cyan wing edges. Ours was a pale silver body with white wing edges. **Cause read and fixed (90, `det-hull-look`, `detonator-look.md`, "The hull's paint"): `emissive` added the hull's second texture as a glow** (`detonator_stripe.gtf`, a facing ramp, on the panels; `detonator_ao.gtf`, the picture itself, on the body), and the panels' iridescent overlay `2 * Colour * AO * stripe(N.V)` is now drawn (`mesh::rcs::hull_overlay`, 85). Side-panel box mean RGB ours `(109.4, 97.4, 98.1)` to `(67.7, 50.1, 50.6)` (`(54.1, 30.9, 32.0)` with the glow refusal alone), original `(60.5, 44.2, 44.5)`; ours is now 6 grey above it, cause open; the other HD pairs unchanged. **Still open:** the fuselage's own overlay (`rich_iridescent`, the iridescence ramp at another coordinate), and the lit blue strip and cyan wing edges (the `Charge1`/`Charge2` block's writer, `detonator-hull.md`). |
| 8 | HUD score panel | 5.7 % | 0.46-0.59 | 2.6-3.4 | The numbers, layout and clip pips agree; the translucent red panel in ours lets the bright sky through (the "red translucent layer"), the original's panel sits on a dark scene. Mostly row 1 bleeding through, not a HUD error; panel alpha not separately measured. |
| 9 | HUD stage, energy, clip | 8 % | 0.05-0.14 | under 1 | Close: `1/14`, `100` (both, with ours held at rest) and the `15`/`12` clip match, including the clip falling to 12 after three volleys; the score reads 0 in the original and 100 in ours after its round hit a mine, ours' mines being elsewhere. |

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
where the original's are (checked by eye, upper right). Row 3's floor violet, row 2's gantry violet and row 8 are not fixed by
these three: the frames still show a dark floor with no violet dashes and a lit grey scenery band at upper right where the
original is dark (Scene colours authored black), so the grade's Scene recolour or the visualiser glow is the next suspect. Re-measure
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
