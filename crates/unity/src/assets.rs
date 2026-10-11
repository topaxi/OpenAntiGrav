//! The SerializedFiles of one bundle, opened together so that a [`PPtr`] can be
//! followed across them.

use crate::bundle::Bundle;
use crate::classes::{GameObject, Texture2d, Transform};
use crate::error::{Error, Result};
use crate::serialized::{ObjectInfo, SerializedFile, class_id};
use crate::value::{PPtr, Value};

/// A bundle's SerializedFiles, in node order.
#[derive(Debug)]
pub struct Assets<'a> {
    bundle: &'a Bundle,
    names: Vec<&'a str>,
    files: Vec<SerializedFile<'a>>,
}

/// An object located in one of the files.
#[derive(Debug, Clone, Copy)]
pub struct Located<'a, 'b> {
    /// Index of the file in [`Assets::files`].
    pub file: usize,
    /// The object's table entry.
    pub info: &'b ObjectInfo,
    /// The file it is in.
    pub serialized: &'b SerializedFile<'a>,
}

impl<'a> Assets<'a> {
    /// Opens every SerializedFile node of `bundle`.
    pub fn open(bundle: &'a Bundle) -> Result<Self> {
        let mut names = Vec::new();
        let mut files = Vec::new();
        for node in bundle.nodes().iter().filter(|n| n.is_serialized_file()) {
            names.push(node.path.as_str());
            files.push(SerializedFile::parse(bundle.node_bytes(node))?);
        }
        Ok(Self {
            bundle,
            names,
            files,
        })
    }

    /// Reads a `Texture2D` value, fetching pixels held in a `.resS` node of the
    /// same bundle. A stream that is not in the bundle is an error.
    pub fn texture(&self, v: &Value) -> Result<Texture2d> {
        let mut t = Texture2d::from_value(v)?;
        if let Some(s) = &t.stream {
            let base = s.path.rsplit('/').next().unwrap_or(&s.path);
            let node = self
                .bundle
                .nodes()
                .iter()
                .find(|n| n.path.eq_ignore_ascii_case(base))
                .ok_or(Error::Malformed(
                    "texture stream is not a node of this bundle",
                ))?;
            let bytes = self.bundle.node_bytes(node);
            let end = s
                .offset
                .checked_add(s.size)
                .filter(|&e| e <= bytes.len() as u64);
            let (start, end) = (
                s.offset as usize,
                end.ok_or(Error::Malformed("texture stream range outside its node"))? as usize,
            );
            let slice = bytes[start..end].to_vec();
            t.fill_from_stream(&slice)?;
        }
        Ok(t)
    }

    /// The files.
    #[must_use]
    pub fn files(&self) -> &[SerializedFile<'a>] {
        &self.files
    }

    /// File names, parallel to [`Self::files`].
    #[must_use]
    pub fn names(&self) -> &[&'a str] {
        &self.names
    }

    /// Follows `p`, written in file `from`, to its object. `None` for a null
    /// pointer, an external that is not in this bundle, or a missing object.
    #[must_use]
    pub fn resolve(&self, from: usize, p: PPtr) -> Option<Located<'a, '_>> {
        if p.is_null() {
            return None;
        }
        let file = if p.file_id == 0 {
            from
        } else {
            let ext = self
                .files
                .get(from)?
                .externals
                .get(usize::try_from(p.file_id - 1).ok()?)?;
            let base = ext.path.rsplit('/').next()?;
            self.names
                .iter()
                .position(|n| n.eq_ignore_ascii_case(base))?
        };
        let serialized = &self.files[file];
        let info = serialized.object(p.path_id)?;
        Some(Located {
            file,
            info,
            serialized,
        })
    }

    /// Reads the object a pointer leads to through its typetree.
    pub fn read(&self, from: usize, p: PPtr) -> Result<Option<(Located<'a, '_>, Value)>> {
        match self.resolve(from, p) {
            Some(l) => {
                let v = l.serialized.read_object(l.info)?;
                Ok(Some((l, v)))
            }
            None => Ok(None),
        }
    }

    /// The `/`-joined names from the root down to `transform`, or an error when
    /// the parent chain is longer than 4096 (a cycle).
    pub fn transform_path(&self, file: usize, transform: PPtr) -> Result<String> {
        let mut parts = Vec::new();
        let mut cur = transform;
        for _ in 0..4096 {
            let Some((l, v)) = self.read(file, cur)? else {
                parts.reverse();
                return Ok(parts.join("/"));
            };
            if l.info.class_id != class_id::TRANSFORM {
                return Err(Error::Malformed(
                    "a transform pointer leads to another class",
                ));
            }
            let t = Transform::from_value(&v)?;
            let name = match self.read(l.file, t.game_object)? {
                Some((_, go)) => GameObject::from_value(&go)?.name,
                None => String::new(),
            };
            parts.push(name);
            cur = t.father;
        }
        Err(Error::Malformed("transform parent chain does not end"))
    }
}
