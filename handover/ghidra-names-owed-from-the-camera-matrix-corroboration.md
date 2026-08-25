# Ghidra names owed from the camera/matrix corroboration

`FUN_08901dc4` (the scene projection builder), `FUN_08900884`/`FUN_08900a9c` (the view-stack writers) and the `Gu_SetMatrix` index mapping are all read and written up on [`exhaust.md`](../docs/ghidra/functions/psp-pulse-usa/exhaust.md), but **none is renamed in Ghidra or in `names.tsv`**. Same for the two plugin-manifest functions [`frontend-boot.md`](../docs/architecture/frontend-boot.md) reads at confidence 80 (`FUN_0888b7dc` and its caller). A future pass should land the rows properly.

## Open

- `FUN_08901dc4`, `FUN_08900884`/`FUN_08900a9c`, and the `Gu_SetMatrix` index mapping are documented but not renamed in Ghidra or `names.tsv`
- `FUN_0888b7dc` and its caller (plugin-manifest functions, confidence 80) are also unrenamed

## Next Steps

- Land the rename rows for these functions in Ghidra and `names.tsv`
