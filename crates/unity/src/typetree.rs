//! The type tree of a SerializedFile type, and the value reader it drives.

use std::sync::Arc;

use crate::common_strings::common_string;
use crate::error::{Error, Result};
use crate::reader::Reader;
use crate::value::{PPtr, Struct, Value};

const META_ALIGN: u32 = 0x4000;
const MAX_DEPTH: usize = 64;

/// One node of a type tree.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeNode {
    /// The type name (`int`, `Vector3f`, `vector`, `Array`, `PPtr<Mesh>`, ...).
    pub type_name: Arc<str>,
    /// The field name.
    pub name: Arc<str>,
    /// Size in bytes for fixed-size types, -1 otherwise.
    pub byte_size: i32,
    /// Meta flags; `0x4000` aligns the stream to 4 after this node.
    pub meta_flags: u32,
    /// Whether this node is an array (`type_flags & 1`).
    pub is_array: bool,
    /// Child nodes.
    pub children: Vec<TypeNode>,
}

/// A flat node as stored in the blob, before the tree is built.
struct FlatNode {
    level: u8,
    node: TypeNode,
}

/// Reads a typetree blob (SerializedFile versions 12 and up).
pub fn read_blob(r: &mut Reader<'_>, version: u32) -> Result<TypeNode> {
    let count = r.u32()? as usize;
    let strings_len = r.u32()? as usize;
    let node_size = if version >= 19 { 32 } else { 24 };
    let need = count
        .checked_mul(node_size)
        .and_then(|n| n.checked_add(strings_len))
        .ok_or(Error::Malformed("typetree size overflows"))?;
    if need > r.remaining() || count == 0 {
        return Err(Error::Malformed(
            "typetree blob is empty or larger than the file",
        ));
    }
    let raw = r.bytes(count * node_size)?;
    let strings = r.bytes(strings_len)?;
    let nr_endian = r.endian();
    let mut nr = Reader::new(raw, nr_endian);
    let name_at = |off: u32| -> Result<Arc<str>> {
        if off & 0x8000_0000 != 0 {
            let o = off & 0x7fff_ffff;
            common_string(o)
                .map(Arc::from)
                .ok_or(Error::UnknownCommonString(o))
        } else {
            let tail = strings
                .get(off as usize..)
                .ok_or(Error::BadStringOffset(off))?;
            let end = tail
                .iter()
                .position(|&b| b == 0)
                .ok_or(Error::BadStringOffset(off))?;
            std::str::from_utf8(&tail[..end])
                .map(Arc::from)
                .map_err(|_| Error::InvalidUtf8)
        }
    };
    let mut flat = Vec::with_capacity(count);
    for _ in 0..count {
        let _version = nr.u16()?;
        let level = nr.u8()?;
        let type_flags = nr.u8()?;
        let type_off = nr.u32()?;
        let name_off = nr.u32()?;
        let byte_size = nr.i32()?;
        let _index = nr.i32()?;
        let meta_flags = nr.u32()?;
        if version >= 19 {
            nr.u64()?; // ref type hash
        }
        flat.push(FlatNode {
            level,
            node: TypeNode {
                type_name: name_at(type_off)?,
                name: name_at(name_off)?,
                byte_size,
                meta_flags,
                is_array: type_flags & 1 != 0,
                children: Vec::new(),
            },
        });
    }
    build_tree(flat)
}

/// Folds the level-tagged list into a tree.
fn build_tree(flat: Vec<FlatNode>) -> Result<TypeNode> {
    if flat[0].level != 0 {
        return Err(Error::Malformed("typetree root is not level 0"));
    }
    let mut stack: Vec<(u8, TypeNode)> = Vec::new();
    for f in flat {
        while let Some(&(lvl, _)) = stack.last() {
            if lvl >= f.level && stack.len() > 1 {
                let (_, done) = stack.pop().expect("non-empty");
                stack.last_mut().expect("parent").1.children.push(done);
            } else {
                break;
            }
        }
        if let Some(&(lvl, _)) = stack.last()
            && f.level != lvl + 1
        {
            return Err(Error::Malformed("typetree levels skip or repeat the root"));
        }
        if usize::from(f.level) > MAX_DEPTH {
            return Err(Error::LimitExceeded {
                what: "typetree depth",
                value: u64::from(f.level),
                limit: MAX_DEPTH as u64,
            });
        }
        stack.push((f.level, f.node));
    }
    while stack.len() > 1 {
        let (_, done) = stack.pop().expect("non-empty");
        stack.last_mut().expect("parent").1.children.push(done);
    }
    Ok(stack.pop().expect("root").1)
}

struct Walk<'a, 'b> {
    r: &'b mut Reader<'a>,
    budget: usize,
}

/// Reads one object's bytes through `root`, returning the value and how many
/// bytes the walk consumed.
pub fn read_value(
    root: &TypeNode,
    data: &[u8],
    endian: crate::reader::Endian,
) -> Result<(Value, usize)> {
    let mut r = Reader::new(data, endian);
    let budget = data.len().saturating_mul(16).saturating_add(4096);
    let mut w = Walk { r: &mut r, budget };
    let v = w.node(root, 0)?;
    Ok((v, r.pos()))
}

impl Walk<'_, '_> {
    fn node(&mut self, n: &TypeNode, depth: usize) -> Result<Value> {
        if depth > MAX_DEPTH * 2 {
            return Err(Error::LimitExceeded {
                what: "value depth",
                value: depth as u64,
                limit: (MAX_DEPTH * 2) as u64,
            });
        }
        self.budget = self.budget.checked_sub(1).ok_or(Error::LimitExceeded {
            what: "values per object",
            value: 0,
            limit: 0,
        })?;
        let v = self.value(n, depth)?;
        if n.meta_flags & META_ALIGN != 0 {
            self.r.align(4)?;
        }
        Ok(v)
    }

    fn value(&mut self, n: &TypeNode, depth: usize) -> Result<Value> {
        if n.is_array {
            return self.array(n, depth);
        }
        let r = &mut *self.r;
        Ok(match &*n.type_name {
            "bool" => Value::Bool(r.u8()? != 0),
            "SInt8" => Value::Int(i64::from(r.u8()? as i8)),
            "UInt8" | "char" => Value::UInt(u64::from(r.u8()?)),
            "SInt16" | "short" => Value::Int(i64::from(r.i16()?)),
            "UInt16" | "unsigned short" => Value::UInt(u64::from(r.u16()?)),
            "SInt32" | "int" => Value::Int(i64::from(r.i32()?)),
            "UInt32" | "unsigned int" | "Type*" => Value::UInt(u64::from(r.u32()?)),
            "SInt64" | "long long" => Value::Int(r.i64()?),
            "UInt64" | "unsigned long long" | "FileSize" => Value::UInt(r.u64()?),
            "float" => Value::Float(r.f32()?),
            "double" => Value::Double(r.f64()?),
            "string" => {
                let len = bounded_len(r, 1)?;
                let s = std::str::from_utf8(r.bytes(len)?)
                    .map_err(|_| Error::InvalidUtf8)?
                    .to_string();
                // The `string` node carries the alignment flag on its
                // `Array` child in some versions and on itself in others.
                if n.children
                    .first()
                    .is_some_and(|c| c.meta_flags & META_ALIGN != 0)
                {
                    r.align(4)?;
                }
                Value::String(s)
            }
            "TypelessData" => {
                let len = bounded_len(r, 1)?;
                Value::Bytes(r.bytes(len)?.to_vec())
            }
            t if t.starts_with("PPtr<") => {
                let fields = self.children(n, depth)?;
                let get = |name: &str| fields.iter().find(|(f, _)| &**f == name).map(|(_, v)| v);
                let file_id = get("m_FileID").and_then(Value::as_i64);
                let path_id = get("m_PathID").and_then(Value::as_i64);
                match (file_id, path_id) {
                    (Some(f), Some(p)) => Value::PPtr(PPtr {
                        file_id: i32::try_from(f).map_err(|_| Error::Malformed("PPtr file id"))?,
                        path_id: p,
                    }),
                    _ => return Err(Error::Malformed("PPtr without m_FileID and m_PathID")),
                }
            }
            // `vector`, `map`, `set` and friends: one `Array` child, read as the array.
            _ if n.children.len() == 1 && n.children[0].is_array => {
                return self.node(&n.children[0], depth + 1);
            }
            _ if n.children.is_empty() => {
                return Err(Error::UnsupportedType(n.type_name.to_string()));
            }
            _ => {
                let fields = self.children(n, depth)?;
                Value::Struct(Struct {
                    type_name: n.type_name.clone(),
                    fields,
                })
            }
        })
    }

    fn children(&mut self, n: &TypeNode, depth: usize) -> Result<Vec<(Arc<str>, Value)>> {
        let mut fields = Vec::with_capacity(n.children.len());
        for c in &n.children {
            fields.push((c.name.clone(), self.node(c, depth + 1)?));
        }
        Ok(fields)
    }

    fn array(&mut self, n: &TypeNode, depth: usize) -> Result<Value> {
        let elem = n
            .children
            .get(1)
            .ok_or(Error::Malformed("array node without an element child"))?;
        let one_byte =
            matches!(&*elem.type_name, "UInt8" | "SInt8" | "char") && elem.children.is_empty();
        let len = bounded_len(self.r, 1)?;
        if one_byte {
            return Ok(Value::Bytes(self.r.bytes(len)?.to_vec()));
        }
        let mut out = Vec::with_capacity(len.min(1 << 16));
        for _ in 0..len {
            out.push(self.node(elem, depth + 1)?);
        }
        Ok(Value::Array(out))
    }
}

/// Reads an `i32` length and checks it against the bytes left, assuming each
/// element takes at least `min_elem` byte(s).
fn bounded_len(r: &mut Reader<'_>, min_elem: usize) -> Result<usize> {
    let n = r.i32()?;
    let limit = r.remaining() / min_elem;
    usize::try_from(n)
        .ok()
        .filter(|&n| n <= limit)
        .ok_or(Error::LimitExceeded {
            what: "array length",
            value: n as u64,
            limit: limit as u64,
        })
}
