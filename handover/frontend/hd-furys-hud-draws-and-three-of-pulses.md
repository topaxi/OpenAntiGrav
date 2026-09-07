# HD/Fury's HUD draws, and three of Pulse's constants applied to every title were why it did not

2026-08-25, branch `worktree-hd-hud-align`; all of it on [hd-hud.md](../../docs/formats/hd-hud.md#the-hud-draws-and-each-sprite-out-of-its-own-texture). **The one to know**: the sheet held only `Layout::atlas()`'s *first* texture, so 45 of the arcade HUD's 138 sprites sampled `HUD_Components.gtf` at coordinates meant for another image - the right rectangle out of the wrong picture, which reads as art rather than as an error, and which the existing source-rectangle check passes either way. `oag_title::HudArt` is the new axis: texture extension, always-on widgets, pickup-backdrop colour. **Compared against a frame of the running original** (`just rpcs3-race`, speed lap) for the first time - one state of one mode, which is why the always-on set is fifteen names and not fifty. **Four things that frame shows and this build does not**: `DamageBar`'s and the lap arcs' runtime tints, `ShieldBarText`'s `100%`-in-red where the original reads `100` in grey, and the per-lap time rows.

## Open

- `DamageBar` and the lap arcs are missing their runtime tints
- `ShieldBarText` reads `100%` in red where the original reads plain `100` in grey
- The per-lap time rows are missing
- Comparison covers only one state of one mode (speed lap), so the always-on widget set of fifteen names may be incomplete

## Next Steps

- Implement the four things the reference frame shows that this build does not: the `DamageBar`/lap-arc tints, `ShieldBarText`'s grey `100`, and the per-lap time rows
