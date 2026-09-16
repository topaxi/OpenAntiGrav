# The AI is authored XML, and its units are the only thing blocking a port of the original's numbers

2026-08-17. Both games' AI tuning (`AIControlStats.xml`, `AIRaceStats*.xml`, `WeaponAIstats.xml`) is parsed and named end to end - [ai.md](../../docs/gameplay/ai.md), [ai-stats.md](../../docs/ghidra/functions/psp-pulse-usa/ai-stats.md) and [weapon-ai.md](../../docs/ghidra/functions/psp-pulse-usa/weapon-ai.md) carry the schema and evidence; **do not requote from this file**. Still open: the *units* the parser stores and never itself consumes (consumer not identified), Pure's `Position` sign convention, which of Pure's two per-class blocks is live, `ControllerPO`'s consumer (Autopilot pickup, confidence 40, not acted on), and porting the fire/absorb decision, which needs the two skill indices at `self+0x34`/`+0x38` and the `0..1` scalar at `+0x2c` - none read yet. **The retraction is the lesson worth keeping**: a claim that nothing reads `WeaponAIstats.xml` shipped at confidence 92 off three static sweeps that were all structurally blind to `record + id*0xc + 8` addressing - it is read ~400 times a second and fires by rolling the authored odds against a five-entry difficulty table. Bound confidence by what the method cannot see, not by how many variants of one blind method were run; and a zero from a race with the player parked was the instrument, not the scenario, since the reader gates on a signed along-track range a stationary craft never satisfies. **The nine renames from this pass live only on the `ai-recovery` branch's `names.tsv`** - land it or revert, or a fresh Ghidra import reproduces nothing.

## Open

- The units the parser stores are never consumed - the consumer is not identified
- Pure's `Position` sign convention is unresolved
- Which of Pure's two per-class blocks is live is unresolved
- Porting the fire/absorb decision needs the two skill indices at `self+0x34`/`+0x38` and the `0..1` scalar at `+0x2c` - none read yet

## Next Steps

- Land the nine renames on the `ai-recovery` branch's `names.tsv`, or revert them, before a fresh Ghidra import
- Identify the consumer of the units the parser stores

## 2026-09-16: the fire-or-absorb decision is ported on a branch, and the branch is red

**Read and merged to main:** the six weapon-AI record fields the port needed
(`+0x2c` held timer, `+0x34`/`+0x38` skill indices, `+0x52`, `+0x54`, `+0x60`
along-track gap) and the ten `WeaponAi_*` scorers around them, at 80-90 -
[weapon-ai.md](../../docs/ghidra/functions/psp-pulse-usa/weapon-ai.md), rows in
`names.tsv`; `WeaponAIstats.xml` is parsed by `oag_tables::weapons::ai` with a
real-disc ground truth (`crates/tables/tests/weapon_ai_ground_truth.rs`); and
the four-times-a-second cadence is recorded as a choice rather than a
re-confirmed reading.

**Ported but NOT merged:** `WeaponAi_DecideFireOrAbsorb` for eleven of
thirteen weapons, replacing the invented `TRIGGER_RATE` policy, plus two
disc-confirmed fixes found on the way (the nearest-craft gap scored off every
craft rather than the driver's perception filter; the `+0x2c` held timer never
reset, latching the absorb bias on). It lives on branch **`plasma-flash-ai`**
(`56ed06cc` the port, `6c390a2d` the gap fix, `6b826d9d` its docs, `5da274b5`
naming the regression) with `just` green at 3708/3708 - and `just test-data`
red: **14-15 rows of `crates/game/tests/ai_clean_lap_gate.rs`** over their
frozen baseline, worst `07_Track` at up to 634 contact ticks against 329. That
file's `LONE` craft is an AI opponent, so it runs real weapon pads through the
port; bisected to the port, root cause not confirmed (working hypothesis: the
Turbo's absorb branch firing where the invented policy fired the Turbo at
once). Left on the branch rather than merged, per the no-inherited-red rule,
and per the user's stance that opponent behaviour is a design axis: the
measured baseline is worth having, but not at the cost of a lone AI craft
scraping the wall twice as much.

**Next step for whoever picks it up:** `git merge main` on `plasma-flash-ai`,
run `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored
ignored-only ai_clean_lap_gate`, and diff the Turbo branch's behaviour against
the old `fire_turbo_at_once` gate on `07_Track` first - a Turbo fired into a
corner is the shape the old policy's own doc comment warned about.
