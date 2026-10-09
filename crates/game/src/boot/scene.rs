//! The HD-style menu backdrop's assets: the widget off the skin, the scene it
//! names, and the camera that scene authors - what `BackgroundAnim_Load`
//! leaves the widget holding when the Fury style does not claim the screen.
//!
//! Its own file rather than a stretch of `boot.rs`, for the reason `fury.rs`
//! gives: that file is baselined by `scripts/check-file-size.py` and may shrink
//! but not grow.

use oag_core::math::{Mat4, camera};
use oag_mesh::mesh::{self, Model};
use oag_ui::scene_backdrop::{FOV_Y, Widget};
use oag_vex::vex::{self, Node};

/// Everything the scene backdrop needs, read once at boot and shared with the
/// menus by `Arc`.
#[derive(Debug)]
pub struct SceneAssets {
    /// The widget as the skin authors it.
    pub widget: Widget,
    /// The scene, decoded for the same mesh pass every menu model draws in.
    pub model: Model,
    data: Vec<u8>,
    nodes: Vec<Node>,
    camera: usize,
}

impl SceneAssets {
    /// The scene camera's combined view and projection at `seconds` into its
    /// loop, at `aspect`: the camera node's world matrix through the animated
    /// chain above it, inverted, under the widget's own clip planes and
    /// [`FOV_Y`]. Column-major, for the GPU.
    #[must_use]
    pub fn view_projection(&self, seconds: f32, aspect: f32) -> [[f32; 4]; 4] {
        let world = vex::world_transforms_at(&self.data, &self.nodes, seconds)
            .get(self.camera)
            .copied()
            .unwrap_or(vex::IDENTITY);
        // The file's matrices are row-major with the translation in row 3
        // (`v' = v * M`); read as columns they are the transpose, which is the
        // column-vector form glam wants.
        let view = Mat4::from_cols_array(&world).inverse();
        let [near, far] = self.widget.depth;
        (camera::perspective(FOV_Y, aspect, near, far) * view).to_cols_array_2d()
    }
}

/// The scene's model and a line saying how it was built.
///
/// Omega ships the scene's geometry in a PS4 `.rcsmodel` and its motion in the
/// `.vex` beside it, which is what [`mesh::rcs::psp2::build_with_vex`] joins;
/// any other source goes through the ordinary preview path.
fn build_model(
    archives: &mut oag_assets::Archives,
    src: &str,
    vex: &[u8],
) -> anyhow::Result<(Model, String)> {
    let geometry = mesh::rcs::sibling_name(src).and_then(|name| archives.read_name(&name).ok());
    if let Some(geometry) = geometry
        && let Some(built) = crate::preview::psp2_scene::build(archives, src, vex, &geometry)?
    {
        return Ok(built);
    }
    let model = crate::preview::model(archives, src)?;
    let triangles = model.indices.len() / 3;
    Ok((model, format!("{triangles} triangles")))
}

/// Reads the scene `widget` names and finds its camera, or says why not.
///
/// `None` with a report line for every reason: the scene will not read, will
/// not decode, or authors no camera while the widget asks for the model's
/// own (`UseModelCamera`, true on every row this project has read). Without
/// the camera the widget's own `x y z RotX RotY` pose would be the view, and
/// that path is unread, so nothing is drawn rather than a guess.
pub(super) fn load(
    archives: &mut oag_assets::Archives,
    widget: Widget,
    report: &mut Vec<String>,
) -> Option<SceneAssets> {
    let src = widget.src.clone();
    let data = match archives.read_name(&src) {
        Ok(data) => data,
        Err(error) => {
            report.push(format!("menu backdrop: {src}: {error:#} - nothing drawn"));
            return None;
        }
    };
    let nodes = match vex::nodes(&data) {
        Ok(nodes) => nodes,
        Err(error) => {
            report.push(format!("menu backdrop: {src} does not parse: {error}"));
            return None;
        }
    };
    if !widget.model_camera {
        report.push(format!(
            "menu backdrop: {src}: the widget does not use the model's camera, and its own pose is unread - nothing drawn"
        ));
        return None;
    }
    let Some(camera) = nodes
        .iter()
        .position(|node| node.class_id == oag_vex::camera::CLASS_CAMERA)
    else {
        report.push(format!(
            "menu backdrop: {src} authors no camera - nothing drawn"
        ));
        return None;
    };
    let (model, built) = match build_model(archives, &src, &data) {
        Ok(pair) => pair,
        Err(error) => {
            report.push(format!("menu backdrop: {src}: {error:#} - nothing drawn"));
            return None;
        }
    };
    report.push(format!(
        "menu backdrop: HD-style scene {src}, {built}, {} screen rows, loops {} s{}",
        widget.rows(),
        widget.anim_length,
        if widget.bands() {
            "; a row turns bands on, which are not drawn"
        } else {
            ""
        },
    ));
    Some(SceneAssets {
        widget,
        model,
        data,
        nodes,
        camera,
    })
}
