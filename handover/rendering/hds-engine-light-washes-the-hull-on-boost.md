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

- **Landed 2026-10-20 (`hd-craft-ibl-probe`): the pale, metallic hull.** The original's IBL variant and the environment maps it
  reads are wired: the maps are not a runtime render but a blend of the circuit's baked `track.probes`, decoded and read at
  each craft (`oag_vex::probes`, `oag_raceplay::env_probes`, `shade.wesl`), the hull's ambient is `4 * irradiance[n]` and
  the materials that declare `paraboloidReflectionTex` add `4 * reflection[reflect(-V, n)] * alpha * reflectivity`. Rest-frame hull
  luma 0.18-0.30 before, 0.56-0.65 after, against the original's 0.56-0.91. Evidence, numbers, cost (+0.42 ms CPU a frame on
  the desktop, nothing on the GPU), the decode-domain choice and the cross-title answers:
  [hd-ship-materials.md](../../docs/rendering/hd-ship-materials.md), "Finding 5"; the file:
  [hd-probes.md](../../docs/formats/hd-probes.md); the engine side:
  [env-probes.md](../../docs/ghidra/functions/ps3-hdfury-eu/env-probes.md). Still open, in order:
  - the nozzle "jewel" is **not drawn at all** in ours (the earlier note said it drew black): the two `emissive_bloom` chunks
    (258 and 24 indices) of the Harimau nozzle are missing from our hull, and the material's vertex law is read
    (`TC4 = (u, v + 1.0 * time)`, additive `SRC_ALPHA, ONE`);
  - two tight corners blend a section pair two apart where `Probes::locate` takes neighbours (weights, 70);
  - 2048 ships 28 `.probes` in a different layout (and an authored sky paraboloid), Omega `.shprobes`: unread;
  - the Steam Deck cost is estimated, not measured; the lever is blending an opponent every second tick;
  - the Assegai wings and pods still read a little low in luma (0.29-0.40 against 0.59-0.62).
- **The white wing rims on the `_n1` skins** are not the SPU term: they stay
  with the craft binding removed. Whether the original shows them is unread.

## Next Steps

1. **Nozzle jewel.** Find why the two `emissive_bloom` chunks of the Harimau nozzle are not in our hull's draws
   (`OAG_ONLY_MATERIAL=emissive_bloom` shows the cockpit dots and exhaust rings, nothing in the nozzle); the shader
   side is read and the scroll is wired. Compare the chunk list of `Ship.vex` against the capture's draws 521-522.
2. **Deck cost.** One `just bench-render hd` on the Deck, with and without the probes (the build has no switch: a
   one-line `Option::filter` in `Scene::new` was the A/B), then decide whether an opponent's blend needs a cadence.
3. `_n1` rims: walk Team Selection to an `_n1` row, take a state as in
   emulator-recipes.md, and read whether the wing rims are white at rest in
   the original.
