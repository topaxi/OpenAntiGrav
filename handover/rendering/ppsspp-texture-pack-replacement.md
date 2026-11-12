---
status: ready-to-wire
titles: [pulse, pure]
updated: 2026-11-12
summary: "HUD, faces, front-end art and the live MODS toggle done; open: Pure, device limits, .pob, ship skins, real ZIM, end-of-race frames"
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

- ~~Atlases cut by pixel offset: the HUD sprite sheet and `.fnt` fonts~~: drawn from the pack at the picture's size (`Sheet::scale`/`sheet_rect`, `Atlas::from_replaced_font`; `docs/formats/ppsspp-texture-pack.md`, "Wired"). The front end's faces (menu rows, ticker, loading line) landed 2026-11-11 (`oag_ui::font::Atlas::from_pack`, `boot::load_shell`; `docs/formats/ppsspp-texture-pack.md`, "Wired"). The menu and boot **art** landed 2026-11-12 (`Placed::scale` / `Placed::sheet_rect` on every front-end draw, `build_replaced` in `boot/sprites.rs`; `docs/formats/ppsspp-texture-pack.md`, "Wired"). **Still the disc's own**: `.pob` particle textures and ship skins. Each needs a frame at the size a player sees.
- **The HUD sprite sheet is not reduced to a device's texture limit**: `Sheet::build` stacks into columns up to 8192 tall and its width is the sum of the columns, measured 2048x2194 on Pulse's time trial with the Pulse pack (8x `PulseHUD`). A device below about 2200 (a downlevel web adapter) would refuse it in `upload_rgba`; the front end's glyph atlases are guarded (`render::face::fit_to_device`, `docs/formats/ppsspp-texture-pack.md`, "A face is used at most 8x"). Reduce the sheet there (its `scales` and `placed` move with it) or pass the device limit into the boot and the race load.
- **Prompt cells are packed at the atlas's own width**: most of a pack face's rows are button glyphs (the 16x Pulse pack's menu face is 1024x4752 at 4x because its prompts at 8x pass 8192). Packing them across the limit's width keeps the menu face at 8x.
- **No frame of a lock-on sight with a pack on**: the sight's ring sprites are layout sprites of the HUD sheet (`sight_draw::concentric_draws` goes through `Sheet::sheet_rect`, covered by `sprite_pack_tests`, not captured), and its bracket art is `.vex` model art, which the mesh path replaces when it decodes the model (not checked in a frame). Capture one with a lock-on.
- **A pack's HUD font is one grey** (`x2_hud_1.png` is white with alpha), so the baked outline the disc's greys carry is not drawn. Whether PPSSPP draws a dark outline from the game's own vertex colours was not measured.
- ~~The 2026 `textures.zip` has not been drawn~~: drawn 2026-11-10 (578 replaced, hull, track and walls read as the pack's art; `docs/formats/ppsspp-texture-pack.md`, "Wired").
- **ZIM** is read (`oag_texture::zim`: raw, zlib, zstd; RGBA8888, RGBA4444, RGB565; level 0), verified on PPSSPP's own three assets (`ppge_atlas.zim` 4444, two 8888), **not on a pack's**: none we hold has one. Unverified: RGB565, zlib, the colour order in 4444, and PPSSPP's hash-named `<key>.zim` files with no ini line. **Pure** is not wired (a runtime-palette font must never be replaced by data hash alone).
- **One pack at a time**: with two on that name the disc, the first in search order is used and the
  other is reported. Race Remix keys the pack on the track source's serial.
- ~~Not on the menus' own art~~ and ~~a MODS toggle of a pack applies at the next launch~~: both done 2026-11-12. `switch_selected_mod` reloads the shell (`Session::resupply_language`, which hands the renderer the new sheet) for a pack, so faces and art follow at once (`drive_ground_truth::the_mods_page_turns_a_texture_pack_on_and_off_and_the_front_ends_faces_follow_live`).
- **The front end's sheet is 4096x2372 with the 2020 Pulse folder** (`pulse_assets.mip` and `pulse_logo.mip` at 8x). A device whose textures are 2048 a side (a downlevel web adapter) refuses it in `upload_rgba`; reduce the sheet there (its placements' `scale` moves with it) or pass the device limit into the boot, as the faces' `fit_to_device` does. Same open item as the race HUD sheet above.
- **Front-end draws not captured with a pack on**: the end-of-race screens, the campaign, the pause menu and the track panel (Track Select and Ship Select were, live and from boot) go through `Placed::sheet_rect` now (a unit test covers `sprite_draw`; the menu pages, `Show Logo` and the MODS page were captured). A draw that derives a screen rect from a uv's width is the mistake to look for: the stepping arrows' glow was one (fixed). `loading/layout.rs`'s `clamped` compares a uv width with the drawn size and its sheet is not replaced, so it is unaffected today.
- **A live toggle reloads the shell on the UI thread** (about a second in a debug build with the 16x faces); a parked race and the other stages keep what they were built with.
- **Pure's front-end faces are held off** (`FrontEnd::runtime_palette_faces`): the 2020 Pure folder's `FX300ANG`/`LTe50325` hit by full key but Pure's palette moves at run time and no oracle dump says what the original draws. Take the dump, then flip the flag; the Pure race HUD still asks the pack.
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
5. ~~Front-end faces.~~ Done. ~~Front-end sheet art and the live MODS toggle.~~ Done. Next: capture the end-of-race, campaign and pause menu with a pack on; reduce the sheets to a device's texture limit.
