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
  - The mist overlay plays (`oag_render::mist`, 2026-10-04). Its draw law matches the
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
  - Billboard roll, the extents of shapes 2/3/6/8 and the animated-attribute array.
  - The `instance[+0x40]` alpha scale, which nothing feeds here.

- **`WO_REPULSER_BLAST` against the original** (2026-10-04, pulse-repulser-2,
  Talon's Junction, craft parked): the original's blast is a compact beaded ring
  round the craft for 0.17-0.67 s, then a white cloud; ours starts about 13.6
  units out and collapses inward by 0.33 s. Proved to be the psys, not the
  field model, by holding the model's alpha at zero live. Flag `0x200000`
  (evenly stepped ring angles) and the selector-5 record are still not played;
  they are the first thing to try. Frames:
  `data/scratch/pulse-repulser-2/shots/psp-seq.png`, `oag-seq.png`. See
  [repulser.md](../../docs/ghidra/functions/psp-pulse-usa/repulser.md).

## Next Steps

- Mist: compare one matched driving pose (feed the original's dumped UVs into ours) and a
  climb, to score the pitch term live.
- `NODAMAGE`: catch a live `damaged == 0` hit (`ShipCollisionFx_Trigger`
  `0x089246b4`, `a2`). A craft dropped onto magstrip or the floor at speed is the
  candidate. Then add a reporting-only floor-impact field to
  `oag_physics::wall::WallResponse` without touching `reacts()`.
- Read the welder's streak law (render class `Streak`, `Capped`) against one
  live particle.
- Do not fire any effect on a guess (the do-not-invent rule in `CLAUDE.md`).
