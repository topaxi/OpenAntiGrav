---
status: ready-to-wire
titles: [all]
updated: 2026-11-04
summary: "Wide-key footer layout; the live Steam Deck check; HD arrow marks and unnamed wide variants"
size: S (30)
difficulty: easy (25)
---

# Button prompts: what is left after the first landing

2026-10-06, branch `button-glyphs`. The evidence and the design are in
[button-prompts.md](../../docs/ui/button-prompts.md); this lists only what is
open.

## Open

- **First check on a real Steam launch.** The Steam Input path (the
  `SteamVirtualGamepadInfo` file naming the real controller, `SteamDeck=1`) is
  tested against the format SDL's source defines, not a live Steam. On a Deck,
  launch through Steam and read the `button prompts:` info line: it names the
  family and the last-used device. If the file is absent or the slot does not
  match the pad's name, `Launch::family_of` falls back to the pad's own ids
  (Xbox for a virtual pad). **Deferred until the maintainer has a
  controller**: the 2026-10-06 launch had none, so the live Steam Input pad
  path is still unobserved.
- **Debug-build Vulkan validation noise.** A debug build logs about 20 x
  `VUID-vkAcquireNextImageKHR-fence-10066` (the fence is already in use) per
  run on RADV. Recorded, not fixed: look at the swapchain's acquire fence in
  `oag-game`'s present path.
- **Steam runs the game under a separate home**, `/home/topaxi-insecure`, which
  is where `SteamVirtualGamepadInfo` points; on 2026-10-06 that file was
  empty because no controller was attached. The game's own log is under that
  home's `~/.local/state/oag/logs/` for a Steam launch.
- **HD's `Δ Γ Β Α`** (four arrow-like marks in `PS_BUTTONS.fnt`) stay the
  disc's: which direction each is was not established.
- **A wide key on a small face.** The keyboard's backspace is fitted into the
  12 px Pulse box and its label is a smear. Growing the box overlaps the next
  label (the footers place labels at authored x, not after the glyph), so a fix
  is a layout one: let the footer step by measured advance, or draw key names
  as text.
- **Start, select and the shoulders' wide variants** have no stand-in
  codepoint in either title's table, so no prompt names them.
- **No Nintendo-coloured face glyphs** in PromptFont; lettered discs are used.
  A different set would be a one-line table change in `crates/game/src/prompts.rs`.

## Next Steps

1. Decide the wide-key layout (see above) with a screenshot of the footer.
2. If a HUD or race-start string ever carries a stand-in, give `hud_font` the
   `with_prompts` call and `hud_overlay::Overlay` a fan-out; today none does
   (`docs/ui/button-prompts.md`, "Where it applies").
