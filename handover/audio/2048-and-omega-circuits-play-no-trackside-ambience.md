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
   circuit banks under `Data\audio\sound\` too (ported, location only:
   `oag_omega::race::SOUND_BANKS` carries it, so its log names the Wwise cause
   as one WARN line; nothing resolves until the Wwise lane maps cues).
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
6. ~~**What remains at WARN on 2048**~~ Proven dangling and moved: every
   remaining miss is the disc's own dangling reference (listing the bank's
   spelled names shows the cue absent: `env_alt~boat`, `env_mal~startline`,
   `env_squ~neoon_small` x44, `crowd~crowdf`, `env_tow~NGP_Tannoy_*`). Per-cue
   lines are debug (`node(s) dangle`) and each circuit gets one WARN summary,
   `docs/formats/2048-audio.md`. Still unread: whether the original falls back
   to another lookup (`Scream_FindSoundInBank`'s hashed path, and whether a
   label compare is case-insensitive: `GENTRAK`, `CHENGOU` and `generalt` stay
   unmatched).
7. ~~**2048's twelve downloadable circuits.**~~ Settled from the executable:
   `TrackStartup_Read` (`0x8121b688`) loads `data/audio/sound/<file>` then
   `data/audio/DLC1/<file>`, never beside the track. 212 of 468 resolve (was
   49); six cues in each `DLC1` bank have no command at all (reported as empty
   cues, debug). Still open: `~advert_fem_r01` binds opcodes `05 15 1a 16`
   nothing reads (WARN, a real decode gap); `Vineta_K` spells Pulse-era labels
   and resolves none.
8. ~~**HD's other circuits are unswept.**~~ Swept, all sixteen:
   `docs/formats/hd-audio.md`. HD's shared list was empty and wrong (the five
   banks sit under `Data\Sound\`, named by the executable); 13 circuits resolve
   most of their cones now. **Open:** `modesto_heights` (5 of 48) and
   `tech_de_ra` (2 of 62): both manifests are copies of Vineta K's and name
   `env1_vinetak.bnk`, while the nodes spell `env_mod`/`env_tec` and the right
   banks ship beside them (`env12_techdera.bnk`) or as the Zone banks. Nothing
   names them; not switched on. Whether the original plays those circuits silent
   is unmeasured (a Vita3K-style capture does not exist for HD; RPCS3 does).
9. ~~**The three `speech_*_NGP` banks.**~~ Not a framing variant: the waveform
   area is a pool of ATRAC9 stream file names (`.at9`), streamed from
   `data/audio/sound/streams/atrac/<language>/`. `Bank::parse` refuses with
   `Error::StreamNames`. **Open:** an ATRAC9 decoder, and what 2048's race does
   for speech meanwhile (the `speech` bank is one of the three).
10. **What applies to Omega** (not worked here): its Tech De Ra manifest names
    the right bank (`env12_techdera.bnk`), unlike HD's; its circuit directory is
    `Data\audio\sound\` (set). Once the Wwise lane maps a node's cue to an event,
    classify Omega's misses the way HD's were: the loader only demotes a miss to
    "dangling" when every bank parsed, so Omega's lines stay WARN until then.
    Omega's shared labels (`crowd`, `gentrak`, and HD's `voppler`/`radios`) need a
    census of which `.bnk` carries them.

## Next Steps

1. Omega via `oag_formats::wwise`: its own lane, a day or more. Start with
   whether a node's cue is `wwise::name_hash` of the spelled name.
2. RPCS3 on HD Tech De Ra and Modesto Heights: is there trackside ambience
   (a few hours)? It settles whether the stale manifests are played as written.
3. An ATRAC9 decoder for 2048's `.at9` speech streams (a day or more).
