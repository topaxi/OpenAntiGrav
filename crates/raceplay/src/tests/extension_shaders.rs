//! A mod's shaders in a race: the report names what did not load, `draw-beam` reaches the view's
//! surfaces between its two crafts, a fill naming a shader leaves the wash alone, and none of
//! it reaches the hash.

use std::sync::Arc;

use oag_core::math::Vec3;
use oag_ext_abi::{Command, Rgba, Slot};
use oag_gameplay::PlayerInputs;

use super::extension_faults::scripted;
use super::extensions::grid;
use super::extensions::on::prepared;
use crate::extension_shaders::ModShaders;
use crate::extensions::Source;

const GOOD: &str = "fn mod_fragment(input: ModInput) -> vec4<f32> { return input.colour; }\n";
const LOOPS: &str = "fn mod_fragment(input: ModInput) -> vec4<f32> { loop { if input.time < 0.0 { break; } } return input.colour; }\n";

fn slot(index: usize) -> Slot {
    Slot::new(index).unwrap()
}

/// A mod scripted to `commands`, its `[shaders]` a good file, one that loops and one missing.
fn shaded(commands: &[Command]) -> Source {
    let mut source = scripted("test.shaders", "", commands, &[]);
    source.shaders = [
        ("a-good", "a.wgsl"),
        ("b-loops", "b.wgsl"),
        ("c-gone", "c.wgsl"),
    ]
    .map(|(name, path)| (name.to_string(), path.to_string()))
    .to_vec();
    source.files = Arc::new(|path| match path {
        "a.wgsl" => Ok(GOOD.as_bytes().to_vec()),
        "b.wgsl" => Ok(LOOPS.as_bytes().to_vec()),
        other => Err(format!("{other}: not in the mod")),
    });
    source
}

fn load(source: &Source, report: &mut Vec<String>) -> ModShaders {
    crate::extension_shaders::load(std::slice::from_ref(source), "wipeout-pulse", None, report)
}

fn race(commands: &[Command]) -> crate::Race {
    let source = shaded(commands);
    let mut race = grid(prepared(std::slice::from_ref(&source)));
    let shaders = load(&source, &mut Vec::new());
    race.view.handles = std::mem::take(&mut race.view.handles).with_shaders(shaders);
    race
}

#[test]
fn a_file_that_does_not_load_keeps_its_place_and_the_report_says_why() {
    let mut report = Vec::new();
    let shaders = load(&shaded(&[]), &mut report);
    let loaded = |place: usize| shaders.list()[place].is_some();
    assert_eq!(shaders.list().len(), 4, "the flat draw and three places");
    assert!(loaded(0) && loaded(1) && !loaded(2) && !loaded(3));
    assert_eq!(shaders.place("test.shaders", 1), Some(2));
    assert_eq!(shaders.place("test.shaders", 3), None);
    assert_eq!(shaders.place("another.mod", 0), None);
    let said = report.join("\n");
    assert!(said.contains("shader a-good: a.wgsl loaded"), "{said}");
    assert!(said.contains("shader b-loops left out"), "{said}");
    assert!(said.contains("`mod_fragment` has a loop"), "{said}");
    assert!(
        said.contains("shader c-gone left out: c.wgsl: not in the mod"),
        "{said}"
    );
    assert!(said.contains("draws nothing"), "{said}");
}

#[test]
fn a_race_with_no_mod_lists_no_shader_and_builds_no_stage() {
    let shaders = crate::extension_shaders::load(&[], "wipeout-pulse", None, &mut Vec::new());
    assert!(shaders.list().is_empty());
}

fn beam(shader: Option<u16>) -> Command {
    Command::DrawBeam {
        slot: slot(1),
        to: slot(2),
        rgba: Rgba::new(255, 128, 0, 255),
        half_width: 1.5,
        shader,
    }
}

#[test]
fn a_beam_reaches_the_views_surfaces_between_its_two_crafts() {
    let mut race = race(&[beam(None), beam(Some(0))]);
    race.tick(&PlayerInputs::none());
    let eye = Vec3::new(0.0, 500.0, 0.0);
    let surfaces = race.mod_surfaces(Vec3::X, Vec3::Y, eye);
    let places: Vec<usize> = surfaces.iter().map(|(place, _)| *place).collect();
    assert_eq!(places, [0, 1], "flat, then the mod's first shader");
    let ends = [1, 2].map(|slot| race.ship_model_matrix_of(slot).w_axis.truncate());
    let (_, flat) = &surfaces[0];
    let first = Vec3::from_array(flat[0].position);
    let last = Vec3::from_array(flat[flat.len() - 2].position);
    assert!((first - ends[0]).length() <= 1.6, "{first:?} {:?}", ends[0]);
    assert!((last - ends[1]).length() <= 1.6, "{last:?} {:?}", ends[1]);
    assert_eq!(flat[0].colour[0], 1.0);
    assert_eq!(flat[0].colour[3], 1.0);
}

/// Out of the world at either end is no beam. A beam is this tick's: a module that stops
/// sending it stops drawing it.
#[test]
fn a_beam_draws_nothing_with_a_craft_out_of_the_world() {
    let mut race = race(&[beam(None)]);
    race.tick(&PlayerInputs::none());
    let eye = Vec3::new(0.0, 500.0, 0.0);
    assert_eq!(race.mod_surfaces(Vec3::X, Vec3::Y, eye).len(), 1);
    race.sim.world.ships[2].active = false;
    assert!(race.mod_surfaces(Vec3::X, Vec3::Y, eye).is_empty());
}

#[test]
fn a_shaded_fill_leaves_the_wash_alone_and_a_scalar_lands_on_its_shader() {
    let fill = Command::ScreenFill {
        slot: slot(0),
        rgba: Rgba::new(0, 255, 0, 255),
        shader: Some(0),
    };
    let param = Command::SetShaderParam {
        slot: slot(0),
        shader: 0,
        index: 2,
        value: 0.75,
    };
    let other_view = Command::SetShaderParam {
        slot: slot(1),
        shader: 0,
        index: 0,
        value: 9.0,
    };
    let mut race = race(&[fill, param, other_view]);
    race.tick(&PlayerInputs::none());
    assert_eq!(
        race.screen_wash(),
        None,
        "the shader draws it, not the wash"
    );
    let drawn = race.mod_fill().expect("a fill through the shader");
    assert_eq!((drawn.shader, drawn.colour), (1, [0.0, 1.0, 0.0, 1.0]));
    let params = race.mod_shader_params();
    assert_eq!(
        params[1],
        [None, None, Some(0.75), None],
        "this view's alone"
    );
}

/// A fill naming a place the mod does not have is no fill and no wash: never the flat colour.
#[test]
fn a_fill_naming_no_shader_of_the_mods_draws_nothing() {
    let fill = Command::ScreenFill {
        slot: slot(0),
        rgba: Rgba::new(0, 255, 0, 255),
        shader: Some(40),
    };
    let mut race = race(&[fill]);
    race.tick(&PlayerInputs::none());
    assert_eq!(race.screen_wash(), None);
    assert_eq!(race.mod_fill(), None);
}

#[test]
fn shader_draws_never_reach_the_hash() {
    let fill = Command::ScreenFill {
        slot: slot(0),
        rgba: Rgba::new(0, 255, 0, 255),
        shader: Some(0),
    };
    let param = Command::SetShaderParam {
        slot: slot(0),
        shader: 0,
        index: 0,
        value: 1.0,
    };
    let hashes = |commands: &[Command]| {
        let mut race = race(commands);
        (0..30)
            .map(|_| {
                race.tick(&PlayerInputs::none());
                race.sim.state_hash()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(hashes(&[]), hashes(&[beam(Some(0)), fill, param]));
}
