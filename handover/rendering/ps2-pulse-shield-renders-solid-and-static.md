# PS2 Pulse's shield renders solid and does not animate

2026-09-17. Reported from play by the user: on the PS2 Pulse source the
Shield pickup's shell "renders solid and not animated", where the PSP source
draws it right (the additive shell that swells on impact and breathes -
`oag_render::shield`, recovered on
[shield-pickup.md](../../docs/ghidra/functions/psp-pulse-usa/shield-pickup.md)).

Not yet diagnosed. Two candidates, both checkable without Ghidra first:

- **The PS2 `shipshield.vex` material/blend.** PS2 models take an external
  texture set and their materials resolve differently
  (`oag_render::mesh::build_with_textures`'s PS2 branch); a shell whose
  additive blend or vertex alpha is not read on the PS2 path would draw
  opaque. Compare `oag-view --mesh` on the PSP and PS2 `shipshield.vex` for
  the same team.
- **The animation is keyed off something the PS2 source does not feed** -
  `oag_render::shield`'s state is driven per tick from the race; if the PS2
  source's shield entry names resolve to a different node layout
  (`crates/game/src/race/assets.rs::shield_entry_names`), the scale/alpha
  may never be applied.

## Open

- Which of the two it is. Reproduce with `cargo run -p oag-game -- --race
  --give shield --press square --ticks N --screenshot ...` on
  `data/images/pulse-ps2-eu.chd` against the same on the PSP image, three
  frames each (activation, mid-breathe, a hit).

## Next Steps

1. Reproduce and diff the two sources' screenshots at player size.
2. Fix on the PS2 path; the PSP law is the recovered one and stays.
3. `docs/formats/ps2-status.md` (or the PS2 texture page) gets the finding.
