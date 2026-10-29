---
status: ready-to-wire
titles: [all]
updated: 2026-10-29
summary: "--drive runs our menus headless (race box, RACE REMIX, track files on Pulse); campaign, prompts, join report no rows; HD/2048/Omega unrun"
---

# `--drive` takes our menus into a race with no window; most overlays report no rows yet

`oag-game --drive "<steps>" --state-log f.jsonl` runs the real `Session` with no window
(`docs/tools/oag-game.md`, "Driving the menus with no window"; `crates/game/src/main/drive.rs`,
`oag_game::drive`). The race box and RACE REMIX with a `--track-file` race are proven on Pulse by
`crates/game/tests/drive_ground_truth.rs`.

## Open

- **Overlays report no rows.** `prompt`, `campaign`, `pure-chain`, `join`, `window-layout` and
  `seat-assign` show as `screen=<kind>` with an empty row list, so a script can wait for them and
  `press` through them but not `select` in them. The custom-campaigns work adds the campaign grid:
  one more arm in `main/drive.rs`'s `screen`, no change to the driver.
- **`string_id` is not reported.** The brief asked for it; `RowMeta` holds `id` only, and the
  string id is resolved away in `menu/definition`. A row without an `id` is selected by its
  setting key or its action name (`launch_race`), which are language-stable.
- **A refused launch shows only in the log.** START on RACE REMIX before the custom-track check
  finishes logs "cannot race this custom track: still checking the files" and stays on the page;
  the script sees `until stage=race` time out. `set remix.track=...` waits for the checked list,
  which avoids it; the refusal itself is not in the state.
- **`--size` is ignored**: the headless surface is `display.window_size`.
- **Only Pulse (PSP, EU) is run.** HD, 2048, Omega and Pure menus are untested; `select` probes
  Down then Right where a screen does not name its buttons, which a strip layout or HD's revealed
  child rows may defeat.
- **Methods that panic headless.** `gpu::Host` derefs to the winit window and panics on anything
  it does not override (`set_fullscreen`, `set_resizable`, `request_inner_size`,
  `set_outer_position`: `apply.rs` reaches them when a DISPLAY row changes). Changing a display
  setting from a script is not supported.
- Not built: the Android and iOS targets (`drive` is `cfg(not(wasm32))`), which `just` does not
  build either.

## Next Steps

1. Give the campaign grid, the prompts and the join screen rows in `drive::screen`, and a
   `campaign` ground-truth test that picks a cell by id and checks the launched race.
2. Run `boot; select racebox; press cross; until page=race` on HD, 2048 and Omega and record
   what holds.
3. Report a refused launch as a state field (`notice`) so a script can fail on it at once.
