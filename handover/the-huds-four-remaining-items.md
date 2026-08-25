# The HUD's four remaining items

The layout was never an RE problem - Pulse ships five layouts as `Data\XML\*_HUD.xml`. What is left is on [hud.md](../docs/ui/hud.md), and the one worth naming here: **a mode's code substitutes string keys into widgets the layout positioned** - a time trial's top right reads `IG_HUD_RECORD` where `TimeTrial_HUD.xml` says `IG_HUD_TOTAL`, and 26 of the binary's 38 `IG_HUD_*` keys appear in no layout at all. The substitution rule is unread and deliberately **not** worked around by hardcoding. Zone, Eliminator, `<Mode3D>` and text outlines are scoped out and listed on the page.

## Open

- The mode-code string-key substitution rule (e.g. time trial's `IG_HUD_RECORD` in place of `TimeTrial_HUD.xml`'s `IG_HUD_TOTAL`) is unread
- 26 of the binary's 38 `IG_HUD_*` keys appear in no layout at all
- Zone, Eliminator, `<Mode3D>` and text outlines are scoped out (details on `hud.md`)

## Next Steps

- Read the mode substitution rule that swaps HUD string keys, rather than hardcode a workaround
