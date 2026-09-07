# Pure's string tables are not read, and its front-end font is absent

Two separate gaps a Pure boot prints and nothing chases. `German names no string table` - `load_strings` finds no entries for Pure's language plugins, so every `idstring` falls back to its own id. And `Data\FE\Fonts\pulse_text.fnt` is not in Pure's `Data.wad` (hash `f55e014c`), so the whole front end draws in the built-in 5x7 glyphs rather than the disc's own font - which is why Pure's picker looks blocky beside the PPSSPP capture. Neither blocks the boot sequence; both make every Pure screenshot a poor likeness.

## Open

- `load_strings` finds no entries for Pure's language plugins; every `idstring` falls back to its own id
- `Data\FE\Fonts\pulse_text.fnt` is not in Pure's `Data.wad`, so the front end draws in built-in 5x7 glyphs instead of the disc's own font

## Next Steps

- No next step named in the original record - read the prose above and decide one.
