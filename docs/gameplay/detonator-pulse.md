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
| EMP | HD's charge, gauge, L1 fire and wave on this project's `EMP` block: ready at 3 pads, full at 6, extent 1 to 3 s, 3 survey entries a tick; drawn as the Repulsor's (see [The EMP](#the-emp)), no light | `weapons_supplement`, `oag_pulse::effects`, `emp_borrows_repulsor` |
| Cannon | HD's cannon on Pulse's own `Cannon` (`rounds`, `rate`); one wall rebound, three hits to a bomb; the rounds leave from the hull's dimensions (Pulse's hulls author no `cannon_flash`); each round is drawn as Pulse's own Cannon's (see [The cannon](#the-cannon)); a wall hit sparks Pulse's `WO_CANNON_SPARKS` | `weapons_supplement` |
| Explosions | a mine and a bomb go up as Pulse's `WO_MINE_EXPLO` | `oag_pulse::effects` |
| Mine slowdown | `velocity_reduction` 0.5, as the supplement names it | `weapons_supplement` |
| Scoring | this project's `<Bullet>` and `<Emp>` values (10 a live mine, 5 a dark one, 5 an EMP'd live mine) | `DetonatorScoring.xml` in the same directory |
| HUD | the disc's Time Trial layout plus the EMP gauge (see [The gauge](#the-gauge)) | `oag_pulse::hud::layouts::DETONATOR`, `ps2_hud` |
| Not drawn | the stage grade (Pulse ships none), the score on the HUD, the medals, the announcer lines | `palette: None`, `shows_medals`, `voices: None` |
| Craft | the team's own race hull and handling | `craft: None`, `handling: None` |

## The cannon

Pulse ships no bolt model, so each Detonator round draws as a fired Cannon pickup's round
does on Pulse (`pulse_muzzleflash.vex` posed on the round, `Cannon_bolt.mip`'s streak, and
the `cannon muzzle flash` quad for the first 0.1 s,
[cannon-quake-leachbeam.md](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md)),
while the flight, hit and wall laws stay Detonator's. The code gates on
`Detonator::objects` being `None`, so HD, Omega and 2048, which name their own round model,
are unchanged. The round flies at 500 units a second (HD's law), well past the Pickup's, so
a round is read as a pair of glowing darts leaving the nose rather than as a lingering shot.

## The EMP

Pulse's `WO_QUAKE`, the first choice, blinded the screen white at the launch and left
nothing visible half a second later. The EMP now borrows the Repulsor:

- `WO_REPULSER` rides the wave's front, in the frame the Repulsor's own wave takes (the
  EMP's front stepper is the Repulsor's, `detonator-emp.md`);
- at the launch: the Repulsor's blue screen flash, `WO_REPULSER_BLAST` riding the firer and
  the field ring `pulse_repulsorwave.vex`, for the Repulsor's 1.6 s life. The ring widens
  after 0.2 s where a Repulsor holds it until its waves start, because the EMP's wave
  leaves at once.

The front itself is soon far ahead on the road and small; the launch look is what tells a
player the EMP fired. Code: `oag_raceplay::emp_repulsor`.

## The gauge

`assets/fallback/Data/XML/DetonatorLayouts_Pulse/Detonator_HUD.xml` pulls the disc's
`TimeTrial_HUD.xml` in (`<LoadXML>`) and adds, over the speed bar: `EMP` in the small HUD
face, six segments cut from Pulse's shield bar art (dim track, cyan fill) and a mark under
the third, shown once the EMP can be fired. Six pips because Pulse's EMP is full at six
pads. The PS2 disc authors its HUD in 640x448, so it reads `Detonator_HUD_PS2.xml`, the
same widgets scaled by 4/3 and 448/272.

## Why the data lives in the fallback layer

The layout files and the Detonator-only weapon blocks are read by the same parsers HD's are
(`oag_tables::detonator_layout`, `oag_tables::weapons::detonator_supplement`), the HUD layouts by `oag_hud::compose`, out of
`assets/fallback/` ([ADR-0061](../architecture/adr/0061-two-asset-layers-around-the-disc.md)),
embedded by `oag_source::layers::fallback_layer`. A player's override layer can replace any of
them. Pulse's own race table stands under the supplement: its `Mine`, `Bomb` and
`Cannon` are the race's, and the supplement adds only what Detonator reads beyond them.

## PS2

The same `Title`, so the same data apart from the HUD layout (above); a PS2 race loads, flies `zone_track.vex` and draws the red
pads with the PS2 disc's own pad texture (26 speed pads on `16_Track`'s Zone twin).

## HD, 2048 and Omega

Unchanged: the new `Title` fields are `false`/`None` there, and their Detonator tests in
`crates/raceplay/tests/detonator_ground_truth.rs` stand.

## Evidence

`crates/raceplay/tests/detonator_ground_truth.rs`,
`a_pulse_detonator_race_lays_pulse_mines_on_a_zone_circuit_with_red_charging_pads` (PSP USA):
the Zone twin is flown, the layout and supplement are read, the 17 speed pads of
`16_Track`'s Zone twin carry the weapon pad in red, the first tick lays 15 to 30 mines, three
pads ready the EMP with no boost, L1 clears the mines ahead for 5 each, and Square spends the
clip. `a_pulse_detonator_hud_shows_the_emp_gauge_lit_by_the_charge` (PSP and PS2) checks the
gauge; `a_pulse_detonator_round_is_drawn_as_the_pulse_cannons_own` and
`a_pulse_emp_wave_is_the_repulsors_wave_and_opening` (`crates/raceplay/src/tests/detonator_fx.rs`)
the cannon and the EMP.
