//! A texture fetch that runs only where the interpolated tangent's length says so takes explicit
//! gradients, and the sun terms the shadowed redraw needs travel with the lit colour.
//!
//! The tangent is a smooth varying, so `dot(tangent, tangent) > 0.25` is not constant across a
//! primitive: an implicit-level fetch (or a `dpdx`) behind it has derivatives in non-uniform control
//! flow. `lit_texel`'s other guards are flat or constant per submesh (`shade.wesl`, the comment on
//! `lit_colour`; `docs/reviews/2026-11-23-rendering-shaders-review.md`, R1). Needs no GPU and no
//! game content: it reads the shader sources, so it fails the moment a conversion is dropped.

const SHADE: &str = include_str!("../shaders/shade.wesl");
const OMEGA_SNOW: &str = include_str!("../shaders/omega_snow.wesl");
const OMEGA_ROCK: &str = include_str!("../shaders/omega_rock.wesl");
const SHADOW: &str = include_str!("../shaders/shadow.wesl");
const BINDINGS: &str = include_str!("../shaders/bindings.wesl");
const MESH: &str = include_str!("../shaders/mesh.wesl");

/// Source lines of `text` that call `material_sample(` (the implicit-level fetch), by line number.
fn implicit_fetches(text: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| {
            let code = line.split("//").next().unwrap_or("");
            code.contains("material_sample(") && !code.contains("fn material_sample(")
        })
        .map(|(n, _)| n + 1)
        .collect()
}

#[test]
fn omega_snow_and_rock_fetch_with_explicit_gradients() {
    for (name, text) in [
        ("omega_snow.wesl", OMEGA_SNOW),
        ("omega_rock.wesl", OMEGA_ROCK),
    ] {
        assert_eq!(
            implicit_fetches(text),
            Vec::<usize>::new(),
            "{name} is only reached under the tangent-length test; its fetches take gradients"
        );
        assert!(
            text.contains("material_sample_grad("),
            "{name} must fetch through material_sample_grad"
        );
    }
}

#[test]
fn the_hull_normal_map_and_the_panel_weight_fetch_with_explicit_gradients() {
    assert!(
        SHADE.contains(
            "let map = material_sample_grad(MATERIAL_PAD_MASK, in.texcoord, uv_grad[0], uv_grad[1]);"
        ),
        "the hull normal map's fetch sits behind the tangent-length test"
    );
    assert!(
        SHADE.contains(
            "weight = material_sample_grad(MATERIAL_SIXTH_MAP, in.texcoord, uv_grad[0], uv_grad[1]).rgb;"
        ),
        "the panel-map road's specular weight sits behind `panel`"
    );
}

#[test]
fn the_panels_other_side_does_not_hold_an_implicit_fetch() {
    // `first = material_sample(MATERIAL_ALBEDO, first_uv)` runs ahead of the `panel` test; the
    // `panel` branch overwrites it with the marched coordinate's fetch.
    let hoisted = SHADE
        .find("first = material_sample(MATERIAL_ALBEDO, first_uv);")
        .expect("the ordinary albedo fetch");
    let after = &SHADE[hoisted..];
    let panel_test = after
        .find("if panel {")
        .expect("the panel override follows");
    assert!(
        after[..panel_test].lines().count() < 4,
        "the albedo fetch must sit directly ahead of the `panel` test, not in an else of it"
    );
    assert!(
        !SHADE.contains("} else if panel {\n        first ="),
        "`first` must not be assigned in a branch of the `panel` test"
    );
}

#[test]
fn the_shadowed_redraw_takes_its_sun_terms_from_the_lit_pixel() {
    for name in ["sun_delta", "sun_known", "shadow_exempt"] {
        assert!(
            !BINDINGS.contains(&format!("var<private> {name}")),
            "{name} is per-invocation private state again"
        );
        assert!(
            !SHADOW.contains(name),
            "shadowed reads {name} instead of its `SunTerms`"
        );
    }
    assert!(SHADOW.contains("fn shadowed(colour: vec3<f32>, sun: SunTerms,"));
    assert!(SHADE.contains("fn lit_texel(raw: VertexOutput) -> Lit {"));
    for call in MESH.lines().filter(|l| l.contains("shadowed(")) {
        assert!(
            call.contains(".sun"),
            "a shadowed() call without the lit pixel's terms: {call}"
        );
    }
}
