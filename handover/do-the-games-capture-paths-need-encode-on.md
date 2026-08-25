# Do the game's capture paths need encode-on-write?

Answered "no" by [ADR-0020](../docs/architecture/adr/0020-gamma-authoritative-colour-space.md) for the colour-space question, but the front end's **authored text and fill colours** were tuned against a window and there is no measurement of the text case the way `render.rs` measured the sprite case (sprites are unlit, so that answer was clean). Settle it with a window-versus-capture comparison of a text-heavy screen before changing anything.

## Open

- Colour-space question answered "no" by ADR-0020, but the front end's text/fill colours were only tuned against a window, never measured against a capture
- No measurement of the text case exists, unlike the sprite case (`render.rs` measured that one cleanly because sprites are unlit)

## Next Steps

- Run a window-versus-capture comparison of a text-heavy screen before changing anything
