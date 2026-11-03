---
status: measure
titles: [all]
updated: 2026-11-03
summary: "Open cleanups before a mod pickup lane: reset triggers, spline placement, pickup lookup, mod ABI command table, camera FOV chain."
kind: debt
---

# Refactors in the code a mod pickup lane would touch

Scout reading of `main` at `038fa0b3a`, 2026-11-03. Nothing here is built. Every
item pays off without the lane, and each one names what it would save the lane.
Sizes are effort for one member, with confidence 0-100 (these are scout readings,
so 20-40 by default). Ranking is payoff times the chance the lane touches the code.

## Answer to the 2048 per-class question (sweep item 1)

**Today's table expresses the odds per class. It cannot express the rocket count.**

- **Odds per class: yes.** 2048's `weaponstats_Race_2048.xml` carries five
  `<Pickupodds class="...">` blocks: `Venom` (line 52), `Flash` (93), `Rapier`
  (134), `Phantom` (175) and `SuperPhantom` (216). `oag_weapons::pickup::table_for`
  (`crates/weapons/src/pickup.rs:509`) matches the name case-insensitively, and the
  caller `crates/raceplay/src/pads.rs:316` passes `self.sim.class`. A fifth class
  needs no code for the odds. (The `SpeedClass` enum has four rungs, as
  `docs/formats/2048-campaign.md:219` records; the odds do not go through it.)
- **Rocket count: no class column.** It is a per-craft field. Each team folder
  `HandlingStats/<team>/<1..4>/handlingstats.xml` has one `<Weapons>` line (line 17
  in `feisar2048/2`, `number_of_rockets="2" number_of_mines="4"`). The values seen:
  `1` or `2` rockets, `none` on folders 1 and 4 (which carry `lockon_speed_multiplier`
  and `gift_cannon_time` instead). Example: `feisar2048/3` has `number_of_rockets="1"`.
  No crate reads `number_of_rockets`, `number_of_mines` or `weapon_damage_multiplier`
  (`rg` over `crates/` finds only the function `weapon_damage` in `physics/src/damage.rs`).
- **The fire side is fixed at three.** `ROCKET_SHOTS: usize = 3`
  (`crates/weapons/src/projectile/rocket.rs:19`), and `launch` returns
  `[(Vec3, Vec3); ROCKET_SHOTS]` (`rocket.rs:89`), so the spawn count is a type, not data.
  Both the player (`crates/raceplay/src/weapons.rs:278`) and the AI
  (`crates/raceplay/src/field/opponent_weapons.rs:69`) call the same `fire_rocket`.
- **The brief's example is not in the XML as stated.** No file says "a speed ship
  fires one rocket, a fighter three". The counts are per craft folder, values 1 or 2.
  Which folder is fighter and which is speed is not written in the files; the
  livery names (`docs/formats/2048-status.md:263-270`) hint at it and nothing confirms it.
  The three-rocket fan is the Pulse law (`docs/gameplay/pickups.md`, "Three at once").
- **Fixing it is a law, not a refactor.** It changes 2048 gameplay (rockets per press)
  and needs the folder-to-class mapping first. Item 8 below.

## Do now (ranked)

### 1. Reset triggers as one per-race struct (sweep item 2)

- **Where.** Constants: `crates/race/src/recovery.rs`: `AIRBORNE_RESET_SECONDS` (197),
  `RESET_SHIELD_CHARGE` (202), `DISTANCE_RESET_UNITS` (214), `DISTANCE_RESET_TICKS` (223),
  plus `BENEATH_LINE` (75), `RESCUE_TICKS` (82), `STALL_SPEED` (168), `STALL_TICKS` (178),
  `RESPAWN_COOLDOWN_TICKS` (25). Triggers: `crates/raceplay/src/respawn.rs`
  `beneath_too_long` (83), `stalled` (135), `reset_zone_touched` (168),
  `airborne_too_long` (199), `far_from_the_sample` (240). Decision chain:
  `crates/raceplay/src/field.rs:970-1002`, one `if alive && (beneath || stalled || airborne || distant || reset || cage)` with a cause ladder.
- **What happens and how often.** Every tick, for every craft: five trigger reads.
  A respawn fires rarely (a few per race at most), so the cost is readability, not speed.
- **The guard is copied five times.** `respawn_disabled[slot]` appears at
  `respawn.rs:93, 158, 169, 200, 254`, and `respawn_cooldown[slot] == 0` at four of them.
  **The airborne check (`respawn.rs:199-204`) omits the cooldown guard**, the other four
  have it. Reachability is low: `put_back` rebuilds the physics state, so `time_airborne`
  restarts at zero, and the cooldown is 30 ticks against a 4 s clock. It is a consistency
  gap, not a live bug.
- **`RESET_SHIELD_CHARGE` (a damage law) lives in `oag-race`**, read only by
  `charge_the_reset` (`respawn.rs` near 320). It belongs with the shield, not the trigger
  timings.
- **The payoff.** One struct per race, one row per trigger, carrying: the constant, the
  enable (`bool`), the provenance (`original` / `chosen` / `measured`, per
  `docs/gameplay/leaving-the-track.md` and ADR-0058 `Title` data), and the
  `RespawnCause` it produces. A lane that needs a trigger off for one title, or a
  mod that disables one, edits one row. Today it edits five guards and a ladder.
- **Size.** M (40). Confidence 40.
- **How to check it is done.** `rg -c "respawn_disabled\[slot\]" crates/raceplay/src`
  drops to one. `systemd-run --user --scope -q -p MemoryMax=8G -p MemorySwapMax=0 cargo nextest run -p oag-raceplay respawn distance_reset` green, and
  `just test-data` for `off_track_rescue_ground_truth` and `stall_rescue_ground_truth`
  unchanged. Read the `state_hash` of a fixed race before and after: it must not move.
- **Risk.** The consts sit in `oag-race` (a determinism crate). Moving them into a
  struct changes no arithmetic and no ordering if every guard keeps its evaluation order.
  Only the airborne cooldown guard changes, and it is unreachable as argued above; a test
  must show that before it is merged. Do not update the determinism reference constants.
  Whether `Title` (ADR-0058) already has a home for per-title trigger data was not checked
  (`crates/title` not read).

### 2. One craft-placement path by spline sample (sweep item 4)

- **Answer to the question.** There is **no** "place a craft at spline distance `s`"
  function. Placement is by sample index, and progress is by index. The shared tail is
  already one function: `put_back` (`crates/raceplay/src/respawn.rs`), reached from
  `respawn` (`:277`, racing-line pose via `Pose::from_sample`) and `reset` (`:345`,
  breadcrumb relocation via `crate::breadcrumbs::relocate`).
- **Where the rest is.**
  - Placement without `put_back`: `crates/trace/src/main.rs:753` and `:926` (`Pose::from_sample` then `ship.place_at`), duplicating the spawn.
  - `Pose::from_sample` `crates/gameplay/src/spawn.rs:59`; `Ship::place_at` `:236`.
  - Five locate or nearest functions: `crates/race/src/course.rs:410` `Course::locate`
    (hinted, route-aware); `crates/gameplay/src/grid_walk.rs:126` `locate` (full scan);
    `crates/raceplay/src/spline.rs:284` `nearest`; `crates/raceplay/src/breadcrumbs.rs:211`
    `locate` (a one-line wrapper of `spline.nearest`); `crates/ai/src/line.rs:257`
    `nearest` (windowed). `spline.rs:384` `distance_to` is the distance version.
- **How often.** Placement: per respawn or reset (rare). Locate: per craft per tick.
- **The payoff.** A mod command "move a craft forward along the spline" needs one path
  that a lap counter and the respawn both use. Today a new caller would pick one of
  six. The `breadcrumbs::locate` wrapper and the trace tool's copy are the cheap cuts.
- **Size.** S (25). Confidence 30.
- **How to check it is done.** `rg -n "Pose::from_sample|place_at\(" crates --type rust | grep -v tests`
  shows one production call site per placement kind. `cargo nextest list -p oag-raceplay`
  count unchanged before and after (a `mod tests;` that never lands reads as green).
  `oag-trace` compare output byte-equal on one capture.
- **Risk.** Merging the locate functions could change which sample wins a tie or which
  window applies: the hinted `Course::locate` and the full-scan `grid_walk::locate` are
  different on purpose (`course.rs:396-408`). Only the wrapper and the duplicated spawn are
  pure moves. The locate merge is a determinism change and must not be bundled.

### 3. One pickup-table lookup, one case rule (sweep item 1 and 3)

- **Where.** `crates/tables/src/weapons.rs:398` `WeaponStats::pickups_for`: exact
  `t.class == class`. `crates/weapons/src/pickup.rs:509` `table_for`: case-insensitive, and
  the only production caller is `crates/raceplay/src/pads.rs:316`. `pickups_for` has **no
  production caller**; `rg` finds it only in `crates/tables/tests/weapons_ground_truth.rs:462,499`
  and `crates/tables/src/weapons/tests.rs:349,367,368`.
- **The payoff.** Two lookups of one table with two rules. A mod pickup adds its own
  weight row; that row must be found by the one lookup that runs. Pure cleanup, XS.
- **Size.** XS (15). Confidence 60.
- **How to check it is done.** `rg -n "pickups_for\(" crates --type rust | grep -v tests`
  is empty after the change; `cargo nextest run -p oag-tables -p oag-weapons` green. Move
  the tests to `table_for` or delete the method. No compatibility re-export (memory:
  unpublished crates, move the call sites).
- **Risk.** None to the simulation (`oag-weapons` and `oag-tables` are not in the
  determinism list, and nothing arithmetic moves).

### 4. One command table for the mod ABI (sweep item 3)

- **Where.** `crates/ext-abi/src/lib.rs`: `Command` enum (`:656`, 24 variants), `encode`
  (`:917`), `encode_pooled` (`:924`), `decode` (`:1012`), `decode_in` (`:1022`); `Event`
  enum (`:489`, 9 variants), `encode` (`:581`), `decode` (`:619`). SDK:
  `crates/ext/src/reply.rs` has 24 `self.push(&Command::...)` sites. Host:
  `crates/modhost/src/reply.rs:38` is an exhaustive `match command` with 25 arms (`:39-158`).
  Its last arm (`:149-158`) is a list of ten variants the host does nothing with.
- **What a new family costs today.** The enum variant, its encode arm, its decode arm,
  its SDK method, its host arm, and the mod-author skill (the memory rule and CLAUDE.md
  both require the skill update for an ABI change). Six places, and the host match is the
  only one the compiler checks for completeness.
- **The payoff.** A table-driven definition (kind code, length, field layout) would
  generate the enum, encode, decode and the no-op list. This is the second enum on the
  same pattern, so the "macro once a second enum uses it" rule in memory
  (`feedback_macro_two_enums`) is met.
- **Size.** L (55). Confidence 25.
- **How to check it is done.** Byte-identical encodings: the golden tests in
  `ext-abi` must pass without edit. `cargo nextest run -p oag-ext-abi -p oag-ext -p oag-modhost`
  green. The mod-author skill is unchanged if no byte changes.
- **Risk.** Nothing ships yet (memory: no version bumps, edit v1 in place), so layout can
  move, but a silent layout change breaks any built mod. Golden bytes are the guard. Not
  simulation code.

### 5. One camera FOV chain (sweep item 6)

- **Where.** The boost-kick widen is written twice, identically:
  `crates/raceplay/src/camera.rs:744` (in `Race::vertical_fov`, `:683`) and
  `crates/raceplay/src/viewpoint.rs:467` (in `extra_vertical_fov`, `:449`):
  `2.0 * ((authored * 0.5).tan() * widen).atan()`. Both then call
  `oag_render::camera::fit_vertical_fov` (`crates/render/src/camera/mod.rs:75`).
  The authored 65 is held in three places: `crates/render/src/camera/craft_view.rs:58`
  `FOV_DEGREES`, `crates/mesh/src/mesh/lod.rs:36` `REFERENCE_FOV_DEGREES`, and the
  handling XML `<InternalCamera fov="65">` read into `self.view.internal_params.fov`.
- **The payoff.** A mod camera FOV curve (lane) needs a third input to the same chain.
  One function `kicked_fov(authored, kick, gain)` removes the duplicated trig and gives the
  curve one place to enter. The 65 constants can point at one source.
- **Size.** S (20). Confidence 30.
- **How to check it is done.** `rg -n "\.atan\(\)" crates/raceplay/src` shows one
  occurrence. A bit-identical frame: `oag-game --screenshot` of a fixed race before and
  after (`--race` skips the front end; use a dev-unlocked profile), `cmp` the PNGs. Headless only.
- **Risk.** Render-side. `oag-raceplay` is not a determinism crate; the FOV feeds no
  simulation state (confirm with the `LodGroups` path, which reads the same field).
  Floating-point order must stay identical, or the screenshots differ by a bit.

## Later

### 6. One "in the world" predicate for a craft (sweep item 5)

- `Craft::active()` (`crates/weapons/src/craft.rs:15`) is the weapons' gate, checked in
  about fifteen places (`projectile/quake.rs:248,280`, `mine.rs:326`, `missile.rs:314`
  (the lock: a craft that is not active cannot be locked), `disruptor.rs:232,247`,
  `blast.rs:120,185,242,252`, `leach_beam.rs:323`, `geometry.rs:66`).
- Raceplay reads the field directly: `crates/raceplay/src/weapons/visuals.rs:42,467`,
  `crates/raceplay/src/wreck_fx.rs:229`. `oag_gameplay::world.rs:510` reads it too.
- **Payoff.** One name for "in play": the draw list, the targeting and the collision
  would all ask it. Today each site asks the flag. Size XS (10). Confidence 40.
- **Check.** `rg -n "\.active\b" crates --type rust | grep -v tests` lists one accessor
  call per site. **Collision:** no craft-versus-craft gate was found by name in this sweep
  (`rg` for `ship_collision`, `craft_vs_craft` and `collide_ships` is empty). Unknown; a
  lane that needs "no collision" must find it first.

### 7. Enable state kept in two places (sweep item 7)

- `Listing.enabled: Option<bool>` (`crates/game/src/mods.rs:72`), built at `:313` from
  `enabled || builtin`, and `settings::Mods::disabled` (a `BTreeSet`, `mods.rs:31`, switched
  by `switch` at `:263`). The read sites use the `Option` (`:167`, `:218`).
- **The manifest has no enable or default field** (`crates/mod/src/manifest.rs`: none found).
  A "default disabled" flag would be a third source.
- **Payoff.** One resolver `is_enabled(manifest_default, settings)`. Size S (20). Confidence 30.
- **Check.** `rg -n "disabled\b|enabled" crates/game/src/mods.rs` shows one read path.

### 8. The rocket count as data (sweep item 1, a law, not a refactor)

- `ROCKET_SHOTS` (`rocket.rs:19`) fixes the fan. The per-craft `number_of_rockets` (1 or 2 on
  2048) is unread. Making `launch` return a count from the craft's stats changes 2048 play.
- **Needs first:** the folder-to-class mapping for `HandlingStats/<team>/<1..4>`, and
  whether Pulse, Pure and HD have the field at all (not checked). Confidence in the law: none yet.
- Size M (45) once the law is known. Not in the lane's scope unless it asks.

### 9. AI firing wrappers (sweep item 8)

- `crates/raceplay/src/field/opponent_weapons.rs`: six `fire_opponent_*` wrappers (`:33`
  missile, `:56` rocket, `:94` quake, `:127` leach beam, `:177` plasma, `:216` shuriken).
  Each reads its `WeaponStats` and calls the shared spawn. The spawn is shared with the
  player (`weapons.rs:278`, `:909`). The AI decision gate is `Driver::holds_fire`
  (`crates/ai/src/driver/weapons.rs:108`), called once at `crates/raceplay/src/weapons.rs:771`.
- **Payoff.** Low. The wrappers are thin; a mod pickup fired by an AI needs one branch at
  `weapons.rs:771` and a decision in `oag-ai`. Size S (20). Confidence 30.
- **Check.** `rg -n "fire_opponent_" crates/raceplay/src` count unchanged after a single
  dispatch on `Weapon`.

### 10. The trace tool's spawn copy (sweep item 4, part)

- `crates/trace/src/main.rs:753` and `:926` duplicate `Pose::from_sample` plus `place_at`.
  See item 2; this is the cheap half of it. Size XS (10). Confidence 40.

## Checked, no standalone refactor

- **Screen flash (sweep item 6).** One owner: `oag_fx::flash::ScreenFlash`, read by
  `Race::screen_wash` (`crates/raceplay/src/effects.rs:1002`). Starters: `craft_flash.rs:157`,
  `weapons/visuals/quake.rs:163`, `weapons/visuals/flash.rs`. Nothing to merge.
- **Overlay pass (sweep item 6).** `crates/game/src/render.rs:617` `overlay` is one
  second pass with `LoadOp::Load`, already shared. `touch_controls.rs:97` `fill` is a
  rectangle helper, not a screen fill.
- **Manifest icons (sweep item 7).** `[icons]` (`crates/mod/src/manifest.rs:84`) is per
  mod, and `[pickup] icon` (`PickupTable`, near `:266`) is the look of the mod's pickup.
  There is **no per-title axis** in the manifest, so "per-title mod icons" is a new field,
  not a cleanup. Lane decision.

## Measurements this sweep did not take

- No `just bench-render` or `oag-game` run. Every item is a reading. Counts are `rg` counts
  on `main` at `038fa0b3a`.
- The `Title` (ADR-0058) home for per-trigger data was not read.
- Craft-versus-craft collision was not located.

## Open

- Whether item 1's airborne cooldown gap is reachable through any path other than the
  one argued above (a test would settle it).
- Which 2048 team folder is fighter and which is speed (item 8).

## Next Steps

1. Lead decides which of items 1-5 a lane takes first. Item 3 (XS) and item 5 (S) are the
   cheapest to land ahead of the lane.
2. Before item 1: `rg -n "AIRBORNE_RESET_SECONDS|RESET_SHIELD_CHARGE" crates --type rust`
   and a test that the cooldown-less airborne path cannot fire within 30 ticks of a respawn.
3. Before item 8: map `HandlingStats/<team>/<1..4>` to a class (read the 2048 livery
   names and the ship select in `docs/formats/2048-status.md`), then decide Pulse's answer.
