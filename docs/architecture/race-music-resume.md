# Race music resumes where it left off

**Status: built (2026-10-12).** Maintainer request: "persist the music track playlist
state across restarts, so that in a race we resume where we left off, one second
resolution accuracy is enough". **Chosen, not measured**: the original's race music is
not recovered (`docs/formats/ps2-audio.md`), so what resumes is this engine's own
playlist. No confidence score applies.

## What the playlist is

One playlist per title, not per circuit or per mode. It is a plain cycle through the
**booted disc's** soundtrack in its own order (`next_race_index` is `(index + 1) % len`).
There is no shuffle, so the order is the index and nothing else is saved. The
`MUSIC SOURCE` setting (PSP or PS2 release of Pulse) only changes which recording plays;
the index is always in the booted order, so the saved state is independent of it.

Within a run the playlist already survived a trip to the menus
(`Audio::pause_race_music` keeps `race_index` and the position). This change carries the
same two numbers across a process.

## What is saved

`oag_sound::RaceMusicState`: track index, whole second (rounded to nearest), the track
count the soundtrack listed, and the track's length. Per title, in `music.toml` beside
`settings.toml` and `records.toml` (`$XDG_CONFIG_HOME/oag/music.toml`; the browser's
`localStorage` on the web, through `oag_game::profile`).

A file of its own rather than a section of `records.toml`: the periodic save would
rewrite every best lap each time. It declares `schemas/v1/music-state.tosd`
([ADR-0064](adr/0064-project-toml-files-declare-a-toml-schema.md),
[TOML Schema](../formats/toml-schema.md)); a file without the header or with another
kind is ignored by name in the log and rewritten at the next save. A write goes to a
temporary file and is renamed over the old one.

## When it is written

| When | Where |
| --- | --- |
| Leaving a race for the menus | `session/escape.rs`, after `pause_race_music` |
| A finished race left for the menus | `session/endrace.rs`, after `pause_race_music` |
| Any quit | `ApplicationHandler::exiting`: window close, a menu's quit, a quit from the race |
| While racing | every `SAVE_EVERY_SECONDS` = 10 s of ticks (**chosen, not measured**) |

A write happens only when the whole-second state moved since the last one (`Saver`), so
the menus do not rewrite the file. The bookkeeping (restore once, save when moved, the
10 s tick) is one struct, `oag_game::music_state::Run`, held by `Session` on every route.

The windowed `--race` route has no front-end shell, so its title key is the one it
opened (`Session::race_title`, via `Session::music_title`): it restores before
`start_race_music` and saves on leaving, on quit and every 10 s like a menu race.
**Headless captures (`--screenshot`, `--trace-out`, `--dry-run`) neither restore nor save,
on purpose**: a capture must not depend on a profile's `music.toml`, the same reason they
never call `records::save`. A windowed `--race` under a fresh `XDG_CONFIG_HOME` starts at
the normal place.

## When it is read

Once per run, in `Session::launch_race` before `reserve_race_music_index` (which only
fills an empty index, so a later restore would silently do nothing).
`Audio::restore_race_music` re-lists the booted soundtrack and refuses the saved state,
leaving the playlist at its normal start with a `warn` line
(`audio: not resuming the saved race music (...)`), when:

- the soundtrack cannot be listed;
- the track count differs (DLC added or removed, another release);
- the saved index is past the end;
- that track's length differs by more than 0.5 s (a swapped track; the decoded length
  and the listing's differ by decoder padding, 83 ms measured on Pulse PSP track 0);
- the saved second is not inside the track.

## Seeking, per title

`Sound` is whole decoded PCM, so `Mixer::seek` works wherever the track reaches the race
voice. Ground truth (`crates/game/tests/music_resume_ground_truth.rs`, `just test-data`)
plays 20 s on a null output, saves, opens a second `Audio`, resumes and asserts the same
track within 1 s, then repeats on track 1 at 7 s:

| Title / release | Result |
| --- | --- |
| Pulse PSP (EU) | resumes; track 1 asserted |
| Pulse PS2 (EU) | resumes; track 1 asserted |
| Pure PSP (EU) | resumes; track 1 asserted |
| HD / Fury (PS3) | resumes; track 1 asserted |
| 2048 (Vita) | resumes; track 1 asserted |
| Omega (PS4) | resumes; track 1 asserted |

Nothing falls back to the track start for lack of a seek.

The PS2 row used to say "track 1 not asserted". The cause was the test's, not the
engine's: it took the second track's length from `library::listing`, which is `None` for
the PS2 (its soundtrack is the `PS2MUSIC.WAD` directory, not an archived listing), while
`Audio::restore_race_music` already checks against `Audio::booted_track_seconds`, which
reads both. The test now uses that, and the PS2 resumes track 1 at 7 s like the rest.

`pulse_psp_run_restores_saves_and_quits` and the `pulse_ps2_` twin in the same file
drive `music_state::Run` through a launch, 25 s of racing ticks (asserting the 20 s
periodic save landed), a quit save and a second run that resumes from the file. A live
`Session` needs a window and a GPU, so `Session` is a one-line wrapper over `Run` and the
test covers everything but the wrapper's title choice.

## Omega / 2048 check

Same playlist code and the same ground truth on both: **ported** (each has its own
test above).
