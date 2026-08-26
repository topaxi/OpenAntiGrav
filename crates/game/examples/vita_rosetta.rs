//! Scratch probe: place Wipeout 2048's 96-byte `WO Track` control point by
//! pairing a DLC circuit against Wipeout HD's own 112-byte copy of it.
//!
//! Twelve circuits ship in both titles with identical path and control-point
//! counts, so point `k` of path `i` is the same point in both files and HD's
//! decoded `0x106` record is ground truth for 2048's `0x107` one. Reproduces
//! every number in the "Wipeout 2048 authors version `0x107`" section of
//! `docs/formats/track.md`.
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_rosetta
//! ```

use std::collections::BTreeSet;

use oag_formats::{ByteOrder, track, vex};

const HD_DIR: &str = "data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR";
const VITA_DLC: [&str; 2] = [
    "data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];
const VITA_BASE: &str = "data/extracted/vita/PCSF00007/base/PSP2/data.psarc";

/// `(HD archive, HD entry, 2048 circuit directory)` for every circuit both
/// titles ship, paired by circuit name alone.
const PAIRS: &[(&str, &str, &str)] = &[
    (
        "DATA02",
        "/data/environments/01_vineta_k/track.vex",
        "Vineta_K",
    ),
    (
        "DATA02",
        "/data/environments/05_ubermall/track.vex",
        "Ubermall",
    ),
    (
        "DATA02",
        "/data/environments/10_sebenco_climb/track.vex",
        "Sebenco_Climb",
    ),
    ("DATA02", "/data/environments/12_sol_2/track.vex", "Sol_2"),
    (
        "DATA00",
        "/data/environments/amphiseum/track.vex",
        "amphiseum",
    ),
    (
        "DATA00",
        "/data/environments/modesto_heights/track.vex",
        "modesto_heights",
    ),
    (
        "DATA00",
        "/data/environments/talons_junction/track.vex",
        "talons_junction",
    ),
    (
        "DATA00",
        "/data/environments/tech_de_ra/track.vex",
        "tech_de_ra",
    ),
    ("DATA00", "/data/environments/zone_1/track.vex", "zone_1"),
    ("DATA00", "/data/environments/zone_2/track.vex", "zone_2"),
    ("DATA00", "/data/environments/zone_3/track.vex", "zone_3"),
    ("DATA00", "/data/environments/zone_4/track.vex", "zone_4"),
];

/// One float field: its name, the offset this probe claims it sits at in
/// 2048's record, and how to read it out of HD's decode.
type Field = (&'static str, usize, fn(&track::SplinePoint) -> f32);

/// The five floats HD's record carries past `lateral`.
const FIELDS: &[Field] = &[
    ("half_width_left", 0x44, |p| p.half_width_left),
    ("half_width_right", 0x48, |p| p.half_width_right),
    ("ai_bound_left", 0x4c, |p| p.ai_bound_left),
    ("ai_bound_right", 0x50, |p| p.ai_bound_right),
    ("racing_line", 0x54, |p| p.racing_line),
];

/// One paired control point: 2048's raw 96 bytes, HD's raw 112, HD's decode.
struct Paired {
    vita: Vec<u8>,
    hd_raw: Vec<u8>,
    hd: track::SplinePoint,
}

fn main() -> anyhow::Result<()> {
    let mut all: Vec<Paired> = Vec::new();
    println!("=== per circuit ===");
    for &(archive, entry, name) in PAIRS {
        let rows = pair(archive, entry, name)?;
        report_sections(name, &rows);
        all.extend(rows);
    }
    println!("\npaired control points: {}", all.len());
    report_words(&all);
    report_floats(&all);
    report_flags(&all);
    report_native()?;
    Ok(())
}

/// Where does the 2048 word stop agreeing with HD's at the same offset?
fn report_words(all: &[Paired]) {
    println!(
        "\n=== 2048 word == HD word, same offset (of {}) ===",
        all.len()
    );
    for w in 0..24 {
        let hits = all
            .iter()
            .filter(|p| {
                ByteOrder::Little.u32(&p.vita, w * 4) == ByteOrder::Big.u32(&p.hd_raw, w * 4)
            })
            .count();
        println!("  {:>#4x}  {hits:>6}", w * 4);
    }
}

/// Exact matches per float, and how far apart the rest are.
fn report_floats(all: &[Paired]) {
    println!("\n=== floats (of {}) ===", all.len());
    for &(name, off, get) in FIELDS {
        let mut exact = 0usize;
        let mut hd_nonzero = 0usize;
        let mut worst = 0.0f32;
        for p in all {
            let a = ByteOrder::Little.f32(&p.vita, off);
            let b = get(&p.hd);
            if a.to_bits() == b.to_bits() {
                exact += 1;
            }
            if b != 0.0 {
                hd_nonzero += 1;
                worst = worst.max((a - b).abs() / b.abs());
            }
        }
        println!(
            "  {name:>17} @{off:#04x}: exact {exact}, HD non-zero {hd_nonzero}, worst relative difference {worst:.4}"
        );
    }
}

/// Does the byte at `0x5d` hold HD's `flags` on every point?
fn report_flags(all: &[Paired]) {
    let nonzero = all.iter().filter(|p| p.hd.flags != 0).count();
    let hit = all.iter().filter(|p| p.vita[0x5d] == p.hd.flags).count();
    println!(
        "\nflags @0x5d: equals HD on {hit}/{} points, of which {nonzero} are non-zero",
        all.len()
    );
}

/// `section_id` is checked against each file's own scene tree, because 2048
/// re-sectioned most of the ports and the values themselves differ.
fn report_sections(name: &str, rows: &[Paired]) {
    let hd_changes: BTreeSet<usize> = (1..rows.len())
        .filter(|&k| rows[k].hd.section_id != rows[k - 1].hd.section_id)
        .collect();
    let mine: BTreeSet<usize> = (1..rows.len())
        .filter(|&k| rows[k].vita[0x5c] != rows[k - 1].vita[0x5c])
        .collect();
    let union = hd_changes.union(&mine).count();
    let agreement = if union == 0 {
        1.0
    } else {
        mine.intersection(&hd_changes).count() as f32 / union as f32
    };
    let ids: BTreeSet<u8> = rows.iter().map(|p| p.vita[0x5c]).collect();
    println!(
        "  {name:<16} {:>5} pts | 0x5c: {} distinct (max {:?}) vs {:?} `section` nodes | \
change-point agreement with HD {agreement:.3}",
        rows.len(),
        ids.len(),
        ids.iter().next_back(),
        section_count(&vita_blob(name).unwrap_or_default()),
    );
}

/// Every circuit 2048's base package ships, under the placed layout.
fn report_native() -> anyhow::Result<()> {
    let mut archive = oag_assets::psarc::Archive::open(VITA_BASE)?;
    let mut names: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with("/track.vex"))
        .cloned()
        .collect();
    names.sort();
    println!("\n=== 2048's own base package, through track::parse ===");
    let mut total = 0usize;
    for name in names {
        let Ok(blob) = archive.read_path(&name) else {
            continue;
        };
        let Some(range) = payload(&blob) else {
            continue;
        };
        let short = name.rsplit('/').nth(1).unwrap_or(&name).to_string();
        let bytes = blob[range.clone()].len();
        match track::parse(&blob[range]) {
            Ok(decoded) => {
                let points: usize = decoded.paths.iter().map(|p| p.points.len()).sum();
                total += points;
                let all: Vec<&track::SplinePoint> =
                    decoded.paths.iter().flat_map(|p| &p.points).collect();
                let widths = all
                    .iter()
                    .flat_map(|p| [p.half_width_left, p.half_width_right]);
                let bounds = all.iter().flat_map(|p| [p.ai_bound_left, p.ai_bound_right]);
                let sections: BTreeSet<u8> = all.iter().map(|p| p.section_id).collect();
                let flags: BTreeSet<u8> = all.iter().map(|p| p.flags).collect();
                println!(
                    "  {short:<17} {points:>5} pts | encoded_len {} vs payload {bytes} | \
half_width {:.2}..{:.2} | ai_bound {:.2}..{:.2} | sections {} (nodes {:?}) | flags {flags:?}",
                    decoded.encoded_len(),
                    widths.clone().fold(f32::MAX, f32::min),
                    widths.fold(f32::MIN, f32::max),
                    bounds.clone().fold(f32::MAX, f32::min),
                    bounds.fold(f32::MIN, f32::max),
                    sections.len(),
                    section_count(&blob),
                );
            }
            Err(e) => println!("  {short:<17} ERROR {e}"),
        }
    }
    println!("  total control points: {total}");
    Ok(())
}

/// The `WO Track` node's payload range in a `.vex` file.
fn payload(blob: &[u8]) -> Option<std::ops::Range<usize>> {
    let nodes = vex::nodes(blob).ok()?;
    Some(track::find_node(blob, &nodes)?.payload())
}

/// How many `section` nodes a file's own scene tree authors.
fn section_count(blob: &[u8]) -> Option<usize> {
    let id = vex::classes_of(blob).ok()?.section?;
    let nodes = vex::nodes(blob).ok()?;
    Some(nodes.iter().filter(|n| n.class_id == id).count())
}

fn vita_blob(name: &str) -> Option<Vec<u8>> {
    for psarc in VITA_DLC {
        let Ok(mut archive) = oag_assets::psarc::Archive::open(psarc) else {
            continue;
        };
        let entry = format!("data/art/published/DLC1/environments/{name}/track.vex");
        if let Ok(blob) = archive.read_path(&entry) {
            return Some(blob);
        }
    }
    None
}

/// Pairs one circuit's control points, refusing any circuit whose two copies
/// do not line up point for point.
fn pair(hd_archive: &str, hd_entry: &str, name: &str) -> anyhow::Result<Vec<Paired>> {
    let mut hd = oag_assets::psarc::Archive::open(&format!("{HD_DIR}/{hd_archive}.PSARC"))?;
    let hd_blob = hd.read_path(hd_entry)?;
    let range = payload(&hd_blob).ok_or_else(|| anyhow::anyhow!("{name}: HD has no WO Track"))?;
    let hd_payload = &hd_blob[range];
    let hd_track = track::parse(hd_payload)?;
    let mut hd_at = 0x40
        + ByteOrder::Big.u32(hd_payload, 8) as usize * 0x20
        + ByteOrder::Big.u32(hd_payload, 12) as usize * 0x10;

    let blob = vita_blob(name).ok_or_else(|| anyhow::anyhow!("{name}: no 2048 entry"))?;
    let range = payload(&blob).ok_or_else(|| anyhow::anyhow!("{name}: 2048 has no WO Track"))?;
    let vita = &blob[range];
    let order = ByteOrder::Little;
    let mut at = 0x40 + order.u32(vita, 8) as usize * 0x20 + order.u32(vita, 12) as usize * 0x10;

    let mut out = Vec::new();
    for (i, hd_path) in hd_track.paths.iter().enumerate() {
        let count = order.u32(vita, 0x40 + i * 0x20) as usize;
        anyhow::ensure!(
            count == hd_path.points.len(),
            "{name} path {i}: 2048 has {count} points, HD {}",
            hd_path.points.len()
        );
        for hd_point in &hd_path.points {
            out.push(Paired {
                vita: vita[at..at + track::POINT_LEN_SHORT].to_vec(),
                hd_raw: hd_payload[hd_at..hd_at + track::POINT_LEN].to_vec(),
                hd: *hd_point,
            });
            at += track::POINT_LEN_SHORT;
            hd_at += track::POINT_LEN;
        }
    }
    Ok(out)
}
