# HD's `.bnk` sound banks read, and HD makes eight of the nine sounds

2026-08-23; this row's refusal is discharged. It held that a framing-only relaxation would leave `sounds()`, `sound_names()` and `decode_adpcm()` unverified - all three now have numbers, in [psp-audio.md](../docs/formats/psp-audio.md#wipeout-hd-the-same-container-byte-swapped-whole) not repeated here: 50 entries parsed 50/50, cue runs and spans and names all tiling 50/50, 0 parsing little-endian. **Both framing differences are alignment**; the tail stays exact. **A third of HD's waveforms are not PS-ADPCM**, and `+0x0e`'s `0x80` says which (99.98% vs 7.19% in spec). **The game side is a five-field path table, not an axis**: cue strings are identical lineage-wide, only the *file* moves, so `oag_title::SoundBanks` carries it per title (HD has no `hud.bnk`; `SPEEDUPPAD` is in `weapons.bnk`, the ship bank `shiphd.bnk`). **One cue misses on HD, and says why**: `~ENGINE` does not exist there (HD's ship audio is a per-event `c_*` set). `.COLLISIONS` was the second miss until 2026-08-24. **The caveat**: every trigger is a *Pulse* reading applied to another game - no Pure or HD dispatch has been looked at - recorded at confidence 50 in `audio::sfx`'s docs. `hd_sound_ground_truth.rs`.

2026-09-02: **the second codec is identified** - 16-bit PCM, big-endian, behind a 16-byte header. `oag_formats::sblk::decode_pcm16` decodes it and `oag_game::audio::sfx` now plays it instead of dropping it. Confidence 85: a primary-source string in the PS3 executable (`"SCREAM: ERROR! Unknown voice type in bank - must be ADPCM or PCM"`, `0x007d0818` in `ps3-hdfury-eu/EBOOT.elf`) names PCM as SCREAM's second voice type, and a per-span roughness measurement (mean 0.257 against ~1.41 for white noise, over all 1,167 not-PS-ADPCM spans) agrees. `~ROCKLOCK` - a cue every one of whose waveforms was in the second codec - went from failing to load at all to loading both, the clearest end-to-end proof. Full evidence in [psp-audio.md](../docs/formats/psp-audio.md#a-third-of-hds-waveforms-are-not-ps-adpcm-and-are-16-bit-pcm).

## Open

- `~ENGINE` cue does not exist on HD (HD uses a per-event `c_*` set instead)
- Every trigger is a Pulse reading applied to HD at confidence 50 - no Pure or HD dispatch has been looked at
- The second codec's confidence is capped at 85: no code reference to the error string that names it was found (only the data table holding it), so the function that actually dispatches on voice type is still `FUN_*`, unnamed

## Next Steps

- In Ghidra, with the PS3 program open: find what reads the pointer table at `0x008c0300` (or walks to `0x007d0818` some other way) and reaches SCREAM's non-ADPCM voice setup, to move the codec identification from "Confident" (85) toward "Established" (95+) with an actual decompiled call site
- No Pure or HD dispatch has been looked at for the SFX trigger provenance caveat above; either would raise it past confidence 50
