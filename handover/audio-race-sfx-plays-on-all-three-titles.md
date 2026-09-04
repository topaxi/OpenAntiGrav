# Audio: race SFX plays on all three titles

2026-08-23. The evidence is in [psp-audio.md](../docs/formats/psp-audio.md#a-cue-owns-a-run-of-the-command-table); this row is what is not there. **The cue-to-waveform blocker was one division** - `first_command = *(u32 *)(cue + 0x08) / 8` - and the base was settled by running seven candidates against every bank, only one of which makes the cue runs **tile the command table exactly** (83/83 banks, 1,282/1,282 cues). **PS2 and Pure came free**: 44 and 29 banks, accepted unmodified, same paths and cue names. **Two corrections to shipped claims:** (1) "Pure ships no `SBlk`" was a magic scan at offset 0, where the magic sits at `0x18`; that scan finds nothing on a Pulse disc either - **a magic-scan miss is only evidence against a positive control**, which the test now asserts. (2) `+0x0e`'s `0x80` means **NOT** ADPCM: `Sas_QueueSetVoice`'s only use of it is to print "THIS SYSTEM ONLY SUPPORTS ADPCM VOICE DATA", and 0 of 916/985/461/461 spans set it on the four discs. **Six cues are wired**, each with a recovered call site; ~1,280 more are decoded and deliberately unwired. **Three stated approximations, flagged where a reader hits each**: the sample rate is not recovered; which alternate sounds - decoded on HD 2026-08-27, corroborated on PSP 2026-09-04, wired into `Banks::pick` 2026-09-04; `~ENGINE`'s pitch is read as cents. **Positional audio closed this row's last line on 2026-08-24**; what stays open there is the *track's* own authored sound sources, not the craft.

## Open

- The sample rate is not recovered.
- ~~Which alternate sounds play is decoded on HD but not corroborated on
  PSP/PS2.~~ **Corroborated on PSP 2026-09-04** (`0x19`, confidence 88 both
  sides now) and wired into `Banks::pick`
  (`crates/game/src/audio/sfx/banks.rs`, 2026-09-04): a uniform draw that
  never repeats the immediately previous pick for a cue. PS2's own binary is
  not independently checked - it is grouped with PSP/Pure as the same
  library generation by operand byte layout, not by having been read itself.
- `~ENGINE`'s pitch is read as cents, an approximation.
- The track's own authored sound sources (as opposed to craft positional audio) remain open.

## Next Steps

- None of this thread's own.
