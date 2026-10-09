# Where Pulse's language picker belongs is unevidenced

A cold boot of `pulse-psp-eu.chd` (2026-08-10, frames not kept in the repository) opens straight into `LogoFMV` playing `Data\Movies\Intro.PMF` and runs on to `Show Logo` **with no picker in between**. This build puts the picker between those two. The movie-first half is now confirmed correct - it had been recorded as merely "the order asked for" - but the picker's own position is not: what the disc does with no language saved was not observed, and the run had a savedata profile in place. Re-check with `UCES00465P0000` moved aside, the way [pure-boot.md](../../docs/architecture/pure-boot.md) records for Pure. Note Pulse's own `Skin.xml` *declares* the picker first and its runtime does not, which is the measurement ADR-0023 turns on.

## Open

- The picker's position in the boot chain is unconfirmed - the observed cold boot had a savedata profile in place.
- What the disc does with no language saved has not been observed.
- `Skin.xml` declares the picker first but the observed runtime does not match that declaration.

- HD, 2048 and Omega offer what their system-language chooser reaches (HD EU 12, 2048 EU 13, 2048 USA 3, Omega EU 14). Open: HD's Europe region value is by elimination (the writer of `0x938564` is not found, and `FUN_00235190` shows `AmericanLegalLine` when it is 2); Omega's retail store text lists 12 against 14; the PS4 extract keeps no `param.sfo` so Omega's release is assumed; a 2048 or Omega Asian SKU (modes/masks 2, 3, 4, 8, 16) has no row because none is held.
- Pure USA's `PI012` US-spelling overlay is loaded by the original and not applied here.
- Pulse's picker order is carried over from Pure's measured manifest order, not seen on Pulse.

## Next Steps

- Re-check the cold boot with `UCES00465P0000` moved aside to observe the no-saved-language case, the way [pure-boot.md](../../docs/architecture/pure-boot.md) records for Pure.
