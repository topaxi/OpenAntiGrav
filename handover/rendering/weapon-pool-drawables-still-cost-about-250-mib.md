# Weapon pool drawables still cost about 250 MiB

2026-09-30, `weapon-pool-memory`. The 8 GiB this thread was opened for is
**closed**, and it was never the pools: the `--screenshot` path built its
scene with no `mesh_render::BuildCacheScope` open, so each of the ~1,200
`Drawable::new` calls compiled its own pipelines (9,859 requests, 53 distinct)
at about 7 MiB resident each. `race::Scene::new` now opens the scope itself,
so no caller can forget it; `race::tests::scene_build` fails if a pool of
identical drawables stops sharing pipelines. The windowed launch (`main::stage`)
always had it, and peaked at 1.6 GiB on lavapipe, not 8.

Peak RSS, headless `--race --ticks 200` (debug), before / after: Pulse 8,453 /
about 540 MiB, HD 8,237 / about 890, Pure 1,282 / 350, 2048 728 / 685.
Race stills with 60-128 projectiles in flight (rocket, mine, cannon, bomb on
Pulse and HD) are byte-identical, and so are two captures of the same tree.

## Open

- What is left per `Drawable::new` is a median 196 kB (mean 237 kB), about
  250 MiB over the 1,152 weapon drawables of a Pulse race: a fog buffer, the
  zone visualiser texture, the animation buffers and their bind groups, one
  texture upload per model. A pool slot needs none of it but a uniform buffer,
  so one shared `Drawable` per weapon model with a uniform per slot would take
  that back and shorten `Scene::new`. Optional: it is roughly half a race's
  resident memory in a capture, not the 8 GiB it looked like.
- 2048 has no weapon pools and gained only 43 MiB from the same change.

## Next Steps

1. Decide whether 250 MiB is worth a shared-drawable refactor of
   `race/scene/weapon_models.rs`; if so, keep the draw order and blend of
   today, and re-run the byte-identical weapon stills as the proof.
