# Handling stats

**Status: understood.** Ship handling is **data, not code**. Every tunable lives
in `Data\Ships\<Team>\handlingstats.xml` inside `Data.wad`, one file per team,
with a block per speed class.

That is the best possible news for M4: the physics code defines the *shape* of
the model, and these files supply every constant.

## Reading them

```sh
oag-wad cat data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad \
    'Data\Ships\Feisar\handlingstats.xml' --expand
```

Stored as [shortened XML](fexml.md), so `--expand` is needed.

Eight files were located by hashing candidate names, one per playable team:
`AG_Systems`, `Assegai`, `EGX`, `Feisar`, `Goteki`, `Piranha`, `Qirex`,
`Triakis`. Each is about 2.7 KiB.

## No values are reproduced here

Per [ADR-0006](../architecture/adr/0006-no-copyrighted-content.md), this page
documents the **schema**: element names, attribute names, and which code
consumes each. The values are the game's design data and are read at runtime
from the user's own disc.

The distinction matters and is worth stating: a field name is a description of
the format, a tuning table is the content itself.

## Schema

```xml
<Handling>
  <Stats team="...">
    <InternalCamera fov headtilt height length pitch/>
    <BackwardCamera fov headtilt height length pitch/>
    <BonnetCamera fov height length pitch/>
    <ExternalCameraFar   fov lookat_height lookat_length
                         pos_height pos_length spring_horiz spring_vert/>
    <ExternalCameraClose fov lookat_height lookat_length
                         pos_height pos_length spring_horiz spring_vert/>
    <AirbrakeGraphics amount down_speed up_speed/>
    <Misc height length shield easyshield width weight_distribution/>
    <FE speed thrust handling shield/>

    <Class name="VENOM|FLASH|RAPIER|PHANTOM">
      <Engine   accelcap amount falloff gain turbo/>
      <Brakes   amount falloff gain/>
      <Turning  amount falloff gain/>
      <Airbrake amount drag falloff gain turn slidegrip sideshift/>
      <Antigrav grip_air grip_ground landing_rebound
                rebound rebound_jump_time ride_height/>
      <Physical flight_gravity mass normal_gravity track_gravity/>
      <pitch    pitch_air pitch_ground pitch_damping antigrav_height_adjust/>
    </Class>
  </Stats>
</Handling>
```

`<FE>` holds the four bar values shown on the ship-select screen. They are
presentation only and need not agree with the physics.

`easyshield` alongside `shield` implies a difficulty-scaled shield pool.

## Cross-check against the code

The names here **independently confirm** the parameter block recovered from the
binary. The Airbrake section was read from disassembly as
`{gain +0xd8, falloff +0xdc, amount +0xe0, turn +0xe4, drag +0xe8,
slidegrip +0xec, sideshift +0xf0}`, and the XML carries exactly those seven
names. Likewise `Physical` matches the recovered `normal_gravity +0xf8`,
`flight_gravity +0xfc`, `track_gravity +0x100`.

Two independent sources agreeing on a set of names is much stronger than either
alone, and it retires three Airbrake names that the static reading had left
unresolved.

See [physics](../physics/README.md) for how each is consumed.

## Related files

Located alongside, same naming pattern, not yet decoded:

| Template | Notes |
| --- | --- |
| `Data\Ships\<Team>\Ship.vex` | The [model](vex.md) |
| `Data\Ships\<Team>\ship_FE.vex` | Front-end preview model |
| `Data\Ships\<Team>\<Team>shield.vex` | Shield effect |
| `Data\Ships\<Team>\ship_eliminator.dat` | Unknown |
| `Data\Ships\<Team>\Definition.xml` | Unknown |

## Open questions

- **Units.** `amount="412"` for Engine and `10` for Airbrake are clearly
  different scales. The consuming code applies its own factors (the airbrake
  slide term multiplies by `0.01` and then `0.001`), so units only make sense
  read together with [physics](../physics/README.md).
- Whether the PS2 release ships the same values. Its archives are compressed but
  now readable, so this is a cheap check and would settle whether
  [ADR-0004](../architecture/adr/0004-asset-pipeline.md)'s "gameplay is
  identical across asset sets" holds.
- Whether tracks carry handling modifiers of their own.
