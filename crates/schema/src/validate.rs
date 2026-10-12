//! The reference validator over our own schemas: the gate's and the drift
//! tests' view of "does the schema accept this text".

use std::path::{Path, PathBuf};

use toml_schema::Schema;

/// The directory the shipped schemas live in, `schemas/` at the repository root.
#[must_use]
pub fn schemas_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas")
}

/// Loads the schema for `kind` at `major`.
///
/// # Panics
///
/// If the schema file is missing or is not a valid schema: a shipped schema
/// that does not load is a build defect, not a verdict.
#[must_use]
pub fn schema(kind: &str, major: u32) -> Schema {
    let path = schemas_dir().join(format!("v{major}/{kind}.tosd"));
    Schema::load(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// `Ok` when the `kind` v`major` schema accepts `text`, else every diagnostic.
///
/// # Errors
///
/// The validator's messages, or the TOML parse error.
pub fn accepts(kind: &str, major: u32, text: &str) -> Result<(), Vec<String>> {
    let table: toml::Table = text
        .parse()
        .map_err(|e: toml::de::Error| vec![e.to_string()])?;
    let result = schema(kind, major).validate(&table);
    if result.valid() {
        Ok(())
    } else {
        Err(result
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.path, e.message))
            .collect())
    }
}
