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

- **The Harimau livery.** The original's in-race `Data\Ships\Harimau` is the
  white and orange scheme its carousel shows. Our `--team harimau` flies an
  orange and blue one. It shows in every paired frame of 2026-10-12. This is
  a texture or skin choice, not lighting, and it is unread.
- **The white wing rims on the `_n1` skins** are not the SPU term: they stay
  with the craft binding removed. Whether the original shows them is unread.

## Next Steps

1. Harimau livery: list the textures that `Data\Ships\Harimau`'s
   `ship.rcsmodel` materials name, and the ones our hull loader binds for
   `harimau`. Check which skin `PI_ModelSkin`/`PI_TeamModel` selects for the
   classic row. The state `hd-timetrial-talons-harimau-classic-start` shows
   the original's livery at rest, on screen in about 20 s.
2. `_n1` rims: walk Team Selection to an `_n1` row, take a state as in
   emulator-recipes.md, and read whether the wing rims are white at rest in
   the original.
