# A cue can play other cues, and that is what `.COLLISIONS` does on HD

2026-08-24. HD's `.COLLISIONS` was recorded as binding no waveform because "all four commands are among the 43 unread opcodes" - true, and a dead end wearing the shape of a finding. **Two of the four are `0x08`, which plays another cue**; `0x05` is the same thing. Both carry a parameter-block offset to a **32-byte** record: `+0x00` volume, `+0x0c` a cue index *or* `0xffffffff`, `+0x10` a 16-byte name. Full evidence in [psp-audio.md](../../docs/formats/psp-audio.md#a-cue-that-plays-other-cues); the numbers are 1,461 child grains across six discs, **1,153 indexed** (every PSP/PS2 one), **300 named** (every HD one), **1 cross-bank**, **7 malformed**, and **0 carrying both forms** - that exclusivity is the load-bearing assertion. Corroborated by strings in HD's own `EBOOT.elf`, which ships SCREAM with debug text: `0x007cfaa0` "Didn't find child sound named -> %s" and `0x007cfad0` "snd_SFX_GRAIN_TYPE_BRANCH invalid sound index %d" - one per form, and `weapons_det.bnk` really does hold index 65 in a 55-cue bank. Confidence **85**; not 94 because **the handler was never located** and `0x05` vs `0x08` is therefore undecided. **HD now has a collision sound**: the tree is `.COLLISIONS -> c_CShipShip|c_CShipWall -> each S/M/L`, 112 PS-ADPCM waveforms. **The severity and ship/wall splits are deliberately not wired.** The guard is `0x22`, undecoded, so `cue_tree_sounds` returns every reachable leaf and `audio::sfx` picks with its own generator - the existing alternate-selection approximation one level down. Pairing `L/M/S` against `sparks::severity`, or ship-vs-wall against the contact type, would be a mapping invented here. **Also**: `0x19` counts a key-on group and the count byte sits at **opposite ends of the operand in the two library generations** (low byte on PSP/PS2/Pure, high byte on HD; the middle byte is voices per alternate, 1 mono 2 stereo). `b * count` predicts 530 of the 576 runs. The old "61 of 87" on this rule came from a walk that ran past the cue boundary and is corrected. Counting a group is still not choosing from it, so **alternate selection remains open**. **Ghidra dead end worth not repeating**: HD's `EBOOT.elf` is open and full of SCREAM strings, but they are reachable only through a pointer table at `0x008c0060` that no code xref resolves - r2 is `0x008ad4d8` (from the `.opd` at the ELF's `e_entry`, `0x00870530`) and the table is far outside its +/-0x8000 window, so `snd_DoGrain` was never found. Anyone wanting `0x22` should start there. `crates/formats/src/sblk/child.rs`, `sblk_child_ground_truth.rs`.

## Open

- ~~The handler was never located, so `0x05` vs `0x08` is undecided.~~ **Located
  2026-08-27**: both are read, and they differ (`0x05` plays the child with a
  computed volume and pan, `0x08` bounds-checks and replaces this voice's own
  state). [ps3-hdfury-eu/sound.md](../../docs/ghidra/functions/ps3-hdfury-eu/sound.md).
- ~~Severity and ship/wall splits are deliberately not wired - guard `0x22` is
  undecoded.~~ **`0x22`'s opcode is decoded 2026-08-27**: a three-way compare
  of a named interpreter variable against an immediate, skipping the grain
  unless it holds - see the sound page above. **Still not wired, and now for a
  structural reason rather than a missing read**: the variable's value is
  runtime voice state (`voice->local_vars` and a runtime global,
  `sysvar_table`) that a static WAD parse cannot see, and which `var_ref`
  value means severity or ship-vs-wall is still unknown. `cue_tree_sounds`
  returning every reachable leaf is the shape the format needs, not a
  placeholder for a static answer this thread could now supply.
- ~~Alternate selection remains open - `0x19` counts a key-on group but does
  not choose from it.~~ **Decoded 2026-08-27**, while confirming the dispatch
  table's base rather than by design: random pick among the group, never
  repeating the immediately previous pick. ~~Wiring it into
  `oag_sound::sfx::Banks::pick` is real, unblocked work now~~ **wired
  2026-09-04**, matching the re-roll-once-on-repeat shape rather than a naive
  retry loop; see `Banks::pick`'s own doc comment in
  `crates/sound/src/sfx/banks.rs`.
- ~~Corroborate `0x22`/`0x23`/`0x24`/`0x19` on the PSP side.~~ **Done
  2026-09-04**: the blocker was never the code, it was the address - the
  table stores `.text`-relative offsets the auto-analyzer never walked, so
  `real = pseudo + 0x08804000` (this binary's own image base) unblocks
  `decompile_function`/`create_function` outright. All four now read on
  `psp-pulse-usa`, field for field the same as HD's own readings. See
  [ps3-hdfury-eu/sound.md#corroborated-on-psp-2026-09-04](../../docs/ghidra/functions/ps3-hdfury-eu/sound.md#corroborated-on-psp-2026-09-04)
  and the fuller writeup on
  [psp-pulse-usa/sound.md](../../docs/ghidra/functions/psp-pulse-usa/sound.md#four-opcodes-corroborated-against-hd-2026-09-04).
  PS2's own binary (a third, separate SCREAM build) is not independently
  checked - it is grouped with PSP/Pure as "the same library generation" by
  operand byte layout, not by having been read itself.

## Next Steps

- Read `sysvar_table` to learn which `var_ref` value is severity and which is
  ship-vs-wall. On HD it is a runtime pointer (`0x008c0038` in `EBOOT.elf`)
  needing a live process. **PSP's equivalent is a static address, found
  2026-09-04 tracing `Scream_OpGuard`/`Scream_OpGoto`'s own constants**:
  `Scream_OpGuard`'s `0x2bf247 - iVar3` and `Scream_OpGoto`'s recursion-depth
  counter `_DAT_002bf268` agree on one base (`0x2bf248` raw, `0x08ac3268 -
  0x08ac3248 = 0x20` matching HD's own "`sysvar_table + 0x20`" for the same
  counter) - `0x08ac3248` corrected, reading as 36 bytes of zero in the
  static image immediately before `g_scream_opcode_table`
  (`0x08ac326c`), consistent with runtime-populated state rather than
  disc-authored data. `get_xrefs_to` on both the raw and corrected addresses
  found no references this session, which reads as writes computed rather
  than a literal load `xref` analysis catches - a byte-pattern search for the
  `lui`/`addiu` pair building `0x08ac3248` (or a live trace, same as HD)
  is what finding the writer needs next. Not chased further this session.
