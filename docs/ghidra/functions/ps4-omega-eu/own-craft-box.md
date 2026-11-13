# Omega: no craft box in a projectile's probe

Question carried over from Pulse
([psp-pulse-usa/rocket-visuals.md](../psp-pulse-usa/rocket-visuals.md), "The own-craft box is a
rotated frame"): does the firing craft's own box hide the track from a projectile's first probes?

**Answer: no. Omega's projectile query tests track meshes only; a projectile probes the track from
its first update.** Confidence **85**, static, read 2026-11-13 on `/omega/eboot-ps4-omega-eu.bin`
(clean x86-64 decompile).

## The query

`Collision_SweepSegment` (`0x01257f00`, was `FUN_01257f00`; [plasma.md](plasma.md) calls it
`Collide`):

```c
for (i = 0; i < world->count; i++) {         // +0x878
    obj = world->objects[i];                 // +0x78, 8-byte slots
    if (obj->type == 5) {                    // +0x08
        mesh = obj->shape;                   // +0x60
        if (FUN_0125a760(1.0f, mesh, mesh->tree /* +0x30 */, from, to - from, &point, &normal, &tri)) {
            // recompute the triangle's plane from its three u16 indices into the vertex array,
            // write point, normal, and the kind = the triangle's material byte
            return 1;
        }
    }
}
return 0;                                     // the caller reads no hit as open air (0xe)
```

The hit record is filled from the triangle's index buffer and per-triangle byte table, so type `5`
is a mesh and a box cannot be one. Shape types in this binary are box `0`, sphere `2`, mesh `5`
([mp-time-trial-collision.md](mp-time-trial-collision.md)).

## Which flyers use it

`PlasmaManager_Update` (`0x0135e140`), `0x013691b0` (`WO_ROCKET_EXPLO_TRACK`: the Rocket's update),
`0x01351930` (`MISSILEEXPWALL`, `WO_MISSILE_BOUNCE`: the Missile's), `0x013644c0`
(`WO_CANNON_SPARKS`: the Cannon's) and `0x0133a740` (`DetonatorMine`), with eleven other callers not
attributed. No projectile query that includes boxes was found.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x01257f00` | `Collision_SweepSegment` | 85 | the listing above |

## Cross-title (Omega and 2048)

**Checked, applies both ways**: 2048's `Collision_SweepSegment` (`0x8118b8a0`,
[vita-2048-eu-v104/own-craft-box.md](../vita-2048-eu-v104/own-craft-box.md)) is the same type-`5`
walk with the same `0xe` open-air default, read on its own binary. HD **differs** (its query reaches
a box, [ps3-hdfury-eu/own-craft-box.md](../ps3-hdfury-eu/own-craft-box.md)), although Omega's front
end and HD mode are HD's.
