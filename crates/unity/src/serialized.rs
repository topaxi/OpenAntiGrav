//! The SerializedFile: the object table, the type table with its type trees,
//! and the external references, for versions 17 through 22.

use crate::error::{Error, Result};
use crate::reader::{Endian, Reader};
use crate::typetree::{self, TypeNode};
use crate::value::Value;

/// Versions with a known layout.
pub const SUPPORTED_VERSIONS: std::ops::RangeInclusive<u32> = 17..=23;

/// Class ids this crate names; the rest are reported by number.
pub mod class_id {
    /// `GameObject`.
    pub const GAME_OBJECT: i32 = 1;
    /// `Transform`.
    pub const TRANSFORM: i32 = 4;
    /// `Material`.
    pub const MATERIAL: i32 = 21;
    /// `Texture2D`.
    pub const TEXTURE_2D: i32 = 28;
    /// `Mesh`.
    pub const MESH: i32 = 43;
    /// `MonoBehaviour`.
    pub const MONO_BEHAVIOUR: i32 = 114;
}

/// One entry of the type table.
#[derive(Debug, Clone)]
pub struct SerializedType {
    /// The class id (114 for every `MonoBehaviour`, with `script_id` telling them apart).
    pub class_id: i32,
    /// Whether the type is stripped from the build.
    pub is_stripped: bool,
    /// Index into the script table, or -1.
    pub script_type_index: i16,
    /// The script's hash for MonoBehaviour types.
    pub script_id: Option<[u8; 16]>,
    /// The type hash.
    pub old_type_hash: [u8; 16],
    /// The type tree, present when the file was built with type trees.
    pub tree: Option<TypeNode>,
}

/// One entry of the object table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectInfo {
    /// The object's id within this file.
    pub path_id: i64,
    /// Byte offset of its data from the start of the file.
    pub byte_start: u64,
    /// Length of its data.
    pub byte_size: u32,
    /// Index into [`SerializedFile::types`].
    pub type_index: usize,
    /// Its class id.
    pub class_id: i32,
}

/// An entry of the externals table.
#[derive(Debug, Clone)]
pub struct External {
    /// The path, such as `archive:/CAB-xxxx/CAB-xxxx.sharedAssets`.
    pub path: String,
    /// The GUID (all zero for bundle members).
    pub guid: [u8; 16],
    /// The reference type.
    pub kind: i32,
}

/// A parsed SerializedFile borrowing the bytes it was read from.
#[derive(Debug)]
pub struct SerializedFile<'a> {
    /// The format version.
    pub version: u32,
    /// The engine version string the file was built with.
    pub unity_version: String,
    /// The target platform id.
    pub target_platform: i32,
    /// Whether type trees were included.
    pub has_type_trees: bool,
    /// The type table.
    pub types: Vec<SerializedType>,
    /// The object table.
    pub objects: Vec<ObjectInfo>,
    /// `(file index, local id)` of each script type.
    pub script_types: Vec<(i32, i64)>,
    /// The externals, which a PPtr's `file_id` indexes 1-based.
    pub externals: Vec<External>,
    endian: Endian,
    data: &'a [u8],
}

fn count(r: &mut Reader<'_>, what: &'static str, min_elem: usize) -> Result<usize> {
    let n = r.i32()?;
    let limit = r.remaining() / min_elem;
    usize::try_from(n)
        .ok()
        .filter(|&n| n <= limit)
        .ok_or(Error::LimitExceeded {
            what,
            value: n as u64,
            limit: limit as u64,
        })
}

fn read_type(
    r: &mut Reader<'_>,
    version: u32,
    has_trees: bool,
    is_ref: bool,
) -> Result<SerializedType> {
    let class_id = r.i32()?;
    let is_stripped = r.u8()? != 0;
    let script_type_index = r.i16()?;
    let mut script_id = None;
    if (is_ref && script_type_index >= 0) || (!is_ref && class_id == class_id::MONO_BEHAVIOUR) {
        script_id = Some(r.bytes(16)?.try_into().expect("16 bytes"));
    }
    let old_type_hash: [u8; 16] = r.bytes(16)?.try_into().expect("16 bytes");
    let mut tree = None;
    if has_trees {
        // Version 23 prefixes the blob with a content hash and its byte size (0 =
        // no tree stored for this type).
        let mut blob_len = None;
        if version >= 23 {
            r.bytes(16)?;
            let n = r.i32()?;
            blob_len = Some(usize::try_from(n).map_err(|_| Error::Malformed("typetree size"))?);
        }
        if blob_len != Some(0) {
            let start = r.pos();
            tree = Some(typetree::read_blob(r, version)?);
            if let Some(n) = blob_len
                && r.pos() - start != n
            {
                return Err(Error::Malformed(
                    "typetree blob differs from its declared size",
                ));
            }
        }
        if version >= 21 {
            if is_ref {
                r.cstr()?;
                r.cstr()?;
                r.cstr()?;
            } else {
                let n = count(r, "type dependencies", 4)?;
                r.bytes(n * 4)?;
            }
        }
    }
    Ok(SerializedType {
        class_id,
        is_stripped,
        script_type_index,
        script_id,
        old_type_hash,
        tree,
    })
}

impl<'a> SerializedFile<'a> {
    /// Parses a SerializedFile (a bundle node, or a standalone `.assets`).
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        let mut r = Reader::new(data, Endian::Big);
        let mut metadata_size = u64::from(r.u32()?);
        let mut file_size = u64::from(r.u32()?);
        let version = r.u32()?;
        let mut data_offset = u64::from(r.u32()?);
        if !SUPPORTED_VERSIONS.contains(&version) {
            return Err(Error::UnsupportedSerializedVersion(version));
        }
        let endian = match r.u8()? {
            0 => Endian::Little,
            1 => Endian::Big,
            _ => return Err(Error::Malformed("endianness flag")),
        };
        r.bytes(3)?;
        if version >= 22 {
            metadata_size = u64::from(r.u32()?);
            file_size = r.u64()?;
            data_offset = r.u64()?;
            r.u64()?;
        }
        if file_size > data.len() as u64 || data_offset > file_size {
            return Err(Error::Malformed("serialized header sizes exceed the file"));
        }
        let _ = metadata_size;
        r.set_endian(endian);

        let unity_version = r.cstr()?.to_string();
        let target_platform = r.i32()?;
        let has_type_trees = r.u8()? != 0;
        let n = count(&mut r, "type count", 20)?;
        let mut types = Vec::with_capacity(n);
        for _ in 0..n {
            types.push(read_type(&mut r, version, has_type_trees, false)?);
        }

        let n = count(&mut r, "object count", 20)?;
        let mut objects = Vec::with_capacity(n);
        for _ in 0..n {
            r.align(4)?;
            let path_id = r.i64()?;
            let start = if version >= 22 {
                r.u64()?
            } else {
                u64::from(r.u32()?)
            };
            let byte_size = r.u32()?;
            let type_index = usize::try_from(r.i32()?)
                .ok()
                .filter(|&t| t < types.len())
                .ok_or(Error::Malformed("object type index outside the type table"))?;
            let byte_start = data_offset
                .checked_add(start)
                .filter(|&s| {
                    s.checked_add(u64::from(byte_size))
                        .is_some_and(|e| e <= data.len() as u64)
                })
                .ok_or(Error::Malformed("object data outside the file"))?;
            objects.push(ObjectInfo {
                path_id,
                byte_start,
                byte_size,
                type_index,
                class_id: types[type_index].class_id,
            });
        }

        let n = count(&mut r, "script type count", 12)?;
        let mut script_types = Vec::with_capacity(n);
        for _ in 0..n {
            let file = r.i32()?;
            r.align(4)?;
            script_types.push((file, r.i64()?));
        }
        let n = count(&mut r, "external count", 6)?;
        let mut externals = Vec::with_capacity(n);
        for _ in 0..n {
            r.cstr()?;
            let guid: [u8; 16] = r.bytes(16)?.try_into().expect("16 bytes");
            let kind = r.i32()?;
            let path = r.cstr()?.to_string();
            externals.push(External { path, guid, kind });
        }
        if version >= 20 {
            let n = count(&mut r, "ref type count", 20)?;
            for _ in 0..n {
                read_type(&mut r, version, has_type_trees, true)?;
            }
        }
        r.cstr()?; // user information, normally empty

        Ok(Self {
            version,
            unity_version,
            target_platform,
            has_type_trees,
            types,
            objects,
            script_types,
            externals,
            endian,
            data,
        })
    }

    /// An object's raw bytes.
    #[must_use]
    pub fn object_bytes(&self, o: &ObjectInfo) -> &'a [u8] {
        let s = o.byte_start as usize;
        &self.data[s..s + o.byte_size as usize]
    }

    /// The object with this path id.
    #[must_use]
    pub fn object(&self, path_id: i64) -> Option<&ObjectInfo> {
        self.objects.iter().find(|o| o.path_id == path_id)
    }

    /// Reads an object through its type tree. **Strict:** the walk must consume
    /// every byte of the object, or this is [`Error::SizeMismatch`].
    pub fn read_object(&self, o: &ObjectInfo) -> Result<Value> {
        let ty = &self.types[o.type_index];
        let tree = ty.tree.as_ref().ok_or(Error::NoTypeTree {
            class_id: o.class_id,
        })?;
        let bytes = self.object_bytes(o);
        let (v, used) = typetree::read_value(tree, bytes, self.endian)?;
        if used != bytes.len() {
            return Err(Error::SizeMismatch {
                expected: bytes.len(),
                consumed: used,
            });
        }
        Ok(v)
    }
}
