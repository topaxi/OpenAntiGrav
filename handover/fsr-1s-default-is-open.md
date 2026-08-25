# FSR 1's default is open

`oag_render::post::fsr1` transliterates AMD's MIT `ffx_fsr1.h`; the route for FSR 3.1 is settled in [ADR-0012](../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md). It cannot reach the front end because `capture::run`'s front-end path has no `Framebuffer`, and giving it one is the same work as the UI-compositing restructure ([modern features](../docs/overview/modern-features.md) has the prerequisite table). **Do not** read a menu capture taken with `--upscaler` as evidence either way: the flag changes the UPSCALER row's own text, so the two images differ for an unrelated reason - that nearly produced a false conclusion.

## Open

- FSR1 cannot reach the front end because `capture::run`'s front-end path has no `Framebuffer`
- A menu capture taken with `--upscaler` is not valid evidence either way (the flag itself changes the UPSCALER row's text)

## Next Steps

- Do the UI-compositing restructure that gives the front-end path a `Framebuffer` (prerequisite table in [modern-features.md](../docs/overview/modern-features.md))
