# 2048's and Omega's circuits play no trackside ambience

2026-10-06, found reading a `just play --race` log. `oag_sound::sfx::TrackEmitters::load`
resolves **0 of 41** emitters on 2048's Altima and **0 of 32** on Omega's Tech De Ra:
every authored `sound` node is silent on both titles. Three separate causes, each
measured with a throwaway probe over `oag_2048::open` / `oag_omega::open`.

## Open

Struck lines below landed 2026-10-06 (audio-2048); see
[2048-audio.md](../../docs/formats/2048-audio.md).

1. ~~**The circuit bank is not beside the track.**~~ Done for 2048:
   `TrackBanks::circuit_directory`, HD proven beside the track. Omega keeps its
   circuit banks under `Data\audio\sound\` too (checked, applies, not wired:
   `oag_omega::race::SOUND_BANKS` stays unset until the Wwise lane maps cues).
2. ~~**2048's banks are `SBlk` version 5.**~~ Done: the FNV-1 rule is proven on 33
   banks and 1,603 spelled names, the track path uses it, and the eight `env_*`
   banks that "did not span the blob" carry a stale waveform size (declared
   larger than shipped, every waveform inside), now read. Altima 40 of 41, ten
   base circuits 174 of 226. Still open: `Speech_NGP.bnk`, `speech_fe_NGP.bnk`,
   `speech_zone_NGP.bnk` declare a waveform length that is not a whole number
   of ADPCM blocks (915, 1,566, 1,242,097 bytes), unread.
3. **Omega's `.bnk` are Wwise (`BKHD`).** Checked, differs (format): the SBlk
   reader, hashed lookup and stale-size reading do not apply. A node's
   `bank~cue` is not mapped to a Wwise event; `oag_formats::wwise` reads the
   banks (`docs/formats/wwise.md`). Its own lane. The name of the cue is
   probably hashed too (`wwise::name_hash`); unchecked.
4. ~~**Shared banks.**~~ Done for 2048 (`crowd_NGP.bnk`, `generaltrack.bnk`) as
   `TrackBanks::shared`, a list with an `Origin`. Which Omega file carries
   `env_tec`/`techder` is still unread.
5. ~~**The root cause logs at `debug`.**~~ Done: `not read:` and `is not a sound
   bank:` lines say `plays nothing`, so they reach WARN with the cause.
6. **What remains at WARN on 2048, all the disc's own dangling references**
   (listing the bank's spelled names shows the cue is absent): `env_alt~boat`;
   `env_mal~startline` (mall's manifest loads `env_tower.bnk`, its nodes spell
   `env_mal`, which ships nowhere); `env_squ~neoon_small` x44 and
   `crowd~crowdf`; `env_tow~NGP_Tannoy_1/_3/_4` (the Tannoy cues are in
   `Speech_NGP_Grid.bnk`, which the nodes do not name). Whether the original
   falls back to another lookup is unread (`Scream_FindSoundInBank`'s hashed
   path on the Vita executable has not been decompiled for this).
7. HD's other circuits are unswept for emitters: only Talons Junction (66 of
   66 cones) was run.

## Next Steps

1. Omega via `oag_formats::wwise`: its own lane, a day or more. Start with
   whether a node's cue is `wwise::name_hash` of the spelled name.
2. The three `speech_*_NGP` banks that still refuse to parse (an hour, likely
   the same kind of size field).
3. Sweep HD's remaining circuits with `TrackEmitters::load`, half an hour.
