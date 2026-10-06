# Where Pulse's language picker belongs is unevidenced

A cold boot of `pulse-psp-eu.chd` (2026-08-10, frames under `data/shots/pulse-cold-boot-2026-08-10/`) opens straight into `LogoFMV` playing `Data\Movies\Intro.PMF` and runs on to `Show Logo` **with no picker in between**. This build puts the picker between those two. The movie-first half is now confirmed correct - it had been recorded as merely "the order asked for" - but the picker's own position is not: what the disc does with no language saved was not observed, and the run had a savedata profile in place. Re-check with `UCES00465P0000` moved aside, the way [pure-boot.md](../../docs/architecture/pure-boot.md) records for Pure. Note Pulse's own `Skin.xml` *declares* the picker first and its runtime does not, which is the measurement ADR-0023 turns on.

## Open

- The picker's position in the boot chain is unconfirmed - the observed cold boot had a savedata profile in place.
- What the disc does with no language saved has not been observed.
- `Skin.xml` declares the picker first but the observed runtime does not match that declaration.

- HD, 2048 and Omega load every language plugin on the disc; whether their executables narrow the offered list (Omega's store lists 12 of 23) is unread.
- Pure USA's `PI012` US-spelling overlay is loaded by the original and not applied here.
- Pulse's picker order is carried over from Pure's measured manifest order, not seen on Pulse.

## Next Steps

- Re-check the cold boot with `UCES00465P0000` moved aside to observe the no-saved-language case, the way [pure-boot.md](../../docs/architecture/pure-boot.md) records for Pure.
