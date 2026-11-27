//! The own-hull file, `hull.glb` (`docs/formats/own-hull.md`): one glTF node per part, one
//! primitive per surface, read into an [`OwnHull`]. Axes and texture coordinates are glTF's, which
//! are the engine's, so nothing is converted. A material is an ordinary glTF material matched to
//! the host texture by its name; images the file carries are counted, not read.

use gltf::accessor::{DataType, Dimensions, Iter};
use gltf::mesh::{Mode, Semantic};
use gltf::scene::Transform;

use super::{Locator, OwnHull, Part, Shadow, Surface, fold};

/// The kind a hull glb declares in its asset extras (`oag_kind`).
pub const KIND: &str = "hull";
/// The major of `oag_version` this reader takes.
pub const MAJOR: u32 = 1;
/// The node that carries the shadow occluder, matched as [`fold`] matches.
const SHADOW: &str = "shadowshape";

/// Why a hull file was refused.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// Not a glb the `gltf` crate reads and validates.
    #[error("not a valid glb: {0}")]
    Glb(String),
    /// The asset extras name another kind, or none.
    #[error("not a hull glb (asset extras oag_kind {0:?})")]
    Kind(Option<String>),
    /// The asset extras name another major of the format.
    #[error("hull glb version {0:?}, this reader takes major {MAJOR}")]
    Version(Option<String>),
    /// The file keeps data outside its one BIN chunk.
    #[error("a hull glb carries no {0}")]
    Carries(&'static str),
    /// A node, primitive or accessor is not of the shape the hull's layout gives it.
    #[error("{0}: {1}")]
    Shape(String, &'static str),
}

/// Reads a hull file.
///
/// # Errors
///
/// The bytes are not a valid glb, its asset extras do not mark a hull of a known major, it
/// has a buffer besides its BIN chunk, or a node is not of the
/// layout's shape (a part or locator without a matrix, a surface without a material name, an
/// attribute of the wrong type or count, triangles naming a vertex past the end).
pub fn parse(bytes: &[u8]) -> Result<OwnHull, ReadError> {
    let gltf = gltf::Gltf::from_slice(bytes).map_err(|e| ReadError::Glb(e.to_string()))?;
    marker(&gltf.document)?;
    let doc = &gltf.document;
    if doc.buffers().len() > 1 {
        return Err(ReadError::Carries("second buffer"));
    }
    if doc
        .buffers()
        .any(|b| !matches!(b.source(), gltf::buffer::Source::Bin))
    {
        return Err(ReadError::Carries("external buffer"));
    }
    let blob = gltf.blob.as_deref().unwrap_or_default();
    let mut hull = OwnHull {
        images: doc.images().len(),
        ..OwnHull::default()
    };
    for node in doc.nodes() {
        let name = node.name().unwrap_or_default().to_owned();
        if node.children().len() > 0 {
            return Err(ReadError::Shape(name, "a hull node has no children"));
        }
        let Some(mesh) = node.mesh() else {
            hull.locators.push(Locator {
                transform: matrix(&node, &name)?,
                name,
            });
            continue;
        };
        if fold(&name) == SHADOW {
            if node.transform().matrix() != IDENTITY {
                return Err(ReadError::Shape(
                    name,
                    "the shadow is in model space, untransformed",
                ));
            }
            let mut prims = mesh.primitives();
            let (Some(prim), None) = (prims.next(), prims.next()) else {
                return Err(ReadError::Shape(name, "the shadow is one primitive"));
            };
            let positions = vectors(&prim, Semantic::Positions, blob, &name)?;
            let triangles = triangles(&prim, blob, positions.len(), &name)?;
            hull.shadow = Some(Shadow {
                positions,
                triangles,
            });
            continue;
        }
        let transform = matrix(&node, &name)?;
        let surfaces = mesh
            .primitives()
            .map(|prim| surface(&prim, blob, &name))
            .collect::<Result<_, _>>()?;
        hull.parts.push(Part {
            name,
            transform,
            surfaces,
        });
    }
    Ok(hull)
}

const IDENTITY: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// Checks the asset extras' `oag_kind` and the major of `oag_version`.
fn marker(doc: &gltf::Document) -> Result<(), ReadError> {
    let extras: Option<gltf::json::Value> = doc
        .as_json()
        .asset
        .extras
        .as_ref()
        .and_then(|raw| gltf::json::deserialize::from_str(raw.get()).ok());
    let text = |key: &str| {
        extras
            .as_ref()
            .and_then(|e| e.get(key))
            .and_then(gltf::json::Value::as_str)
            .map(str::to_owned)
    };
    let kind = text("oag_kind");
    if kind.as_deref() != Some(KIND) {
        return Err(ReadError::Kind(kind));
    }
    let version = text("oag_version");
    let major = version
        .as_deref()
        .and_then(|v| v.split('.').next())
        .and_then(|m| m.parse::<u32>().ok());
    if major != Some(MAJOR) {
        return Err(ReadError::Version(version));
    }
    Ok(())
}

/// A part's or locator's matrix, column-major as glTF and a `.vex` matrix both lay it out.
fn matrix(node: &gltf::Node<'_>, name: &str) -> Result<[f32; 16], ReadError> {
    match node.transform() {
        Transform::Matrix { matrix } => {
            let mut out = [0.0; 16];
            out.copy_from_slice(matrix.as_flattened());
            Ok(out)
        }
        Transform::Decomposed { .. } => Err(ReadError::Shape(
            name.to_owned(),
            "a part or locator carries its transform as a matrix",
        )),
    }
}

fn surface(prim: &gltf::Primitive<'_>, blob: &[u8], part: &str) -> Result<Surface, ReadError> {
    let Some(texture) = prim.material().name().map(str::to_owned) else {
        return Err(ReadError::Shape(
            part.to_owned(),
            "a surface names its host texture as its material",
        ));
    };
    let at = format!("{part} {texture}");
    let positions = vectors(prim, Semantic::Positions, blob, &at)?;
    let normals = vectors(prim, Semantic::Normals, blob, &at)?;
    let uvs = floats::<2>(
        prim,
        Semantic::TexCoords(0),
        blob,
        &at,
        "TEXCOORD_0 must be float VEC2",
    )?;
    let colours = match prim.get(&Semantic::Colors(0)) {
        None => None,
        Some(a)
            if a.data_type() == DataType::U8
                && a.dimensions() == Dimensions::Vec4
                && a.normalized() =>
        {
            Some(
                Iter::<[u8; 4]>::new(a, |_| Some(blob))
                    .ok_or_else(|| ReadError::Shape(at.clone(), "COLOR_0 lies outside the buffer"))?
                    .collect::<Vec<_>>(),
            )
        }
        Some(_) => {
            return Err(ReadError::Shape(at, "COLOR_0 must be normalised u8 VEC4"));
        }
    };
    if normals.len() != positions.len()
        || uvs.len() != positions.len()
        || colours.as_ref().is_some_and(|c| c.len() != positions.len())
    {
        return Err(ReadError::Shape(
            at,
            "normals, uvs and colours must be one per position",
        ));
    }
    let triangles = triangles(prim, blob, positions.len(), &at)?;
    Ok(Surface {
        texture,
        positions,
        normals,
        uvs,
        colours,
        triangles,
    })
}

fn vectors(
    prim: &gltf::Primitive<'_>,
    semantic: Semantic,
    blob: &[u8],
    at: &str,
) -> Result<Vec<[f32; 3]>, ReadError> {
    let what = match semantic {
        Semantic::Positions => "POSITION must be float VEC3",
        _ => "NORMAL must be float VEC3",
    };
    floats::<3>(prim, semantic, blob, at, what)
}

fn floats<const N: usize>(
    prim: &gltf::Primitive<'_>,
    semantic: Semantic,
    blob: &[u8],
    at: &str,
    what: &'static str,
) -> Result<Vec<[f32; N]>, ReadError>
where
    [f32; N]: gltf::accessor::Item,
{
    let dimensions = if N == 2 {
        Dimensions::Vec2
    } else {
        Dimensions::Vec3
    };
    prim.get(&semantic)
        .filter(|a| a.data_type() == DataType::F32 && a.dimensions() == dimensions)
        .and_then(|a| Iter::<[f32; N]>::new(a, |_| Some(blob)))
        .map(Iterator::collect)
        .ok_or_else(|| ReadError::Shape(at.to_owned(), what))
}

fn triangles(
    prim: &gltf::Primitive<'_>,
    blob: &[u8],
    vertices: usize,
    at: &str,
) -> Result<Vec<[u32; 3]>, ReadError> {
    if prim.mode() != Mode::Triangles {
        return Err(ReadError::Shape(at.to_owned(), "a primitive is TRIANGLES"));
    }
    let flat: Vec<u32> = prim
        .indices()
        .filter(|a| a.dimensions() == Dimensions::Scalar)
        .and_then(|a| match a.data_type() {
            DataType::U16 => {
                Iter::<u16>::new(a, |_| Some(blob)).map(|i| i.map(u32::from).collect())
            }
            DataType::U32 => Iter::<u32>::new(a, |_| Some(blob)).map(Iterator::collect),
            _ => None,
        })
        .ok_or_else(|| ReadError::Shape(at.to_owned(), "indices must be u16 or u32"))?;
    if !flat.len().is_multiple_of(3) {
        return Err(ReadError::Shape(at.to_owned(), "a triangle is 3 indices"));
    }
    if flat.iter().any(|&i| i as usize >= vertices) {
        return Err(ReadError::Shape(
            at.to_owned(),
            "a triangle names a vertex past the end",
        ));
    }
    Ok(flat.as_chunks::<3>().0.to_vec())
}
