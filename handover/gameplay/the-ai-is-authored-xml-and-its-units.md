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
