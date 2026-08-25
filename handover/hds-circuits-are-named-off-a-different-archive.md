# HD's circuits are named off a different archive from the one that lists them

2026-08-24, a trap to know rather than a next step. `DATA00` declares the 28 circuits as `NN_Track`; the *served* `entries.xml` (`DATA02`'s) keys their names in the base game's numbering, so folding case against it names eight circuits wrongly. HD ships the file five times and only `DATA06`'s covers all 28. `oag_game::language::CircuitNames` picks on coverage and folds on that copy alone; the rest of the front end keeps the served table. **Still open**: which copy the runtime serves - the same question `skin.xml`'s six copies pose. Evidence and the pinning test: [hd-frontend.md](../docs/formats/hd-frontend.md#a-circuits-name-is-in-a-different-archive-from-the-circuit-list).

## Open

- Which archive copy the runtime actually serves is unknown (same open question as `skin.xml`'s six copies)

## Next Steps

- Determine which of HD's five copies of the circuit-name file the runtime actually serves
