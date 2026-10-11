//! Behaviour on bytes the test builds itself: a valid tiny bundle and
//! SerializedFile, then every way to damage them. No game data, so this runs in
//! plain `just test`.

use oag_unity::bundle::{Bundle, Limits};
use oag_unity::compress::lz4_block;
use oag_unity::error::Error;
use oag_unity::serialized::SerializedFile;
use oag_unity::value::Value;

fn be32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_be_bytes());
}

/// An uncompressed v6 bundle holding `files` as consecutive nodes in one block.
fn bundle(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut data = Vec::new();
    let mut info = vec![0u8; 16];
    be32(&mut info, 1);
    let total: usize = files.iter().map(|f| f.1.len()).sum();
    be32(&mut info, total as u32);
    be32(&mut info, total as u32);
    info.extend_from_slice(&[0, 0]);
    be32(&mut info, files.len() as u32);
    for (name, bytes) in files {
        info.extend_from_slice(&(data.len() as i64).to_be_bytes());
        info.extend_from_slice(&(bytes.len() as i64).to_be_bytes());
        be32(&mut info, 4);
        info.extend_from_slice(name.as_bytes());
        info.push(0);
        data.extend_from_slice(bytes);
    }
    let mut out = b"UnityFS\0".to_vec();
    be32(&mut out, 6);
    out.extend_from_slice(b"5.x.x\x002018.3.8f1\0");
    let size_at = out.len();
    out.extend_from_slice(&0i64.to_be_bytes());
    be32(&mut out, info.len() as u32);
    be32(&mut out, info.len() as u32);
    be32(&mut out, 0x40);
    out.extend_from_slice(&info);
    out.extend_from_slice(&data);
    let len = out.len() as i64;
    out[size_at..size_at + 8].copy_from_slice(&len.to_be_bytes());
    out
}

struct Node(u8, &'static str, &'static str, u8, u32);

/// A v17 SerializedFile with one type, `Base { string m_Name; vector<UInt8> m_Data; int m_Num }`,
/// and one object of it, whose bytes are `payload`.
fn serialized(payload: &[u8]) -> Vec<u8> {
    let nodes = [
        Node(0, "Base", "Base", 0, 0),
        Node(1, "string", "m_Name", 0, 0x4000),
        Node(2, "Array", "Array", 1, 0x4000),
        Node(3, "int", "size", 0, 0),
        Node(3, "char", "data", 0, 0),
        Node(1, "vector", "m_Data", 0, 0),
        Node(2, "Array", "Array", 1, 0x4000),
        Node(3, "int", "size", 0, 0),
        Node(3, "UInt8", "data", 0, 0),
        Node(1, "int", "m_Num", 0, 0),
    ];
    let mut strings = Vec::new();
    let mut offsets = std::collections::HashMap::new();
    let mut intern = |s: &'static str, strings: &mut Vec<u8>| {
        *offsets.entry(s).or_insert_with(|| {
            let at = strings.len() as u32;
            strings.extend_from_slice(s.as_bytes());
            strings.push(0);
            at
        })
    };
    let mut blob = Vec::new();
    for n in &nodes {
        blob.extend_from_slice(&1u16.to_le_bytes());
        blob.push(n.0);
        blob.push(n.3);
        blob.extend_from_slice(&intern(n.1, &mut strings).to_le_bytes());
        blob.extend_from_slice(&intern(n.2, &mut strings).to_le_bytes());
        blob.extend_from_slice(&(-1i32).to_le_bytes());
        blob.extend_from_slice(&0i32.to_le_bytes());
        blob.extend_from_slice(&n.4.to_le_bytes());
    }
    let mut meta = Vec::new();
    meta.extend_from_slice(b"2018.3.8f1\0");
    meta.extend_from_slice(&5i32.to_le_bytes());
    meta.push(1);
    meta.extend_from_slice(&1i32.to_le_bytes());
    meta.extend_from_slice(&49i32.to_le_bytes());
    meta.push(0);
    meta.extend_from_slice(&(-1i16).to_le_bytes());
    meta.extend_from_slice(&[0u8; 16]);
    meta.extend_from_slice(&(nodes.len() as u32).to_le_bytes());
    meta.extend_from_slice(&(strings.len() as u32).to_le_bytes());
    meta.extend_from_slice(&blob);
    meta.extend_from_slice(&strings);
    meta.extend_from_slice(&1i32.to_le_bytes());
    while (20 + meta.len()) % 4 != 0 {
        meta.push(0);
    }
    meta.extend_from_slice(&1i64.to_le_bytes());
    meta.extend_from_slice(&0u32.to_le_bytes());
    meta.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    meta.extend_from_slice(&0i32.to_le_bytes());
    meta.extend_from_slice(&0i32.to_le_bytes());
    meta.extend_from_slice(&0i32.to_le_bytes());
    meta.push(0);
    let data_offset = 20 + meta.len();
    let mut out = Vec::new();
    be32(&mut out, meta.len() as u32);
    be32(&mut out, (data_offset + payload.len()) as u32);
    be32(&mut out, 17);
    be32(&mut out, data_offset as u32);
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&meta);
    out.extend_from_slice(payload);
    out
}

const PAYLOAD: [u8; 20] = [
    3, 0, 0, 0, b'a', b'b', b'c', 0, 2, 0, 0, 0, 9, 9, 0, 0, 7, 0, 0, 0,
];

#[test]
fn a_built_bundle_opens_and_its_node_reads_back() {
    let sf = serialized(&PAYLOAD);
    let b = Bundle::parse(&bundle(&[("CAB-x", &sf), ("CAB-x.resS", b"stream")])).unwrap();
    assert_eq!(
        (b.archive_version, b.engine_version.as_str()),
        (6, "2018.3.8f1")
    );
    assert_eq!(b.nodes().len(), 2);
    assert_eq!(b.node_by_path("CAB-x.resS"), Some(&b"stream"[..]));
    let f = SerializedFile::parse(b.node_by_path("CAB-x").unwrap()).unwrap();
    let v = f.read_object(&f.objects[0]).unwrap();
    assert_eq!(v.get("m_Name"), Some(&Value::String("abc".into())));
    assert_eq!(v.get("m_Data"), Some(&Value::Bytes(vec![9, 9])));
    assert_eq!(v.get("m_Num").and_then(Value::as_i64), Some(7));
}

#[test]
fn leftover_or_missing_object_bytes_are_named() {
    let mut longer = PAYLOAD.to_vec();
    longer.extend_from_slice(&[0, 0, 0, 0]);
    let f = serialized(&longer);
    let f = SerializedFile::parse(&f).unwrap();
    assert_eq!(
        f.read_object(&f.objects[0]),
        Err(Error::SizeMismatch {
            expected: 24,
            consumed: 20
        })
    );
    let f = serialized(&PAYLOAD[..18]);
    let f = SerializedFile::parse(&f).unwrap();
    assert!(matches!(
        f.read_object(&f.objects[0]),
        Err(Error::Truncated { .. })
    ));
}

#[test]
fn an_array_length_beyond_the_object_is_refused() {
    let mut p = PAYLOAD;
    p[8..12].copy_from_slice(&0x7fff_ffffu32.to_le_bytes());
    let f = serialized(&p);
    let f = SerializedFile::parse(&f).unwrap();
    assert!(matches!(
        f.read_object(&f.objects[0]),
        Err(Error::LimitExceeded {
            what: "array length",
            ..
        })
    ));
}

#[test]
fn every_truncation_and_many_corruptions_are_errors_not_panics() {
    let sf = serialized(&PAYLOAD);
    let whole = bundle(&[("CAB-x", &sf)]);
    for n in 0..whole.len() {
        assert!(Bundle::parse(&whole[..n]).is_err(), "prefix {n}");
    }
    for n in 0..sf.len() {
        // Either outcome is fine; it must return.
        if let Ok(f) = SerializedFile::parse(&sf[..n]) {
            for o in &f.objects {
                let _ = f.read_object(o);
            }
        }
    }
    let mut x = 0x2545_f491u32;
    for _ in 0..20_000 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        let mut b = whole.clone();
        let at = x as usize % b.len();
        b[at] ^= 1 << (x >> 29);
        if let Ok(bundle) = Bundle::parse(&b) {
            for node in bundle.nodes() {
                if let Ok(f) = SerializedFile::parse(bundle.node_bytes(node)) {
                    for o in &f.objects {
                        let _ = f.read_object(o);
                    }
                }
            }
        }
        let mut s = sf.clone();
        let at = x as usize % s.len();
        s[at] ^= 1 << (x >> 29);
        if let Ok(f) = SerializedFile::parse(&s) {
            for o in &f.objects {
                let _ = f.read_object(o);
            }
        }
    }
}

#[test]
fn a_header_that_lies_about_sizes_is_refused() {
    let sf = serialized(&PAYLOAD);
    let ok = bundle(&[("CAB-x", &sf)]);
    // Declared total size larger than the bytes given.
    let mut b = ok.clone();
    let size_at = 8 + 4 + 6 + 11;
    b[size_at..size_at + 8].copy_from_slice(&i64::MAX.to_be_bytes());
    assert!(matches!(Bundle::parse(&b), Err(Error::Malformed(_))));
    // A tiny decompressed-size limit.
    let limits = Limits {
        max_decompressed: 10,
        max_nodes: 4,
    };
    assert!(matches!(
        Bundle::parse_with(&ok, &limits),
        Err(Error::LimitExceeded { .. })
    ));
    assert_eq!(Bundle::parse(b"nope").unwrap_err(), Error::BadSignature);
}

#[test]
fn lz4_decodes_literals_and_overlapping_matches_and_refuses_bad_blocks() {
    // 5 literals "abcab", then a match of offset 5 length 4+? : token 0x50 = 5 literals, match len 4.
    let block = [0x50, b'a', b'b', b'c', b'a', b'b', 5, 0];
    assert_eq!(lz4_block(&block, 9).unwrap(), b"abcababca");
    // A run: one literal then offset 1 with a long match.
    let run = [0x1f, b'x', 1, 0, 6];
    assert_eq!(lz4_block(&run, 1 + 4 + 15 + 6).unwrap(), vec![b'x'; 26]);
    assert!(lz4_block(&[0x50, b'a'], 5).is_err(), "literals run off");
    assert!(
        lz4_block(&[0x10, b'a', 9, 0], 5).is_err(),
        "offset before the start"
    );
    assert!(lz4_block(&[0x10, b'a', 0, 0], 5).is_err(), "zero offset");
    assert!(
        lz4_block(&block, 100).is_err(),
        "output shorter than declared"
    );
    assert!(lz4_block(&block, 5).is_err(), "output longer than declared");
    assert!(
        lz4_block(&[0x10, b'a'], usize::MAX).is_err(),
        "absurd declared size"
    );
}

mod mesh {
    use oag_unity::classes::Mesh;
    use oag_unity::error::Error;
    use oag_unity::value::{Struct, Value};
    use std::sync::Arc;

    fn st(name: &str, fields: Vec<(&str, Value)>) -> Value {
        Value::Struct(Struct {
            type_name: Arc::from(name),
            fields: fields.into_iter().map(|(n, v)| (Arc::from(n), v)).collect(),
        })
    }

    fn channel(stream: u64, offset: u64, format: u64, dimension: u64) -> Value {
        st(
            "ChannelInfo",
            vec![
                ("stream", Value::UInt(stream)),
                ("offset", Value::UInt(offset)),
                ("format", Value::UInt(format)),
                ("dimension", Value::UInt(dimension)),
            ],
        )
    }

    /// Five vertices over two streams: stream 0 holds float positions, stream 1
    /// holds an 8-bit colour then half-float UVs. Stream 1 starts at the next
    /// multiple of 16 after stream 0's 60 bytes.
    fn mesh_value(compression: i64, colour_format: u64, data_len_delta: isize) -> Value {
        let mut channels = vec![channel(0, 0, 0, 0); 14];
        channels[0] = channel(0, 0, 0, 3);
        channels[3] = channel(1, 0, colour_format, 4);
        channels[4] = channel(1, 4, 1, 2);
        let mut data = Vec::new();
        for v in 0..5u32 {
            for c in [v as f32, v as f32 * 2.0, -(v as f32)] {
                data.extend_from_slice(&c.to_le_bytes());
            }
        }
        data.resize(64, 0);
        for v in 0..5u8 {
            data.extend_from_slice(&[255, 0, 51, v]);
            // 1.0, 2.0, -2.0, 0.5 as binary16, cycling.
            let h: [u16; 2] = [
                [0x3c00, 0x4000, 0xc000, 0x3800, 0x3c00][usize::from(v)],
                0x3800,
            ];
            for x in h {
                data.extend_from_slice(&x.to_le_bytes());
            }
        }
        data.resize((data.len() as isize + data_len_delta) as usize, 0);
        let index: Vec<u8> = [0u16, 1, 2, 3, 4]
            .iter()
            .flat_map(|i| i.to_le_bytes())
            .collect();
        let sub = st(
            "SubMesh",
            vec![
                ("firstByte", Value::UInt(0)),
                ("indexCount", Value::UInt(5)),
                ("topology", Value::Int(1)),
                ("baseVertex", Value::UInt(0)),
                ("firstVertex", Value::UInt(0)),
                ("vertexCount", Value::UInt(5)),
            ],
        );
        st(
            "Mesh",
            vec![
                ("m_Name", Value::String("synthetic".into())),
                ("m_SubMeshes", Value::Array(vec![sub])),
                ("m_MeshCompression", Value::UInt(compression as u64)),
                ("m_IndexFormat", Value::Int(0)),
                ("m_IndexBuffer", Value::Bytes(index)),
                (
                    "m_VertexData",
                    st(
                        "VertexData",
                        vec![
                            ("m_VertexCount", Value::UInt(5)),
                            ("m_Channels", Value::Array(channels)),
                            ("m_DataSize", Value::Bytes(data)),
                        ],
                    ),
                ),
            ],
        )
    }

    #[test]
    fn two_streams_half_floats_colours_and_strips_decode() {
        let m = Mesh::from_value(&mesh_value(0, 2, 0), 2018).unwrap();
        assert_eq!(m.vertex_count, 5);
        assert_eq!(m.positions().unwrap().unwrap()[3], [3.0, 6.0, -3.0]);
        let uv = m.uvs(0).unwrap().unwrap();
        assert_eq!(
            (uv[0], uv[1], uv[2], uv[3]),
            ([1.0, 0.5], [2.0, 0.5], [-2.0, 0.5], [0.5, 0.5])
        );
        let c = m.colors().unwrap().unwrap();
        assert_eq!(c[2], [1.0, 0.0, 0.2, 2.0 / 255.0]);
        assert_eq!(
            m.triangles(0).unwrap(),
            vec![[0, 1, 2], [2, 1, 3], [2, 3, 4]]
        );
    }

    #[test]
    fn unverified_tables_compressed_meshes_and_wrong_sizes_are_named() {
        // Format id 3 is 8-bit UInt in one pre-2019 table and UNorm8 in another: refused, not guessed.
        assert!(matches!(
            Mesh::from_value(&mesh_value(0, 3, 0), 2018),
            Err(Error::Unsupported(_))
        ));
        assert!(Mesh::from_value(&mesh_value(0, 3, 0), 2019).is_ok());
        assert!(matches!(
            Mesh::from_value(&mesh_value(1, 2, 0), 2018),
            Err(Error::Unsupported("compressed mesh"))
        ));
        assert!(matches!(
            Mesh::from_value(&mesh_value(0, 2, 1), 2018),
            Err(Error::Malformed(_))
        ));
        assert!(matches!(
            Mesh::from_value(&mesh_value(0, 2, -1), 2018),
            Err(Error::Malformed(_))
        ));
    }
}
