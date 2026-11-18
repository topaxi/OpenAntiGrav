# oag-ui

The front end's core: boot movies, menus, screens as data, the HUD's font, the
language strings and pointer handling. It emits a `Draw` list that the host
rasterises.

A drawing crate, not a gameplay one (nothing gameplay-side may depend on it).
`oag-ui-screens` is built on it and it never depends back.
