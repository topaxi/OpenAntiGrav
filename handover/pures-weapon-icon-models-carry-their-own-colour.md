# Pure's weapon-icon `<Model>`s carry their own authored colour; drawing them is not built

2026-09-04. `docs/gameplay/pickups.md`'s "Pure does the same thing with a different widget kind" section and `crates/pure/src/hud.rs`'s module doc have the full measurement: `Arcade_HUD.xml` and `TimeTrial_HUD.xml` draw each weapon icon as a `<Mode3D><Model>` rather than an `<Image>`, ten of them with an authored `colour="0xAARRGGBB"` beside the `.vex`. `oag_game::hud::Model::colour` parses it - confidence 95, read straight off the disc - but nothing selects a `<WEAPON>_icon` model by the held weapon or turns one into a `Draw::Sprite`. This also corrects a same-day claim: `oag_pure::hud::ART` briefly said Pure "has no per-weapon icon widget of any kind to colour", from grepping `<Image name=` alone and never checking for `<Model name=` inside a `<Mode3D>` block.

## Open

- Pure's ten weapon-icon models never draw: no lookup from a held `Weapon` to a `<WEAPON>_icon` widget name, and no rendering path from a `.vex` mesh plus its `colour` to an on-screen quad
- `DISRUPTOR_icon` names a weapon `oag_formats::weapons::Weapon` has no variant for - Pure's own roster genuinely differs from Pulse's (also missing `Cannon`, and missing `LeachBeam`/`Repulser`/`Shuriken` the other way), not a naming mismatch to resolve by guessing
- The sight brackets' rendering path (`model_art`, `SIGHT_SIZE`) is the only working `<Model>`-drawing code so far, and its size/UV convention was read off one mesh by hand - doing the same for ten weapon-icon meshes needs its own reading, not a copy-and-hope

## Next Steps

- Read `Weapon_rockets.vex` and its nine siblings for whatever size/UV convention the existing sight models use, or find the runtime function that turns a `<Model>` into a drawn quad and read its actual rule rather than inferring one from `SIGHT_SIZE`
- Add a title-configurable name rule for "the widget name Pure's pickup icon uses for weapon X" - `<WEAPON>_icon`, uppercase-underscore, distinct from Pulse's `pickup_icon_name` (`format!("{}Icon", weapon.as_type())`)
- Decide whether `Disruptor` needs its own `Weapon`-adjacent vocabulary (a title-scoped enum, or a name-keyed table alongside `Weapon` rather than indexed by it) before wiring the lookup, since `oag_title::HudArt::pickup_colours` is `Weapon`-indexed and does not fit a roster `Weapon` does not cover
- A capture harness that reaches a weapon pad on Pure (Xvfb + PPSSPP, `docs/reverse-engineering/ppsspp-debugger.md` - steering is digital, D-pad `left`/`right`, not the analog stick) would let a drawn icon be checked against the original the way `pickups.md`'s Pulse reference frames were, rather than shipped unverified
