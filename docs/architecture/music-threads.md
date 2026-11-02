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
| A race's first track at launch (`Session::launch_race`) | `race-music` (`MusicFetchWorker`), none when the track is already in `race_cache` (`Audio::race_music_fetch`) | `finish_race_music` applies it, or a later tick does; a cached track is applied by `start_race_music` at the hand-off with no fetch |
| `start_race_music` (`--race`, a resume from the menus) | `race-music`, unless the track is already decoded | a decoded track (the usual resume) starts at once from `race_cache` |
| Next track (prefetch) and a track boundary | `race-music` | the prefetch is handed over at the boundary, never joined |
| The music pool: other titles' discs (`oag_sound::pool`) | `music-discs`, apart from the survey | nothing; the MUSIC page's rows fill in as it lands |
| A configured music folder (`oag_sound::folders`) | `music-folder`, one per folder, detached | nothing; a status (`SCANNING`, a count, `SLOW OR NO ANSWER`, the error) the MUSIC page polls |
| `MUSIC SOURCE` switch | `menu-source` / `race-source` | applies a held sound at once, else polls |
| A saved place (`Audio::restore_race_music`) | the race's `race-music` fetch | checks it on the spot only if the listing is already known; otherwise the fetch checks it and logs the verdict |

Which track a race fetch is for is a `RacePick`: a known index, "the track after this
one", a run's first race (one past the menu's track on a PS2 boot, whose menu plays one of
the sixteen, decided from the booted platform rather than from whether the menu's loop has
landed), and optionally a saved place still to check. "After" and a saved place both need
the booted listing, so the worker resolves them; the frame thread never needs a listing.
Every worker waits for a background survey before it reads (`MusicDiscs::wait_surveyed`).

**A race's song is parked, not started, when it lands.** A title whose original starts the
song at the green light (`oag_title::Music::race_song_start`, every title) has the hand-off arm
`Audio::hold_race_song` first; a track that lands while it is armed waits decoded in `Audio` and
`Audio::race_tick` starts it on the first step the race is no longer gated. Starting is a voice on
a decoded `Arc<Sound>`, so no worker runs at the green light. See
[the now-playing page](../formats/now-playing.md#when-the-song-starts).

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
## Landed 2026-11-02

- **A menu-launched race on a track already in `race_cache`** spawns no `MusicFetchWorker`
  and decodes nothing: `Audio::race_music_fetch` returns `None`. Ground truth
  (`music_resume_ground_truth.rs`, `*_second_launch_decodes_nothing`, Pulse PSP and HD):
  the first launch is a fetch, the second is none and its hand-off does no music work on
  the calling thread. The Deck count of `read and decode on race-music` lines is not
  re-measured (the Deck was not offered).
- **Omega's music bank is read and parsed once per run.** `Listings` keeps an
  `oag_music::omega::Prepared` (the playlist and the bank's bytes) beside the listings;
  `load_entry` takes it from there, and `mix_with` reads an embedded stem out of the kept
  bytes. Ground truth: `omega_reads_its_music_bank_once_across_tracks` (two tracks,
  `MusicDiscs::bank_reads() == 1`). 2048 and HD carry `state_tracks: None` and list their
  tracks by name (`declared_names`, a directory read, no bank): **checked, differs**.

## The music pool (landed 2026-11-02)

OPTIONS > AUDIO > MUSIC lists every soundtrack source this machine can open, each on or off:
the booted disc (on by default), each other title's disc image found on the search path (one
row per image, off) and each folder under `[audio] music_folders` in `settings.toml` (off
unless its `enabled` is set). Switches persist in `[audio] music_switches`, keyed by a stable
source id (`disc:<title slug>:<platform>:<file stem>`, `folder:<path>`); an id absent from the table
takes the default. The page is the MODS page's shape (a SOURCE row that walks the sources and an
ENABLED row for the one it is on), so pad, mouse and touch work as on every other choice row.
Adding a folder from the menu needs a text input widget and is open.

**What a race plays.** The playlist is one numbered list: the booted soundtrack in its own
order first (when the booted disc is on), then every other enabled source's songs, discs
before folders, discs in id order, a folder's files in path order. A song is `Loaded::from =
None`, the same as the PSP front end's loop, so `MUSIC SOURCE` never moves it. With only the
booted disc on the list is the booted soundtrack, `playlist_len` equals its length, and every
`RacePick` resolves as before (`tests::pool::with_only_the_booted_disc_on_the_playlist_is_the_booted_soundtrack`).
The booted title's other pressing is not a row: choosing between the two encodes of one
soundtrack stays the `MUSIC SOURCE` row's job and keeps its seek-across-encodes behaviour. A
different title found on two pressings (HD's encrypted and decrypted image, Pure EU and USA) is
two rows but contributes its songs once, from the first enabled image in id order, so no song
plays from two encodes.

**Menus.** The front end's music is unchanged: it is the disc's authored loop (the PSP's
`frontend1.at3`, HD's mix tables, the PS2's track 0), and the pool does not replace authored
data. The pool feeds the race playlist only.

**Never on the frame thread.** The disc scan and each folder scan are their own workers, so a
hung mount cannot delay the booted disc's survey or its music (`wait_surveyed` gates only the
survey). A scan with no answer after 20 s reads `SLOW OR NO ANSWER`, and moves to its count if
it answers later; its thread is detached, never joined, so it cannot hold the menus or the
process's exit. A race whose booted disc is off waits up to 8 s on its own worker for a scan
still running, then plays what is listed. A song that will not read or decode (a mount that
went away) is logged and passed over for the next one, up to four, so the song playing and the
rest of the playlist are untouched. Folder files are MP3 only (what `oag_music` reads next to
the discs); other formats are not read yet.

**The folder cache.** `folders::Disk` keeps, per walked folder, each directory's modification
time with its songs and subfolders in `<audio cache dir>/music-folder-<hash>.tsv`; a later walk
stats a directory and reuses its entry when the time is unchanged, and lists only the
directories that moved (a change below a directory moves only that directory's time).

Measured on the host's two NFS shares (2026-11-02, loaded machine, a cold walk then a cached
one): Beatport 90.6 s then 43.2 s (3,328 songs), Music 258.0 s then 126.1 s (16,225 songs); a
share that does not exist fails in 20 ms; `FolderScan::start` returns in under 100 microseconds
in every case. The cache halves the walk and does not remove it: the cost is a `stat` per
directory over NFS. Because the walk is on a worker the menus and a race with the booted disc
on are not affected; a pool without the booted disc waits at most 8 s and then plays what has
been listed.
