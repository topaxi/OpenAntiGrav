---
status: ready-to-wire
titles: [all]
updated: 2026-11-02
summary: "95 allow(clippy::too_many_arguments) sites, 36 in raceplay and 25 in game; render_view takes 17 parameters with four bools"
size: L (35)
difficulty: medium (35)
kind: debt
---

# Long parameter lists carry 95 `too_many_arguments` allows

2026-11-02, `debt-survey`. `rg 'allow\(clippy::too_many_arguments'` finds 95 sites: `oag-raceplay` 36,
`oag-game` 25, `oag-post` 7, `oag-mesh` 7, `oag-fx` 6, `oag-sound` 4, `oag-weapons` 3, `oag-ai` 3. Each one
silences the lint instead of bundling what travels together.

Worst read: `crates/raceplay/src/scene/viewports.rs:135` `render_view` takes `device`, `queue`,
`encoder`, `view`, `race`, `window`, `slot`, `area` (a 4-tuple), `fov`, `cull`, `pvs_cull`,
`anim_seconds`, `shadows`, `zone_spectrum`, `shared`, `effects`: 17 parameters, four of them `bool`.
`render_viewports` (`:202`) and `render_window` (`:263`) repeat most of the list. Other 8+ parameter
public functions: `game/src/preview.rs` (4), `game/src/render.rs` (3), `game/src/main/race_stage/endrace_touch.rs` (3).
A boolean-heavy call site (`..., true, false, None, ...`) cannot be read without the signature.

## Why it hurts

Adding one input means editing every signature in the chain and every call site; the compiler cannot
catch two swapped `bool`s; the same parameter group is re-decided in each function.

## Open

- Which groups recur (GPU handles `device, queue, encoder`; view settings `fov, cull, pvs_cull, shadows`;
  window identity `window, slot, area`). Count by reading the 95 signatures: unmeasured.
- Whether some bools are really modes (an enum) rather than flags.

## Next Steps

1. Introduce a `ViewParams` struct (settings) and a `Gpu<'a>` borrow struct for the three handles in
   `scene/viewports.rs`; turn the bools into named fields. One crate per lane, `raceplay` first.
2. Delete each `allow` as its signature falls under 8 arguments; `just lint` fails if one is left
   dead (`allow` stays legal), so also `rg` the count before and after.

Verify: `just lint`, `just gate-affected`, and the render ground-truth tests of the touched crate.
The numbers must be bit-identical: this is a signature change only.
