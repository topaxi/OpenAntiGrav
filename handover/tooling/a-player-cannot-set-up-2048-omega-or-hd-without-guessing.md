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
Landed 2026-10-07 (setup-polish): auto-detect prefers Europe over USA (Pulse,
Pure, 2048 extracts; `DEFAULT_IMAGE` and every suggested image name too); a
missing `ffmpeg` logs one `warn` line and skips the conversion; `--help` names
all five titles; 2048's 1.04 patch was left unmounted on evidence, then mounted by
title-patches (see `docs/formats/2048-status.md`); stale `data/README.md` and `packaging.md` lines
fixed; the cache is `data/cache/` only in a checkout (a `justfile` and `data/`)
or where one already exists.

2. **Not walked:** Windows and macOS, a clean machine with no system
   libraries, the 2048 and Omega unpack chains, Pulse DLC mounting (the dry
   run prints no pack line), and audio.

## Next Steps

1. Decide whether a player-facing unpack command for 2048 and Omega is wanted
   at all, given that it needs a license key from the player's own copy.
2. ~~Mounting 2048's 1.04 patch~~ landed 2026-10-07 (title-patches): `ArchiveCandidates::patch`,
   see `docs/formats/patches.md`. Still open there: `data1` against `data2` and the
   DLC packs against the patch are chosen, not measured (both patch copies of
   `Definition.xml` are read by the original, so a collision test needs a path
   only one consumer reads).
