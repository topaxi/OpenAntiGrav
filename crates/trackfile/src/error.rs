use std::path::Path;

/// Why a track file was refused. Every variant names what it refused.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A file could not be read or written.
    #[error("{path}: {message}")]
    Io {
        /// The file.
        path: String,
        /// The OS's message.
        message: String,
    },
    /// A path that has no usable file name.
    #[error("{0}: not a usable track manifest path")]
    Path(String),
    /// The manifest is not valid TOML of the expected shape.
    #[error("manifest: {0}")]
    Manifest(String),
    /// No `[toml-schema]` header, or one naming another kind or major.
    #[error("manifest: {0}")]
    Schema(#[from] oag_schema::Error),
    /// The glb container or its glTF document is malformed.
    #[error("glb: {0}")]
    Glb(String),
    /// A record the manifest counts is not in the glb, or the counts differ.
    #[error("{what}: the manifest says {manifest}, the glb has {glb}")]
    CountMismatch {
        /// What was counted.
        what: String,
        /// The manifest's count.
        manifest: usize,
        /// The glb's.
        glb: usize,
    },
    /// A collision mesh tag this version does not know.
    #[error("collision node {node}: unknown oag_kind {tag:?}")]
    UnknownKind {
        /// The node's index among the glb's collision nodes.
        node: usize,
        /// The tag found.
        tag: String,
    },
    /// An accessor that is not what the format asks for.
    #[error("{record}: attribute {attribute}: {why}")]
    Accessor {
        /// The record.
        record: String,
        /// The attribute.
        attribute: String,
        /// What is wrong.
        why: String,
    },
    /// A record node carries a transform; this version reads baked geometry.
    #[error("{record}: the node has a non-identity transform; apply it before export")]
    Transformed {
        /// The record.
        record: String,
    },
    /// A number the simulation would read is not finite.
    #[error("{record}: {field} is not finite")]
    NonFinite {
        /// The record.
        record: String,
        /// The field.
        field: String,
    },
    /// A reference or index out of range.
    #[error("{what}: {why}")]
    Range {
        /// What.
        what: String,
        /// Why.
        why: String,
    },
    /// A property or trigger a mod could not read: a string, an empty or clashing
    /// name, a list that does not match its pads, a cap passed.
    #[error("{what}: {why}")]
    Props {
        /// Where.
        what: String,
        /// Why.
        why: String,
    },
    /// The track cannot be held by this format.
    #[error("cannot write: {0}")]
    Unwritable(String),
}

impl Error {
    pub(crate) fn io(path: &Path, e: &std::io::Error) -> Self {
        Self::Io {
            path: path.display().to_string(),
            message: e.to_string(),
        }
    }
    pub(crate) fn accessor(record: &str, attribute: &str, why: impl Into<String>) -> Self {
        Self::Accessor {
            record: record.to_string(),
            attribute: attribute.to_string(),
            why: why.into(),
        }
    }
}
