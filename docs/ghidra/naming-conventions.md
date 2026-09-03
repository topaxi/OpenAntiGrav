# Ghidra naming conventions

The rules and their rationale are in
[ADR-0005](../architecture/adr/0005-ghidra-conventions.md). This is the
quick reference.

## Functions

`Subsystem_VerbNoun`, PascalCase.

```
Ship_UpdateSteering
Wad_OpenEntry
Race_ComputeLapTime
Render_SubmitShipMesh
Audio_PlayOneShot
```

## Confidence in the name

| Confidence | Name |
| --- | --- |
| 70 and above | `Ship_UpdateSteering` |
| 50 to 69 | `Ship_UpdateSteering_q` |
| Below 50 | **Do not rename.** Leave `FUN_08831af0`. |

The `_q` suffix makes uncertainty visible at every call site, not just on the
documentation page.

## Subsystem prefixes

Established as analysis proceeds. Everything below `Race_` is **in use**; the
rest are reserved for areas not yet analysed.

| Prefix | Area |
| --- | --- |
| `Game_` | Main loop, bootstrap, global lifecycle |
| `StateMachine_` | The string-keyed state machine itself |
| `InGame_`, `Demo_`, `Loading_`, `Utility_` | Individual states and their loops |
| `Input_` | Controller reading and mapping |
| `Wad_` | Archive access |
| `Vfs_` | Device and path resolution beneath the archives |
| `Lzss_` | The LZSS bitstream decoder |
| `Resource_` | Resource loading and lifetime |
| `Vex_` | `.vex` model loading and node trees |
| `Mesh_`, `Texture_` | Geometry and texture nodes within a model |
| `Gfx_` | The render manager |
| `Gu_` | The PSP graphics-unit wrappers, mirroring `sceGu*` |
| `Collision_`, `CollisionMesh_`, `CollisionNode_` | Queries, per-mesh data, node parsing |
| `Sap_` | The sweep-and-prune broadphase |
| `AiTrack_` | The track spline graph, named for its `"AI track data"` resource |
| `World_` | Track and scene assembly |
| `Ship_` | Craft state and dynamics |
| `Body_` | The rigid body under a craft: integration, force/torque accumulators, contact resolution |
| `Movie_`, `MoviePlayer_` | The XML movie widget and its `sceMpeg` wrapper |
| `Race_` | Race rules, timing, positions |
| `Weapon_` | Pickups, projectiles, damage |
| `Ai_` | Opponent behaviour |
| `Audio_` | Sound and music |
| `Ui_` | HUD and menus |
| `Mem_` | Allocation |
| `Plugin_` | Front-end plugin manifest loading (`Data\Plugins\PIxxx`) |
| `Math_` | Shared math helpers |
| `Sys_` | Platform and OS interaction |

Add a prefix when a genuinely new area appears, and add it to this table in the
same change.

`Track_` is deliberately absent: the engine's own name for the spline graph is
`"AI track data"`, so `AiTrack_` matches the binary rather than inventing a
tidier word. `Resource_` is spelled out rather than abbreviated to `Res_`.

## Library and import symbols

Recovered standard-library and PSP system functions keep their **real** names:
`strlen`, `_ctype_`, `zlib_uncompress`, `sceCtrlPeekBufferPositive`. The scheme
above is for the game's own code. A name that can be checked against an external
definition is worth more than a consistent one, and pretending `strlen` is
`Sys_StringLength` would hide that it is checkable at all.

## Structures

PascalCase, no prefix: `Ship`, `PadState`, `WadEntry`, `TrackSection`.

Fields are `snake_case`. Unknown fields are named for their offset and size so
they are obviously unknown:

```c
typedef struct Ship {
    Vec3  position;        // +0x00
    Vec3  velocity;        // +0x0c
    float steering_angle;  // +0x1c
    u32   unk_0x20;        // +0x20, unknown
    u8    unk_0x24[12];    // +0x24, unknown
} Ship;
```

`unk_0x20` is better than `field20` or `padding`: it states the offset and
admits ignorance. Calling something `padding` is a claim, and usually a wrong
one.

Each recovered structure gets a page under `docs/ghidra/structures/` with the
evidence for each field.

## Enums

PascalCase type, `SCREAMING_SNAKE_CASE` values, prefixed with the type:

```c
typedef enum RacePhase {
    RACE_PHASE_COUNTDOWN = 0,
    RACE_PHASE_RUNNING   = 1,
    RACE_PHASE_FINISHED  = 2,
} RacePhase;
```

## Globals

`g_` prefix, `snake_case`: `g_current_race`, `g_frame_counter`.

## Labels and comments in Ghidra

Comment freely; comments in the database are a working aid and cost nothing. But
the database is not committed and cannot be reviewed, so **anything worth
keeping goes in `docs/ghidra/`**. If you find yourself writing a long comment in
Ghidra, that is the signal to write the page instead.
