# Pads

`Speedup Pad` (`0x3bd`) and `Weapon Pad` (`0x3be`): the plates on the track
surface that boost a craft or hand it a pickup.

Decoded by `crates/vex/src/pads.rs`, drawn by `oag_mesh::mesh::build_pads`,
asserted against the disc by `crates/vex/tests/pads_ground_truth.rs`. The
runtime side - bind, containment, the swept test - is
[`docs/ghidra/functions/psp-pulse-usa/pads.md`](../ghidra/functions/psp-pulse-usa/pads.md);
the force is in [`engine.md`](../ghidra/functions/psp-pulse-usa/engine.md).

## Both classes are drawn

`Weapon Pad` `0x3be` is a `Mesh` subclass exactly as `Speedup Pad` `0x3bd` is, so
its geometry ships inside the track file and drawing it is the mesh builder
pointed at a different class id - `oag_mesh::mesh::build_weapon_pads`. On
`16_Track` that is **738 triangles across 27 materials** against the speed pads'
612 and 51.

Kept as a separate model and a separate draw rather than merged into one buffer,
because the two are separate *gameplay* objects: a speed pad pushes and a weapon
pad hands something out, and a merged buffer could not later show one without the
other.

**A weapon pad hands out a pickup** as of 2026-08-11, in the one mode that arms
them - see [pickups](../gameplay/pickups.md) for the trigger, the draw and the
recovered-versus-ours split. Its trigger volumes are carried on
`oag_raceplay::Setup::weapon_pads` and consumed by `Race::test_weapon_pads`.

`crates/game/tests/race_ground_truth.rs::the_weapon_pads_are_drawn_where_they_trigger`
is what makes "drawn where they trigger" a fact rather than a hope: the geometry
comes through the **mesh** path and the volumes through the **payload** path,
neither knows about the other, and the test asserts every volume has drawn
geometry inside it. That is the check that catches a transform applied on one
side and not the other - which is the mistake this format invites, since a pad's
payload is a mesh payload and its placement comes from the parent chain rather
than from itself.

## A pad is a `Mesh`

The single fact everything else follows from. The class `0x3bd` bind handler
calls the `Mesh` bind first and only then reads its own fields, so:

- a pad's payload **is** a mesh payload, with the same header, the same
  bounding-box pair at `+0x10`/`+0x20` and the same stride-`0x14` material array
  at `+0x30` that [`vex.md`](vex.md) already documents;
- a pad **carries its own geometry**, in the track file, and draws on the same
  pipeline as any other mesh;
- the pad's trigger volume **is** the mesh's bounding box, expanded vertically.

Confidence **85**. That also settles a long-standing note in `HANDOVER.md` that
the `Data\Pads\` geometry on the disc "is never loaded": it is never loaded
because it is never needed - the pads a track draws are inside the track file.

```text
payload +0x10  {x, y, z, _}   bounding-box minimum, pad-local space
payload +0x20  {x, y, z, _}   bounding-box maximum, pad-local space
payload +0x30  ...            the mesh's material array, as any mesh's
```

At bind the box is expanded: `min.y -= 2.0`, `max.y += 8.0`. The expansion is
asymmetric and `+y` is world up (see [`track.md`](track.md)), so a pad is
entered from above - a plate about 0.15 units thick becomes a volume about 10
units tall.

## The box is local; the placement is the transform chain

All nine of `01_Track`'s `Speedup Pad` nodes share **one byte-identical
payload**, under nine different `Transform` parents. So nothing about where a pad
is lives in its payload, and containment has to run in pad-local space rather
than against a world-space box - pads are rotated to follow the track.

Row 2 of a pad's world matrix - its local `+Z` - is the direction the boost
pushes along.

## What the disc authors

Across the 40 PSP track files (`data/images/pulse-psp-usa.chd`):

| | Count |
| --- | --- |
| `Speedup Pad` nodes | 544 |
| `Weapon Pad` nodes | 218 |
| `01_Track` speedup / weapon | 9 / 7 |
| Distinct payload lengths | 5 (1136, 1312, 1696, 1728, 1760) |
| Box width across all 544 | **9.55 to 9.71 units** |

That width range is the load-bearing number. The bind copies the box floats raw
and then adds literal `-2.0` and `+8.0` in world units, while the `Mesh` base
divides the same floats by a quantisation scale - so the literals only mean
anything if a pad mesh's scale is `1.0`. A per-mesh scale would put these orders
of magnitude apart; instead every pad on every track is one authored size, about
a craft's width. Confidence **85**, capped there by this being agreement with
shipped data rather than a runtime trace.

## Not determined

- ~~**What writes `pad+0x1a0`**~~ - **answered.** `WeaponPads_TestCraft`
  (`0x0888727c`) stamps it with `<WeaponPad refresh_time>` and
  `WeaponPad_UpdateRefreshTimer` (`0x0892c034`) counts it down, at confidence 90:
  see [the runtime page](../ghidra/functions/psp-pulse-usa/pads.md). It is a
  **debounce** rather than the pickup-respawn timer this entry guessed at -
  `0.55` seconds for every speed class, less than a craft takes to clear a pad.
  Parsed by `oag_tables::handling::WeaponPad`.
- ~~**Whether `Weapon Pad` shares more than the payload**~~ - the geometry, the
  volumes and the trigger are all shared with `Speedup Pad`; what differs is only
  what happens on a hit. See [pickups](../gameplay/pickups.md).
- ~~**The pad's own colour cycle is still unimplemented.**~~ **Implemented
  2026-08-17** - `oag_render::weapon_pad` and
  `oag_raceplay::drawable::Drawable::tint_weapon_pads`. See
  [`docs/ghidra/functions/psp-pulse-usa/pads.md`](../ghidra/functions/psp-pulse-usa/pads.md#retired-pad0x1a0s-writer-is-weaponpad-refresh_time-exactly-as-guessed)
  for the recovered grey, the `WeaponPad_ColourKeyframes` table and the one
  simplification (every ready pad shares one phase rather than each resuming
  its own).
- **Nothing here has been verified under an emulator.** This is static reading
  plus agreement with shipped data, which is what caps the scores at 94.
