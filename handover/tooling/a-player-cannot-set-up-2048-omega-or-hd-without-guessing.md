# A player cannot set up 2048, Omega or HD without guessing

`docs/overview/installing.md` was written by following it literally on Linux
(2026-10-07). Pulse (PSP and PS2) and Pure work as dropped in. The rest of
the title list does not, and the program's own messages do not say why.
Ranked by how badly each stops a player; the full list is the
lane's scratch file `data/scratch/readme-setup/friction.md` in the main
checkout (gitignored), summarised here.

## Open

Landed 2026-10-07 (setup-friction): a no-argument boot, `--dry-run` and
`--screenshot` find unpacked 2048 and Omega folders; the not-found error names
every place and form including `.pkg`; an encrypted HD `.iso` is reported as
encrypted and the `-dec.iso` is tried first; `$OAG_IMAGE` takes unpacked
folders; `scripts/ps3iso.py` has usage and a clear missing-`cryptography`
message; the "beside the executable" hint shows only under an AppImage.

1. **No player-facing unpack command exists for 2048 and Omega `.pkg`s.** The
   unpack chains (pkg2zip, psvpfsparser with a license key, LibOrbisPkg) are
   manual and were not re-run. The error now says a `.pkg` must be unpacked and
   points at installing.md.
2. **The intro is a black `INTRO FRAME n / N (NO PICTURE)` without ffmpeg**, so
   a first screenshot looks broken. `oag-game --help` still opens with "Run
   Wipeout Pulse from a disc image".
3. **2048's 1.04 patch is not mounted** (`crates/2048/src/lib.rs`), so a
   player who unpacks it gains nothing and nothing says so.
4. **Stale lines elsewhere:** `data/README.md` calls Pure and HD "read-only
   format targets, not playable" and says `Error::WrongTitle` for Pure;
   `docs/tools/packaging.md` says "five normalised names" (there are six, now
   seven with the `-dec` HD name).
5. **Not walked:** Windows and macOS, a clean machine with no system
   libraries, the 2048 and Omega unpack chains, Pulse DLC mounting (the dry
   run prints no pack line), and audio. Also: no-arg default is USA-first for
   Pulse while `just play` is EU; the cache directory depends on whether a
   `data/` folder exists in the current directory.

## Next Steps

1. Decide whether a player-facing unpack command for 2048 and Omega is wanted
   at all, given that it needs a license key from the player's own copy.
2. Say in `--help` that the other four titles exist, and say in the loader
   report why the intro has no picture without ffmpeg.
3. Fix the stale lines in item 4.
