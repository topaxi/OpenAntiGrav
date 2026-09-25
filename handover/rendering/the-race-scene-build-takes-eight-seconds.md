---
categories: [rendering, tooling]
---

# The race scene build no longer freezes the loading screen, but still takes seven to eight seconds

2026-09-25. The loading screen froze for eight seconds before every race
because the race scene was built inside one frame of it. That build now runs
on a `race-build` thread and the loading screen animates through it (user
confirmed live on the desktop). Measurements, the before/after table and the
worker layout are in
[race-load-transition.md](../../docs/architecture/race-load-transition.md).

## Open

- **The Steam Deck is unmeasured.** It is where the report came from. After
  `just deploy-deck`, from a Desktop Mode terminal:
  `~/Desktop/OpenAntiGrav-x86_64-portable.AppImage ~/.local/share/oag/images/pulse-psp-eu.chd --no-audio --measure-race-load 2`,
  and again with `~/.local/share/oag/images/hdfury-ps3-eu-dec.iso`. Record the
  printed tables on the doc page.
- **The build is 7.2 to 8.1 s in a release build on a desktop**, and it is the
  whole of the wait now. A perf profile is almost all naga: `mesh_render::build`
  calls `create_shader_module` on `mesh.wgsl` for every model and then builds
  that model's pipelines, each re-translating the module with its own override
  constants. Two candidate fixes: share one `ShaderModule` per WGSL source per
  device, and cache pipelines by their full key (format, sample count, depth
  role, blend, glow mask, velocity, override constants), so eight craft built
  from one shader stop building eight identical pipelines.
- **`RaceStage::warm_up` warms `Shadows::Blob` and `MotionBlur::Off` only.**
  A player on shadow maps or motion blur compiles those pipelines on the first
  frame that uses them. No spike showed at the default settings; measure with
  those settings on before acting.
- **Escaping a race back to the menus costs about 80 ms on the frame thread**
  (`escape` plus `open_menus`), seen as the interval before the second run's
  first loading frame. It is the race-to-menu direction, outside this thread's
  transition.

## Next Steps

1. Run the Deck command above and put its tables on the doc page.
2. Profile `race::Scene::new` per drawable (`mesh_render::build` call count and
   time) before choosing between the shader-module share and a pipeline cache.
