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

Landed 2026-10-07 (install-guide): `docs/overview/installing.md` is rewritten as
a step-by-step player's guide per OS (Windows, macOS, Linux, Steam Deck,
Android); the program also searches the folder its executable is in (a Windows
zip works from any folder); a windowed run with nothing found shows a
`NO DISC IMAGE FOUND` screen naming the per-user folder and the file endings,
on every desktop OS and not only Android; the README has a "How to play" entry.
Omega `.pkg` pairs, `.vpk` and the encrypted PS3 `.iso` are read in place, so
the old item 1 (an unpack command for them) no longer applies.

1. **A Vita `.pkg` is still not readable** (the PFS key is console-derived,
   `docs/formats/vita-package.md`). The guide says so and names `.vpk` as the
   way in. No real NoNpDrm `.vpk` has been tried, only stand-ins.
Landed 2026-10-07 (setup-polish): auto-detect prefers Europe over USA (Pulse,
Pure, 2048 extracts; `DEFAULT_IMAGE` and every suggested image name too); a
missing `ffmpeg` logs one `warn` line and skips the conversion; `--help` names
all five titles; 2048's 1.04 patch was left unmounted on evidence, then mounted by
title-patches (see `docs/formats/2048-status.md`); stale `data/README.md` and `packaging.md` lines
fixed; the cache is `data/cache/` only in a checkout (a `justfile` and `data/`)
or where one already exists.

2. **Not walked:** a real Windows PC (only Wine) and macOS (no download exists),
   a clean machine with no system libraries, the unpack chains, Pulse DLC
   mounting (the dry run prints no pack line), and audio.
3. **A renamed PS3 image loses its `.dkey`**: the key is found beside the image
   only under the image's own stem, so `Foo.iso` with `hdfury-ps3-eu.dkey`
   beside it fails (any name works in the keys folder). Trying every `.dkey` or
   `.key` in the image's folder is safe, each key being checked against the
   image; it is `oag-disc` code, `sibling_key_paths` in `ps3_crypt.rs`.

## Next Steps

1. Decide whether a player-facing unpack command for a Vita `.pkg` is wanted
   at all, given that it needs a licence key from the player's own copy.
2. Walk the guide on a real Windows PC and a Mac (once a Mac build exists).
3. ~~Mounting 2048's 1.04 patch~~ landed 2026-10-07 (title-patches): `ArchiveCandidates::patch`,
   see `docs/formats/patches.md`. Still open there: `data1` against `data2` and the
   DLC packs against the patch are chosen, not measured (both patch copies of
   `Definition.xml` are read by the original, so a collision test needs a path
   only one consumer reads).
