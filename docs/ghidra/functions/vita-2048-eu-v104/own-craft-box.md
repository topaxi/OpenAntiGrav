# 2048: no craft box in a projectile's probe

Question carried over from Pulse
([psp-pulse-usa/rocket-visuals.md](../psp-pulse-usa/rocket-visuals.md), "The own-craft box is a
rotated frame"): does the firing craft's own box hide the track from a projectile's first probes?

**Answer: no. 2048's projectile query tests track meshes only, so no craft box, the firer's or
another's, can answer a probe; a projectile probes the track from its first update.** Confidence
**85**, static, read 2026-11-13 on `/2048/eboot-vita-2048-eu-v104.elf`.

## The query

`Collision_SweepSegment` (`0x8118b8a0`, was `FUN_8118b8a0`):

```c
world  = g_collision_world;             // DAT_8151faec, set by Collision_Construct
result = 0xe;                           // open air
for (i = 0; i < world->count; i++) {    // +0x43c
    shape = world->objects[i];          // +0x3c, 0x100 slots of 4 bytes
    if (shape->type == 5 &&             // +0x04
        Collision_RaycastMeshShape(shape->mesh /* +0x44 */, from, to, &hit, 0, flag, 0)) {
        *out_point = hit.point; *out_normal = hit.normal; *to = hit.point;
        return hit.kind;                // the triangle's material byte
    }
}
return 0xe;
```

`0x8118b818` (`Collision_RaycastMeshes`) is the same walk returning a bool. `0x8118b6a8`, the only
test either calls, is a mesh test and nothing else: it reads the triangle's three `u16` indices into
a `0x10`-stride vertex array and the kind from a per-triangle byte table (`shape->table[4] + tri`).
A box has no index buffer, so type `5` cannot be a box. The world is the one `Collision_Construct`
(`0x81187abe`, the sole writer of the `"Backend/General/Collision/Collision.cpp"` tag) builds:
`objects` at `+0x3c`, `count` at `+0x43c`, the global at `0x8151faec`.

## Which flyers use it

Its callers, by the effect names they push: `0x81204248` (`WO_ROCKET_EXPLO_TRACK`, the Rocket),
`0x811f5ef6` (`MISSILEEXPWALL`, `WO_MISSILE_BOUNCE`, the Missile), `0x8120058c`
(`WO_CANNON_SPARKS`, the Cannon), and `0x811fc650`, a flyer with the `50.0` fall arm that pushes no
named effect (the Plasma by elimination, not attributed). Each makes the probe and the travel query
through it. `0x811ddde4`, `0x811e25fe`, `0x811e6bae` and `0x8117cf22` also call it and were not
attributed. No other projectile query was found.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x8118b8a0` | `Collision_SweepSegment` | 82 | the listing above; default `0xe`, Omega's `0x01257f00` |
| `0x8118b818` | `Collision_RaycastMeshes` | 78 | the same type-`5` walk, bool result |
| `0x8118b6a8` | `Collision_RaycastMeshShape` | 75 | triangle indices, the material byte, the hit record |
| `0x81187abe` | `Collision_Construct` | 80 | stores the `Collision.cpp` tag, the `0x100` object slots, the count `+0x43c`, the global |

## Cross-title (2048 and Omega)

**Checked, applies both ways**: Omega's `Collision_SweepSegment` (`0x01257f00`,
[ps4-omega-eu/own-craft-box.md](../ps4-omega-eu/own-craft-box.md)) is the same walk on its own
binary (type `5` only, default `0xe`), read independently. Against HD, 2048's engine lineage:
**differs**. HD's query reaches a box branch ([ps3-hdfury-eu/own-craft-box.md](../ps3-hdfury-eu/own-craft-box.md)).
