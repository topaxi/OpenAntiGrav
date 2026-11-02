---
status: ready-to-wire
titles: [all]
updated: 2026-11-02
summary: "apply_setting(&str, &Value) is a 658-line match over 73 string literals; a bad key only logs a warn at runtime; keys are spelled in menu.toml too"
size: M (35)
difficulty: medium (35)
kind: debt
---

# Settings are applied by matching a string key

2026-11-02, `debt-survey`. `crates/game/src/main/session/apply.rs:172` `apply_setting(&mut self,
setting: &str, value: &menu::Value)` runs to line 830 and matches 73 string-literal arms
(`"display.monitor" =>` at `:178` and so on). The same keys are spelled in `assets/ui/menu.toml`
rows, and called from `session/menus.rs` and `session/window_layout.rs`.

## Why it hurts

A key renamed in `menu.toml` (or typoed) falls through to `other => warn!("nothing applies {other}")` at `:822`: a log line, no compile error and no test failure. Adding a
setting means editing the menu row, the settings struct, the apply arm and the settings tests in four
places. The 658-line function is also the second-longest tier in the long-functions thread.

## Open

- Whether `check-strings` or a schema already cross-checks keys against `menu.toml`; unmeasured.

## Next Steps

1. Add a test that every `menu.toml` setting
   key has an arm (the cheapest fix, no refactor).
2. Then a `SettingKey` enum (macro-free, ADR-0058 style: data on the setting) parsed once from the
   TOML key, with `apply` split per group (`display`, `audio`, `controls`).

Verify: the `oag-game` settings tests (`crates/game/src/settings/tests.rs`), `just check-strings`,
`just gate-affected`.
