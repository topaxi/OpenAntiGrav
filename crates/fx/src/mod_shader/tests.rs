//! The file a mod ships, accepted or refused by name, with no device.

use super::*;

const GOOD: &str = "//! blend = \"add\"\n//! texture = \"fx/x.png\"\n//!\n//! [[param]]\n//! name = \"speed\"\n//! default = 2.5\n\nfn mod_fragment(input: ModInput) -> vec4<f32> {\n    return input.colour * param_speed() + mod_texture(input.uv);\n}\n";

#[test]
fn the_host_text_has_the_one_fragment_a_mod_replaces() {
    assert_eq!(HOST.matches(MOD_FRAGMENT).count(), 1, "{HOST}");
    for helper in [
        "fn mod_texture(",
        "fn engine_texture(",
        "fn engine_uv(",
        "fn engine_texture_bound(",
        "fn craft_frame(",
        "fn eye_ray(",
        "fn view_aspect(",
        "fn fs_backdrop(",
        "fn fs_fringe(",
    ] {
        assert!(HOST.contains(helper), "the link dropped {helper}");
    }
}

#[test]
fn a_shader_reads_its_header_and_builds_on_the_contract() {
    let shader = compile("fx/x.wgsl", GOOD).unwrap();
    assert_eq!(shader.blend, Blend::Add);
    assert_eq!(shader.texture.as_deref(), Some("fx/x.png"));
    assert_eq!(shader.defaults, [2.5, 0.0, 0.0, 0.0]);
    assert!(shader.wgsl.starts_with("//! blend"), "the mod's text first");
}

#[test]
fn a_header_says_a_fill_through_the_shader_is_a_backdrop() {
    assert!(
        !compile("fx/x.wgsl", GOOD).unwrap().backdrop,
        "over with none"
    );
    let behind = format!("//! fill = \"backdrop\"\n{GOOD}");
    assert!(compile("fx/x.wgsl", &behind).unwrap().backdrop);
    let unknown = format!("//! fill = \"under\"\n{GOOD}");
    assert!(compile("fx/x.wgsl", &unknown).is_err());
    let reads = "fn mod_fragment(input: ModInput) -> vec4<f32> {\n    let at = craft_frame()[3].xyz + eye_ray(input.screen_uv);\n    return vec4<f32>(at, 1.0);\n}\n";
    compile("fx/x.wgsl", reads).expect("the craft's frame and the eye's ray are there to read");
}

#[test]
fn the_flat_draw_validates_as_it_stands() {
    oag_gpu::shader_file::validate(&flat().wgsl, "flat").unwrap();
}

#[test]
fn a_syntax_error_comes_back_as_nagas_report_on_the_mods_own_line() {
    let why = compile(
        "fx/bad.wgsl",
        "// one\n// two\nfn mod_fragment(input: ModInput) -> vec4<f32> { return oops; }\n",
    )
    .unwrap_err();
    assert!(why.contains("fx/bad.wgsl:3:"), "{why}");
    assert!(why.contains("oops"), "{why}");
}

#[test]
fn a_file_with_no_fragment_is_told_so() {
    let why = compile("x.wgsl", "fn other() {}\n").unwrap_err();
    assert!(why.contains("defines no `fn mod_fragment"), "{why}");
}

/// Every loop form WGSL has, each refused naming its function, and in a helper as well as in
/// the fragment.
#[test]
fn a_loop_is_refused_whichever_way_it_is_written() {
    for body in [
        "loop { }",
        "while (true) { }",
        "for (var i = 0; i < 4; i++) { }",
        "if (input.time > 0.0) { loop { break; } }",
    ] {
        let source = format!(
            "fn helper(input: ModInput) {{ {body} }}\nfn mod_fragment(input: ModInput) -> vec4<f32> {{ helper(input); return input.colour; }}\n"
        );
        let why = compile("loop.wgsl", &source).unwrap_err();
        assert!(why.contains("`helper` has a loop"), "{body}: {why}");
    }
}

#[test]
fn what_the_host_does_not_provide_is_refused_before_the_device_sees_it() {
    let fragment = "fn mod_fragment(input: ModInput) -> vec4<f32> { return input.colour; }\n";
    for (extra, named) in [
        (
            "@group(1) @binding(0) var<uniform> mine: f32;\n",
            "@group(1)",
        ),
        (
            "@group(0) @binding(9) var<uniform> mine: f32;\n",
            "@binding(9)",
        ),
        ("override mine: f32;\n", "`mine`"),
        (
            "@fragment fn other() -> @location(0) vec4<f32> { return vec4<f32>(); }\n",
            "entry points",
        ),
        ("var<private> mine: array<f32, 1000000>;\n", "array"),
    ] {
        let why = compile("x.wgsl", &format!("{extra}{fragment}")).unwrap_err();
        assert!(why.contains(named), "{extra}: {why}");
    }
}

#[test]
fn a_header_past_the_contract_is_refused_by_name() {
    let params = "//! [[param]]\n//! name = \"a\"\n".repeat(5);
    let why = compile("x.wgsl", &format!("{params}fn mod_fragment() {{}}")).unwrap_err();
    assert!(why.contains("5 params"), "{why}");
    let why = compile("x.wgsl", "//! colour = 1\nfn mod_fragment() {}").unwrap_err();
    assert!(why.contains("the header"), "{why}");
    let why = compile("x.wgsl", &"/".repeat(70_000)).unwrap_err();
    assert!(why.contains("bytes"), "{why}");
}

/// No loop, and still a billion calls: each helper calls the next one twice. The work is
/// counted through the calls, so the chain is refused by name where it passes the bound, and
/// the same helpers called in a row, which is what a loop unrolls to, are not.
#[test]
fn helpers_that_multiply_through_their_calls_are_refused() {
    let fragment =
        "fn mod_fragment(input: ModInput) -> vec4<f32> { return vec4<f32>(f0(input.uv.x)); }\n";
    let mut doubling = String::from("fn f30(x: f32) -> f32 { return x * 0.5; }\n");
    for n in (0..30).rev() {
        doubling += &format!(
            "fn f{n}(x: f32) -> f32 {{ return f{next}(x) + f{next}(x + 1.0); }}\n",
            next = n + 1
        );
    }
    assert!(doubling.len() < 4096, "a small file: {}", doubling.len());
    let why = compile("x.wgsl", &format!("{doubling}{fragment}")).unwrap_err();
    assert!(why.contains("operations through the calls"), "{why}");

    let mut flat = String::from("fn f30(x: f32) -> f32 { return x * 0.5; }\n");
    for n in (0..30).rev() {
        flat += &format!("fn f{n}(x: f32) -> f32 {{ return f30(x) + f30(x + 1.0); }}\n");
    }
    let calls: String = (0..30).map(|n| format!(" + f{n}(input.uv.y)")).collect();
    let fragment = format!(
        "fn mod_fragment(input: ModInput) -> vec4<f32> {{ return vec4<f32>(0.0{calls}); }}\n"
    );
    compile("x.wgsl", &format!("{flat}{fragment}")).expect("sixty calls in a row are sixty calls");
}

/// A circle on `screen_uv` needs the view's shape: the helper is there and a shader can call it.
#[test]
fn a_shader_can_ask_for_the_views_aspect() {
    let round = "fn mod_fragment(input: ModInput) -> vec4<f32> {\n    let across = (input.screen_uv - vec2<f32>(0.5)) * vec2<f32>(view_aspect(), 1.0);\n    return input.colour * select(0.0, 1.0, length(across) < 0.4);\n}\n";
    assert!(compile("fx/round.wgsl", round).is_ok());
}
