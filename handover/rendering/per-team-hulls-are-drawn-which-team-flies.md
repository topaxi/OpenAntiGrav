---
categories: [rendering, gameplay]
---

# Per-team hulls are drawn; which team flies which slot is recovered and ported

2026-08-15, `crates/game/src/livery.rs`. Every grid slot loads its own team's `Ship.vex`, `shipboost.vex`, `Engine Flare` locator and `Ship Collision Fx` anchors; `race::load` returns `Vec<Livery>` and `Scene` builds a `Drawable` per slot from it. **Measured first, because it decided the test**: the eight teams the PSP disc declares are eight *different models* - 845 to 1,497 triangles, radius 6.45 to 7.09 - where every team's *Zone* hull is byte-identical geometry and differs only in paint. So the ground-truth test counts distinct triangle counts, which would be vacuous on Zone. ~~**What's open is the slot assignment: it is this project's.** `Race_SpawnAiRacer` takes an `id` from a racer list built upstream that nothing has read, so the original may draw teams by championship entry, by player choice or at random; `livery::teams_for_slots` fills from the catalogue in file order and the load report says so on every run.~~ (closed 2026-10-04, below). **The list length cuts both ways**: with all four DLC packs mounted the catalogue is twelve teams for eight slots and the extras do not race, while Pure and the PS2 set can be *shorter* than the grid, where the list cycles and the report names the repeat. Sparks still use slot 0's anchors alone, because `oag_fx::sparks` triggers off the player's contacts only.

**The alternate-paint half of this thread split off 2026-09-02**, once read: `PI_TeamModel`/`PI_ModelSkin`'s XML schema is documented in `docs/formats/dlc-pack.md`, and `ship_alt.dat`/`ship_eliminator.dat` are shape-read as paletted image data in `docs/formats/handling-stats.md`. The `.dat` payload is decoded as of 2026-09-05 (`docs/ghidra/functions/psp-pulse-usa/ship-skin.md`) - a skin is a texture swap on the same geometry, not a second model. The parser and the drawing landed 2026-09-09 and are tracked in `a-race-flies-a-skin-what-selects-one-is-chosen.md` rather than here.

**2026-10-04: which team flies which slot is recovered and ported.** Single Race draws its AI roster in `RaceSession_DrawAiRoster` (`0x08821bd4`, confidence 85, live on seven launches with three player teams): every eligible team but the player's, shuffled with a naive swap off a wall-clock seed, the first seven race, the player last. `oag_game::livery::teams_for_slots` now follows it off the race seed. Evidence: `docs/ghidra/functions/psp-pulse-usa/grid.md`, "Which team flies which slot, recovered". Two earlier claims on this thread were wrong and are corrected there: the `FUN_0882e57c` route was `Tournament_Construct`, not Single Race, and `Craft_Construct` is exactly where the team name resolves (`craft+0x370 = Team_FindByName(name)`, falling back to `Feisar`).

## Open

- **The roster seed is the race seed**, which no real launch varies, so every `--race` without `--seed` flies the same order. The original reseeds from the wall clock every launch. Varying the seed per launch moves every other seeded draw too, so it is a session decision.
- **The DLC case is static only**: with four packs mounted, seven of eleven eligible teams race, picked by the draw. No live run with packs mounted.
- **An execution breakpoint on `RaceSession_DrawAiRoster`'s entry never fired** under PPSSPP although the function ran (JIT markers, a fresh draw per launch). Matters only for a capture that must stop inside the draw.
- **Tournament's later-leg grid reorder is not ported**: `Tournament_Construct`'s non-first branch reorders racers by `+0xd0` and reuses a per-racer team cache at `+0x110`; `FUN_0882e2fc` (a resume path, unread) rebuilds both from a stored record. The port keeps the roster (one seed) but not the reorder.
- Sparks use slot 0's (the player's) anchors alone, not the full grid.

## Next Steps

- To capture inside the draw, arm the breakpoint before Main Menu, or try PPSSPP's IR interpreter. `scripts/psp-team-roster.py` reads `g_race_session` `+0x3c` and each craft's `+0x370` after a load, memory reads only.
- Whether `RESTART RACE` redraws the roster is not determined: run `psp-team-roster.py` before and after a restart.
- A DLC-mounted live run would lift the seven-of-eleven claim off static.
- The unlock filter (`Definition_IsUnlocked`) is not applied; it is assumed, not checked, that the caller's list holds only teams the player could race against. Check what `boot_shell.teams` filters before relying on that.
