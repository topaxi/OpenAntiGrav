# MONITOR has only ever run on a one-screen machine

The *miss* path is covered; the working path is not. A wrong scale factor offsets the window by the difference on any screen not at 100 %, and it centres on the *inner* extent while setting the *outer* position, so a decorated window sits high by about a title bar (known, documented, not corrected). **Try it on two screens**: `just play`, OPTIONS -> DISPLAY -> MONITOR, windowed and borderless. A tiling compositor may refuse all of it.

## Open

- The working (non-miss) monitor path has never been run on a multi-screen machine
- A wrong scale factor offsets the window on any screen not at 100%
- Centring uses the inner extent but sets the outer position, so a decorated window sits high by about a title bar (known, not corrected)

## Next Steps

- Test on a two-screen setup: `just play` -> OPTIONS -> DISPLAY -> MONITOR, in both windowed and borderless modes (a tiling compositor may refuse it)
