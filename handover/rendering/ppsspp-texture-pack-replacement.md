---
status: ready-to-wire
titles: [pulse, pure]
updated: 2026-11-10
summary: "Race HUD sheet, HUD fonts and ZIM done; open: Pure, front-end/menu art, loading screen, .pob, ship skins, a real pack ZIM"
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

- ~~Atlases cut by pixel offset: the HUD sprite sheet and `.fnt` fonts~~: drawn from the pack at the picture's size (`Sheet::scale`/`sheet_rect`, `Atlas::from_replaced_font`; `docs/formats/ppsspp-texture-pack.md`, "Wired"). **Still the disc's own**: the menu and boot art and fonts (the front end's sheets add rects to `placed.x` unscaled, so `Sheet::build_with` never asks the pack; a front-end lane has to move each draw onto `sheet_rect` first), the loading screen, `.pob` particle textures and ship skins. Each needs a frame at the size a player sees.
- **No frame of a lock-on sight with a pack on**: its art is `.vex` model art (`vex_model_art`), which the mesh path already replaces and the sheet takes whole (uv is the whole image, so the picture's size does not matter); read, not captured.
- **A pack's HUD font is one grey** (`x2_hud_1.png` is white with alpha), so the baked outline the disc's greys carry is not drawn. Whether PPSSPP draws a dark outline from the game's own vertex colours was not measured.
- ~~The 2026 `textures.zip` has not been drawn~~: drawn 2026-11-10 (578 replaced, hull, track and walls read as the pack's art; `docs/formats/ppsspp-texture-pack.md`, "Wired").
- **ZIM** is read (`oag_texture::zim`: raw, zlib, zstd; RGBA8888, RGBA4444, RGB565; level 0), verified on PPSSPP's own three assets (`ppge_atlas.zim` 4444, two 8888), **not on a pack's**: none we hold has one. Unverified: RGB565, zlib, the colour order in 4444, and PPSSPP's hash-named `<key>.zim` files with no ini line. **Pure** is not wired (a runtime-palette font must never be replaced by data hash alone).
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

1. ~~Unpack the 2026 `textures.zip` and judge a race on it.~~ Done.
2. ~~HUD sheet and fonts.~~ Done for the race HUD.
3. ~~Read ZIM.~~ Done; get a real pack's `.zim` (the forum's late pack) and run `zim_ground_truth`'s decode over it.
4. Pure: take a Pure oracle dump the way Pulse's was to confirm the 650, then wire it behind the runtime-palette guard.
5. A front-end lane: move the menu and boot sheets onto `Sheet::sheet_rect`, then call `build_replaced` there.
