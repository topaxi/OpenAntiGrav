# FSR 1's default is open

`oag_render::post::fsr1` transliterates AMD's MIT `ffx_fsr1.h`; the route for FSR 3.1 is settled in [ADR-0012](../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md). It cannot reach the front end because `capture::run`'s front-end path has no `Framebuffer`, and giving it one is the same work as the UI-compositing restructure ([modern features](../docs/overview/modern-features.md) has the prerequisite table). **Do not** read a menu capture taken with `--upscaler` as evidence either way: the flag changes the UPSCALER row's own text, so the two images differ for an unrelated reason - that nearly produced a false conclusion.

## Phase 0, shared: the UI composites at presentation resolution

**This section is duplicated, on purpose, in
[Dynamic resolution wants a viewport, not an allocation](dynamic-resolution-wants-a-viewport-not-an-allocation.md) - whoever picks up either thread does this
first, and it is the same piece of work both times.** Duplicated rather than
linked because a thread file is deleted the moment its work lands: if the other
file is gone, this work is very likely already done, and
[modern-features.md](../docs/overview/modern-features.md)'s prerequisite table
is the thing to check before starting rather than the missing file.
When it lands, delete this section from whichever thread is still open.

**Roughly two days.** The scene resolves to the surface first (blit or
upscaler, exactly as `upscale::Framebuffer::resolve` does now), and the HUD,
the menus, the front end and the perf overlay draw *after* it, into the
surface, at presentation size.

Three things it has to get past, all of them already in the tree:

- **`capture::run`'s front-end path has no `Framebuffer` at all**
  (`crates/game/src/capture.rs:28`). That is the whole of the difficulty and
  the reason FSR 1 has never reached the front end. The race path is not the
  same shape - `crates/game/src/race/capture.rs:380` builds one conditionally
  on `presented` and calls `upscale::target_size` itself at line 317 - so both
  paths need looking at, and a fix to one is not a fix to the other.
- **`perf.rs`'s "the overlay should cost what the game costs" is the argument
  that has to be consciously overturned.** It is right for a fixed render scale
  and wrong for a moving one: the overlay is the row a player uses to judge
  what the resolution controller is doing, and an overlay that resamples with
  the scene cannot be read while it moves.
- **The 480x272 authored grid** (`display.rs:50`) is what every overlay lays
  out in. Compositing later means scaling that grid to the *surface* rather
  than to the offscreen target - one substitution, in more places than it
  looks.

What it delivers, in the words `docs/overview/modern-features.md` uses: the
prerequisite table's last row, *a scene without UI in it*, stops being
**Absent**. Update that row in the same change.

## Open

- FSR1 cannot reach the front end because `capture::run`'s front-end path has no `Framebuffer`
- A menu capture taken with `--upscaler` is not valid evidence either way (the flag itself changes the UPSCALER row's text)
- Whether the default should move to `fsr1` at all. It has been compared once, at 50 % on one frame of one track, and FSR 1 won clearly - but the doubt is about content that frame did not contain: the menus and the HUD are 480x272-era paletted raster and glyphs off a coverage atlas, and a sharpener rings on those. The comparison that settles it is only possible *after* Phase 0, because before it there is no menu frame an upscaler has ever touched.

## Next Steps

- Do Phase 0 above. It is the prerequisite for everything else here, and it is shared with [dynamic-resolution-wants-a-viewport-not-an-allocation.md](dynamic-resolution-wants-a-viewport-not-an-allocation.md).
- Then re-run the default comparison on menu and HUD content, which is the sample the default flip is actually held to.
