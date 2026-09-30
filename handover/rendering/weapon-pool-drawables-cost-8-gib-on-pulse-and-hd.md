# Weapon pool drawables cost about 8 GiB on Pulse and HD

2026-09-30, found by `lane/omega-memory` while measuring Omega; not fixed
there because `race/scene/weapon_models.rs` is the weapons lane's.

Peak RSS of a headless `--race` (debug build, 200 ticks): **Pulse 8,450 MiB,
HD 8,266 MiB**, Pure 1,281, 2048 719, Omega now 3,360. None of it is asset
data: Pulse's whole disc is 200 MiB, and its texture CPU copies are 0 MiB
after upload.

The growth is linear in `Drawable::new` calls. `weapon_models::build` makes
`oag_gameplay::projectile::MAX_PROJECTILES` (128) full drawables per weapon
model - Pulse 9 models = 1,152, HD 7 = 896 - and RSS climbs about 7 MiB per
drawable (RSS 645 MiB after the first to 8,155 MiB at the last, HD). The
render pipelines are already shared through `pipeline_cache::Scope`, so it is
something else `Drawable::new` or `mesh_render::build` costs per call.

## Open

- What the 7 MiB is. Candidates not yet tested: the per-drawable
  `zone_vis`/fog/anim/node/emissive buffers and their bind groups, the
  vertex/index buffer allocations (each `create_buffer` on some drivers takes
  its own host-visible block), and `Model::clone()` per slot.
- Whether a pool slot needs a drawable of its own: a projectile pool could
  share one vertex/index buffer and one set of bind groups and differ only in
  the per-instance matrix `write_weapon_models` already writes.

## Next Steps

1. Log RSS before and after one `Drawable::new` for a Rocket, then bisect the
   buffers and bind groups it creates (30 minutes with the RSS-per-log-line
   trick: a `.format` closure reading `/proc/self/statm`).
2. Share what does not vary per slot; the target is Pulse under 1 GiB.
