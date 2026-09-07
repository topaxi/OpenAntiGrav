//! What [`super::draw::pickup_model_draws`] puts on screen: Pure's
//! `<Mode3D><Model>` dialect for the pickup, against
//! [`super::tests`]'s `pickup_sprites` coverage of Pulse's `<Image>` one.
//!
//! Its own file rather than more of `hud/tests.rs`, which the 200-line-inline
//! / 1,000-line-file rule in `scripts/check-file-size.py` would otherwise
//! push over - the same reason [`super::reticle_tests`] and
//! [`super::zone_tests`] are their own files.

use super::draw::*;
use super::*;

/// Pure's `<Mode3D>` weapon-icon block, cut to the backdrop grid plus two
/// icons - `TURBO_icon`, authored `colour` like nine of Pure's real ten, and
/// `QUAKE_icon` left uncoloured on purpose. No real widget on either PSP disc
/// is like the second one; it exists so the white fallback
/// `pickup_model_draws` takes for a table entry the layout does not actually
/// tint has a test at all.
const PURE_PICKUP_ICONS: &str = r#"
<Screen>
<Mode3D>
<Model name="weapon_icon_grid">
<Values Src="Data\HUD\grid.vex" x="240" y="250.0" z="0" ztest="0"></Values>
</Model>
<Model name="TURBO_icon">
<Values Src="Data\HUD\Weapon_turbo.vex" colour="0xff40ff40" x="240" y="250" z="-10" ztest="0"></Values>
</Model>
<Model name="QUAKE_icon">
<Values Src="Data\HUD\Weapon_quake.vex" x="240" y="250" z="-10" ztest="0"></Values>
</Model>
</Mode3D>
</Screen>
"#;

/// A sheet holding [`PURE_PICKUP_ICONS`]'s three models, with the quad
/// extents `crate::race::hud::vex_model_art` would have computed off their
/// real `.vex` vertices - the grid's own is invented (this fixture has no
/// real `grid.vex` to read), the icons' are the real numbers a race against
/// `pure-psp-usa.chd` reads.
fn pure_pickup_sheet() -> crate::sprite::Sheet {
    crate::sprite::Sheet::placed_at(&[
        (
            r"Data\HUD\grid.vex",
            crate::sprite::Placed {
                x: 0,
                y: 0,
                width: 64,
                height: 64,
                quad_extent: Some([50.0, 50.0]),
                blend: None,
            },
        ),
        (
            r"Data\HUD\Weapon_turbo.vex",
            crate::sprite::Placed {
                x: 64,
                y: 0,
                width: 32,
                height: 16,
                quad_extent: Some([31.423_908, 6.484_658_2]),
                blend: None,
            },
        ),
        (
            r"Data\HUD\Weapon_quake.vex",
            crate::sprite::Placed {
                x: 96,
                y: 0,
                width: 32,
                height: 16,
                quad_extent: Some([56.622_99, 11.677_669]),
                blend: None,
            },
        ),
    ])
}

fn strings() -> crate::language::StringTable {
    crate::language::StringTable::default()
}

fn pure_pickup_context<'a>(
    layout: &'a Layout,
    strings: &'a crate::language::StringTable,
    sheet: &'a crate::sprite::Sheet,
) -> Context<'a> {
    Context {
        default_border: layout.default_border(),
        layout,
        strings,
        sheet,
        art: oag_pure::hud::ART,
        hud_line_height: 25.0,
        small_line_height: 10.0,
    }
}

/// The backdrop grid draws untinted and the held weapon's own icon draws in
/// its authored `colour` - Pure's counterpart to
/// `super::tests::the_pickup_backdrop_takes_its_weapons_own_colour_and_the_icon_keeps_its_authored_one`,
/// a substitution table on Pulse's side against nothing to substitute at all
/// on Pure's: the XML already carries the final colour.
#[test]
fn pure_draws_the_backdrop_grid_and_the_held_weapons_own_coloured_icon() {
    use oag_formats::weapons::Weapon;
    let layout = Layout::from_xml(PURE_PICKUP_ICONS);
    let strings = strings();
    let sheet = pure_pickup_sheet();
    let cx = pure_pickup_context(&layout, &strings, &sheet);

    assert_eq!(
        oag_pure::hud::PICKUP_ICON_MODELS[Weapon::Turbo as usize],
        Some("TURBO_icon")
    );

    let drawn = pickup_model_draws(&cx, Weapon::Turbo);
    assert_eq!(
        drawn.len(),
        2,
        "the grid and the icon, in that paint order: {drawn:?}"
    );

    let Draw::BlendedSprite {
        color: grid_color, ..
    } = drawn[0]
    else {
        panic!("expected a sprite draw: {:?}", drawn[0]);
    };
    assert_eq!(
        grid_color,
        [1.0, 1.0, 1.0, 1.0],
        "the grid authors no colour of its own"
    );

    let Draw::BlendedSprite {
        color: icon_color,
        rect,
        ..
    } = drawn[1]
    else {
        panic!("expected a sprite draw: {:?}", drawn[1]);
    };
    assert_eq!(
        icon_color,
        argb_to_rgba(0xff40_ff40),
        "TURBO_icon's own colour"
    );
    // Centred on the model's authored (240, 250), the quad's own width and
    // height either side.
    assert_eq!(
        rect,
        [
            240.0 - 31.423_908 / 2.0,
            250.0 - 6.484_658_2 / 2.0,
            31.423_908,
            6.484_658_2
        ]
    );
}

/// A table entry the layout does not actually tint falls back to white -
/// untested by any real widget today, since all ten of Pure's really are
/// coloured, but the fallback exists in the code and needs its own coverage.
#[test]
fn an_icon_model_with_no_authored_colour_draws_white() {
    use oag_formats::weapons::Weapon;
    let layout = Layout::from_xml(PURE_PICKUP_ICONS);
    let strings = strings();
    let sheet = pure_pickup_sheet();
    let cx = pure_pickup_context(&layout, &strings, &sheet);

    let drawn = pickup_model_draws(&cx, Weapon::Quake);
    assert_eq!(drawn.len(), 2, "{drawn:?}");
    let Draw::BlendedSprite { color, .. } = drawn[1] else {
        panic!("expected a sprite draw: {:?}", drawn[1]);
    };
    assert_eq!(color, [1.0, 1.0, 1.0, 1.0]);
}

/// A weapon this title's table has no icon for - `Shuriken` on the real
/// `oag_pure::hud::PICKUP_ICON_MODELS`, predating this disc's own roster -
/// draws the backdrop grid and nothing else, rather than nothing at all or a
/// panic. See that table's own doc for why this is a live path on a real
/// race and not a corner nobody reaches.
#[test]
fn a_weapon_with_no_icon_model_still_draws_the_backdrop_and_nothing_else() {
    use oag_formats::weapons::Weapon;
    let layout = Layout::from_xml(PURE_PICKUP_ICONS);
    let strings = strings();
    let sheet = pure_pickup_sheet();
    let cx = pure_pickup_context(&layout, &strings, &sheet);

    assert_eq!(
        oag_pure::hud::PICKUP_ICON_MODELS[Weapon::Shuriken as usize],
        None,
        "this test is asserting the no-icon path, which only fires when this is None"
    );
    let drawn = pickup_model_draws(&cx, Weapon::Shuriken);
    assert_eq!(drawn.len(), 1, "the grid alone: {drawn:?}");
}

/// A title with no `oag_title::HudArt::pickup_icon_models` table at all -
/// every title but Pure, so far - draws nothing through this path, leaving
/// `super::draw::pickup_sprites`'s `<Image>` dialect as the only one that
/// fires.
#[test]
fn a_title_with_no_icon_model_table_draws_nothing_through_this_path() {
    use oag_formats::weapons::Weapon;
    let layout = Layout::default();
    let strings = strings();
    let sheet = crate::sprite::Sheet::default();
    let cx = Context {
        default_border: layout.default_border(),
        layout: &layout,
        strings: &strings,
        sheet: &sheet,
        art: oag_pulse::hud::ART,
        hud_line_height: 25.0,
        small_line_height: 10.0,
    };
    assert!(oag_pulse::hud::ART.pickup_icon_models.is_none());
    assert!(pickup_model_draws(&cx, Weapon::Turbo).is_empty());
}
