# `psp-pulse-usa`'s import carries unrelocated address constants, at least around `0x08835830`

2026-08-11, found while reading the AI parser. The decompiler prints `0x276c70` where the string is at `0x08a7ac70` - **add `0x08804000`**. This is the same wart [the Pure re-import row](../HANDOVER.md#open-threads) records as a consequence of analyse-then-rebase, now confirmed on the USA Pulse import as well. **The consequence is worse than cosmetic: the call graph does not resolve in that region.** Every call decompiles as `func_0x000318a4` rather than as a named function, `get_xrefs_to` returns nothing for any string or data address there, and `get_function_callers` returns nothing for functions that certainly have callers - all three read exactly like "this is dead code", which is what stopped the AI consumer hunt. **The technique that does work**: take the rebased address, subtract `0x08804000`, and `search_instructions` for the low half as an operand substring - `Data\XML\AIControlStats.xml` at `0x08a7ac70` becomes `0x276c70`, a single `addiu` hit that anchored the whole parser chain. **Answered 2026-08-17: the damage is local.** An adversarial sweep working in `0x0882xxxx` and `0x0885xxxx` found `jal`, `lui` and data operands all rendering **absolute and in range** there - `jal 0x08851c94`, `lui a0,0x8b3` + `0x17b4` resolving to `0x08b317b4` - and `get_function_callers` working normally. So the wart is regional, not program-wide, and `0x08835830`'s neighbourhood is the affected one. **The practical consequence cuts both ways**: applying the image-relative workaround where it is not needed sends you searching for immediates that do not exist, which is how one sweep in this session reached a confident wrong conclusion. Check whether a `jal` in the region you are reading resolves to a plausible function before assuming either.

## Open

- The unrelocated-address wart is confirmed regional (around `0x08835830`), not program-wide, but the exact boundary of the affected region isn't mapped
- Applying the image-relative workaround where it isn't needed has already produced one confident wrong conclusion

## Next Steps

- Check whether a `jal` in the region you are reading resolves to a plausible function before assuming the unrelocated-address workaround applies
