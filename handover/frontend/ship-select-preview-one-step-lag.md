---
status: measure
titles: [hd]
difficulty: easy (30)
updated: 2026-11-16
summary: "In --drive screenshots of the live Ship Select the hull drawn lags the selection by one step"
---

# Live Ship Select hull lags one step in a `--drive` screenshot

Seen by the `zone-preview` lane on HD, in Time Trial and in Zone: after `press right` and 3000
ticks, the screenshot shows the new team's name and stats but the previous team's hull. The
`--menu-page ship-select` still (no stepping) is right. Run on the lane's branch only; not
re-run on main, so whether it predates the lane is unknown. A real window was not checked.

## Open

- Whether a windowed session shows it, or only the headless `--drive` screenshot draw.
- Whether `PickerStage::refresh_preview` is called before the picker's index moves in the same tick.

## Next Steps

1. Re-run on main: `--drive` script with `set race.mode=time_trial`, pick a track, then
   `until screen.id=ship-select`, `ticks 3000`, `press right`, `ticks 3000`, `screenshot a.png`;
   compare the hull with the selected team.
2. If it reproduces, trace `refresh_preview` / `poll_preview` / `land` in
   `crates/game/src/main/picker_stage.rs` against the index the frame draws.
