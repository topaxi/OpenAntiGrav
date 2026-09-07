# `/psp-pure-usa`, `/psp-pure-eu` and `/psp-pure-eu-reimport` were missing from the project on 2026-08-10, despite the 2026-08-09 note describing them as already imported and saved

**Most likely just a different workstation's copy of the `OpenAntiGrav` Ghidra project**, not data loss - the project is per-machine (`data/ghidra/` is gitignored, per-checkout), so two workstations naturally hold independent Ghidra databases under the same project name. Not confirmed which machine wrote the 2026-08-09 note. Whatever the cause, the two rows are a full, independent redo done on 2026-08-10, not a continuation of the earlier one - and it landed the same swap the earlier note asked for. **If this happens again on a workstation that expects the earlier state, check which machine before assuming loss.**

## Open

- Not confirmed which machine wrote the 2026-08-09 note describing the imports as already saved

## Next Steps

- If this happens again on a workstation expecting the earlier state, check which machine wrote it before assuming data loss
