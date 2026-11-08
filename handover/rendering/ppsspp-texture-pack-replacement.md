---
status: ready-to-wire
titles: [pulse, pure]
updated: 2026-11-08
summary: "Pack is an off-by-default mod, Pulse .vex and .mip draw from it; open: pixel-offset atlases (HUD, fonts), ZIM, Pure, 2026 zip frame"
categories: [tooling]
size: L (60)
difficulty: medium (60)
---

# Draw a PPSSPP texture pack (opt-in)

2026-11-08. Lane `psp-texture-pack-wire` wired it for Pulse: see
[ppsspp-texture-pack.md](../../docs/formats/ppsspp-texture-pack.md) ("Wired") and
[mods.md](../../docs/tools/mods.md) ("Texture packs"). `just fetch-texture-packs` fetches the
packs; `just push-data` carries them.

## Landed

- `just fetch-texture-packs` writes an `oag-mod.toml` (`oag-mod init-pack`) into each checkout; it wins
  over recognition, is git-excluded and is never rewritten (`docs/tools/mods.md`, "Texture packs").

- A pack is a mod that starts off: `oag-mod` recognises a `textures.ini` folder or zip (or a folder
  holding `textures.zip`); `[texture-pack]` and `default-enabled` in a written manifest; settings
  `[mods] enabled`; search path `data/texture-packs/`, `<data dir>/oag/texture-packs/` and a PPSSPP
  memstick's `PSP/TEXTURES/`; the MODS page lists it, its ADDS line reads TEXTURES; the author skill
  and `docs/tools/mods.md` updated.
- `oag_texture::replacer`: `Replacements` (key table over the mod's files, PPSSPP's lookup order,
  blank lines, lazy PNG under a 192 MiB cap) and the active-pack switch the race load sets.
- Pulse `.vex` textures (hulls, tracks) and whole-image `.mip` (flare, cloud, mist) draw from it;
  off is byte-identical to `main`. Report: `debug` for the key and replaced counts, `warn` for
  `texture pack not used: <name>: <why>`.

## Open

- **Atlases cut by pixel offset are not replaced**: the HUD sprite sheet (`oag_hud::sprite`, whose
  `Placed` rects and `alpha_at` work in the original's pixels), `.fnt` fonts, menu and boot art, the
  loading screen, `.pob` particle textures and ship skins. Each needs its UV or rect scaled by the
  picture's own factor (1x to 16x, sometimes per axis) and a frame judged at the size a player sees.
- **The 2026 `textures.zip` has not been drawn**: it sits inside `data/GitHub - WipEout Mods.zip`
  (553 MB); the frames used the 2020 reTokyo folder. The zip route is covered by a synthetic zip.
- **ZIM images** (a later pack's format, layout on the doc page) are not read, and **Pure** is not
  wired (a runtime-palette font must never be replaced by data hash alone).
- **One pack at a time**: with two on that name the disc, the first in search order is used and the
  other is reported. Race Remix keys the pack on the track source's serial.
- **Not on the menus' own textures**: only a race load installs the pack, so menu and boot art draw
  the disc's own even with a pack on.
- Unresolved misses (unchanged): Pulse ship hull texels rewritten after load (167 ship keys), Pure
  `main_menu/previews`, about 20 effect textures with no source on the disc.
- **Later version (maintainer):** an in-game download-and-enable button, instead of or beside
  `just fetch-texture-packs`.
- **Other assets in the maintainer's `data/GitHub - WipEout Mods.zip`** (the "WipEout Portable
  Collection"; replicating it is out of scope): a Pulse PS2 PCSX2 pack (PCSX2's own hash, a second
  importer), Pulse EU DLC zips and six team skins (check whether `oag-source` reads them as they
  are), and `Mods/Documentation/WAD Filenames` (check against our entry names).

## Next Steps

1. Unpack the 2026 `textures.zip` into its own empty directory outside the repo, point a pack
   folder's `textures.zip` at it, and judge a race on it (hull, track, walls) against the 2020 pack.
2. Take the HUD sheet: scale each `Placed` rect and `alpha_at` by the replacement's factor, or keep
   the logical size and upload at the picture's, then judge the HUD and the fonts in a race.
3. Read ZIM (`flate2` is in the workspace; zstd needs a decoder) and take a Pure oracle dump the
   way Pulse's was to confirm the 650.
