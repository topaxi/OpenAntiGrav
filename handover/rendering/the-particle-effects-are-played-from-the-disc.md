# The particle effects are played from the disc; 27 of the PSP's 35 play, 8 wait on a trigger

2026-08-12, census rewritten 2026-10-02. `oag_vex::pob` parses every emitter
tree, `oag_render::psys::Library` loads any `Data\Psys\<name>.POB` by name, and
`psys::Stage` plays any number at once (`attach`/`follow`/`detach` for one riding
a moving owner, `play` for a burst). **The mechanism is generic and finished;
what is per-effect is the trigger.** `crates/game/tests/psys_inventory_ground_truth.rs`
is the authority on which bucket each disc effect is in and fails when one is
unaccounted for; the table below is its reading on 2026-10-02.

## Census: the PSP disc's 35 systems

**Wired, trigger recovered (25):**

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
| **`WO_BLUE_WELDER`** (new) | placed by circuits 01 and 05 as `ParticleSystem` nodes, played from load | `race::scenery_fx` |
| **`WO_MODESTO_STEAM_A`** (new) | placed by circuits 05 and 07 the same way | `race::scenery_fx` |

**Embedded in a wired tree (2):** `WO_SHIP_COLL_SPARK`, `WO_SHIP_COLL_SPARK_TRAIL`.

**Not wired (8), ranked by how often a player would see them:**

| Rank | Effect | State | Best lead |
| ---: | --- | --- | --- |
| 1 | `WO_RAIN` + `WO_RAIN_LENS` | trigger read, unwired | Fort Gale's `TrackStartup` `Weather` element; `Weather_Construct` `0x088f184c`, `Weather_Update` `0x088f1e58` |
| 2 | `WO_SNOW` | trigger read, unwired | Outpost 7's `TrackStartup` `Weather`, same functions |
| 3 | `WO_SHIP_COLL_SPARK_NODAMAGE` | trigger read, not seen live | `Ship_DispatchCollisionFx` `0x0883df38`: a contact with non-positive friction (floor, magstrip) |
| 4 | `WO_SHURIKEN_TRAIL` | trigger read, unwired | `Shuriken_Init` `0x08877280`: a second anchor rotated -pi/2 about the blade; a `Projectile` here has no roll |
| 5 | `WO_REPULSER`, `WO_REPULSER_BLAST` | weapon not built | strings `0x08a7cd2c`, `0x08a7cd18` |
| 6 | `WO_SHIP_COLL_SPARK_TRAIL_SMOKE` | unread | no string in the executable; a sibling of the embedded `_TRAIL`, nothing found referencing it |

Evidence for the new rows and the weather:
[placed-particle-systems.md](../../docs/ghidra/functions/psp-pulse-usa/placed-particle-systems.md);
for `NODAMAGE`: [contact-response.md](../../docs/ghidra/functions/psp-pulse-usa/contact-response.md).

## Open

- **The weather** (ranks 1-2): the trigger is read and needs a port.
  - `Weather_Update` puts the env instance on a camera node's matrix, hides it
    when the camera is inside (`FUN_0887866c` region tests), throttles the
    screen effect on the switch, and writes a wind vector into the env effect's
    first modifier each frame.
  - It also builds a screen-mist node (`FUN_088fa0a0`).
  - Not built in Zone; the whole `LevelFx` block is skipped when
    `g_display+0x5dec` is set.
- **`weatherPos` (`0x3da`)** carries no attributes and names no effect, so it is
  not what places rain or snow. What it does is open; see
  [weatherpos.md](../../docs/ghidra/functions/psp-pulse-usa/weatherpos.md).
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

## Next Steps

- Port the weather: a camera-attached env instance for `WO_RAIN`/`WO_SNOW`, the
  `ScreenPsys` lens effect, the inside/outside switch and the wind modifier.
  Then compare on Fort Gale and Outpost 7 against PPSSPP. Start at
  `Weather_Update` (`0x088f1e58`) and `FUN_08912930`, the camera-matrix source.
- `NODAMAGE`: catch a live `damaged == 0` hit (`ShipCollisionFx_Trigger`
  `0x089246b4`, `a2`). A craft dropped onto magstrip or the floor at speed is the
  candidate. Then add a reporting-only floor-impact field to
  `oag_physics::wall::WallResponse` without touching `reacts()`.
- Read the welder's streak law (render class `Streak`, `Capped`) against one
  live particle.
- Do not fire any effect on a guess (the do-not-invent rule in `CLAUDE.md`).
