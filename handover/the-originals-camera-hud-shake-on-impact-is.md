# The original's camera/HUD shake on impact is untraced

A hard wall hit in the original visibly shakes the camera/HUD; `oag_render` has no shake at all. The once-suspected function calls no camera API (contact-response.md carries the retraction) and `FUN_088418e0`'s reactions - sound, hull damage, shield flash - do not shake anything either, so the consumer is elsewhere: likely the camera update reading impact state off the craft, or a screen-space offset in the HUD draw. The severity plumbing recovered for the sparks (`min(|impulse| * 0.0125, 1)`) is the obvious input to look for.

## Open

- The actual camera/HUD shake consumer is not located - the once-suspected function and `FUN_088418e0` are both ruled out.
- `oag_render` has no shake implemented at all.

## Next Steps

- Check the camera update for code reading impact state off the craft.
- Check for a screen-space offset applied in the HUD draw.
- Look for the sparks' severity plumbing (`min(|impulse| * 0.0125, 1)`) as the likely input signal.
