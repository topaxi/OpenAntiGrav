# Implement `BOOT_PRESS_START`'s fade-in and throb

`BOOT_PRESS_START` carries `pulse="true"` and `delay="1"`, so on hardware it
fades in a second after `Show Logo` appears and then throbs. This build draws
it static. The two numbers implementing the throb needs are now measured (see
[frontend-boot.md](../docs/architecture/frontend-boot.md), the
`BOOT_PRESS_START does not pulse` section): period ≈1.10 s, holding at a dim
floor and a bright ceiling roughly 42% apart in luminance rather than fading
to black, confidence 75 - a screen-pixel luminance proxy, not a live read of
the widget's own alpha. That confidence gap, and the `delay="1"` fade-in
itself (not re-measured this pass), are `## Open` below.

Neither `pulse` nor `delay` is parsed anywhere in this crate today, and there
is no per-widget animation-state struct in the boot/frontend sequence to hang
one off - both confirmed by reading the code, not assumed:

- `crates/game/src/screen.rs`'s `Text` struct (lines 105-141) has no `pulse`
  or `delay` field; `Skin::text_from_node` (lines 601-622) never calls
  `node.flag("pulse")` or reads a `delay` value. The doc comment on
  `collect_widgets` (lines 459-473) already names this as a known gap shared
  with every other widget's `delay`/`transition` attribute.
- `crates/game/src/frontend/draw.rs`'s `Frontend::draw_screen` (lines
  461-500) builds `Draw::Text` straight from the XML colour with no
  time-based modulation; its own doc comment says so explicitly ("no state,
  no input, no animation").
- `Frontend::on_screen_for: f64` (`crates/game/src/frontend.rs:374-375`) is
  already exactly "time since `Show Logo` was entered" and could drive
  `delay`-then-pulse without new state, unless a second timed widget shows up
  on the same screen needing an independent clock.
- The menu system (not the boot/frontend one) already has the idiom to copy:
  `Draw::colour_mut()` (`frontend/draw.rs:693-711`) plus `menu.rs`'s
  `fade(draw, alpha)` (lines 1844-1848), which multiplies `color[3]` in
  place. Reusing that pattern needs no new generic abstraction.

## Open

- The luminance-ratio measurement (≈42% floor/ceiling) is a pixel proxy, not
  a direct read of the widget's own 0-255/0.0-1.0 alpha - the exact pair to
  hard-code is a judgement call from that ratio, not a value read off memory.
- `delay="1"`'s fade-in (from invisible to the throb's floor, a second after
  `Show Logo` appears) was not captured this pass; the 20 s captures started
  well after boot, so they only cover the repeating throb, not the one-time
  fade-in's own shape or duration.
- Whether `on_screen_for` is precise enough to drive this alone, or whether a
  second widget on `Show Logo` (or a future screen) needing independent
  timing forces a per-widget clock, is a design call for whoever implements
  this - not investigated here.

## Next Steps

- Add `pulse: bool` and `delay: f32` to `Text` in `crates/game/src/screen.rs`
  (near `start_enabled`, line ~126) and read them in `text_from_node` (line
  601) via the existing `Node::flag`/`Skin::number` helpers - no parser
  changes needed.
- In `Frontend::draw_screen` (`crates/game/src/frontend/draw.rs:461-500`),
  gate a pulse/delay alpha multiplier on `text.pulse`/`text.delay` being set,
  using `self.on_screen_for` as the clock and the `Draw::colour_mut()` +
  `fade()`-style idiom from `menu.rs:1844` to apply it after building
  `Draw::Text`.
- Pick and document a concrete `delay="1"` fade-in shape (linear ease is the
  obvious default absent better evidence) since it was not captured, and say
  so next to the choice rather than presenting it as measured.
- If a fresh capture ever earns a live memory read of `Show Logo`'s widget
  alpha (rather than a screen-pixel proxy), it would raise confidence past 75
  and is worth doing before this is called fully closed - but is not a
  blocker to landing a reasonable implementation off the current numbers.
