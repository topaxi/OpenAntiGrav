---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-25
summary: "Benchmark follow-ups: a vanilla or with-mods run recorded in the result, and COPY TO CLIPBOARD on the stats screen"
categories: [frontend]
size: S (40)
difficulty: easy (40)
---

# Benchmark follow-ups: the mods choice and copying the results

## Open

The in-game benchmark (`docs/tools/in-game-benchmark.md`) starts its race the way a player's race starts, so every mod the player has enabled loads with it: extensions, mod shaders, asset overrides, extra effects. A result from one player's setup is then not comparable with another's, and nothing in the stats screen or the result JSON says which mods were loaded.

The maintainer wants two kinds of run:

- **Vanilla:** no mods at all, so desktop, Deck and phone results line up and a regression in the engine shows on its own.
- **With mods:** the player's enabled mods, so a player or a mod author can see what their mods cost.

## Next Steps

Start only after `benchmark-no-input` (benchmark input: abort only) has merged; both touch the stats screen.

**A. Copy to clipboard (maintainer, 2026-11-25).** The stats screen gets a COPY TO CLIPBOARD action beside BACK and RUN AGAIN, with pointer and touch support, a `string_id` and English text.
- What it copies: a plain-text summary a player can paste into a forum or issue: title, track, average FPS, 1 % and 0.1 % lows, median, P95, P99, worst, frames over budget, the settings summary, the mods line (below), device, GPU, driver, build version.
- `crates/game/src/app/clipboard.rs` only reads today (`read`), so add a write on every target: desktop (the crate it already uses), Android and the web build. Where a platform cannot write, hide the action rather than offer one that fails.
- A short "Copied" confirmation on the screen.
- Tests:
  - the text formatter, with a unit test pinning its exact lines;
  - the action's presence or absence per platform.

**B. The mods choice:**


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
