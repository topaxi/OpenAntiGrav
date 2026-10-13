//! The race HUD of a whole screen: one HUD, or one per split-screen view.
//!
//! Two callers draw the race HUD - the window's frame loop
//! (`RaceStage::draw_hud`) and the headless capture (`race_capture`) - and both
//! used to build the readout and fix up the fields the sim does not know
//! (`zone_stage`, the pace tier) by hand. [`readout`] is that fix-up once, for a
//! slot, and [`draw`] is the draw: with one view exactly what each did, with a
//! split one HUD per view.
//!
//! # A split screen's HUD (chosen, not measured)
//!
//! No capture of the original's split screen has been read, so how a view's HUD
//! sits in its rectangle is **chosen, not measured**: the title's whole-screen
//! layout, scaled uniformly to fit the view with its aspect kept and centred
//! ([`placements`]). View `i` shows slot `i`'s craft - its lap, place, times,
//! speed, shield, held weapon and lock - and draws nothing it cannot read for
//! that slot (the message lines and split gap, see
//! `oag_raceplay::Race::readout_for`). All the views are painted in one list so
//! the HUD renderer's buffers are written once.

use crate::hud_overlay::ViewDraw;
use oag_hud::{Context, Placement, Readout};
use oag_raceplay::{Race, Scene};

/// The HUD readout of the view that follows `slot`: the race's own
/// [`Race::readout_for`] plus the three things it cannot know, which live on the
/// scene and the stage.
///
/// `campaign` and `record_target` are the stage's: a campaign cell races its
/// own ladder and any other Time Trial or Speed Lap races the record. See
/// `oag_hud::Readout::time_trial_pace`. `campaign_cell` is title-blind (HD
/// reuses the same `oag_tables::race_campaign::Cell`), so nothing here stops it
/// computing on HD - what keeps this Pulse-only is `record_target` (`stats.xml`
/// is read off Pulse's PSP disc alone) and `draw.rs`'s widget-name match: no
/// shipped HD layout authors a widget named `TotalTime`/`TotalTimeTxt`. The
/// capture path has no cell and passes `None`.
#[must_use]
pub fn readout(
    race: &Race,
    scene: &Scene,
    slot: usize,
    campaign: Option<&oag_tables::race_campaign::Cell>,
    record_target: Option<&oag_hud::RecordTarget>,
) -> Readout {
    let mut readout = race.readout_for(slot);
    // The rung the grade is showing, which is what names the speed class - see
    // `Scene::zone_stage`. One grade per scene, so every view reads the same
    // rung (open for a Zone race in a split).
    readout.zone_stage = scene.zone_stage().unwrap_or(0);
    readout.zone_next_in =
        scene.zones_to_next_stage(u16::try_from(readout.zone).unwrap_or(u16::MAX));
    readout.time_trial_pace = oag_hud::pace_for(
        readout.mode,
        readout.race_ticks,
        readout.lap_ticks,
        campaign,
        record_target,
    );
    readout
}

/// Where a layout's `region` goes inside the view rectangle `view`, for a HUD
/// that is drawn into `viewport` and authored in `space`: the region scaled
/// uniformly to the largest size that fits the view, aspect kept, and centred.
///
/// `region` is `[x, y, width, height]` in the layout's grid: the whole screen for
/// a whole-screen layout, one player's half of it for an authored split layout.
/// What lands outside the view is clipped away ([`Placement::clip`]).
#[must_use]
pub fn place_into(
    view: oag_render::viewport::Rect,
    region: [f32; 4],
    viewport: (f32, f32, f32, f32),
    space: oag_display::space::Space,
) -> Placement {
    let (hud_x, hud_y, pixels_per_grid) = grid_of(viewport, space);
    let (x, y, w, h) = view;
    let (region_w, region_h) = (region[2] * pixels_per_grid, region[3] * pixels_per_grid);
    let scale = (w / region_w).min(h / region_h);
    // Centred in the view, then back from pixels to the grid the draws are in.
    let origin_x = x + (w - region_w * scale) * 0.5;
    let origin_y = y + (h - region_h * scale) * 0.5;
    Placement {
        scale,
        offset: [
            (origin_x - hud_x) / pixels_per_grid - scale * region[0],
            (origin_y - hud_y) / pixels_per_grid - scale * region[1],
        ],
        clip: [
            (x - hud_x) / pixels_per_grid,
            (y - hud_y) / pixels_per_grid,
            w / pixels_per_grid,
            h / pixels_per_grid,
        ],
    }
}

/// Where the HUD's grid sits in the target: the pixel position of its origin and
/// how many pixels one grid unit is.
fn grid_of(viewport: (f32, f32, f32, f32), space: oag_display::space::Space) -> (f32, f32, f32) {
    let (left, top, width, height) = viewport;
    let [fit_x, fit_y] =
        crate::render::letterbox_in((width as u32, height as u32), space.display_aspect);
    // The whole-screen HUD's rectangle in pixels, and its pixels per grid unit.
    let (hud_w, hud_h) = (width * fit_x, height * fit_y);
    (
        left + (width - hud_w) * 0.5,
        top + (height - hud_h) * 0.5,
        hud_w / space.size.0,
    )
}

/// A rectangle of the HUD's grid as the whole target pixels `set_scissor_rect`
/// takes, `[x, y, width, height]`, kept inside `viewport`.
#[must_use]
pub fn scissor_rect(
    grid: [f32; 4],
    viewport: (f32, f32, f32, f32),
    space: oag_display::space::Space,
) -> [u32; 4] {
    let (hud_x, hud_y, per) = grid_of(viewport, space);
    let edge = |low: f32, high: f32, from: f32, to: f32| {
        let start = (low.round()).clamp(from, to);
        let end = (high.round()).clamp(from, to);
        [start as u32, (end - start).max(0.0) as u32]
    };
    let [x, w] = edge(
        hud_x + grid[0] * per,
        hud_x + (grid[0] + grid[2]) * per,
        viewport.0,
        viewport.0 + viewport.2,
    );
    let [y, h] = edge(
        hud_y + grid[1] * per,
        hud_y + (grid[1] + grid[3]) * per,
        viewport.1,
        viewport.1 + viewport.3,
    );
    [x, y, w, h]
}

/// A view's rectangle in the HUD's grid, `[x, y, width, height]`.
fn grid_rect(
    view: oag_render::viewport::Rect,
    viewport: (f32, f32, f32, f32),
    space: oag_display::space::Space,
) -> [f32; 4] {
    let (hud_x, hud_y, per) = grid_of(viewport, space);
    [
        (view.0 - hud_x) / per,
        (view.1 - hud_y) / per,
        view.2 / per,
        view.3 / per,
    ]
}

/// Where each of `count` views' whole-screen HUD goes: [`place_into`] with the
/// whole screen as the region, for each of `oag_render::viewport::layout`'s views.
#[must_use]
pub fn placements(
    count: usize,
    viewport: (f32, f32, f32, f32),
    space: oag_display::space::Space,
) -> Vec<Placement> {
    let whole = [0.0, 0.0, space.size.0, space.size.1];
    oag_render::viewport::layout(count, viewport)
        .into_iter()
        .map(|view| place_into(view, whole, viewport, space))
        .collect()
}

/// Every view's draw: `readouts[i]` is slot `i`'s, drawn into view `i`.
///
/// Which layout a view draws is `oag_hud::split_hud::source`'s answer for its
/// shape: a player's override file, the title's authored layout (two views, one per
/// player, whole-screen and sitting in its half, its reticle view-relative), this
/// project's split-HUD file, or the whole-screen layout `cx` carries scaled into
/// the view. A file places each of its groups of widgets on its own, so it yields
/// one draw per group and one more for the reticle.
#[must_use]
pub fn view_draws<'a>(
    cx: Context<'a>,
    split: Option<&'a oag_hud::SplitAssets>,
    split_hud: Option<&'a oag_hud::split_hud::SplitHudAssets>,
    readouts: Vec<Readout>,
    viewport: (f32, f32, f32, f32),
    space: oag_display::space::Space,
) -> Vec<ViewDraw<'a>> {
    let count = readouts.len();
    let views = oag_render::viewport::layout(count, viewport);
    let whole = [0.0, 0.0, space.size.0, space.size.1];
    let mut out = Vec::new();
    for (slot, (readout, view)) in readouts.into_iter().zip(views).enumerate() {
        let aspect = view.2 / view.3;
        match oag_hud::split_hud::source(split, split_hud, readout.mode, count, aspect) {
            oag_hud::split_hud::Source::Authored(split) => {
                let layout = &split.layouts[slot];
                let (width, height) = split.view;
                let region = [0.0, height * slot as f32, width, height];
                out.push(ViewDraw {
                    cx: Context {
                        layout,
                        default_border: layout.default_border(),
                        ..cx
                    },
                    readout,
                    placement: place_into(view, region, viewport, space),
                    sight: Some(place_into(view, [0.0, 0.0, width, height], viewport, space)),
                    sight_only: false,
                });
            }
            oag_hud::split_hud::Source::File(_, pieces, _) => {
                let rect = grid_rect(view, viewport, space);
                for piece in pieces {
                    out.push(ViewDraw {
                        cx: Context {
                            layout: &piece.layout,
                            ..cx
                        },
                        readout: Readout {
                            sight: None,
                            ..readout.clone()
                        },
                        placement: piece.placement(rect, space.size),
                        sight: None,
                        sight_only: false,
                    });
                }
                out.push(ViewDraw {
                    cx,
                    placement: place_into(view, whole, viewport, space),
                    sight: Some(place_into(view, whole, viewport, space)),
                    readout,
                    sight_only: true,
                });
            }
            oag_hud::split_hud::Source::Scaled => out.push(ViewDraw {
                cx,
                readout,
                placement: place_into(view, whole, viewport, space),
                sight: None,
                sight_only: false,
            }),
        }
    }
    out
}

/// Where the track-description panel goes: the whole `viewport`, or under a split
/// the first view's rectangle, which is the one that flies the intro. Chosen,
/// not measured.
#[must_use]
pub fn panel_viewport(count: usize, viewport: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    oag_render::viewport::layout(count, viewport)[0]
}

/// Draws the race HUD over `view`: one HUD, or one per view of a split screen.
///
/// With one view this is `overlay.draw` on the slot's readout, untouched.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    overlay: &mut crate::hud_overlay::Overlay,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    race: &Race,
    scene: &Scene,
    campaign: Option<&oag_tables::race_campaign::Cell>,
    record_target: Option<&oag_hud::RecordTarget>,
    viewport: (f32, f32, f32, f32),
) {
    let count = race.viewport_count();
    if count <= 1 {
        let readout = readout(race, scene, race.player_slot(), campaign, record_target);
        overlay.draw(device, queue, encoder, view, &readout, viewport);
        return;
    }
    let readouts: Vec<Readout> = (0..count)
        .map(|slot| readout(race, scene, slot, campaign, record_target))
        .collect();
    overlay.draw_views(device, queue, encoder, view, readouts, viewport);
}

#[cfg(test)]
mod tests;
