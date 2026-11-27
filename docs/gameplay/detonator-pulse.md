# Detonator on Wipeout Pulse

Wipeout Pulse ships no Detonator. This engine races one on Pulse (PSP and PS2) at the
maintainer's request: "backport detonator to Pulse, shall use zone races for tracks, normal
mines for mines, speed and weapon pads replaced by red weapon pads". The mode's law is HD's
(`docs/ghidra/functions/ps3-hdfury-eu/detonator-*.md`); everything shown is Pulse's own disc
data. Nothing on this page is measured on Pulse: every value is **chosen, not measured**,
because the original has no such mode.

The mode's files are the official mod `oag.detonator-pulse` (`assets/mods/official/detonator-pulse/`,
on by default; see [Why the data is a mod](#why-the-data-is-a-mod)). With it on, try
`just play <pulse disc> --race --mode detonator`, or RACEBOX, MODE `DETONATOR` (the CIRCUIT row
then lists the Zone circuits only). With it off, Pulse has no Detonator.

## What a race has

| Part | On Pulse | Where |
| --- | --- | --- |
| Circuit | the picked circuit's Zone twin (`zone_track.vex` beside `track.vex`), as a Zone race flies | `oag_title::Detonator::races_zone_circuit` |
| Mines and bombs | Pulse's own `Pulse_Mine.vex` and `Pulse_Bomb.vex`, laid by HD's placement law; contact radius, damage and push from this project's Detonator table (see [The numbers](#the-numbers)) | `objects: None` (the ordinary mine and bomb pools draw them), `oag_title::weapons::Weapons::detonator` |
| Mine size | the mines hold their authored size; the music does not scale them (see [Mine size](#mine-size)) | `MinesScaleToMusic="0"` |
| Layout | this project's six pattern groups; Fury's fourteen stage counts (80 mines at stage 1 to 200 at 14) | `assets/mods/official/detonator-pulse/fallback/wipeout-pulse/Data/XML/DetonatorLayouts_Pulse/` |
| Pads | every speed pad (a Zone circuit authors no weapon pad: 0 on all eight PSP `zone_track.vex`, against 7 to 15 on the race files) is drawn as the race circuit's own first `Weapon Pad`, held at the red key of its ready cycle (`READY_KEYFRAMES[0]`, 204, 102, 102; `docs/rendering/pads.md`); the circuit's own speed pads are not drawn | `DetonatorPads::circuit_weapon_pad_key` |
| Pad law | a pad gives no boost and charges the EMP once per entry, HD's arm | `DetonatorPads` `speed_pad_charges`, `weapon_pad_charges` |
| EMP | HD's charge, gauge, L1 fire and wave: ready at 4 pads, full at 8, extent 0.25 to 2 s, 1 survey entry a tick; drawn as the Repulsor's (see [The EMP](#the-emp)), no light | `WeaponStats_Detonator.xml`, `oag_pulse::effects`, `emp_borrows_repulsor` |
| Cannon | HD's cannon: 15 volleys, one every 0.1 s, one wall rebound, four hits to a bomb; the rounds leave from the hull's dimensions (Pulse's hulls author no `cannon_flash`); each round is drawn as Pulse's own Cannon's (see [The cannon](#the-cannon)); a wall hit sparks Pulse's `WO_CANNON_SPARKS` | `WeaponStats_Detonator.xml` |
| Explosions | a mine and a bomb go up as Pulse's `WO_MINE_EXPLO` | `oag_pulse::effects` |
| Sounds | Pulse's own `MINEEXPL` and `BOMBEXPL` (`weapons.bnk`) and, on a pad charge, `WEAPONPICKUP` (`hud.bnk`) | `oag_sound::sfx::Banks::load_detonator` |
| End of a run | as a Pulse Zone run: out of the race when the 0.5 s explosion ends, the results at once, the blast, wreck and destroy camera going on behind them (the maintainer's ruling, 2026-11-27) | `DestroyedCraft::detonator_end` `None` |
| Scoring | Fury's figures (100 a live mine, 50 a dark one, 2000 a bomb, 50 an EMP'd live mine) | `DetonatorScoring.xml` in the same directory |
| HUD | this project's own layout, arranged as Fury's (see [The HUD](#the-hud)) | `oag_pulse::hud::layouts::DETONATOR`, `ps2_hud` |
| Not drawn | the stage grade (Pulse ships none), the medals, the announcer lines | `palette: None`, `shows_medals`, `voices: None` |
| Craft | this project's own hull drawn on the team's Zone hull (`Zone.vex`): its textures, render state, boost plume and wreck, with the geometry, flare anchor, spark anchors and shadow of `assets/mods/official/detonator-pulse/fallback/wipeout-pulse/Data/Ships/Detonator_Pulse/hull.glb` (see [The craft](#the-craft)); the team's race handling; Ship Select previews it on `Zone_FE.vex` with the skin row fixed and the team's own ratings | `craft: Some`, `DetonatorCraft::own_hull`, `handling: None` |

## The numbers

The maintainer asked for the rules close to Fury's (2026-11-27). The figures in this project's
`WeaponStats_Detonator.xml`, `GamePlayValues.xml` and `DetonatorScoring.xml` are chosen after
HD's own, as recorded in [weapon-stats.md](../formats/weapon-stats.md) and
[detonator-layouts.md](../formats/detonator-layouts.md). Pulse's shields run 95 to 110 against
HD's 100, so a damage figure means about the same.

| Figure | Value |
| --- | --- |
| Mine | 10 damage, halves the speed; no push, no slowdown time |
| Bomb | 20 damage, push 30, four cannon hits |
| Cannon | `rate` 10 (a volley every 0.1 s), 15 volleys, 0.75 s reload either way |
| EMP | ready at 4 pads, full at 8, 0.25 to 2 s, speed 1 |
| Stages | 80 to 200 mines; 10 shield back a stage; 100 for a chain reaction mine |
| Score | multiplier 1.0, 100 / 50 a mine, 2000 + 500 a bomb; EMP 50 / 25 / 1000 |

The pattern groups stay this project's six: HD's `PatternFile.xml` is authored content and is
not copied.

Still not Fury's: the handling is the team's own at the picked speed class (HD flies one
classless Detonator file, and Pulse ships none); the circuits are the Zone ones; a destroyed
craft ends the run as a Pulse Zone run, with none of Fury's 5.5 s wait; there are no
medals, score targets, campaign events, EMP voice lines or floating `+N` popups.

## Mine size

HD scales a laid mine's body to the music. On Pulse the maintainer ruled that out (2026-11-27),
so this project's `DetonatorLayouts_Pulse/GamePlayValues.xml` sets `MinesScaleToMusic="0"`. The
attribute is this project's: no shipped file names it, so HD, Omega and 2048 scale as before.

A held mine is sized to the distance a craft sets it off at (the maintainer's ruling,
2026-11-27): the contact test is the craft's position within the Mine's `trigger_radius` of the
mine, 3 in this project's Detonator table, and `Pulse_Mine.vex` reaches 3.18 from its origin at its
authored size (its additive spikes; the solid body ends at 1.32). The size is `trigger_radius`
over that reach, about 0.94, both read at load, so a modded model or table sizes itself.
Pinned by `a_pulse_detonator_mine_scales_to_the_music_only_if_its_layout_says_so`
(`crates/raceplay/src/tests/detonator_fx.rs`). A Pulse race mine is drawn smaller, at 0.6, so a
Detonator mine looks about 1.6 times a race one.

The mine spins on the model's own animation. It does not take the race mine's code tumble
(`Mine_PoseNode`, [mine.md](../ghidra/functions/psp-pulse-usa/mine.md)).

An override brings the scaling back: copy the whole file to
`overrides/wipeout-pulse/Data/XML/DetonatorLayouts_Pulse/GamePlayValues.xml`
([ADR-0061](../architecture/adr/0061-two-asset-layers-around-the-disc.md)) and set
`MinesScaleToMusic="1"`. An override replaces the file, so a copy holding the attribute alone
loses every other value in it.

## The cannon

Pulse ships no bolt model, so each Detonator round draws as a fired Cannon pickup's round
does on Pulse (`pulse_muzzleflash.vex` posed on the round, `Cannon_bolt.mip`'s streak, and
the `cannon muzzle flash` quad for the first 0.1 s,
[cannon-quake-leachbeam.md](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md)),
while the flight, hit and wall laws stay Detonator's. The code gates on
`Detonator::objects` being `None`, so HD, Omega and 2048, which name their own round model,
are unchanged. The round flies at 500 units a second (HD's law), well past the Pickup's, so
a round is read as a pair of glowing darts leaving the nose rather than as a lingering shot.

The clip is 15 volleys of two rounds. A table naming more is held to 15
(`oag_weapons::detonator::cannon::CLIP_VOLLEYS`): the cannon has hit slots, streak awards and
HUD icons for no more.

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

## The craft

The craft is a hull of this project's own, drawn in Blender, on each team's own Zone hull. It
wears the team's Zone paint, the textures embedded in its `Zone.vex`, so the file carries only
geometry, normals, texture coordinates and vertex colours, and names the textures it paints
with. Format and rules: [own-hull.md](../formats/own-hull.md); authoring:
[blender.md](../tools/blender.md#a-craft-hull).

- `oag_pulse::race::DETONATOR`'s `own_hull` lists `Detonator_Pulse\hull.glb` (the PSP
  model) and then `Detonator_Pulse\hull_ps2.glb`. The first that fits the disc's `Zone.vex`
  is drawn.
- Pulse PS2's `Zone.vex` is another model (other node names, one `Zone_ColourMap.tga`
  atlas), so the PS2 pressing has a hull of its own, modelled apart at that model's density:
  462 triangles on the PSP (the disc's Zone hull has 417), 1,470 on the PS2 (1,494).
- Both are original shapes in the manner of Fury's Detonator craft: forward-swept drooping
  wings ending in cannon pods, a wedge fuselage under a bubble canopy, twin canted fins and a
  tail boom. Neither has a far LOD part; the hull draws at every distance.
- The hull's exhaust anchor (`engine_flare`), spark anchors and shadow occluder come from
  the file. The cannon still leaves from the hull's dimensions (on PS2 from the disc hull's
  `cannon_flash` locators, at its nose and not at the new pods) and the Plasma node is the
  disc's `ship_muzzle`, because the simulation reads them off the `.vex`.
- A player's override layer replaces the file by path. A file that will not read or fit
  draws the disc's hull and says so.

## The HUD

`Data\XML\DetonatorLayouts_Pulse\Detonator_HUD.xml` (in the mod) is a whole layout of this
project's. None of the four Pulse layouts read (Arcade, Elimination, Time Trial, Zone) uses
`<LoadXML>`; Detonator's does not either. It is arranged as Fury's:

- bottom left, the score (`DetonatorScorePlain`, plain digits as Pulse's Zone score) under `SCORE`, at the
  coordinates each pressing's `Zone_HUD.xml` gives its own;
- above it, the EMP: `EMP` over eight segments cut from the speed bar's art and a mark under
  the fourth, shown once the EMP can be fired. Eight pips because the EMP is full at eight
  pads;
- bottom right, over the shield bar: the volleys left (`PulseBullets`), an icon a volley
  (`Bullet00`-`14`) and over each a pip lit when that volley hits (`Multi00`-`14`);
- the lap (the stage) top left, the countdown and the centre messages where Pulse puts them;
- no speed readout, lap times or pickup box.

It names the disc's own HUD texture, fonts, strings and countdown models. The PS2 disc authors
its HUD in 640x448, so it reads `Detonator_HUD_PS2.xml`. Both replace by path from the override
layer.

## Why the data is a mod

The layout files, the weapon table, the HUD layouts and the two hulls are the official mod
`oag.detonator-pulse`'s fallback entries, `fallback/wipeout-pulse/` of the mod
([ADR-0098](../architecture/adr/0098-a-mod-carries-fallback-entries.md)): an enabled mod's
entries answer a name no disc or DLC holds, ahead of this project's embedded fallback. They are
read by the same parsers HD's are (`oag_tables::detonator_layout`, `oag_tables::weapons`), the
HUD layouts by `oag_hud::compose`. A player's override layer can replace any of them.

`oag_pulse::race::DETONATOR` still names the files; the engine names no mod. A mode that loads
data of its own is offered only where every entry it names resolves in the title's archives
(`oag_title::Title::detonator_data`, `oag_ui::menu::mode_offered`): on HD the disc holds them, on
Pulse the mod does. With the mod off, the MODE row has no `DETONATOR`, a saved
`race.mode = "detonator"` races and previews the default mode, and `--race --mode detonator`
refuses with a `warn` naming the missing entries. Switching the mod on the MODS page re-offers
the mode at once.

Packaged builds do not copy the official mods yet, and web and phone builds have none
([mods.md](../tools/mods.md#official-mods-assetsmodsofficial)), so Pulse Detonator is absent there until they do.
Detonator is single player, so the mod sets `[mod] network-races = false`: no LAN or online
race mounts its entries and it leaves the network fingerprint alone, so a player with it on
races one without it.
In Detonator the table is read in place of Pulse's race table, so a mod to the race table
does not reach the mode.

## PS2

The same `Title`, so the same data apart from the HUD layout (above); a PS2 race loads, flies `zone_track.vex` and draws the red
pads with the PS2 disc's own pad texture (26 speed pads on `16_Track`'s Zone twin).

## HD, 2048 and Omega

Unchanged: the new `Title` fields are `false`/`None` there, and their Detonator tests in
`crates/raceplay/tests/detonator_ground_truth.rs` stand.

## Evidence

`crates/raceplay/tests/detonator_ground_truth.rs`,
`a_pulse_detonator_race_lays_pulse_mines_on_a_zone_circuit_with_red_charging_pads` (PSP USA):
the Zone twin is flown, the layout and table are read, the 17 speed pads of
`16_Track`'s Zone twin carry the weapon pad in red, the first tick lays 80 mines, four
pads ready the EMP with no boost, L1 clears the mines ahead for 50 each, and Square spends the
clip. `a_pulse_detonator_hud_shows_the_emp_gauge_and_the_clip` (PSP and PS2) checks the
three sounds, the gauge and the clip; `a_pulse_detonator_run_ends_as_a_pulse_zone_run_does`
the end of a run; `a_pulse_detonator_round_is_drawn_as_the_pulse_cannons_own` and
`a_pulse_emp_wave_is_the_repulsors_wave_and_opening` (`crates/raceplay/src/tests/detonator_fx.rs`)
the cannon and the EMP.
