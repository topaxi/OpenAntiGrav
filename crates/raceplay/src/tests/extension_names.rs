//! A mod names its shaders, effects, sounds and icons by their key in its manifest, never by
//! their place in name order: a file added ahead of the others in the manifest moves every
//! place and no command, and a key the manifest lacks faults the module, naming it.

use std::sync::Arc;

use oag_ext_abi::{Command, Distance, ModName, Rgba, Slot, SoundMode};
use oag_gameplay::PlayerInputs;

use super::extension_faults::scripted;
use super::extensions::grid;
use super::extensions::on::prepared;
use crate::extension_effects::Fired;
use crate::extensions::Source;

fn slot(index: usize) -> Slot {
    Slot::new(index).unwrap()
}

fn name(key: &str) -> ModName {
    ModName::new(key).unwrap()
}

fn table(keys: &[&str], ext: &str) -> Vec<(String, String)> {
    keys.iter()
        .map(|key| ((*key).to_string(), format!("{key}.{ext}")))
        .collect()
}

/// The text of the shader `key`: a file of its own, so a draw says which file it used.
fn shader_text(key: &str) -> String {
    format!("// {key}\nfn mod_fragment(input: ModInput) -> vec4<f32> {{ return input.colour; }}\n")
}

/// A mod sending `commands` whose manifest lists `keys` in each table, in that order.
fn manifest(commands: &[Command], keys: &[&str]) -> Source {
    let mut source = scripted("test.names", "", commands, &[]);
    source.shaders = table(keys, "wgsl");
    source.effects = table(keys, "toml");
    source.sounds = table(keys, "wav");
    source.files = Arc::new(|path| {
        let key = path.strip_suffix(".wgsl").unwrap_or(path);
        Ok(shader_text(key).into_bytes())
    });
    source
}

/// The two manifests of one mod: the key `target` is the second in one, and in the other a
/// file that sorts ahead of everything has been added, so it is the third.
const BEFORE: [&str; 2] = ["m-other", "target"];
const AFTER: [&str; 3] = ["0-added", "m-other", "target"];

#[test]
fn a_shader_is_the_file_its_key_names_when_another_sorts_ahead_of_it() {
    let fill = Command::ScreenFill {
        slot: slot(0),
        rgba: Rgba::new(0, 255, 0, 255),
        shader: Some(name("target")),
    };
    let param = Command::SetShaderParam {
        slot: slot(0),
        shader: name("target"),
        index: 1,
        value: 0.5,
    };
    for keys in [&BEFORE[..], &AFTER[..]] {
        let source = manifest(&[fill, param], keys);
        let mut race = grid(prepared(std::slice::from_ref(&source)));
        let shaders = crate::extension_shaders::load(
            std::slice::from_ref(&source),
            "wipeout-pulse",
            None,
            &mut Vec::new(),
        );
        race.view.handles = std::mem::take(&mut race.view.handles).with_shaders(shaders);
        race.tick(&PlayerInputs::none());
        let drawn = race.mod_fill().expect("a fill through the shader");
        let file = race.mod_shaders()[drawn.shader].as_ref().expect("loaded");
        assert!(
            file.compiled.wgsl.starts_with("// target\n"),
            "{keys:?}: the fill drew {:?}",
            file.compiled.wgsl.lines().next()
        );
        let params = race.mod_shader_params();
        assert_eq!(
            params[drawn.shader],
            [None, Some(0.5), None, None],
            "{keys:?}: the scalar lands on the same shader"
        );
    }
}

#[test]
fn a_sound_is_the_one_its_key_names_when_another_sorts_ahead_of_it() {
    let play = Command::PlayModSound {
        slot: slot(1),
        sound: name("target"),
        mode: SoundMode::Once,
    };
    let mut places = Vec::new();
    for keys in [&BEFORE[..], &AFTER[..]] {
        let mut race = grid(prepared(&[manifest(&[play], keys)]));
        race.tick(&PlayerInputs::none());
        let [event] = &race.drain_mod_sounds()[..] else {
            panic!("one sound");
        };
        places.push(event.index);
    }
    assert_eq!(
        places,
        [1, 2],
        "the key follows its file, the guest's call is the same"
    );
}

#[test]
fn an_effect_is_the_one_its_key_names_when_another_sorts_ahead_of_it() {
    let at_nozzle = Command::FireModEffect {
        slot: slot(1),
        effect: name("target"),
    };
    let ahead = Command::FireModEffectAhead {
        slot: slot(1),
        effect: name("target"),
        distance: Distance::Units(20.0),
    };
    for (command, keys, want) in [
        (at_nozzle, &BEFORE[..], 1),
        (at_nozzle, &AFTER[..], 2),
        (ahead, &BEFORE[..], 1),
        (ahead, &AFTER[..], 2),
    ] {
        let mut race = grid(prepared(&[manifest(&[command], keys)]));
        for _ in 0..60 {
            race.tick(&PlayerInputs::none());
        }
        race.tick_sim_only(&PlayerInputs::none());
        let fired = race.sim.extensions.effects_for_tests();
        let got = fired.iter().find_map(|(_, fired)| match fired {
            Fired::Own { index, .. } | Fired::OwnAt { index, .. } => Some(*index),
            Fired::Title(_) | Fired::Stop { .. } => None,
        });
        assert_eq!(got, Some(want), "{command:?} in {keys:?}");
    }
}

#[test]
fn a_key_the_manifest_lacks_faults_the_module_naming_it_and_the_table() {
    let beam = Command::DrawBeam {
        slot: slot(0),
        to: slot(1),
        rgba: Rgba::new(1, 2, 3, 4),
        half_width: 1.0,
        shader: Some(name("tagret")),
    };
    let wrong = [
        (
            "screen-fill",
            "[shaders]",
            Command::ScreenFill {
                slot: slot(0),
                rgba: Rgba::new(1, 2, 3, 4),
                shader: Some(name("tagret")),
            },
        ),
        ("draw-beam", "[shaders]", beam),
        (
            "set-shader-param",
            "[shaders]",
            Command::SetShaderParam {
                slot: slot(0),
                shader: name("tagret"),
                index: 0,
                value: 1.0,
            },
        ),
        (
            "fire-effect",
            "[effects]",
            Command::FireModEffect {
                slot: slot(0),
                effect: name("tagret"),
            },
        ),
        (
            "fire-mod-effect-ahead",
            "[effects]",
            Command::FireModEffectAhead {
                slot: slot(0),
                effect: name("tagret"),
                distance: Distance::Units(10.0),
            },
        ),
        (
            "play-mod-sound",
            "[sounds]",
            Command::PlayModSound {
                slot: slot(0),
                sound: name("tagret"),
                mode: SoundMode::Once,
            },
        ),
    ];
    for (kind, heading, command) in wrong {
        let mut race = grid(prepared(&[manifest(&[command], &BEFORE)]));
        race.tick(&PlayerInputs::none());
        let (_, why) = race.extension_reports()[0]
            .faulted
            .clone()
            .unwrap_or_else(|| panic!("{kind}: the module did not fault"));
        assert!(
            why.contains(kind) && why.contains("\"tagret\"") && why.contains(heading),
            "{kind}: {why}"
        );
        assert!(
            why.contains("target"),
            "{kind}: it lists what it declares: {why}"
        );
    }
}
