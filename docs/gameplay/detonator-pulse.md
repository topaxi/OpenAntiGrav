# Detonator on Wipeout Pulse

Wipeout Pulse ships no Detonator. This engine races one on Pulse (PSP and PS2) at the
maintainer's request: "backport detonator to Pulse, shall use zone races for tracks, normal
mines for mines, speed and weapon pads replaced by red weapon pads". The mode's law is HD's
(`docs/ghidra/functions/ps3-hdfury-eu/detonator-*.md`); everything shown is Pulse's own disc
data. Nothing on this page is measured on Pulse: every value is **chosen, not measured**,
because the original has no such mode.

Try it: `just play <pulse disc> --race --mode detonator`, or RACEBOX, MODE `DETONATOR` (the
CIRCUIT row then lists the Zone circuits only).

## What a race has

| Part | On Pulse | Where |
| --- | --- | --- |
| Circuit | the picked circuit's Zone twin (`zone_track.vex` beside `track.vex`), as a Zone race flies | `oag_title::Detonator::races_zone_circuit` |
| Mines and bombs | Pulse's own `Pulse_Mine.vex` and `Pulse_Bomb.vex`, laid by HD's placement law; contact radius, damage and slowdown are Pulse's race table's `Mine` and `Bomb` | `objects: None` (the ordinary mine and bomb pools draw them) |
| Layout | this project's six pattern groups and fourteen stage counts (30 mines at stage 1 to 108 at 14) | `assets/fallback/Data/XML/DetonatorLayouts_Pulse/` |
| Pads | every speed pad (a Zone circuit authors no weapon pad: 0 on all eight PSP `zone_track.vex`, against 7 to 15 on the race files) is drawn as the race circuit's own first `Weapon Pad`, held at the red key of its ready cycle (`READY_KEYFRAMES[0]`, 204, 102, 102; `docs/rendering/pads.md`); the circuit's own speed pads are not drawn | `DetonatorPads::circuit_weapon_pad_key` |
| Pad law | a pad gives no boost and charges the EMP once per entry, HD's arm | `DetonatorPads` `speed_pad_charges`, `weapon_pad_charges` |
| EMP | HD's charge, gauge, L1 fire and wave on this project's `EMP` block: ready at 3 pads, full at 6, extent 1 to 3 s, 3 survey entries a tick; drawn as Pulse's `WO_QUAKE` on the front, no light | `weapons_supplement`, `oag_pulse::effects` |
| Cannon | HD's cannon on Pulse's own `Cannon` (`rounds`, `rate`); one wall rebound, three hits to a bomb; the rounds leave from the hull's dimensions (Pulse's hulls author no `cannon_flash`); **the rounds and muzzle flash are not drawn** (Pulse has no bolt model); a wall hit sparks Pulse's `WO_CANNON_SPARKS` | `weapons_supplement` |
| Explosions | a mine and a bomb go up as Pulse's `WO_MINE_EXPLO` | `oag_pulse::effects` |
| Mine slowdown | `velocity_reduction` 0.5, as the supplement names it | `weapons_supplement` |
| Scoring | this project's `<Bullet>` and `<Emp>` values (10 a live mine, 5 a dark one, 5 an EMP'd live mine) | `DetonatorScoring.xml` in the same directory |
| Not drawn | the stage grade (Pulse ships none), the EMP gauge and score on the HUD (Pulse's Detonator HUD is the Time Trial layout), the medals, the announcer lines | `palette: None`, `binds_emp_ready`, `shows_medals`, `voices: None` |
| Craft | the team's own race hull and handling | `craft: None`, `handling: None` |

## Why the data lives in the fallback layer

The layout files and the Detonator-only weapon blocks are read by the same parsers HD's are
(`oag_tables::detonator_layout`, `oag_tables::weapons::detonator_supplement`), out of
`assets/fallback/` ([ADR-0061](../architecture/adr/0061-two-asset-layers-around-the-disc.md)),
embedded by `oag_source::layers::fallback_layer`. A player's override layer can replace any of
the four files. Pulse's own race table stands under the supplement: its `Mine`, `Bomb` and
`Cannon` are the race's, and the supplement adds only what Detonator reads beyond them.

## PS2

The same `Title`, so the same data; a PS2 race loads, flies `zone_track.vex` and draws the red
pads with the PS2 disc's own pad texture (26 speed pads on `16_Track`'s Zone twin).

## HD, 2048 and Omega

Unchanged: the three new `Title` fields are `false`/`None` there, and their Detonator tests in
`crates/raceplay/tests/detonator_ground_truth.rs` stand.

## Evidence

`crates/raceplay/tests/detonator_ground_truth.rs`,
`a_pulse_detonator_race_lays_pulse_mines_on_a_zone_circuit_with_red_charging_pads` (PSP USA):
the Zone twin is flown, the layout and supplement are read, the 17 speed pads of
`16_Track`'s Zone twin carry the weapon pad in red, the first tick lays 15 to 30 mines, three
pads ready the EMP with no boost, L1 clears the mines ahead for 5 each, and Square spends the
clip.
