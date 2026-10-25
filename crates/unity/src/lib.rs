//! A generic Unity asset-bundle reader.
//!
//! UnityFS containers, SerializedFiles with their typetrees, a typetree-driven
//! value reader, and the built-in classes a scene is made of. It holds Unity's
//! specifics and nothing else: no game's class names, no content format built
//! on top. A game-specific layer reads through [`value::Value`].

pub mod assets;
pub mod bundle;
pub mod classes;
pub mod common_strings;
pub mod compress;
pub mod error;
mod reader;
pub mod serialized;
pub mod typetree;
pub mod value;

pub use error::{Error, Result};
