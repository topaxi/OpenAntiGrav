# Texture names and their spellings

One rule reads a texture by name on every title: **the name as written, then the
platform's own spelling of it.** It lives in
[`crates/assets/src/texture_name.rs`](../../crates/assets/src/texture_name.rs)
(`spellings`, `Archives::resolve_texture`, `Archives::read_texture`); the four
per-feature retries it replaced (the HUD's `HudArt::texture_extension`, the weapons'
`platform_sibling`, the front end's `.gtf` to `.gxt`/`.gnf` retry, the gantry's
32-name list) are gone.

## The spellings

| Platform | Extension tried after the name as written | Also |
| --- | --- | --- |
| PSP (Pulse, Pure) | none | `.pct` for a `.mip` or `.tga` name (the PS2 build) |
| PS2 (Pulse) | none | `.pct`, same rule (`oag_formats::wad::ps2_texture_name`) |
| PS3 (HD/Fury) | `.gtf` | |
| Vita (2048) | `.gxt` | |
| PS4 (Omega) | `.gnf` | `textures` becomes `texturesps4` in the path, with both extensions |

The stem is kept and the **extension is replaced whatever it was** (an extensionless
name gains one; a dot in a directory name is not an extension; the appended
`name.tga.gtf` spelling HD keeps elsewhere survives). The XML of HD, 2048 and Omega
names the exporter's input (`.mip`, `.tga`, `.gtf`) and the package holds the
conversion: HD's twelve HUD textures resolve twelve of twelve under this rule, 192
of the HUD's 1,029 sprites dangle without it (`hd-hud.md`, confidence 90). The
search is `Archives::locate`'s, so a player's override and the fallback layer answer
for a respelling exactly as they do for the name as written.

`read_texture` returns **the name that was held beside its bytes**. A caller keys its
own tables by the name it asked for (the HUD sheet is keyed by the XML's `src`), and
hands the held name to the decoder.

A name no spelling holds is a `debug` line listing every spelling tried (it used to
be silent). `resolve_texture` is a query and logs nothing, for a caller probing an
optional texture.

## One decoder, and stems

`oag_texture::decode` picks the container from the resolved name
(`Container::of_name`) or, for a blob that arrives without one, from its bytes
(`decode_sniffed`: `.mip`, `.pct`, `.gtf`, `.gxt`, `.gnf`, reporting the `.mip`
reader's complaint when nothing reads it). It returns the base level as RGBA with the
file's own row order (`Picture::bottom_up` is true for a `.gtf`); the HUD sheet flips
those rows, exactly where it was measured. The mesh path keeps its GPU-shaped
builders and picks between them with the same `Container`.

A name **matcher** compares stems (`oag_assets::texture_name::stem`, `has_stem`).
`oag_render::gantry` matches `321backplate` and `billboard1`..`billboard8` that way;
the 32-spelling list is gone.

## Census: HD's names on 2048 and Omega

`crates/raceplay/tests/hd_texture_spelling_ground_truth.rs` resolves 85 names HD's own
tables reference (HUD atlases, campaign and end-of-race art, loading chrome, features
and backdrops, menu blocks, Zone sky and both stage sets, wing-tip ribbon, flare
sprite) on all three, and asserts the miss lists exactly.

| Title | Resolve | Held under no spelling |
| --- | --- | --- |
| HD | 85 of 85 | none |
| 2048 | 72 of 85 | the twelve `Data\FE\Ads\LSAD_*` loading stills and `Hexmedal_HD` |
| Omega | 70 of 85 | the fifteen `/data/tex/zonemode<N>.gtf` (Zone's "general" stage set) |

**Zone's "track" stage set and the sky resolve on both 2048 and Omega**: the textures
are shipped, only `zone_stage_textures` and `zone_sky` are unwired there
(lane `hd-zone-stage`). Omega alone lacks the flat-white "general" set.

Names Omega's own front-end XML references and no spelling holds (read off the
debug lines the resolver now writes, not asserted by a test): the ten
`data\environments2048\<circuit>\FE\TrackSelectEmblem_Fury.gtf` and the three
`Data\FE\NewImages\Demo\` images (`AGRC_logo`, `arrows`, `WOC_box_art`). They draw
nothing, as before; the log used to say nothing.

## Checked against the other title

2048 and Omega share the rule and its tests (`texture_tests.rs` in
`crates/assets/src/source/`): **ported**. Picture checks (Pulse, Pure, HD, 2048,
Omega at ticks 20 and 450, one front-end screen and one menu page each): 20 of 20
PNGs byte-identical before and after. Neither 2048 nor Omega gained a texture in
those flows.

## Not routed through the resolver yet

- Particle sprites (`oag_raceplay::assets`, the `Effect::parse_with` callback) build
  their entry from a per-title extension and directory (`/data/psys/tex/`,
  `Data\...\Tex\`); that is a path rule, not a respelling, and still reads by
  `read_name`.
- Model material textures (`oag_mesh` `textures` callbacks) read the full path the
  `.rcsmodel` names; Omega's already say `.gnf`.
- The front end's per-title `.gtf`-spelled tables in `oag_ui` compare the XML's
  declared name, which is the same on every title.
