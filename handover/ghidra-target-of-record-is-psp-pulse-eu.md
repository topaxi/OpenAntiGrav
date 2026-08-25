# Ghidra target of record is `psp-pulse-eu`; EU cross-verification is owed

Search and decompile in `/psp-pulse-eu/BOOT.BIN` first, cross-verify against `/psp-pulse-usa/BOOT.BIN` with `find_similar_functions_fuzzy`/`diff_functions`, document under `docs/ghidra/functions/psp-pulse-eu/`. **The camera pass got two of six**: `Camera_SubmitScene` (EU `0x088786d0`) and `Camera_PublishTripod` (EU `0x08885bd4`) are body-identical to their USA counterparts including the `1.7647059`, the `65.0 / fov` near-plane term and the literal `480.0 / 272.0` (confidence 90); the other four are listed as not-checked rather than guessed. Data rows stay out until the import is rebased - see [the workflow page](../docs/ghidra/workflow.md#psp-pulse-eus-data-addresses-do-not-share-usas-base).

## Open

- Only 2 of 6 camera functions are cross-verified against EU (`Camera_SubmitScene`, `Camera_PublishTripod`); the other four are not-checked
- Data rows stay out until the EU import is rebased

## Next Steps

- Cross-verify the remaining four camera functions against `psp-pulse-usa` with `find_similar_functions_fuzzy`/`diff_functions` and document under `docs/ghidra/functions/psp-pulse-eu/`
