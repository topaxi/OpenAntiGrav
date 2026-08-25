# Task #33: `oag-trace` cannot exercise the mag-lock hold

`replay`/`drive` take one `Environment` for the whole run and the track samples change per tick, so a replay's blend is 0 by construction. Needs per-tick locator plumbing. It carries the locator-fidelity lead too: the hold explains 49.5 % of the inverted-section residual with nothing fitted, and the remaining magnitude points at the *locator* - our 4-per-segment resampled spline may not be the original's evaluated curve.

## Open

- `replay`/`drive` use one `Environment` for the whole run, so a replay's mag-lock blend is 0 by construction
- The residual left after the hold's 49.5 % may come from locator fidelity - our 4-per-segment resampled spline may not match the original's evaluated curve

## Next Steps

- Add per-tick locator plumbing to `replay`/`drive` so the mag-lock hold can be exercised
