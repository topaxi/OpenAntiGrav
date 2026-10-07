# The PSN Wipeout HD download opens and races; its menu art is Fury's and absent

2026-10-07. `NPEA00057` v3.00 (plain HD, no Fury), installed with RPCS3 and copied to
`data/extracted/ps3/hd-psn-eu/`, is a source: found by shape (`PARAM.SFO` beside
`USRDIR/`, Europe first), opened as `Wipeout HD` through `oag_hd::psn::PSN` (own race
default, Zone shape, exhaust ribbon and no Fury selection pickers), 6,674 entries over
`data01`-`data04`, no patch to mount. Race on Vineta K is tick-for-tick the disc's.
Census, route and every row of the variant: [hd-psn.md](../../docs/formats/hd-psn.md);
tests: `crates/hd/tests/hd_psn_ground_truth.rs` (5), `crates/source/tests/psn_source_ground_truth.rs` (1).

## Open

- Menu boxes draw as nothing: `file2.gtf` is `DATA06`'s. `file.gtf` (4,224 bytes, both
  sources) is a different size from the 64x64 the nine-patch code assumes; what the PSN
  build draws there is unmeasured (its executable is an encrypted SELF).
- No menu backdrop (Fury point clouds are `DATA06`'s).
- The race box has plain TRACK and TEAM rows, not the pickers: the package's older
  `Selection_Definition.xml` is a different dialect no picker reads.
- `SameCircuit` Zone, `DATA03`-before-`DATA02` mount order and the plain exhaust
  template are all chosen, not measured.
- The PSN package was installed with the licence file already in place; installing
  without it was not tried.
- Five effects the disc's `DATA02` ships are absent from the PSN `data02`.

## Next Steps

- Boot the install under RPCS3 (own config, muted) and capture the main menu, to see what
  draws the boxes and backdrop, and whether the Zone mode runs on every circuit.
- Read `Selection_Definition.xml` with a picker so the PSN race box gets its real screens.
- Try the install without the licence file and say so in `installing.md`.
