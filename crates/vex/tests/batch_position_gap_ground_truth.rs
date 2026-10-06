//! What batch `+0x14` is not - and what `+0x10` beside it is - censused over
//! the whole disc.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! # What this is for
//!
//! [`vex.md`](../../../docs/formats/vex.md) lists the 4 bytes at batch
//! `+0x14` (between the position scale at `+0x10` and the s16 bounding box at
//! `+0x18`) as undetermined. That leaves the field's most basic property
//! unchecked: whether it carries data at all, or is padding nobody wrote
//! anything meaningful into. This is the check, not the decode - no
//! consuming instruction has been found for it, so nothing here is claimed
//! as a name.
//!
//! Its neighbour `+0x10` *is* decoded, and the second test here holds up the
//! file-side invariant the runtime's read of it depends on.

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::vex;

const PSP_DATA: &str = "PSP_GAME/USRDIR/Data.wad";

/// Same convention `vex::mesh_batches` uses to decode the s16 bounding box.
const POSITION_DIVISOR: f32 = 32768.0;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// Every version-6 `.vex` blob in `Data.wad`, decompressed. Same walk as
/// `vex_layer_ground_truth.rs`.
fn psp_vex_blobs(disc: &mut DiscImage) -> Vec<Vec<u8>> {
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == PSP_DATA)
        .unwrap_or_else(|| panic!("{PSP_DATA} present"))
        .clone();

    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, Directory::directory_len(count))
        .expect("directory");
    let dir = Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    let mut out = Vec::new();
    for entry in &dir.entries {
        if entry.size == 0 {
            continue;
        }
        let raw = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let bytes = match entry.compression {
            Compression::None => raw,
            Compression::Lzss => {
                oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize).expect("lzss")
            }
            Compression::Zlib => continue,
        };
        if !vex::has_magic(&bytes) || vex::version(&bytes) != Ok(6) {
            continue;
        }
        out.push(bytes);
    }
    out
}

/// One batch's `+0x14` field alongside enough context to test candidate
/// readings against: the position scale it sits next to, and the batch's own
/// bounding-box size in the same (scaled) units the vertices decode to.
struct Row {
    /// The raw 4 bytes at `+0x14`, read as an `f32` - a choice, not a finding.
    /// Nothing here establishes the field is one `f32` rather than, say, two
    /// packed `s16` halves the way the texture-transform block's tracks are;
    /// `f32` is what this test evaluates because it is what the position
    /// scale next to it already is.
    gap_as_f32: f32,
    scale: f32,
    half_diagonal: f32,
}

/// Re-walks a batch list the way `vex::mesh_batches` does internally, keeping
/// `+0x14` instead of discarding it. VIF (PS2-shaped) batches are skipped -
/// their layout puts the bounding box somewhere else entirely.
fn walk(payload: &[u8], batch_list: u8, out: &mut Vec<Row>) {
    let u16_at = |p: &[u8], o: usize| u16::from_le_bytes([p[o], p[o + 1]]);
    let u32_at = |p: &[u8], o: usize| u32::from_le_bytes([p[o], p[o + 1], p[o + 2], p[o + 3]]);
    let f32_at = |p: &[u8], o: usize| f32::from_bits(u32_at(p, o));

    let list_offset = u32_at(payload, if batch_list == 0 { 4 } else { 8 }) as usize;
    let terminator: u16 = if batch_list == 0 { 1 } else { 2 };
    let mut at = list_offset;
    while at + 0x40 <= payload.len() {
        let pass_mask = u16_at(payload, at);
        if pass_mask & terminator == 0 {
            break;
        }
        let flags = payload[at + 3];
        let header_size = if flags & 0x40 != 0 { 0x80 } else { 0x40 };
        let vertex_type = u16_at(payload, at + 0x0a);
        let payload_size = usize::from(u16_at(payload, at + 0x0c));
        let scale = f32_at(payload, at + 0x10);

        if vex::is_vif_batch(vertex_type) {
            break;
        }

        let corner = |off: usize| -> [f32; 3] {
            let mut v = [0.0f32; 3];
            for (i, c) in v.iter_mut().enumerate() {
                let raw =
                    i16::from_le_bytes([payload[at + off + i * 2], payload[at + off + i * 2 + 1]]);
                *c = f32::from(raw) / POSITION_DIVISOR * scale;
            }
            v
        };
        let min = corner(0x18);
        let max = corner(0x20);
        let diag = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
        let half_diagonal =
            0.5 * (diag[0] * diag[0] + diag[1] * diag[1] + diag[2] * diag[2]).sqrt();

        out.push(Row {
            gap_as_f32: f32_at(payload, at + 0x14),
            scale,
            half_diagonal,
        });

        at += header_size + payload_size;
    }
}

fn percentile(v: &mut [f32], p: f64) -> f32 {
    v.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
    v[((v.len() - 1) as f64 * p).round() as usize]
}

/// **`+0x14` is not padding.** It is nonzero, finite and positive on every
/// batch surveyed - the signature of a field something wrote a real value
/// into, not of an unused gap. Whether that value is an `f32` at all, and
/// what it means, are both still open; see `vex.md`.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn batch_plus_0x14_is_never_zero_or_negative() {
    let Some(path) = image() else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let blobs = psp_vex_blobs(&mut disc);

    let mut rows = Vec::new();
    for bytes in &blobs {
        let Ok(nodes) = vex::nodes(bytes) else {
            continue;
        };
        for node in nodes.iter().filter(|n| n.class_id == vex::CLASS_MESH) {
            let range = node.payload();
            if range.end > bytes.len() {
                continue;
            }
            let payload = &bytes[range];
            for list in [0u8, 1] {
                walk(payload, list, &mut rows);
            }
        }
    }

    const MIN_BATCHES: usize = 40_000;
    assert!(
        rows.len() >= MIN_BATCHES,
        "{} batches is too few to census; the walk is broken",
        rows.len()
    );

    let zero = rows.iter().filter(|r| r.gap_as_f32 == 0.0).count();
    let negative = rows.iter().filter(|r| r.gap_as_f32 < 0.0).count();
    let non_finite = rows.iter().filter(|r| !r.gap_as_f32.is_finite()).count();
    println!(
        "{} batches: {zero} zero, {negative} negative, {non_finite} non-finite at +0x14",
        rows.len()
    );
    assert_eq!(
        zero, 0,
        "+0x14 is zero on some batches - it is not universally populated"
    );
    assert_eq!(negative, 0, "+0x14 reads negative as f32 on some batches");
    assert_eq!(non_finite, 0, "+0x14 is not finite as f32 on some batches");

    // Not asserted - the ratio bands are a lead, not an invariant this test
    // pins. Printed so `--nocapture` shows the measurement `vex.md` cites.
    let mut ratio_scale: Vec<f32> = rows
        .iter()
        .filter(|r| r.scale != 0.0)
        .map(|r| r.gap_as_f32 / r.scale)
        .collect();
    let mut ratio_half_diag: Vec<f32> = rows
        .iter()
        .filter(|r| r.half_diagonal != 0.0)
        .map(|r| r.gap_as_f32 / r.half_diagonal)
        .collect();
    println!(
        "gap/scale: p0={:.4} p50={:.4} p100={:.4}",
        percentile(&mut ratio_scale.clone(), 0.0),
        percentile(&mut ratio_scale.clone(), 0.5),
        percentile(&mut ratio_scale, 1.0),
    );
    println!(
        "gap/half_diagonal: p0={:.4} p10={:.4} p50={:.4} p90={:.4} p100={:.4}",
        percentile(&mut ratio_half_diag.clone(), 0.0),
        percentile(&mut ratio_half_diag.clone(), 0.1),
        percentile(&mut ratio_half_diag.clone(), 0.5),
        percentile(&mut ratio_half_diag.clone(), 0.9),
        percentile(&mut ratio_half_diag, 1.0),
    );
}

/// **Every batch in a mesh carries a bit-identical position scale**, and
/// `+0x14` does not - which is what makes the runtime's own read of the scale
/// safe.
///
/// `Mesh_InitFromPayload` (`0x0890e998`) walks both batch lists purely to find
/// the **last** batch, then takes that one batch's `+0x10` as the whole mesh's
/// scale (`lwc1 f12, 0x10(s2)` at `0x0890ef5c`, stored to the runtime mesh at
/// `+0x54`) - see
/// [vex.md](../../../docs/formats/vex.md#the-scale-which-decides-whether-models-come-out-the-right-size).
/// A single batch disagreeing would silently rescale the entire model, so the
/// file must guarantee uniformity for that read to be correct; this asserts the
/// disc actually does. It also pins the difference between the two fields: the
/// scale is a mesh-wide constant stored per batch, `+0x14` genuinely varies
/// batch to batch, so whatever `+0x14` is, it is not a second copy of something
/// mesh-wide.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_batch_in_a_mesh_carries_the_same_position_scale() {
    let Some(path) = image() else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let blobs = psp_vex_blobs(&mut disc);

    let mut meshes = 0usize;
    let mut batches = 0usize;
    let mut scale_varies = 0usize;
    let mut gap_uniform = 0usize;
    for bytes in &blobs {
        let Ok(nodes) = vex::nodes(bytes) else {
            continue;
        };
        for node in nodes.iter().filter(|n| n.class_id == vex::CLASS_MESH) {
            let range = node.payload();
            if range.end > bytes.len() {
                continue;
            }
            let payload = &bytes[range];
            let mut rows = Vec::new();
            for list in [0u8, 1] {
                walk(payload, list, &mut rows);
            }
            let Some(first) = rows.first() else {
                continue;
            };
            meshes += 1;
            batches += rows.len();
            let scale = first.scale.to_bits();
            if rows.iter().any(|r| r.scale.to_bits() != scale) {
                scale_varies += 1;
            }
            let gap = first.gap_as_f32.to_bits();
            if rows.iter().all(|r| r.gap_as_f32.to_bits() == gap) {
                gap_uniform += 1;
            }
        }
    }

    const MIN_MESHES: usize = 20_000;
    assert!(
        meshes >= MIN_MESHES,
        "{meshes} meshes is too few to census; the walk is broken"
    );
    println!(
        "{meshes} meshes, {batches} batches: {scale_varies} with a varying +0x10, \
         {gap_uniform} with a uniform +0x14"
    );
    assert_eq!(
        scale_varies, 0,
        "some mesh's batches disagree about +0x10 - the runtime reads only the \
         last batch's copy, so that mesh would be drawn at the wrong size"
    );
    assert!(
        gap_uniform < meshes,
        "+0x14 is uniform within every mesh, which would make it mesh-wide data \
         rather than the per-batch value this file treats it as"
    );
}
