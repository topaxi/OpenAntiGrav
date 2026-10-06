//! Parses **the disc's own five HUD layouts** and checks what came out.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(hud_layout_ground_truth)'
//! ```
//!
//! # Why this file is the real test of the layout parser
//!
//! The unit tests in `oag_hud` parse a sample written by hand from what the
//! disc looks like, so they check the parser against this project's *reading* of
//! the format. They cannot catch a reading that is wrong in the same way twice.
//! These do: the input is the shipped bytes, and the assertions are properties
//! the original's own HUD must have.
//!
//! The load-bearing one is [`every_widget_lands_on_screen`]. Every rectangle in
//! these files is authored for a 480x272 screen, so a widget that resolves
//! outside it is a parser bug and nothing else - and the specific bug it catches
//! is dropping an enclosing `<Item>`'s offset, which piles the whole HUD into the
//! top-left corner. That failure looks like a layout problem in a screenshot and
//! is a parsing problem in fact.
//!
//! [`every_widget_lands_on_screen_on_the_ps2`] is its PS2 counterpart, checked
//! against the PS2 release's own 640x448 grid rather than the PSP's - added
//! 2026-09-05. Before it, no PS2 HUD layout had ever been checked for this bug
//! at all, on the strength of a doc comment that read the two grids as "not a
//! flat scaling" and left it there; see `oag_hud::inside_screen`'s doc
//! for why the raw XML's exceptions turn out not to threaten this check once
//! the `<Item>` composition already in [`Layout::from_xml`] is accounted for.
//!
//! # The counts are ground truth, not aspiration
//!
//! [`EXPECTED`]'s widget counts were taken by expanding each file with
//! `oag-wad cat --expand` and counting elements with `grep` - **not** by running
//! this parser and writing down what it said, which would make the test agree
//! with itself no matter what it did. If a future change loses a widget, the count
//! moves and this fails. They are not a claim about how many widgets are *drawn*;
//! most are inactive in any given frame.
//!
//! Two properties of the shipped data that the first version of this file got
//! wrong, both worth keeping written down:
//!
//! - **`HeadToHeadBar` is an `<Image>` with no `Src`.** It carries a `Color` and
//!   `height="0"`, so it is a solid bar whose length is a runtime quantity - the
//!   same colour-only convention `oag_ui::screen` already uses for front-end
//!   backdrops. Read as a sprite it is a widget with no texture, and the parser
//!   dropped it.
//! - **`PlrTag0`-`PlrTag7` carry no authored `x`/`y` at all** (`MPTag_HUD.xml`,
//!   the multiplayer-only sibling layout) - a genuine runtime anchor with
//!   nothing to compose, which is why `hud::is_screen_positioned` exists and
//!   why the on-screen check below skips it. **`PosTag0`-`PosTag7` are not
//!   this** - corrected 2026-09-08, see
//!   [`postag_is_a_fixed_column_not_a_runtime_anchor`] for the measurement
//!   and `docs/ui/hud.md` for where the same correction landed. An earlier
//!   reading took the arcade layout's inner `<Item OffsetX="-40">` alone and
//!   called the result negative; `<Item>` offsets compose, and the outer
//!   `<Item OffsetX="445" OffsetY="5">` around it composes to `(405, 5)`, a
//!   fixed on-screen column stepping twenty pixels a row by index -
//!   `Elimination_HUD.xml`'s own single-level `<Item OffsetX="460">`
//!   corroborates without needing composing to read positive at all. Still
//!   grouped with `PlrTag` in `hud::RUNTIME_ANCHORED` regardless, because
//!   what the eight rows draw is unread and a label's own text width is
//!   unmeasurable without it - see that test for why removing the exemption
//!   is left open rather than done here.

use std::path::PathBuf;

use oag_hud::{self as hud, Layout};
use oag_pulse as pulse;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// The USA PSP pressing - what [`EXPECTED`] and [`layout_of`] measure against.
const PSP: &str = "data/images/pulse-psp-usa.chd";
/// The PS2 pressing, a EU one - the only one this project has. See
/// `crates/game/tests/frontend_grid_ground_truth.rs` for why a EU-to-EU
/// comparison against the EU *PSP* disc has one trap (`Show Logo`'s PRESS
/// START `y`) that does not apply here, since this file never reads `Skin.xml`.
const PS2: &str = "data/images/pulse-ps2-eu.chd";

/// `(entry, sprites, fills, labels, models)` for each shipped layout.
///
/// Counted **independently of this parser**, by expanding each file with
/// `oag-wad cat --expand` and counting elements with `grep`, so the numbers are
/// not this code's own output fed back as its expectation. `sprites` is
/// `<Image>` elements carrying a `Src` and `fills` those without one, which is
/// why the two columns sum to the `<Image>` count rather than either matching it:
///
/// ```text
/// Arcade       Image=35 (34 with Src)  Text=35  Model=11
/// Elimination  Image=33 (33 with Src)  Text=35  Model=11
/// TimeTrial    Image=10 (10 with Src)  Text=25  Model=2
/// Zone         Image=7  (7 with Src)   Text=24  Model=2
/// MPTag        Image=0                 Text=8   Model=0
/// ```
const EXPECTED: &[(&str, usize, usize, usize, usize)] = &[
    (oag_pulse::hud::layouts::ARCADE, 34, 1, 35, 11),
    (oag_pulse::hud::layouts::ELIMINATION, 33, 0, 35, 11),
    (oag_pulse::hud::layouts::TIME_TRIAL, 10, 0, 25, 2),
    (oag_pulse::hud::layouts::ZONE, 7, 0, 24, 2),
    (oag_pulse::hud::layouts::MP_TAG, 0, 0, 8, 0),
];

fn open() -> Option<oag_assets::Archives> {
    let image = image(PSP)?;
    pulse::open(&image.display().to_string()).ok()
}

/// The PS2 pressing, opened the same way. A separate function rather than a
/// parameter on [`open`] because every other test in this file is PSP-only and
/// should stay that way rather than grow a platform argument it never uses.
fn open_ps2() -> Option<oag_assets::Archives> {
    let image = image(PS2)?;
    pulse::open(&image.display().to_string()).ok()
}

fn layout_of(archives: &mut oag_assets::Archives, entry: &str) -> Layout {
    let blob = archives
        .read_name(entry)
        .unwrap_or_else(|e| panic!("reading {entry}: {e}"));
    let xml = oag_tables::fexml::expand(&blob).unwrap_or_else(|e| panic!("expanding {entry}: {e}"));
    Layout::from_xml(&xml)
}

/// The PS2 disc's own `Data\XML\*_HUD.xml` are plain `<?xml`, unlike the
/// PSP's shortened dictionary form - see `docs/ui/hud.md`'s "PS2 ships the
/// same HUD" section - so this reads either without assuming which.
fn layout_of_either(archives: &mut oag_assets::Archives, entry: &str) -> Layout {
    let blob = archives
        .read_name(entry)
        .unwrap_or_else(|e| panic!("reading {entry}: {e}"));
    let xml = if oag_tables::fexml::is_fexml(&blob) {
        oag_tables::fexml::expand(&blob).unwrap_or_else(|e| panic!("expanding {entry}: {e}"))
    } else {
        String::from_utf8(blob).unwrap_or_else(|e| panic!("{entry} is not UTF-8 text: {e}"))
    };
    Layout::from_xml(&xml)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn all_five_layouts_parse_with_the_widget_counts_the_disc_has() {
    let Some(mut archives) = open() else {
        return;
    };

    for &(entry, sprites, fills, labels, models) in EXPECTED {
        let layout = layout_of(&mut archives, entry);
        println!(
            "{entry}: {} sprite(s), {} fill(s), {} label(s), {} model(s)",
            layout.sprites.len(),
            layout.fills.len(),
            layout.labels.len(),
            layout.models.len()
        );
        assert!(
            layout.skipped.is_empty(),
            "{entry} skipped {} widget(s): {:?}",
            layout.skipped.len(),
            layout.skipped
        );
        assert_eq!(layout.sprites.len(), sprites, "{entry} sprite count");
        assert_eq!(layout.fills.len(), fills, "{entry} fill count");
        assert_eq!(layout.labels.len(), labels, "{entry} label count");
        assert_eq!(layout.models.len(), models, "{entry} model count");
    }
}

/// `PosTag0`-`PosTag7` are a fixed column, not a runtime anchor - corrected
/// 2026-09-08, against this file's own doc comment and against
/// `docs/ui/hud.md`.
///
/// **The doc comment above and `docs/ui/hud.md` both say these "resolve to
/// negative coordinates" because the arcade layout nests them in
/// `<Item OffsetX="-40">`.** That reading takes the `-40` alone; `<Item>`
/// offsets compose (`Layout::collect`'s `x = offset_x + OffsetX`, landed and
/// checked against nine assertions - none of which touched `PosTag`), and
/// the enclosing `<Item OffsetX="445" OffsetY="5">` composes with it to
/// `(405, 5)`. Measured here rather than argued: `PosTag0` through
/// `PosTag7` land at `x=405`, `y=25, 45, ..., 165` - one fixed column,
/// twenty pixels a row, both **on screen** and **evenly spaced by index**,
/// which a set of eight independent per-craft anchors would have no reason
/// to be. `Elimination_HUD.xml` corroborates independently, at its own
/// `x=460` (single-level `<Item OffsetX="460" OffsetY="5">`, so this one
/// needs no composing to read positive) and the same twenty-pixel step -
/// two files, two `<Item>` shapes, the same column.
///
/// **`PlrTag0`-`PlrTag7` (`MPTag_HUD.xml`, the multiplayer-only sibling
/// layout) are the opposite: genuinely unpositioned.** Their `<Values>`
/// carries no `x`/`y` at all, so [`Layout::from_xml`] leaves both at their
/// `f32` default, `0.0` - the real "nothing to read a position from" case
/// `hud::RUNTIME_ANCHORED`'s doc describes, unlike `PosTag`.
///
/// **What is not settled, and is not this test's job to settle**: what text
/// each of the eight rows carries. Neither carries an `idstring` or a
/// `string`, so the content is runtime-supplied and unread - filling it
/// would be inventing what the asset does not author, which `CLAUDE.md`
/// forbids. `hud::RUNTIME_ANCHORED`/`hud::is_screen_positioned` still skip
/// `PosTag` in [`every_widget_lands_on_screen`] below, deliberately left
/// alone in this change: the *anchor* now passes that check on its own
/// merits (`405`/`460`, `25` to `165`, all inside `480x272`), but a label's
/// own text width is unmeasurable without knowing what fills it, and
/// `align="left"` at `x=405` on a 480-wide screen leaves only 75 px before
/// the right edge - narrower than `TotalTime`'s own measured 92 px at
/// `HUD` scale 1.0 (see this file's "TotalTime overflows the right edge"
/// open item in `docs/ui/hud.md`). Untangling that needs the content first.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn postag_is_a_fixed_column_not_a_runtime_anchor() {
    let Some(mut archives) = open() else {
        return;
    };

    let arcade = layout_of(&mut archives, oag_pulse::hud::layouts::ARCADE);
    for row in 0..8u32 {
        let name = format!("PosTag{row}");
        let label = arcade
            .label(&name)
            .unwrap_or_else(|| panic!("{name} missing from {}", oag_pulse::hud::layouts::ARCADE));
        assert!(
            (label.x - 405.0).abs() < f32::EPSILON,
            "{name}: x={}, expected 405 (the composed column, not -40 read alone)",
            label.x
        );
        assert!(
            (label.y - (25.0 + row as f32 * 20.0)).abs() < f32::EPSILON,
            "{name}: y={}",
            label.y
        );
    }

    let elimination = layout_of(&mut archives, oag_pulse::hud::layouts::ELIMINATION);
    for row in 0..8u32 {
        let name = format!("PosTag{row}");
        let label = elimination.label(&name).unwrap_or_else(|| {
            panic!(
                "{name} missing from {}",
                oag_pulse::hud::layouts::ELIMINATION
            )
        });
        assert!(
            (label.x - 460.0).abs() < f32::EPSILON,
            "{name}: x={}, expected 460",
            label.x
        );
        assert!(
            (label.y - (25.0 + row as f32 * 20.0)).abs() < f32::EPSILON,
            "{name}: y={}",
            label.y
        );
    }

    // The multiplayer sibling, for contrast: no authored position at all,
    // which is the genuine runtime-anchor case `PosTag` was believed to be.
    let mp_tag = layout_of(&mut archives, oag_pulse::hud::layouts::MP_TAG);
    let plr_tag0 = mp_tag
        .label("PlrTag0")
        .expect("PlrTag0 missing from MPTag_HUD.xml");
    assert_eq!(
        (plr_tag0.x, plr_tag0.y),
        (0.0, 0.0),
        "PlrTag0 carries no x/y, unlike PosTag - this is what an unauthored anchor looks like"
    );
}

/// Every authored rectangle must land inside the PSP's screen.
///
/// This is the test that catches a dropped `<Item>` offset. A widget authored at
/// `x="6"` inside `OffsetX="300"` belongs at 306; read as absolute it sits at 6,
/// which is still *on* screen - so the check that actually bites is the opposite
/// direction, a widget whose offset pushes it off the right or bottom edge if the
/// offset is applied twice, plus the centre-of-mass check below.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_widget_lands_on_screen() {
    let Some(mut archives) = open() else {
        return;
    };

    let psp = oag_display::space::Space::PSP.size;
    let mut checked = 0usize;
    for &(entry, ..) in EXPECTED {
        let layout = layout_of(&mut archives, entry);

        for sprite in &layout.sprites {
            if !hud::is_screen_positioned(&sprite.name) {
                continue;
            }
            assert!(
                hud::inside_screen(sprite.rect, psp),
                "{entry}: {} resolved to {:?}, outside 480x272",
                sprite.name,
                sprite.rect
            );
            checked += 1;
        }
        // A label's rectangle depends on its text, which is not known here, so
        // only the anchor is checkable.
        for label in &layout.labels {
            if !hud::is_screen_positioned(&label.name) {
                continue;
            }
            assert!(
                label.x >= 0.0 && label.x <= psp.0 && label.y >= 0.0 && label.y <= psp.1,
                "{entry}: {} anchors at ({}, {}), off screen",
                label.name,
                label.x,
                label.y
            );
            checked += 1;
        }
    }

    // A skip list plus an assertion is a way to accidentally test nothing, so the
    // count of what was actually checked is asserted too.
    println!("{checked} screen-positioned widget(s) checked");
    assert!(
        checked > 150,
        "only {checked} widgets were checked; the exclusion list has swallowed the test"
    );
}

/// The PS2 counterpart of [`every_widget_lands_on_screen`], checked against its
/// own 640x448 grid rather than the PSP's 480x272.
///
/// **Why this is safe despite the raw XML not being a flat scaling.** 22 of
/// `Arcade_HUD.xml`'s 141 raw `x`/`y` attributes do not scale by 640/480 or
/// 448/272 - 18 are `<Mode3D><Model>` placements this test never reaches
/// (`Layout::models` is not [`Sprite`](hud::Sprite) or
/// [`Label`](hud::Label)), and 4 are small integer nudges on a handful of
/// widgets that stay byte-identical across consoles while the `<Item>`
/// enclosing every one of them scales its own `OffsetX`/`OffsetY` correctly.
/// [`Layout::from_xml`] composes that offset into the widget's `rect`/`x,y`
/// before this test ever sees it, so the nudge is a pixel or two of slop on a
/// widget the offset already moved most of the way, not a widget in the wrong
/// space. See `oag_hud::inside_screen`'s doc comment for the coordinate
/// tally this rests on. If some future layout widget's own unscaled nudge ever
/// grows large enough to matter, this test is exactly what would catch it -
/// nothing here assumes composed rects stay on screen without checking.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_widget_lands_on_screen_on_the_ps2() {
    let Some(mut archives) = open_ps2() else {
        return;
    };

    let ps2 = oag_display::space::Space::PS2.size;
    let mut checked = 0usize;
    for &(entry, ..) in EXPECTED {
        let layout = layout_of_either(&mut archives, entry);

        for sprite in &layout.sprites {
            if !hud::is_screen_positioned(&sprite.name) {
                continue;
            }
            assert!(
                hud::inside_screen(sprite.rect, ps2),
                "{entry}: {} resolved to {:?}, outside 640x448",
                sprite.name,
                sprite.rect
            );
            checked += 1;
        }
        for label in &layout.labels {
            if !hud::is_screen_positioned(&label.name) {
                continue;
            }
            assert!(
                label.x >= 0.0 && label.x <= ps2.0 && label.y >= 0.0 && label.y <= ps2.1,
                "{entry}: {} anchors at ({}, {}), off the PS2's 640x448",
                label.name,
                label.x,
                label.y
            );
            checked += 1;
        }
    }

    println!("{checked} screen-positioned widget(s) checked on the PS2 layout");
    assert!(
        checked > 150,
        "only {checked} widgets were checked; the exclusion list has swallowed the test"
    );
}

/// The speed and shield bars must be in the bottom-right, where Pulse draws them.
///
/// An independent check on the `<Item>` offset, and a much sharper one than "on
/// screen": these four widgets live inside `OffsetX="300" OffsetY="210"`, so
/// losing the offset moves them to the top-left and this fails. It asserts a
/// fact about the original's HUD rather than about the file.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_speed_and_shield_bars_sit_in_the_bottom_right() {
    let Some(mut archives) = open() else {
        return;
    };

    let layout = layout_of(&mut archives, oag_pulse::hud::layouts::TIME_TRIAL);
    for name in ["SpeedBarBg", "SpeedBar", "ShieldBarBg", "ShieldBar"] {
        let sprite = layout
            .sprite(name)
            .unwrap_or_else(|| panic!("{name} is missing from the time-trial layout"));
        let [x, y, w, h] = sprite.rect;
        assert!(
            x + w / 2.0 > 240.0,
            "{name} centres at x={}, which is not the right half",
            x + w / 2.0
        );
        assert!(
            y + h / 2.0 > 136.0,
            "{name} centres at y={}, which is not the bottom half",
            y + h / 2.0
        );
    }
}

/// A bar and its background share a rectangle, differing only in colour.
///
/// This is the evidence for how the fill is drawn: `SpeedBar` sits exactly on
/// `SpeedBarBg` with the same source rectangle, so the fill can only be a
/// horizontal crop of the same art rather than a second sprite. The draw code
/// depends on that, so it is pinned here rather than left as a reading.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn each_bar_exactly_overlays_its_own_background() {
    let Some(mut archives) = open() else {
        return;
    };

    let layout = layout_of(&mut archives, oag_pulse::hud::layouts::TIME_TRIAL);
    for (fill, background) in [("SpeedBar", "SpeedBarBg"), ("ShieldBar", "ShieldBarBg")] {
        let fill = layout.sprite(fill).unwrap_or_else(|| panic!("{fill}"));
        let background = layout
            .sprite(background)
            .unwrap_or_else(|| panic!("{background}"));
        assert_eq!(fill.rect, background.rect, "{} rect", fill.name);
        assert_eq!(fill.uv, background.uv, "{} uv", fill.name);
        assert_ne!(
            fill.color, background.color,
            "{} and its background are the same colour, so the fill would be invisible",
            fill.name
        );
    }
}

/// Every sprite samples the one atlas, and every colour resolves.
///
/// A `FEConst->` that does not resolve falls back to white, which is visible but
/// wrong; catching it here is cheaper than noticing a white bar in a screenshot.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_sprite_samples_the_hud_atlas_and_every_constant_resolves() {
    let Some(mut archives) = open() else {
        return;
    };

    for &(entry, ..) in EXPECTED {
        let layout = layout_of(&mut archives, entry);

        for sprite in &layout.sprites {
            assert_eq!(
                sprite.src,
                oag_pulse::hud::ATLAS,
                "{entry}: {} samples {} rather than the HUD atlas",
                sprite.name,
                sprite.src
            );
            assert!(
                sprite.uv[2] > 0.0 && sprite.uv[3] > 0.0,
                "{entry}: {} has an empty source rectangle {:?}",
                sprite.name,
                sprite.uv
            );
        }

        // The constants themselves must parse as ARGB, or every colour that
        // names one silently goes white.
        for (name, value) in &layout.constants {
            assert!(
                oag_ui::screen::parse_argb(value).is_some(),
                "{entry}: constant {name} = {value:?} is not an ARGB literal"
            );
        }
    }
}

/// No two widgets the HUD draws may share an anchor, on any shipped layout.
///
/// `oag_hud::Frame` splits text into two font passes, which gives up
/// document paint order *between* fonts, so two live labels at one anchor would
/// leave which of them is visible to the order the renderer happens to bind the
/// atlases in. The unit test of the same name checks a hand-written sample; this
/// checks the disc.
///
/// **It has already caught one.** `Arcade_HUD.xml` authors `TotalTime` and
/// `Position` at exactly `(445, 35)`, right-aligned, same font and scale - see
/// `oag_hud::place_owns_the_anchor`, which is the rule that keeps at most
/// one of them live and is why this passes rather than a coincidence that it does.
///
/// The readout is deliberately the *busiest* one a race can produce: a place in a
/// field, a lap of a lap count, a best lap set, a pickup held and the wrong-way
/// warning up. A quiet readout would pass this test by drawing almost nothing.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn no_two_live_widgets_share_an_anchor_on_any_shipped_layout() {
    let Some(mut archives) = open() else {
        return;
    };

    let readout = hud::Readout {
        speed_kmh: 300.0,
        shield: 50.0,
        shield_max: 100.0,
        lap: 2,
        laps: 3,
        place: 3,
        ships: 8,
        race_ticks: 3600,
        lap_ticks: 1200,
        best_lap_ticks: Some(3000),
        wrong_way: true,
        zone: 4,
        score: 1234,
        pickup: Some(oag_tables::weapons::Weapon::Rocket),
        ..hud::Readout::blank()
    };
    let strings = oag_ui::language::StringTable::default();

    let mut checked = 0usize;
    for &(entry, ..) in EXPECTED {
        let layout = layout_of(&mut archives, entry);
        let frame = hud::draw_list(
            &hud::Context {
                default_border: layout.default_border(),
                layout: &layout,
                strings: &strings,
                // Empty: this test reads only the label passes, and no sprite
                // reaches a draw without a texture to sample.
                sheet: &oag_hud::sprite::Sheet::default(),
                art: oag_pulse::hud::ART,
                // The real line heights of `PulseHud.fnt` and `small.fnt`. They
                // only move the anchors vertically and together, so the exact
                // values do not decide this test - but a wrong one could hide a
                // collision, so they are the disc's.
                hud_line_height: 25.0,
                small_line_height: 10.0,
                default_line_height: 10.0,
            },
            &readout,
        );

        let mut anchors: Vec<(String, f32, f32)> = Vec::new();
        for draw in frame.hud_text.iter().chain(frame.small_text.iter()) {
            let oag_ui::frontend::Draw::Text { x, y, text, .. } = draw else {
                continue;
            };
            if let Some((other, ..)) = anchors
                .iter()
                .find(|(_, at_x, at_y)| *at_x == *x && *at_y == *y)
            {
                panic!(
                    "{entry}: {text:?} and {other:?} are both live at ({x}, {y}), so paint \
                     order between the two font passes decides which is visible"
                );
            }
            anchors.push((text.clone(), *x, *y));
            checked += 1;
        }
    }

    println!(
        "{checked} live label(s) checked across {} layout(s)",
        EXPECTED.len()
    );
    // A busy readout across five layouts draws dozens of labels. A handful would
    // mean the readout stopped driving anything and the test proves nothing.
    assert!(
        checked > 30,
        "only {checked} live label(s) were drawn at all; this test has stopped checking anything"
    );
}

/// The atlas the layouts name must actually be on the disc, in both archives.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_atlas_and_both_hud_fonts_are_readable() {
    let Some(mut archives) = open() else {
        return;
    };

    let atlas = archives
        .read_name(oag_pulse::hud::ATLAS)
        .unwrap_or_else(|e| panic!("reading {}: {e}", oag_pulse::hud::ATLAS));
    let texture = oag_texture::texture::Texture::parse(&atlas)
        .unwrap_or_else(|e| panic!("decoding {}: {e}", oag_pulse::hud::ATLAS));
    println!(
        "{}: {}x{}, {} bpp, {} palette entry(s), {} index(es)",
        oag_pulse::hud::ATLAS,
        texture.width,
        texture.height,
        texture.bits_per_pixel,
        texture.palette.len(),
        texture.indices.len()
    );
    assert!(texture.width > 0 && texture.height > 0);
    // The decoder guarantees this, and the HUD's UVs are in these coordinates,
    // so a mismatch would silently sample the wrong pixels.
    assert_eq!(
        texture.indices.len(),
        usize::from(texture.width) * usize::from(texture.height),
        "the atlas decoded to a buffer that is not its own size"
    );

    for name in [pulse::names::fonts::HUD, pulse::names::fonts::SMALL] {
        let blob = archives
            .read_name(name)
            .unwrap_or_else(|e| panic!("reading {name}: {e}"));
        let font =
            oag_texture::fnt::Font::parse(&blob).unwrap_or_else(|e| panic!("decoding {name}: {e}"));
        println!(
            "{name}: {}x{} atlas, {} glyph(s), line height {}",
            font.width,
            font.height,
            font.glyphs.len(),
            font.line_height
        );
        assert!(!font.glyphs.is_empty());
    }
}

/// **The pickup icon is found by name, and the disc is what says so.**
///
/// `docs/ui/hud.md` recorded these widgets as "14 `*Icon` widgets" whose "icon
/// ids are numeric", with no id-to-weapon mapping known - which made drawing the
/// right icon look like it needed an unrecovered table. It does not:
/// `Arcade_HUD.xml` authors **thirteen** and names each one after its weapon's
/// own `type` string, so `oag_hud::pickup_icon_name` is the whole lookup.
///
/// # The layouts also say which modes hand out what, and they agree with the manual
///
/// This is the half worth reading twice, because it was found by this test
/// failing against an assumption rather than by anybody looking for it. The four
/// layouts author three different pickup sets:
///
/// | Layout | Pickup widgets |
/// | --- | --- |
/// | `Arcade_HUD.xml` | the backdrop and **all thirteen** icons |
/// | `Elimination_HUD.xml` | the same |
/// | `TimeTrial_HUD.xml` | the backdrop and **`TurboIcon` alone** |
/// | `Zone_HUD.xml` | **none** |
///
/// A time trial races with `g_weapons_enabled == 0` and its `Weapon Pad`s
/// hidden, so a lone Turbo icon in its layout would be inexplicable - except
/// that the disc's own event text says exactly this: `MSC_EVENT_TT` and `MSC_EVENT_SL`
/// each promise *"You will be given a free turbo pickup once per lap"*. **A
/// string table and a HUD layout agreeing is two independent records of one
/// rule**, which is the same shape of evidence that settled the weapons-off
/// energy recovery in `oag_gameplay::damage_rules`. See
/// `docs/gameplay/pickups.md`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_weapon_has_an_icon_widget_named_after_it() {
    use oag_tables::weapons::Weapon;

    let Some(mut archives) = open() else {
        return;
    };

    // The two weapons-bearing layouts author every weapon, which is what makes
    // the name rule worth having: it has to resolve for all thirteen and not
    // only for the one this engine implements. **Pulse's thirteen**: the
    // fourteenth `Weapon`, the Disruptor, is Pure's alone and Pulse's layouts
    // author no `DisruptorIcon` - see `Weapon`'s own doc on why it is
    // appended rather than absent.
    for entry in [
        oag_pulse::hud::layouts::ARCADE,
        oag_pulse::hud::layouts::ELIMINATION,
    ] {
        let layout = layout_of(&mut archives, entry);
        assert!(
            layout.sprite("PickupBackground").is_some(),
            "{entry}: no PickupBackground to draw an icon on"
        );
        let missing: Vec<String> = Weapon::ALL
            .into_iter()
            .filter(|weapon| *weapon != Weapon::Disruptor)
            .map(hud::pickup_icon_name)
            .filter(|name| layout.sprite(name).is_none())
            .collect();
        assert!(
            missing.is_empty(),
            "{entry} authors no widget for {missing:?}, so the name rule does not hold"
        );
        println!(
            "{entry}: all {} Pulse weapon icons resolve by name",
            Weapon::ALL.len() - 1
        );

        // **The two premises the backdrop substitution rests on**, asserted
        // against the shipped file rather than against the hand-written sample:
        // the disc authors the backdrop and the icon in one colour, which is
        // what makes the pair unreadable as authored, and it defines the
        // background colour that stands in for it. If a layout ever stopped
        // defining `HudBGColour`, the substitution would silently not happen
        // and the icon would go back to being invisible - see
        // `oag_hud::PICKUP_BACKDROP_COLOUR`.
        let backdrop = layout.sprite("PickupBackground").expect("PickupBackground");
        let turbo = layout
            .sprite(&hud::pickup_icon_name(Weapon::Turbo))
            .expect("TurboIcon");
        assert_eq!(
            backdrop.color, turbo.color,
            "{entry}: the backdrop and the icon are authored in different \
             colours, so the substitution may no longer be needed"
        );
        assert!(
            layout.constants.contains_key("HudBGColour"),
            "{entry} defines no HudBGColour, so the pickup backdrop would draw \
             in the authored opaque white and hide the icon"
        );
    }

    // The time trial's single icon, and it must stay single: a second one
    // appearing here would mean the free-turbo reading is too narrow.
    let time_trial = layout_of(&mut archives, oag_pulse::hud::layouts::TIME_TRIAL);
    let present: Vec<Weapon> = Weapon::ALL
        .into_iter()
        .filter(|&weapon| time_trial.sprite(&hud::pickup_icon_name(weapon)).is_some())
        .collect();
    assert_eq!(
        present,
        vec![Weapon::Turbo],
        "{} authors {present:?}; the disc's own event text says a time trial gets \
         a free turbo and nothing else",
        oag_pulse::hud::layouts::TIME_TRIAL
    );
    assert!(
        time_trial.sprite("PickupBackground").is_some(),
        "an icon with no backdrop to sit on"
    );

    // Zone authors nothing, which is the one mode where "weapons off" really is
    // the whole story.
    let zone = layout_of(&mut archives, oag_pulse::hud::layouts::ZONE);
    let zone_icons: Vec<Weapon> = Weapon::ALL
        .into_iter()
        .filter(|&weapon| zone.sprite(&hud::pickup_icon_name(weapon)).is_some())
        .collect();
    assert!(
        zone_icons.is_empty() && zone.sprite("PickupBackground").is_none(),
        "{} authors pickup widgets {zone_icons:?}",
        oag_pulse::hud::layouts::ZONE
    );
}

/// Every shipped layout names **at most one** texture, on either title.
///
/// The assumption `oag_hud::Layout::atlas` rests on. It returns one name
/// because nine of nine layouts across the two discs carry one; a layout that
/// broke the rule would have its later sprites drawn from the wrong sheet, which
/// looks like a UV bug rather than a missing texture. Failing here instead names
/// the file and the second name.
///
/// Pure is walked as well as Pulse, and by its own layout list rather than
/// [`EXPECTED`] - Pure ships four of Pulse's five (no `Elimination_HUD.xml`,
/// which is a mode it does not have) and **names no texture in any of them**,
/// its HUD being `<Model>` geometry off `Data\HUD\*.vex`. A zero is as much a
/// pass here as a one.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_layouts_name_at_most_one_texture() {
    for (label, path) in [
        ("pulse", "data/images/pulse-psp-eu.chd"),
        ("pure", "data/images/pure-psp-eu.chd"),
    ] {
        let full = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        if !full.exists() {
            println!("skipping: {} not present", full.display());
            continue;
        }
        let mut archives =
            oag_source::title::open_source(&full.display().to_string(), Vec::new(), Vec::new())
                .expect("opening the source")
                .archives;

        for &(entry, ..) in EXPECTED {
            let Ok(blob) = archives.read_name(entry) else {
                println!("{label}: {entry} is not on this disc, skipped");
                continue;
            };
            let Ok(xml) = oag_tables::fexml::text(&blob) else {
                continue;
            };
            let layout = hud::Layout::from_xml(&xml);
            let mut names: Vec<&str> = layout
                .sprites
                .iter()
                .map(|sprite| sprite.src.as_str())
                .filter(|src| !src.is_empty())
                .collect();
            names.sort_unstable();
            names.dedup();
            assert!(
                names.len() <= 1,
                "{label}: {entry} names {} distinct textures ({names:?}); \
                 `Layout::atlas` returns one and the rest would draw from the \
                 wrong sheet",
                names.len()
            );
            assert_eq!(
                layout.atlas(),
                names.first().copied(),
                "{label}: {entry}: `Layout::atlas` should return the one it names"
            );
        }
    }
}
