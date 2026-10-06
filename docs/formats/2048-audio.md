# 2048's circuit sound banks and trackside ambience

Wipeout 2048 (Vita) keeps its sound banks as `SBlk` **version 5**
([2048-xfx.md](2048-xfx.md) has the container and the FNV-1 name rule). This
page is what a circuit's authored `sound` nodes need to find a cue in one, and
what the disc ships for them. The location rule is read off the executable's own
loader ([`TrackStartup_Read`](../ghidra/functions/vita-2048-eu-v104/track-startup-sound-bank.md),
confidence 85); the rest is a census of
`data/extracted/vita/PCSF00007/base/PSP2/data.psarc` and the ground truths named
below.

## Where a circuit's banks are

| Claim | Evidence |
| --- | --- |
| A circuit's own bank is `data/audio/sound/<LoadSoundBank filename>`, or, when that file does not exist, `data/audio/DLC1/<filename>`. **Never beside the track.** | `TrackStartup_Read` formats those two paths and has no other (confidence 85, the existence test inferred). `env_altima.bnk` reads under `sound`; the twelve downloadable circuits' beside-the-track copies are dead data. |
| Four downloadable circuits (`Vineta_K`, `Anulpha_Pass`, `Chenghou_Project`, `Moa_Therma`) name a bank that also ships under `sound`, so the base copy wins; the other eight read `DLC1`. | `the_downloadable_circuits_read_sound_first_then_dlc1_never_beside_the_track`. |
| Two shared banks hold the other labels a node spells: `crowd_NGP.bnk` (label `crowd`) and `generaltrack.bnk` (`gentrak`, **0 cues on 2048**). | The executable names both literals; Altima's `crowd~crowd` nodes resolve in the first. |
| HD keeps resolving beside the track. | [hd-audio.md](hd-audio.md): every HD circuit. |

This is `oag_title::TrackBanks` (`shared`, `circuit`, `origin`) on
`oag_title::SoundBanks::track`; `circuit` is `CircuitBanks::BesideTrack`
(Pulse, Pure, HD) or `CircuitBanks::Directories` (2048:
`Data\audio\sound`, `Data\audio\DLC1`; Omega: `Data\audio\sound`). Pulse and
Pure carry `generaltrack.bnk` as a one-element list, HD five banks, Omega none.

## The name rule, proven on every bank that parses

A node spells its cue (`~neoon_small`, `~startline`) and a v5 bank keys its
table by `name_hash`. The name block carries 16-byte records from `+0x10`
(`{u32 name offset, u32 hash, u16 cue, u16 next, u32 index}`) and a string pool
at the offset `+0x0c` gives, so a bank **spells** most of its names beside the
hashes: `Bank::hashed_names`. Across **33 hashed banks of the base `data.psarc` (the DLC and patch archives were not censused) and 1,603 spelled
names every name hashes to its own record's hash**
(`sblk_v5_hashed_names_ground_truth`), which takes the rule from "32 names" to
exhaustive. Confidence 98.

The track path alone uses the hashed lookup (`load_track_cue`,
`Bank::cue_named_or_hashed`); the race's own cues keep `Bank::cue_named`, whose
2048 triggers are unchecked (the 2026-10-05 trap: a hashed lookup everywhere
played a perfect-lap announcement mid-lap).

## `env_altima.bnk` and the others: a stale size field, not a section

`sblk::Bank::parse` refused eight `env_*` banks ("the section table does not
span the blob exactly"): `env_altima`, `env_arena`, `env_bridge`,
`env_cathedral`, `env_sol`, `env_square`, `env_subway`, `env_tower`. The
section table's waveform size and the descriptor's `+0x28`/`+0x2c` **agree with
each other and exceed the bytes behind them** (Altima: `0x26c020` declared,
`0x172860` shipped). It is a size field and not a missing section: every
waveform each bank binds ends inside what is shipped, **the last at the file's
end**, in all eight, and the six that share a template (`env_altima`,
`env_arena`, `env_bridge`, `env_cathedral`, `env_subway`, `env_tower`) carry the
same waveform bytes as `env_park.bnk`, which parses (121 bytes differ, all in
the descriptor). `Bank::parse` now reads the section as the file holds it and
keeps the claim in `Bank::declared_waveform_len`; a short file whose
waveforms reach past its end is still refused. Confidence 90.

## `speech_*_NGP`: a pool of stream names, not ADPCM

`Speech_NGP.bnk`, `speech_fe_NGP.bnk` and `speech_zone_NGP.bnk` declare a
waveform length (1,566, 915 and 1,242,097 bytes) that is no multiple of a
16-byte block, and **it is not a framing variant**: the waveform area holds a
pool of NUL-terminated ATRAC9 stream file names (`WOVO_FE_01.at9`, 61 of them in
`speech_fe_NGP`, 83 in `Speech_NGP`, 52 in `speech_zone_NGP`) after a zero
prefix. The speech is streamed from
`data/audio/sound/streams/atrac/<language>/<name>.at9`, and `WOVO_FE_01.at9`
exists there (24,676 bytes). `speech_zone_NGP` carries 1.2 MB of data after its
names that is not PS-ADPCM either (not decoded here). So `Bank::parse` refuses
them with `Error::StreamNames { size, names }` instead of
`PartialAdpcmBlock`, naming the cause, and no ADPCM decoder is ever pointed at a
name pool (that would play noise). Decoding the `.at9` streams needs an ATRAC9
decoder this project does not have; open. Pinned by
`sblk_v5_hashed_names_ground_truth::a_speech_bank_refuses_as_a_pool_of_stream_names`.

## How many of a circuit's emitters resolve

`track_audio_2048_ground_truth` (`#[ignore]`d, `just test-data`) pins every
base-package circuit:

| Circuit | Emitters | Resolve | Unresolved |
| --- | --- | --- | --- |
| altima | 41 | 40 | `env_alt~boat` |
| arena | 3 | 3 | |
| bridge | 4 | 4 | |
| cathedral | 12 | 12 | |
| mall | 5 | 3 | `env_mal~startline` x2 |
| park | 2 | 2 | |
| sol | 57 | 57 | |
| square | 45 | 0 | `env_squ~neoon_small` x44, `crowd~crowdf` |
| subway | 9 | 9 | |
| tower | 48 | 44 | `env_tow~NGP_Tannoy_1`, `_3` (x2), `_4` |

Every unresolved reference is the disc's own dangling one, found by listing the
named bank's spelled names rather than by a failed lookup: Altima's bank holds
exactly `~advert_fem_l01 ~advert_mle_r01 ~startline ~Heli_2 ~Heli_1
~neoon_small ~CITY ~neon_big ~INTERNAL ~wind_high`, no boat. `mall`'s manifest
loads `env_tower.bnk` (label `env_tow`) while its nodes spell `env_mal`, a bank
that ships nowhere. The Tannoy cues exist, in `Speech_NGP_Grid.bnk`, but the
nodes name `env_tow`; resolving them there would be inventing a join. All are reported once per circuit as one WARN summary line, "N cue(s) the circuit names and its banks do not author", and per cue at debug.

## The twelve downloadable circuits

Read the way the executable does (`sound` first, then `DLC1`, never beside the
track), the twelve resolve **212 of 468** emitters, where the beside-the-track
v3 copies gave 49 (those parse as v3 banks with no name table). Pinned by
`the_downloadable_circuits_read_sound_first_then_dlc1_never_beside_the_track`:

| Circuit | Emitters | Resolve | Bank read from |
| --- | --- | --- | --- |
| Metropia | 13 | 4 | `DLC1` |
| Sebenco_Climb | 37 | 9 | `DLC1` |
| Sol_2 | 20 | 10 | `DLC1` |
| Ubermall | 131 | 29 | `DLC1` (101 `voppler~voppler` dangle: 2048 has no `voppler.bnk`) |
| amphiseum | 25 | 23 | `DLC1` |
| modesto_heights | 23 | 19 | `DLC1` |
| talons_junction | 57 | 54 | `DLC1` |
| tech_de_ra | 32 | 29 | `DLC1` |
| Vineta_K | 48 | 0 | `sound` (nodes spell the Pulse-era `VINETTA` and `GENTRAK`) |
| Anulpha_Pass | 52 | 18 | `sound` |
| Chenghou_Project | 13 | 12 | `sound` |
| Moa_Therma | 17 | 5 | `sound` |

Most of the gap to 100% is cues with **no command at all**: six of the 63 cues in
each `DLC1` 63-cue bank (`~neoon_small`, `~eleccarrier`, `~ind_factory_1`, ...)
author nothing: the raw cue record of every one is the same, `first_command` `0x1fffffff` (the no-command sentinel, `raw` `0xfffffff8`), `commands` 0, `flags` 0, volume 120, where Altima's playing `~neoon_small` is `first_command` 21, `commands` 2, `flags` 9. So `~neoon_small` is silent on Metropia, Sebenco and Sol 2 where
Altima's own `~neoon_small` plays. They are reported as an *empty cue* at debug,
alongside the no-op and register-write control cues, and are not counted
dangling. Cues with opcodes `0x05 0x15 0x1a 0x16` (`~advert_fem_r01`) bind no
waveform and stay at WARN: a genuine decode gap, not by design.

Heard, not only resolved: a 40 s Altima autopilot lap rendered to WAV through
the real mixer, once with its emitters and once with them dropped
(`altima-ambience.wav`, scratch). The 861 ticks with no emitter in range or in
the ten before it are byte-identical in both, and 1,506 of the 1,539 ticks near
one differ, up to the mix's full peak; the mix clips 0 samples with or without the emitters.

## Checked against Omega

**Checked, differs (format), agrees (directory).** Omega's `.bnk` are Wwise
(`BKHD`), not `SBlk`: none of `sblk::Bank::parse`, the hashed lookup or the
stale-size reading applies, and a node's `bank~cue` pair is not yet mapped to a
Wwise event. Omega keeps its circuits' banks under `Data\audio\sound\` too, so
`oag_omega::race::SOUND_BANKS` carries `CircuitBanks::Directories` for it (**ported**,
location only): its load report now names the real cause, `env12_techdera.bnk is
not a sound bank: unsupported bank version 1145588546` (`BKHD`), as one WARN
line instead of a missing-file one. Nothing resolves on Omega. 2048's three shared labels (`crowd`, `gentrak`) exist on
Omega as `crowd.bnk`, `crowd_NGP.bnk` and `generaltrack.bnk`.
