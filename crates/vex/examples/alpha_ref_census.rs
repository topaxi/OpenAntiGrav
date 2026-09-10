//! Scratch census: which of `Gfx_BuildBatchStateList`'s three alpha-test
//! references each alpha-tested batch on the Pulse PSP disc would get.
//!
//! ```sh
//! cargo run -q -p oag-vex --example alpha_ref_census -- data/images/pulse-psp-usa.chd
//! ```

use std::collections::BTreeMap;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::vex;

fn psp_vex_blobs(disc: &mut DiscImage, archive_path: &str) -> Vec<(usize, Vec<u8>)> {
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path.ends_with(archive_path))
        .unwrap_or_else(|| panic!("{archive_path} present"))
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
    for (index, entry) in dir.entries.iter().enumerate() {
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
        if !vex::has_magic(&bytes) {
            continue;
        }
        out.push((index, bytes));
    }
    out
}

/// The bucket `Gfx_BuildBatchStateList` puts a batch in, in its own branch
/// order.
fn bucket(pass_mask: u16, header_flags: u8) -> &'static str {
    if header_flags & 0x10 != 0 {
        "no-test (header_flags & 0x10)"
    } else if pass_mask & 0x0700 != 0 {
        "ref 0 (transparent)"
    } else if pass_mask & 0x0800 == 0 {
        "no-test (ALWAYS, not alpha-tested)"
    } else if header_flags & 0x20 != 0 {
        "ref 0x10"
    } else if pass_mask & 0x0080 != 0 {
        "ref 0"
    } else {
        "ref 0x7f"
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("disc image path");
    let mut disc = DiscImage::open(&path).expect("open");
    let archive = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "PSP_GAME/USRDIR/Data.wad".to_string());
    let blobs = psp_vex_blobs(&mut disc, &archive);

    let mut all: BTreeMap<&'static str, usize> = BTreeMap::new();
    // Only batches the renderer routes into `alpha_tested_draws`:
    // `!is_transparent() && is_alpha_tested()`.
    let mut cutout: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut cutout_bits: BTreeMap<(u16, u8), usize> = BTreeMap::new();
    let mut vertex_alpha: BTreeMap<&'static str, BTreeMap<u8, usize>> = BTreeMap::new();
    let mut meshes = 0usize;
    let mut batches = 0usize;

    // Per bucket: texels currently drawn (alpha > 0) that the recovered
    // reference would newly discard, and batches whose texture vanishes whole.
    let mut newly_discarded: BTreeMap<&'static str, (u64, u64)> = BTreeMap::new();
    let mut vanishing_batches: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut textured_batches: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut vanishing_names: BTreeMap<String, usize> = BTreeMap::new();
    #[allow(clippy::type_complexity)]
    let mut every_texture: BTreeMap<(&'static str, String), (usize, BTreeMap<u8, u64>)> =
        BTreeMap::new();
    #[allow(clippy::type_complexity)]
    let mut per_texture: BTreeMap<
        (&'static str, String),
        (u64, u64, usize, BTreeMap<u8, u64>),
    > = BTreeMap::new();

    for (_, bytes) in &blobs {
        let Ok(nodes) = vex::nodes(bytes) else {
            continue;
        };
        // Alpha histogram of each of this model's textures, decoded once.
        #[allow(clippy::type_complexity)]
        let tex_alpha: Vec<Option<(BTreeMap<u8, u64>, Option<String>)>> = vex::textures(bytes)
            .map(|ts| {
                ts.into_iter()
                    .map(|t| {
                        t.map(|t| {
                            let rgba = t.to_rgba();
                            let mut h: BTreeMap<u8, u64> = BTreeMap::new();
                            for texel in rgba.as_chunks::<4>().0 {
                                *h.entry(texel[3]).or_default() += 1;
                            }
                            (h, t.asset_path.clone())
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let Ok(classes) = vex::classes_of(bytes) else {
            continue;
        };
        // A pad node's payload is a mesh payload - see `mesh::build_pads` -
        // and the pads are where the third selector pattern lives.
        let drawn: Vec<u32> = [classes.mesh, classes.speedup_pad, classes.weapon_pad]
            .into_iter()
            .flatten()
            .collect();
        for node in nodes.iter().filter(|n| drawn.contains(&n.class_id)) {
            let range = node.payload();
            if range.end > bytes.len() {
                continue;
            }
            let payload = &bytes[range];
            meshes += 1;
            let materials = vex::mesh_materials(payload);
            for list in 0..2 {
                let Ok(list_batches) = vex::mesh_batches(payload, list) else {
                    continue;
                };
                for b in &list_batches {
                    batches += 1;
                    let k = bucket(b.pass_mask, b.header_flags);
                    *all.entry(k).or_default() += 1;
                    // Every batch, whatever bucket, so a named texture can be
                    // located rather than only counted where it is a cutout.
                    if let Some(Some((hist, name))) = materials
                        .get(usize::from(b.material_index))
                        .copied()
                        .flatten()
                        .map(|m| m.texture as usize)
                        .and_then(|t| tex_alpha.get(t))
                    {
                        let e = every_texture
                            .entry((k, name.clone().unwrap_or_else(|| "<unnamed>".into())))
                            .or_insert((0usize, hist.clone()));
                        e.0 += 1;
                    }
                    if !b.is_transparent() && b.is_alpha_tested() {
                        *cutout.entry(k).or_default() += 1;
                        *cutout_bits
                            .entry((b.pass_mask & 0x0880, b.header_flags & 0x30))
                            .or_default() += 1;
                        let refv: Option<u8> = match k {
                            "ref 0x7f" => Some(0x7f),
                            "ref 0x10" => Some(0x10),
                            _ => None,
                        };
                        if let (Some(refv), Some(Some((hist, name)))) = (
                            refv,
                            materials
                                .get(usize::from(b.material_index))
                                .copied()
                                .flatten()
                                .map(|m| m.texture as usize)
                                .and_then(|t| tex_alpha.get(t)),
                        ) {
                            *textured_batches.entry(k).or_default() += 1;
                            let drawn: u64 =
                                hist.iter().filter(|(a, _)| **a > 0).map(|(_, n)| n).sum();
                            let kept: u64 = hist
                                .iter()
                                .filter(|(a, _)| **a > refv)
                                .map(|(_, n)| n)
                                .sum();
                            let e2 = newly_discarded.entry(k).or_default();
                            e2.0 += drawn;
                            e2.1 += drawn - kept;
                            let per = per_texture
                                .entry((k, name.clone().unwrap_or_else(|| "<unnamed>".into())))
                                .or_insert((0u64, 0u64, 0usize, hist.clone()));
                            per.0 += drawn;
                            per.1 += drawn - kept;
                            per.2 += 1;
                            if kept == 0 {
                                *vanishing_batches.entry(k).or_default() += 1;
                                *vanishing_names
                                    .entry(name.clone().unwrap_or_else(|| "<unnamed>".into()))
                                    .or_default() += 1;
                            }
                        }
                        let e = vertex_alpha.entry(k).or_default();
                        for v in &b.vertices {
                            if let Some(c) = v.colour {
                                *e.entry(c[3]).or_default() += 1usize;
                            } else {
                                *e.entry(255).or_default() += 1usize;
                            }
                        }
                    }
                }
            }
        }
    }

    println!("blobs={} meshes={meshes} batches={batches}", blobs.len());
    println!("\n== every batch, by the original's own branch ==");
    for (k, v) in &all {
        println!("{v:>8}  {k}");
    }
    println!("\n== the renderer's cutout bucket (!is_transparent && is_alpha_tested) ==");
    for (k, v) in &cutout {
        println!("{v:>8}  {k}");
    }
    println!("\n== cutout bucket, vertex-colour alpha histogram ==");
    for (k, h) in &vertex_alpha {
        let total: usize = h.values().sum();
        let distinct: Vec<_> = h.iter().map(|(a, n)| format!("{a}:{n}")).collect();
        println!(
            "  {k}: {total} verts, {} distinct alphas: {}",
            h.len(),
            distinct.join(" ")
        );
    }
    println!("\n== texel impact of the recovered reference, per bucket ==");
    for (k, (drawn, discarded)) in &newly_discarded {
        let pct = if *drawn > 0 {
            (*discarded as f64) * 100.0 / (*drawn as f64)
        } else {
            0.0
        };
        println!(
            "  {k}: {} textured batches, {drawn} texels with alpha>0, {discarded} newly discarded ({pct:.2}%), {} batches vanish whole",
            textured_batches.get(k).copied().unwrap_or(0),
            vanishing_batches.get(k).copied().unwrap_or(0)
        );
    }
    println!("\n== textures that vanish whole ==");
    for (n, c) in &vanishing_names {
        println!("  {c:>6}  {n}");
    }
    let mut ranked: Vec<_> = per_texture.iter().collect();
    ranked.sort_by_key(|(_, v)| std::cmp::Reverse(v.1));
    println!("\n== top cutout textures by newly-discarded texels ==");
    for ((k, name), (drawn, discarded, batches, hist)) in ranked.iter().take(12) {
        let lo: Vec<String> = hist
            .iter()
            .filter(|(a, _)| **a > 0 && **a <= 0x7f)
            .map(|(a, n)| format!("{a}:{n}"))
            .take(8)
            .collect();
        println!(
            "  {k:>8}  {name}  {batches} batches, {drawn} drawn, {discarded} discarded; low alphas: {}",
            lo.join(" ")
        );
    }
    if let Ok(want) = std::env::var("OAG_PROBE_TEX") {
        println!("\n== textures matching {want:?} ==");
        for ((k, name), (batches, hist)) in &every_texture {
            if !name.contains(&want) {
                continue;
            }
            println!("  {k}  {name}  {batches} batches");
            println!("    alpha histogram: {hist:?}");
        }
    }
    println!("\n== cutout bucket, raw selector bits (pass_mask & 0x880, header_flags & 0x30) ==");
    for ((pm, hf), v) in &cutout_bits {
        println!("{v:>8}  pass_mask&0x880={pm:#06x} header_flags&0x30={hf:#04x}");
    }
}
