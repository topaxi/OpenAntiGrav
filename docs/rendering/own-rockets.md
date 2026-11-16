# Our own rocket effect for Pulse and Pure

An opt-in extra of this project's, **not the original's**: `graphics.own_rockets` (GRAPHICS ADVANCED, `off`/`on`,
default `off`, takes effect the next race) plays a rocket effect of ours in place of the flare and the blasts
Pulse's or Pure's disc authors. It is a sanctioned exception to "never invent what the assets author": the effect is
authored here and reads **no byte from any disc**, so it works with only a Pulse or Pure disc present. The row is
dropped on HD, 2048 and Omega.

Wipeout HD's rocket is the reference for *which* elements exist and how they read in motion (a rocket with a lit
tail, a pale smoke trail that outlives it, a flash at the muzzle, a fireball with sparks and smoke where it lands).
The look is Pulse's: additive round glows in Pulse's white-yellow to orange to deep red fire range, and chunky
flat puffs for smoke and fire, as its own `.pob` sprites are. Nothing of HD's is copied: no ribbon, no HD texture.

## What it is made of

- **Three effect files** in the fallback layer ([asset-layers.md](../architecture/asset-layers.md)), written in the
  format a mod's effect is ([mod-effect.md](../formats/mod-effect.md)) and played by the same `oag_fx::psys::System`
  the disc's `.pob` effects play through:

  | File (`assets/fallback/Data/Psys/`) | Takes over | Emitters |
  | --- | --- | --- |
  | `OAGRocketFlight.toml` | `Trigger::RocketFlare`, riding the rocket | `launch-flash`, `launch-sparks` (once, at the muzzle), `core`, `halo`, `flame`, `smoke`, `smoke-b` (looping) |
  | `OAGRocketImpactTrack.toml` | `Trigger::TrackBlast` | `flash`, `fireball`, `core`, `sparks`, `smoke` |
  | `OAGRocketImpactCraft.toml` | `Trigger::CraftBlast` | the same five |

- **Four puff sprites**, `assets/fallback/Data/Psys/Tex/OAGRocketPuff1..4.oagmask`: 64 x 64 alpha masks drawn by
  `scripts/gen-fallback-rocket-art.py` from value noise (a disc with a wandering edge, dense inside, clear at the
  border). The smoke trail alternates two of them, the fireball and the impact smoke use the other two. The glows
  (flash, core, halo, flame, sparks) use the particle shader's own round profile and name no texture.
- **The projectile** is the disc's own `Rocket.vex`, as before: the model is the rocket, not part of the effect.
- **The launch flash** is the flight effect's two one-shot emitters: every emitter starts when the effect is
  attached, which is the tick the rocket leaves the muzzle, so no new trigger was needed.
- **The trail** needs no ribbon. The flight effect's frame points `+Y` along the rocket's velocity (the Rocket's
  measured flare frame, `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`), a negative `speed` throws a
  particle back along the flight line, and a particle stays in the world where it was put. Flame and smoke puffs are
  spread over the gap a rocket covers in one tick (3.7 units at the Venom class's 800 km/h) by a signed speed under heavy drag. When the
  rocket is gone the effect is released: emission stops and the smoke finishes its own life (about a second).

## Wiring

- `oag_title::Looks::own_rockets` (a `Rule`, `Origin::Chosen`) is on for Pulse and Pure.
  `oag_raceplay::Options::own_rockets` is the player's setting; `oag_raceplay::own_rockets::on` is the two together.
- `own_rockets::load` reads the three files and their sprites through the race's archives (so the fallback
  layer answers, and a player's override layer can replace any of them), places them in the race's effect library
  after every title effect (no title sprite moves on the sheet), and `EffectHandles::lending` puts each on its
  trigger. A file that does not load is named in the loader report and its trigger keeps the disc's effect.
- With the setting off nothing is read, no library entry is added, no report line is written and no trigger moves.
- Render-side only. No simulation crate changed: the rocket's law, timing, damage and the screen flash a detonation
  starts are what they were.

## Evidence

Captures: `oag-game <image> --race --mode single_race --give rocket --no-audio --size 960x544 --input-script F
--ticks T --screenshot out.png`, `F` holding `cross` and pressing `square` on ticks 420 and 421 (the volley leaves
at tick 422), a settings file with `[graphics] own_rockets = true` for "on". Pulse USA is Talon's Junction, Pure USA
its default circuit, both from the back of the grid, so the volley meets the wall or the pack about a second later.

- **Off is byte-identical.** Eleven ticks (423 to 560: launch, flight, impact, smoke gone) on Pulse USA and Pure
  USA, captured with the binary before the change and the one after with the setting off: all 22 PNGs compare
  equal with `cmp`.
- **On, as a player sees it (chase camera, 960 x 544), beside HD and the disc's own effect at the same ticks:**

  | Phase | HD (ours rendering HD's disc) | Pulse, disc effect | Pulse, ours | Pure, disc effect | Pure, ours |
  | --- | --- | --- | --- | --- | --- |
  | Launch (tick 423 to 436) | white ribbons leave the nose | an orange fire wash over the road | a warm flash at the nose, then three pale trails leaving it | three dark dots, no flash | a pale flash, then one bright streak ahead of the nose (the three trails overlap from behind) |
  | Flight (445 to 460) | three white ribbons to the vanishing point | a fire trail the length of the flight | three lumpy pale smoke trails with an orange head each | dots, nothing behind them | pale trails with orange heads at the pack |
  | Impact (470 to 500) | a white flash, orange sparks, dark smoke | a large orange fireball with dark chunks | a yellow-white heart, a lumpy orange fireball, sparks, grey smoke that thins over 1.5 s | a spark burst on a craft; nothing on a wall | the same fireball, sparks and smoke, on a wall too |

- **What a player would still say.** On Pulse the disc's own flare is a longer, hotter fire trail than ours: ours
  trades fire for smoke, HD's way. From far behind, a rocket's orange head is a few pixels. On a bright road the pale
  smoke has little contrast for its first half second (Talon's Junction's start straight, Pure's grid). From the
  chase camera the firing craft flies through its own trails. Pure's three trails merge into one streak until they
  fan out.
- **The first version drew next to nothing**, and passed its tests: the flame and smoke were authored at a hull's
  scale (half-size 0.6) with the round default sprite, and at chase distance they were a few faint pixels. The
  sizes and the puff sprite came from reading captures, not from the numbers.
- Tests: `own_rockets::tests` (off reads nothing; on takes the three triggers on Pulse and Pure alone; the
  flight effect loops, opens with a one-shot flash and lays puff sprites; a missing file leaves the disc's effect),
  `own_shield_ground_truth` (`rockets_are_the_discs_or_ours` on Pulse PSP, Pulse PS2 and Pure PSP on the loads that
  file already makes: off holds none of ours, on plays ours on each trigger with every sprite placed; HD loads
  none), `menu_definition_settings::the_own_rockets_row_is_offered_only_on_pulse_and_pure`, and the three files in
  `shipped_schema`.

## Open

- **A hotter fire tail** would need a streak emitter along the flight line; the effect format has billboards only.
- **Smoke is unlit and unsorted against other particles.** It is one tone whatever the circuit's light, and a puff
  in front of a flame draws by the stage's instance order, not by depth.
- **The Missile, Mine and the other weapons keep the disc's effects.** Only the Rocket's three triggers move.
- **Opponents' rockets** use the same triggers, so they draw ours too; no capture of one fired at the player was
  taken.
- **Pulse PS2** takes the same files (the disc test covers its load); no PS2 capture was taken.
- **Cross-title check (CLAUDE.md "2048 and Omega")**: checked, differs. HD draws its own ribbon and effects, and
  2048 and Omega are its lineage, so the setting is not offered there and the row is dropped. Not wired for them by
  design.
