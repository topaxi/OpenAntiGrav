---
status: ready-to-wire
titles: [pulse, pure]
updated: 2026-11-08
summary: "Draw a PPSSPP texture pack as an off-by-default mod: keys reproduce (2026 pack 5,039 of 7,476); wire lookup, zip and ZIM reading, rendering only"
categories: [tooling]
size: L (60)
difficulty: medium (60)
---

# Draw a PPSSPP texture pack (opt-in)

2026-11-08, lane `psp-texture-packs`. The measure half is done and wires no drawing:
[ppsspp-texture-pack.md](../../docs/formats/ppsspp-texture-pack.md) has the key, the
census, the oracle against PPSSPP's own dump of Pulse EU, and a "Ready to wire"
section. `just fetch-texture-packs` fetches the packs; `just push-data` carries them.

## Open

- **The wiring.** `oag_texture::replacement::Stored::replacement` keys a texture from its
  file bytes; `replacement_pack::Pack` reads `textures.ini`. Nothing looks a texture up,
  loads a PNG or binds a replacement. Opt-in setting, default off, lazy (load a PNG when
  its texture is first drawn), rendering only (never `World` state, never a snapshot).
- **A setting and a loader-report line** (`warn` when asked for and the pack is absent;
  `debug` for the count replaced). Search order: `data/texture-packs/<name>/`, then a PPSSPP
  `PSP/TEXTURES/<serial>/`.
- **Replacement size.** The packs' PNGs are 4x the original on 101 of 175 sampled Pulse
  keys, but 1x to 16x and sometimes per axis; fonts and the HUD sheet atlas by pixel offset
  and must be checked in a race.
- **Heap-tail textures (4-bit, under 1,024 bytes)**: their key depends on the allocator;
  look them up by data hash alone (PPSSPP's `(0, data)` wildcard). Runtime-palette fonts
  (Pure) must not be replaced by data hash alone.
- **Only 1,592 Pulse and 110 Pure reproduced keys name a PNG the pack ships.**
- Unresolved misses: Pulse ship hull texels rewritten after load (167 ship keys), Pure
  `main_menu/previews` (truecolour images converted at load), about 20 effect textures
  with no source on the disc.
- **The maintainer wants a pack to be a mod** (enabled and disabled like any other, off by
  default): the design is the doc page's "A pack is a mod to the game" (native recognition
  of a `textures.ini` folder or zip in `oag-mod`, `[texture-pack]`, `default-enabled`, a
  PPSSPP memstick search root). It is a mod-format change: update the `oag-mod-author`
  skill and `docs/tools/mods.md` in the same change.
- **`textures.zip` read in place** (the 2026 pack is only that) and **ZIM images** (a later
  pack's format, layout on the doc page) are first-class, not optional.

- **Size, estimated (lead, 2026-11-08), not measured.** About 2,000 to 3,000 changed lines with
  tests and docs. Roughly: `oag-mod` recognition, manifest, `default-enabled` and the MODS page
  400 to 600; lookup at every PSP texture load 500 to 900; lazy PNG decode, cache and memory
  cap 300 to 400 (553 MB of images cannot all be resident on the Deck); ZIM 200; tests, docs
  and skill 500 to 800. The pixel-offset atlases (fonts, HUD sheet) are the overrun risk.
  **Split it into two lanes:** (a) a pack is an off-by-default mod, done when the MODS page
  lists the 2026 pack and the reTokyo one; (b) drawing, done when menus and tracks show the
  pack in frames judged in a race.
- **Later version (maintainer):** an in-game download-and-enable button, instead of or beside
  `just fetch-texture-packs`.
- **Other assets in the maintainer's `data/GitHub - WipEout Mods.zip`** (NR74W's mods, the
  "WipEout Portable Collection"; replicating the collection is out of scope): a Pulse PS2 PCSX2
  pack (637 MB, PCSX2's own hash, a second importer), Pulse EU DLC zips and six team skins
  (check whether `oag-source` reads them as they are), and `Mods/Documentation/WAD Filenames`
  (check against our entry names).

## Next Steps

1. Make a pack a mod (see above), then pick where the lookup sits (a render-side `Replacements` the texture upload asks, keyed by
   `Stored::replacement`), and read `Pack` once per boot.
2. Wire Pulse's `.vex` and `.mip` paths first (91.6 % of its keys); judge menu, track and
   hull draws in a race at the size a player sees, with the pack on and off.
3. Take a Pure oracle dump the way Pulse's was (hardware renderer; `SaveNewTextures`), to
   confirm the 650.
