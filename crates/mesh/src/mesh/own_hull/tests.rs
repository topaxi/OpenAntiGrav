use serde_json::{Value, json};

use super::*;

/// Where the official mod `oag.detonator-pulse` keeps the two Pulse hulls, in the official-mods
/// submodule: read at run time, so a checkout without the submodule still builds.
const HULLS: &str =
    "../../assets/mods/official/detonator-pulse/fallback/wipeout-pulse/Data/Ships/Detonator_Pulse";

/// `file` of [`HULLS`], or `None` with a line when the submodule is not checked out. A checked-out
/// mod without the file fails.
fn committed(file: &str) -> Option<Vec<u8>> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(HULLS);
    let manifest = dir.join("../../../../../oag-mod.toml");
    if !manifest.exists() {
        eprintln!(
            "skipped: the official-mods submodule is not checked out ({})",
            manifest.display()
        );
        return None;
    }
    let path = dir.join(file);
    Some(std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
}

/// Pulse PSP `Zone.vex`'s mesh nodes and the textures their draws use
/// (`docs/formats/own-hull.md`, "The Pulse host").
const PSP_PARTS: [&str; 8] = [
    "shipshape",
    "cockpitshape",
    "pilotshape",
    "airbrake_leftshape",
    "airbrake_rightshape",
    "canopyshape",
    "flashersshape",
    "lodshape",
];
const PSP_TEXTURES: [&str; 9] = [
    "engine_general.tga",
    "texture1.tga",
    "texture2.tga",
    "texture3.tga",
    "texture4.tga",
    "glass_add.tga",
    "colours_flashing_glow.tga",
    "envtest4bit.tga",
    "envmap_stripe2.tga",
];

/// Pulse PS2 `Zone.vex`'s, another model (`docs/formats/own-hull.md`, "The Pulse host").
const PS2_PARTS: [&str; 8] = [
    "shipshape",
    "self_illuminated",
    "canapoyshape",
    "glowingshape",
    "airbrake_leftshape",
    "airbrake_rightshape",
    "lodshape",
    "glowing_lodshape",
];
const PS2_TEXTURES: [&str; 5] = [
    "zone_colourmap.tga",
    "zone_engine.tga",
    "lights_glow.tga",
    "glass2_add.tga",
    "envmap_stripe2.tga",
];

fn names_only(bytes: &[u8], parts: &[&str], textures: &[&str]) -> OwnHull {
    let hull = parse(bytes).expect("the committed hull parses");
    // They paint with the host's textures and may hold no pixel of the disc: no image, texture
    // or sampler, though the format allows them.
    let doc = gltf::Gltf::from_slice(bytes).expect("a glb").document;
    assert_eq!(
        (
            doc.images().len(),
            doc.textures().len(),
            doc.samplers().len()
        ),
        (0, 0, 0)
    );
    assert!(!hull.parts.is_empty());
    for part in &hull.parts {
        assert!(parts.contains(&fold(&part.name).as_str()), "{}", part.name);
        assert!(!part.surfaces.is_empty(), "{} has no surface", part.name);
        for surface in &part.surfaces {
            let texture = fold(&surface.texture);
            assert!(textures.contains(&texture.as_str()), "{}", surface.texture);
            assert!(!surface.triangles.is_empty());
        }
    }
    assert!(hull.parts.iter().any(|p| fold(&p.name) == "shipshape"));
    let (vertices, triangles) = hull.counts();
    assert!((4..65_536).contains(&vertices), "{vertices} vertices");
    assert!((4..20_000).contains(&triangles), "{triangles} triangles");
    let shadow = hull.shadow.as_ref().expect("a shadow occluder");
    assert!(
        oag_vex::shadow_occluder::Occluder::from_triangles(&shadow.positions, &shadow.triangles)
            .is_some()
    );
    assert!(hull.locator("engine_flare").is_some());
    hull
}

#[test]
fn the_committed_detonator_hulls_name_only_their_pulse_hosts_parts_and_textures() {
    let (Some(psp), Some(ps2)) = (committed("hull.glb"), committed("hull_ps2.glb")) else {
        return;
    };
    let psp = names_only(&psp, &PSP_PARTS, &PSP_TEXTURES);
    let ps2 = names_only(&ps2, &PS2_PARTS, &PS2_TEXTURES);
    // Neither may fit the other pressing's model, or the first listed would fly on both: each
    // names a part the other host lacks.
    let part_outside = |hull: &OwnHull, parts: &[&str]| {
        hull.parts
            .iter()
            .any(|p| !parts.contains(&fold(&p.name).as_str()))
    };
    assert!(part_outside(&psp, &PS2_PARTS));
    assert!(part_outside(&ps2, &PSP_PARTS));
}

#[test]
fn a_name_folds_off_blenders_suffix_its_directory_and_case() {
    assert_eq!(fold("AirBrake_LeftShape.001"), "airbrake_leftshape");
    assert_eq!(
        fold(r"Data\Ships\Feisar\Textures\zone\texture1.tga"),
        "texture1.tga"
    );
    assert_eq!(fold("texture1.tga.002"), "texture1.tga");
    assert_eq!(fold("shipShape.v2"), "shipshape.v2");
}

/// A hull glb in the file's layout (`docs/formats/own-hull.md`), built in memory so no `.glb`
/// lives in this repository; `edit` changes the JSON chunk before it is packed.
fn glb(hull: &OwnHull, edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    #[derive(Default)]
    struct Bin {
        data: Vec<u8>,
        views: Vec<Value>,
        accessors: Vec<Value>,
    }
    impl Bin {
        fn add(
            &mut self,
            bytes: &[u8],
            component: u32,
            ty: &str,
            count: usize,
            more: Value,
        ) -> usize {
            self.views.push(json!({
                "buffer": 0, "byteOffset": self.data.len(), "byteLength": bytes.len()
            }));
            self.data.extend_from_slice(bytes);
            while !self.data.len().is_multiple_of(4) {
                self.data.push(0);
            }
            let mut accessor = json!({
                "bufferView": self.views.len() - 1, "componentType": component,
                "count": count, "type": ty
            });
            accessor
                .as_object_mut()
                .unwrap()
                .extend(more.as_object().unwrap().clone());
            self.accessors.push(accessor);
            self.accessors.len() - 1
        }
        fn floats<const N: usize>(&mut self, rows: &[[f32; N]], ty: &str) -> usize {
            let bytes: Vec<u8> = rows
                .as_flattened()
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            self.add(&bytes, 5126, ty, rows.len(), json!({}))
        }
        fn positions(&mut self, rows: &[[f32; 3]]) -> usize {
            let bytes: Vec<u8> = rows
                .as_flattened()
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            let fold = |f: fn(f32, f32) -> f32, start: f32| {
                (0..3)
                    .map(|i| rows.iter().map(|r| r[i]).fold(start, f))
                    .collect::<Vec<_>>()
            };
            let bounds =
                json!({ "min": fold(f32::min, f32::MAX), "max": fold(f32::max, f32::MIN) });
            self.add(&bytes, 5126, "VEC3", rows.len(), bounds)
        }
        fn indices(&mut self, triangles: &[[u32; 3]]) -> usize {
            let flat = triangles.as_flattened();
            let bytes: Vec<u8> = flat
                .iter()
                .flat_map(|&i| u16::try_from(i).unwrap().to_le_bytes())
                .collect();
            self.add(&bytes, 5123, "SCALAR", flat.len(), json!({}))
        }
    }
    let mut bin = Bin::default();
    let (mut nodes, mut meshes, mut materials) = (Vec::new(), Vec::new(), Vec::<String>::new());
    for part in &hull.parts {
        let prims: Vec<Value> = part
            .surfaces
            .iter()
            .map(|s| {
                let mut attributes = json!({
                    "POSITION": bin.positions(&s.positions),
                    "NORMAL": bin.floats(&s.normals, "VEC3"),
                    "TEXCOORD_0": bin.floats(&s.uvs, "VEC2"),
                });
                if let Some(colours) = &s.colours {
                    attributes["COLOR_0"] = json!(bin.add(
                        colours.as_flattened(),
                        5121,
                        "VEC4",
                        colours.len(),
                        json!({ "normalized": true })
                    ));
                }
                if !materials.contains(&s.texture) {
                    materials.push(s.texture.clone());
                }
                let material = materials.iter().position(|m| *m == s.texture).unwrap();
                json!({
                    "attributes": attributes, "indices": bin.indices(&s.triangles),
                    "material": material, "mode": 4
                })
            })
            .collect();
        meshes.push(json!({ "primitives": prims }));
        nodes
            .push(json!({ "name": part.name, "matrix": part.transform, "mesh": meshes.len() - 1 }));
    }
    for locator in &hull.locators {
        nodes.push(json!({ "name": locator.name, "matrix": locator.transform }));
    }
    if let Some(shadow) = &hull.shadow {
        let prim = json!({
            "attributes": { "POSITION": bin.positions(&shadow.positions) },
            "indices": bin.indices(&shadow.triangles), "mode": 4
        });
        meshes.push(json!({ "primitives": [prim] }));
        nodes.push(json!({ "name": "shadowShape", "mesh": meshes.len() - 1 }));
    }
    let mut doc = json!({
        "asset": { "version": "2.0", "extras": { "oag_kind": "hull", "oag_version": "1.0.0" } },
        "scene": 0,
        "scenes": [{ "nodes": (0..nodes.len()).collect::<Vec<_>>() }],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials.iter().map(|m| json!({ "name": m })).collect::<Vec<_>>(),
        "buffers": [{ "byteLength": bin.data.len() }],
        "bufferViews": bin.views,
        "accessors": bin.accessors,
    });
    edit(&mut doc);
    let mut text = serde_json::to_vec(&doc).unwrap();
    while !text.len().is_multiple_of(4) {
        text.push(b' ');
    }
    let word = |v: usize| u32::try_from(v).unwrap().to_le_bytes();
    let mut out = b"glTF".to_vec();
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&word(12 + 8 + text.len() + 8 + bin.data.len()));
    out.extend_from_slice(&word(text.len()));
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&text);
    out.extend_from_slice(&word(bin.data.len()));
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin.data);
    out
}

const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// One coloured triangle on `shipShape` (a hinge-like pivot), an `engine_flare` and a shadow.
fn tiny() -> OwnHull {
    let mut pivot = IDENTITY;
    pivot[12..15].copy_from_slice(&[1.5, -0.25, 3.0]);
    OwnHull {
        parts: vec![Part {
            name: "shipShape".into(),
            transform: pivot,
            surfaces: vec![Surface {
                texture: "texture1.tga".into(),
                positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, -0.0]],
                normals: vec![[0.0, 0.0, 1.0]; 3],
                uvs: vec![[0.1, 0.2], [0.9, 0.2], [0.1, 0.7]],
                colours: Some(vec![[255, 128, 0, 255]; 3]),
                triangles: vec![[0, 1, 2]],
            }],
        }],
        locators: vec![Locator {
            name: "engine_flare".into(),
            transform: IDENTITY,
        }],
        shadow: Some(Shadow {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            triangles: vec![[0, 1, 2]],
        }),
        images: 0,
    }
}

#[test]
fn a_hull_glb_reads_back_what_was_written() {
    let hull = tiny();
    assert_eq!(
        parse(&glb(&hull, |_| {})).expect("a well-formed hull"),
        hull
    );
    let mut plain = hull.clone();
    plain.parts[0].surfaces[0].colours = None;
    plain.shadow = None;
    assert_eq!(
        parse(&glb(&plain, |_| {})).expect("no colours, no shadow"),
        plain
    );
}

#[test]
fn a_hull_glb_refuses_a_wrong_shape_by_name() {
    let hull = tiny();
    let shape = |edit: fn(&mut Value)| match parse(&glb(&hull, edit)) {
        Err(ReadError::Shape(_, what)) => what,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        shape(|d| {
            d["nodes"][0].as_object_mut().unwrap().remove("matrix");
        }),
        "a part or locator carries its transform as a matrix"
    );
    let mut past = hull.clone();
    past.parts[0].surfaces[0].triangles = vec![[0, 1, 3]];
    assert!(matches!(
        parse(&glb(&past, |_| {})),
        Err(ReadError::Shape(
            _,
            "a triangle names a vertex past the end"
        ))
    ));
    assert_eq!(
        shape(|d| {
            d["meshes"][0]["primitives"][0]
                .as_object_mut()
                .unwrap()
                .remove("material");
        }),
        "a surface names its host texture as its material"
    );
    assert_eq!(
        shape(|d| d["accessors"][3]["normalized"] = json!(false)),
        "COLOR_0 must be normalised u8 VEC4"
    );
    assert_eq!(
        shape(|d| d["nodes"][2]["matrix"] = json!(IDENTITY.map(|v| v * 2.0))),
        "the shadow is in model space, untransformed"
    );
    assert!(matches!(
        parse(&glb(&hull, |d| d["asset"]["extras"]["oag_kind"] = json!("track"))),
        Err(ReadError::Kind(Some(kind))) if kind == "track"
    ));
    assert!(matches!(
        parse(&glb(&hull, |d| d["asset"]["extras"]["oag_version"] =
            json!("2.0.0"))),
        Err(ReadError::Version(Some(_)))
    ));
    assert!(matches!(
        parse(&glb(&hull, |d| {
            d["asset"].as_object_mut().unwrap().remove("extras");
        })),
        Err(ReadError::Kind(None))
    ));
    assert!(matches!(parse(b"not a glb"), Err(ReadError::Glb(_))));
}

#[test]
fn a_hull_glb_whose_material_has_an_image_still_reads_by_material_name() {
    let hull = tiny();
    let textured = glb(&hull, |d| {
        d["samplers"] = json!([{}]);
        d["images"] = json!([{ "bufferView": 0, "mimeType": "image/png" }]);
        d["textures"] = json!([{ "source": 0, "sampler": 0 }]);
        d["materials"][0]["pbrMetallicRoughness"] = json!({ "baseColorTexture": { "index": 0 } });
    });
    let read = parse(&textured).expect("an image does not keep a hull out");
    assert_eq!(read.images, 1);
    assert_eq!(read.parts[0].surfaces[0].texture, "texture1.tga");
    assert_eq!(read, OwnHull { images: 1, ..hull });
}
