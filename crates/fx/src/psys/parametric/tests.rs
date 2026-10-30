use oag_core::Rng;

use super::*;
use crate::psys::System;

const HEADER: &str = "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/effect.tosd\", version = \"1.0.0\" }\n";

const GOOD: &str = r##"
axis = "rear"
[[emitter]]
name = "arc"
duration = 6
per-emission = [2, 4]
lifetime = [12, 4]
speed = [0.8, 0.3]
spread = 25
drag = 0.95
size = [[0.0, 0.05], [1.0, 0.01]]
alpha = [[0.0, 1.0], [1.0, 0.0]]
colour = [[0.0, "#ffffff"], [0.3, "#ff3010"], [1.0, "#400000"]]
"##;

fn none(_: &str) -> Option<Sprite> {
    None
}

fn file(body: &str) -> String {
    format!("{HEADER}{body}")
}

#[test]
fn an_effect_file_plays_through_the_same_system_as_a_disc_effect() {
    let parsed = parse(&file(GOOD), &mut none).expect("a good file parses");
    assert_eq!(parsed.axis, Axis::Rear);
    assert_eq!(parsed.effect.emitters.len(), 1);
    assert_eq!(parsed.effect.roots(), &[0]);

    let mut rng = Rng::new(7);
    let mut system = System::new();
    system.ignite(&parsed.effect, Vec3::ZERO, 1.0);
    let mut most = 0;
    for _ in 0..30 {
        system.advance(&parsed.effect, 1.0 / 60.0, Vec3::ZERO, Vec3::Y, &mut rng);
        most = most.max(system.alive_count());
    }
    assert!(most > 0, "the emitter never put a particle in the air");
    for _ in 0..60 {
        system.advance(&parsed.effect, 1.0 / 60.0, Vec3::ZERO, Vec3::Y, &mut rng);
    }
    assert_eq!(system.alive_count(), 0, "a 6-tick emitter died out");
}

#[test]
fn the_gradient_is_baked_over_a_particles_life() {
    let parsed = parse(&file(GOOD), &mut none).unwrap();
    let palette = &parsed.effect.emitters[0].palette;
    assert_eq!(palette[0][..3], [1.0, 1.0, 1.0]);
    let mid = palette[(0.3_f32 * 255.0).round() as usize];
    assert!((mid[0] - 1.0).abs() < 0.01 && mid[1] < 0.3, "{mid:?}");
    assert!(palette[255][0] < 0.3 && palette[255][1] == 0.0);
}

#[test]
fn a_broken_file_is_refused_by_name() {
    let cases = [
        ("not toml", "[[emitter".to_string(), "TOML"),
        ("no header", GOOD.to_string(), "toml-schema"),
        ("no emitter", file("axis = \"up\"\n"), "no [[emitter]]"),
        ("unknown key", file(&GOOD.replace("drag", "dragg")), "dragg"),
        (
            "zero interval",
            file(&GOOD.replace("duration = 6", "duration = 6\ninterval = [0, 1]")),
            "interval",
        ),
        (
            "keys out of order",
            file(&GOOD.replace("[1.0, 0.01]", "[0.0, 0.01]")),
            "ascend",
        ),
        (
            "bad colour",
            file(&GOOD.replace("#400000", "red")),
            "colour",
        ),
        (
            "nan",
            file(&GOOD.replace("speed = [0.8, 0.3]", "speed = [nan, 0.3]")),
            "finite",
        ),
        (
            "a texture the mod lacks",
            file(&GOOD.replace("drag = 0.95", "drag = 0.95\ntexture = \"fx/none.png\"")),
            "fx/none.png",
        ),
    ];
    for (what, text, names) in cases {
        let error = parse(&text, &mut none).expect_err(what);
        assert!(
            error.to_string().contains(names),
            "{what}: {error} should name {names}"
        );
    }
}

#[test]
fn a_texture_of_the_mod_reaches_the_emitter() {
    let sprite = Sprite {
        width: 1,
        height: 1,
        rgba: vec![255; 4].into(),
    };
    let text = file(&GOOD.replace("drag = 0.95", "drag = 0.95\ntexture = \"fx/glow.png\""));
    let parsed = parse(&text, &mut |path| {
        (path == "fx/glow.png").then(|| sprite.clone())
    })
    .unwrap();
    assert_eq!(parsed.effect.emitters[0].sprite.as_ref(), Some(&sprite));
}
