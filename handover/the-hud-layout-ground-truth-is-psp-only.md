# The HUD layout ground truth is PSP-only, and the PS2 layouts are unchecked

Measured 2026-08-09 while settling `frontend::SCREEN`, and re-measured coordinate by coordinate after the first phrasing over-claimed: the PS2 release authors the same five HUD layouts in its own **640x448** grid. `Data\XML\Arcade_HUD.xml` carries 141 coordinate values on both discs in the same order, and **121 of them are the PSP's own value scaled by 640/480 or 448/272 and rounded to an integer**. `Skin.xml` was measured the same way on 2026-08-10, after a review caught `frontend::Space` making the same over-claim this row had already been corrected for: **30 of its 43 shared coordinates scale, 13 do not**, and the exceptions are pinned in `crates/game/tests/frontend_grid_ground_truth.rs`. Three of those thirteen are `<Animation><Key>` travels rather than positions - the same mixed meaning of `x` as `TimeDiffIcon` below, in a second file, which makes it a fact about the dialect rather than about one layout. That file also pins **which PSP pressing the PS2 layout came from**: `y=220` USA, `y=230` EU, and the PS2's 362 scales from 220, so a EU-to-EU comparison of that widget looks 4% wrong for a reason that is not the console. The 20 that are not are 18 `<Mode3D><Model>` placements, unchanged at the PSP's `x="-240" y="136"` in an orthographic 3D mode nothing here places, and `TimeDiffIcon`'s two small negative nudges inside an `<Item>`. **It is not a flat scaling, and that matters for the fix**: a PS2 sweep needs more than `Space::PS2.size` in `inside_screen`, because it also has to agree with the `RUNTIME_ANCHORED` skip about which coordinates are screen positions at all. Consequence: `hud::inside_screen` and `hud_layout_ground_truth::every_widget_lands_on_screen` are PSP-only by construction, so **no PS2 HUD layout has ever been checked for a dropped `<Item>` offset**, which is the defect that test exists to catch. The runtime draw path is unaffected - it takes its `screen` uniform from the source's own space. The sweep needs `Space::PS2.size` in `inside_screen`, a second `open()` in that test, and a decision about the unscaled offsets above.

## Open

- No PS2 HUD layout has ever been checked for a dropped `<Item>` offset - `hud::inside_screen` and `hud_layout_ground_truth::every_widget_lands_on_screen` are PSP-only by construction
- Scaling is not flat: 13 of `Skin.xml`'s 43 shared coordinates do not scale, and it is undecided how to treat the unscaled offsets (18 `<Mode3D><Model>` placements, `TimeDiffIcon`'s two nudges)

## Next Steps

- Add `Space::PS2.size` to `inside_screen`
- Add a second `open()` call in `hud_layout_ground_truth` for the PS2 layout
- Decide how to treat the unscaled offsets (the `<Mode3D><Model>` placements and `TimeDiffIcon`'s nudges)
