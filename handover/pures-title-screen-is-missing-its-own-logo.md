# Pure's `Title Screen` is missing its own logo wordmark and most of its frame-line decoration - not chased further

Comparing `oag-game`'s own render against the real PPSSPP capture: the big "WipEout"/"pure" logo (`TitleFrame`, `x=0 y=76 width=480 height=128`, `StartEnabled="false"`) never gets a `src` in `Skin.xml` at all - it is set programmatically on the original, the same way Pulse's own dangling globals are, and finding it needs either Ghidra (where `TitleFrame`'s texture gets assigned) or a full image-content scan of `Data.wad`/`FE.wad`, not a name guess - one guess (`Data\FE\Images\Logo.mip`, hash `be900df4`) did resolve to a real entry but at 2064 bytes is far too small to be a 480x128 texture, so it was not pursued. Separately, `Title Screen` authors five `Animation`-driven corner-line `Image`s sharing one source (`Data\FE\Images\FETextures_startscreen.mip`) at different `U`/`V` sub-rects - `Image` (`crates/game/src/screen.rs`) does not capture `U`/`V` at all, only `x`/`y`/`width`/`height`, so all five collapse onto whichever one `draw_backdrops` happens to place, drawn at the wrong position. Confirmed this is not new: Pulse's own `ArrowSelect` widget authors `U="53" V="0"` and would have the same problem, unconfirmed whether it is visibly wrong there too.

## Open

- `TitleFrame`'s logo texture has no `src` in `Skin.xml` - it is set programmatically on the original
- The one guessed candidate (`Data\FE\Images\Logo.mip`) is far too small to be the 480x128 texture
- `Image` (`crates/game/src/screen.rs`) does not capture `U`/`V`, so five corner-line images collapse onto one position
- Unconfirmed whether Pulse's `ArrowSelect` widget shows the same `U`/`V` problem visibly

## Next Steps

- Find the logo texture via Ghidra (where `TitleFrame`'s texture is assigned) or a full image-content scan of `Data.wad`/`FE.wad`
- Add `U`/`V` sub-rect capture to `Image` so multi-source `Animation` images position correctly
- Check whether Pulse's `ArrowSelect` widget is visibly wrong for the same reason
