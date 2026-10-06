//! Build-time shader validation for the render-side crates' `build.rs`.
//!
//! `naga` otherwise runs only inside `wgpu::Device::create_shader_module`, so
//! an undeclared name or a type mismatch surfaced at race start. Every shader
//! a crate ships goes through [`link`] (WESL, linked and then validated) or
//! [`check_dir`] (plain `.wgsl`), and a failure panics the build script, which
//! fails `cargo build` with the message built here.
//!
//! What is validated is what `wgpu` is handed: the same text, parsed with
//! `naga`'s WGSL front end and checked with every `ValidationFlags` on, under
//! no optional shader capabilities. The renderer requests none (its
//! `required_features` are texture compression and timestamps, which map to no
//! `naga` capability), so a shader that needs one fails here as it would there.
//!
//! A WESL artifact is printed from the syntax tree, so its line numbers belong
//! to no source file. The failure is therefore reported as the generated line
//! with context, the declaration it sits in, and the module that declaration
//! came from (via `wesl`'s source map, which stays populated under
//! `ManglerKind::None`), with the declaration's line in that module and the
//! first line of it that spells the offending text.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use naga::valid::{Capabilities, ValidationFlags, Validator};
use wesl::sourcemap::{BasicSourceMap, SourceMap};
use wesl::syntax::{GlobalDeclaration, GlobalDeclarationNode, ModulePath, PathOrigin};
use wesl::{
    CompileOptions, CompileResult, Compiler, ManglerKind,
    resolver::{FileResolver, Router, StandardResolver},
};

/// The package name a shader imports the shared modules under:
/// `import oag_shaders::fullscreen::fullscreen_triangle;`.
const SHARED: &str = "oag_shaders";

/// Where the shared modules live: this crate's own `shaders/`, found from its
/// manifest so every crate that calls [`link`] reads the same files.
const SHARED_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/shaders");

/// Lines of the text shown either side of the failing one.
const CONTEXT: usize = 3;

/// Links `module` under `shaders_dir` with `features` on, validates the result,
/// and writes it to `$OUT_DIR/<artifact>.wgsl` for `include_str!`.
///
/// Names are not mangled: an `override` constant, an entry point or a binding
/// must reach `wgpu` exactly as written.
///
/// # Panics
///
/// On a link error, which `wesl` reports with module and line, and on a shader
/// `naga` rejects, which [`explain`] reports.
pub fn link(shaders_dir: &str, artifact: &str, module: &str, features: &[&str]) {
    let mut options = CompileOptions {
        mangler: ManglerKind::None,
        ..Default::default()
    };
    for feature in features {
        options.features.set(*feature, true);
    }
    let mut router = Router::new();
    router.mount_resolver(
        ModulePath::new(PathOrigin::Package(SHARED.to_string()), Vec::new()),
        FileResolver::new(SHARED_DIR),
    );
    // `wesl` reports only the files under `shaders_dir` to cargo, so a shared
    // module an edit lands in would otherwise leave the artifact stale.
    println!("cargo::rerun-if-changed={SHARED_DIR}");
    router.mount_fallback_resolver(StandardResolver::new(shaders_dir));
    let compiler = Compiler::new_with_resolver(options, router);
    let mut compiled = compiler
        .compile_module(&module.parse().expect("a module path"))
        .inspect_err(|error| eprintln!("{error}"))
        .unwrap_or_else(|_| panic!("{module} did not compile"));
    compiled.emit_rerun_if_changed();
    stabilise_order(&mut compiled, module);
    let source = compiled.to_string();
    let generated = format!("{artifact}.wgsl (generated from {module})");
    if let Err(problem) = validate(&source, &generated) {
        let sources = compiled.sourcemap.as_ref();
        panic!("{}", explain(&generated, &source, &problem, sources));
    }
    compiled.write_artifact(artifact);
}

/// Puts the declarations in an order that does not change from build to build.
///
/// `wesl` visits the modules a shader imports through a `HashMap`, so the same
/// source printed its declarations in a different order on every build (the
/// text differed; sorted, it was equal). WGSL's module scope is order
/// independent, so this reorders and nothing else: the root module's
/// declarations first, then each imported module's by path, each keeping the
/// order its own source gave it (the sort is stable). A build that does not
/// change a shader now writes the same bytes, which is what lets two builds be
/// compared with `cmp`.
fn stabilise_order(compiled: &mut CompileResult, root: &str) {
    let Some(sources) = compiled.sourcemap.as_ref() else {
        return;
    };
    let origin = |declaration: &GlobalDeclarationNode| -> (bool, String) {
        let name = match &**declaration {
            GlobalDeclaration::Declaration(d) => d.ident.name().to_string(),
            GlobalDeclaration::TypeAlias(t) => t.ident.name().to_string(),
            GlobalDeclaration::Struct(s) => s.ident.name().to_string(),
            GlobalDeclaration::Function(f) => f.ident.name().to_string(),
            _ => return (true, String::new()),
        };
        let path = sources
            .item(&name)
            .map_or_else(String::new, |entry| entry.path.to_string());
        (path != root, path)
    };
    let mut keyed: Vec<_> = std::mem::take(&mut compiled.syntax.global_declarations)
        .into_iter()
        .map(|d| (origin(&d), d))
        .collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    compiled.syntax.global_declarations = keyed.into_iter().map(|(_, d)| d).collect();
}

/// Validates every `*.wgsl` under `dir` (recursively) except `skip`, a list of
/// paths relative to `dir` that are not complete modules on their own.
///
/// The walk, rather than a list, is what stops a new shader going unchecked.
///
/// # Panics
///
/// On the first shader `naga` rejects, or an unreadable directory.
pub fn check_dir(dir: &str, skip: &[&str]) {
    println!("cargo::rerun-if-changed={dir}");
    let root = Path::new(dir);
    let mut files = Vec::new();
    collect(root, &mut files);
    files.sort();
    for file in files {
        let relative = file.strip_prefix(root).expect("under the root");
        if skip.iter().any(|s| Path::new(s) == relative) {
            continue;
        }
        println!("cargo::rerun-if-changed={}", file.display());
        let source = std::fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", file.display()));
        let name = file.display().to_string();
        if let Err(problem) = validate(&source, &name) {
            panic!("{}", explain(&name, &source, &problem, None));
        }
    }
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "wgsl") {
            out.push(path);
        }
    }
}

/// What `naga` said about one shader: the rendered message and the byte range
/// of the first thing it pointed at.
#[derive(Debug)]
pub struct Problem {
    pub message: String,
    pub range: Option<std::ops::Range<usize>>,
}

/// Parses and validates `source` as `wgpu` would; `path` names it in the message.
///
/// # Errors
///
/// The parse or validation error, rendered against `source`.
pub fn validate(source: &str, path: &str) -> Result<(), Problem> {
    let module = naga::front::wgsl::parse_str(source).map_err(|e| Problem {
        message: e.emit_to_string_with_path(source, path),
        range: e
            .labels()
            .next()
            .map(|(span, _)| span.to_range().expect("a span")),
    })?;
    Validator::new(ValidationFlags::all(), Capabilities::empty())
        .validate(&module)
        .map(|_| ())
        .map_err(|e| Problem {
            message: e.emit_to_string_with_path(source, path),
            range: e.spans().next().and_then(|(span, _)| span.to_range()),
        })
}

/// The full failure text: naga's own, then where it lands.
#[must_use]
pub fn explain(
    what: &str,
    source: &str,
    problem: &Problem,
    sources: Option<&BasicSourceMap>,
) -> String {
    let mut out = format!("\nshader validation failed: {what}\n\n{}", problem.message);
    let Some(range) = problem.range.clone() else {
        return out;
    };
    let line_of = |offset: usize| source[..offset].matches('\n').count();
    let line = line_of(range.start);
    let lines: Vec<&str> = source.lines().collect();
    let _ = writeln!(out, "{what}, line {} with context:", line + 1);
    for (n, text) in lines
        .iter()
        .enumerate()
        .take(line + CONTEXT + 1)
        .skip(line.saturating_sub(CONTEXT))
    {
        let marker = if n == line { ">" } else { " " };
        let _ = writeln!(out, "{marker} {:>5} | {text}", n + 1);
    }
    let Some(sources) = sources else {
        return out;
    };
    let Some(declaration) = (0..=line).rev().find_map(|n| declared_name(lines[n])) else {
        return out;
    };
    let Some(entry) = sources.item(&declaration) else {
        let _ = writeln!(out, "inside `{declaration}` (not in the source map)");
        return out;
    };
    let module_name = sources
        .display_name(&entry.path)
        .map_or_else(|| entry.path.to_string(), str::to_string);
    let _ = write!(out, "inside `{}` from module {module_name}", entry.name);
    if let Some(text) = sources.source(&entry.path) {
        let module_lines: Vec<&str> = text.lines().collect();
        if let Some(start) = module_lines
            .iter()
            .position(|l| declared_name(l).as_deref() == Some(entry.name.as_str()))
        {
            let _ = write!(out, ", declared at line {}", start + 1);
            let culprit = source[range].trim();
            if !culprit.is_empty() && !culprit.contains('\n') {
                let end = module_lines
                    .iter()
                    .enumerate()
                    .skip(start + 1)
                    .find(|(_, l)| declared_name(l).is_some())
                    .map_or(module_lines.len(), |(n, _)| n);
                if let Some(hit) = module_lines[start..end]
                    .iter()
                    .position(|l| l.contains(culprit))
                {
                    let _ = write!(
                        out,
                        "; `{culprit}` first appears there at line {}",
                        start + hit + 1
                    );
                }
            }
        }
    }
    out.push('\n');
    out
}

/// The name a top-level declaration on this line introduces, if the line opens
/// one: it starts in column 0 and has a declaring keyword.
fn declared_name(line: &str) -> Option<String> {
    if line.starts_with([' ', '\t', '}', '/']) || line.is_empty() {
        return None;
    }
    let mut words = line
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty());
    while let Some(word) = words.next() {
        if matches!(
            word,
            "fn" | "struct" | "const" | "override" | "var" | "alias"
        ) {
            // `var<uniform> name`: the address space is a word of its own.
            let mut next = words.next()?;
            if word == "var"
                && matches!(
                    next,
                    "uniform"
                        | "storage"
                        | "private"
                        | "workgroup"
                        | "read"
                        | "write"
                        | "read_write"
                        | "handle"
                )
            {
                next = words.next()?;
                if matches!(next, "read" | "write" | "read_write") {
                    next = words.next()?;
                }
            }
            return Some(next.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests;
