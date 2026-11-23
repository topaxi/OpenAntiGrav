# The footer legend on Omega: `NavigationController_Layout`

2026-11-23, lane `hd-legend-re`. Wipeout Omega Collection EU `eboot.bin` (`/omega/eboot-ps4-omega-eu.bin`, image base
`0x01000000`), static reading, checked against the `omega/menu-cell-select` pair (shadPS4, 1920x1080, cold boot,
2026-10-26). HD's law, which this one departs from, is [../ps3-hdfury-eu/navigation-legend.md](../ps3-hdfury-eu/navigation-legend.md).

**Checked, differs** from HD. Omega's `skin.xml` (`data09.psarc`) authors the same `NavigationController` children
with the same x and `scale="0.8"`, and the class is the same file (`C:\WOPS4\Wipeout\Code\Frontend\Items\NavigationController_Item.cpp`,
`0x0181d77b`). The layout is rewritten.

## `NavigationController_Init` - `0x01475290`

Confidence **85**. Finds `ControlTextConfirm`, `ControlTextBack`, `ControlTextConfirmButton`, `ControlTextBackButton`
(`+0x100`, `+0x108`, `+0x110`, `+0x118`), the shared Invite pair (`0x01afdd80`, `0x01afdd88`), then
`ControlTextNavigation` and a new `ControlTextNavigationRL`. Left to right: `cursor = (int)(w(nav) + 20.0 + nav.x)`
(x at `Text + 0x114`, width the vtable slot `+0xa0`), flag `+0x140` = 0, the same seed as HD. Right to left (only
`ControlTextNavigationRL` present): `cursor = (int)(navRL.x - (w + 20))` and flag 1.

## `NavigationController_Layout` - `0x01475f50`

Confidence **85**. Left to right, for each pair (glyph `g`, word `t`) in HD's order (Confirm, Back, the registered
extras from `0x01afdd30`, Invite last):

```text
g.x = cursor
t.x = cursor + (int)w(g) + 14
cursor = cursor + (int)w(g) + (int)w(t) + 34      ; the word's end plus 20
```

HD's fixed 44 (or 84 for a `ther1button` font) is gone: the word follows the glyph's own measured width plus 14.
The right-to-left branch mirrors it (`g.x = cursor - 14 - w(g) - w(t)`, `t.x = cursor - w(t)`, `cursor -= 34 + w(g) + w(t)`).

## Against the frame

With HD's `helv.fnt`/`ps_buttons.fnt` metrics at scale 0.8 (Omega ships the same faces at twice the texel size and
lays them out at half, `oag_ui::font::Atlas::with_texel_scale`) and **no** pen snap, at 1:1 (the pair records no Safe
Area, and `ControlTextNavigation` sits at its authored 260):

| Prompt | Predicted x | Original x |
| --- | ---: | ---: |
| `NAVIGATION` | 260 | 260 |
| Cross glyph | 436 | 435 |
| `CONFIRM` | 488 | 487 |
| Circle glyph | 627 | 625 |
| `BACK` | 679 | 677 |
| Triangle glyph | 771 | 768 |
| `RECORDS` | 823 | 820 |
| Square glyph | 969 | 965 |
| `AI DIFFICULTY (NOVICE)` | 1021 | 1017 |

Glyph to word is 52 px in the frame on all four prompts (`(int)(48 * 0.8) + 14`). The prediction drifts about 1 px
per prompt; with HD's `ceilf` snap it drifts 4 to 14 px, so Omega's measure most likely does not snap, or snaps at
its doubled texel size. Not read: Omega's `Text` width virtual (`+0xa0`) and its font measure. Read recipe: follow the
`+0xa0` slot of a `Text` vtable to the measure, as HD's page does through `0x00202748`, and run the table on
Omega's own `helv.fnt` (`data00.psarc`).

## Wired

2026-11-23, `hd-legend-wire`: `oag_title::legend::LegendFlow` for Omega is `gap 20`, word at the glyph's measured width plus 14,
**no pen snap** (**chosen, not measured**: the prediction above drifts 4 to 14 px with the snap and about 1 px a prompt without;
Omega's own measure is unread). On `omega/menu-cell-select` the glyphs and words land at 435, 487, 625, 677, 768, 820, 965 and
1017 within 3 px (`omegas_footer_prompts_flow_to_where_the_original_draws_them`). The original draws Records (the triangle) before
Difficulty; ours dropped it because the file authors both at x 944, and now draws it. Nothing is behind the prompt (no leaderboard; triangle opens no panel on Omega's Cell Selection, which authors no `Cell
Help`), and it is drawn in Difficulty's colour, which differs from the other prompts'. **2048: checked, differs**: its front-end
root authors no `NavigationController` and ours reads no legend from it (`the_2048_root_authors_no_footer_legend`), so there is
nothing to flow.

## Names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x01475290` | `NavigationController_Init` | 85 |
| `0x01475f50` | `NavigationController_Layout` | 85 |
