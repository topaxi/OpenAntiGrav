# An own craft format, with the originals and BallisticNG as importers into it

2026-10-12, the maintainer asked for a format for custom craft, the twin of the own
track format (`oag-trackfile`, [ADR-0063](../../docs/architecture/adr/0063-the-own-track-format-is-glb-plus-a-toml-manifest.md),
[track-file.md](../../docs/formats/track-file.md)). **Nothing below is built**; it is a
design to check against the code before building, and the open questions at the end
are the maintainer's.

## Why

- A craft today exists only as a title's own data: `oag_livery` builds a race's hulls
  from archive entries, and the handling comes from the title's tables
  (`oag_physics::params::Handling`). A modder has no way in.
- BallisticNG's `.csf` (a bare UnityFS bundle, 47 of 47 on this host) is one source
  of craft, not the format craft are authored in, as `.wtrk` is for tracks.
- One add-on (Blender) and one picker can then serve tracks and craft alike.

## Shape: the track format's, for a craft

`<name>.craft.toml` (manifest) beside `<name>.craft.glb` (geometry), versioned
`major.minor` and refusing an unknown major by name, as the track file does.

**The glb** carries what is drawn and where things attach. Nodes are found by an
`extras.oag_kind` tag, never by name:

| `oag_kind` | Needed | Maps to (`oag_livery` today) |
| --- | --- | --- |
| `hull` | yes | `hull` |
| `wreck` | no | `wreck` (none: the hull is drawn, labelled in the load report) |
| `shield` | no | `shield` (none: the host title's shield) |
| `boost`, `flare`, `shine` | no | `boost`, `flare`, `shine` |
| `airbrake_left`, `airbrake_right` | no | flaps swinging about their own local X, as HD/2048/Omega/Pure author them |
| empty `engine_flare` | no | `nozzle` (exhaust and flare anchor) |
| empties `collision_fx` (n) | no | `collision_fx` |
| empties `cannon_flash_left/right`, `arc_anchor`, `absorb` (n) | no | the matching fields |

Materials are glTF's own (base colour texture, emissive texture and factor); the
renderer maps them onto the host title's lighting the way it maps a title's own hull.

**The manifest** carries identity and the numbers the simulation reads:

```toml
[format]
major = 1
minor = 0

[craft]
name = "Cliffhawk"
team = "Cliffhawk"       # shown in pickers and the HUD
author = "..."
colours = ["#d02020", "#202020"]

# Either borrow a title's handling...
[handling]
based_on = "pulse:feisar"
# ...and override single numbers, all four classes alike:
[handling.override]
engine.thrust = 1.05

# ...or author every class in full (`[handling.venom]` ... `[handling.phantom]`),
# one table per field of `oag_physics::params::Handling`.
```

Determinism: the track file keeps every simulation float in the glb so it is
bit-exact. Handling numbers here are typed by a person, so TOML is the honest home;
each is parsed as `f64` and rounded to `f32` once at load, which is exact for any
value written as the shortest `f32` representation. That rule goes in the format page
and in a ground truth that round-trips every title's handling tables through TOML.

## Importers and exporters

1. **Disc exporter, local only** (like `export_track`): every team of every title
   written out and read back equal, the format's ground truth. Never committed.
2. **BallisticNG `.csf` importer** on top of `oag-ballisticng`: mesh, textures and
   locators read; BallisticNG's stats mapped onto `Handling` as **chosen, not
   measured** constants (as `.wtrk`'s scale 8.7 is), or `based_on` a host team when a
   stat has no counterpart.
3. **The Blender add-on** (queued for tracks) grows a craft panel: tag nodes, place
   empties, write both files.

## Where it lives

A sibling crate `oag-craftfile` beside `oag-trackfile`, sharing the glb read/write
helpers by moving them into whichever of the two lands second (no compat re-exports).
Same dependency rule as the track file: forbidden to gameplay crates; the handling
it yields is plain `Handling` data.

## Open questions (the maintainer's)

- Per-class handling in full, or one set the classes scale? Pulse authors four
  tables per team; a modder may only want one.
- May opponents fly custom craft (race box, remix), or only the player?
- Engine sound: borrow the host title's, or ship an audio file in the craft?
- ~~Whether a custom craft is offered in the campaign~~ **Decided 2026-10-12 (maintainer):
  the original campaigns stay vanilla** (their own craft, tracks and the original AI).
  A custom campaign, this project's own or a mod's, is where custom craft, tracks and
  controllers belong, with more freedom.

## Next Steps

1. Read `.csf` far enough to know what BallisticNG ships per craft (mesh, textures,
   stats, locators): a census over the 47 on this host, before fixing the manifest.
2. ADR for the format, then `oag-craftfile` with the disc exporter as ground truth.
3. `--craft-file <manifest>` on `oag-game`, then a picker row beside the track one.
