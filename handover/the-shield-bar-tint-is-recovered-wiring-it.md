# The shield bar's tint rule is recovered; wiring it into the renderer is the open half

2026-09-05. Split off
[the-hud-shows-a-place-measured-against-the.md](the-hud-shows-a-place-measured-against-the.md),
which found the shield bar reading cyan at 100% and solid red at 79% in two
captured frames and could not tell a threshold from a gradient with only two
samples. The colour writer, `Hud_UpdateEnergyBar` (`0x0881c638`, previously
`FUN_0881c638`), was read directly instead of gathering more frames - see
[shield.md](../docs/ghidra/functions/psp-pulse-usa/shield.md#hud_updateenergybar-0x0881c638-tints-the-bar-from-a-20-threshold-not-a-gradient)
for the decompile, confidence 82.

**The rule, in full**: the bar is forced **solid red** (`0x00ff0000` in the
XML's own `0xAARRGGBB` order) whenever the pool is at or under **20%**, or
during a brief one-shot flash that starts the instant the pool *drops*
(i.e. right after a hit lands), whichever fires first. Outside both of
those, it draws the widget's own authored `Color` - `ShieldBar`'s is
`FEConst->HudColour3`, `0xFF0DDFDD`, a light cyan - with only the alpha byte
modulated for a slow blink below the same 20% floor. **Never a blend
between two colours by percentage** - the 79% frame reads red because it was
captured inside the post-hit flash window, not because 79% is itself past
some lower threshold.

`oag_game::hud` (`crates/game/src/hud.rs` and its `draw`/`compose`/`assets`
submodules) currently draws `ShieldBar` at one static authored colour and
has no notion of the pool's current percentage feeding back into the fill
colour at all - `ShieldBarText`'s percentage string is already live (see the
other thread), but the fill's tint is not read from anywhere at runtime.

## Open

- `oag_game::hud` has no per-frame shield percentage -> colour path; wiring
  one needs a `Readout` field carrying the current shield percentage through
  to `draw_list`/`draw`, the same way `ShieldBarText`'s string already
  arrives
- The one-shot post-hit flash needs a timer somewhere in `World` or the HUD
  readout - it is not a pure function of the current percentage, it depends
  on whether the value *just dropped*, so it needs one frame of memory
  (mirroring the original's `hud+0x118`/`hud+0x11c` pair)
- `iVar1` in `Hud_UpdateEnergyBar` (an external override flag read off a
  pointer at `hud+0x2c0`, per `func_0x0003a904`) is not identified - it
  suppresses the forced-red branch entirely when set. Not resolved before
  wiring the Rust side; if left out, the Rust HUD would always take the
  forced-red branch on a genuine drop even in whatever mode this flag is
  meant to suppress it for (a ghost/practice run is the leading guess, from
  the shape alone, not confirmed)
- No test today exercises `oag_game::hud`'s colour output against a known
  shield percentage; adding the threshold-plus-flash logic wants a unit test
  at at least three points (above 20%, at/below 20%, and immediately after a
  synthetic drop) headlessly, no GPU needed since `draw_list` is already
  GPU-free

## Next Steps

- Add a shield-percentage field to the HUD `Readout` (or wherever
  `ShieldBarText`'s percentage is already sourced from) and a one-tick-memory
  flash flag/timer alongside it
- Compute the fill colour in `draw`/`compose` from that pair using the
  threshold-plus-flash rule above, defaulting to the widget's own authored
  `Color` when neither condition holds - do not invent a gradient as a
  simpler stand-in, the disc's own rule is a hard threshold and the CLAUDE.md
  rule against authoring a stand-in for something the disc already specifies
  applies here just as much as to an asset
- Leave `iVar1`'s meaning as an open question in code (a comment, not a
  guessed branch) unless it turns out to matter for the modes this project
  currently implements
