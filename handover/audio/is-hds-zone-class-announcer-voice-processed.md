# Is HD's Zone speed-class announcer voice processed at runtime?

2026-10-06, maintainer's memory, unverified: the Zone speed-class announcer
sounds like "a distorted voice". The maintainer wants the distortion only if it
exists and there is evidence for it.

## What is established

- Only HD/Fury has a speed-class announcer: the fourteen `MR_*` cues in
  `Data\Sound\speech_class.bnk` (plus `ZONEMALE` at cue 0), also folded into
  `speech_zone.bnk` at cues 26-39. Pulse has no class ladder, so if the memory
  is of Pulse it is not this announcer. See `docs/formats/psp-audio.md`,
  "speech_zone.bnk names the zone announcer, one ladder per title".
- This port plays the disc's own waveforms unprocessed through
  `oag_sound::sfx::ClassAnnouncer`, on `Bus::Speech` at gain 1.0. All fourteen
  decode (`sfx_ground_truth.rs`,
  `wipeout_hd_s_speed_class_announcer_decodes_all_fourteen_cues`).
- Nobody has judged how the voice sounds, and nobody has read whether HD
  applies a DSP effect (filter, ring modulation, reverb send) to the speech
  bus or to these cues.

So there are two cases, and only the second needs work:
1. The processing is baked into the waveform. Then the port already
   reproduces it.
2. HD applies it at runtime. Then the port is missing it.

## Open

1. Settle which case it is without anyone listening:
   - **Disc side:** dump `MR_VEN` and a plain milestone cue (`zone_5`) from the
     same bank to WAV. Compare objectively: bandwidth, spectral sidebands, and
     whether the speech formants look carrier-modulated.
   - **Runtime side:** read HD's executable (`/hdfury/EBOOT-ps3-hdfury-eu.elf`)
     for anything between the class-announcer play call and the mixer that sets
     a filter, effect send or voice flag. Also read the SBlk v3 cue record's
     unread fields for a per-cue effect bit.
   - **Arbiter if both are unclear:** an RPCS3 audio capture of a real Zone
     class change, compared sample-for-sample against the raw waveform.
     Different is runtime processing; equal is baked in.
2. Only if case 2 is proven: implement that effect, measured, on HD's speech
   path (and Fury's), with its own ground truth.

## Next Steps

1. The disc-side WAV dump and objective comparison: about an hour, and needs
   a small cue-to-WAV example in `oag-sound`.
2. The runtime read, or the RPCS3 capture: half a day to a day.
