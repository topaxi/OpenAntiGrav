---
status: ready-to-wire
titles: [pulse]
updated: 2026-11-20
summary: "Pulse's pickers author Help screens (Track, Team, Tournament, Music) not drawn; the PS2 footer's Help does nothing; key-hint string names the circle"
difficulty: medium (50)
---

# The pickers' Help screens, and a key hint that names the circle

Back and Help come from one resolver on every front-end screen now (`docs/ui/back-and-help.md`); two
leftovers of that lane are open.

## Open

- `Track Creation`/`Track Select`, `Team Selection`, `Tournament C` and `Pre Race Music Select` author a Help
  (or music) overlay on a face button (`forward="triangle"` on the PSP, `circle` on the PS2), read in
  `docs/ui/selection-screens.md` ("Open") and not drawn. The PS2's footers draw `Help` beside the circle on
  Track Select and the circle does nothing. `Picker` already holds `Roles::help`; the screens are the work.
- `OAG_HINTS_SHELL_KEYS` ("backspace, circle or escape goes back", twenty languages) names the circle, which is Help on
  the PS2 and not Back under MENU BACK = `triangle`. It is the window title's key hint, prose with no codepoint
  to substitute.
- Photo mode, the replay viewer and the disc chooser leave on the circle on every pressing: they read the
  device (`Controls::view_take`) or run before a title is known.

## Next Steps

1. Draw `Track Help`/`Team Help` from the XML on `Picker::help_open`, opened by `Roles::help`, closed by cross, triangle or circle as `Cell Help` is.
2. Build the key-hint sentence from the resolved Back button's bound keys (`oag_input::keys::bound_keys`).
