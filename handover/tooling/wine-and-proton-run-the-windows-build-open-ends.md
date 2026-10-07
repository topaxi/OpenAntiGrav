# The Windows build runs under Wine and Proton; what is left unchecked

`just wine-run` / `just wine-check` (`scripts/wine-run.sh`, `docs/tools/wine.md`)
cross-build `x86_64-pc-windows-gnu` and run it under wine. Measured 2026-10-07:
every determinism hash equals the committed reference, and Pulse and HD
screenshots (front end and race) and a race WAV are byte-identical to the native
build; UMU-Proton 9.0-3.2 matches too. Nothing failed.

## Open

- A `determinism-wine` job is proposed in `ci.yml` and has never run on GitHub
  (local only, by decision).
- Not measured: sustained windowed frame rate under wine against native, a real
  audio device through wine, 2048 and Omega, movies (`create_factory_media failed`
  is logged at start), gamepad input under Proton.
- The MSVC build `release.yml` ships was not run under wine; the MinGW build was.
- Under Proton an image path must be absolute (the container's working directory
  differs); a relative one exits silently.

## Next Steps

1. Run `just wine-run` windowed on a real GPU display for a frame-time number.
2. Smoke 2048 and Omega unpacked folders under wine.
3. Once `release.yml` has produced a Windows zip, run its `oag-game.exe` under wine.
