//! The 3-D flyer card behind HD/Fury's `Grid Selection`: decoded off the disc
//! once, put on the GPU the first time a grid shows it, and drawn by the same
//! [`crate::preview::Preview`] pass a circuit's outline ribbon uses.
//!
//! Nothing here is a second mesh-in-menu mechanism. [`crate::preview::model`]
//! learned the PS3 `.vex` + `.rcsmodel` pair (it could only read a PSP or PS2
//! `.vex` before, which is why HD's own `preview_meshes` is `false`), and
//! [`crate::preview::Preview::draw_matrices`] is `draw_mode3d`'s second half
//! with the camera passed in. This module adds the two things specific to a
//! flyer: which entries to read ([`oag_ui::campaign::flyer`]) and the camera
//! ([`flyer_view_projection`]).
//!
//! # Authored versus chosen
//!
//! Authored, off `CellMode_Definition.xml` and the executable's own format
//! strings: which model (`Data/FE/Flyers/<FlyerName>/flyer.vex`), the widget's
//! position `x y z`, its `OriginX`/`OriginY`, `nearZ`/`farZ`,
//! `RotationCentreOffsetX` as a pivot, and the materials and textures, which
//! come from the disc's own `.rcsmaterial`s (`basicnonalpha`, `basicalpha`,
//! `scrollingalpha` - three unlit programs, no light enters any of them).
//!
//! **Chosen, not measured**: the field of view, and the settled yaw. The
//! widget's own `RotY="1.5"` is not a settled pose (as radians it is an
//! 86 degree turn, nearly edge-on, which is what RPCS3 shows mid-transition),
//! and the native `Flyer` class that would say what it means is unread. Both
//! constants below were fitted to settled RPCS3 frames of `Grid Selection`,
//! which makes them a match to a picture rather than a reading of the
//! executable.
//!
//! **Not drawn**: the glow around an unlocked card, the floor reflection
//! under it, the flip to `flyer_back.vex`, and the elements animating in on a
//! page change. A card is drawn flat-lit, static and settled.

use std::cell::RefCell;
use std::collections::HashMap;

use oag_core::math::{Mat4, Vec3, camera};
use oag_display::space::Space;
use oag_render::mesh::Model;
use oag_render::mesh_render::Anisotropy;
use oag_ui::campaign::flyer::{self, FlyerWidget};
use oag_ui::frontend::Draw;

use crate::preview::Preview;
use crate::render::Renderer;

mod clip;

/// The vertical field of view, radians. **Chosen, not measured** - see the
/// module doc.
pub const CHOSEN_FOV_Y: f32 = 0.545;

/// The settled yaw, radians. **Chosen, not measured.** Negative turns the
/// card's right edge toward the camera, which is how every settled RPCS3 frame
/// shows it (the right edge is the taller one).
pub const CHOSEN_SETTLED_YAW: f32 = -0.37;

/// Where the card's centre sits in camera space, `(x right, y up, z)`, with
/// `z` the authored `-200`. **Chosen, not measured.** The camera only pins
/// the ratio of field of view to distance, so the authored depth is kept and
/// the field of view fitted to it; `x` is the card's offset from the optical
/// axis, and `y` is zero - the card is centred on the screen's vertical
/// middle in every reference frame, where the authored `y="-33.3"` would put
/// it a third of a card below.
pub const CHOSEN_CENTRE: [f32; 3] = [20.6, 0.0, -200.0];

/// The camera and model matrices for a flyer widget on `space`'s grid.
///
/// - **Projection**: a symmetric perspective frustum at [`CHOSEN_FOV_Y`] and
///   the display's own aspect, with the widget's authored `nearZ`/`farZ`.
/// - **`OriginX`/`OriginY`**: the screen point the optical axis passes
///   through, as a post-projection shift from the grid's centre. On `Grid
///   Selection` that is `(960, 540)`, the centre of the 1920 by 1080 grid, so
///   no shift at all; `Campaign Selection`'s pair author `(640, 450)` and
///   `(1280, 450)`. **Chosen, not measured**: that these are absolute
///   positions rather than offsets as `Mode3D`'s are - the two readings
///   agree on `Grid Selection`'s own numbers.
/// - **Model**: the card's centre at [`CHOSEN_CENTRE`], turned by the
///   widget's authored `RotX` (`0.0`) and `yaw`.
///
/// **The widget's own `x y z`, `RotY` and `RotationCentreOffsetX` are read and
/// not applied.** Applied as written (a card at `(80, -33.3, -200)` turned
/// `1.5` radians about a pivot 60 units left of it) they give a near edge-on
/// card well off the picture, and no settled frame looks like that; see the
/// module doc. The numbers [`CHOSEN_CENTRE`] and [`CHOSEN_SETTLED_YAW`] stand
/// where they would be, fitted to a planar homography between a head-on render
/// of the card and two settled RPCS3 frames (`docs/ui/campaign-screens.md`).
#[must_use]
pub fn flyer_view_projection(widget: &FlyerWidget, space: Space, yaw: f32) -> (Mat4, Mat4) {
    let [near, far] = widget.depth;
    let projection = camera::perspective(CHOSEN_FOV_Y, space.display_aspect, near, far);
    let shift = Vec3::new(
        2.0 * widget.origin[0] / space.size.0 - 1.0,
        1.0 - 2.0 * widget.origin[1] / space.size.1,
        0.0,
    );
    let view_projection = Mat4::from_translation(shift) * projection;
    let model = Mat4::from_translation(Vec3::from(CHOSEN_CENTRE))
        * Mat4::from_rotation_x(widget.rotation[0])
        * Mat4::from_rotation_y(yaw);
    (view_projection, model)
}

/// The time a card's own node animations are done by, in seconds: the latest
/// key of any channel that actually moves, held just short of the node's own
/// `LoopEnd`.
///
/// A card's elements animate in from a collapsed start (an RPCS3 frame taken a
/// second into a page change shows the wordmark and stripes half-grown), and
/// every reference frame this was fitted to is the settled end of that.
///
/// **Two traps, both met on `01_uplift`.** A channel with one key does not
/// move, and one node authors its single key at frame 36,000 - ten minutes -
/// which read as a last key would have pushed every other node's clock past
/// its six-second `LoopEnd` and wrapped the whole card back to its collapsed
/// start. And a node's last key sits *on* the loop point (360 frames of a
/// 6.0000005 s loop), so the time is backed off by a frame to stay on the
/// hold rather than on the wrap.
fn settled_seconds(model: &Model) -> f32 {
    use oag_render::mesh::Motion;
    model
        .anim_nodes
        .iter()
        .filter_map(|node| match &node.transform {
            Motion::Vex(transform) => {
                let unit = if transform.seconds_per_key > 0.0 {
                    transform.seconds_per_key
                } else {
                    1.0 / 60.0
                };
                let last = [
                    &transform.translation,
                    &transform.rotation,
                    &transform.scale,
                ]
                .iter()
                .filter(|channel| channel.times.len() > 1)
                .filter_map(|channel| channel.times.last().copied())
                .max()?;
                Some((f32::from(last) * unit).min(transform.loop_seconds - unit))
            }
            Motion::Rig(_) => None,
        })
        .fold(0.0, f32::max)
}

/// Every flyer card a campaign screen can show, decoded and waiting.
///
/// The card caches sit behind `RefCell`s so a stage that only holds the
/// campaign by shared reference (`MenuStage::render` reads it through
/// `self.campaign.as_ref()` for the whole of a frame) can still build a card's
/// GPU state the first time a grid shows it.
pub struct Flyers {
    widget: FlyerWidget,
    /// Decoded, not yet on the GPU, by `FlyerName`.
    waiting: RefCell<HashMap<String, Model>>,
    /// On the GPU, by `FlyerName`.
    cards: RefCell<HashMap<String, Preview>>,
    /// [`settled_seconds`] per card, by `FlyerName`.
    settled: HashMap<String, f32>,
    /// Set by whoever opens the campaign, from the session's own setting.
    pub anisotropy: Anisotropy,
    /// One line per card that would not read, for the loader's log.
    pub report: Vec<String>,
}

impl std::fmt::Debug for Flyers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Flyers")
            .field("widget", &self.widget.name)
            .field("waiting", &self.waiting.borrow().len())
            .field("cards", &self.cards.borrow().len())
            .finish_non_exhaustive()
    }
}

impl Flyers {
    /// Decodes the front of every named flyer through `archives`.
    ///
    /// A card that will not read or decode is reported and left out, which
    /// draws nothing for that grid - never another grid's card in its place.
    #[must_use]
    pub fn load(
        archives: &mut oag_assets::Archives,
        widget: FlyerWidget,
        names: &[String],
    ) -> Self {
        let mut waiting = HashMap::new();
        let mut settled = HashMap::new();
        let mut report = Vec::new();
        for name in names {
            if waiting.contains_key(name) {
                continue;
            }
            let entry = flyer::front_entry(name);
            match crate::preview::model(archives, &entry) {
                Ok(mut model) => {
                    // No light enters a flyer: its three materials are
                    // texture-only programs (`basicnonalpha` is `TEX` then
                    // `MOV`, `basicalpha` and `scrollingalpha` add a second
                    // `TEX` for the alpha), read off their microcode with
                    // `scripts/ps3-microcode.py`. `lit = 0` is this
                    // renderer's name for that; left at 1 the card is
                    // multiplied by the stand-in two-light rig and comes out
                    // at about 0.4 of its own colours.
                    for vertex in &mut model.vertices {
                        vertex.lit = 0.0;
                    }
                    // Posed at its settled moment and cut to its body - see
                    // [`clip`] for both, and for what is measured about the cut.
                    let at = settled_seconds(&model);
                    clip::bake(&mut model, at);
                    clip::clip(&mut model, clip::CARD_RECT);
                    settled.insert(name.clone(), settled_seconds(&model));
                    waiting.insert(name.clone(), model);
                }
                Err(error) => report.push(format!("{entry}: {error:#} - no flyer is drawn")),
            }
        }
        Self {
            widget,
            waiting: RefCell::new(waiting),
            cards: RefCell::new(HashMap::new()),
            settled,
            anisotropy: Anisotropy::default(),
            report,
        }
    }

    /// The card's bounding rectangle on `space`'s grid, `[x, y, width,
    /// height]`: [`clip::CARD_RECT`]'s four corners through the same camera
    /// [`Self::draw`] uses - what a click on the card should hit.
    #[must_use]
    pub fn card_rect(&self, space: Space) -> [f32; 4] {
        let (view_projection, model) =
            flyer_view_projection(&self.widget, space, CHOSEN_SETTLED_YAW);
        let to_screen = view_projection * model;
        let [x0, y0, x1, y1] = clip::CARD_RECT;
        let mut lo = [f32::MAX; 2];
        let mut hi = [f32::MIN; 2];
        for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
            let clip = to_screen * oag_core::math::Vec4::new(x, y, 0.0, 1.0);
            let ndc = [clip.x / clip.w, clip.y / clip.w];
            let at = [
                (ndc[0] * 0.5 + 0.5) * space.size.0,
                (0.5 - ndc[1] * 0.5) * space.size.1,
            ];
            for k in 0..2 {
                lo[k] = lo[k].min(at[k]);
                hi[k] = hi[k].max(at[k]);
            }
        }
        [lo[0], lo[1], hi[0] - lo[0], hi[1] - lo[1]]
    }

    /// Whether `name` has a card to draw.
    #[must_use]
    pub fn has(&self, name: &str) -> bool {
        self.waiting.borrow().contains_key(name) || self.cards.borrow().contains_key(name)
    }

    /// Draws `name`'s card into the screen, over whatever `view` holds.
    ///
    /// The first call for a name builds its GPU state; a card whose pipelines
    /// will not build is logged, dropped, and draws nothing from then on.
    #[allow(clippy::too_many_arguments)]
    fn draw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
        space: Space,
        name: &str,
    ) {
        if let Some(model) = self.waiting.borrow_mut().remove(name) {
            match Preview::new(device, queue, format, self.anisotropy, model) {
                Ok(card) => {
                    self.cards.borrow_mut().insert(name.to_string(), card);
                }
                Err(error) => log::warn!("flyer {name}: {error:#} - it draws nothing"),
            }
        }
        let mut cards = self.cards.borrow_mut();
        let Some(card) = cards.get_mut(name) else {
            return;
        };
        let (view_projection, model) =
            flyer_view_projection(&self.widget, space, CHOSEN_SETTLED_YAW);
        let seconds = self.settled.get(name).copied().unwrap_or(0.0);
        card.draw_matrices(
            device,
            queue,
            encoder,
            view,
            viewport,
            target_size,
            space,
            view_projection,
            model,
            seconds,
        );
    }
}

/// Renders a campaign screen's draw list, with the flyer card named by `card`
/// between its first `split` draws (the backdrop) and the rest - so the
/// card sits over the backdrop and under every widget, `Flyer Pad Lock`
/// included. With no card (`None`, or one that did not load) this is exactly
/// `renderer.render_with(load, .., list, ..)`.
///
/// `list` is the draws and the backdrop's length; `frame` the viewport, the
/// target's full size and the grid; `gpu` the device, queue and target format.
/// `clip` belongs to the un-split list and is handed through only on that
/// path; a split frame is HD's, which draws no ticker.
#[allow(clippy::too_many_arguments)]
pub fn render_list(
    renderer: &mut Renderer,
    load: wgpu::LoadOp<wgpu::Color>,
    gpu: (&wgpu::Device, &wgpu::Queue, wgpu::TextureFormat),
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    list: (&[Draw], usize),
    frame: ((f32, f32, f32, f32), (u32, u32), Space),
    card: Option<(&Flyers, &str)>,
    clip: Option<(usize, f32, f32)>,
) {
    let (device, queue, format) = gpu;
    let ((list, split), (viewport, target_size, space)) = (list, frame);
    let Some((flyers, name)) = card.filter(|(flyers, name)| flyers.has(name)) else {
        renderer.render_with(load, device, queue, encoder, view, list, viewport, clip);
        return;
    };
    let split = split.min(list.len());
    renderer.render_with(
        load,
        device,
        queue,
        encoder,
        view,
        &list[..split],
        viewport,
        None,
    );
    flyers.draw(
        device,
        queue,
        format,
        encoder,
        view,
        viewport,
        target_size,
        space,
        name,
    );
    let over = wgpu::LoadOp::Load;
    renderer.render_with(
        over,
        device,
        queue,
        encoder,
        view,
        &list[split..],
        viewport,
        None,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn widget() -> FlyerWidget {
        FlyerWidget {
            name: "FlyerModel".to_string(),
            src: String::new(),
            origin: [960.0, 540.0],
            depth: [1.0, 1000.0],
            position: [80.0, -33.3, -200.0],
            rotation: [0.0, 1.5],
            rotation_centre_offset_x: -60.0,
            ortho_scale: None,
        }
    }

    /// `Grid Selection`'s own origin is the centre of its grid, so the
    /// projection carries no shift: a point on the optical axis lands at the
    /// middle of the screen.
    #[test]
    fn a_centred_origin_does_not_shift_the_axis() {
        let (view_projection, _) = flyer_view_projection(&widget(), Space::HD, 0.0);
        let clip = view_projection * oag_core::math::Vec4::new(0.0, 0.0, -100.0, 1.0);
        assert!((clip.x / clip.w).abs() < 1e-5, "{clip:?}");
        assert!((clip.y / clip.w).abs() < 1e-5, "{clip:?}");
    }

    /// The card's centre lands where [`CHOSEN_CENTRE`] says, whatever the
    /// widget's own (unapplied) `x y z` are.
    #[test]
    fn the_card_centre_lands_at_the_chosen_centre() {
        let (_, model) = flyer_view_projection(&widget(), Space::HD, 0.0);
        let origin = model.transform_point3(Vec3::ZERO);
        assert!(
            (origin - Vec3::from(CHOSEN_CENTRE)).length() < 1e-4,
            "{origin:?}"
        );
    }

    /// The card's rectangle on the HD grid sits where the measured frames put
    /// it: its left edge near authored `x` 751 and its centre right of the
    /// grid's middle.
    #[test]
    fn the_card_rect_lands_where_the_frames_put_it() {
        let flyers = Flyers {
            widget: widget(),
            waiting: RefCell::new(HashMap::new()),
            cards: RefCell::new(HashMap::new()),
            settled: HashMap::new(),
            anisotropy: Anisotropy::default(),
            report: Vec::new(),
        };
        let [x, y, width, height] = flyers.card_rect(Space::HD);
        assert!((x - 751.0).abs() < 25.0, "left edge {x}");
        assert!(x + width / 2.0 > 960.0, "centre {}", x + width / 2.0);
        assert!(y > 100.0 && y + height < 1000.0, "{y} {height}");
    }

    /// A negative yaw brings the card's right edge toward the camera (toward
    /// +Z), the direction every settled RPCS3 frame shows.
    #[test]
    fn a_negative_yaw_brings_the_right_edge_closer() {
        let (_, model) = flyer_view_projection(&widget(), Space::HD, CHOSEN_SETTLED_YAW);
        let right = model.transform_point3(Vec3::new(51.2, 0.0, 0.0));
        let left = model.transform_point3(Vec3::new(-51.2, 0.0, 0.0));
        assert!(right.z > left.z, "{right:?} {left:?}");
    }
}
