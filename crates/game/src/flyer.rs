//! The 3-D flyer card behind HD/Fury's `Grid Selection`: decoded off the disc
//! once, put on the GPU the first time a grid shows it, and drawn by the same
//! [`crate::preview::Preview`] pass a circuit's outline ribbon uses.
//!
//! Nothing here is a second mesh-in-menu mechanism. [`crate::preview::model`]
//! learned the PS3 `.vex` + `.rcsmodel` pair (it could only read a PSP or PS2
//! `.vex` before, which is why HD's own `preview_meshes` is `false`), and
//! [`crate::preview::Preview::draw_matrices`] is `draw_mode3d`'s second half
//! with the camera passed in. This module adds the things specific to a
//! flyer: which entries to read ([`oag_ui_screens::campaign::flyer`]), what the card
//! shows ([`clip`]) and where it stands ([`flyer_view_projection`]).
//!
//! # What a card is
//!
//! **A flat picture of the flyer's own scene, turned on the screen.** Every
//! flyer authors a camera (`camera1`, [`oag_vex::camera`]) and is composed for
//! it; the Fury cards are stacks of layers 28 units deep and every card's
//! elements overhang its frame. RPCS3 shows the camera's image on a rectangle
//! and nothing beyond it - the picture behind the reading is in [`clip`] - so
//! the scene is flattened through its own camera and cut to the card, and the
//! card is a rectangle in 3-D.
//!
//! # Authored versus chosen
//!
//! **Authored**: which model (`Data/FE/Flyers/<FlyerName>/flyer.vex`), the
//! widget's `OriginX`/`OriginY`, `nearZ`/`farZ`, the flyer's own camera
//! position, and the materials and textures, which come from the disc's own
//! `.rcsmaterial`s. **Measured** off the executable and confirmed by fitting:
//! the widget's vertical field of view, [`FOV_Y`].
//!
//! **Chosen, not measured**, each fitted to settled RPCS3 frames of `Grid
//! Selection` - two Fury tiers and four base tiers, jointly, mean correlation
//! 0.91: the grid card's [`GRID_POSE`], its height [`CARD_HEIGHT`], and how much of
//! the camera's image it shows ([`HD_WINDOW`], [`FURY_WINDOW`]). The widget's own `x y z`,
//! `RotY` and pivot describe a start pose (as radians `RotY="1.5"` is an 86
//! degree turn, which no settled frame shows) and the native code that settles
//! them is unread.
//!
//! **Not drawn**: the card's body colour under the picture, the glow around it, the flip to `flyer_back.vex`, the elements
//! animating, and the light: Fury's cards are lit by `simpletexture*`
//! programs whose light direction and colours (`0x02df31e5`, `0x2dba643d`,
//! `0x81db67ea`) are written by code nobody has found, so every card is drawn
//! at its own texture colours.

use std::cell::RefCell;
use std::collections::HashMap;

use oag_core::math::{Mat4, Vec3, camera};
use oag_display::space::Space;
use oag_mesh::mesh::Model;
use oag_mesh::mesh_render::Anisotropy;
use oag_ui::frontend::Draw;
use oag_ui_screens::campaign::flyer::{self, FlyerWidget};

use crate::preview::Preview;
use crate::render::Renderer;

mod clip;
mod shell;

pub use shell::Shell;

/// The vertical field of view the card is seen at, radians: **1.0,
/// measured**. `Flyer_Item`'s render function (`0x001a2510`) builds its
/// projection from `tanf(0.5)`, a literal in the executable, and a planar fit
/// with the focal length free lands on 0.90 to 1.03 rad on four base frames.
pub const FOV_Y: f32 = 1.0;

/// The card's height, in the units [`GRID_POSE`] is written in. **Chosen**; 66.6 is
/// what `00_flyer.vex`'s `cardShape` spans.
pub const CARD_HEIGHT: f32 = 66.6;

/// The moment of a card's own animation it is drawn at, seconds. **Chosen**:
/// inside the widget's own idle loop (the constructor at `0x001a2060` holds
/// `6.0` and `3.0`, and the draw at `0x001a2510` wraps a time past `6.0` back
/// by 3), after the elements are in, with `10_impact`'s ship assembled, and
/// before the glitch a Fury card flashes at the end of its four-second loop.
pub const SETTLED_SECONDS: f32 = 3.0;

/// How the card stands in front of the widget's camera: turned about its
/// vertical axis, and moved from straight ahead.
///
/// **Chosen, not measured.**
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    /// Turn about the card's vertical axis, radians. Negative brings the
    /// card's right edge toward the camera, which is how every settled RPCS3
    /// frame shows it (the right edge is the taller one).
    pub yaw: f32,
    /// Where the card's centre is: `(x right, y up, z)` with `z` negative in
    /// front of the camera, in the card's own units.
    pub offset: [f32; 3],
}

/// `Grid Selection`'s settled pose: one for every card, base campaign and Fury
/// alike.
///
/// **Chosen, not measured.** Fitted jointly on two Fury frames and four base
/// frames, each frame a flat warp of the card's own camera image against the
/// RPCS3 frame: yaw -0.289, centre 22.0 units right and 111.1 in front, all
/// six frames within 0.87 to 0.94 correlation. It puts the card's left edge at
/// authored column 747, where every settled RPCS3 frame puts it (750).
pub const GRID_POSE: Pose = Pose {
    yaw: -0.289,
    offset: [22.0, 0.0, -111.1],
};

/// `Cell Selection`'s card: the grid's flyer seen from behind, face-on, filling
/// the screen's frame.
///
/// **Measured rectangle, derived pose.** The settled RPCS3 frame puts the card
/// at authored columns 165 to 1756 and rows 139 to 937 of the 1920 by 1080 grid
/// (the `GridTopBar` rule inside it spans 190 to 1690 and the frame's own
/// `Bracket` on `Campaign Selection` 160 to 1755, which brackets the same
/// area) - centred on the screen, 1.994 wide for its height. A card
/// [`CARD_HEIGHT`] tall at [`FOV_Y`] fills 798 of the grid's 1080 rows at a
/// distance of `33.3 / (tan 0.5 * 798 / 1080)` = 82.5 units, so the pose is
/// straight ahead at that distance: nothing in it is fitted by eye.
pub const CELL_POSE: Pose = Pose {
    yaw: 0.0,
    offset: [0.0, 0.0, -82.5],
};

/// The tangent of half the vertical field of view of the camera's image the
/// base campaign's cards show - how much of what the flyer's camera sees fits
/// on the card. **Chosen, not measured**: fitted on four base frames (0.346,
/// 680 pixels of a 1080-tall picture). The camera frames the artwork, not the
/// backing plane: on `04_vertigo` the elements run to `+-48` units and the
/// window is `+-48.9` wide at the picture's own depth.
pub const HD_WINDOW: f32 = 0.346;

/// The same for Fury's cards and the two `Campaign Selection` cards.
/// **Chosen, not measured**: 0.321 fitted on two Fury frames, and 0.319 from
/// `hd_campaign`'s own geometry, whose 22.2 by 20.4 background sits 32 units
/// from its camera and is exactly the picture's height at 0.319.
///
/// The camera's `+0x20` word moves with the two windows (`0x4f15` and `0x4a2c`
/// against 0.346 and 0.321) but not with the campaign cards' (`0x3621`, which
/// the same proportionality would put at 0.236), so it is not what sets them.
pub const FURY_WINDOW: f32 = 0.321;

/// The window of a back card's camera image the card shows, and how much wider
/// than its camera's aspect it is drawn. **Chosen, not measured** (see the
/// `Cell Selection` section of `docs/ui/campaign-screens.md`).
pub const BACK_WINDOW: f32 = 0.321;
pub const BACK_STRETCH: f32 = 1.2965;

/// How much wider than its camera's aspect the two `Campaign Selection`
/// cards are drawn: **chosen, not measured**. On RPCS3's frames they stand
/// 800 by 710 authored pixels (aspect 1.13) where the camera's `+0x1c` says
/// 1.083, and the wordmark on `Fury`'s is 5 percent wider than the picture at
/// the camera's aspect makes it, so the picture is stretched, not shown wider.
pub const CAMPAIGN_STRETCH: f32 = 1.048;

/// Which of a flyer's two models a card is: `flyer.vex` or `flyer_Back.vex`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// `Grid Selection` and `Campaign Selection` draw this one.
    Front,
    /// `Cell Selection` draws this one, face-on ([`CELL_POSE`]).
    Back,
}

impl Side {
    /// The archive entry of `flyer`'s model on this side.
    fn entry(self, flyer: &str) -> String {
        match self {
            Self::Front => flyer::front_entry(flyer),
            Self::Back => flyer::back_entry(flyer),
        }
    }

    /// What a side's card is stored under: a front card by the bare name.
    fn key(self, flyer: &str) -> String {
        match self {
            Self::Front => flyer.to_string(),
            Self::Back => format!("{flyer}/back"),
        }
    }
}

/// How much brighter a card is than its textures: every card texel is doubled
/// (and clamped), on both models. **Measured, mechanism unread.** A flat
/// panel's texel is 63 and RPCS3 shows 130 (`Cell Selection`, the back's
/// `flyer_back_colour.gtf`); the stripe texture's 197 shows 255 on both
/// the back and `Grid Selection`'s front, whose dominant red is 255 against
/// the 198 an undoubled texel drew. The same factor on all of them, so it is
/// the card's, not one material's; what writes it is unread.
pub const CARD_GAIN: f32 = 2.0;

/// One card to load: a flyer's model, the window of its camera's image it
/// shows and how much wider than its camera's aspect it is drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct CardSpec {
    /// The `FlyerName`.
    pub flyer: String,
    /// Which model.
    pub side: Side,
    /// [`HD_WINDOW`] or [`FURY_WINDOW`].
    pub window: f32,
    /// `1.0` or [`CAMPAIGN_STRETCH`].
    pub stretch: f32,
}

/// One card on a screen: which flyer, seen through which widget, in which pose.
#[derive(Debug, Clone, PartialEq)]
pub struct Show {
    /// The flyer's `FlyerName`: `01_uplift`, `fury_campaign`.
    pub flyer: String,
    /// Which of its two models.
    pub side: Side,
    /// The `<Flyer>` widget that places it: `FlyerModel`,
    /// `FuryCampaignFlyerModel`.
    pub widget: String,
    /// Where it stands.
    pub pose: Pose,
}

impl Show {
    /// What the card is stored under in [`Flyers`].
    fn key(&self) -> String {
        self.side.key(&self.flyer)
    }
}

/// The pose of a `Campaign Selection` card, from its widget's own numbers.
///
/// **Authored, with three things chosen**: the widget's `orthoScale` (`0.75`)
/// scales its placement, so the card stands `z / scale` in front of the
/// camera (70 / 0.75 = 93.3 units), turned by `RotY` about a pivot
/// `RotationCentreOffsetX / scale` beside its centre (`+-40`), and its
/// `y` of `-33.3` puts the card's centre 11.1 units under the axis. The
/// unselected card turns by `RotY` (`0.6` on Fury's, `-0.6` on `HD`'s) and the
/// selected one faces the camera, which is **chosen**: that reading fits the
/// RPCS3 frames where Fury's card faces front and `HD`'s is turned when Fury
/// is selected and the reverse when `HD` is, but the widget authors only the
/// turned pose. The half card height in the `y` is the placeholder's own
/// (`cardShape` runs from `0` up by 66.6, so `-33.3` centres it - read from
/// the header's bounds, not yet decoded).
#[must_use]
pub fn campaign_pose(widget: &FlyerWidget, selected: bool) -> Pose {
    let scale = widget.ortho_scale.unwrap_or(1.0);
    let yaw = if selected { 0.0 } else { widget.rotation[1] };
    let pivot = widget.rotation_centre_offset_x / scale;
    // The card's centre is `pivot` from the pivot; turning it about the
    // pivot moves it by `pivot - R(yaw) * pivot`.
    let offset = [
        widget.position[0] / scale + pivot - pivot * yaw.cos(),
        widget.position[1] / scale + CARD_HEIGHT / 2.0,
        widget.position[2] / scale + pivot * yaw.sin(),
    ];
    Pose { yaw, offset }
}

/// The camera and model matrices for the card on `space`'s grid.
///
/// - **Projection**: a symmetric perspective frustum at [`FOV_Y`] and the
///   display's own aspect, with the widget's authored `nearZ`/`farZ`.
/// - **View**: none - the card stands in front of the widget's camera at
///   `pose.offset`.
/// - **`OriginX`/`OriginY`**: the screen point the optical axis passes
///   through, as a post-projection shift from the grid's centre. On `Grid
///   Selection` that is `(960, 540)`, the centre of the 1920 by 1080 grid, so
///   no shift at all; `Campaign Selection`'s pair author `(640, 450)` and
///   `(1280, 450)`. **Chosen, not measured**: that these are absolute
///   positions rather than offsets as `Mode3D`'s are - the two readings
///   agree on `Grid Selection`'s own numbers.
/// - **Model**: the card turned by the widget's authored `RotX` (`0.0`) and
///   `pose.yaw`, then moved by `pose.offset`.
///
/// **The widget's own `x y z`, `RotY` and `RotationCentreOffsetX` are read and
/// not applied** - see the module doc.
#[must_use]
pub fn flyer_view_projection(widget: &FlyerWidget, space: Space, pose: Pose) -> (Mat4, Mat4) {
    let [near, far] = widget.depth;
    let projection = camera::perspective(FOV_Y, space.display_aspect, near, far);
    let shift = Vec3::new(
        2.0 * widget.origin[0] / space.size.0 - 1.0,
        1.0 - 2.0 * widget.origin[1] / space.size.1,
        0.0,
    );
    let view_projection = Mat4::from_translation(shift) * projection;
    let model = Mat4::from_translation(Vec3::from(pose.offset))
        * Mat4::from_rotation_x(widget.rotation[0])
        * Mat4::from_rotation_y(pose.yaw);
    (view_projection, model)
}

/// The textures of the effects a Fury card plays over itself: the crash-screen
/// glitch panel and its noise bars, the rings and the flashes.
///
/// **Not drawn, on purpose.** Each is a quad textured through a material that
/// takes its UV offset and scale (`simpletextureandtexturealphauvoffsetscale`)
/// or its alpha from a parameter the native code animates, and drawn with
/// those parameters at rest they come out as solid white discs, white bars and
/// a brown static panel where RPCS3 shows nothing at all at three different
/// moments of the card's loop. An effect whose driver is unread is left
/// absent rather than drawn wrong.
const UNREAD_EFFECTS: [&str; 5] = [
    "loops",
    "flashes",
    "noise_bar",
    "failscreen",
    "crash_screen",
];

/// Removes every draw that samples one of [`UNREAD_EFFECTS`]. `textures` is
/// the `.gtf` path of each texture slot, in order.
fn hide_effects(model: &mut Model, textures: &[String]) {
    let hidden = |draw: &oag_mesh::mesh::DrawCall| {
        draw.texture
            .and_then(|slot| textures.get(slot))
            .is_some_and(|path| {
                let path = path.to_ascii_lowercase();
                UNREAD_EFFECTS.iter().any(|effect| path.contains(effect))
            })
    };
    model.draws.retain(|draw| !hidden(draw));
    model.alpha_tested_draws.retain(|draw| !hidden(draw));
    model.transparent_draws.retain(|draw| !hidden(draw));
}

/// Where a card sits on its rectangle: half its width and height, card units.
#[derive(Debug, Clone, Copy)]
struct Placement {
    half_size: [f32; 2],
}

/// The card's outline widened by `stretch`.
fn stretched(outline: &[[[f32; 2]; 3]], stretch: f32) -> Vec<[[f32; 2]; 3]> {
    outline
        .iter()
        .map(|triangle| triangle.map(|[x, y]| [x * stretch, y]))
        .collect()
}

/// `outline` turned about the card's vertical axis: the back face.
fn mirrored(outline: &[[[f32; 2]; 3]]) -> Vec<[[f32; 2]; 3]> {
    outline
        .iter()
        .map(|triangle| triangle.map(|[x, y]| [-x, y]))
        .collect()
}

/// Every flyer card a campaign screen can show, decoded and waiting.
///
/// The card caches sit behind `RefCell`s so a stage that only holds the
/// campaign by shared reference (`MenuStage::render` reads it through
/// `self.campaign.as_ref()` for the whole of a frame) can still build a card's
/// GPU state the first time a grid shows it.
pub struct Flyers {
    /// Every `<Flyer>` widget the screen file authors, by name.
    widgets: HashMap<String, FlyerWidget>,
    /// Decoded, not yet on the GPU, by `FlyerName`.
    waiting: RefCell<HashMap<String, Model>>,
    /// On the GPU, by `FlyerName`.
    cards: RefCell<HashMap<String, Preview>>,
    /// The rectangle of each card, by `FlyerName`.
    placed: HashMap<String, Placement>,
    /// Set by whoever opens the campaign, from the session's own setting.
    pub anisotropy: Anisotropy,
    /// One line per card that would not read, for the loader's log.
    pub report: Vec<String>,
}

impl std::fmt::Debug for Flyers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Flyers")
            .field("widgets", &self.widgets.len())
            .field("waiting", &self.waiting.borrow().len())
            .field("cards", &self.cards.borrow().len())
            .finish_non_exhaustive()
    }
}

impl Flyers {
    /// Decodes the front of every flyer `cards` names through `archives`.
    ///
    /// A card that will not read or decode, or whose `.vex` authors no camera,
    /// is reported and left out, which draws nothing for that grid - never
    /// another grid's card in its place.
    #[must_use]
    pub fn load(
        archives: &mut oag_assets::Archives,
        widgets: Vec<FlyerWidget>,
        cards: &[CardSpec],
    ) -> Self {
        let mut waiting = HashMap::new();
        let mut placed = HashMap::new();
        let mut report = Vec::new();
        // The card's own shape and reflection. Without them a card is a plain
        // rectangle with no reflection, and the loader says so.
        let shell = match shell::Shell::load(archives) {
            Ok(shell) => Some(shell),
            Err(error) => {
                report.push(format!(
                    "{}: {error:#} - cards are plain rectangles with no reflection",
                    shell::ENTRY
                ));
                None
            }
        };
        for CardSpec {
            flyer,
            side,
            window,
            stretch,
        } in cards
        {
            let name = &side.key(flyer);
            if waiting.contains_key(name) {
                continue;
            }
            let entry = side.entry(flyer);
            let camera = archives
                .read_name(&entry)
                .ok()
                .and_then(|blob| oag_vex::camera::cameras(&blob).into_iter().next());
            let Some(camera) = camera else {
                report.push(format!("{entry}: no Camera node - no flyer is drawn"));
                continue;
            };
            match crate::preview::model_named(archives, &entry) {
                Ok((mut model, textures)) => {
                    hide_effects(&mut model, &textures);
                    // Fury's cards are lit by `simpletexture*` programs
                    // (`(ambient + saturate(N.L) * sun) * texture`), the base
                    // campaign's are not (`basicnonalpha` is `TEX` then `MOV`,
                    // `basicalpha` and `scrollingalpha` add a second `TEX` for
                    // the alpha) - read off their microcode with
                    // `scripts/ps3-microcode.py`. What feeds the light is a
                    // native write nobody has found, so both are drawn unlit
                    // (`lit = 0`, this renderer's name for that): left at 1 a
                    // card is multiplied by the stand-in two-light rig and
                    // comes out at about 0.4 of its own colours.
                    for vertex in &mut model.vertices {
                        vertex.lit = 0.0;
                        vertex.colour = [CARD_GAIN, CARD_GAIN, CARD_GAIN, 1.0];
                    }
                    model.vertex_colour_is_light = false;
                    clip::bake(&mut model, SETTLED_SECONDS);
                    clip::flatten(&mut model, &camera.to_world, *window, CARD_HEIGHT, *stretch);
                    // The card is as wide as its camera's aspect makes it (times
                    // `stretch`): the shell's outline, a 1.54 rectangle, is
                    // widened or narrowed to that - by nothing on the base and
                    // Fury grids, whose cameras say 1.54, and to 0.74 on the
                    // squarer campaign cards.
                    let half_width = camera.value_1c * CARD_HEIGHT / 2.0 * stretch;
                    let half = [half_width, CARD_HEIGHT / 2.0];
                    match &shell {
                        Some(shell) => {
                            let widen = half_width / shell.half_size[0];
                            let outline = stretched(&shell.outline, widen);
                            match side {
                                Side::Front => {
                                    clip::clip_to_shape(&mut model, &outline);
                                    clip::reflect(&mut model, shell.fade);
                                }
                                // The back's outline is the front's seen from the other
                                // side, and RPCS3 draws no reflection under it.
                                Side::Back => {
                                    clip::clip_to_shape(&mut model, &mirrored(&outline));
                                }
                            }
                        }
                        None => clip::clip(&mut model, [-half[0], -half[1], half[0], half[1]]),
                    }
                    placed.insert(name.clone(), Placement { half_size: half });
                    waiting.insert(name.clone(), model);
                }
                Err(error) => report.push(format!("{entry}: {error:#} - no flyer is drawn")),
            }
        }
        Self {
            widgets: widgets
                .into_iter()
                .map(|widget| (widget.name.clone(), widget))
                .collect(),
            waiting: RefCell::new(waiting),
            cards: RefCell::new(HashMap::new()),
            placed,
            anisotropy: Anisotropy::default(),
            report,
        }
    }

    /// The card `Grid Selection` shows for the flyer named `flyer`.
    #[must_use]
    pub fn grid_show(flyer: &str) -> Show {
        Show {
            flyer: flyer.to_string(),
            side: Side::Front,
            widget: flyer::GRID_WIDGET.to_string(),
            pose: GRID_POSE,
        }
    }

    /// The card `Cell Selection` shows for the flyer named `flyer`: the back
    /// of the grid's own flyer, through the same widget as [`Self::grid_show`].
    #[must_use]
    pub fn cell_show(flyer: &str) -> Show {
        Show {
            flyer: flyer.to_string(),
            side: Side::Back,
            widget: flyer::GRID_WIDGET.to_string(),
            pose: CELL_POSE,
        }
    }

    /// The two cards `Campaign Selection` shows with `selected` chosen: each
    /// through its own widget, the chosen one facing the camera and the other
    /// turned as its widget authors it ([`campaign_pose`]). Empty when either
    /// widget is missing.
    #[must_use]
    pub fn selection_shows(
        &self,
        selected: oag_ui_screens::campaign::selection::Campaign,
    ) -> Vec<Show> {
        use oag_ui_screens::campaign::selection::Campaign;
        [
            (
                Campaign::Fury,
                flyer::FURY_CAMPAIGN_FLYER,
                flyer::FURY_CAMPAIGN_WIDGET,
            ),
            (
                Campaign::Hd,
                flyer::HD_CAMPAIGN_FLYER,
                flyer::HD_CAMPAIGN_WIDGET,
            ),
        ]
        .into_iter()
        .filter_map(|(campaign, name, widget)| {
            let pose = campaign_pose(self.widgets.get(widget)?, campaign == selected);
            Some(Show {
                flyer: name.to_string(),
                side: Side::Front,
                widget: widget.to_string(),
                pose,
            })
        })
        .collect()
    }

    /// `show`'s camera and model matrices on `space`'s grid, or `None` when its
    /// widget or its card is not loaded.
    fn matrices(&self, show: &Show, space: Space) -> Option<(Mat4, Mat4)> {
        let widget = self.widgets.get(&show.widget)?;
        self.has(show)
            .then(|| flyer_view_projection(widget, space, show.pose))
    }

    /// The card's bounding rectangle on `space`'s grid, `[x, y, width,
    /// height]`: its four corners through the same camera [`Self::draw`] uses -
    /// what a click on the card should hit. `None` for a card that is not
    /// loaded.
    #[must_use]
    pub fn card_rect(&self, show: &Show, space: Space) -> Option<[f32; 4]> {
        let [hx, hy] = self.placed.get(&show.key())?.half_size;
        let (view_projection, model) = self.matrices(show, space)?;
        let to_screen = view_projection * model;
        let mut lo = [f32::MAX; 2];
        let mut hi = [f32::MIN; 2];
        for (x, y) in [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)] {
            let clip = to_screen * oag_core::math::Vec4::new(x, y, 0.0, 1.0);
            let at = [
                (clip.x / clip.w * 0.5 + 0.5) * space.size.0,
                (0.5 - clip.y / clip.w * 0.5) * space.size.1,
            ];
            for k in 0..2 {
                lo[k] = lo[k].min(at[k]);
                hi[k] = hi[k].max(at[k]);
            }
        }
        Some([lo[0], lo[1], hi[0] - lo[0], hi[1] - lo[1]])
    }

    /// Whether `show` has a card to draw.
    #[must_use]
    pub fn has(&self, show: &Show) -> bool {
        let key = show.key();
        self.waiting.borrow().contains_key(&key) || self.cards.borrow().contains_key(&key)
    }

    /// Draws `show`'s card into the screen, over whatever `view` holds.
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
        show: &Show,
    ) {
        let key = show.key();
        if let Some(model) = self.waiting.borrow_mut().remove(&key) {
            match Preview::new(device, queue, format, self.anisotropy, model) {
                Ok(card) => {
                    self.cards.borrow_mut().insert(key.clone(), card);
                }
                Err(error) => log::warn!("flyer {}: {error:#} - it draws nothing", show.flyer),
            }
        }
        let Some((view_projection, model)) = self.matrices(show, space) else {
            return;
        };
        let mut cards = self.cards.borrow_mut();
        let Some(card) = cards.get_mut(&key) else {
            return;
        };
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
            SETTLED_SECONDS,
        );
    }
}

/// Renders a campaign screen's draw list, with the flyer cards `cards` names
/// between its first `split` draws (the backdrop) and the rest - so the
/// cards sit over the backdrop and under every widget, `Flyer Pad Lock`
/// included. With no card (`None`, or none that loaded) this is exactly
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
    cards: Option<(&Flyers, &[Show])>,
    clip: Option<(usize, f32, f32)>,
) {
    let (device, queue, format) = gpu;
    let ((list, split), (viewport, target_size, space)) = (list, frame);
    let shown: Vec<&Show> = cards
        .map(|(flyers, shows)| shows.iter().filter(|show| flyers.has(show)).collect())
        .unwrap_or_default();
    let Some((flyers, _)) = cards.filter(|_| !shown.is_empty()) else {
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
    for show in shown {
        flyers.draw(
            device,
            queue,
            format,
            encoder,
            view,
            viewport,
            target_size,
            space,
            show,
        );
    }
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

    /// Where a card-local point lands on `space`'s grid.
    fn on_grid(point: Vec3, space: Space) -> [f32; 2] {
        let (view_projection, model) = flyer_view_projection(&widget(), space, GRID_POSE);
        let clip =
            view_projection * model * oag_core::math::Vec4::new(point.x, point.y, point.z, 1.0);
        [
            (clip.x / clip.w * 0.5 + 0.5) * space.size.0,
            (0.5 - clip.y / clip.w * 0.5) * space.size.1,
        ]
    }

    /// `Grid Selection`'s own origin is the centre of its grid, so the
    /// projection carries no shift: a point on the optical axis lands at the
    /// middle of the screen.
    #[test]
    fn a_centred_origin_does_not_shift_the_axis() {
        let (view_projection, _) = flyer_view_projection(&widget(), Space::HD, GRID_POSE);
        let clip = view_projection * oag_core::math::Vec4::new(0.0, 0.0, -100.0, 1.0);
        assert!((clip.x / clip.w).abs() < 1e-5, "{clip:?}");
        assert!((clip.y / clip.w).abs() < 1e-5, "{clip:?}");
    }

    /// The card's centre lands where [`GRID_POSE`] says, whatever the widget's own
    /// (unapplied) `x y z` are.
    #[test]
    fn the_card_centre_lands_at_the_pose() {
        let (_, model) = flyer_view_projection(&widget(), Space::HD, GRID_POSE);
        let origin = model.transform_point3(Vec3::ZERO);
        assert!(
            (origin - Vec3::from(GRID_POSE.offset)).length() < 1e-4,
            "{origin:?}"
        );
    }

    /// The card's left edge sits where every settled RPCS3 frame puts it:
    /// authored column 750, on the base campaign's card and Fury's alike (the
    /// two differ by less than a tenth of a unit in width).
    #[test]
    fn the_left_edge_lands_at_the_column_rpcs3_shows() {
        for width in [102.43_f32, 102.49] {
            let [x, _] = on_grid(Vec3::new(-width / 2.0, 0.0, 0.0), Space::HD);
            assert!((x - 750.0).abs() < 6.0, "{width}: left edge {x}");
        }
    }

    /// A negative yaw brings the card's right edge toward the camera (toward
    /// +Z), the direction every settled RPCS3 frame shows.
    #[test]
    fn a_negative_yaw_brings_the_right_edge_closer() {
        let (_, model) = flyer_view_projection(&widget(), Space::HD, GRID_POSE);
        let right = model.transform_point3(Vec3::new(51.2, 0.0, 0.0));
        let left = model.transform_point3(Vec3::new(-51.2, 0.0, 0.0));
        assert!(right.z > left.z, "{right:?} {left:?}");
    }

    /// The selected card faces the camera at the distance its widget puts it:
    /// `z` over the `orthoScale`, and its `y` puts the centre under the axis.
    #[test]
    fn a_selected_campaign_card_faces_front_at_the_distance_the_widget_puts_it() {
        let widget = FlyerWidget {
            position: [0.0, -33.3, -70.0],
            rotation: [0.0, -0.6],
            rotation_centre_offset_x: 30.0,
            ortho_scale: Some(0.75),
            ..widget()
        };
        let pose = campaign_pose(&widget, true);
        assert_eq!(pose.yaw, 0.0);
        assert!((pose.offset[2] + 93.33).abs() < 0.01, "{pose:?}");
        assert!((pose.offset[1] + 11.1).abs() < 0.01, "{pose:?}");
        assert!(pose.offset[0].abs() < 1e-4, "{pose:?}");
    }

    /// The unselected card turns by `RotY` about its pivot, which moves its
    /// centre: `HD`'s (pivot on the right, `RotY` negative) swings back and
    /// toward its pivot, and Fury's mirrors it.
    #[test]
    fn an_unselected_campaign_card_swings_about_its_pivot() {
        let widget = |x: f32, turn: f32, pivot: f32| FlyerWidget {
            position: [0.0, -33.3, -70.0],
            rotation: [0.0, turn],
            rotation_centre_offset_x: pivot,
            ortho_scale: Some(0.75),
            ..FlyerWidget {
                origin: [x, 450.0],
                ..self::tests::widget()
            }
        };
        let hd = campaign_pose(&widget(1280.0, -0.6, 30.0), false);
        let fury = campaign_pose(&widget(640.0, 0.6, -30.0), false);
        assert_eq!(hd.yaw, -0.6);
        assert!(hd.offset[2] < -93.33 - 20.0, "{hd:?}");
        assert!(hd.offset[0] > 5.0 && hd.offset[0] < 9.0, "{hd:?}");
        assert!(
            (hd.offset[2] - fury.offset[2]).abs() < 1e-4,
            "{hd:?} {fury:?}"
        );
        assert!(
            (hd.offset[0] + fury.offset[0]).abs() < 1e-4,
            "{hd:?} {fury:?}"
        );
    }
}
