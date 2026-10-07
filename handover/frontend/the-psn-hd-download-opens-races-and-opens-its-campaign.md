# The PSN Wipeout HD download opens, races and opens its campaign; its backdrop is absent

2026-10-07. `NPEA00057` v3.00 (plain HD, no Fury), installed with RPCS3 and copied to
`data/extracted/ps3/hd-psn-eu/`, is a source: found by shape (`PARAM.SFO` beside
`USRDIR/`, Europe first), opened as `Wipeout HD` through `oag_hd::psn::PSN` (own race
default, Zone shape, exhaust ribbon and no Fury selection pickers), 6,674 entries over
`data01`-`data04`, no patch to mount. Race on Vineta K is tick-for-tick the disc's.
Census, route and every row of the variant: [hd-psn.md](../../docs/formats/hd-psn.md);
tests: `crates/hd/tests/hd_psn_ground_truth.rs` (5), `crates/source/tests/psn_source_ground_truth.rs` (1).

## Open

- RACE CAMPAIGN opening nothing on PSN: closed 2026-10-07: it opens the base campaign
  directly (no HD/Fury chooser), same screens and grids as the disc's HD branch; measured on
  RPCS3. Left open from it: `Cell Selection` art against RPCS3 (emblems and value colours,
  shared with the disc; the `TARGET (NOVICE)` header, PSN only), and the grid source
  (`data02` flat is read on both sources, the PS3's overlay would read `data04`'s per-difficulty).
- No menu backdrop (the Fury point clouds are `DATA06`'s and the executable names none).
- The race box has plain TRACK and TEAM rows, not the pickers: the package's older
  `Selection_Definition.xml` is a different reader no picker uses.
- `SameCircuit` Zone, `DATA03`-before-`DATA02` mount order (72 shared files differ) and the
  default circuit are chosen, not measured. Exhaust and menu frame are read off the decrypted
  PSN executable's literals (85); the pointer table was not walked.
- The install was made with the licence file in place; installing without it was not tried.
- Five effects the disc's `DATA02` ships are absent from the PSN `data02`.

## Next Steps

- Boot the install under RPCS3 (own config, muted) and capture the main menu, to confirm the
  frame, palette and Zone behaviour.
- Read `Selection_Definition.xml` (`data03`'s 62,322-byte copy is the one served; `data02`'s is the disc's `DATA02` copy) so PSN gets its race box pickers.
- Try the install without the licence file and say so in `installing.md`.
