# The shield path's last unmeasured field: `entity + 0x368`

2026-08-10. Everything else about the pool now has a runtime leg, including the coefficient (`amount / |p| = 0.035000` on 25 of 25 calls) - see [shield.md](../docs/ghidra/functions/psp-pulse-usa/shield.md).

**2026-08-25: the writer is found.** `Craft_Construct_q` (`0x08840c74`) stores its own second argument there verbatim (`move s2,a1` at entry, `sw s2, 0x368(s0)` at `0x08840fec`, nothing modifies `s2` in between) - confidence 80, decompilation-only. That rules out "local human player slot": rereading `Ship_Damage` in full shows its real pool-subtraction branch treats `0` and `2` identically while a separate branch for `1` skips the pool entirely (only bumps a telemetry counter) and `3` reaches neither branch. The shape instead fits a controller/ownership-kind selector - `0` local human (also the only value with a HUD update and full telemetry), `2` a second, silent, fully-simulated category (AI), `1` a category whose pool this machine does not authoritatively own (a remote network player), `3` untouched by damage entirely - but that mapping is inference from behaviour, not a read of an enum, confidence 60, and **is not runtime-verified**.

What's now blocking a close: `Craft_Construct_q` has **no static caller anywhere in the binary** - no `jal` xref, no data reference to its address (checked by byte-pattern search for its address as a little-endian literal). It is reached only through some indirect dispatch this pass could not resolve. So which numeric value each real craft (the player's, each AI opponent's) actually gets at construction time is still unread from the disc's own code - full details, including the exact instruction addresses and the `Ship_Damage` re-read that refutes the old hypothesis, are on [shield.md](../docs/ghidra/functions/psp-pulse-usa/shield.md#entity--0x368-is-craft_construct_qs-own-second-argument).

## Open

- Which numeric value `Craft_Construct_q`'s second argument gets for the local human's craft versus each AI opponent's is unmeasured - there is no static caller to read it off.
- The controller/ownership-kind hypothesis (`0`=local human, `1`=network human, `2`=AI, `3`=unused by damage) fits `Ship_Damage`'s branching but is inference, not a confirmed enum - confidence 60.
- Whatever indirectly calls `Craft_Construct_q` (a class-descriptor or vtable-style dispatch) is itself unidentified.

## Next Steps

- Breakpoint `0x08840fec` (or `Craft_Construct_q`'s entry, `0x08840c74`) in a live PPSSPP session during a race with a known mix of local human and AI craft, and read `a1`/`s2` per call, matching each value to which craft it constructed. This is the same kind of probe [shield.md](../docs/ghidra/functions/psp-pulse-usa/shield.md) already used for the `0.035` coefficient - the calling convention there is a useful cross-check (`Ship_Damage` takes its entity in `a0`, not `a1`, so do not assume this constructor follows o32 either without checking).
- If the breakpoint approach is blocked (e.g. the constructor also runs before a race, at menu/livery-select time, complicating "known mix"), a second route is finding what calls it indirectly: search for a class descriptor or vtable holding `0x08840c74` as a *relocated* value, since a straight byte-pattern search for the raw little-endian address found nothing.
