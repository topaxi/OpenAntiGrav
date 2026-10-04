# The particle effects are played from the disc; 32 of the PSP's 35 play, 3 wait on a trigger

2026-08-12, census rewritten 2026-10-02. `oag_vex::pob` parses every emitter
tree, `oag_render::psys::Library` loads any `Data\Psys\<name>.POB` by name, and
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
| 4 | `WO_SHURIKEN_TRAIL` | trigger read, unwired | `Shuriken_Init` `0x08877280`: a second anchor rotated -pi/2 about the blade; a `Projectile` here has no roll |
| 5 | `WO_SHIP_COLL_SPARK_TRAIL_SMOKE` | unread | no string in the executable; a sibling of the embedded `_TRAIL`, nothing found referencing it |

Evidence for the new rows and the weather:
[placed-particle-systems.md](../../docs/ghidra/functions/psp-pulse-usa/placed-particle-systems.md);
for `NODAMAGE`: [contact-response.md](../../docs/ghidra/functions/psp-pulse-usa/contact-response.md).

## Open

- **The weather is wired** (Fort Gale rain and lens, Outpost 7 snow), see
  [weather.md](../../docs/ghidra/functions/psp-pulse-usa/weather.md). Open:
  - ~~Outpost 7's snow withheld~~: closed 2026-10-04. `WO_SNOW` sets the immortal flag
    `0x800`, which ours ignored, so its flakes died five ticks in. The flag is now honoured
    in `oag_render::psys`, and the withhold is lifted. This also makes `WO_LEACHBEAM_ENERGY`
    immortal, the only other Pulse emitter with the flag. Its instance is killed on every
    re-spawn, so the pool resets, but its on-screen particle count was not re-checked
    against the original.
  - The mist overlay is recovered and live-checked (`WeatherMist_*`, `0x088fa0a0`) but not
    played. It needs a screen pass: two additive full-screen quads over `Mist.mip` with
    repeat wrap.
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
  - Shape 8's extent, and the animated-attribute selectors nothing on the disc authors.
  - The `instance[+0x40]` alpha scale, which nothing feeds here.

- **`WO_REPULSER_BLAST` matches the original** (2026-10-04, pulse-psys-ring). It
  needed four interpreter laws, now played: the emitter's playback rate, selector 5 as
  the lifetime co-factor, flag `0x200000`'s even ring and the ring's azimuth sign. It
  also needed the beads to ride the blast's live matrix (flag `0x2`, owner matrix by
  pointer). Frames: `data/scratch/pulse-psys-ring/shots/final-cmp.png`. See
  [particle-system.md](../../docs/ghidra/functions/psp-pulse-usa/particle-system.md),
  "The emitter's clock and the burst laws". Still open from it:
  - **The wave-start whiteout is `WO_REPULSER`.** Its `shazzam` template draws a
    full-screen white bar at update 49, and its class-6 root emitter draws a white
    blob at 52. The original shows a blue tint and then a thin streak. Shape 8 and
    the class-6 draw are both unread.
  - **The rate changed eight emitter records in six effects, plus five templates under
    them.** Besides the blast, these play at their authored rate now: the LeachBeam
    charge (all three), the Missile explosion's root, the Shuriken bounce and expiry,
    and the absorb. The sub-frame spread also changed every wall scrape's spark trails
    and the missile trail. Only the Missile was looked at, and none of them was compared
    against the original.
  - **`WO_REPULSER`'s extent co-factor is `|step| / 100`**, set by
    `Repulser_AdvanceWave`. It is not played, because the root's shape 8 is unread.
  - **Flag `0x2` riding is opt-in per caller.** Only the blast opts in. 25 emitters
    carry the flag, so the other owners' `Psys_Spawn_q` `param_5` is the open question.
    Read each call site before opting it in. The collision sparks have a claim on
    record against riding.
  - **The spin's sense** (`Quat::from_axis_angle` against `Math_RotateByAxisAngle`) now
    shows, because the beads turn with it. It is unmeasured.

## Next Steps

- Play the mist overlay off the law in weather.md's "The mist overlay" section. Check the
  camera-motion signs (`CameraMotion_Sample_q`) against a live turn first.
- `NODAMAGE`: catch a live `damaged == 0` hit (`ShipCollisionFx_Trigger`
  `0x089246b4`, `a2`). A craft dropped onto magstrip or the floor at speed is the
  candidate. Then add a reporting-only floor-impact field to
  `oag_physics::wall::WallResponse` without touching `reacts()`.
- Read the welder's streak law (render class `Streak`, `Capped`) against one
  live particle.
- `WO_REPULSER`'s whiteout: on PPSSPP, fire a Repulser parked on Talon's Junction and
  read the two `WO_REPULSER` instances at updates 48 to 52. Read their `+0xf0` matrix,
  the `shazzam` template's position, size and aspect, and the root pool. Then read
  `FUN_08917c7c` (class 6) and shape 8's emit function.
- Before opting another flag-`0x2` effect into `System::set_rides_frame`, read its
  owner's `Psys_Spawn_q` call for `param_5 & 1`.
- Do not fire any effect on a guess (the do-not-invent rule in `CLAUDE.md`).
