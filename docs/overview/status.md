# Status map

A cross-title matrix answering "where do we stand": one axis per subsystem,
titles as columns, every non-`?` cell a status word plus a link to the page,
test or commit that is its evidence. [`overview/roadmap.md`](roadmap.md) stays
the authority on *why* and *what's next*; this page is *what's true right now*,
read across all six titles at once instead of one milestone narrative and four
per-title prose pages.

## Status vocabulary

Fixed here, used everywhere below:

| Word | Meaning |
| --- | --- |
| 📦 `authored` | the disc ships the data (table/asset) for this title; nothing read or built |
| 📖 `read` | reverse-engineered and documented (page + confidence), not implemented |
| 🟩 `built` | implemented in this engine from the read |
| ✅ `verified` | built and checked against the original - ground-truth test, emulator trace, or screenshot diff; the cell names which |
| 🟨 `partial` | built with a named gap (≤6 words) |
| ➖ `n/a` | the title does not have this |
| ⬜ `?` | not investigated - said rather than guessed |

The roadmap table uses the same colours: ✅ `done`, 🟨 `in progress`, ⬜ `not started`. Read a row left to right as a traffic light: ✅ is done done, 🟩 is done but unverified against the original, 🟨 has a named gap, 📖/📦 is not started in code.

A cell claiming `built` or `verified` for something the disc merely authors is
the error this page exists to prevent - see CLAUDE.md's "never invent what the
assets author": `authored` and `read` are both short of a runtime consumer.

## Roadmap at a glance

| Milestone | Exit criterion | Status |
| --- | --- | --- |
| [M0 - Foundation](roadmap.md#m0---foundation) | Both Pulse discs listed and identified; every format has a status-table row | ✅ **done** |
| [M1 - Asset archaeology](roadmap.md#m1---asset-archaeology) | A Pulse track and ship render in `oag-view` from an unmodified disc, both PSP and PS2 paths | ✅ **done** |
| [M2 - Binary understanding](roadmap.md#m2---binary-understanding) | Engine lifecycle and memory map documented; 50+ functions documented | ✅ **done** (2026-09-16) - 928 symbols documented; lifecycle on the memory, resource-loading and state-machine pages; [memory maps](../ghidra/memory-maps/README.md) for both platforms |
| [M3 - Verification harness](roadmap.md#m3---verification-harness) | One command diffs any subsystem against the original and reports where/by how much | ✅ **done** (2026-09-16) - `oag-trace` does this for PPSSPP and, since the PCSX2 capture landed, for the PS2 asset path; craft-level fields only |
| [M4 - Playable core](roadmap.md#m4---playable-core) | A single-ship time trial passes trace comparison for a full lap and feels right | ✅ **done** - ship holds the track for 600 ticks, worst spline distance 27.2/114 units |
| [M5 - Full race](roadmap.md#m5---full-race) | An eight-ship race indistinguishable from the original, per-tick trace in tolerance | 🟨 **in progress** - eight-craft grid built and AI drives; full-race trace parity not yet claimed |
| [M6 - Rendering fidelity](roadmap.md#m6---rendering-fidelity) | A still frame is hard to tell from a PPSSPP frame at the same pose; every departure written down | 🟨 **in progress** - bloom, colour grading, motion blur not yet ported |
| [M7 - Shell and polish](roadmap.md#m7---shell-and-polish) | Pulse is feature complete, start to finish, on both asset paths | 🟨 **in progress** - shell exists and navigates, not yet feature-complete |
| [M8 - Beyond Pulse](roadmap.md#m8---beyond-pulse) | A second title boots and plays on the same engine | 🟨 **in progress** - Pure boots to a Time Trial; HD/Fury and 2048 race and draw textured; Race Remix backend verified; Omega's front end boots and its campaign screens draw, racing out of scope ([omega-status.md](../formats/omega-status.md)) |

## 1. Weapons

Rosters differ by title (disc-measured, not assumed): Pulse ships 13
(`Data\XML\WeaponStats_Race.xml`), Pure ships 10 of those 13 plus its own
Disruptor, and HD/Fury and Omega's `WeaponManager` classes name 9 of Pulse's
13 plus two nobody else has, EMP and Light Barrier. See
[weapon-stats.md](../formats/weapon-stats.md#the-pure-dialect-ten-weapons-one-disruptor-no-fuse-on-the-bomb)
for the full roster diff. 2048's weapon set is not investigated - dropped
below rather than shown all-`?`; `docs/formats/2048-status.md` has no weapons
section yet.

| Weapon | Pulse (PSP/PS2) | Pure (PSP) | HD/Fury (PS3) | Omega (PS4) |
| --- | --- | --- | --- | --- |
| Rocket | ✅ verified - [rocket_floor_trace.rs](../../crates/game/tests/rocket_floor_trace.rs) | 🟩 built - shares Pulse's code, table decoded ([pure_weapons_ground_truth.rs](../../crates/tables/tests/pure_weapons_ground_truth.rs)) | 📖 read - `Rocket_Construct`, `RocketManager_Construct` ([weapons.md](../ghidra/functions/ps3-hdfury-eu/weapons.md)) | 📖 read - same classes ([weapons.md](../ghidra/functions/ps4-omega-eu/weapons.md)) |
| Missile | ✅ verified - [missile_ground_truth.rs](../../crates/game/tests/missile_ground_truth.rs), lock/guidance in [missile.md](../ghidra/functions/psp-pulse-usa/missile.md) | 🟩 built - same code, no Pure-specific behavioural test | 📖 read - `MissileManager_Construct` ([weapons.md](../ghidra/functions/ps3-hdfury-eu/weapons.md)) | 📖 read - same ([weapons.md](../ghidra/functions/ps4-omega-eu/weapons.md)) |
| Quake | 🟨 partial - fires, travels and blasts ([quake.rs](../../crates/gameplay/src/projectile/quake.rs)); the wave's own track deformation/vertex animation is not built and where the original produces it is unresolved ([cannon-quake-leachbeam.md](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md)); no dedicated ground-truth test | 🟨 partial - same code, same gap; Pure's wave also authors the grey `bobs` emitter nothing here plays | 📖 read - `QuakeManager_Construct` | 📖 read - same |
| Cannon | ✅ verified - [cannon_ground_truth.rs](../../crates/game/tests/cannon_ground_truth.rs), fire path in [cannon-quake-leachbeam.md](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md) | ➖ n/a - not in Pure's roster | 📖 read - `CannonManager_Construct` | 📖 read - same |
| Turbo | 🟩 built - [pickups.md#what-a-fired-turbo-does](../gameplay/pickups.md#what-a-fired-turbo-does) | 🟩 built - same code | ⬜ `?` - no manager class located yet (non-projectile like Pulse) | ⬜ `?` |
| Shield | 🟩 built - [pickups.md#what-a-fired-shield-does](../gameplay/pickups.md#what-a-fired-shield-does) | 🟩 built - same code | 📖 read - `ShipShield_Construct`/`Activate`/`Update`/`Hit`, same law as Pulse's, two colours moved ([shield.md](../ghidra/functions/ps3-hdfury-eu/shield.md)) | ⬜ `?` |
| Autopilot | 🟩 built - AI takeover, `Ai_Construct` per [pickups.md](../gameplay/pickups.md) | 🟩 built - same code | ⬜ `?` | ⬜ `?` |
| Plasma | ✅ verified - [plasma_ground_truth.rs](../../crates/game/tests/plasma_ground_truth.rs); direct-hit blast and the launch-speed ramp both built 2026-09-16, see [pickups.md](../gameplay/pickups.md) rows on `Plasma_HitCraft` and `Plasma_SpeedForClass` | 🟩 built - Pure hardcodes the same 1s charge, see [weapons.md](../ghidra/functions/psp-pure-usa/weapons.md) | 📖 read - `PlasmaManager_Construct`; blast mechanics read in [plasma.md](../ghidra/functions/ps3-hdfury-eu/plasma.md) | 📖 read - [plasma.md](../ghidra/functions/ps4-omega-eu/plasma.md) |
| Bomb | ✅ verified - shares [mine.rs](../../crates/gameplay/src/projectile/mine.rs), tested in [mine_ground_truth.rs](../../crates/game/tests/mine_ground_truth.rs); gap: `damageradius` unread, no draw | 🟩 built - fuse-less dialect, same code | 📖 read - `BombManager_Construct` | 📖 read - same |
| Mine | ✅ verified - [mine_ground_truth.rs](../../crates/game/tests/mine_ground_truth.rs); gap: no draw ([pickups.md](../gameplay/pickups.md)) | 🟩 built - same code | 📖 read - `MineManager_Construct` | 📖 read - same |
| LeachBeam | 🟩 built - drain/repair, target-lock and the `slowShipFactor` throttle ([pickups.md](../gameplay/pickups.md), dated passages 2026-09-08 and 2026-09-16); no disc-backed ground-truth test yet | ➖ n/a | 📖 read - `LeachBeamManager_Construct` | 📖 read - shares one composite function with Mine, not yet decompiled ([weapons.md](../ghidra/functions/ps4-omega-eu/weapons.md)) |
| Repulser | 📖 read - mechanism (a field on the firing craft) identified, no craft-state field to attach it to; deferred Eliminator-only ([pickups.md](../gameplay/pickups.md#shuriken-and-repulser-are-gated-by-mode-not-by-the-pool)) | ➖ n/a | 📖 read - `Repulser_Construct` | 📖 read - not named this pass |
| Shuriken | ✅ verified - [shuriken_ground_truth.rs](../../crates/game/tests/shuriken_ground_truth.rs), Eliminator-only per authored odds | ➖ n/a | ⬜ `?` | ⬜ `?` |
| Disruptor | ➖ n/a | 🟩 built - [disruptor.rs](../../crates/gameplay/src/projectile/disruptor.rs), table decoded in [pure_weapons_ground_truth.rs](../../crates/tables/tests/pure_weapons_ground_truth.rs) | ➖ n/a | ➖ n/a |
| EMP | ➖ n/a | ➖ n/a | 📖 read - named on Omega, not yet located on HD's own binary | 📖 read - `EMPManager_Construct` ([weapons.md](../ghidra/functions/ps4-omega-eu/weapons.md)) |
| Light Barrier | ➖ n/a | ➖ n/a | ⬜ `?` - not located on HD's own binary | 📖 read - `LightBarrierManager_Construct` ([weapons.md](../ghidra/functions/ps4-omega-eu/weapons.md)) |

### Pulse (PSP) weapon pieces

The only title with enough recovered granularity to break a weapon into its
parts. `n/a` in Visuals/Audio means the disc's own effect exists but nothing
here plays it yet, per CLAUDE.md's "draw nothing and say so" rule.

| Weapon | Fire/trigger | Flight | Ending/blast | Visuals | Audio | AI use |
| --- | --- | --- | --- | --- | --- | --- |
| Rocket | 🟩 built | ✅ verified - shares [flight.rs](../../crates/gameplay/src/projectile/flight.rs) | 🟩 built - fanned `spread`, see [pickups.md](../gameplay/pickups.md#what-a-fired-rocket-does) | 🟨 partial - placeholder billboard, real `Ship Muzzle`/`cannon_flash` effects not built ([pickups.md](../gameplay/pickups.md#what-is-not-built)) | ⬜ `?` | 📖 read - [weapon-ai.md](../ghidra/functions/psp-pulse-usa/weapon-ai.md) |
| Missile | 🟩 built - lock via `Ship_AcquireLock`; the reticle it drives is 🟨 partial, see the front-end table | ✅ verified - [missile.md](../ghidra/functions/psp-pulse-usa/missile.md) | 🟩 built | ⬜ `?` | ⬜ `?` | 📖 read - [weapon-ai.md](../ghidra/functions/psp-pulse-usa/weapon-ai.md) |
| Quake | 🟩 built - single travelling instance | 🟩 built - fixed 270 units/s, unauthored ([pickups.md](../gameplay/pickups.md)) | 🟩 built - authored `radius` | 🟨 partial - the disc's own `WO_QUAKE` psys plays, axis-aligned (orientation not established); the wave's track deformation/vertex animation is not built | ⬜ `?` | ⬜ `?` |
| Cannon | 🟩 built - reload gated on held fire button ([cannon-quake-leachbeam.md](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md)) | 🟩 built - flat 500 km/h, no per-class speed authored | 🟨 partial - direct damage only, no splash (schema authors none) | ⬜ `?` | ⬜ `?` | ⬜ `?` |
| Turbo | 🟩 built | ➖ n/a | ➖ n/a | ⬜ `?` | ⬜ `?` | ⬜ `?` |
| Shield | 🟩 built | ➖ n/a | ➖ n/a | 🟨 partial - HD's own hit/activation colours ported behind `oag_render::shield::Palette`, selected by title; the steady-state colour is still Pulse's white, chosen not measured ([shield.md](../ghidra/functions/ps3-hdfury-eu/shield.md)) | ⬜ `?` | ⬜ `?` |
| Autopilot | 🟩 built - `Ai_Construct` names the input source | ➖ n/a | ➖ n/a | ➖ n/a | ⬜ `?` | 🟩 built (it *is* the AI) |
| Plasma | 🟩 built - 1s wind-up, `Plasma_Init`/`Plasmas_Update` | ✅ verified - shares Rocket's floor-follower, own launch-speed ramp to the class speed over 1s, read fresh at release ([plasma.md](../ghidra/functions/psp-pulse-usa/plasma.md#plasma_speedforclass-0x0885c5a4-and-the-launch-ramp)) | ✅ verified - three distinct endings (craft/wall/timeout), see [pickups.md](../gameplay/pickups.md) | 🟩 built - charge glow grows over the wind-up | 🟩 built - `PLASMA`/`~PLASMATVL`/`PLASMAHITWALL`/`PLASMAHITSHIP`, see [cue.rs](../../crates/game/src/audio/sfx/cue.rs) | 🟩 built - `Race::fire_opponent_plasma` ([opponent_weapons.rs](../../crates/game/src/race/field/opponent_weapons.rs)), same `Driver::wants_to_fire` gate as the Rocket - chosen, not measured |
| Bomb | 🟩 built - shares Mine's rear anchor | ✅ verified - [mine_ground_truth.rs](../../crates/game/tests/mine_ground_truth.rs) | 🟨 partial - `damageradius` unread, no consumer | ➖ n/a - `Pulse_Bomb.vex` located, nothing draws it | ⬜ `?` | ⬜ `?` |
| Mine | 🟩 built - laid one per 0.1s | ✅ verified - [mine.md](../ghidra/functions/psp-pulse-usa/mine.md) | 🟩 built | ➖ n/a - `Pulse_Mine.vex` located, nothing draws it | ⬜ `?` | ⬜ `?` |
| LeachBeam | 🟩 built - link to Missile's own lock; its reticle is 🟨 partial, see the front-end table | 🟩 built - whole-race pool cursor | 🟩 built - drain/repair spent, `slowShipFactor` throttles the victim through `Environment::thrust_scale` | ⬜ `?` | ⬜ `?` | ⬜ `?` |
| Repulser | 📖 read only | ➖ n/a | ➖ n/a | ➖ n/a | ➖ n/a | ➖ n/a |
| Shuriken | 🟩 built - ±20 degree throw | ✅ verified - shares Rocket/Plasma's floor follower, bounces off walls | 🟩 built - authored `fuse` | ⬜ `?` | ⬜ `?` | ⬜ `?` |

## 2. Race modes and rules

| Mode/system | Pulse (PSP/PS2) | Pure (PSP) | HD/Fury (PS3) |
| --- | --- | --- | --- |
| Time trial | ✅ verified - per-class lap counts confirmed live ([race-modes.md](../gameplay/race-modes.md#time-trial)) | ✅ verified - `just play --source pure-psp-eu.chd` reaches a Time Trial ([roadmap.md](roadmap.md#m8---beyond-pulse)) | ⬜ `?` |
| Speed lap | 🟩 built - [race-modes.md](../gameplay/race-modes.md#speed-lap) | ⬜ `?` | ⬜ `?` |
| Zone | 🟨 partial - no self-ending yet, numbers not in this repo ([race-modes.md](../gameplay/race-modes.md#zone)) | ⬜ `?` | 🟨 partial - colour grade escalation read for both HD-lineage titles ([race-modes.md](../gameplay/race-modes.md#the-colour-grade-escalates-too-on-the-two-hd-lineage-titles)) |
| Single race | ✅ verified - eight-craft grid within 2.40 units of the original ([race-modes.md](../gameplay/race-modes.md#single-race), [race_ground_truth.rs](../../crates/game/tests/race_ground_truth.rs)) | ⬜ `?` | ⬜ `?` |
| Eliminator | 🟩 built - [race-modes.md](../gameplay/race-modes.md#eliminator), kill attribution chosen not measured | ➖ n/a | ⬜ `?` |
| Head-to-head | 📖 read, not implemented - `race_mode_for_cell` refuses it ([race-modes.md](../gameplay/race-modes.md#tournament)) | ⬜ `?` | ⬜ `?` |
| Tournament | 📖 read, not implemented - full scoring law recovered ([tournament.md](../ghidra/functions/psp-pulse-usa/tournament.md)) | ⬜ `?` | ⬜ `?` |
| Campaign grids/medals | 🟩 built - `evaluate_medal` on `Cell` ([race-campaign.md](../formats/race-campaign.md#the-medal-law-implemented-on-cell-evaluate_medal)) | ⬜ `?` | 📖 read - same schema, extended, split across four archives ([race-campaign.md](../formats/race-campaign.md#wipeout-hd-and-fury-the-same-schema-extended-split-across-four-archives)) |
| Unlocks | 📖 read - `Unlock_GridPointsMet` and `Grid0..Grid10` gating named, not wired ([race-campaign.md](../formats/race-campaign.md), [race-modes.md](../gameplay/race-modes.md)) | ⬜ `?` | ⬜ `?` |
| Records (best lap/time) | 🟩 built - `records.toml`/`records.rs`, per (title, track, mode, class) ([persistence.md](../architecture/persistence.md)) | 🟩 built - same mechanism, title-keyed | ➖ n/a |
| Ghosts | 🟩 built - best-lap ghost in Time Trial and Speed Lap, recorded as a replay ([ADR-0055](../architecture/adr/0055-replays-are-inputs-and-a-ghost-is-poses.md)); drawn with `MeshNode_Ghost`'s three passes and proximity law, read statically ([ghost.md](../ghidra/functions/psp-pulse-usa/ghost.md)), not yet compared against a capture | 🟩 built - same mechanism; Pulse's look, chosen not measured (Pure's ghost not read) | 🟩 built - same mechanism, Pulse's look chosen not measured; replay reproduces on HD and 2048 ([replay_ground_truth.rs](../../crates/game/tests/replay_ground_truth.rs)) |

## 3. Front end

| Element | Pulse (PSP) | Pulse (PS2) | Pure (PSP) | HD/Fury (PS3) | 2048 (Vita) |
| --- | --- | --- | --- | --- | --- |
| Boot movies | 🟩 built - [frontend-boot.md](../architecture/frontend-boot.md) | 🟩 built - same chain | ✅ verified - 5-screen boot chain, [pure-boot.md](../architecture/pure-boot.md) | ✅ verified - measured boot chain, 3 cold RPCS3 boots ([hd-frontend.md](../formats/hd-frontend.md)) | 🟨 partial - `front_end` is `Some` and the 5-screen chain is wired as data (`oag_2048::frontend::BOOT_PROFILE`, `Provenance::Declared`, [ADR-0054](../architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md)); not yet playable - `oag-game`'s menu-driven boot still refuses this title (no `MenuSkin`-shaped menu to draw after it) and the one movie (`intro.mp4`) is an MP4 container `oag-video` does not decode yet ([2048-frontend.md](../formats/2048-frontend.md)) |
| Menus | 🟩 built - page tree, Race/Options/Display/Graphics/Controls | 🟩 built - own widget grid, scaled from PSP ([frontend-boot.md](../architecture/frontend-boot.md#the-ps2-places-its-widgets-in-a-different-grid)) | ⬜ `?` | 🟨 partial - gate-tested, not seen on a real display ([roadmap.md](roadmap.md#m8---beyond-pulse)) | 📖 read - a touch-icon grid over a 3D campaign map, not `MenuSkin`'s list/strip idiom at all; wired as data since [ADR-0054](../architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md) (`oag_title::touch::TouchFrontEnd`, `oag_2048::frontend::TOUCH`), no touch-icon renderer built yet ([2048-frontend.md](../formats/2048-frontend.md#the-front-end-is-a-touch-icon-grid-not-feglobalsmenuskin)) |
| Language selection | 🟩 built - [frontend-boot.md](../architecture/frontend-boot.md#the-language-selection-screen) | ⬜ `?` | ⬜ `?` | ⬜ `?` | ➖ n/a - no picker screen found; 17 language plugins read instead ([2048-frontend.md](../formats/2048-frontend.md)) |
| Ship/track select | 🟩 built - [selection-screens.md](../ui/selection-screens.md) | ⬜ `?` | ⬜ `?` | ⬜ `?` | ➖ n/a |
| Loading screens | 🟩 built - Pulse's own reading | ⬜ `?` | ⬜ `?` | 📖 read - [hd-loading.md](../formats/hd-loading.md) | 🟨 partial - a percentage bar seen on Vita3K, no plugin XML located for it ([2048-frontend.md](../formats/2048-frontend.md)) |
| Results/end-race | 🟩 built - [endrace-screens.md](../ui/endrace-screens.md) | ⬜ `?` | ⬜ `?` | 📖 read - [endrace-screens.md](../formats/endrace-screens.md) | ➖ n/a |
| HUD | ✅ verified - screen-space reference-frame comparison caught a font bug ([hud.md](../ui/hud.md#what-a-reference-frame-settled)) | ⬜ `?` | ⬜ `?` | 📖 read - all 18 in-race layouts compose, 2,320 widgets ([hd-hud.md](../formats/hd-hud.md)) | 🟨 partial - five always-on sprites draw, the shield fill and the held pickup slot (UV table read off `eboot.elf`) are both wired and checked against live frames, speed fill and the pickup grant announcement still are not; `ALWAYS_ON` and the 960x544 space measured off Vita3K frames ([2048-hud.md](../formats/2048-hud.md#energybar-is-wired-a-vertical-crop-from-the-bottom-2026-09-20)) |
| Lock-on reticle (Missile, LeachBeam) | 🟩 built - placement, `0.8` s hold, chase, `~ROCKLOCK` tone and the recovered hue (red locked, yellow/white seeking) built ([lock-sight.md](../ghidra/functions/psp-pulse-usa/lock-sight.md)); the far-target alpha step reads out as a live depth-buffer occlusion sample rather than a tuning constant, and is structurally unportable into a gameplay crate - documented, not built | ⬜ `?` | 🟩 built - same law, Pure's own widgets, inherits the hue fix | 🟨 partial - own per-weapon sight updates and a `0.5` s hold read ([hud-sight.md](../ghidra/functions/ps3-hdfury-eu/hud-sight.md)), engine keeps the PSP's `0.8`; the LeachBeam's four widgets now reveal one at a time as the hold progresses, ported at reduced confidence onto this engine's own hold constant rather than HD's absolute quarter-second marks | 🟨 partial - HD's reading, same ported reveal, 2048 never independently disassembled |

## 4. Rendering

| Element | Pulse (PSP/PS2) | HD/Fury (PS3) | 2048 (Vita) |
| --- | --- | --- | --- |
| Track geometry | ✅ verified - PS2 vs PSP radius within 0.1 ([roadmap.md](roadmap.md#m1---asset-archaeology)) | 🟨 partial - LOD, scenery animation read ([README.md](../rendering/README.md)) | 🟨 partial - 413k triangles draw, PVS not this project's HD layout ([2048-status.md](../formats/2048-status.md)) |
| Ships | ✅ verified - visual parity both asset paths | 🟩 built - hull materials, [hd-ship-materials.md](../rendering/hd-ship-materials.md) | 🟨 partial - 2,782/2,800 draws, 521/527 materials ([2048-status.md](../formats/2048-status.md)) |
| Skybox | 🟩 built - [skycube_ground_truth.rs](../../crates/vex/tests/skycube_ground_truth.rs) | ⬜ `?` | ⬜ `?` |
| Lighting | 🟩 built - authored normals | 🟨 partial - the `.envsettings` sun/ambient/prelit rig drawn ([envsettings.md](../formats/envsettings.md)); the SPU vertex-light sum built per vertex with one producer wired, the per-craft engine light ([README.md](../rendering/README.md), "SPU vertex lights") | 🟨 partial - stand-in rig, `.envsettings` does not parse for this title ([2048-status.md](../formats/2048-status.md)) |
| Shadows | 🟩 built - [shadows.md](../rendering/shadows.md) | ⬜ `?` | ⬜ `?` |
| PVS/culling | ✅ verified - [pvs_placement_ground_truth.rs](../../crates/render/tests/pvs_placement_ground_truth.rs) | 📖 read - [README.md](../rendering/README.md) | ➖ n/a - falls back to drawing every chunk ([2048-status.md](../formats/2048-status.md)) |
| Fog | 🟩 built - [fog.md](../ghidra/functions/psp-pulse-usa/fog.md) | ⬜ `?` | ⬜ `?` |
| Particles | 🟨 partial - `oag_render::psys::Library`/`Stage` generic, some triggers unwired (see `HANDOVER.md`) | ⬜ `?` | ⬜ `?` |
| Engine trails | 🟩 built - [trail-ribbon.md](../rendering/trail-ribbon.md) | ⬜ `?` | ⬜ `?` |
| Weapon models | 🟨 partial - Mine/Bomb located, not drawn ([pickups.md](../gameplay/pickups.md#what-is-not-built)) | ⬜ `?` | ⬜ `?` |
| Animated textures | 🟩 built - [scenery-animation.md](../rendering/scenery-animation.md) | 📖 read - [README.md](../rendering/README.md) | ⬜ `?` |
| Animated scenery (nodes) | 🟩 built - [scenery-animation.md](../rendering/scenery-animation.md) | 🟩 built - `Anim Transform`, [README.md](../rendering/README.md) | 🟩 built - `.rcsskeleton`/`.rcsanimclip` rig, checked node by node against HD's authored `Anim Transform`s on the twelve shared circuits ([2048-animation.md](../formats/2048-animation.md), [psp2_scenery_animation_ground_truth.rs](../../crates/render/tests/psp2_scenery_animation_ground_truth.rs)); Zone's and the start grid's clips not wired |
| Bloom/grading/motion blur | 📖 read only - not ported ([roadmap.md](roadmap.md#m6---rendering-fidelity)) | ⬜ `?` | ⬜ `?` |
| Upscaling | 🟩 built - FSR3 wired ([fsr3.md](../rendering/fsr3.md), [dynamic-resolution.md](../rendering/dynamic-resolution.md)) | ➖ n/a | ➖ n/a |

## 5. Physics and AI

Engine-generic (`oag-physics`/`oag-ai` know nothing of a title), so a `built`
row applies to whichever title's data is loaded; the verification link is
whichever title actually captured a reference trace, which today is Pulse PSP
only.

| System | Status | Evidence |
| --- | --- | --- |
| Hover/suspension | ✅ verified | [force-balance-ground-truth.md](../physics/force-balance-ground-truth.md) |
| Steering/pitch | ✅ verified | [engine.md](../ghidra/functions/psp-pulse-usa/engine.md), [cornering-ground-truth.md](../physics/cornering-ground-truth.md) |
| Airbrakes | ✅ verified | [airbrake_flaps_ground_truth.rs](../../crates/render/tests/airbrake_flaps_ground_truth.rs) |
| Sideshift | ⬜ `?` | not investigated |
| Barrel roll | ✅ verified | [ai_roll_ground_truth.rs](../../crates/game/tests/ai_roll_ground_truth.rs) |
| Wall/collision response | ✅ verified | [wall_collision_ground_truth.rs](../../crates/game/tests/wall_collision_ground_truth.rs), `crates/physics/src/wall.rs` |
| Respawn/stall rescue | ✅ verified | [stall_rescue_ground_truth.rs](../../crates/game/tests/stall_rescue_ground_truth.rs), [off_track_rescue_ground_truth.rs](../../crates/game/tests/off_track_rescue_ground_truth.rs) |
| Weapon slowdown | 🟩 built - every blast, the Cannon and the Quake credit `slowdown_time` and `oag_gameplay::slowdown::drain` spends it through `Ship_AddSlowdown`'s port; the LeachBeam's `slowShipFactor` throttle is the separate one-shot mechanic | [engine.md](../ghidra/functions/psp-pulse-usa/engine.md), `crates/physics/src/slowdown.rs` |
| AI driving line | 🟩 built | [ai.md](../gameplay/ai.md) |
| AI weapon use | 📖 read | [weapon-ai.md](../ghidra/functions/psp-pulse-usa/weapon-ai.md) |

## 6. Audio

| System | Pulse (PSP) | Pulse (PS2) | HD/Fury (PS3) | 2048 (Vita) |
| --- | --- | --- | --- | --- |
| Engine sound | 🟩 built - [psp-audio.md](../formats/psp-audio.md) | 🟩 built - [ps2-audio.md](../formats/ps2-audio.md) | ⬜ `?` | ⬜ `?` |
| Music | ✅ verified - `.bnk` decoder, PS-ADPCM | ✅ verified - 48kHz PCM cross-validated against PSP masters | 📖 read - exercised by [hd_music_ground_truth.rs](../../crates/game/tests/hd_music_ground_truth.rs) | ➖ n/a - `music: None`, not read ([2048-status.md](../formats/2048-status.md)) |
| SFX cues | 🟩 built - [sfx.rs](../../crates/game/src/audio/sfx.rs), [sfx_ground_truth.rs](../../crates/game/tests/sfx_ground_truth.rs) | ⬜ `?` | ⬜ `?` | ⬜ `?` |
| Front-end sounds | 🟩 built | ⬜ `?` | ⬜ `?` | ➖ n/a |
| Zone announcer | ➖ n/a | ➖ n/a | ⬜ `?` | 📖 read - dispatch decompiled, not verified against real audio, extracted data not in this tree ([2048-status.md](../formats/2048-status.md#what-a-race-does-today)) |

## 7. RE coverage per binary

**Generated, not maintained.** `just gen-status` rewrites the table below
from every `docs/ghidra/functions/<binary>/names.tsv` and the evidence pages
beside it (`scripts/gen-re-coverage.py`), and `just check-status` - part of
the default gate - fails when a row lands without the table moving. Edit the
script, never the table. "Below 70" is the `_q` share: names the rubric still
calls hypotheses. `psp-pure-eu` reads deeper than `psp-pure-usa` only on
paper: most of Pure's evidence is established once on USA and its addresses
transferred to EU, so EU's count includes names that add no page.

<!-- re-coverage:begin - generated by scripts/gen-re-coverage.py, do not edit by hand -->

| Binary | names.tsv rows | below 70 (`_q`) | Evidence pages | Emulator harness | Newest dated page |
| --- | ---: | ---: | ---: | --- | --- |
| [ps2-pulse-eu](../ghidra/functions/ps2-pulse-eu/) | 185 (159 fn, 26 data) | 1 (0%) | 16 | `pcsx2-drive.py`, `pcsx2-trace.py`, `pcsx2_pine.py`, `pcsx2_trace_fields.py` | 2026-09-16, [craft-update.md](../ghidra/functions/ps2-pulse-eu/craft-update.md) |
| [ps3-hdfury-eu](../ghidra/functions/ps3-hdfury-eu/) | 401 (350 fn, 51 data) | 13 (3%) | 38 | `rpcs3-drive.py`, `rpcs3-spu-job-binary-dump.py`, `rpcs3-spu-light-dump.py`, `rpcs3-trail-dump.py`, `rpcs3_debugger.py` (+3 measurement scripts) | 2026-09-23, [absorb-feedback.md](../ghidra/functions/ps3-hdfury-eu/absorb-feedback.md) |
| [ps4-omega-eu](../ghidra/functions/ps4-omega-eu/) | 51 (51 fn, 0 data) | 0 (0%) | 9 | none | 2026-09-16, [plasma.md](../ghidra/functions/ps4-omega-eu/plasma.md) |
| [psp-pulse-eu](../ghidra/functions/psp-pulse-eu/) | 430 (413 fn, 17 data) | 74 (17%) | 4 | `ppsspp_debugger.py`, `psp-drive.py`, `psp-trace.py`, `psp_trace_fields.py` (+15 measurement scripts) | 2026-09-23, [lighting.md](../ghidra/functions/psp-pulse-eu/lighting.md) |
| [psp-pulse-usa](../ghidra/functions/psp-pulse-usa/) | 1047 (928 fn, 119 data) | 25 (2%) | 61 | `ppsspp_debugger.py`, `psp-drive.py`, `psp-trace.py`, `psp_trace_fields.py` (+15 measurement scripts) | 2026-09-23, [bloom.md](../ghidra/functions/psp-pulse-usa/bloom.md) |
| [psp-pure-eu](../ghidra/functions/psp-pure-eu/) | 67 (66 fn, 1 data) | 3 (4%) | 8 | `ppsspp_debugger.py`, `psp-drive.py`, `psp-trace.py`, `psp_trace_fields.py` (+15 measurement scripts) | 2026-09-15, [dlc-download-check.md](../ghidra/functions/psp-pure-eu/dlc-download-check.md) |
| [psp-pure-usa](../ghidra/functions/psp-pure-usa/) | 71 (70 fn, 1 data) | 2 (2%) | 9 | `ppsspp_debugger.py`, `psp-drive.py`, `psp-trace.py`, `psp_trace_fields.py` (+15 measurement scripts) | 2026-09-23, [exhaust-sound.md](../ghidra/functions/psp-pure-usa/exhaust-sound.md) |
| [vita-2048-eu-v104](../ghidra/functions/vita-2048-eu-v104/) | 52 (44 fn, 8 data) | 0 (0%) | 9 | `vita-gxp.py`, `vita-self-decrypt.py` | 2026-09-21, [frontend-campaign-map.md](../ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md) |

<!-- re-coverage:end -->

## 8. Formats

Not duplicated here - [`docs/formats/README.md`](../formats/README.md) is the
authority, with one row per format crossed against platform and title, plus a
byte-coverage table showing what fraction of each container a parser actually
reads (the discipline that caught HD losing 40% of its render geometry to an
unread field). See that page directly rather than a stale copy of its totals.

## How to keep this current

A row changes in the same commit that changes the thing it describes - the
same rule [`docs/formats/README.md`](../formats/README.md) already follows.
`just check-docs` validates every link on this page, including anchors, so a
renamed heading or moved test file fails the gate rather than rotting
silently; run it after any edit here. Prefer linking a test or a commit over a
prose claim - a roadmap sentence that doesn't name either is not evidence
enough for a cell here, only for the roadmap's own narrative. Section 7 is
the exception: it is generated, so after a `names.tsv` or evidence-page
change run `just gen-status` and commit the result - `just check-status`
fails the gate otherwise.
