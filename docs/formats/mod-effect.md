# A mod's particle effect (`effect.tosd`)

**Status: built (2026-10-30).** Our own format, **chosen, not measured**; the laws it plays
(emission, ageing, curves, blend) are the disc `.pob` effects' own, through the same
`oag_fx::psys::System`. Schema: `schemas/v1/effect.tosd` (ADR-0064). Reader:
`oag_fx::psys::parametric`. A mod declares its effects in `oag-mod.toml`:

```toml
[effects]
discharge = "fx/discharge.toml"
```

A module fires one with `fire-effect` and bit 15 of the argument set (`Command::FireModEffect`,
SDK `reply.fire_mod_effect(&craft, index)`): the index is the effect's place in `[effects]`
in name order. It plays at the craft's nozzle (the hull's back end along the body's heading),
its cone axis `rear` (away from the nose) or `up`. Render-side only: never hashed, never in
the simulation; a race firing mod effects hashes like one firing none
(`tests::extensions::on::a_mod_effect_is_queued_under_its_mod_and_a_race_firing_one_hashes_like_one_firing_none`).

## Shape

```toml
toml-schema = { location = ".../v1/effect.tosd", version = "1.0.0" }
axis = "rear"            # or "up"

[[emitter]]              # at most 64; every emitter starts with the effect
name = "core"
duration = 3             # ticks emitting (looping = true: until stopped)
interval = 1             # ticks between emissions, or [min, max]
per-emission = [4, 6]    # particles, or one number
lifetime = [9, 3]        # centre, spread (ticks)
speed = [0.7, 0.4]       # centre, spread (world units per tick)
spread = 28              # cone half angle, degrees
drag = 0.93              # per-tick velocity factor
gravity = 0.0
live-cap = 64
blend = "additive"       # or "alpha-over"
size = [[0.0, 0.55], [1.0, 0.1]]   # half-size over life: a number or [age, value] keys
alpha = [[0.0, 1.0], [1.0, 0.0]]   # 0..1 over life
colour = [[0.0, "#ff6a48"], [1.0, "#600008"]]  # "#rrggbb" or stops over life
texture = "fx/glow.png"  # optional: a PNG of the mod, or a disc texture by name:
#   texture = { entry = "Data\\Environments\\16_Track\\track.vex", name = "energy_GLOW.tga" }
# none draws the soft round default
```

Ranges are the loader's (`parametric.rs`); the schema carries the shape. A file that fails to
read, parse or validate **draws nothing**, the load report names it
(`mod <id>: effect <name> left out: <why>; draws nothing`) and the race runs
(`a_mods_effect_files_are_loaded_and_a_broken_one_draws_nothing_and_is_named`).
Mod effects join the race's effect library after every title effect, so no title sprite moves.

A disc texture is found by the lookup a track file's host references use
(`oag_raceplay::foreign::art::host_texture`: a `Texture` node of a circuit on the player's disc, by
name); one that does not resolve, or is not plain RGBA8, refuses the effect by name
(`the_discharge_finds_the_discs_energy_glow_by_name`, Pulse PSP EU).

## Anchor

A `fire-effect`, the title's or the mod's, plays at the hull's back end along the body's heading
(`position - forward * length / 2`). The title's effects used to be placed through the hull
model's `+Z`, which put them near the nose (seen 2026-10-30 on Pulse PSP's Energy Turbo); both
now share the body anchor. The orange sparks at the nose of a craft scraping a wall are the
vanilla wall-scrape sparks (a run without the mod shows them).

## Open

- Only billboard emitters, cone directions and point spawns: no child emitters, streaks,
  spawn shapes or sprite atlases. A disc `.pob` effect is not yet describable in full.
- The Blender preview is approximate (`docs/tools/blender.md`).
- Effects are left where they were emitted: a craft at 500 km/h outruns its own sparks, so a
  burst reads for a few ticks near the nozzle. Fire often (Energy Turbo fires every 2 ticks).
