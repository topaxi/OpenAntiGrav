# Wipeout HD's circuit sound banks and trackside ambience

HD's `.bnk` are ordinary `SBlk` version 3 banks (the PSP container, name table
in the clear; [psp-audio.md](psp-audio.md)). This page is what a circuit's
authored `soundcone` nodes need to find a cue, and what the disc ships for them.
Evidence: `crates/game/tests/track_audio_hd_ground_truth.rs` (`#[ignore]`d, `just
test-data`) over all sixteen environment directories of `hdfury-ps3-eu-dec.iso`,
and [the executable's bank literals](../ghidra/functions/ps3-hdfury-eu/sound-bank-loads.md).

## Where the banks are

| Claim | Confidence | Evidence |
| --- | --- | --- |
| A circuit's own bank is the `<LoadSoundBank Filename>` of its `trackstartup.xml`, **beside the track**. | 85 | Every HD circuit's bank sits in its own directory; the file exists nowhere else. (HD's loader was not decompiled for the path format; 2048's was, and has no beside-the-track read.) |
| Five shared banks hold the other labels a node spells, all under `Data\Sound\`: `generaltrack` (`gentrak`), `voppler`, `crowd`, `speech_preracechatter` (`radios`) and `shiphd` (`shipHD`). | 85 for the first three (loaded at sound-manager construction), 65 for `crowd` (a per-mode table, mode condition unread) | The executable names each literal; see the Ghidra page. A node's label matches the bank's own label byte for byte. |
| The shared banks are what resolves most nodes. | measured | `the_shared_banks_are_what_resolve_the_voppler_and_crowd_nodes`: Ubermall resolves 27 of 132 without the list and 131 with it. |

Before this list HD's `TrackBanks::shared` was empty and the comment said
`Data\audio\sound\generaltrack.bnk` resolved to nothing. That path is 2048's;
HD's is `Data\Sound\`.

## How many of a circuit's emitters resolve

All 16 circuits author **cones only** (no omnidirectional `sound`). Pinned by
sixteen tests, one per circuit:

| Circuit | Cones | Resolve | Unresolved |
| --- | --- | --- | --- |
| `amphiseum` | 54 | 49 | `env_amp~neon`, `g_sbkEnv~neoon_small`, `g_sbkEnv~rndmnship_1`, `gentrak~startlineneon` x2 |
| `modesto_heights` | 48 | 5 | nine `env_mod~...` cues, 43 nodes |
| `talons_junction` | 66 | 66 | |
| `tech_de_ra` | 62 | 2 | seven `env_tec~...`, `techder~bluelight`, `techder~tunnelsigns`, `crowd~neoon_small`; 60 nodes |
| `zone_1`..`zone_4` | 0 | 0 | no emitter at all |
| `01_vineta_k` | 54 | 52 | `GENTRAK~STARTLINE` x2 |
| `02_track` | 42 | 42 | |
| `03_track` | 40 | 33 | 7 unassigned nodes |
| `04_chenghou_project` | 92 | 71 | `CHENGOU~CRAFT`, `env_met~rndmnship_1` x6, `env_moa~rndmnship_1`/`_2` x8, `generalt~startline` x5, `shipHD<TAB>~ship_idle` |
| `05_ubermall` | 132 | 131 | `generalt~startline` |
| `10_sebenco_climb` | 68 | 64 | `generalt~laser` x3, `generalt~windturbines` |
| `12_sol_2` | 67 | 67 | |
| `15_anulpha_pass` | 52 | 51 | 1 unassigned node |

Before the shared list the same sweep resolved, for instance, Ubermall 27 of
132, Chenghou 34 of 92, Sebenco 37 of 68, and Vineta K 51 of 54.

## Every remaining miss, classified

**Dangling reference** means: every bank the circuit loaded parsed, and the
reference names a label no loaded bank carries or a cue the labelled bank does
not spell. Proven from the banks' own name lists (dumped for every `.bnk` on the
disc), never by a failed lookup alone.

- **Typo, case or truncation of a real label.** `GENTRAK` (the bank is
  `gentrak`), `CHENGOU` (the bank is `env_che`), `generalt` (an eight-character
  cut of the file name; the label is the seven-character `gentrak`),
  `shipHD<TAB>` (a tab in the field), `g_sbkEnv` (a code symbol that leaked into
  the field). These are **not folded**: whether the original's label comparison
  is case-insensitive or prefix-matching is unmeasured on HD (Pulse's cue-name
  compare is a 16-byte `memcmp`, [track-sound-emitters.md](../ghidra/functions/psp-pulse-usa/track-sound-emitters.md),
  inherited evidence only), and matching them would be inventing a join.
- **A cue the right bank does not spell.** `env_amp~neon` (the bank has
  `~neon_big` and `~neoon_small`), `gentrak~startlineneon` (`generaltrack.bnk`
  holds exactly `~neon_small ~neon_big ~startline`), `crowd~neoon_small` (the
  bank holds `~crowd`), `generalt~laser` and `~windturbines` (no such cue in any
  bank), `shipHD~ship_idle` resolves, its tab-padded twin does not.
- **Another circuit's label, copied.** Chenghou's `env_met~rndmnship_1` and
  `env_moa~rndmnship_*` name Metropia's and Moa Therma's banks, which that
  circuit's manifest does not load.
- **`modesto_heights` and `tech_de_ra`: the manifest names the wrong bank.** Both
  `trackstartup.xml` files are byte-identical copies of Vineta K's (1,251 bytes,
  `<LoadSoundBank Filename="env1_vinetak.bnk"/>`, label `env_vin`), while the
  nodes spell `env_mod` and `env_tec`. The right bank exists:
  `tech_de_ra/env12_techdera.bnk` (label `env_tec`, 19 cues) sits beside the
  track, and the four Zone banks `env0_zone1..4.bnk` carry `env_mod`. **No file on
  the disc names either**: one copy of each manifest in `DATA00.PSARC`, no later
  archive holds another, and a search of every `.xml`/`.txt`/`.cfg`/`.lua`/`.ini`
  entry for `env12_techdera`, `env0_zone` and `env_tec` finds nothing. This is the
  disc's own authoring, played as the manifest says: 43 and 60 nodes silent.
  **Not switched on**: picking the bank whose label matches the nodes would be
  choosing, and what the original does about a label it cannot find is unread.
  Checked against Omega: its Tech De Ra manifest does name `env12_techdera.bnk`
  (its load report reads `Data\audio\sound\env12_techdera.bnk`), as does 2048's downloadable Tech De Ra
  (`DLC1\environments\tech_de_ra`), so the HD copy is a stale manifest the later
  releases corrected (**checked, differs**).
- **Unassigned nodes.** `03_track` has 7 nodes and `15_anulpha_pass` 1 whose bank
  and cue fields are both empty. The raw payload keeps the field layout of every
  other node (`+0x14` bank, `+0x1c` cue) with the exporter's default buffer text
  (`undNam`) after the first NUL, so these are nodes nothing was assigned to, not
  a decode variant. They are reported once as "author no bank and no cue name",
  at debug.

## What the log says now

Per-cue dangling lines moved from WARN to debug ("N node(s) dangle: ..."), and
each circuit with any gets **one** WARN: "N cue(s) the circuit names and its
banks do not author (M node(s)) play nothing". A miss only counts as dangling
when every bank the circuit tried to load parsed: Omega's Wwise banks do not, so
its lines stay WARN with their cause.

## Trackside voppler

`voppler.bnk` is `~voppler`, `vopplerfast`, `vopplermedium`, `vopplerslow` and
`~vopplerTest`: the speed-driven names suggest a fly-by effect, but the nodes
name `~voppler` itself, an authored trigger like every other. 101 of Ubermall's
131 resolved cones are it. A 30 s autopilot lap of Ubermall through the real
mixer, with it and with those nodes dropped (`audio-hd/05_ubermall-*.wav`,
scratch): peak ambient voices 7 against 5, **0 clipped, 0 starved** in both; the
same on Chenghou (30 nodes): 5 voices, 0 clipped, 0 starved, peak sample 22,083
in both. The pool (`MAX_VOICES` 32) is not the constraint.

## Checked against 2048

**Checked, differs where noted.** 2048's executable loads `generaltrack.bnk` at
boot and `crowd_NGP.bnk` and `speech_PreRaceChatter.bnk` among its race banks, the
same shape as HD, so 2048's shared list is `crowd_NGP` and `generaltrack`
(**ported**: `crowd`/`gentrak`; the `radios` label is not spelled by any 2048
node). 2048 has **no** `voppler.bnk` literal, so its `voppler~voppler` nodes
(Ubermall 101, Anulpha Pass 6) dangle; see [2048-audio.md](2048-audio.md).
HD's circuit bank is beside the track and 2048's is not: **checked, differs**.
