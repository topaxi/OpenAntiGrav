# Audio: race SFX plays on all three titles

2026-08-23. The evidence is in [psp-audio.md](../docs/formats/psp-audio.md#a-cue-owns-a-run-of-the-command-table); this row is what is not there. **The cue-to-waveform blocker was one division** - `first_command = *(u32 *)(cue + 0x08) / 8` - and the base was settled by running seven candidates against every bank, only one of which makes the cue runs **tile the command table exactly** (83/83 banks, 1,282/1,282 cues). **PS2 and Pure came free**: 44 and 29 banks, accepted unmodified, same paths and cue names. **Two corrections to shipped claims:** (1) "Pure ships no `SBlk`" was a magic scan at offset 0, where the magic sits at `0x18`; that scan finds nothing on a Pulse disc either - **a magic-scan miss is only evidence against a positive control**, which the test now asserts. (2) `+0x0e`'s `0x80` means **NOT** ADPCM: `Sas_QueueSetVoice`'s only use of it is to print "THIS SYSTEM ONLY SUPPORTS ADPCM VOICE DATA", and 0 of 916/985/461/461 spans set it on the four discs. **Six cues are wired**, each with a recovered call site; ~1,280 more are decoded and deliberately unwired. **Three stated approximations, flagged where a reader hits each**: the sample rate is not recovered; which alternate sounds is undecoded (`0x19` is the lead, 61 of 87); `~ENGINE`'s pitch is read as cents. **Positional audio closed this row's last line on 2026-08-24**; what stays open there is the *track's* own authored sound sources, not the craft.

## Open

- The sample rate is not recovered.
- Which alternate sounds play is undecoded (`0x19` is the lead, 61 of 87).
- `~ENGINE`'s pitch is read as cents, an approximation.
- The track's own authored sound sources (as opposed to craft positional audio) remain open.

## Next Steps

- Chase `0x19` to decode alternate-sound selection (currently the lead, 61 of 87 explained).
