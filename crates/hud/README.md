# oag-hud

The in-race HUD: its layout model read from the disc's own HUD XML, the
readouts, the draw list and the sprite sheet every screen shares.

It has no GPU pass of its own; the host draws its list. A drawing crate: nothing
gameplay-side may depend on it.
