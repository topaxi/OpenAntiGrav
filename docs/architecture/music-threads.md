# Where music work runs

**The frame thread never reads, lists or decodes music.** Every disc read, soundtrack
listing and decode in `oag-sound` runs on a worker (`oag_thread::Task`: a thread natively,
a Web Worker in the browser), and the frame thread only polls. A result that lands late
starts the music a little late; no frame waits for it. The design stays whole-buffer: a
worker decodes a whole track into one `oag_audio::Sound`, as before, because seek indexes
that buffer (progressive decode was turned down for that reason).

## The paths

| Path | Worker | What the frame thread does |
| --- | --- | --- |
| Music survey (which releases exist; a PSP or PS2 boot lists its soundtrack and opens every image on the search path to find the other one) | `music-survey` (`MusicDiscs::survey_in_background`) | nothing; the discs read as "not known yet" until it lands |
| Front-end music and HD's mix tables (`Audio::start_music`) | `menu-music` (`menu_music::MenuJob`) | starts the loop on the tick it lands; a race already started keeps it for after the race |
| A race's first track at launch (`Session::launch_race`) | `race-music` (`MusicFetchWorker`) | `finish_race_music` applies it, or a later tick does |
| `start_race_music` (`--race`, a resume from the menus) | `race-music`, unless the track is already decoded | a decoded track (the usual resume) starts at once from `race_cache` |
| Next track (prefetch) and a track boundary | `race-music` | the prefetch is handed over at the boundary, never joined |
| `MUSIC SOURCE` switch | `menu-source` / `race-source` | applies a held sound at once, else polls |
| A saved place (`Audio::restore_race_music`) | the race's `race-music` fetch | checks it on the spot only if the listing is already known; otherwise the fetch checks it and logs the verdict |

Which track a race fetch is for is a `RacePick`: a known index, "the track after this
one", a run's first race (one past the menu's track on a PS2 boot, whose menu plays one of
the sixteen, decided from the booted platform rather than from whether the menu's loop has
landed), and optionally a saved place still to check. "After" and a saved place both need
the booted listing, so the worker resolves them; the frame thread never needs a listing.
Every worker waits for a background survey before it reads (`MusicDiscs::wait_surveyed`).

**`--dump-audio` waits.** A dump is compared sample for sample at a tick count, so with a
dump every music start blocks until its worker lands (`Audio::settle_music`). Waiting does
no music work on the calling thread; it only blocks it. The browser never waits: its page
thread polls.

## Granular reads

A listing needs each track's length, which sits in the entry's head: a RIFF header
(ATRAC3+, ATRAC9) in the first hundred bytes, an MP3's `Info` frame in its first frame.
`oag_music::measure` reads `MEDIA_HEADER_PEEK` (64 KiB) through `Archives::peek_name`,
which on a PSARC inflates only the leading block (`psarc::Directory::read_entry_head`) and
on a WAD reads a stored or LZSS prefix (`Archive::peek`). A head that does not say falls
back to the whole entry. HD's listing used to inflate all fifteen tracks (about 75 MiB).
Omega's plan reads a loose stem's header for its channel count instead of the whole stem.
Only the track that will play is decoded, one per fetch. One at a time is the rule, not a
guarantee: a prefetch that a `MUSIC SOURCE` switch drops keeps decoding on its detached
thread beside the switch's own fetch, and its result is thrown away.

`hd_music_ground_truth.rs` and `vita_2048_music_ground_truth.rs` assert, on every track,
that the head-read length equals the whole entry's, to the bit.

## The probe

`oag_sound::probe` counts every listing, track decode, front-end decode, mix-table read
and survey on the thread that ran it. `oag-game` marks its frame thread, where each is
logged at warn with its duration (`audio: <what> on the frame thread took N ms`); on a
worker it is a debug line naming the thread. The 40 ms frame warning cannot stand in for
it: a frame the session marks as a load stall is never logged, and both HD's menu-music
start and the launch frame were such frames.

`crates/game/tests/music_resume_ground_truth.rs` (`just test-data`) asserts the probe does
not move on the calling thread across the survey, the menu music, a race start, its first
tick, a resume from the menus, a saved place, a track change and (on both Pulse discs) a
`MUSIC SOURCE` switch, on all six titles. Putting one synchronous listing read back into
`start_race_music` fails it on HD and on Pulse PS2.

## Measured on the Steam Deck (2026-10-19)

Steam Deck LCD, gamescope 1280x800 at 90 Hz, vsync, windowed `:1` with Steam's
environment over ssh, `--frame-log`, release build with `-C target-cpu=znver2` in the
AppImage container, AC power, Deck load average 0.8 to 1.3. "Before" is main at
`71805dd91` plus the probe alone. Times are what the probe logged; "frame thread" is work
that blocked a frame (or the window's start, for `--race`).

| Case | Before: on the frame thread | After: on the frame thread | After: on the worker | Frames over 40 ms (worst), before / after |
| --- | --- | --- | --- | --- |
| HD front-end music start | 231 ms (13 mix tables + 218 MP3 decode) | 0 | 14 + 216 ms | 0 / 0 (a stalled frame hid it) |
| HD menu-launched race with a saved place (`--measure-race-load 2`) | 305 ms listing; worst main-thread frame 328.3 ms, `launch_race` span 325.1 ms | 0; worst frame 21.1 to 22.2 ms, span 16.8 to 17.2 ms | listing 24 ms, track 598 to 603 ms | 0 / 0 (a stalled frame hid it) |
| HD `--race` | 224 to 308 ms listing + 293 to 416 ms track, before the first frame | 0 | listing 23 ms, track 425 to 431 ms | 1 (41.7 to 45.7) per run over 4 runs / 1 or 2 (43.0 to 49.8) over 4 runs |
| HD track change (saved place at 218 of 227.5 s, real audio) | 0 at the change (the prefetch was in) | 0 | next track 437 to 441 ms | 0 or 1 (42.6) / 0 or 1 (41.9) |
| Pulse PSP front-end music start | 94 ms | 0 | 109 ms | 1 (96.7) / 0 and 0 |
| Pulse PSP `--race` | 670 to 687 ms track decode, before the first frame | 0 | 702 to 706 ms | 0 / 0 |
| Pulse PSP survey (boot, or a launcher pick's load frame) | 359 to 366 ms on `main` | 0 | 359 to 364 ms; HD's 19 ms | not separable from the load |

The 40 ms frame warning cannot show most of this: the blocked frames were either before
the first window frame (`--race`) or marked as load stalls. What it does count, the first
race frames, did not move: over nine `--race --autopilot` runs of 18 to 25 s each, base
logged one or two frames of 41.7 to 45.7 ms per run and the fix one or two of 41.4 to
50.0 ms (uploads and first passes, not music), steady 90 fps on both.

**Measured and dropped:** starting the next-track prefetch five seconds into a track
rather than on its first tick, in case two decodes beside the first race frames cost a
second slow frame. With real audio (so the music clock moves) base, fix and the variant
logged 1/2/1, 1/1/1 and 1/1/1 slow frames over three runs each: noise, so the prefetch
still starts at once.

Commands (Deck side; `run.sh` sets private `XDG_*` directories and Steam's environment):
`./run.sh <label> 25 hdfury-ps3-eu-dec.iso --race --autopilot`,
`./run.sh <label> 75 hdfury-ps3-eu-dec.iso --measure-race-load 2` with a `music.toml`
place for `Wipeout HD`, `./run.sh <label> 45 <image>` for the front end, all with
`RUST_LOG=info,oag_sound=debug`; `--no-audio` except the track change, where the music
clock has to move.

## Not moved

- **Opening the booted source to identify it** for a launcher pick now runs on the
  survey worker too, but the launcher's load itself (archives, front end) still opens the
  same source on the frame thread inside one stalled frame. That is the load's, not the
  music's.
- **A menu-launched race whose track is already decoded** still fetches it again on the
  `race-music` worker (`Session::launch_race` always spawns one). Off the frame thread, so
  only a wasted decode.
- **Omega's `load_entry`** re-reads the bank and re-plans for each track, on the worker.
