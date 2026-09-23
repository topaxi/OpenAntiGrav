# The ghost is drawn and raced, and not yet compared against the original

Landed 2026-09-23 under ADR-0055 (`docs/architecture/adr/0055-replays-are-inputs-and-a-ghost-is-poses.md`):
`oag-replay`, the recorder on `race::Race`, `oag_game::ghosts` (files under
`<config dir>/oag/ghosts/`), and `oag_render::ghost` drawing Pulse's three
`MeshNode_Ghost` passes off `docs/ghidra/functions/psp-pulse-usa/ghost.md`.
Replays re-drive bit for bit on Pulse, Pure, HD and 2048
(`crates/game/tests/replay_ground_truth.rs`); the ghost was seen ahead and
faded through on Pulse and Pure in headless captures (`--ghost`/`--record-ghost`).

## Open

- Everything on ghost.md is a static read. No capture of the original's ghost
  exists, so the cross-fade weight, the proximity ramp and the screen-space
  static (confidence 60) are unchecked against a frame.
- No in-game option to hide the ghost (Pulse's `Ghost Ship Visible`,
  `options+0x45c`). A new menu row needs a `string_id` and pointer support.
- Pure, HD and 2048 draw Pulse's recipe, chosen not measured. A lead, not a
  claim: Pure's disc carries `Data\Tex\staticglow.mip` too (the race loader
  reports it, 32x128), which suggests Pure shares `MeshNode_Ghost`. HD's
  `MeshNode_Ghost` (`vex-classes.md`) and its `Ghost Ship Visible` option were
  not read.
- The windowed session's arm/save path (`main/race_stage/ghost.rs`) compiles
  and shares every piece the capture path exercises, but was not driven in a
  window this pass.
- A full-race replay viewer: the file format serves it; nothing plays one.

## Next Steps

- Capture a second Time Trial lap on PPSSPP with a ghost ahead and compare a
  frame at 10 and 25 units against `oag-game --race --ghost`.
- Add the ghost on/off row, keyed per title, with its string and pointer support.
