//! The speed and shield bars' fill law on Pulse: the marker and the value label
//! ride the tip of the fill. See [`oag_title::hud::BarTip`] for the law and its
//! source.

use oag_title::hud::{BarTip, TipBar, TipSource};

use crate::draw::Context;
use crate::{Readout, Sprite};

/// The bar of the law this widget is the fill, marker or label of.
fn bar_of<'a>(art: &'a oag_title::HudArt, name: &str) -> Option<(&'a BarTip, &'a TipBar)> {
    let law = art.bar_tip.as_ref()?;
    let bar = law
        .bars
        .iter()
        .find(|bar| bar.fill == name || bar.mark == name || bar.text == name)?;
    Some((law, bar))
}

/// The speed the bar shows: zero under the law's idle floor.
fn shown_kmh(law: &BarTip, readout: &Readout) -> f32 {
    if readout.speed_kmh < f32::from(law.idle_kmh) {
        0.0
    } else {
        readout.speed_kmh
    }
}

/// The bar's `floor + span * fraction`, the number the whole law runs on.
fn tipped_fraction(law: &BarTip, bar: &TipBar, readout: &Readout, art: &oag_title::HudArt) -> f32 {
    let fraction = match bar.source {
        TipSource::Speed => {
            let shown = Readout {
                speed_kmh: shown_kmh(law, readout),
                ..Readout::blank()
            };
            shown.speed_fraction(art.speed_full_kmh as f32)
        }
        TipSource::Shield => readout.shield_fraction(),
    };
    f32::from(law.floor_permille) / 1000.0 + f32::from(law.span_permille) / 1000.0 * fraction
}

/// The fill's width in layout units: the whole units of `fraction` of the
/// bar's authored width, as the executable's floor does.
fn tip_width(fraction: f32, authored_width: f32) -> f32 {
    (fraction * authored_width).floor()
}

/// The fill's `floor + span * fraction`, or `None` for a widget that is not a
/// bar of the law. Stands in for [`crate::draw::bar_fraction`]'s answer.
pub(super) fn fill_fraction(art: &oag_title::HudArt, readout: &Readout, name: &str) -> Option<f32> {
    let (law, bar) = bar_of(art, name).filter(|(_, bar)| bar.fill == name)?;
    Some(tipped_fraction(law, bar, readout, art))
}

/// The fill cropped by the law: the width is the floored tip, and the texture
/// window is the **right** `fraction` of the art, so the art's pointed end is
/// the tip at every value.
pub(super) fn crop(sprite: &Sprite, fraction: f32) -> Sprite {
    let mut out = sprite.clone();
    out.rect[2] = tip_width(fraction, sprite.rect[2]);
    out.uv[0] = sprite.uv[0] + sprite.uv[2] * (1.0 - fraction);
    out.uv[2] = sprite.uv[2] * fraction;
    out
}

/// How far the marker and the label of a bar sit right of their authored
/// column: the tip's width plus the law's offset. Zero for any other widget.
pub(super) fn dx(cx: &Context<'_>, readout: &Readout, name: &str) -> f32 {
    let Some((law, bar)) = bar_of(cx.art, name).filter(|(_, bar)| bar.fill != name) else {
        return 0.0;
    };
    let Some(fill) = cx.layout.sprite(bar.fill) else {
        return 0.0;
    };
    let fraction = tipped_fraction(law, bar, readout, cx.art);
    tip_width(fraction, fill.rect[2]) + f32::from(law.tip_dx)
}

/// The label a bar's value shows: whole units, truncated, as the executable's
/// integer conversion does. `None` for any other widget.
pub(super) fn text(
    art: &oag_title::HudArt,
    readout: &Readout,
    name: &str,
    speed_unit: bool,
) -> Option<String> {
    let (law, bar) = bar_of(art, name).filter(|(_, bar)| bar.text == name)?;
    Some(match bar.source {
        TipSource::Speed if speed_unit => format!("{} kmh", shown_kmh(law, readout) as i32),
        TipSource::Speed => (shown_kmh(law, readout) as i32).to_string(),
        TipSource::Shield => format!("{}%", (readout.shield_fraction() * 100.0) as i32),
    })
}
