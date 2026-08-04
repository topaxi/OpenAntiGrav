# `/psp-pure-eu/BOOT.BIN` - corroborated names

Same status as `psp-pure-usa`: not a reverse-engineering target, here for
opportunistic format corroboration only, see
[source-images.md](../../../reverse-engineering/source-images.md#pure-psp-euchd---wipeout-pure-psp-eu).
See [psp-pure-usa/corroboration.md](../psp-pure-usa/corroboration.md) for why
this binary mostly does not fuzzy-match against `psp-pulse-usa` the way the
EU Pulse binary does - the short version is that Pure and Pulse are different
executables from different SDK eras, so "shared engine lineage" mostly did
not hold as a match hypothesis here.

## `strcasecmp` -> `0x088dfe9c`

Confirmed two independent ways:

- Directly against `psp-pulse-usa`'s `strcasecmp` at `0x089732d8`
  (confidence 92, see [xml-reader.md](../psp-pulse-usa/xml-reader.md)):
  fuzzy match score 0.8664, 40/46 instructions equal.
- Against `psp-pure-usa`'s own `strcasecmp` at `0x088e05e8` (this page's
  sibling, [corroboration.md](../psp-pure-usa/corroboration.md)): fuzzy match
  score 0.9093, the expected same-title/cross-region shape seen for Pulse.

Both diffs show only literal-pool address differences, not logic changes.
Standard PSP SDK libc, not game code.
