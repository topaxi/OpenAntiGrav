use super::*;
use wesl::resolver::VirtualResolver;

const GOOD: &str = "@fragment\nfn fs() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }\n";

#[test]
fn a_valid_shader_passes() {
    assert!(validate(GOOD, "good.wgsl").is_ok());
}

#[test]
fn an_undeclared_name_is_a_problem_that_points_at_it() {
    let source = GOOD.replace("1.0", "missing");
    let problem = validate(&source, "bad.wgsl").unwrap_err();
    assert!(
        problem.message.contains("bad.wgsl:2:"),
        "{}",
        problem.message
    );
    let range = problem.range.expect("a span");
    assert_eq!(&source[range], "missing");
}

#[test]
fn a_type_mismatch_the_parser_lets_through_is_a_problem() {
    let source = "@vertex\nfn vs() -> @builtin(position) vec4<f32> { let x = 1.0 * 2u; return vec4<f32>(x); }\n";
    assert!(validate(source, "t.wgsl").is_err());
}

#[test]
fn declarations_are_named_from_their_column_zero_line() {
    let cases = [
        ("fn fogged(a: f32) -> f32 {", Some("fogged")),
        ("@vertex fn vs_main() {", Some("vs_main")),
        (
            "@group(0) @binding(1) var<uniform> scene: Scene;",
            Some("scene"),
        ),
        ("var<storage, read_write> out: array<u32>;", Some("out")),
        ("struct Scene {", Some("Scene")),
        ("override linear_out: f32 = 0.0;", Some("linear_out")),
        ("    let x = 1.0;", None),
        ("}", None),
    ];
    for (line, name) in cases {
        assert_eq!(declared_name(line).as_deref(), name, "{line}");
    }
}

#[test]
fn a_failure_in_a_linked_module_names_the_module_and_its_declaration() {
    let mut resolver = VirtualResolver::new();
    resolver.add_module(
        "package::main".parse().unwrap(),
        "import package::helper::tint;\n@fragment\nfn fs() -> @location(0) vec4<f32> { return tint(); }\n"
            .into(),
    );
    resolver.add_module(
        "package::helper".parse().unwrap(),
        "fn tint() -> vec4<f32> {\n    let a = 1.0;\n    return vec4<f32>(a, exp(1u), 0.0, 1.0);\n}\n"
            .into(),
    );
    let options = CompileOptions {
        mangler: ManglerKind::None,
        ..Default::default()
    };
    let compiled = Compiler::new_with_resolver(options, resolver)
        .compile_module(&"package::main".parse().unwrap())
        .expect("links");
    let source = compiled.to_string();
    let problem = validate(&source, "g.wgsl").unwrap_err();
    let text = explain("g.wgsl", &source, &problem, compiled.sourcemap.as_ref());
    assert!(text.contains("inside `tint` from module"), "{text}");
    assert!(text.contains("declared at line 1"), "{text}");
    assert!(
        text.contains("`exp` first appears there at line 3"),
        "{text}"
    );
}
