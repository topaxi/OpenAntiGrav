# `just apply-names` reported success for renames the bridge did not make

2026-08-10, found while landing the shield rows. `scripts/apply-ghidra-names.py docs/ghidra/functions/psp-pulse-usa/names.tsv --program /psp-pulse-usa/BOOT.BIN` printed **"383 applied, 0 skipped"**, and every one of the eight new symbols was still `FUN_`/`DAT_` afterwards - proved by the manual MCP call, which reported renaming *from* `FUN_0883dd24`. There is only one Ghidra instance (`list_instances`, TCP 8089), so it is not a wrong-target problem. **Data rows have a second, separate failure**: `rename_data` is rejected outright by the server's Hungarian-prefix validator (`g_skill_level` -> `missing_hungarian_prefix`), which conflicts with ADR-0005's own `g_snake_case` convention that all 45 existing data rows use; `create_label` works and is what landed them. Root cause not chased - the eight renames were made directly through MCP and the program saved. **This matters because `just apply-names` is the project's stated mechanism for making the Ghidra database reproducible from the repository**, and a silent no-op means a fresh import stays at `FUN_` while the tool says otherwise.

## Open

- Root cause of the silent no-op on function/data-label renames not chased
- `rename_data`'s Hungarian-prefix validator conflicts with ADR-0005's own `g_snake_case` convention, unresolved

## Next Steps

- Check a sample symbol after running `just apply-names` to confirm the renames actually landed
