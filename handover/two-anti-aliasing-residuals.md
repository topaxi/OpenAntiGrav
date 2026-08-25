# Two anti-aliasing residuals

MSAA and FXAA/SMAA are in; TAA is deliberately not. (1) `Entry::warning` is a single `Option<Warning>` and cannot express two independently-triggered warnings without restructuring that type, so the 200 % render-scale case has no live warning. (2) There is no `--anti-aliasing` CLI flag, unlike `--upscaler`/`--render-scale`; add one the same way if an AA comparison capture needs it. Note **MSAA 2x fails wgpu validation here** - `sample_count 2` needs a device feature this project never requests.

## Open

- `Entry::warning`'s single `Option<Warning>` cannot express two independently-triggered warnings, so the 200% render-scale case has no live warning.
- There is no `--anti-aliasing` CLI flag.
- MSAA 2x fails wgpu validation here since `sample_count 2` needs a device feature this project never requests.

## Next Steps

- Restructure `Entry::warning` to express more than one warning, to cover the 200% render-scale case.
- Add a `--anti-aliasing` CLI flag the same way as `--upscaler`/`--render-scale`, if an AA comparison capture needs it.
