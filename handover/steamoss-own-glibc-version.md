# SteamOS's own glibc version

Established from Valve's own package mirror, not yet confirmed on physical
hardware. `just appimage-portable`'s floor is `GLIBC_2.34` (`objdump -T` and
`readelf -V` agree, and the determinism test passes inside that container, so
the two builds are interchangeable for the simulation). `steamdeck-packages.steamos.cloud`'s
per-branch repos give glibc directly: 2.37 on SteamOS 3.5, 2.40 on 3.7, 2.41 on
3.8 (current stable, 2026-08), 2.43 on 3.9 (preview) - every branch back to 3.5
clears the floor with rising margin, superseding the 2.33 figure from a 2022
forum post. [`packaging.md`](../docs/tools/packaging.md).

## Open

- Whether a Deck that has gone a long time without updating could still sit on
  a pre-3.5 glibc below the floor is unconfirmed - the mirror shows what Decks
  update *to*, not what an actual installed system reports.

## Next Steps

- If `GLIBC_... not found` ever appears on a Steam Deck, record the version it
  names - it would mean an unusually stale install, worth knowing about either
  way.
