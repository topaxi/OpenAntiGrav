# A player cannot set up 2048, Omega or HD without guessing

`docs/overview/installing.md` was written by following it literally on Linux
(2026-10-07). Pulse (PSP and PS2) and Pure work as dropped in. The rest of
the title list does not, and the program's own messages do not say why.
Ranked by how badly each stops a player; the detail is the lane's
`friction.md`, summarised here.

## Open

1. **2048 and Omega ship as `.pkg` and the program reads only an unpacked,
   decrypted folder.** A `.pkg` in `data/images/` gives `no disc image found`
   with a `Searched:` list that names neither `.pkg` nor `data/extracted/`.
   The unpack chains (pkg2zip, psvpfsparser with a license key, LibOrbisPkg)
   are manual and were not re-run. No player-facing command exists.
2. **A no-argument boot and `--dry-run` never find those folders.**
   `oag_source::source::resolve` scans images only; `candidates()` (the
   `--launcher` chooser) scans extracts too. The chooser shows them as
   `unknown` with the folder name as the serial.
3. **HD's encrypted `.iso` fails with a message about Pulse's WADs**, and with
   both `hdfury-ps3-eu.iso` and `-dec.iso` present the encrypted one wins
   (`IMAGE_NAMES` lists it, not the `-dec` one). `scripts/ps3iso.py` needs the
   Python package `cryptography`, declares nothing, and prints a traceback on
   too few arguments.
4. **`$OAG_IMAGE` rejects an unpacked 2048 or Omega folder**, though
   `source.rs`'s module doc says it takes the same things as the command line
   and the not-found error suggests it. `[source] image` accepts them.
5. **The not-found hint is wrong for a plain install** ("beside the
   executable" only applies to an AppImage); the real default is `data/images/`
   relative to the current directory.
6. **The intro is a black `INTRO FRAME n / N (NO PICTURE)` without ffmpeg**, so
   a first screenshot looks broken. `oag-game --help` still opens with "Run
   Wipeout Pulse from a disc image".
7. **Stale lines elsewhere:** `data/README.md` calls Pure and HD "read-only
   format targets, not playable" and says `Error::WrongTitle` for Pure;
   `docs/tools/packaging.md` says "five normalised names" (there are six).
8. **Not walked:** Windows and macOS, a clean machine with no system
   libraries, the 2048 and Omega unpack chains, Pulse DLC mounting (the dry
   run prints no pack line), and audio.

## Next Steps

1. Make `resolve` fall back to `candidates()` when no image is found, so
   `oag-game` and `--dry-run` open a lone extract.
2. Say "encrypted PS3 image" in the open error (the region table in sector 0
   is a cheap test, see `docs/formats/ps3-disc.md`), and try the decrypted
   image first.
3. Teach the not-found error about `.pkg` files and `data/extracted/`.
4. Let `$OAG_IMAGE` name a package folder; add a `uv` header or usage text to
   `scripts/ps3iso.py`.
5. Decide whether a player-facing unpack command for 2048 and Omega is wanted
   at all, given that it needs a license key from the player's own copy.
6. Fix the stale lines in item 7.
