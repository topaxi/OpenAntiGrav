# 2048's circuit sound banks and trackside ambience

Wipeout 2048 (Vita) keeps its sound banks as `SBlk` **version 5**
([2048-xfx.md](2048-xfx.md) has the container and the FNV-1 name rule). This
page is what a circuit's authored `sound` nodes need to find a cue in one, and
what the disc ships for them. Evidence is a census of
`data/extracted/vita/PCSF00007/base/PSP2/data.psarc` and the ground truths
named below; none of it is read off the executable's own loader, so the
location rule is **confidence 60** (an archive census, not the loader).

## Where a circuit's banks are

| Claim | Evidence |
| --- | --- |
| A circuit's own bank is `Data\audio\sound\<LoadSoundBank filename>`, **not** beside the track. | `env_altima.bnk` reads there; `data/art/published/environments/altima/env_altima.bnk` does not exist. The filename is `trackstartup.xml`'s `<LoadSoundBank>`, as on Pulse and HD. |
| Two shared banks hold the other labels a node spells: `crowd_NGP.bnk` (label `crowd`) and `generaltrack.bnk` (`gentrak`). | Altima's `crowd~crowd` nodes resolve in the first. |
| HD keeps resolving beside the track. | `track_audio_ground_truth::hd_still_reads_its_circuit_bank_beside_the_track`: Talons Junction, 66 of 66. |

This is `oag_title::TrackBanks` (`shared`, `circuit_directory`, `origin`) on
`oag_title::SoundBanks::track`, which replaced the one-`Option`
`track_general`; Pulse and Pure carry `generaltrack.bnk` as a one-element list,
HD an empty one, Omega an empty one unswept.

## The name rule, proven on every bank that parses

A node spells its cue (`~neoon_small`, `~startline`) and a v5 bank keys its
table by `name_hash`. The name block carries 16-byte records from `+0x10`
(`{u32 name offset, u32 hash, u16 cue, u16 next, u32 index}`) and a string pool
at the offset `+0x0c` gives, so a bank **spells** most of its names beside the
hashes: `Bank::hashed_names`. Across **33 hashed banks and 1,603 spelled
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

Three other v5 banks still do not parse, for a different reason and unread
here: `Speech_NGP.bnk`, `speech_fe_NGP.bnk` and `speech_zone_NGP.bnk` declare a
waveform length (915, 1,566 and 1,242,097 bytes) that is not a whole number of
ADPCM blocks.

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
nodes name `env_tow`; resolving them there would be inventing a join. All stay
at WARN with the cause.

Heard, not only resolved: a 40 s Altima autopilot lap rendered to WAV through
the real mixer, once with its emitters and once with them dropped
(`altima-ambience.wav`, scratch). The 861 ticks with no emitter in range or in
the ten before it are byte-identical in both, and 1,506 of the 1,539 ticks near
one differ, up to the mix's full peak.

## Checked against Omega

**Checked, differs (format), agrees (directory).** Omega's `.bnk` are Wwise
(`BKHD`), not `SBlk`: none of `sblk::Bank::parse`, the hashed lookup or the
stale-size reading applies, and a node's `bank~cue` pair is not yet mapped to a
Wwise event. Omega keeps its circuits' banks under `Data\audio\sound\` too, so
`TrackBanks::circuit_directory` will take the same value once the Wwise lane
lands; it is left unset in `oag_omega::race::SOUND_BANKS` rather than switched on
an unmeasured loader. 2048's three shared labels (`crowd`, `gentrak`) exist on
Omega as `crowd.bnk`, `crowd_NGP.bnk` and `generaltrack.bnk`.
