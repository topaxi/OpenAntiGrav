# Frame comparison: three residuals

The pipeline landed and settled the fov unit. Left: (1) the fov reading rests on one ship and one view - a second team's authored value plus the internal view closes it fully, about an hour of emulator time; (2) the shot-versus-row phase is bounded at two ticks but not pinned, which only a fast-moving per-tick capture would do; (3) `place` always writes the basis rows and the `+0xc0` transpose together, so whether the integrator's post-loop rebuild covers the transpose alone was never isolated - academic while the pair works.

## Open

- The fov reading rests on only one ship and one view
- The shot-versus-row phase is bounded at two ticks but not pinned
- Whether the post-loop rebuild covers the `+0xc0` transpose alone (versus paired with the basis rows) was never isolated

## Next Steps

- Get a second team's authored fov value plus the internal view to close the fov reading fully (about an hour of emulator time)
- Take a fast-moving per-tick capture to pin the shot-versus-row phase
