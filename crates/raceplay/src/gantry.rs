//! The start gantry: loading `321Go_StartFinish.vex`, standing it on the
//! circuit's own mount, and running its authored timeline off the race clock.
//!
//! `docs/rendering/start-gantry.md` has what the asset *does*: one 12.35-second
//! timeline of `Anim Transform` one-shots and a `TEXOFFSET` walk that plays
//! `3`, `2`, `1`, `GO` and then hands the board over to a `FINAL LAP` and a
//! chequered state. What it did not have, for seven passes, was where the thing
//! stands - `TrackStartup.xml`'s slot 8 names the model and carries no
//! transform, and neither does the billboard system that loads it.
//!
//! [`oag_render::gantry`] is the half that answers that, off the circuit's own
//! geometry. This is the half that draws it.
//!
//! # The clock starts 92 ticks in, so `GO` lands on the release
//!
//! **Measured against the original, 2026-09-29** (Time Trial, Talon's Junction,
//! `pulse-psp-usa.chd` in PPSSPP; one screenshot per tick through the countdown,
//! the tick numbered by a breakpoint in `Ship_UpdateCraft`, on four runs that
//! agree). The gantry does not start its timeline when the race starts:
//!
//! - The board is a blank dark-red panel until **tick 132**, when the `3`
//!   first reads white; `2` follows at tick 178 and `1` at tick 222 - spacing 46
//!   and 44 ticks, the asset's own 45.
//! - The board turns **green and shows `GO` on tick 273**, one tick after the
//!   thrust gate opens at [`oag_race::COUNTDOWN_TICKS`] = 272, and the HUD's
//!   clocks start on that same release ([`oag_race::race_clock_ticks`]).
//!
//! **The 92 comes from the green step**, the one sharp edge: the asset's `u`
//! step at frame 181 lands on tick 273, so frame 0 is tick 92 -
//! [`CLOCK_START_TICK`], which is `272 - 180`. The digit windows are softer and
//! bracket it: their midpoints (ticks 152, 197, 242) against the asset's lit
//! windows put frame 0 at about 85, and against the texel-row windows at about
//! 96, so the digits may be up to ~7 ticks off in either direction; only the
//! `GO` handover is pinned to a tick. **Measured on Pulse only** - see
//! [`clock`] for what every other title gets: Pulse's rule on its own `GO` edge.
//!
//! This **retires** the earlier reading of this module and of
//! `docs/rendering/start-gantry.md`, which drove the gantry off `world.tick / 60`
//! from tick 0 on the argument that the timeline and the countdown "share a
//! zero", and so drew `GO` about 1.5 s before the craft could move (the
//! maintainer's "noticeable delay after `GO`"). They do not share a zero:
//! `g_ingame->0x40`, the clock that argument leaned on, is not reset by a
//! restart (it read 41.9 s and 77.0 s at the start of two captures) and is not
//! what this timeline rides. Nothing else on the track moves - the scenery's own
//! animation is still `world.tick / 60`, and is not touched.
//!
//! # After the release: the race manager's windows
//!
//! The asset's timeline keeps going after the countdown: the `Board` dressing
//! teleports in at frame 350, the `FINAL LAP` board at 560 and the chequered
//! one at 740. **What picks between them is read from `BOOT.BIN`**
//! (`docs/ghidra/functions/psp-pulse-usa/gantry-clock.md`, confidence 85):
//! `RaceManager_Update` (`0x08829778`) holds the gantry's time in a window
//! chosen by the player's crossing count, resetting it to the window's start
//! whenever it reads outside - `[3.2, 5.5)` before the first crossing (`GO`
//! strobing, [`clock::PULSE_PRE_LAP_WINDOW`]), then `[6.0, 9.0)`, `[9.5,
//! 12.0)` on the lap before the last and `[12.4, 13.3)` on the last
//! ([`board::PULSE_LAP_WINDOWS`]). HD's race manager runs the same law with
//! its own numbers. This replaces the chosen `GO` loop over frames 216..349,
//! and the stop at [`CLOCK_LIMIT`], which now holds only on a clock with no
//! windows (every title that inherits Pulse's rule unread).
//!
//! **Measured against the original, 2026-09-30** (PPSSPP, Talon's Junction,
//! craft stationary, about 21 s of race clock): the board shows a strobing `GO`
//! throughout. It never replays `3 2 1`, the panel never leaves the aperture,
//! and the `Board` dressing never arrives - the pre-lap window, which ends at
//! frame 330, twenty frames before the `Board` teleport.

mod board;
mod clock;

pub use board::{
    BoardTracker, BoardWindow, HD_BETWEEN_LAPS_WINDOW, HD_CHEQUERED_WINDOW,
    HD_FINAL_LAP_AHEAD_WINDOW, HD_LAP_WINDOWS, LapWindows, PULSE_LAP_WINDOWS, PanelCull,
    WindowEntry,
};
pub use clock::{
    Clock, GoEdge, HD_PRE_LAP_WINDOW, PULSE_PRE_LAP_WINDOW, ReleaseWindow, SEARCH_FRAMES, go_edge,
};

use super::*;

use oag_mesh::mesh::Bounds;
use oag_render::gantry::Mount;

/// Where the gantry's authored timeline is held, in seconds.
///
/// **The frame is authored; stopping there is this project's decision**, and
/// it applies only to a clock with no race-manager windows ([`Clock::laps`]):
/// every title that inherits Pulse's rule without its own race code read.
/// `docs/rendering/start-gantry.md`'s timeline table has frame 560/561
/// (9.333 s) as `Final_Lap`'s first key, so this is the last frame before the
/// first state such a title has no trigger for. Holding rather than looping is
/// what makes the gantry sit in its post-countdown idle - `Board`, `Text` and
/// `Arrow` in place, the digit panel already teleported out of the aperture -
/// for the rest of the race, which is the state the asset itself holds from
/// 7.25 s to 9.333 s.
pub const CLOCK_LIMIT: f32 = 559.0 / 60.0;

/// The tick the gantry's authored timeline starts on: frame 0 of `3`, `2`, `1`,
/// `GO`.
///
/// **Measured, ~3 ticks of uncertainty**: see the module doc for the capture.
/// Written as `COUNTDOWN_TICKS - 180` because 180 is the authored frame just
/// before the `u` step that hands the board from the digits to `GO`, and the
/// measured green step falls one tick after the release; the measurement is the
/// 92, the subtraction is only how it is spelled.
pub const CLOCK_START_TICK: u64 = oag_race::COUNTDOWN_TICKS - 180;

/// `seconds` on a gantry clock held over `span`: straight through until its
/// end, then round the span forever: what [`Clock::inherited`] holds a
/// title's `GO` on.
#[must_use]
pub fn held_within(seconds: f32, (from, to): (f32, f32)) -> f32 {
    if seconds < to {
        seconds
    } else {
        from + (seconds - from) % (to - from)
    }
}

/// The gantry's own clock in seconds at race tick `tick`: zero until
/// `start_tick`, then the asset's timeline at 60 frames a second.
///
/// **`start_tick` is [`CLOCK_START_TICK`] on Pulse - measured - and, on every
/// other title, the tick [`Clock::inherited`] derives from that title's own
/// `GO` edge: Pulse's rule - measured on HD since 2026-10-04, unmeasured
/// elsewhere.** Pulse's and HD's countdowns were captured; HD's and 2048's gantry files are different timelines (2048's own
/// `GO` slides in at frame 200, not 181), so the literal 92 is not copied, the
/// rule is. A title whose edge cannot be found keeps `0` - chosen, not
/// measured. [`Gantry::write`] clamps it at [`CLOCK_LIMIT`].
#[must_use]
pub fn clock_seconds(start_tick: u64, tick: u64) -> f32 {
    tick.saturating_sub(start_tick) as f32 / 60.0
}

/// The gantry, placed: the model and the matrix that stands it on the mount.
///
/// Produced by [`place`] at load and consumed by [`Gantry::new`], so the
/// measurement happens once on the loading screen and not per frame.
pub struct Placed {
    pub(super) model: Model,
    pub(super) matrix: Mat4,
    /// Where the timeline starts and what keeps it on `GO`: [`Clock::PULSE`]
    /// on Pulse, HD's race-manager window on the PS3 titles ([`Clock::hd`]),
    /// or the timeline off the race start where no edge could be found.
    pub(super) clock: Clock,
    /// What to leave out at each moment of the timeline, on a title whose
    /// clock reaches the later board states (HD); `None` where the loader
    /// removed the parked states outright (Pulse, PS2).
    pub(super) cull: Option<PanelCull>,
}

impl Placed {
    /// The gantry model as placed, for a test to sample what the clock shows.
    #[must_use]
    pub fn model(&self) -> &Model {
        &self.model
    }

    /// The clock the loader chose for it.
    #[must_use]
    pub fn clock(&self) -> Clock {
        self.clock
    }

    /// The per-moment cull, where the model keeps its later states.
    #[must_use]
    pub fn cull(&self) -> Option<&PanelCull> {
        self.cull.as_ref()
    }
}

impl std::fmt::Debug for Placed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Placed")
            .field("model", &self.model.label)
            .field("matrix", &self.matrix)
            .field("clock", &self.clock)
            .finish()
    }
}

/// Finds the circuit's mount, loads the gantry and works out its matrix.
///
/// `None` - and a line in `report` saying why - whenever any link in that chain
/// is missing. A circuit whose track authors no mount gets **no gantry**, not a
/// gantry at a coordinate borrowed from a circuit that does: the mount is a
/// per-circuit measurement and eleven of the twelve values differ.
pub(super) fn place(
    archives: &mut oag_assets::Archives,
    name: &str,
    track_model: &Model,
    start: Option<&StartPosition>,
    clock: ClockRule,
    report: &mut Vec<String>,
) -> Option<Placed> {
    let Some(mount) = oag_render::gantry::mount(track_model) else {
        report.push(
            "no start gantry: this circuit's track authors no 321backplate/billboard8 \
             surface, so there is nothing measured to stand one on"
                .to_string(),
        );
        return None;
    };
    let Some(start) = start else {
        report.push(
            "no start gantry: the mount was found but the track authors no Start Position, \
             and the mount's plane normal has no sign without one"
                .to_string(),
        );
        return None;
    };
    // Built with the texture sink set aside: `clock::go_edge` reads this
    // model's texels, and a sunk texture has none. One 64x128 texture, so the
    // peak the sink exists to keep down does not move.
    let model = match oag_mesh::mesh_render::without_texture_sink(|| load(archives, name, report)) {
        Ok(model) => model,
        Err(e) => {
            report.push(format!("no start gantry: {name} did not load ({e})"));
            return None;
        }
    };
    let mut model = model;
    let matrix = matrix(&mount, Vec3::from(start.forward));
    if clock == ClockRule::HdRaceManager {
        return Some(place_hd(model, matrix, &mount, name, report));
    }
    // Slot 7's FX-350 art is never shown on Pulse in any reference frame, and
    // what Pulse's slot 7 binds is unread: left out for the whole race.
    let fx350 = oag_render::gantry::strip_fx350_art(&mut model);
    let (cull, parked) = panel_cull(&model, Vec::new(), &mount, &PULSE_LAP_WINDOWS);
    place_bounds(&mut model, matrix);
    report.push(format!(
        "start gantry {name} on node {:?}: centre {:?}, on a {:.1} x {:.1} surface \
         ({:.1} thick over {} vertices), {:.0} units ahead of the Start Position - \
         measured off this circuit's own geometry (docs/rendering/start-gantry.md). \
         Before the first line crossing {parked} draw(s) parked outside the panel are not \
         drawn; after it, Pulse's race manager (0x08829778) plays the Board, FINAL LAP and \
         the chequered flag by lap, and each frame draws only what stands on the panel \
         (read from BOOT.BIN, confidence 85). {fx350} draw(s) bound to slot 7's own \
         fx350_nomip.gtf art are not drawn: embedded in this model but never shown in a \
         reference frame on Pulse. Clock: frame 0 on tick {} (measured), then [{:.1}, {:.1}) \
         s from the release, reset to its start when outside - GO strobes every {} ticks \
         until the first crossing (read, confidence 85)",
        mount.node,
        mount.centre.to_array().map(|v| (v * 10.0).round() / 10.0),
        mount.width,
        mount.height,
        mount.thickness,
        mount.vertices,
        (mount.centre - Vec3::from(start.position)).length(),
        CLOCK_START_TICK,
        PULSE_PRE_LAP_WINDOW.from,
        PULSE_PRE_LAP_WINDOW.to,
        PULSE_PRE_LAP_WINDOW.period(),
    ));
    Some(Placed {
        model,
        matrix,
        clock: Clock::PULSE,
        cull: Some(cull),
    })
}

/// The per-frame cull for a panel on `mount`: `countdown` plus every draw
/// [`oag_render::gantry::clip_to_panel`] would park, hidden before the
/// later windows, and the panel test per frame across them. Returns the cull
/// and how many draws were parked.
fn panel_cull(
    model: &Model,
    mut countdown: Vec<u32>,
    mount: &Mount,
    windows: &LapWindows,
) -> (PanelCull, usize) {
    let half_width = mount.width / 2.0;
    // Measured on a copy, so the count and the guard against clipping
    // everything stay `clip_to_panel`'s own.
    let mut probe = model.clone();
    let keep: Vec<u32> = {
        oag_render::gantry::clip_to_panel(&mut probe, half_width, 0.0);
        let lists = [
            &probe.draws,
            &probe.alpha_tested_draws,
            &probe.transparent_draws,
        ];
        lists
            .into_iter()
            .flatten()
            .map(oag_render::gantry::panel::key)
            .collect()
    };
    let parked: Vec<u32> = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ]
    .into_iter()
    .flatten()
    .map(oag_render::gantry::panel::key)
    .filter(|key| !keep.contains(key))
    .collect();
    for key in &parked {
        if !countdown.contains(key) {
            countdown.push(*key);
        }
    }
    let cull = PanelCull::build(model, countdown, windows, half_width, mount.height / 2.0);
    (cull, parked.len())
}

/// HD's gantry, which keeps every state it authors: the countdown's own
/// leave-outs are named rather than removed, and the later windows cull per
/// frame ([`PanelCull`]).
/// HD's gantry as a card: loaded, clocked and culled exactly as [`place_hd`]
/// does, but framed by the model's own camera and drawn into the target the
/// track's `billboard8` quad shows ([`oag_title::adverts::Adverts::gantry_card`]).
///
/// Nothing is placed: the quad is the track's own, so no mount basis, matrix or
/// start position is read. The mount is measured only for the panel's extent,
/// which [`PanelCull`] tests the later board states against.
pub(super) fn place_card(
    archives: &mut oag_assets::Archives,
    name: &str,
    track_model: &Model,
    spec: &oag_title::adverts::Adverts,
    report: &mut Vec<String>,
) -> Option<crate::adverts::Card> {
    let Some(mount) = oag_render::gantry::mount(track_model) else {
        report.push(
            "no start gantry: this circuit's track authors no billboard8 surface to show a card on"
                .to_string(),
        );
        return None;
    };
    let loaded =
        oag_mesh::mesh_render::without_texture_sink(|| load_with_blob(archives, name, report));
    let (model, blob) = match loaded {
        Ok(loaded) => loaded,
        Err(e) => {
            report.push(format!("no start gantry: {name} did not load ({e})"));
            return None;
        }
    };
    let countdown = oag_render::gantry::panel::fx350_draws(&model);
    let (cull, parked) = panel_cull(&model, countdown, &mount, &HD_LAP_WINDOWS);
    let clock = hd_clock(&model, name, report);
    let mut card = match crate::adverts::card_from(8, model, &blob, spec) {
        Ok(card) => card,
        Err(why) => {
            report.push(format!(
                "no start gantry: {name} not drawn as a card ({why})"
            ));
            return None;
        }
    };
    let fx350 = oag_render::gantry::panel::fx350_draws(&card.model).len();
    report.push(format!(
        "start gantry {name}: drawn through its own camera into a {}x{} target shown on the \
         circuit's billboard8 quad(s), as the original does (RPCS3, 2026-10-08). The panel's \
         {:.1} x {:.1} extent is measured off this circuit's own geometry \
         (docs/rendering/start-gantry.md). Before the first line crossing {parked} draw(s) parked \
         outside the panel and {fx350} bound to slot 7's own fx350_nomip.gtf art are not drawn; \
         after it, HD's race manager plays the FX-350 board, FINAL LAP and the chequered flag, and \
         each frame draws only what stands on the panel (measured on RPCS3, 2026-10-04)",
        spec.target.0, spec.target.1, mount.width, mount.height,
    ));
    card.timeline = Some(CardTimeline {
        clock,
        cull,
        tracker: std::cell::Cell::default(),
    });
    Some(card)
}

fn place_hd(
    mut model: Model,
    matrix: Mat4,
    mount: &Mount,
    name: &str,
    report: &mut Vec<String>,
) -> Placed {
    let countdown = oag_render::gantry::panel::fx350_draws(&model);
    let fx350 = countdown.len();
    let (cull, parked) = panel_cull(&model, countdown, mount, &HD_LAP_WINDOWS);
    place_bounds(&mut model, matrix);
    report.push(format!(
        "start gantry {name} on node {:?}: centre {:?}, on a {:.1} x {:.1} surface \
         ({:.1} thick over {} vertices) - measured off this circuit's own geometry \
         (docs/rendering/start-gantry.md). Before the first line crossing {} draw(s) parked \
         outside the panel and {fx350} bound to slot 7's own fx350_nomip.gtf art are not \
         drawn; after it, HD's race manager plays the FX-350 board, FINAL LAP and the \
         chequered flag, and each frame draws only what stands on the {:.1} x {:.1} panel \
         (measured on RPCS3, 2026-10-04)",
        mount.node,
        mount.centre.to_array().map(|v| (v * 10.0).round() / 10.0),
        mount.width,
        mount.height,
        mount.thickness,
        mount.vertices,
        parked,
        mount.width,
        mount.height,
    ));
    let clock = hd_clock(&model, name, report);
    Placed {
        model,
        matrix,
        clock,
        cull: Some(cull),
    }
}

/// How [`place`] sets the timeline's clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ClockRule {
    /// Pulse's own, measured against the original: [`Clock::PULSE`].
    Measured,
    /// The PS3 geometry titles: the start tick off the asset's own `GO` edge
    /// as [`Clock::inherited`] gives it, then from the release
    /// HD's own race-manager window ([`Clock::hd`]), **read from HD's EBOOT
    /// and matched against its RPCS3 capture**.
    HdRaceManager,
}

/// HD's clock on `model`'s own `GO` edge, or the timeline off the race start
/// with the reason in the report.
fn hd_clock(model: &Model, name: &str, report: &mut Vec<String>) -> Clock {
    match go_edge(model).map(|edge| (edge, Clock::hd(edge))) {
        Ok((edge, Some(clock))) => {
            let window = HD_PRE_LAP_WINDOW;
            report.push(format!(
                "start gantry clock: {name}'s own GO edge is asset frame {} (the first frame the \
                 board samples its authored green), so frame 0 is tick {} and the edge would land \
                 on tick {}; from tick {} the clock is kept in [{:.2}, {:.2}) s and reset to {:.2} \
                 when outside it, the window HD's race manager (0x0005e948) holds the gantry in \
                 before the first line crossing - so GO is lit on the release and then loops \
                 every {} ticks (read from the EBOOT, confidence 85; the green step on the release \
                 is measured on RPCS3, 2026-10-04, confidence 75, and bounds the start tick from \
                 below). After the first crossing the race manager's later windows take over \
                 by lap: [6.02, 9.30) between laps, [9.50, 9.90) on the lap before the last, \
                 [12.35, 13.30) on the last (read from the EBOOT, confidence 85)",
                edge.frame,
                clock.start_tick,
                oag_race::COUNTDOWN_TICKS + 1,
                window.from_tick,
                window.from,
                window.to,
                window.from,
                window.period(),
            ));
            clock
        }
        Ok((edge, None)) => {
            report.push(format!(
                "start gantry clock: {name}'s GO edge (frame {}) is later than the thrust gate, so \
                 there is no start tick for it; the timeline runs from the race start (chosen, \
                 not measured)",
                edge.frame
            ));
            Clock::FROM_RACE_START
        }
        Err(why) => {
            report.push(format!(
                "start gantry clock: no GO edge found in {name} ({why}); the timeline runs from \
                 the race start (chosen, not measured)"
            ));
            Clock::FROM_RACE_START
        }
    }
}

/// Rewrites every draw's bounds into the space [`Placed::matrix`] actually
/// draws it in.
///
/// # Why this is needed at all, and why Pulse never hit it
///
/// [`oag_render::pvs::visible`]'s frustum test reads [`DrawCall::bounds`]
/// straight off the model and trusts it to already be in world space - true
/// for the track, whose vertices are baked into world space at build time,
/// and false for a standalone prop like this one, whose own bounds describe
/// it centred near its own origin, before [`matrix`] ever moves it. A draw
/// under an `Anim Transform` (`DrawCall::moving`) skips the frustum test
/// outright for the identical reason - its own bounds are stale the moment
/// the node animates - so this file's placement matrix is exactly one more
/// case of the same problem the `moving` flag already exists for, just
/// applied once at load rather than every frame.
///
/// **Pulse never needed this.** Every one of `321Go_StartFinish.vex`'s nine
/// `Mesh` nodes sits under its own `Anim Transform`
/// (`docs/rendering/start-gantry.md`'s node tree), so every Pulse gantry draw
/// is already `moving` and the frustum test never ran on one. HD's own
/// `321go_startfinish.vex` mixes animated and plain `Mesh` nodes - the digit
/// board and the background panel both teleport at 6.000 s and are `moving`,
/// but several smaller pieces are not - so this is the first gantry pass
/// this bug had a draw left for it to cull. Left unfixed, every non-`moving`
/// draw's local-space sphere is tested against a world-space frustum and
/// fails everywhere except the world's own origin, which reads exactly like
/// "the gantry never draws" and is not a placement bug at all.
fn place_bounds(model: &mut Model, matrix: Mat4) {
    // The placement matrix here is rotation, translation and a uniform scale
    // (`matrix`'s own doc comment) - never a shear - so one axis's length is
    // the scale every axis shares, and it is what a sphere's radius needs.
    let scale = matrix.x_axis.truncate().length();
    for draws in [
        &mut model.draws,
        &mut model.alpha_tested_draws,
        &mut model.transparent_draws,
    ] {
        for draw in draws.iter_mut() {
            draw.bounds = Bounds {
                centre: matrix
                    .transform_point3(Vec3::from(draw.bounds.centre))
                    .to_array(),
                radius: draw.bounds.radius * scale,
            };
        }
    }
}

/// Reads the gantry model, trying the manifest's spelling and then the
/// archive's.
///
/// **The two do not match on Pulse and the mismatch is the manifest's, not a
/// decode fault.** `16_Track`'s slot 8 authors
/// `/Data/Environments/321_Go/321Go_StartFinish.vex` - a leading separator and
/// forward slashes - while the WAD hashes its entries under
/// `Data\Environments\321_Go\321Go_StartFinish.vex`. HD's PSARC paths really
/// are `/`-joined, so the authored spelling is tried first and unchanged; the
/// PSP spelling is a fallback rather than a rewrite.
fn load(
    archives: &mut oag_assets::Archives,
    name: &str,
    report: &mut Vec<String>,
) -> Result<Model> {
    load_with_blob(archives, name, report).map(|(model, _)| model)
}

/// [`load`], with the file's own bytes beside the model, for a caller that
/// reads a node the model does not carry - the billboard adverts' camera.
pub(super) fn load_with_blob(
    archives: &mut oag_assets::Archives,
    name: &str,
    report: &mut Vec<String>,
) -> Result<(Model, Vec<u8>)> {
    let trimmed = name.trim_start_matches(['/', '\\']);
    let mut error = None;
    for candidate in [name, &trimmed.replace('/', "\\"), trimmed] {
        match archives.read_name(candidate) {
            Ok(blob) => {
                let model = build(archives, name, candidate, &blob, report)?;
                return Ok((model, blob));
            }
            Err(e) => error = error.or(Some(e)),
        }
    }
    Err(error.map_or_else(|| anyhow::anyhow!("{name} is in no archive"), Into::into))
}

/// Builds the model once its blob is in hand - the PSP/PS2 path unchanged,
/// and HD's own route through the `.rcsmodel` beside it.
///
/// **`321Go_StartFinish.vex` is shaped like a craft, not like the track.**
/// Its own `Mesh` payloads are a bounding box and a hash - see
/// `docs/rendering/start-gantry.md`'s "The geometry moved out of the `.vex`" -
/// so [`mesh::build_with_textures`] bails on it outright
/// (`"a PS3 .vex carries no render geometry"`). [`mesh::rcs::build`] is the
/// same function a ship's own livery loads through
/// (`oag_livery`/`livery::flare`), not [`mesh::rcs::build_scene`]:
/// the gantry has no world-baked second pass of its own to catch a
/// wrongly-skipped part, the same reasoning that function's own doc comment
/// gives for every non-track caller.
fn build(
    archives: &mut oag_assets::Archives,
    name: &str,
    candidate: &str,
    blob: &[u8],
    report: &mut Vec<String>,
) -> Result<Model> {
    if !mesh::geometry_is_external(blob) {
        let mut model = mesh::build_with_textures(name, blob, None)?;
        // The same PS2 signature `livery::shield_model` and `livery::plume`
        // both gate on: every `Texture` node present and none decoded,
        // because a PS2 model's texture block is empty by design and its
        // pixels are the archive entry directly before it (`ps2-texture.md`).
        // `321Go_StartFinish.vex` is 74,944 bytes on the PS2 disc against
        // 54,032 on PSP - a different file, not a missing one - and it takes
        // this branch the same way the shield and the plume do; a PSP/PS3
        // gantry has its textures embedded and never enters it.
        if !model.textures.is_empty() && model.textures.iter().all(Option::is_none) {
            ps2_skin(archives, name, candidate, blob, &mut model, report);
        }
        return Ok(model);
    }
    let sibling = mesh::rcs::sibling_name(candidate)
        .with_context(|| format!("{name}: a PS3 .vex with no .rcsmodel spelling"))?;
    let geometry = archives
        .read_name(&sibling)
        .with_context(|| format!("{name}: the .rcsmodel beside it, {sibling}"))?;
    let (model, _report) = mesh::rcs::build(
        name,
        blob,
        &geometry,
        &mut |path| archives.read_name(path).ok(),
        |c| c.mesh,
    )?;
    Ok(model)
}

/// Re-skins a PS2 gantry model from the texture set in the archive entry
/// before it - the same directory-position rule `livery::ps2_skin` applies to
/// the hull, the plume and the shield models, reimplemented here rather than
/// exported from `livery` since the two live in different modules and the
/// rule itself is generic (`ps2_texture_set`, `docs/formats/ps2-texture.md`).
fn ps2_skin(
    archives: &mut oag_assets::Archives,
    name: &str,
    candidate: &str,
    blob: &[u8],
    model: &mut Model,
    report: &mut Vec<String>,
) {
    let slots = model.textures.len();
    let Some(external) = oag_livery::entry::ps2_texture_set(archives, candidate) else {
        report.push(format!(
            "{name}: {slots} empty texture slot(s) and the preceding archive entry is \
             not a texture set - the gantry draws untextured"
        ));
        return;
    };
    let found = external.entry_count();
    let decoded = external.decoded_count();
    match mesh::build_with_textures(name, blob, Some(&external)) {
        Ok(rebuilt) => {
            *model = rebuilt;
            report.push(format!(
                "{name}: {decoded} of {found} texture(s) from the preceding archive entry, \
                 into {slots} slot(s)"
            ));
        }
        Err(error) => report.push(format!(
            "{name}: the preceding archive entry holds {found} texture(s) but re-skinning \
             failed ({error}) - the gantry draws untextured"
        )),
    }
}

/// How far in front of the mount the gantry stands, in track units.
///
/// **Chosen, not measured** - no confidence score, because nothing on the disc
/// says it. What the disc does say is that the mounting surface is a *backing*
/// panel: `321backplate.tga` and the 8x8 `billboard8.tga` stub are two
/// co-located surfaces on one node, and on eleven of the twelve circuits that
/// author both they sit **0.48 to 1.63 units apart** on the same plane
/// (`gantry_mount_ground_truth`). So the artists' own step between stacked
/// surfaces there is about a unit, and this is that order.
///
/// It is not zero because it cannot be: standing the model exactly on the
/// panel makes its board coplanar with the track's own, and the earlier
/// surface wins a `Less` depth test at every distance - which is not a
/// subtlety, it is the whole gantry invisible with the placeholder showing
/// through in its place. That was the first thing this drew.
const CLEARANCE: f32 = 1.0;

/// The model matrix that stands the gantry on `mount` for a race running along
/// `forward`.
///
/// Three decisions, each from a measurement:
///
/// 1. **The board faces the grid, so it faces `-forward`.** The mount sits
///    163-174 units *ahead* of the `Start Position` on all twelve circuits that
///    author one (`gantry_mount_ground_truth`, `ahead-ness` +0.99 to +1.00), so
///    the craft approach it from behind and the side they see is the one facing
///    back down the track.
/// 2. **The model's own board faces its `+Z`.** `start_light_321goShape`'s
///    plane normal is `(0, 0, 1)` and its authored vertex normals agree - the
///    same test's first block.
/// 3. **Scale 1.0, and it is measured rather than assumed.** The gantry's
///    countdown board - `start_light_321goShape`, the one the countdown is
///    actually on - is 34.02 units across, and the panel the track authors is
///    36.30 to 45.50 wide over the twelve circuits that author one. So the
///    board fits its mount at 1:1 on every circuit, and
///    `gantry_mount_ground_truth` asserts exactly that rather than leaving it
///    as a remark. No factor is fitted to improve on it.
///
///    **Not the model's widest piece.** `polySurfaceShape7`, the chequered
///    state, is 43.25 across and overhangs `13_Track`'s 36.30 panel - but that
///    is one of the pieces [`oag_render::gantry::clip_to_panel`] drops, so it
///    never reaches the screen. The claim is about the board being drawn, not
///    about the file's bounding box.
///
///    **HD's own board and mount fit at 1.0 too, measured separately rather
///    than inherited from Pulse's numbers.** `pasted__Go_HD_start_light_321go`
///    spans `x` -16.60..16.59 (33.2 units) against a mount measured 42.4-46.6
///    wide on four HD circuits (Talon's Junction, Amphiseum, `01_Vineta_K`,
///    `Tech_De_Ra`) - a different asset on a different mount, fitting with
///    room to spare rather than by the same 1:1 coincidence Pulse's own
///    numbers show. **HD's mount is not the flat stub Pulse's is, though**:
///    its own [`Mount::thickness`] measures 1.9-3.0 across those four
///    circuits, against Pulse's 0.000-0.100 - the `billboard8.gtf`-bound chunk
///    is part of a real 3D structure (a boost-gate frame on Talon's Junction),
///    not a thin placeholder quad, so [`CLEARANCE`] clears the digit board's
///    own near-zero local depth but not a backing piece authored several
///    units further into that structure - see `docs/rendering/start-gantry.md`'s
///    "Implemented on HD" section.
fn matrix(mount: &Mount, forward: Vec3) -> Mat4 {
    let facing = mount.facing(-forward);
    let mut placed = mount.matrix(-forward, Vec3::Z, 1.0);
    placed.w_axis += (facing * CLEARANCE).extend(0.0);
    placed
}

/// `clock`'s time at this tick of `race`: [`Clock::seconds`] before the player's
/// first line crossing, then the race-manager window the lap picks
/// ([`BoardWindow::of`]), entered on the crossing that picked it.
fn race_clock(clock: &Clock, tracker: &std::cell::Cell<BoardTracker>, race: &Race) -> f32 {
    let tick = race.sim.world.tick;
    let standing = race.player_standing();
    let Some(laps) = clock.laps else {
        return clock.seconds(tick);
    };
    let window = BoardWindow::of(standing, race.sim.world.laps_target(), &laps);
    let mut state = tracker.get();
    let entry = state.observe(tick, window, standing.lap_start_tick);
    tracker.set(state);
    clock.seconds_in(tick, entry)
}

/// What drives a gantry drawn as a card ([`place_card`]): the same clock, the
/// same per-window cull as [`Gantry`], on the card's own drawable.
#[derive(Debug)]
pub struct CardTimeline {
    clock: Clock,
    cull: PanelCull,
    tracker: std::cell::Cell<BoardTracker>,
}

impl CardTimeline {
    /// The clock the loader chose for it.
    #[must_use]
    pub fn clock(&self) -> Clock {
        self.clock
    }

    /// The per-moment cull of the board's later states.
    #[must_use]
    pub fn cull(&self) -> &PanelCull {
        &self.cull
    }

    /// This timeline's clock now: [`race_clock`], or `anim_seconds` where a
    /// capture pins one.
    pub(super) fn seconds(&self, race: &Race, anim_seconds: Option<f32>) -> f32 {
        anim_seconds.unwrap_or_else(|| race_clock(&self.clock, &self.tracker, race))
    }

    /// Sets `drawable`'s hidden draws for `clock` and returns the time its
    /// animation tables are written at.
    pub(super) fn apply(&self, drawable: &Drawable, clock: f32) -> f32 {
        if self.clock.window.is_some() {
            drawable.set_hidden(self.cull.hidden(clock));
            clock
        } else {
            clock.min(CLOCK_LIMIT)
        }
    }
}

/// The gantry on the GPU: one drawable and the matrix it is drawn at.
#[derive(Debug)]
pub(super) struct Gantry {
    drawable: Drawable,
    matrix: Mat4,
    clock: Clock,
    cull: Option<PanelCull>,
    tracker: std::cell::Cell<BoardTracker>,
}

impl Gantry {
    /// Builds the pipelines for an already-placed gantry.
    ///
    /// # Errors
    ///
    /// Propagates a pipeline or geometry upload failure, the same as every
    /// other [`Drawable`].
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        placed: Placed,
        format: wgpu::TextureFormat,
        anisotropy: Anisotropy,
        sample_count: u32,
        shadow_maps: mesh_render::ShadowMaps<'_>,
    ) -> Result<Self> {
        let drawable = Drawable::new(
            device,
            queue,
            placed.model,
            format,
            anisotropy,
            sample_count,
            mesh_render::Depth::Scene,
            mesh_render::TRANSPARENT_BLEND,
            mesh_render::GlowMask::Protected,
            &mesh_render::zone::StageArt::NONE,
            shadow_maps,
            // Scenery, like the track: it stands on the road and the road's
            // own shadow tiers reach it.
            mesh_render::ShadowReceiver::Both,
            mesh_render::Velocity::Write,
        )?;
        Ok(Self {
            drawable,
            matrix: placed.matrix,
            clock: placed.clock,
            cull: placed.cull,
            tracker: std::cell::Cell::default(),
        })
    }

    /// This gantry's timeline clock now: [`Clock::seconds`] before the
    /// player's first line crossing, then the race-manager window its lap
    /// picks ([`BoardWindow::of`]), entered on the crossing that picked it.
    pub(super) fn clock_seconds(&self, race: &Race) -> f32 {
        race_clock(&self.clock, &self.tracker, race)
    }

    /// The fog block this frame, shared with the rest of the scenery.
    pub(super) fn fog(&self) -> &wgpu::Buffer {
        &self.drawable.fog
    }

    /// Writes the frame's matrices and both animation tables.
    ///
    /// `seconds` is the gantry's own clock ([`clock_seconds`]), and it is clamped
    /// at [`CLOCK_LIMIT`] here rather than by the caller so there is one place
    /// the decision lives - on a clock with no race-manager window. HD's
    /// reaches its later states on purpose, and draws only what stands on the
    /// panel at `seconds` ([`PanelCull`]).
    pub(super) fn write(
        &self,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        previous_view_projection: Mat4,
        seconds: f32,
    ) {
        self.drawable.write(
            queue,
            view_projection,
            self.matrix,
            previous_view_projection,
        );
        // The gantry is static, so its previous model matrix is this one: the
        // velocity buffer sees only the camera's own motion across it.
        let seconds = match &self.cull {
            Some(cull) if self.clock.window.is_some() => {
                self.drawable.set_hidden(cull.hidden(seconds));
                seconds
            }
            _ => seconds.min(CLOCK_LIMIT),
        };
        self.drawable.write_anims(queue, seconds);
        self.drawable.write_node_anims(queue, seconds);
    }

    /// Draws it, frustum-culled and nothing else.
    ///
    /// No PVS: the gantry is not in the track file, so it carries no `section`
    /// id to look one up with - the same exemption the speed pads take.
    pub(super) fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        frustum: Option<&oag_core::math::frustum::Frustum>,
    ) -> SceneStats {
        self.drawable.draw(pass, None, None, None, frustum)
    }
}
