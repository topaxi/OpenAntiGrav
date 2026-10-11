# The two asset layers around the disc

The decision is [ADR-0061](adr/0061-two-asset-layers-around-the-disc.md); this page
is the mechanism, the census that justified the first use, and where each title
stands. Code: `oag_assets::layers` (the layer type), `oag_assets::Archives` (the
search order), `oag_source::layers` (where the layers come from).

## The search order

```text
override (player)  ->  update archives  ->  bulk  ->  companion  ->  extra  ->  DLC packs  ->  fallback (this project)
```

First hit wins. Override and fallback are loose entries, not archives: a name
maps to a file or to embedded bytes. A name is compared lowercase, with `/` for
`\`, without a leading separator, so `Data\XML\Foo.xml` (a WAD's spelling) and
`/data/xml/foo.xml` (a PSARC's) are one entry. The hash-addressed reads
(`read_hash`) and the directory-position reads (`read_preceding`/`read_following`,
`read_font`'s atlas) skip both layers: an entry with no hash and no position cannot
answer them. A mod that replaces a PS2 model therefore does not move its texture
set, which sits by position in the disc's own archive.

`locate`, `locations` and `read_every_name` list the layers in the same order.
`Archives::take_layer_report` drains one line per name a layer answered; a race
load appends them to its report (`oag_raceplay::load`). Only layer answers are
kept, so a disc read costs nothing and a load nothing answered for adds no line.

## The override

`overrides/<title>/` where `<title>` is the title's name lowercased and hyphenated
(`wipeout-pulse`, `wipeout-pure`), searched in `data/overrides/` (a checkout),
beside the program, and `<data dir>/oag/overrides/`; `$OAG_OVERRIDES` names the
parent directories outright. Every file under the title's directory is an entry,
named by its relative path: `overrides/wipeout-pulse/Data/XML/Arcade_HUD.xml`
replaces the disc's `Data\XML\Arcade_HUD.xml`. Loose files only, no whole-archive
mounting. Empty by default; user content, never committed; every entry it answers
is reported as replacing the disc's own.

**A mod that changes a table changes the simulation.** A replay carries the
initial state hash and one every interval ([ADR-0055](adr/0055-replays-are-inputs-and-a-ghost-is-poses.md)), so a replay recorded
without the mod and played with it reports a desync instead of diverging; a ghost
is poses and is drawn regardless. The header does not yet say that mods were
active - open.

## The fallback

Carried in the binary from `assets/fallback/` (`oag_source::layers::fallback_layer`),
so a packaged build needs no directory beside it. It answers only a name no archive
holds, so it cannot shadow the disc or its DLC; `asset_layers_ground_truth` pins that
on real Pulse (both pressings) and Pure discs. **Everything in it is ours**:
geometry computed by `scripts/gen-fallback-assist-art.py`, a layout written by hand.
Nothing extracted from a disc may be placed there, and `just audit-leakage` covers
the tree. It is **chosen, not measured** wherever it is described, and the loader
report says so.

### The `OAGMASK` art format

A plain-text alpha mask (`oag_texture::mask`): the header `OAGMASK <w> <h>`, then one
row per line, one character a pixel, `.` clear and `0`-`9`/`a`-`f` white at that
hex digit's share of full alpha. Comment lines start `#`. Colour is the widget's
own tint, as the originals' white HUD masks are. `oag_hud::sprite::Sheet` decodes it
beside the `.mip`, `.gtf`, `.gxt` and `.gnf` it already reads.

## First use: Pilot Assist's indicator on Pulse and Pure

### Census: no pre-HD disc authors one

`scripts/census-assist-indicator.sh` extracts every entry of each WAD, hashes the names
HD and 2048 use for the layout and the art (plus plausible ports of them) against each
WAD's directory, and searches every decompressed entry for "assist" other than the
credits' "Assistant". (`pilot` is left out on purpose: it is the drivers and Autopilot, in
every language.) A WAD stores only name hashes, so a texture's own name cannot be
searched for; `asset_layers_ground_truth::no_pre_hd_hud_layout_authors_an_assist_indicator`
closes that gap from the other side by composing every HUD layout each title ships and
asserting no widget is named like the indicator's, or like assist or pilot. 2026-10-11:

| Image | WAD | Entries | Candidate-name hash hits | Files mentioning "assist" |
| --- | --- | ---: | ---: | ---: |
| pulse-psp-eu | BEData.wad | 54 | 0 | 0 |
| pulse-psp-eu | Data.wad | 1138 | 0 | 0 |
| pulse-psp-eu | FE.wad | 27 | 0 | 0 |
| pulse-psp-eu | FEData.wad | 242 | 0 | 0 |
| pulse-psp-usa | BEData.wad | 54 | 0 | 0 |
| pulse-psp-usa | Data.wad | 1142 | 0 | 0 |
| pulse-psp-usa | FE.wad | 27 | 0 | 0 |
| pulse-psp-usa | FEData.wad | 242 | 0 | 0 |
| pulse-ps2-eu | WADS2.WAD | 7200 | 0 | 0 |
| pulse-ps2-eu | WADSP.WAD | 193 | 0 | 0 |
| pure-psp-eu | Data.wad | 841 | 0 | 0 |
| pure-psp-eu | FE.wad | 158 | 0 | 0 |
| pure-psp-eu | FEData.wad | 242 | 0 | 0 |
| pure-psp-usa | Data.wad | 832 | 0 | 0 |
| pure-psp-usa | FE.wad | 157 | 0 | 0 |
| pure-psp-usa | FEData.wad | 240 | 0 | 0 |

Zero everywhere, on both Pulse PSP pressings, the PS2 pressing and both Pure pressings
(EU and USA), and no widget of any HUD layout either title ships is named for it.
**So the indicator is drawn from the fallback.**

### Wiring

`oag_title::hud::AssistIndicator` (the six widget names, as HD authors them) lives on
`HudArt::assist`, with a `fragment` naming the layout that holds them for a title whose
own layouts do not embed them: `Data\XML\HUD_assist_indicator.xml`, HD's name for it.
`oag_raceplay::splice_assist_indicator` composes that fragment through the archives
and appends its sprites to the race's HUD layout, scaling from the PSP's 480x272 to the
source's grid (the PS2's 640x448). A layout that already carries the widgets is left
alone. On Pulse and Pure nothing holds the name but the fallback; on HD `fragment` is
`None` and the disc's layouts embed the widgets themselves, which the ordering test
exercises. The show and blink rules are HD's, unchanged
([`oag_hud::assist`](../../crates/hud/src/assist.rs)); `RuntimeHud` is untouched, so no
HD shield or arc rule reaches Pulse or Pure.

### Placement is chosen, not measured

HD's layout puts a plate low in the centre, a main icon on it, two small arrows beside
the icon and two large ones at the screen's sides (1920x1080). Pulse and Pure take the same
parts. The placement is **chosen by eye on Pulse's HUD**: the plate low and left of centre,
clear of the craft and its exhaust (centred it vanished into the exhaust glow), the big
arrows at the sides at mid-height. The art is geometry of this project's own: a rounded
plate, a ring with a craft triangle, arrowheads and chevrons.

### Checked against the other titles

- **2048**: draws its own `PilotAssist` icon off the layout (`oag_hud::dialect_2048`, lit
  by `Readout::pilot_assist`), so it has an indicator already and `HudArt::assist` stays
  `None`. Nothing to port; the fallback is never asked.
- **Omega**: ships `Data/xml/2048_hud/HUD_assist_indicator.xml` in `data00.psarc`, and VR
  variants in the day-one patch (`data08.psarc`). **Checked, applies, not wired**:
  `oag_omega::hud`'s `assist` is still `None`; wiring it is naming the widgets and
  checking the fragment composes under Omega's `.gnf` atlases.
- **HD**: unchanged; the disc's layouts embed the widgets, `fragment` is `None`.

## Open

- The override has no hash-addressed or archive-mounting form, and the replay header does
  not record that mods were active.
- PromptFont glyphs (`assets/ui/prompts`) and the cursors (`assets/cursors`) load by their
  own paths and have not been moved into the fallback layer.
- The optional second step: when the source title's own disc is also present (2048 when a
  backported element borrows its asset), read the real asset from it before ours.
