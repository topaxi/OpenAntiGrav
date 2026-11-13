//! A shader a person wrote and the game reads at run time: a player's screen filter
//! (`oag_post::screen`) or a mod's fragment (`oag_fx::mod_shader`). What both share: the file's
//! `//!` TOML header, the `param_<name>()` accessors its tunables become, and validation with
//! naga before the device sees the text, so a bad file is a message and never wgpu's panic.
//!
//! naga is named directly rather than through `wgpu::naga`, which wgpu only re-exports where
//! it links wgpu-core: not on the web's WebGPU backend.

/// Splits a file into its `//!` header, with the markers stripped, and the rest.
///
/// The header is the run of lines at the top that start with `//!`; blank
/// lines inside it are kept, being TOML's own paragraph breaks. The first line
/// that is neither ends it.
#[must_use]
pub fn split_header(source: &str) -> (String, &str) {
    let mut header = String::new();
    let mut consumed = 0;
    for line in source.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("//!") {
            header.push_str(rest.trim_prefix(' '));
            if !rest.ends_with('\n') {
                header.push('\n');
            }
            consumed += line.len();
        } else if trimmed.trim().is_empty() && !header.is_empty() {
            // A blank line inside the header is a TOML paragraph break. The
            // one that ends the header is swallowed too, which costs the body
            // a blank line and nothing else.
            header.push('\n');
            consumed += line.len();
        } else {
            break;
        }
    }
    (header, &source[consumed..])
}

/// Whether `name` can be spliced into WGSL as an identifier.
#[must_use]
pub fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `fn param_<name>() -> f32` for each name, reading `array` (an array of `vec4<f32>`) in
/// declaration order.
#[must_use]
pub fn param_accessors<'a>(array: &str, names: impl Iterator<Item = &'a str>) -> String {
    let mut out = String::new();
    for (index, name) in names.enumerate() {
        out.push_str(&format!(
            "fn param_{name}() -> f32 {{ return {array}[{}u][{}u]; }}\n",
            index / 4,
            index % 4
        ));
    }
    out
}

/// Whether the textual `fn <name>(` is in `body`, whatever its spacing. Run before the parse,
/// whose own report for a missing function points at the prelude's call site: the one line of
/// the module the author did not write.
#[must_use]
pub fn defines(body: &str, name: &str) -> bool {
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .contains(&format!("fn {name}("))
}

/// Parses and validates `source` with naga, without a device.
///
/// # Errors
///
/// naga's annotated report, the source with the offending line marked under `path`.
pub fn validate(source: &str, path: &str) -> Result<naga::Module, String> {
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|error| error.emit_to_string_with_path(source, path))?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::default(),
    )
    .validate(&module)
    .map_err(|error| error.emit_to_string_with_path(source, path))?;
    Ok(module)
}

/// Runs `build`, whose device calls may fail on what naga could not see (a shader the backend
/// compiler refuses), and hands back the device's validation error rather than wgpu's panic.
///
/// **On the web nothing is caught**: the error scope resolves through a browser promise the
/// page's thread may not block on, so `build` runs bare and the browser logs what it refuses.
///
/// # Errors
///
/// The device's error, when it raised one.
pub fn scoped<T>(device: &wgpu::Device, build: impl FnOnce() -> T) -> Result<T, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = device;
        Ok(build())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let built = build();
        match pollster::block_on(scope.pop()) {
            Some(error) => Err(error.to_string()),
            None => Ok(built),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_stops_at_the_first_line_of_code() {
        let (header, body) = split_header("//! a = 1\n//! b = 2\n\nfn main() {}\n");
        assert_eq!(header, "a = 1\nb = 2\n\n");
        assert_eq!(body, "fn main() {}\n");
    }

    #[test]
    fn accessors_walk_the_vec4s_in_order() {
        let text = param_accessors("p", (0..5).map(|i| ["a", "b", "c", "d", "e"][i]));
        assert!(text.contains("fn param_a() -> f32 { return p[0u][0u]; }"));
        assert!(text.contains("fn param_e() -> f32 { return p[1u][0u]; }"));
    }

    #[test]
    fn a_bad_module_comes_back_as_the_report_naming_its_path() {
        let report = validate("fn broken( {", "my/file.wgsl").unwrap_err();
        assert!(report.contains("my/file.wgsl"), "{report}");
        assert!(validate("fn fine() {}", "x").is_ok());
    }
}
