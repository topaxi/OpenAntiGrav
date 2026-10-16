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
use oag_hud::{Context, Placement, Readout, Span};
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
    let (hud_x, hud_y, per_x, per_y) = grid_of(viewport, space);
    let (x, y, w, h) = view;
    let (region_w, region_h) = (region[2] * per_x, region[3] * per_y);
    let scale = (w / region_w).min(h / region_h);
    // Centred in the view, then back from pixels to the grid the draws are in.
    let origin_x = x + (w - region_w * scale) * 0.5;
    let origin_y = y + (h - region_h * scale) * 0.5;
    Placement {
        scale,
        offset: [
            (origin_x - hud_x) / per_x - scale * region[0],
            (origin_y - hud_y) / per_y - scale * region[1],
        ],
        clip: [
            (x - hud_x) / per_x,
            (y - hud_y) / per_y,
            w / per_x,
            h / per_y,
        ],
    }
}

/// Where the HUD's grid sits in the target: the pixel position of its origin and
/// how many pixels one grid unit is across and down.
///
/// **The two differ for a grid shown at another shape than its own**: the PS2's 640x448
/// is shown 16:9, so a unit is 2.0 pixels wide and 1.607 tall at 1280x720. Every
/// conversion between pixels and the grid takes the axis it is on; one number for both
/// made a PS2 view 20% short in grid rows, and its bottom-anchored groups floated at
/// about 76% of the view.
fn grid_of(
    viewport: (f32, f32, f32, f32),
    space: oag_display::space::Space,
) -> (f32, f32, f32, f32) {
    let (left, top, width, height) = viewport;
    let [fit_x, fit_y] =
        crate::render::letterbox_in((width as u32, height as u32), space.display_aspect);
    // The whole-screen HUD's rectangle in pixels, and its pixels per grid unit.
    let (hud_w, hud_h) = (width * fit_x, height * fit_y);
    let per_x = hud_w / space.size.0;
    // A grid shown as itself has square units; take the one number for both axes there, so
    // two roundings of the same ratio cannot move a pixel on the 16:9 grids.
    let square = (space.display_aspect - space.size.0 / space.size.1).abs() < 1e-3;
    let per_y = if square { per_x } else { hud_h / space.size.1 };
    (
        left + (width - hud_w) * 0.5,
        top + (height - hud_h) * 0.5,
        per_x,
        per_y,
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
    let (hud_x, hud_y, per_x, per_y) = grid_of(viewport, space);
    let edge = |low: f32, high: f32, from: f32, to: f32| {
        let start = (low.round()).clamp(from, to);
        let end = (high.round()).clamp(from, to);
        [start as u32, (end - start).max(0.0) as u32]
    };
    let [x, w] = edge(
        hud_x + grid[0] * per_x,
        hud_x + (grid[0] + grid[2]) * per_x,
        viewport.0,
        viewport.0 + viewport.2,
    );
    let [y, h] = edge(
        hud_y + grid[1] * per_y,
        hud_y + (grid[1] + grid[3]) * per_y,
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
    let (hud_x, hud_y, per_x, per_y) = grid_of(viewport, space);
    [
        (view.0 - hud_x) / per_x,
        (view.1 - hud_y) / per_y,
        view.2 / per_x,
        view.3 / per_y,
    ]
}

/// Where each of `count` views' whole-screen HUD goes: [`place_into`] with the
/// whole screen as the region, for each of `oag_render::viewport::layout`'s views.
#[must_use]
pub fn placements(
    count: usize,
    cut: oag_render::viewport::Cut,
    viewport: (f32, f32, f32, f32),
    space: oag_display::space::Space,
) -> Vec<Placement> {
    let whole = [0.0, 0.0, space.size.0, space.size.1];
    oag_render::viewport::layout(count, viewport, cut)
        .into_iter()
        .map(|view| place_into(view, whole, viewport, space))
        .collect()
}

/// Every view's draw: `readouts[i]` is slot `i`'s, drawn into view `i`.
///
/// Which layout a view draws is `oag_hud::split_hud::source`'s answer for its
/// shape: a player's override file, the title's authored layout (two views, one per
/// player, the family of `cut`: `split` for rows, `split_columns` for a vertical cut, whole-screen and sitting in its half, its reticle view-relative), this
/// project's split-HUD file, or the whole-screen layout `cx` carries scaled into
/// the view. A file places each of its groups of widgets on its own, so it yields
/// one draw per group and one more for the reticle.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn view_draws<'a>(
    cx: Context<'a>,
    split: Option<&'a oag_hud::SplitAssets>,
    split_columns: Option<&'a oag_hud::SplitAssets>,
    split_hud: Option<&'a oag_hud::split_hud::SplitHudAssets>,
    readouts: Vec<Readout>,
    cut: oag_render::viewport::Cut,
    viewport: (f32, f32, f32, f32),
    space: oag_display::space::Space,
) -> Vec<ViewDraw<'a>> {
    let count = readouts.len();
    let views = oag_render::viewport::layout(count, viewport, cut);
    let whole = [0.0, 0.0, space.size.0, space.size.1];
    // **The window's cut picks the family**, not the load's: a `2h,2v` race draws the rows
    // family in one window and the `vert_` columns in the other.
    let split = oag_hud::split_family(
        split,
        split_columns,
        cut == oag_render::viewport::Cut::Vertical,
    );
    let mut out = Vec::new();
    for (slot, (mut readout, view)) in readouts.into_iter().zip(views).enumerate() {
        let aspect = view.2 / view.3;
        // Two views cut top and bottom are the original's `Split ScreenH`, whose Head to Head
        // bar is shorter and whose leader row sits on the bar's top.
        if let Some(gap) = readout.head_to_head.as_mut() {
            gap.stacked = count == 2 && cut == oag_render::viewport::Cut::Horizontal;
        }
        match oag_hud::split_hud::source(split, split_hud, readout.mode, count, aspect) {
            oag_hud::split_hud::Source::Authored(split) => {
                let layout = &split.layouts[slot];
                let (width, height) = split.view;
                let region = split.region(slot);
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
pub fn panel_viewport(
    count: usize,
    cut: oag_render::viewport::Cut,
    viewport: (f32, f32, f32, f32),
) -> (f32, f32, f32, f32) {
    oag_render::viewport::layout(count, viewport, cut)[0]
}

/// How tall a countdown digit is, as a fraction of the HUD's grid height fitted into
/// the view. **Chosen**, to read at a glance across a short view.
const COUNTDOWN_HEIGHT: f32 = 0.3;

/// The resume countdown's word as text in the title's `HUD` face, one per view,
/// each centred across its view and `crate::pause::anchor_y(count)` of the way down
/// it, clipped to it. **Chosen, not measured**: where a title authors no countdown
/// art (`oag_title::hud::ResumeCountdown::Text`).
///
/// Sized from the whole-screen HUD fitted into the view ([`place_into`]'s scale), so
/// a view shows the same-sized digit whatever its shape; placed from the view itself,
/// so a tall narrow view's digit is not pushed off its fit region's letterbox. A
/// [`Frame`] to be [`Frame::extend`]ed onto the HUD's own, never painted alone.
#[must_use]
pub fn countdown_frame(
    cx: &Context<'_>,
    text: &str,
    count: usize,
    cut: oag_render::viewport::Cut,
    viewport: (f32, f32, f32, f32),
    space: oag_display::space::Space,
) -> oag_hud::Frame {
    let whole = [0.0, 0.0, space.size.0, space.size.1];
    let mut frame = oag_hud::Frame::default();
    for view in oag_render::viewport::layout(count, viewport, cut) {
        let fit = place_into(view, whole, viewport, space).scale;
        let scale = fit * COUNTDOWN_HEIGHT * space.size.1 / cx.hud_line_height;
        let [x, y, width, height] = grid_rect(view, viewport, space);
        frame.hud_text.push(oag_ui::frontend::Draw::Text {
            x: x + width * 0.5,
            y: y + height * crate::pause::anchor_y(count) - cx.hud_line_height * scale * 0.5,
            scale,
            color: [1.0, 1.0, 1.0, 1.0],
            border: Some(cx.default_border),
            align: oag_ui::frontend::Align::Centre,
            text: text.to_string(),
            wrap_width: None,
        });
        frame.spans.push(Span {
            clip: [x, y, width, height],
            start: [frame.sprites.len(), frame.hud_text.len() - 1, 0, 0],
        });
    }
    frame
}

/// Draws the race HUD over `view`: one HUD, or one per view of a split screen.
///
/// With one view this is `overlay.draw` on the slot's readout, untouched. **In a multi-window
/// race** ([`Race::windowed`]) `window` is the window being drawn: its views
/// ([`Race::window_drawn`]) are one whole-screen HUD for a window of one seat, and a split's
/// HUD - the layout rules picked by the window's own view count and cut - for a window of
/// several. Otherwise `window` is not read. `countdown` is the resume countdown's word when
/// the title draws it as text ([`countdown_frame`]), one per view this window cuts; `None`
/// adds nothing, so a frame without one is the HUD as it always was.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    overlay: &mut crate::hud_overlay::Overlay,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    race: &Race,
    scene: &Scene,
    window: usize,
    campaign: Option<&oag_tables::race_campaign::Cell>,
    record_target: Option<&oag_hud::RecordTarget>,
    countdown: Option<&str>,
    viewport: (f32, f32, f32, f32),
) {
    let (slots, cut) = if race.windowed() {
        (race.window_drawn(window), race.window_cut(window))
    } else if race.viewport_count() <= 1 {
        (vec![race.player_slot()], race.cut())
    } else {
        ((0..race.viewport_count()).collect(), race.cut())
    };
    if let [slot] = slots[..] {
        let readout = readout(race, scene, slot, campaign, record_target);
        overlay.draw(device, queue, encoder, view, &readout, countdown, viewport);
        return;
    }
    let readouts: Vec<Readout> = slots
        .iter()
        .map(|&slot| readout(race, scene, slot, campaign, record_target))
        .collect();
    overlay.draw_views(
        device, queue, encoder, view, readouts, countdown, cut, viewport,
    );
}

#[cfg(test)]
mod tests;
