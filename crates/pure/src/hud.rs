//! Pure's in-race HUD layout entries.
//!
//! Entry names in the [ADR-0022] sense, the same division [`oag_pulse::hud`]
//! draws: *which* files this disc ships. Everything that reads them is
//! `oag_hud`.
//!
//! # Three names, all Pulse's, and that is a measurement
//!
//! Read directly off `pure-psp-eu.chd`'s `Data.wad` on 2026-08-18, one
//! `oag-wad cat` per name: `Data\XML\TimeTrial_HUD.xml`,
//! `Data\XML\Zone_HUD.xml` and `Data\XML\Arcade_HUD.xml` all return a layout,
//! spelled exactly as Pulse spells them. `Data\XML\SpeedLap_HUD.xml` returns
//! `no entry named ... (hash 1af0a646)`, the same answer Pulse's disc gives, so
//! speed lap draws the time trial's layout here too.
//!
//! **Written out rather than re-exported from `oag-pulse`.** A title package
//! states what its own disc ships; a table pointing at another title's constant
//! would say "the same as Pulse", which is a claim about Pulse rather than a
//! measurement of Pure, and it is precisely the shape that let every title be
//! served Pulse's HUD until finding S2 of the 2026-08-18 review. `oag_pure::race`
//! makes the same choice for `DEFAULT_TEAM`, with a test asserting the sharing
//! is deliberate.
//!
//! # No atlas, and no constant recording the absence
//!
//! Pure's HUD layouts name no texture at all - `oag_raceplay::hud` reports it
//! per race and draws their sprites from `<Model>` geometry. So there is no
//! `ATLAS` here to match `oag_pulse::hud::ATLAS`, for the reason
//! [`crate::race`]'s docs give at length: an absent entry is a fact, and a
//! constant naming a file this disc does not carry would be an invention.
//!
//! # The weapon icons are `<Model>`s, not `<Image>`s, and colour-coded as such
//!
//! **Corrects a claim this file made until 2026-09-04**: `pickup_colours`
//! used to read `None` on the grounds that "this disc has no per-weapon icon
//! widget of any kind to colour". That was checked by grepping `<Image
//! name=` alone, and it was wrong, not merely incomplete - `Arcade_HUD.xml`'s
//! and `TimeTrial_HUD.xml`'s own `<Mode3D>` block authors ten weapon icons as
//! `<Model>` widgets, each with an authored `colour="0xAARRGGBB"` beside its
//! `.vex`, e.g. `<Model name="TURBO_icon"><Values
//! Src="Data\HUD\Weapon_turbo.vex" colour="0xff40ff40" .../></Model>`. Byte-
//! identical between the two files, so this is the played mode's data, not a
//! Time-Trial-only quirk. Confidence **95**: read directly off
//! `pure-psp-usa.chd`'s `Data.wad`, both layouts, no inference.
//!
//! Ten names, one colour each: `ROCKET_icon`/`MISSILE_icon`/`DISRUPTOR_icon`
//! (`0xff40acff`), `QUAKE_icon`/`PLASMA_icon` (`0xff0000ff`),
//! `TURBO_icon`/`SHIELD_icon`/`AUTOPILOT_icon` (`0xff40ff40`) and
//! `MINE_icon`/`BOMB_icon` (`0xffffc040`) - a four-colour scheme, not
//! thirteen distinct ones, the same shape Pulse's own measured table has.
//!
//! **`DISRUPTOR_icon` names a weapon `oag_tables::weapons::Weapon` has no
//! variant for** (prose, not a link - this crate does not depend on
//! `oag-formats` outside tests, for the same "tables only" reason
//! [ADR-0022] gives). Pure's own executable strings carry `WO_DISRUPTOR_EXPLO`
//! alongside `WO_MISSILE_EXPLO` and the rest
//! (`docs/formats/pure-status.md`'s rocket/collision-fx section), so this is
//! a weapon this disc's roster genuinely has and Pulse's does not - not a
//! second spelling of `Cannon`, which has no icon here at all. Pure's ten
//! also omit `LeachBeam`, `Repulser` and `Shuriken`, three of Pulse's
//! thirteen. `Weapon`'s variant list is scoped to what Pulse's own
//! `WeaponStats_*.xml` ships (`crates/formats/src/weapons.rs`'s module doc),
//! so a title with a different roster needing its own vocabulary is expected,
//! not a bug to reconcile by guessing a mapping.
//!
//! **Why `ART.pickup_colours` still reads `None` below, correctly this
//! time**: that field exists to *override* a single backdrop sprite's colour
//! at runtime, which is what Pulse's `PickupBackground` needs since one
//! `<Image>` stands in for thirteen weapons. Pure needs no such table: each
//! of its ten `<Model>`s already carries its own final colour in the XML,
//! read by `oag_hud::Model::colour` with no substitution step at all
//! (this crate does not depend on `oag-game`, so that is prose, not a link).
//! Drawing them - selecting the held weapon's model by name and painting it
//! tinted - shipped the same day: [`PICKUP_ICON_MODELS`] and
//! `ART.pickup_icon_backdrop_model` name the widgets,
//! `oag_raceplay::hud::vex_model_art` reads each `.vex` mesh's own quad
//! size off its vertices rather than a hand-measured constant, and
//! `oag_hud::draw::pickup_model_draws` draws them. See
//! `docs/gameplay/pickups.md`'s Pure section for the evidence, including a
//! real capture of a held Turbo.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
//! [`oag_pulse::hud`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/pulse/src/hud.rs

/// The layout each mode this engine runs reads its HUD from, as
/// [`oag_title::Title::hud`] carries it.
pub const LAYOUTS: &oag_title::HudLayouts = &oag_title::HudLayouts {
    arcade: layouts::ARCADE,
    time_trial: layouts::TIME_TRIAL,
    speed_lap: layouts::TIME_TRIAL,
    zone: layouts::ZONE,
    // Pure's own Eliminator layout has not been checked for; `arcade` is the
    // one other title fact this crate already reuses this way (`speed_lap`
    // above). See `oag_title::HudLayouts::elimination`.
    elimination: layouts::ARCADE,
};

/// How Pure's HUD sprites reach the screen, as [`oag_title::Title::hud_art`]
/// carries it.
///
/// **There is nothing `<Image>`-shaped for it to reach, and that is still true -
/// there is a `<Model>`-shaped one instead.** Composed off `pure-psp-eu.chd` on
/// 2026-08-25, Pure's three layouts hold **37, 32 and 18 `<Text>` widgets and
/// not one `<Image>`** - no sprites and no fills at all, which is the same
/// finding the "no atlas" section above states from the other end. So
/// [`ALWAYS_ON`] is empty rather than Pulse's seven names copied over: there is
/// no *sprite* of that name here to be always on. What the `<Image>` count does
/// not show is the ten weapon-icon `<Model>`s the "weapon icons are `<Model>`s"
/// section above measures separately - a different widget kind with its own
/// colour, not covered by this field's `Image`-shaped substitution mechanism.
pub const ART: &oag_title::HudArt = &oag_title::HudArt {
    // Moot on a disc whose layouts name no texture, and `None` on the same
    // terms as Pulse's: this is a PSP disc, and its own XML asks for `.mip`.
    texture_extension: None,
    always_on: ALWAYS_ON,
    raster: true,
    // Four instances of one corner-bracket model plus the closed box at the
    // middle, bound by `HudSight_Bind` (`0x0881b604`) in this order - which is
    // also the order `sight::BRACKET_ROTATIONS` is indexed by. See
    // `docs/ghidra/functions/psp-pulse-usa/lock-sight.md`.
    sights: &oag_title::hud::Sights::Brackets {
        brackets: [
            "missile_sight_1",
            "missile_sight_2",
            "missile_sight_3",
            "missile_sight_4",
        ],
        inner: "missile_sight_inner",
        // **`None`, measured.** `Data\XML\Arcade_HUD.xml` on `pure-psp-usa.chd`
        // authors these five sight widgets and no `leachbeam_sight_*`, which
        // agrees with `Data\XML\weaponstats.xml` authoring no `<Weapon
        // type="LeachBeam">`: the disc agrees that is a Pulse weapon. So a
        // LeachBeam here would draw nothing rather than borrow the Missile's
        // brackets - and nothing hands one out in the first place.
        leach: None,
    },
    // Moot for the same reason - this disc authors no `PickupBackground` - and
    // recorded as Pulse's answer rather than as `None`, because `None` here
    // would read as the measured "drawn as authored" that HD carries.
    pickup_backdrop_colour: Some("HudBGColour"),
    // `None`, correctly rather than merely unmeasured - see the module doc's
    // "weapon icons are `<Model>`s" section. This disc's ten weapon icons are
    // `<Model>` widgets, each with its own authored `colour`, not one
    // `<Image>` backdrop needing a runtime substitute the way Pulse's is. This
    // field exists to override a single sprite's colour; Pure's icons need no
    // override, so `None` is the disc's own answer, not a placeholder for one.
    pickup_colours: None,
    pickup_icon_models: Some(PICKUP_ICON_MODELS),
    // `weapon_icon_grid`, authored beside the ten icons at the same
    // placeholder position (`Data\HUD\grid.vex`, `x="240" y="250.0" z="0"` -
    // the icons themselves sit at `z="-10"`, in front of it) and with no
    // `colour` of its own.
    pickup_icon_backdrop_model: Some("weapon_icon_grid"),
    // Pure's icons are `<Mode3D><Model>`s (the row above), not a single
    // rewritten-UV `<Image>` widget - 2048's own dialect.
    pickup_icon_uv: None,
    // `None`: no Zone speed-class ladder has been read on this title.
    zone_speed_classes: None,
    // Unmeasured on Pure; Pulse's own reading (`true`) carries over rather
    // than guessing a change. See `oag_title::HudArt::shield_percent`.
    shield_percent: true,
    // Measured: Pure's own plugins fill the same `HUD`/`HUDSmall` roles
    // Pulse's do, resolving to `HUDFont.fnt`/`small.fnt` rather than
    // Pulse's files - see `oag_ui::language::roles`' own doc table. See
    // `oag_title::HudArt::hud_font_role`.
    hud_font_role: "HUD",
    hud_small_font_role: Some("HUDSmall"),
    total_time_timed_modes_only: false,
    kill_column: false,
    message_slots: true,
    runtime: None,
};

/// Which `<Mode3D><Model>` widget draws each weapon's own icon, indexed in
/// `Weapon::ALL`'s order - see [`ART`]'s "weapon icons are `<Model>`s" doc for
/// the full XML, both layouts checked byte-identical on this point.
///
/// `Cannon`, `LeachBeam` and `Repulser` have no icon on this disc, and
/// `Disruptor` has one nothing else does, matching [ART]'s finding that
/// Pure's own weapon roster is not Pulse's. Neither does
/// `Shuriken` - Pure predates it - even though `oag_weapons::pickup::
/// IMPLEMENTED` can hand one out on this title, same as any other; a Pure race
/// that grants one draws no icon for it, which is the disc's own gap rather
/// than a reading this project has not done. `Quake`'s slot is filled even
/// though nothing hands a Quake out today - the widget is on the disc, so
/// recording it costs nothing.
pub const PICKUP_ICON_MODELS: [Option<&str>; 14] = [
    Some("ROCKET_icon"),    // Rocket
    Some("MISSILE_icon"),   // Missile
    Some("QUAKE_icon"),     // Quake
    None,                   // Cannon
    Some("TURBO_icon"),     // Turbo
    Some("SHIELD_icon"),    // Shield
    Some("AUTOPILOT_icon"), // Autopilot
    Some("PLASMA_icon"),    // Plasma
    Some("BOMB_icon"),      // Bomb
    Some("MINE_icon"),      // Mine
    None,                   // LeachBeam
    None,                   // Repulser
    None,                   // Shuriken
    // The tenth of this disc's ten icons, and the one weapon here that is
    // Pure's alone - `Weapon::Disruptor` is appended to the pool rather than
    // slotted where Pure's own class-name run puts it (fourth), see that
    // enum's doc.
    Some("DISRUPTOR_icon"), // Disruptor
];

/// The sprite widgets Pure draws whenever its HUD is up: **none**.
///
/// See [`ART`]. Empty because this disc's HUD layouts carry no `<Image>`
/// element of any kind, not because nothing has been looked for.
pub const ALWAYS_ON: &[&str] = &[];

/// The three layouts this disc ships, by race mode.
pub mod layouts {
    /// Single race.
    pub const ARCADE: &str = r"Data\XML\Arcade_HUD.xml";
    /// Time trial, and speed lap with it - this disc ships no `SpeedLap_HUD.xml`.
    pub const TIME_TRIAL: &str = r"Data\XML\TimeTrial_HUD.xml";
    /// Zone mode.
    pub const ZONE: &str = r"Data\XML\Zone_HUD.xml";
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sharing with Pulse is deliberate and measured, not a leftover - the
    /// same assertion [`crate::race`] makes about the default team. A future
    /// reader should not "fix" these into a divergence, and should not collapse
    /// them into a re-export either: they are two discs that were both read.
    #[test]
    fn the_layouts_are_pulses_spellings_because_this_disc_carries_them() {
        assert_eq!(layouts::ARCADE, oag_pulse::hud::layouts::ARCADE);
        assert_eq!(layouts::TIME_TRIAL, oag_pulse::hud::layouts::TIME_TRIAL);
        assert_eq!(layouts::ZONE, oag_pulse::hud::layouts::ZONE);
        assert_eq!(
            LAYOUTS.speed_lap, LAYOUTS.time_trial,
            "no SpeedLap_HUD.xml is on this disc, so speed lap draws the time \
             trial's layout"
        );
    }
}
