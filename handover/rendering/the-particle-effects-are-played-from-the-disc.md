# The particle effects are played from the disc; 32 of the PSP's 35 play, 3 wait on a trigger

2026-08-12, census rewritten 2026-10-02. `oag_pob` parses every emitter
tree, `oag_fx::psys::Library` loads any `Data\Psys\<name>.POB` by name, and
`psys::Stage` plays any number at once (`attach`/`follow`/`detach` for one riding
a moving owner, `play` for a burst). **The mechanism is generic and finished;
what is per-effect is the trigger.** `crates/game/tests/psys_inventory_ground_truth.rs`
is the authority on which bucket each disc effect is in and fails when one is
unaccounted for; the table below is its reading on 2026-10-02.

## Census: the PSP disc's 35 systems

**Wired, trigger recovered (27):**

| Effect | Trigger | Where |
| --- | --- | --- |
| `WO_SHIP_COLL_SPARK_DAMAGE` | wall contact, and `Ship_Damage`'s weapon branch | `race::tick`, `race::hit_sparks` |
| `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` | a LeachBeam drain's hit | `race::hit_sparks` |
| `WO_WEAPON_ABSORB` | `Ship_PlayAbsorbFeedback` | `race::absorb` |
| `WO_SHIP_FXNODE_EXPLO`, `WO_SHIP_DEATH_SPARKS`, `WO_SHIP_EXPLOSION` | a wreck | `race::wreck_fx` |
| `WO_ROCKET_FLARE`, `WO_ROCKET_EXPLO`, `WO_ROCKET_EXPLO_TRACK` | Rocket init / craft hit / track hit | weapons visuals |
| `WO_MISSILE_HEAD`, `WO_MISSILE_EXPLO`, `WO_MISSILE_BOUNCE` | Missile init / detonation / bounce | weapons visuals |
| `WO_PLASMA_HEAD`, `WO_PLASMA_FLASH` | Plasma init / detonation | weapons visuals |
| `WO_SHURIKEN_HEAD`, `WO_SHURIKEN_BOUNCE`, `WO_SHURIKEN_EXPIRE` | Shuriken init / bounce / teardown | weapons visuals |
| `WO_MINE_EXPLO`, `WO_BOMB_SMOKERING`, `WO_CANNON_SPARKS` | detonations, a round's wall hit | weapons visuals |
| `WO_QUAKE`, `WO_LEACHBEAM_ENERGY`, `WO_LEACHBEAM_CHARGING` | the Quake wave, the beam, the held pickup | weapons visuals |
| **`WO_REPULSER_BLAST`, `WO_REPULSER`** (2026-10-04) | `Repulser_SpawnBlastEffect` at the fire / `Repulser_SpawnWaves` per wave | `race::weapons::repulser` |
| **`WO_BLUE_WELDER`** (new) | placed by circuits 01 and 05 as `ParticleSystem` nodes, played from load | `race::scenery_fx` |
| **`WO_MODESTO_STEAM_A`** (new) | placed by circuits 05 and 07 the same way | `race::scenery_fx` |

**Embedded in a wired tree (2):** `WO_SHIP_COLL_SPARK`, `WO_SHIP_COLL_SPARK_TRAIL`.

**Not wired (3), ranked by how often a player would see them:**

| Rank | Effect | State | Best lead |
| ---: | --- | --- | --- |
| 3 | `WO_SHIP_COLL_SPARK_NODAMAGE` | trigger read, not seen live | `Ship_DispatchCollisionFx` `0x0883df38`: a contact with non-positive friction (floor, magstrip) |
| 4 | ~~`WO_SHURIKEN_TRAIL`~~ | wired 2026-10-06 | `Shuriken_Init` `0x08877280`: a second anchor rotated -pi/2 about the blade; played on `Stage::orient` to the velocity |
| 5 | `WO_SHIP_COLL_SPARK_TRAIL_SMOKE` | unread | no string in the executable; a sibling of the embedded `_TRAIL`, nothing found referencing it |

Evidence for the new rows and the weather:
[placed-particle-systems.md](../../docs/ghidra/functions/psp-pulse-usa/placed-particle-systems.md);
for `NODAMAGE`: [contact-response.md](../../docs/ghidra/functions/psp-pulse-usa/contact-response.md).

## Wipeout 2048 (2026-10-05, v2048-particles)

All 174 of 2048's `.pob` parse (4 refused by name); a race reads
`Data\Particles2048` with its `.gxt` sprites (`RaceDefaults::effect_dir`,
`Effect::parse_with`). 27 of the 36 `RACE_EFFECTS` load. **Firing today, with the
title-neutral triggers:** wall-contact sparks, weapon visuals (Rocket, Missile,
Plasma, Shuriken, Mine, Bomb, Cannon, Quake, Repulser, LeachBeam), trail hits.
Seen on screen: wall sparks, a rocket flare and a rocket detonation
(`data/scratch/v2048-particles/shots/`). **Not firing, trigger unread or Pulse-gated:**

- Wreck (`wreck_fx`, Pulse-gated): 2048 authors `WO_SHIP_EXPLOSION_PLAYER`,
  `WO_ZONE_SHIP_EXPLOSION`, `WO_SHIP_DEATH_DAMAGE_PLUME`, not the Pulse names.
- Hit sparks and absorb (`hit_sparks`, `absorb`: Pulse/Pure/HD only).
- Zone: the executable swaps in `WO_SHIP_COLL_SPARK_DAMAGE_ZONE` (and
  `_NODAMAGE_ZONE`, `WO_DAMAGE_ELECTRIC`) in Zone modes
  (`vita-2048-eu-v104/particle-paths.md`); not wired.
- `WO_DAMAGE_MILD/MODERATE/CRITICAL`, `WO_FORCE_FIELD*`, `WO_DEBRIS_*`,
  `WO_MAGSTRIP_*`, `WO_DUST_TRAIL*`, bomb/mine halos: named by the executable,
  event unread.
- Engine flare, welder, weather (`WO_RAIN`/`WO_SNOW` absent from 2048).
- The 17 patch-only effects; the patch archives are not mounted.
- Which game modes (`>= 23`) read `Data/Particles` instead.
- No reference capture of 2048 was compared; the pictures are judged alone.

## Wipeout: Omega Collection (2026-10-05, omega-particles)

`Data\particles` / `Data\particles2048` with `.gnf` sprites
(`Sprite::from_gnf`); `RaceDefaults::effect_dir_by_circuit` picks the set per
circuit, **chosen, not measured** - the eboot's flag at `0x01f99bc0` decides
and which circuit sets it is unread (`ps4-omega-eu/particle-paths.md`). 80/97
and 110/112 (95/97) parse since 2026-10-06 (logwarn-2048): blend class 8 (distortion and
heat haze: rocket, missile, mine, bomb-ring, plasma-expand, `WO_RB_HEAT*`) is read as
`Blend::Distort`, simulated and **not drawn**, and the loader names each such emitter at
WARN. Seen: wall sparks, rocket flare (`data/scratch/omega-particles/shots/`), the rocket
explosion's fireball (`data/scratch/logwarn-2048/shots/omega-rocket.png`). Open: **draw
the distortion** (what `psys_normal_heathaze_vp/fp` samples and how it offsets is
unrecovered; no scene-colour grab pass exists in `oag-render`/`oag-post`/`oag-fx`; Omega
only, no 2048 `.pob` uses class 8); read the flag's writers to make the per-circuit
directory choice measured; the 2048 leftovers (rocket flare, Zone spark swap).
Pulse's `WO_PLASMA_FLASH`, `WO_SHIP_ENGINEFLARE`, `WO_LEACHBEAM_ENERGY`, `WO_BLUE_WELDER`,
`WO_RAIN*` and `WO_SNOW` are authored by neither 2048 nor Omega (`pob.md`, "Effects Pulse
names that 2048 and Omega never authors") and no longer load there.

## Open

- **The weather is wired** (Fort Gale rain and lens, Outpost 7 snow), see
  [weather.md](../../docs/ghidra/functions/psp-pulse-usa/weather.md). Open:
  - ~~Outpost 7's snow withheld~~: closed 2026-10-04. `WO_SNOW` sets the immortal flag
    `0x800`, which ours ignored, so its flakes died five ticks in. The flag is now honoured
    in `oag_fx::psys`, and the withhold is lifted. This also makes `WO_LEACHBEAM_ENERGY`
    immortal, the only other Pulse emitter with the flag. Its instance is killed on every
    re-spawn, so the pool resets, but its on-screen particle count was not re-checked
    against the original.
  - The mist overlay plays (`oag_fx::mist`, 2026-10-04). Its draw law matches the
    original's pixels (corr 0.84 and 0.92, weather.md "Played, and checked"). Open: a
    matched pose while driving, a climb, and the rain lens over it were not compared.
  - The PS2 disc authors `<Weather>` on three circuits and plays none here.
  - Covered-section rain was not distinguishable in either side's frames.
- **The welder against the original** (matched frames on Basilico Black):
  - The halo now flashes where the original's does. The fix: a periodic
    channel keeps its equal-time keys.
  - Ours still draws the sparks as long streaks where the original's are short
    falling dots, and ours has less white core.
- **The steam vents were not compared on screen.** Outpost 7's 18 loads are
  live-confirmed, but the vents sit 80 or more units off and below the racing line.
- **Placed effects are not culled.** Unread whether the draw slot `0x08915fd0`
  skips them; ours draws all of them. They are also Pulse PSP only: Pure, HD and
  the PS2 port were not checked for placed nodes.
- **Interpreter gaps**, unchanged:
  - HD's `.gtf` sprites, sprites on streaks and the atlas frame over life.
  - ~~Shape 8's extent~~ (a half ring, 2026-10-04); the animated-attribute selectors
    nothing on the disc authors.
  - The class-7 pool draw `FUN_08918160`: ours plays the template routine for it, as
    it did for class 6. Unread.
  - The `instance[+0x40]` alpha scale, which nothing feeds here.

- **`WO_REPULSER_BLAST` matches the original** (2026-10-04, pulse-psys-ring). It
  needed four interpreter laws, now played: the emitter's playback rate, selector 5 as
  the lifetime co-factor, flag `0x200000`'s even ring and the ring's azimuth sign. It
  also needed the beads to ride the blast's live matrix (flag `0x2`, owner matrix by
  pointer). Frames: `data/scratch/pulse-psys-ring/shots/final-cmp.png`. See
  [particle-system.md](../../docs/ghidra/functions/psp-pulse-usa/particle-system.md),
  "The emitter's clock and the burst laws". Still open from it:
  - ~~**The wave-start whiteout is now only the `shazzam` template**~~ Closed 2026-10-06 (weapon-visuals): PPSSPP replays of two GE dumps show the out-of-range quad is dropped and an in-range one draws a band; played as `oag_fx::psys::guard`. Still open: the boundary itself, and the other wave's `shazzam` absent from later submissions.
  - **Every class-6 emitter changed draw** (23 on the PSP disc): it is a bar `2 size`
    wide, capped by `aspect * size`, not the wedge. Before/after frames of the Missile,
    Shuriken, LeachBeam and Fort Gale rain (`data/scratch/pulse-psys-shape8/shots/ba-*.png`)
    show no breakage, but none caught its class-6 effect on screen except the Missile's
    trail, and none was compared against the original.
  - **The class-6 `+0x60` end is read, not played**: the pool bar's other end is last
    drawn frame's view-space position for the five carriers with neither `0x1000000`
    nor `0x2000000` (`plasma_goo`, the missile `trail`, `plasma_spikes`, the spark
    `bits`). Ours uses the previous tick's world position.
  - **The rate changed eight emitter records in six effects, plus five templates under
    them.** Besides the blast, these play at their authored rate now: the LeachBeam
    charge (all three), the Missile explosion's root, the Shuriken bounce and expiry,
    and the absorb. The sub-frame spread also changed every wall scrape's spark trails
    and the missile trail. Only the Missile was looked at, and none of them was compared
    against the original.
  - ~~`WO_REPULSER`'s extent co-factor is `|step| / 100`~~: wrong, it is the track's
    width over 100 (read and measured 2026-10-04), and it is played.
  - **Flag `0x2` riding is opt-in per caller.** Only the blast opts in. 25 emitters
    carry the flag, so the other owners' `Psys_Spawn_q` `param_5` is the open question.
    Read each call site before opting it in. The collision sparks have a claim on
    record against riding.
  - **The spin's sense** (`Quat::from_axis_angle` against `Math_RotateByAxisAngle`) now
    shows, because the beads turn with it. It is unmeasured.

## Next Steps

- Mist: compare one matched driving pose (feed the original's dumped UVs into ours) and a
  climb, to score the pitch term live.
- `NODAMAGE`: catch a live `damaged == 0` hit (`ShipCollisionFx_Trigger`
  `0x089246b4`, `a2`). A craft dropped onto magstrip or the floor at speed is the
  candidate. Then add a reporting-only floor-impact field to
  `oag_physics::wall::WallResponse` without touching `reacts()`.
- Read the welder's streak law (render class `Streak`, `Capped`) against one
  live particle.
- `WO_REPULSER`'s `shazzam`: take a GE dump 3 to 6 updates after the waves start
  (`data/scratch/pulse-psys-shape8/probe5.py 45091 <dir> 1 <skip>`), when the quad is
  15 to 40 units out and inside `+-2048` px. A drawn band there makes the guard-band
  cull the law; then cull psys triangles by it in `upload_particles`, where `vp` is.
- Read `FUN_08918160`, the class-7 pool draw, the way `0x08917c7c` was read.
- Before opting another flag-`0x2` effect into `System::set_rides_frame`, read its
  owner's `Psys_Spawn_q` call for `param_5 & 1`.
- Do not fire any effect on a guess (the do-not-invent rule in `CLAUDE.md`).
