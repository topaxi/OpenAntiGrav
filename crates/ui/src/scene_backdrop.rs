//! The HD-style menu backdrop's model: which scene plays, how far into its
//! loop it is, and the look the page it sits behind asks for.
//!
//! Wipeout HD's `skin.xml` `Top FE Screen` carries a `<BackgroundAnim>` widget
//! (`BackgroundAnim_Item.cpp`). It names a 3D scene (`Src`), plays the scene's
//! own authored animation and camera (`UseModelCamera`), renders it into a
//! target cleared to white, and filters that target through
//! `FEBackgroundAnim_fp` into a grey drawing: a flat `fill_level` darkening
//! wherever the scene is, and an `edge_level` darkening along its edges,
//! `edge_width` pixels wide. A global blur of `blur` pixels (at 1080 lines)
//! follows. Every number is on a `<ScreenSetting>` row, one per screen, and
//! the widget eases its working copy toward the row of the screen showing.
//!
//! This module is the GPU-free half: it parses the widget, keeps the working
//! copy and the clock, and hands `oag_game::render` a [`Frame`]. The camera
//! lives in the scene file, so the matrices are the caller's to fill - this
//! crate reads no `.vex`. Read off
//! [menu-backdrop-scene.md](../../../docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop-scene.md).
//!
//! # What is not drawn
//!
//! - **Bands.** `use_bands` is `false` on every row of every skin this project
//!   has read, so the `Band1`/`Band2` levels and the band timers never run.
//!   They are parsed and kept, and [`Widget::bands`] says when a skin turns
//!   them on, so the report can name the gap rather than hide it.
//! - **`GroundPlane`.** The child `<Model>` names `frontendscene_ground_VR.vex`
//!   and is not drawn: no flag says it is a VR-only child, but its file name
//!   does, and this build has no VR.

use oag_tables::fexml::Node;

/// Frames a second the original steps the widget at: `Update` is called once
/// a frame and eases by a fixed fraction, not by the time step.
pub const FRAMES_PER_SECOND: f32 = 60.0;

/// What `Update` (`0x0017af80`) keeps of the working copy each call: the copy
/// is multiplied by this and the target row by `1 -` this. Read as float
/// literals at `0x008ac0ec` (`0.97`) and `0x008ac0f0` (`0.03`).
pub const KEEP: f32 = 0.97;

/// The fraction of the target row added each call. See [`KEEP`].
pub const PULL: f32 = 0.03;

/// The scene's own loop length when the skin names none: `fmodf(t, 60.0)` in
/// `0x0017bac0`, the literal at `0x008ac110`. Omega authors the same value as
/// `AnimLength`.
pub const LOOP_SECONDS: f32 = 60.0;

/// The vertical field of view of the scene's camera, radians: the model
/// widget's default `fov` attribute, `57.3` degrees (`0x001d326c`), which is
/// `1.000` radians to three places - the value `Flyer_Item` measured.
pub const FOV_Y: f32 = 57.3 * std::f32::consts::PI / 180.0;

/// One `<Main>` row: the three numbers the filter takes.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Levels {
    /// How much an edge darkens the white, `0` to `1`.
    pub edge_level: f32,
    /// How much a filled pixel (anything the scene drew) darkens it, `0` to `1`.
    pub fill_level: f32,
    /// The tap spacing in pixels, `0` to `24`.
    pub edge_width: f32,
}

/// What a page asks the widget for: one `<ScreenSetting>`'s `blur` and `Main`
/// levels.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Look {
    /// The blur radius in pixels at 1080 lines.
    pub blur: f32,
    /// The filter's levels.
    pub main: Levels,
}

impl Look {
    fn eased_toward(self, target: Self) -> Self {
        let ease = |from: f32, to: f32| from * KEEP + to * PULL;
        Self {
            blur: ease(self.blur, target.blur),
            main: Levels {
                edge_level: ease(self.main.edge_level, target.main.edge_level),
                fill_level: ease(self.main.fill_level, target.main.fill_level),
                edge_width: ease(self.main.edge_width, target.main.edge_width),
            },
        }
    }
}

/// The widget as the skin authors it.
#[derive(Debug, Clone, PartialEq)]
pub struct Widget {
    /// `Src`: the scene's entry name, in the skin's own spelling.
    pub src: String,
    /// `AnimLength`, seconds; [`LOOP_SECONDS`] when the skin authors none.
    pub anim_length: f32,
    /// `nearZ` and `farZ`.
    pub depth: [f32; 2],
    /// `UseModelCamera`: the scene's own camera drives the view.
    pub model_camera: bool,
    rows: Vec<(String, Look)>,
    bands: bool,
}

impl Widget {
    /// The first `<BackgroundAnim>` under `root`, using its first `<Values>`
    /// row that is not for VR - the PSVR-only rows carry `IsVR="true"` and
    /// name a different scene.
    ///
    /// `None` when the file authors no such widget or names no `Src`.
    #[must_use]
    pub fn read(root: &Node) -> Option<Self> {
        let widget = find(root, "BackgroundAnim")?;
        let values = widget
            .children_named("Values")
            .find(|values| !values.attr("IsVR").is_some_and(is_true))?;
        let number = |name: &str, default: f32| {
            values
                .attr(name)
                .and_then(|text| text.trim().parse::<f32>().ok())
                .unwrap_or(default)
        };
        let rows: Vec<(String, Look)> = widget
            .children_named("ScreenSetting")
            .filter_map(|setting| Some((setting.attr("name")?.to_string(), look_of(setting))))
            .collect();
        let bands = widget
            .children_named("ScreenSetting")
            .any(|setting| setting.attr("use_bands").is_some_and(is_true));
        Some(Self {
            src: values.attr("Src")?.to_string(),
            anim_length: number("AnimLength", LOOP_SECONDS),
            depth: [number("nearZ", 20.0), number("farZ", 100.0)],
            model_camera: values.attr("UseModelCamera").is_some_and(is_true),
            rows,
            bands,
        })
    }

    /// The row a screen names, falling back to `default`, then to nothing -
    /// the working copy's zero-filled start.
    #[must_use]
    pub fn look_for(&self, screen: &str) -> Look {
        self.rows
            .iter()
            .find(|(name, _)| name == screen)
            .or_else(|| self.rows.iter().find(|(name, _)| name == "default"))
            .map_or_else(Look::default, |(_, look)| *look)
    }

    /// The row for one of this build's pages: its root is the disc's `Main
    /// Menu`, and every other page is drawn as `default`, the row the
    /// original's `Controls`, `SoundTest` and records screens fall back to
    /// only by carrying their own (`blur` 6). This build's pages are not the
    /// disc's screens, so those rows have nothing here to claim them.
    #[must_use]
    pub fn look_for_root(&self, root: bool) -> Look {
        self.look_for(if root { "Main Menu" } else { "default" })
    }

    /// How many screens author a row.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.rows.len()
    }

    /// Whether any row turns the band effect on. Not drawn either way; see the
    /// module docs.
    #[must_use]
    pub fn bands(&self) -> bool {
        self.bands
    }
}

fn is_true(text: &str) -> bool {
    text.trim().eq_ignore_ascii_case("true")
}

fn look_of(setting: &Node) -> Look {
    let number = |node: &Node, name: &str| {
        node.attr(name)
            .and_then(|text| text.trim().parse::<f32>().ok())
            .unwrap_or(0.0)
    };
    let main = setting.children_named("Main").next();
    // `ParseScreenSetting` clamps the levels to `0..=1` and the width to
    // `0..=24` (the literals at `0x008ac0d0`, `0x008ac0f8` and `0x008ac1b4`).
    let level = |name: &str| main.map_or(0.0, |main| number(main, name).clamp(0.0, 1.0));
    Look {
        blur: number(setting, "blur"),
        main: Levels {
            edge_level: level("edge_level"),
            fill_level: level("fill_level"),
            edge_width: main.map_or(0.0, |main| number(main, "edge_width").clamp(0.0, 24.0)),
        },
    }
}

/// One frame of the backdrop: what the renderer needs, and nothing it would
/// have to decide.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// Seconds into the scene's loop: the clock of its authored animation.
    pub seconds: f32,
    /// The working copy of the page's row at this frame.
    pub look: Look,
    /// The scene camera's combined view and projection, column-major, filled
    /// by the caller from the scene's camera at [`Self::seconds`].
    pub view_projection: [[f32; 4]; 4],
}

/// The widget's state: the clock and the eased working copy.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    widget: Widget,
    ticks: u64,
    current: Look,
}

impl Scene {
    /// A scene at its first frame, the working copy zero-filled as `Load`
    /// leaves it - so the picture fades in over about a second.
    #[must_use]
    pub fn new(widget: Widget) -> Self {
        Self {
            widget,
            ticks: 0,
            current: Look::default(),
        }
    }

    /// The widget this plays.
    #[must_use]
    pub fn widget(&self) -> &Widget {
        &self.widget
    }

    /// One frame on: the clock a step, the working copy a fraction closer to
    /// the row of the page showing.
    pub fn tick(&mut self, root: bool) {
        self.ticks += 1;
        self.current = self.current.eased_toward(self.widget.look_for_root(root));
    }

    /// Seconds into the loop.
    #[must_use]
    pub fn seconds(&self) -> f32 {
        loop_position(self.ticks as f32 / FRAMES_PER_SECOND, self.widget.anim_length)
    }

    /// The working copy now.
    #[must_use]
    pub fn look(&self) -> Look {
        self.current
    }

    /// The frame to draw, with the camera the caller derived from
    /// [`Self::seconds`].
    #[must_use]
    pub fn frame(&self, view_projection: [[f32; 4]; 4]) -> Frame {
        Frame {
            seconds: self.seconds(),
            look: self.current,
            view_projection,
        }
    }

    /// A frame at `seconds` into the loop with the page's row fully eased in:
    /// what a still of a settled page shows, where there is no frame count to
    /// have eased across.
    #[must_use]
    pub fn settled(&self, seconds: f32, root: bool, view_projection: [[f32; 4]; 4]) -> Frame {
        Frame {
            seconds: loop_position(seconds, self.widget.anim_length),
            look: self.widget.look_for_root(root),
            view_projection,
        }
    }
}

/// `fmodf(t, length)` for a non-negative `t`, and `0` for a loop with no
/// length.
#[must_use]
pub fn loop_position(seconds: f32, length: f32) -> f32 {
    if length > 0.0 {
        seconds.max(0.0) % length
    } else {
        0.0
    }
}

fn find<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    if node.name == name {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, name))
}

#[cfg(test)]
mod tests;
