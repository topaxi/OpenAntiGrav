//! The `[controls]` section: how the pilot's buttons reach the ship.
//!
//! Its own module rather than another block in [`crate::settings`] because two
//! of its three keys are about a device the original never had. The PSP has no
//! analog triggers at all, so nothing here is recovered behaviour and nothing
//! here claims to be - `scheme` is the one key that mirrors the original's own
//! `Control_Type`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use oag_input::bindings::Bindings;
use oag_input::pad::TriggerMode;
use oag_input::prompt::PromptStyle;

/// How the pilot's buttons reach the ship.
///
/// Its own section rather than a corner of `[race]` because these are pilot
/// preferences that outlive any one race, the way `[display]` is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Controls {
    /// Control scheme: `veteran` or `novice`.
    ///
    /// A **token**, not a typed enum, for the reason `Race::mode` is: a bad
    /// value in this file must not fail the boot, and `load` propagates a serde
    /// error rather than falling back. An unrecognised scheme is reported and
    /// the default is used - that fallback lives in `main.rs`'s
    /// `resolve_scheme`, which is also where `--scheme` overrides this.
    ///
    /// The two differ in *how a sideshift is asked for*, not in what the ship
    /// does: veteran double-taps an airbrake, novice holds a dedicated button
    /// and flicks the stick. See
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
    #[serde(default = "default_scheme")]
    pub scheme: String,
    /// What the analog triggers do: `airbrakes` or `thrust_brake`.
    ///
    /// A **token** for the same reason `scheme` is one, and resolved the same
    /// way - `main.rs`'s `resolve_triggers` reports an unrecognised value and
    /// carries on. See [`TriggerMode`] for what each mapping is and why the
    /// older one is still offered.
    #[serde(default = "default_triggers")]
    pub triggers: String,
    /// How hard a trigger has to be pulled for a given amount of airbrake.
    ///
    /// **Typed rather than a token**, unlike the two above, for the reason
    /// [`oag_display::display::Brightness`] is typed: this one is a number, and a
    /// number out of range is a file that fails to load with a message rather
    /// than a control discovered by feel to do nothing.
    #[serde(default)]
    pub trigger_sensitivity: TriggerSensitivity,
    /// Whether the on-screen GO button adds an airbrake in its bottom-left
    /// and bottom-right corners (`true`), or is thrust alone (`false`). Only a
    /// touchscreen's overlay reads it.
    ///
    /// **On by default - chosen, not measured**, at the maintainer's request;
    /// off for a phone whose screen or hands suit the separate BRAKE buttons
    /// better. See `oag_input::touch::GoZone`.
    #[serde(default = "default_touch_go_zones")]
    pub touch_go_zones: bool,
    /// Which on-screen scheme a touchscreen shows: `standard` or `easy` (see
    /// `oag_input::touch::Scheme`). A token, so an unrecognised value is
    /// reported and falls back to standard rather than failing the file.
    /// **Chosen, not measured.**
    #[serde(default = "default_touch_scheme")]
    pub touch_scheme: String,
    /// How strong the on-screen racing controls draw, in percent of the
    /// design's own translucency (`100`), for a phone or a track where they
    /// get in the way. Only a touchscreen's overlay reads it. **Chosen, not
    /// measured.**
    #[serde(default = "default_touch_opacity")]
    pub touch_opacity: u8,
    /// Which key produces each abstract button, when a player has moved one
    /// off `oag_input::keys::candidates`'s built-in layout.
    ///
    /// A plain `BTreeMap<String, String>` and not [`Bindings`] itself: this
    /// crate already depends on serde and [`Bindings`] deliberately does
    /// not, per its own module doc, so the table this file persists is the
    /// same shape [`AnisotropyDef`](super::AnisotropyDef) and
    /// `LodDef` was before `[graphics] lod` went, a type this crate owns standing in for
    /// one it does not derive on.
    ///
    /// **Complete on every write, one row per candidate key, `"none"` for a
    /// key nothing currently produces** - never a diff against the default -
    /// so a button a rebind emptied stays empty across a restart rather than
    /// reverting the moment the file that recorded the steal is rewritten.
    /// See [`Bindings::to_pairs`].
    ///
    /// An entry this build cannot parse - a name it does not offer, or a
    /// value that is not a button name or `"none"` - is dropped and the
    /// default kept for that key, never a load failure: see
    /// [`Bindings::from_pairs`] and `main::args::resolve_bindings`, which is
    /// what reports the fallback for a live keyboard. [`Self::live_bindings`]
    /// is the same fallback without the reporting, for a caller - the
    /// `--menu-page` capture - that only draws a picture of the file rather
    /// than driving a ship with it.
    #[serde(default = "default_bindings")]
    pub bindings: BTreeMap<String, String>,
    /// Which glyphs the on-screen button prompts draw: `auto`, `original`,
    /// `playstation`, `xbox`, `nintendo` or `keyboard`.
    ///
    /// A **token** for the reason `scheme` is one: a bad value is reported and
    /// `auto` used, never a failed boot. Config only, no menu row: `auto`
    /// follows the device last used, and the rest are for a player whose pad
    /// is not what it looks like. See [`oag_input::prompt`] and
    /// `docs/ui/button-prompts.md`.
    #[serde(default = "default_prompt_style")]
    pub prompt_style: String,
    /// Pilot Assist for the player's craft: a corridor spring that yaws it off a
    /// wall it is about to meet, at a few percent of thrust (`oag_physics::pilot_assist`).
    /// Only titles that author its table have it.
    ///
    /// **On by default on Android only** (maintainer, 2026-10-10): touch controls are
    /// far harder than a pad, and a player on a desktop is assumed to know Wipeout.
    #[serde(default = "default_pilot_assist")]
    pub pilot_assist: bool,
}

fn default_pilot_assist() -> bool {
    cfg!(target_os = "android")
}

fn default_prompt_style() -> String {
    PromptStyle::default().name().to_string()
}

impl Controls {
    /// [`Self::prompt_style`] as a value, `auto` for a token this build does
    /// not know. Silent; `main::args::resolve_prompt_style` is the one that
    /// reports it.
    #[must_use]
    pub fn prompt_style_value(&self) -> PromptStyle {
        PromptStyle::from_name(&self.prompt_style).unwrap_or_default()
    }
}

/// See [`Controls::bindings`]: the default table, complete, with nothing
/// rebound.
fn default_bindings() -> BTreeMap<String, String> {
    Bindings::default().to_pairs()
}

impl Controls {
    /// [`Self::bindings`] turned back into a live table, falling back to the
    /// default for any entry that does not parse.
    ///
    /// Silent about what it fell back on, unlike `main::args::resolve_bindings`:
    /// this is the settings side handing a picture over, the same seam
    /// [`crate::settings::menu_seeds`]'s own doc comment describes, and a
    /// capture drawing one page of the menus has nobody to tell.
    #[must_use]
    pub fn live_bindings(&self) -> Bindings {
        Bindings::from_pairs(&self.bindings).0
    }
}

/// How much airbrake a given amount of trigger travel asks for, as a
/// percentage. 100 is a linear pull.
///
/// The same shape as [`oag_display::display::Gamma`], and for the same reason: an
/// exponent curve is the honest way to say "the same travel, distributed
/// differently", and a percentage is the only way to say it that the menus can
/// render - their entry kinds are `action`, `back`, `binding`, `choice` and
/// `submenu`, and there is no slider among them.
///
/// Above 100 the brake arrives with less travel; below it the first half of the
/// pull buys less brake, which is what a pilot who wants to feather one side
/// through a corner is asking for. Both ends still map to nothing and to
/// everything at every setting - see `oag_input::pad::condition`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct TriggerSensitivity(u32);

impl TriggerSensitivity {
    /// A linear pull: the airbrake tracks the trigger one for one.
    pub const NEUTRAL: Self = Self(100);

    /// The gentlest and sharpest this may be.
    ///
    /// Bounded at both ends because both ends are a trigger that does not work:
    /// far enough down and full braking is unreachable in practice, far enough
    /// up and the first millimetre of travel is a full airbrake.
    pub const RANGE: std::ops::RangeInclusive<u32> = 25..=400;

    /// The percentages the menus offer.
    pub const OFFERED: [Self; 6] = [
        Self(50),
        Self(75),
        Self(100),
        Self(125),
        Self(150),
        Self(200),
    ];

    /// The exponent this percentage means, for `oag_input::pad::condition`.
    ///
    /// Inverted, because the percentage reads as *sensitivity* and the exponent
    /// works the other way: 200% is an exponent of 0.5, which reaches a full
    /// airbrake at half travel.
    #[must_use]
    pub fn exponent(self) -> f32 {
        100.0 / self.0 as f32
    }

    /// The percentage itself.
    #[must_use]
    pub fn percent(self) -> u32 {
        self.0
    }
}

impl Default for TriggerSensitivity {
    fn default() -> Self {
        Self::NEUTRAL
    }
}

impl TryFrom<u32> for TriggerSensitivity {
    type Error = String;

    fn try_from(percent: u32) -> Result<Self, Self::Error> {
        if Self::RANGE.contains(&percent) {
            Ok(Self(percent))
        } else {
            Err(format!(
                "trigger sensitivity {percent} is outside {}..={}",
                Self::RANGE.start(),
                Self::RANGE.end()
            ))
        }
    }
}

impl From<TriggerSensitivity> for u32 {
    fn from(value: TriggerSensitivity) -> Self {
        value.0
    }
}

impl std::fmt::Display for TriggerSensitivity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for TriggerSensitivity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let percent: u32 = s
            .trim()
            .parse()
            .map_err(|_| format!("{s:?} is not a whole percentage"))?;
        Self::try_from(percent)
    }
}

/// Veteran, because it is what the original ships unless a profile says
/// `novice` - `Options_LoadControlMapping`'s fall-through sets the scheme flag
/// and the built-in mapping blob carries the same value. Confidence 75; the
/// value a never-configured profile holds was not read.
fn default_scheme() -> String {
    oag_gameplay::ControlScheme::default().name().to_string()
}

/// The airbrakes, because they are the only trigger mapping that uses what the
/// hardware offers. See [`TriggerMode::Airbrakes`].
fn default_triggers() -> String {
    TriggerMode::default().name().to_string()
}

impl Controls {
    /// The overlay's scheme and zone setting; an unrecognised scheme token
    /// is standard.
    #[must_use]
    pub fn touch_setup(&self) -> oag_input::touch::Setup {
        oag_input::touch::Setup::new(
            oag_input::touch::Scheme::parse(&self.touch_scheme).unwrap_or_default(),
            self.touch_go_zones,
        )
    }

    /// The overlay's alpha multiplier: `touch_opacity` as a share of 1.
    #[must_use]
    pub fn touch_alpha(&self) -> f32 {
        f32::from(self.touch_opacity) / 100.0
    }
}

fn default_touch_go_zones() -> bool {
    true
}

fn default_touch_scheme() -> String {
    oag_input::touch::Scheme::default().name().to_string()
}

fn default_touch_opacity() -> u8 {
    100
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            scheme: default_scheme(),
            triggers: default_triggers(),
            trigger_sensitivity: TriggerSensitivity::default(),
            touch_go_zones: default_touch_go_zones(),
            touch_scheme: default_touch_scheme(),
            touch_opacity: default_touch_opacity(),
            bindings: default_bindings(),
            prompt_style: default_prompt_style(),
            pilot_assist: default_pilot_assist(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_neutral_sensitivity_is_a_linear_pull() {
        assert_eq!(TriggerSensitivity::default().exponent(), 1.0);
        assert_eq!(TriggerSensitivity::NEUTRAL.percent(), 100);
    }

    /// Higher percent means more sensitive, which is the opposite direction to
    /// the exponent it becomes - the single most likely slip in this file.
    #[test]
    fn a_higher_percentage_reaches_full_braking_sooner() {
        let sharp = TriggerSensitivity::try_from(200).unwrap();
        let gentle = TriggerSensitivity::try_from(50).unwrap();
        assert!(sharp.exponent() < TriggerSensitivity::NEUTRAL.exponent());
        assert!(gentle.exponent() > TriggerSensitivity::NEUTRAL.exponent());

        let half = 0.5f32;
        assert!(half.powf(sharp.exponent()) > half.powf(gentle.exponent()));
    }

    #[test]
    fn a_percentage_outside_the_range_is_refused_rather_than_clamped() {
        assert!(TriggerSensitivity::try_from(0).is_err());
        assert!(TriggerSensitivity::try_from(10_000).is_err());
        assert!(TriggerSensitivity::try_from(*TriggerSensitivity::RANGE.start()).is_ok());
        assert!(TriggerSensitivity::try_from(*TriggerSensitivity::RANGE.end()).is_ok());
    }

    #[test]
    fn every_offered_percentage_is_inside_the_range() {
        for offered in TriggerSensitivity::OFFERED {
            assert!(
                TriggerSensitivity::RANGE.contains(&offered.percent()),
                "the menus offer {offered}, which cannot be applied"
            );
            assert_eq!(offered.to_string().parse(), Ok(offered));
        }
        assert!(
            TriggerSensitivity::OFFERED.contains(&TriggerSensitivity::NEUTRAL),
            "no way back to a linear pull once it is changed"
        );
    }

    /// The two ends of this range and the two ends of
    /// `oag_input::pad::CURVE_RANGE` are one bound written twice, and the
    /// menus can never reach it - `OFFERED` stops at 200 and 50 - so only a
    /// hand-edited file finds out whether they agree. If they drift, the
    /// clamp one layer down silently overrules a value this type accepted.
    #[test]
    fn the_widest_sensitivity_is_the_widest_curve_the_pad_will_apply() {
        let range = oag_input::pad::CURVE_RANGE;
        let gentlest = TriggerSensitivity::try_from(*TriggerSensitivity::RANGE.start()).unwrap();
        let sharpest = TriggerSensitivity::try_from(*TriggerSensitivity::RANGE.end()).unwrap();
        assert_eq!(gentlest.exponent(), *range.end());
        assert_eq!(sharpest.exponent(), *range.start());
    }

    #[test]
    fn the_defaults_are_tokens_the_input_layer_knows() {
        let controls = Controls::default();
        assert_eq!(
            TriggerMode::from_name(&controls.triggers),
            Some(TriggerMode::Airbrakes)
        );
        assert!(oag_gameplay::ControlScheme::from_name(&controls.scheme).is_some());
    }
}
