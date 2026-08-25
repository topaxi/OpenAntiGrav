# HD's `.bnk` sound banks read, and HD makes five of the six sounds

2026-08-23; this row's refusal is discharged. It held that a framing-only relaxation would leave `sounds()`, `sound_names()` and `decode_adpcm()` unverified - all three now have numbers, in [psp-audio.md](../docs/formats/psp-audio.md#wipeout-hd-the-same-container-byte-swapped-whole) not repeated here: 50 entries parsed 50/50, cue runs and spans and names all tiling 50/50, 0 parsing little-endian. **Both framing differences are alignment**; the tail stays exact. **A third of HD's waveforms are not PS-ADPCM**, and `+0x0e`'s `0x80` says which (99.98% vs 7.19% in spec); **that codec is unidentified** (no head magic, not 16-bit PCM either way; MP3/ATRAC3 untested) and nothing decodes it. **The game side is a five-field path table, not an axis**: cue strings are identical lineage-wide, only the *file* moves, so `oag_title::SoundBanks` carries it per title (HD has no `hud.bnk`; `SPEEDUPPAD` is in `weapons.bnk`, the ship bank `shiphd.bnk`). **One cue misses on HD, and says why**: `~ENGINE` does not exist there (HD's ship audio is a per-event `c_*` set). `.COLLISIONS` was the second miss until 2026-08-24. **The caveat**: every trigger is a *Pulse* reading applied to another game - no Pure or HD dispatch has been looked at - recorded at confidence 50 in `audio::sfx`'s docs. `hd_sound_ground_truth.rs`.

## Open

- A third of HD's waveforms are not PS-ADPCM and use an unidentified codec (no head magic, not 16-bit PCM either way, MP3/ATRAC3 untested)
- `~ENGINE` cue does not exist on HD (HD uses a per-event `c_*` set instead)
- Every trigger is a Pulse reading applied to HD at confidence 50 - no Pure or HD dispatch has been looked at

## Next Steps

- Identify and decode the unidentified codec used by a third of HD's waveforms
