//! A `<Mode3D>` widget's `<Model>` children, collected so a screen that
//! places a 3D model can draw it - `EndRace Rewards`' `TrophyPanel` on
//! Pulse, three trophies at one origin.
//!
//! The camera numbers are `Mode3D_ReadValues`' (`0x088b7c8c` in Pulse's
//! `BOOT.BIN`): `OriginX`/`OriginY` default `0`, `nearZ`/`farZ` default
//! `20.0`/`100.0`, the defaults `docs/ghidra/functions/psp-pulse-usa/race-box-screens.md`'s
//! "`Mode3D`'s own fixed camera" section records off the widget's own
//! constructor. The pose is the `Model`'s own `x y z RotX RotY`.
//! `crate::picker::slideshow`'s reader predates those defaults and keeps
//! its own; this one applies them because `TrophyPanel` authors no depth
//! at all.

use super::{Node, Screens};
use crate::picker::slideshow::Model;

/// One `<Model>` inside a `<Mode3D>`, with its parent's camera.
#[derive(Debug, Clone, PartialEq)]
pub struct Mode3dModel {
    /// The `Model`'s own `name` - `g_trophy`/`s_trophy`/`b_trophy`.
    pub name: Option<String>,
    /// Source, pose and camera, in the shape
    /// `oag_game::preview::mode3d_view_projection` takes.
    pub model: Model,
    /// `StartPaused="yes"` - the model's own animation holds at its first
    /// frame until code releases it.
    pub start_paused: bool,
}

/// `Mode3D_ReadValues`' `nearZ` default.
pub const DEFAULT_NEAR_Z: f32 = 20.0;
/// `Mode3D_ReadValues`' `farZ` default.
pub const DEFAULT_FAR_Z: f32 = 100.0;

impl Screens {
    pub(super) fn mode3d_models_from_node(&self, node: &Node) -> Vec<Mode3dModel> {
        let number = |n: &Node, key: &str| self.number(n.value(key));
        let origin = [
            number(node, "OriginX").unwrap_or(0.0),
            number(node, "OriginY").unwrap_or(0.0),
        ];
        let depth = [
            number(node, "nearZ").unwrap_or(DEFAULT_NEAR_Z),
            number(node, "farZ").unwrap_or(DEFAULT_FAR_Z),
        ];
        node.children_named("Model")
            .filter_map(|model| {
                let src = model.value("src")?.to_string();
                let at = |key: &str| number(model, key).unwrap_or(0.0);
                Some(Mode3dModel {
                    name: model.attr("name").map(str::to_string),
                    model: Model {
                        src,
                        position: [at("x"), at("y"), at("z")],
                        rotation: [at("RotX"), at("RotY")],
                        origin,
                        depth,
                    },
                    start_paused: model.value("StartPaused").is_some_and(|value| {
                        value.eq_ignore_ascii_case("yes") || value.eq_ignore_ascii_case("true")
                    }),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::screen::Screens;

    /// `TrophyPanel`'s own shape: an origin, no depth (so
    /// `Mode3D_ReadValues`' `20`/`100`), and three named models.
    #[test]
    fn a_mode3d_s_models_carry_its_camera_and_its_defaults() {
        let screens = Screens::from_xml(
            r#"<Screen name="EndRace Rewards">
<Mode3D name="TrophyPanel"><Values OriginX="145.0" OriginY="60.0"></Values>
<Model name="g_trophy"><Values src="Data\FE\trophies\gold.vex" x="0.0" y="0.0" z="-75.0" StartPaused="yes"></Values></Model>
<Model name="s_trophy"><Values src="Data\FE\trophies\silver.vex" x=".0" y="0.0" z="-75.0"></Values></Model>
</Mode3D>
</Screen>"#,
        );
        let models = &screens.screens[0].models;
        assert_eq!(models.len(), 2);
        let gold = &models[0];
        assert_eq!(gold.name.as_deref(), Some("g_trophy"));
        assert_eq!(gold.model.src, r"Data\FE\trophies\gold.vex");
        assert_eq!(gold.model.position, [0.0, 0.0, -75.0]);
        assert_eq!(gold.model.origin, [145.0, 60.0]);
        assert_eq!(gold.model.depth, [20.0, 100.0]);
        assert!(gold.start_paused);
        assert!(!models[1].start_paused);
    }
}
