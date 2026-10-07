# Running under Wine / Proton

People run the Windows build under Wine or Proton even where a native build
exists (bugs, performance), so the Windows `oag-game.exe` is a target in its
own right, not only a test proxy. Measured 2026-10-07 on Arch, wine 11.19,
UMU-Proton 9.0-3.2, an AMD RADV GPU.

## The recipe

```sh
just wine-build                                          # cross-build only
just wine-run --dry-run data/images/pulse-psp-eu.chd     # cross-build, then run under wine
just wine-run --race --screenshot out.png --ticks 120 data/images/pulse-psp-eu.chd
just wine-check                                          # the determinism tests and reports, as .exe under wine
RELEASE=1 just wine-check                                # the same in release, which is what CI asserts
```

`scripts/wine-run.sh` builds for `x86_64-pc-windows-gnu` (it runs `rustup target
add` for you) and uses its own prefix, `data/wine/prefix` (`OAG_WINEPREFIX`
overrides it; never `~/.wine`). The linker is `x86_64-w64-mingw32-gcc` when
installed (`pacman -S mingw-w64-gcc`, `apt install gcc-mingw-w64-x86-64`).
Without it the script falls back to `zig cc` through
`scripts/zig-cc-windows-gnu.sh`, which drops the `-nodefaultlibs`, `-lmsvcrt`,
`-lgcc*` and `-l:libpthread.a` arguments `rustc` passes by name and adds
`-lunwind`. That links and runs, but zig supplies its own CRT rather than
MinGW's msvcrt, so prefer MinGW; the release workflow itself builds with MSVC.

A path on the command line is relative to the working directory under plain
Wine. Under Proton (a container) pass an **absolute** image path, or the exe
starts, finds nothing and exits silently.

## What was measured

| Check | Result |
| --- | --- |
| `--dry-run` Pulse PSP EU, HD (decrypted ISO) | boots, same report as native |
| front-end screenshot (Language Selection), Pulse and HD | byte-identical PNG to the native Linux build |
| `--race --screenshot` Pulse and HD | byte-identical PNG to native |
| `--dump-audio` 600 ticks of a Pulse race | WAV byte-identical to native, non-silent |
| determinism tests, core/physics/gameplay/ai, release | 2 + 5 + 3 + 4 pass: every hash equals the committed reference |
| determinism report examples (core, physics, ai) | print the committed values |
| Proton (UMU-Proton 9.0-3.2 through `umu-run`) | Pulse race screenshot byte-identical to native |

Identical bytes with identical GPU are expected: the renderer is deterministic
on one adapter, and nothing in the path is Windows-specific beyond the window.

**Backend.** wgpu's `PRIMARY` set on Windows is Vulkan and DX12. Under Wine,
Vulkan runs through `winevulkan` onto the host driver; `d3d12.dll` is loaded and
dropped again. Under Proton `vkd3d-proton` also answers for DX12. Both reach the
same host Vulkan driver.

**Speed.** Headless runs (debug build, one frame): Pulse race screenshot 3.2 s
native, 3.7 s wine; HD 8.3 s native, 8.8 s wine; the front-end screenshot is
0.5 s native against 1.7-2.6 s wine, which is Wine's start-up (the prefix, the
service processes). Not measured: sustained windowed frame rate, because the
only display available to a headless lane is Xvfb with software Vulkan.

## Known issues

- `create_factory_media failed: 0x80004002` is logged once at start (Media
  Foundation absent from the prefix); movies are not exercised by these runs.
- Wine's `d3d12` probe and, under Proton, `openxr` messages are noise.
- The first run creates the prefix and takes several seconds.
- `--dump-audio` was checked, a real device through wine's audio driver was not.

## CI

`ci.yml`'s `determinism-wine` job builds the MinGW target on Ubuntu and runs the
same four determinism tests and three reports under wine, so a Windows-only
divergence is also caught on the one OS that needs no Windows runner.
