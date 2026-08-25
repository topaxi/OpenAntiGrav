# Per-team hulls are drawn; which team flies which slot is not recovered, and the paints are not read

2026-08-15, `crates/game/src/livery.rs`. Every grid slot loads its own team's `Ship.vex`, `shipboost.vex`, `Engine Flare` locator and `Ship Collision Fx` anchors; `race::load` returns `Vec<Livery>` and `Scene` builds a `Drawable` per slot from it. **Measured first, because it decided the test**: the eight teams the PSP disc declares are eight *different models* - 845 to 1,497 triangles, radius 6.45 to 7.09 - where every team's *Zone* hull is byte-identical geometry and differs only in paint. So the ground-truth test counts distinct triangle counts, which would be vacuous on Zone. **Two things are open.** (1) **The slot assignment is this project's.** `Race_SpawnAiRacer` takes an `id` from a racer list built upstream that nothing has read, so the original may draw teams by championship entry, by player choice or at random; `livery::teams_for_slots` fills from the catalogue in file order and the load report says so on every run. (2) **`PI_TeamModel`/`PI_ModelSkin` are still unread**, and they are *not* what this row closed - they are the alternate paints with `loyalty` unlock thresholds, and `PI_TeamModel`'s `location` is a **file stem** (the literal `"ship"`), which is the `shipwreck.vex`-not-`Assegaiwreck.vex` trap. **The list length cuts both ways**: with all four DLC packs mounted the catalogue is twelve teams for eight slots and the extras do not race, while Pure and the PS2 set can be *shorter* than the grid, where the list cycles and the report names the repeat. Sparks still use slot 0's anchors alone, because `oag_render::sparks` triggers off the player's contacts only.

## Open

- Slot assignment (which team flies which grid slot) is not recovered - the racer list `Race_SpawnAiRacer` reads its `id` from has never been read
- `PI_TeamModel`/`PI_ModelSkin` (alternate paints with `loyalty` unlock thresholds) are still unread
- With all four DLC packs mounted, the catalogue is twelve teams for eight slots and the extras do not race
- Sparks use slot 0's (the player's) anchors alone, not the full grid

## Next Steps

- Read what builds the racer list upstream of `Race_SpawnAiRacer`'s `id` to recover the real slot assignment
- Read `PI_TeamModel`/`PI_ModelSkin` for the alternate paints and loyalty thresholds - watch the file-stem trap (`PI_TeamModel`'s `location` is the literal `"ship"`)
