# `search_instructions`'s `mnemonic` filter is exact-match, and delay-slot forms are their own mnemonics

2026-08-17, and it invalidated a sweep in this session. `mnemonic="lw"` does **not** match `_lw`, and `mnemonic="lv"` returns **zero** where `lv.q` returns 323. MIPS delay-slot instructions render with a leading underscore - `_lw`, `_lwc1`, `_addiu`, `_move`, `_sw` - so any sweep that filters by mnemonic silently misses every instruction in a branch delay slot, which on this compiler's output is a large fraction of loads. **Sweep with no mnemonic filter and an operand pattern only**, or run each form explicitly. A sweep that concluded "nothing reads this offset" from `lwc1` alone has checked maybe half of what it thinks it has. Also note `run_script_inline` is broken in this Ghidra instance (`GhidraPlaceholderBundle cannot be cast to GhidraSourceBundle`), so a single exhaustive listing pass is not available and coverage is the union of per-mnemonic sweeps.

## Open

- `mnemonic` filter is exact-match; delay-slot forms (`_lw`, `_lwc1`, `_addiu`, `_move`, `_sw`) are distinct mnemonics a plain filter misses
- `run_script_inline` is broken in this Ghidra instance, so no single exhaustive listing pass is available
- A sweep that filtered on `lwc1` alone has checked maybe half of what it thinks it has

## Next Steps

- Sweep with no mnemonic filter and an operand pattern only, or run each mnemonic form explicitly
