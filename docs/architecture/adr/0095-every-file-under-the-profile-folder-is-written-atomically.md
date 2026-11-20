# ADR-0095: Every file under the profile folder is written atomically

## Status

Accepted (2026-11-20, maintainer decision from the ADR-0093 review). Lands with its implementation.

## Context

The profile folder (`<config dir>/oag/`, with the replays under `<data dir>/oag/` and the rejoin
record under `<state dir>/oag/`) holds what a player cannot get back: medals, settings, the
player key. Four files were already written through a temp file and a rename, each with its own
copy of the code (`rejoin_record.toml`, `music.toml`, `identity.toml`, and `.partial` for ghosts
and replays). `records.toml` and `settings.toml` went through `profile::write`, which on native
was `std::fs::write`: open, truncate, write. A process killed (or a machine that lost power)
between the truncate and the last byte left a short file.

That is worst for `records.toml`. A file that does not parse is moved aside as
`records.toml.invalid` and the player starts with no records (`docs/architecture/persistence.md`,
"What a bad file does"). One crash during a save cost every medal.

## Decision

1. **`profile::write` is atomic, and it is the only way a file under the profile folder is
   written.** On native it writes the content to `<file name>.tmp` beside the file (a fresh file:
   a stray from an earlier crash is removed first), flushes it with `sync_all`, then renames it over
   the file. The file holds the old content or the new, never a mix. On any error the temp file is
   removed and the old file is as it was.
2. **One implementation, three entry points** (`crates/game/src/profile/native.rs`): `write`
   (text, also on the web), `write_bytes` (a file that is not text: ghosts, replays, pilot files,
   the rejoin record) and `write_private` (owner-only mode `0600` from creation, for
   `identity.toml`). The hand-rolled temp-and-rename code in `rejoin_record`, `music_state`,
   `identity`, `ghosts` and `replays` is gone, and `oag_raceplay::pilots::write_pilot` with it
   (the pilot editor writes through `write_bytes`).
3. **The temp name is fixed**, `<file name>.tmp`, and a process-wide lock covers the temp write and
   the rename, so two threads never share one. A temp file left by a crash is ignored by every
   loader (each reads the exact file name, and the replay and ghost listings match on the
   extension) and is replaced by the next write of that file. Nothing sweeps the folder.
4. **Flush the file, not the directory.** `sync_all` on the temp file before the rename: without
   it a power cut can leave the rename recorded and the data not, which is an empty file in the
   place of the old one. The directory is not flushed: losing the rename only keeps the old file,
   which is consistent. Measured on the maintainer's workstation, a 4 KB write, flush and rename
   takes about 1 ms (median; 2.3 ms at the 99th percentile), against 0.1 ms unflushed.
5. **The web build writes one `setItem`.** There is no file to truncate: the Storage standard
   either stores the value or throws (quota) and leaves the old value, so a single `setItem` is
   already all or nothing and a temp key would only double the quota use. `write_bytes` and
   `write_private` return `Unsupported` on the web, as `std::fs` did there: binary files are not
   kept, and a secret is not put where any script of the origin reads it.
6. **Windows.** `std::fs::rename` replaces an existing file there (`MoveFileExW`, and on Windows
   10 1607 and later the same behaviour as Unix where the file system supports it). It fails
   with access denied when another process holds the destination open without sharing deletes
   (a virus scanner, an indexer). The helper retries five times over about 300 ms, then keeps the
   old file, removes the temp and returns the error, which the callers already log and carry on
   from. Written from the toolchain's documentation; not run on Windows.

## Consequences

- A crash during a save leaves the previous `records.toml` and `settings.toml` readable; the
  `.invalid` path is reached only by a file a person edited wrong.
- Each save costs about a millisecond more for the flush. `settings::save` runs on the render
  thread on a menu change and the race music place every 10 seconds of racing; on a slow disk
  that is a hitch where there was none. Records are already written from a background thread.
- Two processes writing one file at once share the temp name and can corrupt it. Nothing locked
  that before either; the last writer won, now the last writer can also lose to an interleave.
  Two game instances on one profile are not a supported case.
- A file whose directory is on a file system without rename-over (some network shares) fails to
  save with an error where `std::fs::write` worked.
- Left as they were, because they are not the player's profile: the log files, the caches
  (`custom_tracks`, `prefetch`, movie transcodes) which are regenerated, a photo (a new file per
  shot, nothing to lose), the PS3 disc key cache in `oag-disc` and the mod campaign editor's
  output in `oag-mod`, which cannot reach `oag-game`.
