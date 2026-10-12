# Omega's original opponent flies its campaign

2026-10-12, `orig-ai-omega`. Evidence: [ps4-omega-eu/ai-controls.md](../../docs/ghidra/functions/ps4-omega-eu/ai-controls.md)
(static map, both lineages, the coin), [gameplay/ai.md](../../docs/gameplay/ai.md),
`crates/game/tests/omega_original_ai_ground_truth.rs`.

## Landed

- `oag_omega::race::ORIGINAL_DRIVER`: HD's four branches and halved-lookahead rows,
  2048's coin search (ungated on Omega, absent on HD), 2048's other lineage laws off.
  `original_opponent = true`: every HD-schema campaign cell maps to a named mode id
  (HD lineage), and the field finishes on four circuits with no respawn.
- `original_laws::resolve` reads `PI_Track` from `track_plugin_definition` when a
  title has one (Omega's `plugin_definition` is its team file).

## Open

- **Static only.** No PS4 emulator: no Omega figure has been seen running. PS4
  `rand()`'s width is chosen (HD's 30 bits).
- **The route from a published cell `Mode` to `0x01f999e4`** at race start was not
  walked (confidence 75 that a cell runs its named id).
- **Omega's 2048 campaign** (`campaign2048_definition.xml`) is not read by this
  project; its events would run the 2048-lineage laws (every tick, event ring rate,
  preferred lane follows choice) and `AI_ResolveSkillScale`'s `+1.9` under a game-mode
  `+0x21c` of `2`, which 2048 v1.04 lacks.
- **The 1.07 patch's `eboot.bin`** was not compared with the base one read here.
- Not ported, as on HD and 2048: the Eliminator spin, Duel block, split screen, pad
  tracking and the contact sidestep (2048 lineage).

## Next Steps

1. Walk `FUN_01526370`'s published `Mode` to the race-start write of `0x01f999e4`.
2. Diff the patch `eboot.bin`'s `AiManager_AddDriver` and `AI_UpdateCraft` against the
   base's (byte patterns around `0.00065` and `05_Track`).
3. When Omega's 2048 campaign is read, key the laws per event lineage (data on the
   event, not a title branch) and fly 2048's events on Omega.
