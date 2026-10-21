# HD's hull wash on boost is the original's; what is left on the hull is the Harimau livery and the `_n1` wing rims

2026-09-20, reported from play: "the light work looks amazing, but it
exaggerates the bloom". **Closed 2026-10-12 (`hd-hull-wash-2`, confidence 85):
the original washes the same panels at a boost.** Assegai gets a white housing
rim and Harimau a white inner channel. Our engine light, its hull binding and
its per-vertex law are the original's. Nothing was ported. Do not tune the
bloom and do not drop the craft binding to hide the wash.

The evidence is in
[renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md), "The
original washes `Assegai` and `Harimau` at a real boost the way ours does":

- the flare node matches our locator on `Assegai`, `Assegai_c1` and `Harimau`;
- an injected light lights the same panels on both sides, with matched pixel counts;
- in a real boosted race frame on the original, holding the player's light at its
  rest colour removes the original's own wash.

The earlier "clean" reads (2026-10-02, 2026-10-07) were the same rim, read at
player size as nozzle glow. The earlier sections of that page carry the full
history: the hull binding, the `EdgeGeom` space, the `SVC1` combine and the
"no clamp" result.

The capture tool is `scripts/rpcs3-hd-hull-light.py` (`team`, `flare`,
`inject`, `boost`). The three Time Trial states it was run on are listed in
`docs/reverse-engineering/emulator-recipes.md`.

## Open

- **2026-10-21 (`hd-hull-bay`): Assegai_c1's engine light is Fury orange now (case-blind `_c1` test). Metropia's and Sol 2's blue bay
  is open**: on Sol 2 the original does not take the hull's `SVC1` branch, so it is not the engine light; ours stays black there
  ([hd-ship-materials.md](../../docs/rendering/hd-ship-materials.md), "The engine bay: Assegai_c1's Fury light, and the two circuits
  that switch the SPU lights off").
- The drawn-hull shake (the craft offset on every HD pair) has its own thread: [hds-drawn-hull-shakes-off-its-body.md](hds-drawn-hull-shakes-off-its-body.md).
- **Landed 2026-10-20 (`hd-craft-ibl-probe`): the pale, metallic hull.** The original's IBL variant and the environment maps it
  reads are wired: the maps are not a runtime render but a blend of the circuit's baked `track.probes`, decoded and read at
  each craft (`oag_vex::probes`, `oag_raceplay::env_probes`, `shade.wesl`), the hull's ambient is `4 * irradiance[n]` and
  the materials that declare `paraboloidReflectionTex` add `4 * reflection[reflect(-V, n)] * alpha * reflectivity`. Rest-frame hull
  luma 0.18-0.30 before, 0.56-0.65 after, against the original's 0.56-0.91. Evidence, numbers, cost (+0.42 ms CPU a frame on
  the desktop, nothing on the GPU), the decode-domain choice and the cross-title answers:
  [hd-ship-materials.md](../../docs/rendering/hd-ship-materials.md), "Finding 5"; the file:
  [hd-probes.md](../../docs/formats/hd-probes.md); the engine side:
  [env-probes.md](../../docs/ghidra/functions/ps3-hdfury-eu/env-probes.md). Still open, in order:
  - **Landed 2026-10-20 (`hd-hull-nozzle`): the nozzle "jewel" is the engine flare's five shapes, drawn at rest by the original from
    the first race frame; ours hid them at a standstill (`hd_flame_resting`, removed).** The `emissive_bloom` chunks are drawn by
    both (the original rebuilds their index list per frame, 258 of 444). Evidence and residuals (ours a little larger and lower;
    the `LightsShape` light schedule unread): hd-ship-materials.md, "The nozzle jewel, resolved";
  - two tight corners blend a section pair two apart where `Probes::locate` takes neighbours (weights, 70);
  - 2048 ships 28 `.probes` in a different layout (and an authored sky paraboloid), Omega `.shprobes`: unread;
  - the Steam Deck cost is estimated, not measured; the lever is blending an opponent every second tick;
  - the Assegai wings and pods still read a little low in luma (0.29-0.40 against 0.59-0.62).
- **The white wing rims on the `_n1` skins** are not the SPU term: they stay
  with the craft binding removed. Whether the original shows them is unread.

## Next Steps

1. **Rest flame size.** Match the pose exactly (the original start is 1.8 m higher than ours) and measure the rest flame's
   projected box against the original's 45 x 45 px; then read the `LightsShape` per-frame index schedule (which lights are on).
2. **Deck cost.** One `just bench-render hd` on the Deck, with and without the probes (the build has no switch: a
   one-line `Option::filter` in `Scene::new` was the A/B), then decide whether an opponent's blend needs a cadence.
3. `_n1` rims: walk Team Selection to an `_n1` row, take a state as in
   emulator-recipes.md, and read whether the wing rims are white at rest in
   the original.
4. Metropia/Sol 2 bay: run the `0x003eb368` count longer on Sol 2, then read what the non-`SVC1` hull variant adds in the bay.
