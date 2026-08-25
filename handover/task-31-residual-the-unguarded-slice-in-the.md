# Task #31 residual: the unguarded `slice(..)` in the race's draw path

`oag-view --collision`'s panic on an empty vertex buffer is fixed. `crates/game/src/race/drawable.rs:275` (`Drawable::draw`) has the same unguarded slice, left alone deliberately: a track always has collision geometry, so guarding it would be speculative. The `orbit` guard is the one with no coverage - it needs a window.

## Open

- `Drawable::draw` (`crates/game/src/race/drawable.rs:275`) has the same unguarded slice as the fixed panic, left unguarded deliberately since a track always has collision geometry
- The `orbit` guard has no test coverage

## Next Steps

- Add coverage for the `orbit` guard - it needs a window
