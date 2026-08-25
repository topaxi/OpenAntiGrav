# SteamOS's own glibc version

Unestablished. `just appimage-portable`'s floor is `GLIBC_2.34` (`objdump -T` and `readelf -V` agree, and the determinism test passes inside that container, so the two builds are interchangeable for the simulation). If `GLIBC_... not found` ever appears on a Deck, the version it names is the missing datum. [`packaging.md`](../docs/tools/packaging.md).

## Open

- SteamOS's own glibc version is unestablished

## Next Steps

- If `GLIBC_... not found` ever appears on a Steam Deck, record the version it names
