---
categories: [rendering, tooling]
---

# The race scene build no longer freezes the loading screen, and on a desktop it no longer takes seven to eight seconds either

2026-09-25. The loading screen froze for eight seconds before every race
because the race scene was built inside one frame of it. That build now runs
on a `race-build` thread and the loading screen animates through it (user
confirmed live on the desktop). The build itself was then found to spend
almost all of that time re-parsing and re-translating one WGSL shader once
per drawable; sharing the module and caching pipelines by their descriptor
cut it from 7.2-8.1 s to 195-372 ms on a desktop, measured on Pulse PSP, PS2
and Wipeout HD Fury. Measurements, the before/after tables and the worker
layout are in
[race-load-transition.md](../../docs/architecture/race-load-transition.md).

## Open

- **The Steam Deck is unmeasured.** It is where the report came from, and it
  is the only thing left open on this thread - the desktop build time below
  is fixed. After `just deploy-deck`, from a Desktop Mode terminal:
  `~/Desktop/OpenAntiGrav-x86_64-portable.AppImage ~/.local/share/oag/images/pulse-psp-eu.chd --no-audio --measure-race-load 2`,
  and again with `~/.local/share/oag/images/hdfury-ps3-eu-dec.iso`. Record the
  printed tables on the doc page; delete this thread once they land, the
  desktop side of it is done.
- **The build was 7.2-8.1 s and is now 195-372 ms - 20-38x, measured on Pulse
  PSP, PS2 and Wipeout HD Fury.** The naga profile named the cause right:
  `mesh_render::build` called `create_shader_module` on `mesh.wgsl` and built
  eight-to-thirteen render pipelines once per drawable, re-translating the
  same shader with the same override constants a sibling drawable had already
  asked for. `oag_render::mesh_render::pipeline_cache` now shares the module
  and caches a pipeline by everything that can make one differ from another,
  opened for the one `race::Scene::new` call in `Stage::build_race_stage` - on
  Pulse PSP that turned 1,197 shader asks and 9,859 pipeline asks into 53
  distinct pipelines built once each. Byte-identical `--race --screenshot`
  PNGs before and after on all three titles; see
  `docs/architecture/race-load-transition.md`'s own "shader module and
  pipeline cache" section for the full numbers and the mechanism.
- **`RaceStage::warm_up` warms `Shadows::Blob` and `MotionBlur::Off` only.**
  A player on shadow maps or motion blur compiles those pipelines on the first
  frame that uses them. No spike showed at the default settings; measure with
  those settings on before acting.
- **Escaping a race back to the menus costs about 80 ms on the frame thread**
  (`escape` plus `open_menus`), seen as the interval before the second run's
  first loading frame. It is the race-to-menu direction, outside this thread's
  transition.

## Next Steps

1. Run the Deck command above and put its tables on the doc page, then delete
   this thread - everything else on it is done.
