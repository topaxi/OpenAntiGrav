//! Which tick the gantry's timeline starts on, and what holds it on `GO`.
//!
//! **Pulse's rule, applied to a title's own asset.** Pulse is the one title
//! whose countdown was captured: the board turns green and shows `GO` on race
//! tick 273, one tick after the thrust gate opens
//! ([`oag_race::COUNTDOWN_TICKS`]), and keeps showing it. The timeline's `GO`
//! edge is asset frame 181 there, so frame 0 is tick 92 ([`Clock::PULSE`]).
//!
//! The maintainer's rule (2026-10-03) is that a title with no measured rule of
//! its own runs Pulse's, labelled **inherited from Pulse, unmeasured on
//! <title>**. Inheriting means the *rule*, not the literal 92: the title's own
//! `GO` edge lands on the same tick and the clock is held on `GO` from there.
//! [`go_edge`] finds that edge in the title's own asset and [`Clock::inherited`]
//! turns it into a start tick. Where no edge can be found the clock runs from
//! the race start, as it always did, and the loader says so.
//!
//! # What `GO` is on Wipeout HD
//!
//! HD authors no `TEXOFFSET` step. Its board is one material's Edge Animation
//! curve (`docs/formats/edge-animation.md`) walking `uvOffset` over a texture
//! whose staircase is Pulse's at four times the size. The backdrop's red and
//! its green are texels of that texture, so the edge is read the way Pulse's
//! was measured: the first frame at which a drawn vertex samples the authored
//! green. That is the frame the `u` offset finishes stepping across to the
//! green marker column - the same "green step" Pulse's tick 273 was pinned on.
//! Nothing here is a measurement of HD's own countdown: no capture of it
//! exists.

use oag_render::mesh::{DrawCall, Model};
use oag_render::mesh_render::TexAnims;

use oag_race::COUNTDOWN_TICKS;

use super::{CLOCK_START_TICK, GO_LOOP};

/// How far into the asset's timeline [`go_edge`] looks, in frames: the 6.000 s
/// at which both Pulse's and HD's countdown panels leave and their texture
/// loops close.
pub const SEARCH_FRAMES: u32 = 360;

/// A node counts as having moved once its translation is this far from where
/// it sat at the `GO` edge, in world units. The panels' own exits are about
/// ten.
const MOVED: f32 = 1.0;

/// The gantry's timeline clock: where it starts and what holds it on `GO`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clock {
    /// The race tick the timeline starts on.
    pub start_tick: u64,
    /// The span, in seconds, the clock loops once `GO` is up, or `None` to run
    /// the authored timeline straight on.
    pub hold: Option<(f32, f32)>,
}

impl Clock {
    /// The timeline off the race start, never held: how every title ran
    /// before Pulse's countdown was measured, and what a title whose `GO` edge
    /// cannot be found still does - **chosen, not measured**.
    pub const FROM_RACE_START: Self = Self {
        start_tick: 0,
        hold: None,
    };

    /// Pulse's, **measured** (confidence 85): frame 0 on tick 92 and `GO`
    /// held over [`GO_LOOP`].
    pub const PULSE: Self = Self {
        start_tick: CLOCK_START_TICK,
        hold: Some(GO_LOOP),
    };

    /// Pulse's rule on a title's own asset: `edge`'s `GO` frame lands on the
    /// tick after the thrust gate opens, as Pulse's frame 181 lands on 273, and
    /// the clock is held over the frames from the digits' last fade to the
    /// asset's own exit.
    ///
    /// `None` when the edge is later than the release, which would need a
    /// negative start tick.
    #[must_use]
    pub fn inherited(edge: GoEdge) -> Option<Self> {
        let start_tick = (COUNTDOWN_TICKS + 1).checked_sub(u64::from(edge.frame))?;
        Some(Self {
            start_tick,
            hold: Some((
                edge.settled_frame as f32 / 60.0,
                edge.last_frame_before_exit as f32 / 60.0,
            )),
        })
    }

    /// The gantry's clock in seconds at race tick `tick`.
    #[must_use]
    pub fn seconds(&self, tick: u64) -> f32 {
        let seconds = super::clock_seconds(self.start_tick, tick);
        match self.hold {
            Some(span) => super::held_within(seconds, span),
            None => seconds,
        }
    }
}

/// Where a title's own asset shows `GO`, in frames of its own timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoEdge {
    /// The first frame a drawn vertex samples the authored green.
    pub frame: u32,
    /// The first frame from `frame` on at which every vertex that sampled lit
    /// white before the edge - the digits - samples no alpha at all: the
    /// earliest the clock can loop back to without replaying a digit. `frame`
    /// itself where they never clear.
    pub settled_frame: u32,
    /// The last frame before a drawn node first leaves its place after the
    /// edge: where the asset stops showing `GO` and starts to exit.
    pub last_frame_before_exit: u32,
}

/// The vertices an animated texture track reaches, once each: `(track, uv,
/// texture slot)`.
fn animated_samples(model: &Model) -> Vec<(usize, [f32; 2], usize)> {
    let mut seen = Vec::new();
    let lists: [&[DrawCall]; 3] = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ];
    for draw in lists.into_iter().flatten() {
        let Some(texture) = draw.texture else {
            continue;
        };
        for index in &model.indices[draw.range.start as usize..draw.range.end as usize] {
            let vertex = &model.vertices[*index as usize];
            if vertex.anim == 0 {
                continue;
            }
            let sample = (vertex.anim as usize, vertex.texcoord, texture);
            if !seen.contains(&sample) {
                seen.push(sample);
            }
        }
    }
    seen
}

/// The animated node slots a drawn vertex rides.
fn moving_slots(model: &Model) -> Vec<usize> {
    let mut slots = Vec::new();
    let lists: [&[DrawCall]; 3] = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ];
    for draw in lists.into_iter().flatten() {
        for index in &model.indices[draw.range.start as usize..draw.range.end as usize] {
            let slot = model.vertices[*index as usize].xform as usize;
            if slot >= 1 && !slots.contains(&slot) {
                slots.push(slot);
            }
        }
    }
    slots
}

/// Whether a texel is the authored green: green clearly above red and blue and
/// not transparent. HD's marker is `(0, 170, 25)` at half alpha.
fn is_green([r, g, b, a]: [u8; 4]) -> bool {
    a > 0 && g >= 128 && i32::from(g) > i32::from(r) + 64 && i32::from(g) > i32::from(b) + 64
}

/// Whether a texel is opaque white: a lit digit.
fn is_lit_white([r, g, b, a]: [u8; 4]) -> bool {
    a >= 200 && r >= 200 && g >= 200 && b >= 200
}

/// Finds `model`'s `GO` edge, or says why it cannot.
///
/// # Errors
///
/// A line for the loader report: the model has no animated texture, its
/// texels are not on the CPU, or no drawn vertex ever samples green within
/// [`SEARCH_FRAMES`].
pub fn go_edge(model: &Model) -> Result<GoEdge, String> {
    let samples = animated_samples(model);
    if samples.is_empty() {
        return Err("no drawn vertex rides an animated texture".to_string());
    }
    let mut textures = Vec::with_capacity(samples.len());
    for (_, _, slot) in &samples {
        let rgba = model
            .textures
            .get(*slot)
            .and_then(Option::as_ref)
            .and_then(|t| {
                t.to_rgba()
                    .map(|texels| (t.width as usize, t.height as usize, texels.into_owned()))
            });
        textures.push(rgba);
    }
    if textures.iter().all(Option::is_none) {
        return Err("the board's texels are not readable on the CPU".to_string());
    }

    let texel =
        |frame: u32, (track, uv, _): &(usize, [f32; 2], usize), at: usize| -> Option<[u8; 4]> {
            let (width, height, rgba) = textures[at].as_ref()?;
            let table = TexAnims::sample(model, frame as f32 / 60.0);
            let [su, sv, ou, ov] = table.transform[*track];
            let u = (uv[0] * su + ou).rem_euclid(1.0);
            let v = (uv[1] * sv + ov).rem_euclid(1.0);
            let x = ((u * *width as f32) as usize).min(width - 1);
            let y = ((v * *height as f32) as usize).min(height - 1);
            let at = (y * width + x) * 4;
            rgba.get(at..at + 4).map(|p| [p[0], p[1], p[2], p[3]])
        };

    let frame = (0..=SEARCH_FRAMES)
        .find(|&frame| {
            samples
                .iter()
                .enumerate()
                .any(|(at, s)| texel(frame, s, at).is_some_and(is_green))
        })
        .ok_or_else(|| {
            format!(
                "no drawn vertex samples the authored green in the first {SEARCH_FRAMES} frames"
            )
        })?;

    // The digits: what sampled lit white before the edge. Once all of them
    // are clear the board shows no digit, and a loop that starts there never
    // replays one - Pulse's loop starts 35 frames after its edge for this.
    let digits: Vec<usize> = (0..samples.len())
        .filter(|&at| (0..frame).any(|f| texel(f, &samples[at], at).is_some_and(is_lit_white)))
        .collect();
    let settled_frame = (frame..=SEARCH_FRAMES)
        .find(|&f| {
            digits
                .iter()
                .all(|&at| texel(f, &samples[at], at).is_none_or(|p| p[3] == 0))
        })
        // An asset whose digits and `GO` share texels never clears (Pulse's own
        // board does not): the loop then starts at the edge, which can replay
        // a digit, and the loader line names the span so that is visible.
        .unwrap_or(frame);

    let slots = moving_slots(model);
    let at_edge = model.sample_anim_nodes(frame as f32 / 60.0);
    let exit = (frame + 1..=SEARCH_FRAMES).find(|&later| {
        let now = model.sample_anim_nodes(later as f32 / 60.0);
        slots.iter().any(|slot| {
            let (Some(a), Some(b)) = (at_edge.get(slot - 1), now.get(slot - 1)) else {
                return false;
            };
            (12..15).any(|i| (a[i] - b[i]).abs() > MOVED)
        })
    });
    Ok(GoEdge {
        frame,
        settled_frame,
        last_frame_before_exit: exit.map_or(SEARCH_FRAMES, |f| f - 1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pulse's own edge through the inherited rule gives Pulse's measured
    /// start tick: the rule and the measurement are one thing.
    #[test]
    fn pulses_edge_through_the_rule_is_the_measured_start() {
        let edge = GoEdge {
            frame: 181,
            settled_frame: 216,
            last_frame_before_exit: 349,
        };
        let clock = Clock::inherited(edge).expect("181 is before the release");
        assert_eq!(clock.start_tick, CLOCK_START_TICK);
        assert_eq!(clock.start_tick, 92);
        assert_eq!(clock.hold, Some((216.0 / 60.0, 349.0 / 60.0)));
    }

    /// Whatever the edge, it lands on the tick after the thrust gate.
    #[test]
    fn the_go_edge_lands_one_tick_after_the_release() {
        for frame in [181, 200, 203, 272] {
            let clock = Clock::inherited(GoEdge {
                frame,
                settled_frame: frame,
                last_frame_before_exit: 359,
            })
            .expect("before the release");
            let at_release = clock.seconds(COUNTDOWN_TICKS + 1) * 60.0;
            assert!(
                (at_release - frame as f32).abs() < 1e-3,
                "{frame}: {at_release}"
            );
            let before = clock.seconds(COUNTDOWN_TICKS) * 60.0;
            assert!(
                before < frame as f32,
                "{frame} lit on the release tick itself"
            );
        }
    }

    /// An edge after the release has no start tick to give.
    #[test]
    fn an_edge_after_the_release_is_refused() {
        let late = GoEdge {
            frame: (COUNTDOWN_TICKS + 2) as u32,
            settled_frame: (COUNTDOWN_TICKS + 2) as u32,
            last_frame_before_exit: 359,
        };
        assert_eq!(Clock::inherited(late), None);
    }

    /// Held, the clock never reaches the frame the asset exits on.
    #[test]
    fn an_inherited_clock_stays_inside_its_hold() {
        let clock = Clock::inherited(GoEdge {
            frame: 203,
            settled_frame: 222,
            last_frame_before_exit: 359,
        })
        .expect("before the release");
        for tick in (0..20_000).step_by(7) {
            let seconds = clock.seconds(tick);
            assert!(seconds < 359.0 / 60.0, "tick {tick}: {seconds}");
        }
        assert!(clock.seconds(COUNTDOWN_TICKS + 5000) >= 222.0 / 60.0);
    }

    /// Without a hold the clock is the plain timeline, as it was for every
    /// title before this rule.
    #[test]
    fn the_race_start_clock_is_the_plain_timeline() {
        assert_eq!(Clock::FROM_RACE_START.seconds(90), 1.5);
        assert_eq!(Clock::PULSE.seconds(CLOCK_START_TICK + 60), 1.0);
    }
}
