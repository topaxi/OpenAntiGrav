# Front-end gaps behind `Image`

`BOOT_LEGAL` now draws, wrapped, on the USA disc - see
`docs/architecture/frontend-boot.md`'s `Viewport`/`widthlimited` section for
the fix and its evidence. What is left is the one claim in this thread that
was never about `BOOT_LEGAL` at all: `Show Logo`'s `Image` (the `pulse_logo`
sprite) has a `y` and no `x`, and is drawn centred on a texture-column reading
rather than on anything that says "centred" directly.

## Open

- An `Image` with no `x` is centred on a guess (confidence 85, per
  `docs/architecture/frontend-boot.md`'s `An Image with no x is centred`
  section) - the texture is transparent out to column 23 and from column 488
  of its own 512, so centring and left-pinning differ by 16px, but nothing
  says which the disc actually does with an unrendered widget. It would take a
  captured real-hardware or PPSSPP frame of `Show Logo` to settle at higher
  confidence than "measured from the one image there is".

## Next Steps

- Get a reference frame of `Show Logo` (PPSSPP screenshot, or the user's own
  play capture) and compare the logo's left/right margins against both
  hypotheses - centred (7..471) and left-pinned (0..487) predict different
  numbers, so one frame settles it.
