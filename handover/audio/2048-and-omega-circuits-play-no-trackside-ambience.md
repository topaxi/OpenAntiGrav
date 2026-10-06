# 2048's and Omega's circuits play no trackside ambience

2026-10-06, found reading a `just play --race` log. `oag_sound::sfx::TrackEmitters::load`
resolves **0 of 41** emitters on 2048's Altima and **0 of 32** on Omega's Tech De Ra:
every authored `sound` node is silent on both titles. Three separate causes, each
measured with a throwaway probe over `oag_2048::open` / `oag_omega::open`.

## Open

1. **The circuit bank is not beside the track.** `circuit_bank_entry` joins the
   `trackstartup.xml` `<LoadSoundBank>` filename to the track's directory, which is
   Pulse's and HD's layout. On both titles the file sits under `Data\audio\sound\`
   instead: `env_altima.bnk` and `env12_techdera.bnk` both read there, and
   `Data\art\published\environments\altima\env_altima.bnk` does not exist. Omega
   keeps HD-era circuits' banks there too (`env9_talonsjunction.bnk`). HD must keep
   resolving beside the track. Confidence 60: an archive census, not the
   executable's loader.
2. **2048's banks are `SBlk` version 5 and partly unreadable here.**
   `crowd_NGP.bnk` (label `crowd`), `generaltrack.bnk` (`gentrak`),
   `env0_zone.bnk` (`env_zon`) and `env1_vinetak.bnk` (`env_vin`) parse, but
   `sound_names()` is empty on all of them: v5 names are FNV-1 hashes, see
   [2048-xfx.md](../../docs/formats/2048-xfx.md). The nodes spell cue names
   (`neoon_small`, `startline`), so a lookup has to hash the name. And
   `env_altima.bnk`, `env_arena.bnk` and `env_sol.bnk` (1,520,336 / 1,772,192
   bytes) do not parse at all: "the section table does not span the blob
   exactly".
3. **Omega's `.bnk` are Wwise (`BKHD`).** All thirteen probed, including
   `crowd.bnk`, `crowd_NGP.bnk`, `generaltrack.bnk` and every `env*.bnk`, fail
   `sblk::Bank::parse` with version `0x44484B42`. `oag_formats::wwise` reads them
   (`docs/formats/wwise.md`), but a node's `bank~cue` pair is not yet mapped to a
   Wwise event or sound.
4. **Shared banks.** The nodes name `crowd` and, on Omega, `env_tec` and
   `techder` besides the circuit's own. `oag_title::SoundBanks::track_general` is
   one `Option` and `None` on both titles; this needs a list (crowd plus
   general track), as `Title` data with an `Origin`. Which file carries
   `env_tec`/`techder` on Omega is unread.
5. **The root cause logs at `debug`.** `track audio: <bank> not read: ... has no
   entry` is not an absence phrase in `oag_raceplay::loader_log::ABSENCE`, so
   only its symptom (`play nothing`) reached `warn`.

Cross-title check: done here, both titles checked; 2048 and Omega differ in
format (SBlk v5 against Wwise) and agree on the directory.

## Next Steps

1. Items 1 and 4 for 2048 alone, with v5 hashed lookup (item 2's first half):
   half a day. Add an `#[ignore]`d ground truth asserting how many of Altima's 41
   resolve.
2. The `env_altima.bnk` framing failure: an hour to a day, depending on whether
   it is a new section or a size field.
3. Omega via `oag_formats::wwise`: its own lane, a day or more.

## Log state, 2026-10-06 (logwarn-2048)

The 2048 default log still carries `crowd~crowd` plus six `env_alt~...` lines at WARN and
Omega carries the `crowd`/`env_tec`/`techder` ones: they are this thread's open items 1-4
and are the honest absence, not noise. Nothing was demoted. Item 1 alone moves no cue
(the 2048 banks need item 2's hashed lookup first), so the order above stands.
