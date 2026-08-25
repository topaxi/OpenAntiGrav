# `Movie::entry_name` hardcodes `_US`, so a European Pure disc shows the American card

`crates/game/src/screen.rs`'s `entry_name` appends `_US.PMF` to **every** `localised="true"` widget on every source. That was defensible while `_US` was the only known name; all four cuts of both Pure movies are now recovered and sit in `oag_pure::names::INTRO_MOVIE_CUTS` / `FMV_INTRO_MOVIE_CUTS` - `_EU`, `_US`, `_JAP`, `_KO`, each verified by resolving it against a real WAD entry. Both Pure pressings carry the first three; the Korean cut is on the EU disc only. Frame 144 of the `_EU` cut reads EUROPE where `_US` reads AMERICA, so booting `pure-psp-eu.chd` through this build currently shows the wrong regional card. The fix wants a region to resolve against; the mechanism the original uses is still open. **Side effect worth banking**: these names close the question at `docs/ghidra/functions/psp-pulse-usa/frontend-video.md`'s "no name produces those" for all three Pulse-side reel hashes, and `oag_pulse::names::DEVPUB_REEL` is now the name rather than `hash:b1ba72c3`. They are owed to a `names.tsv` - but that file is `psp-pulse-usa` only, and these are entry names rather than symbols, so decide where they belong before adding rows.

## Open

- The mechanism the original game uses to resolve a movie's region is unknown
- `Movie::entry_name` still hardcodes `_US`, so `pure-psp-eu.chd` shows the wrong regional card
- Where the newly-recovered entry names belong in `names.tsv` is undecided (that file is `psp-pulse-usa`-only and holds symbols, not entry names)

## Next Steps

- Recover the mechanism the original uses to resolve a movie's region, then fix `Movie::entry_name` to use it instead of hardcoding `_US`
- Decide where the recovered entry names belong before adding rows to `names.tsv`
