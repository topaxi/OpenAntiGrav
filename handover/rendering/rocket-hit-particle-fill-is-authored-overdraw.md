---
status: blocked-hardware
titles: [hd]
updated: 2026-11-25
summary: "Rocket craft hit near the camera: ~40 screens of authored smoke fill left at 4K; Deck numbers and a fidelity check of the smoke owed"
categories: [tooling]
size: S (40)
difficulty: medium (40)
---

# A Rocket hit near the camera is still authored particle fill

`docs/rendering/particle-fill.md` has the reproduction, the cause (GPU fill of the CraftBlast
particles, sustained, not a hitch) and the exact cut that landed (quads with colour alpha 0
dropped before upload, 4K worst frame -15 % on the workstation, byte-identical).

## Open

- **The Deck at 3840x2160, 1920x1080 and 1280x800** (vsync off, no frame limit) was not run:
  the workstation reproduced the drop and the cause is not Deck-specific. The page's script and
  command give `gpu_scene_us` over ticks 760 to 789 with an AppImage built from main, before and
  after the cut.
- **Is our CraftBlast smoke the original's size?** At tick 774 the alpha-over smoke covers 42
  screens (31 live). If our particle sizes, counts or lifetimes run larger than HD's own, that is
  a fidelity defect that also costs fill. Measure with a matched RPCS3 pair of a craft hit at
  close range (`scripts/hd-weapon-pair.py`), same ship and track.
- **A picture-changing lever, maintainer's call**: particles into a half-resolution target and
  composited (a setting with `original` as its default), or a cap on near-plane particle area.
  Neither is byte-identical, so neither was built.

## Next Steps

1. On the Deck: build the AppImage from main and from the parent of the cut, run the page's
   command at the three sizes, and compare `gpu_scene_us` over ticks 760 to 789.
2. The RPCS3 pair above, counted with the same coverage sum (per quad, clipped area over the
   screen's) added as a throwaway probe in `Scene::upload_particles`.
