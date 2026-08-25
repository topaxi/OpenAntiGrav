# The 200% render-scale case has no live warning; `--anti-aliasing` is landed

MSAA and FXAA/SMAA are in; TAA is deliberately not. `Entry::warning` is a single `Option<Warning>` and cannot express two independently-triggered warnings without restructuring that type, so the 200 % render-scale case - real product policy per [ADR-0013](../docs/architecture/adr/0013-anti-aliasing-architecture.md), where SMAA/FXAA on top of 200% supersampling is redundant work rather than a broken combination - has no live warning; only documentation says so. `--anti-aliasing` (mirroring `--upscaler`/`--render-scale`) is landed: `crates/game/src/main/cli.rs`, wired into `main.rs`, overrides `[graphics] anti_aliasing` for the run without touching the settings file. Note **MSAA 2x fails wgpu validation here** - `sample_count 2` needs a device feature this project never requests; that is documented behaviour (ADR-0013), not a bug to fix.

## Open

- `Entry::warning`'s single `Option<Warning>` cannot express two independently-triggered warnings, so the 200% render-scale case has no live warning. ADR-0013 calls this out as "left for a follow-up that touches the menu definition type generally": it touches every `Entry::Choice`/`Entry::Toggle` call site, `menu.toml` parsing and the ground-truth tests that read warning text, not just one flag.

## Next Steps

- Restructure `Entry::warning` (`crates/game/src/menu.rs`) from `Option<Warning>` to something that can carry two independently-triggered warnings, then give `menu.toml`'s ANTI-ALIASING row a `warn_when` for render scale >= 200%. Start from ADR-0013's Alternatives section, which already scoped the shape of the fix.
