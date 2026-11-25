---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-25
summary: "The in-game benchmark loads the player's mods; offer a vanilla run and a with-mods run, and record which in the result"
categories: [frontend]
size: S (40)
difficulty: easy (40)
---

# A benchmark run does not say whether mods load

## Open

The in-game benchmark (`docs/tools/in-game-benchmark.md`) starts its race the way a player's race starts, so every mod the player has enabled loads with it: extensions, mod shaders, asset overrides, extra effects. A result from one player's setup is then not comparable with another's, and nothing in the stats screen or the result JSON says which mods were loaded.

The maintainer wants two kinds of run:

- **Vanilla:** no mods at all, so desktop, Deck and phone results line up and a regression in the engine shows on its own.
- **With mods:** the player's enabled mods, so a player or a mod author can see what their mods cost.

## Next Steps

1. Decide the shape of the choice, recommended default first:
   - A `--benchmark-mods vanilla|enabled` CLI option (default `vanilla`), plus the same choice on the BENCHMARK and RUN BENCHMARK entry points. A preset in the benchmark preset TOML (`schemas/v1`) may also set it.
   - Or two rows: BENCHMARK (VANILLA) and BENCHMARK (WITH MODS). Hide the second when no mod is enabled (rows with no choice stay hidden).
2. Find where a race resolves its mod list (`oag-mod` search path, `oag-extras`, the extension host behind `extensions`) and give the benchmark launch a way to load none of them, without touching the player's saved mod settings.
   - Built-in `oag-extras` content that only adds menu rows is not a mod for this purpose. Decide whether built-in visual extras (the official `oag.*` mods) count as mods; record the choice.
3. Record the choice in the result: add `mods: "vanilla"` or `mods: [{id, version, hash}, ...]` to the JSON, which already carries the module and stream hashes screenshots use. Show one line on the stats screen ("Mods: none" or the count).
4. `just bench-compare` should warn when it compares a vanilla run with a modded one, as it already warns for in-game against offscreen.
5. Tests:
   - a vanilla run loads no extension and no override, with a test that goes red if the guard is removed;
   - a with-mods run records each loaded mod's id and hash;
   - the player's mod settings are byte-identical after either run.
6. If the format of a mod-facing file changes, update `assets/mods/examples/.claude/skills/oag-mod-author/SKILL.md`, which a mod API change requires.
