# Front-end gaps behind `Image`

`screen.rs` reads only a screen's direct children, so `Show Logo`'s `BOOT_LEGAL` line is absent - now a *guarded* absence. The fix is not a scissor rect: the `Viewport` around it clips nothing vertically, and what the line needs is `widthlimited="true"`, i.e. wrapping a 118-character string, and `Draw::Text` has no width or wrap. That is text layout. USA disc only - EU drops both. An `Image` with no `x` is centred on a guess.

## Open

- `Show Logo`'s `BOOT_LEGAL` line is absent because `screen.rs` reads only a screen's direct children
- `Draw::Text` has no width or wrap support, which the `widthlimited="true"` 118-character string needs
- An `Image` with no `x` is centred on a guess, unverified
- USA disc only - EU drops both lines

## Next Steps

- Add text layout/wrapping to `Draw::Text` so `widthlimited="true"` can render, fixing `BOOT_LEGAL`
