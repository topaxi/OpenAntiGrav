# Audio: race SFX plays on all three titles

2026-08-23. The evidence is in [psp-audio.md](../../docs/formats/psp-audio.md#a-cue-owns-a-run-of-the-command-table); this row is what is not there. **The cue-to-waveform blocker was one division** - `first_command = *(u32 *)(cue + 0x08) / 8` - and the base was settled by running seven candidates against every bank, only one of which makes the cue runs **tile the command table exactly** (83/83 banks, 1,282/1,282 cues). **PS2 and Pure came free**: 44 and 29 banks, accepted unmodified, same paths and cue names. **Two corrections to shipped claims:** (1) "Pure ships no `SBlk`" was a magic scan at offset 0, where the magic sits at `0x18`; that scan finds nothing on a Pulse disc either - **a magic-scan miss is only evidence against a positive control**, which the test now asserts. (2) `+0x0e`'s `0x80` means **NOT** ADPCM: `Sas_QueueSetVoice`'s only use of it is to print "THIS SYSTEM ONLY SUPPORTS ADPCM VOICE DATA", and 0 of 916/985/461/461 spans set it on the four discs. **Six cues are wired**, each with a recovered call site; ~1,280 more are decoded and deliberately unwired. **Three stated approximations, flagged where a reader hits each**: the sample rate is not recovered; which alternate sounds - decoded on HD 2026-08-27, corroborated on PSP 2026-09-04, wired into `Banks::pick` 2026-09-04; `~ENGINE`'s pitch is read as cents. **Positional audio closed this row's last line on 2026-08-24**; what stays open there is the *track's* own authored sound sources, not the craft.

## Open

- ~~The sample rate is not recovered.~~ **Recovered 2026-09-16, confidence
  95**: the descriptor's `+0x02`/`+0x03` centre note and fine through the
  engine's own note-to-pitch walk (`oag_formats::sblk::pitch`), 190 of 190
  live `sceSasSetPitch` hits reproduced to the bit. See
  [psp-audio.md](../../docs/formats/psp-audio.md#the-rate-each-waveform-plays-at).
  Still open from it: the PS2's own arithmetic (`SCREAM.IRX` unread, its
  byte-identical banks play through the PSP walk).
- ~~HD plays every SFX through the PSP's own scale and base rate, unverified
  for HD.~~ **HD's own walk read 2026-09-16, confidence 90**: `Scream_KeyOnVoice`
  (`ps3-hdfury-eu`, `0x00630310`) runs the identical three-function chain on
  byte-identical semitone/fine tables, with its own negative-centre scale
  (`0x10f4a`, not the PSP's `0x1278b`) and its own core rate (48,000 Hz, not
  44,100) - both read from disassembly, the base rate independently
  re-resolved through the per-function TOC. `Sound::pitch`/`sample_rate` now
  switch on the bank's own byte order. See
  [ps3-hdfury-eu/sound.md](../../docs/ghidra/functions/ps3-hdfury-eu/sound.md#the-pitch-hds-own-scale-and-base-rate-2026-09-16)
  and [psp-audio.md](../../docs/formats/psp-audio.md#the-rate-each-waveform-plays-at).
  Still open from it: no runtime trace on this binary (an RPCS3 breakpoint at
  `_opd_FUN_0060e3d0` would settle it), and HD's volume byte still reads
  `-27..=127` against the PSP's documented `60..=127`, unresolved.
- ~~Which alternate sounds play is decoded on HD but not corroborated on
  PSP/PS2.~~ **Corroborated on PSP 2026-09-04** (`0x19`, confidence 88 both
  sides now) and wired into `Banks::pick`
  (`crates/game/src/audio/sfx/banks.rs`, 2026-09-04): a uniform draw that
  never repeats the immediately previous pick for a cue. PS2's own binary is
  not independently checked - it is grouped with PSP/Pure as the same
  library generation by operand byte layout, not by having been read itself.
- ~~`~ENGINE`'s pitch is read as cents, an approximation.~~ **Settled the
  other way, 2026-09-16**: the unit is 1/128 of a semitone, 1536 to the
  octave, confidence 92 - the same live capture caught the engine at rest
  with an offset of `-1148` and a pitch word that is `2^(-1148/1536)` of its
  descriptor's rate. `crates/game/src/audio/sfx/engine.rs` divides by 1536
  now. Still not carried: the Doppler term.
- The track's own authored sound sources (as opposed to craft positional audio) remain open.

## Next Steps

- `FUN_0898f6a4` (the branch's "existing instance" lookup), `FUN_089929a4`
  (the parent-child link) and `FUN_08993944` (the kill) are the three the
  child opcodes lean on and are unnamed; the seven opcodes the Pulse banks
  use with no named handler (`0x0a`, `0x16`, `0x17`, `0x18`, `0x1c`, `0x28`,
  `0x2b`) are 65 of `Data.wad`'s 2,881 commands. See
  [sound.md](../../docs/ghidra/functions/psp-pulse-usa/sound.md#not-determined).
