//! scratch
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::swizzle::{FLAG_SWIZZLED, swizzle};
use oag_formats::wad::{Compression, Directory};
use oag_texture::replacement::{self, Key};
use oag_texture::replacement_pack::Pack;
use oag_texture::{fnt, texture};

struct Obs {
    kind: &'static str,
    label: String,
    w: u32,
    h: u32,
    bpp: u32,
    clut: u32,
    /// (variant, hash)
    hashes: Vec<(&'static str, u32)>,
    pal: Vec<u8>,
    ram: Vec<u8>,
    reach: usize,
    clutv: Vec<(&'static str, u32)>,
    mem: Vec<u8>,
    own: usize,
}

fn walk(disc: &mut DiscImage, mut f: impl FnMut(&str, usize, &[u8])) {
    let entries = disc.entries().expect("entries").to_vec();
    for e in entries.iter().filter(|e| e.path.to_lowercase().ends_with(".wad")) {
        let header = disc.read_entry_range(e, 0, oag_formats::wad::HEADER_LEN as u64).unwrap();
        let count = Directory::peek_entry_count(&header).unwrap();
        let dir_bytes = disc.read_entry_range(e, 0, Directory::directory_len(count)).unwrap();
        let dir = Directory::parse(&dir_bytes, Some(e.size)).unwrap();
        for (i, ent) in dir.entries.iter().enumerate() {
            if ent.size == 0 {
                continue;
            }
            let raw = disc.read_entry_range(e, u64::from(ent.offset), u64::from(ent.size)).unwrap();
            let blob = match ent.compression {
                Compression::None => raw,
                Compression::Lzss => oag_formats::lzss::decompress(&raw, ent.size_uncompressed as usize).unwrap(),
                Compression::Zlib => continue,
            };
            f(&e.path, i, &blob);
        }
    }
}

fn pack_dir(name: &str) -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/texture-packs").join(name);
    p.join("textures.ini").exists().then_some(p)
}

fn hash_of(ram: &[u8], bufw: u32, w: u32, h: u32, bpp: u32) -> u32 {
    replacement::data_hash(ram, bufw, w, h, bpp, 0).unwrap_or(0)
}

/// The texel block as the GE sees it in RAM: flagged as stored, otherwise each
/// level swizzled at its own stride.
fn ram_block(block: &[u8], w: u32, h: u32, bpp: u32, levels: u8, flagged: bool, pad_level0: bool) -> Vec<u8> {
    if flagged {
        return block.to_vec();
    }
    let mut out = Vec::with_capacity(block.len());
    let mut at = 0usize;
    for l in 0..u32::from(levels.max(1)) {
        let lw = (w >> l).max(1);
        let lh = (h >> l).max(1) as usize;
        let row = (lw * bpp / 8).max(1) as usize;
        let stride = if l == 0 && !pad_level0 { row } else { row.next_multiple_of(16) };
        let Some(plane) = block.get(at..at + stride * lh) else { break };
        out.extend_from_slice(&swizzle(plane, stride, lh));
        at += stride * lh;
    }
    out.extend_from_slice(&block[at.min(block.len())..]);
    out
}

fn clut_from(mem: &[u8]) -> u32 {
    replacement::clut_hash(&mem[..mem.len().min(1024)])
}

fn observe(image: &str) -> Vec<Obs> {
    let path = oag_testdata::image(image).expect("image");
    let mut disc = DiscImage::open(&path).expect("open");
    let mut out = Vec::new();
    walk(&mut disc, |arch, idx, blob| {
        let label = format!("{arch}#{idx}");
        if oag_vex::vex::has_magic(blob) {
            let Ok(tree) = oag_vex::vex::nodes(blob) else { return };
            let Ok(stored) = oag_vex::vex::stored_textures(blob, &tree) else { return };
            // The whole texture region, as it sits in RAM: palette, texels, next palette...
            let mut region = Vec::new();
            let mut rawregion = Vec::new();
            let mut starts = Vec::new();
            for t in stored.iter() {
                starts.push(region.len());
                if let Some(t) = t {
                    let (w, h, bpp) = (u32::from(t.width), u32::from(t.height), u32::from(t.bits_per_pixel));
                    rawregion.extend_from_slice(t.palette);
                    rawregion.extend_from_slice(t.texels);
                    region.extend_from_slice(t.palette);
                    region.extend_from_slice(&ram_block(t.texels, w, h, bpp, t.mip_count, t.flags & FLAG_SWIZZLED != 0, true));
                }
            }
            for (n, t) in stored.iter().enumerate() {
                let Some(t) = t else { continue };
                let (w, h, bpp) = (u32::from(t.width), u32::from(t.height), u32::from(t.bits_per_pixel));
                let row = (usize::from(t.width) * usize::from(t.bits_per_pixel)).div_ceil(8);
                let stride = row.next_multiple_of(16);
                let at = starts[n] + t.palette.len();
                let Some(ram) = region.get(at..at + stride * usize::from(t.height)) else { continue };
                let bufw = (stride * 8) as u32 / bpp;
                out.push(Obs {
                    kind: "vex",
                    label: format!("{label}.{n}"),
                    w, h, bpp,
                    clut: clut_from(&region[starts[n]..]),
                    hashes: vec![("ram", hash_of(ram, bufw, w, h, bpp))],
                    pal: t.palette.to_vec(),
                    ram: Vec::new(),
                    reach: region.len() - starts[n],
                    mem: region[starts[n]..].iter().take(2048).copied().collect(),
                    own: t.palette.len() + t.texels.len(),
                    clutv: {
                        let mut padded = t.palette.to_vec();
                        padded.resize(1024, 0);
                        vec![("swz", clut_from(&region[starts[n]..])), ("raw", clut_from(&rawregion[starts[n]..])), ("zero", clut_from(&padded)), ("pal", replacement::clut_hash(t.palette))]
                    },
                });
            }
        } else if texture::Texture::looks_like_texture(blob) {
            let (w, h, bpp) = (u32::from(u16::from_le_bytes([blob[0], blob[1]])), u32::from(u16::from_le_bytes([blob[2], blob[3]])), u32::from(blob[4]));
            let pal = 4usize << bpp;
            let levels = blob[6].max(1);
            let flagged = blob[7] & FLAG_SWIZZLED != 0;
            let block = &blob[16 + pal..];
            let ram = ram_block(block, w, h, bpp, levels, flagged, false);
            let ramp = ram_block(block, w, h, bpp, levels, flagged, true);
            let row = (w * bpp / 8) as usize;
            let stride = row.next_multiple_of(16);
            let mut mem = blob[16..16 + pal].to_vec();
            mem.extend_from_slice(&ram);
            let l0 = &ram[..row * h as usize];
            // padded variant: copy rows to the padded stride
            let mut padded = vec![0u8; stride * h as usize];
            for y in 0..h as usize { padded[y * stride..y * stride + row].copy_from_slice(&block[y * row..(y + 1) * row]); }
            let lp = if flagged { padded.clone() } else { swizzle(&padded, stride, h as usize) };
            let _ = ramp;
            out.push(Obs {
                kind: "mip", label, w, h, bpp,
                clut: clut_from(&mem),
                hashes: vec![("A", hash_of(l0, w, w, h, bpp)), ("B", hash_of(&lp, (stride * 8) as u32 / bpp, w, h, bpp))],
                pal: blob[16..16 + pal].to_vec(),
                ram: Vec::new(),
                reach: mem.len(),
                mem: mem.iter().take(2048).copied().collect(),
                own: mem.len(),
                clutv: {
                    let mut raw = blob[16..].to_vec();
                    raw.truncate(1024);
                    let mut padded = blob[16..16 + pal].to_vec();
                    padded.resize(1024, 0);
                    vec![("swz", clut_from(&mem)), ("raw", replacement::clut_hash(&raw)), ("zero", clut_from(&padded)), ("pal", replacement::clut_hash(&blob[16..16 + pal]))]
                },
            });
        } else if blob.len() == oag_texture::ship_skin::FILE_LEN && oag_texture::ship_skin::parse(blob).is_ok() {
            let mut at = oag_texture::ship_skin::HEADER_LEN;
            for (b, side) in [128u32, 128, 128, 64].iter().enumerate() {
                let pixels = (side * side / 2) as usize;
                let pal = &blob[at..at + 64];
                let texels = &blob[at + 64..at + 64 + pixels];
                let row = (*side / 2) as usize;
                let sw = swizzle(texels, row, *side as usize);
                let bufw = *side;
                let mut mem = pal.to_vec();
                mem.extend_from_slice(&sw);
                out.push(Obs {
                    kind: "dat", label: format!("{label}.{b}"), w: *side, h: *side, bpp: 4,
                    clut: clut_from(&mem),
                    hashes: vec![("ram", hash_of(&sw, bufw, *side, *side, 4)), ("lin", hash_of(texels, bufw, *side, *side, 4))],
                    pal: pal.to_vec(), ram: Vec::new(), reach: mem.len(), mem: mem.iter().take(2048).copied().collect(), own: mem.len(),
                    clutv: vec![("swz", clut_from(&mem))],
                });
                at += 64 + pixels;
            }
        } else if fnt::looks_like_font(blob) {
            if let Ok(m) = fnt::Metrics::parse(blob) {
                if m.has_embedded_atlas(blob) {
                    let a = &blob[m.atlas_at..];
                    if a.len() > 64 {
                        let (w, h, bpp) = (u32::from(m.order.u16(a, 0)), u32::from(m.order.u16(a, 2)), u32::from(a[4]));
                        let clut_size = m.order.u32(a, 8) as usize;
                        let texels = &a[64 + clut_size..];
                        out.push(Obs {
                            kind: "fnt", label, w, h, bpp,
                            clut: clut_from(&a[64..]),
                            hashes: vec![("ram", hash_of(texels, w, w, h, bpp))],
                            pal: a[64..64 + clut_size].to_vec(),
                            ram: Vec::new(),
                            reach: a.len() - 64,
                            mem: a[64..].iter().take(2048).copied().collect(),
                            own: a.len() - 64,
                            clutv: vec![("swz", clut_from(&a[64..])), ("pal", replacement::clut_hash(&a[64..64 + clut_size]))],
                        });
                    }
                }
            }
        }
    });
    out
}

#[test]
#[ignore = "scratch sweep"]
fn sweep() {
    for (image, pack) in [("pulse-psp-eu.chd", "WipEout-Pulse-Upscale-Version"), ("pure-psp-eu.chd", "WipEout-Pure-HD-Texture-Pack")] {
        let Some(dir) = pack_dir(pack) else { continue };
        let ini = Pack::parse(&std::fs::read_to_string(dir.join("textures.ini")).unwrap());
        let obs = observe(image);
        let mut full: HashSet<Key> = HashSet::new();
        let mut data: HashMap<u32, Vec<&Obs>> = HashMap::new();
        for o in &obs {
            let (v, _) = match o.kind { "vex" => ("ram", 0), "mip" => ("A", 0), _ => ("ram", 0) };
            let h = o.hashes.iter().find(|(n, _)| *n == v).unwrap().1;
            let k = Key::with_palette(replacement::dim(o.w, o.h), o.clut, 0xC500FF03, h);
            full.insert(k);
            data.entry(h).or_default().push(o);
        }
        let mut per: BTreeMap<String, [usize; 4]> = BTreeMap::new();
        let mut miss_samples: Vec<String> = Vec::new();
        for l in &ini.hashes {
            let Some(k) = l.key() else { continue };
            if l.level != 0 { continue; }
            let top: Vec<&str> = l.target.split('/').collect();
            let d = if l.target.is_empty() { "(blank)".to_string() } else if top.len() > 3 && top[1] == "ships" { format!("{}/{}/{}", top[0], top[1], top[3]) } else if top.len() > 2 && top[0] == "in_game" { format!("{}/{}", top[0], top[1]) } else { top[0].to_string() };
            let e = per.entry(d).or_default();
            e[0] += 1;
            if full.contains(&k) { e[1] += 1; }
            else if let Some(os) = data.get(&k.data) { e[2] += 1; if miss_samples.len() < 25 { let o = os[0]; miss_samples.push(format!("{} want {:08x} got {:08x} ({} {}x{} {}bpp) {}", k, k.dim_clut, replacement::dim(o.w,o.h) ^ o.clut ^ 0xC500FF03, o.kind, o.w, o.h, o.bpp, l.target)); } }
            else { e[3] += 1; }
        }
        println!("== {image}: textures observed {} distinct keys {}", obs.len(), full.len());
        let mut tot = [0usize; 4];
        for (d, e) in &per { println!("  {d:28} keys {:5} full {:5} data-only {:4} none {:4}", e[0], e[1], e[2], e[3]); for i in 0..4 { tot[i] += e[i]; } }
        println!("  TOTAL keys {} full {} data-only {} none {}", tot[0], tot[1], tot[2], tot[3]);
        for m in &miss_samples { println!("  dataonly: {m}"); }
    }
}


#[test]
#[ignore = "scratch sweep"]
fn other_blobs() {
    for image in ["pulse-psp-eu.chd", "pure-psp-eu.chd"] {
        let path = oag_testdata::image(image).expect("image");
        let mut disc = DiscImage::open(&path).expect("open");
        let mut tally: BTreeMap<(String, String), (usize, usize)> = BTreeMap::new();
        walk(&mut disc, |arch, _idx, blob| {
            let kind = if oag_vex::vex::has_magic(blob) { "vex".to_string() }
                else if texture::Texture::looks_like_texture(blob) { "mip".to_string() }
                else if fnt::looks_like_font(blob) { "fnt".to_string() }
                else { format!("{:02x}{:02x}{:02x}{:02x}", blob[0], blob.get(1).copied().unwrap_or(0), blob.get(2).copied().unwrap_or(0), blob.get(3).copied().unwrap_or(0)) };
            let e = tally.entry((arch.rsplit('/').next().unwrap().to_string(), kind)).or_default();
            e.0 += 1; e.1 += blob.len();
        });
        println!("== {image}");
        let mut v: Vec<_> = tally.iter().collect();
        v.sort_by_key(|(_, (_, b))| std::cmp::Reverse(*b));
        for ((a, k), (n, b)) in v.iter().take(30) { println!("  {a:12} {k:10} n={n:5} bytes={b}"); }
    }
}

#[test]
#[ignore = "scratch sweep"]
fn heads() {
    let path = oag_testdata::image("pure-psp-eu.chd").expect("image");
    let mut disc = DiscImage::open(&path).expect("open");
    let mut seen = 0;
    walk(&mut disc, |arch, idx, blob| {
        if blob.starts_with(&[0, 0, 2, 0]) && seen < 3 {
            seen += 1;
            println!("{arch}#{idx} len {} head {:02x?}", blob.len(), &blob[..96.min(blob.len())]);
            let s: String = blob.iter().take(4000).filter(|b| b.is_ascii_graphic()).map(|b| *b as char).collect();
            println!("  ascii: {}", &s[..s.len().min(300)]);
        }
    });
}

#[test]
#[ignore = "scratch sweep"]
fn oracle() {
    let dump = std::env::var("OAG_PPSSPP_DUMP").expect("dump dir");
    let obs = observe("pulse-psp-eu.chd");
    let dir = pack_dir("WipEout-Pulse-Upscale-Version").unwrap();
    let ini = Pack::parse(&std::fs::read_to_string(dir.join("textures.ini")).unwrap());
    let packkeys: HashSet<Key> = ini.hashes.iter().filter(|l| l.level == 0).filter_map(|l| l.key()).collect();
    let mut full: HashMap<Key, &Obs> = HashMap::new();
    let mut data: HashMap<u32, Vec<&Obs>> = HashMap::new();
    for o in &obs {
        let h = o.hashes[0].1;
        full.insert(Key::with_palette(replacement::dim(o.w, o.h), o.clut, 0xC500FF03, h), o);
        data.entry(h).or_default().push(o);
    }
    let (mut exact, mut data_only, mut none, mut in_pack, mut nonclut) = (0, 0, 0, 0, 0);
    let mut samples: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut total = 0;
    let mut own_tally: BTreeMap<(u32, bool, bool), usize> = BTreeMap::new();
    let mut resid: BTreeMap<(u32, usize, u32), usize> = BTreeMap::new();
    let mut variants: BTreeMap<&str, usize> = BTreeMap::new();
    for e in std::fs::read_dir(&dump).unwrap() {
        let name = e.unwrap().file_name().to_string_lossy().to_string();
        let Some(k) = Key::parse(name.trim_end_matches(".png")) else { continue };
        total += 1;
        if packkeys.contains(&k) { in_pack += 1; }
        if k.dim_clut < 0x10000 { nonclut += 1; }
        let ob = data.get(&k.data).map(|v| v[0]);
        if let Some(o) = ob { let key = (o.bpp, o.own >= 1024, full.contains_key(&k)); *own_tally.entry(key).or_default() += 1; }
        if full.contains_key(&k) { exact += 1; }
        else if let Some(os) = data.get(&k.data) {
            data_only += 1;
            let o = os[0];
            let d = replacement::dim(o.w, o.h);
            let mut hit = "no-variant"; let _ = d;
            for oo in os { let d = replacement::dim(oo.w, oo.h); for (vn, vh) in &oo.clutv { if k.dim_clut == d ^ vh ^ 0xC500FF03 { hit = vn; } } }
            *variants.entry(hit).or_default() += 1;
            if hit == "no-variant" {
                for oo in os {
                    let d = replacement::dim(oo.w, oo.h);
                    for n in (4..=2048usize).step_by(4) {
                        if n > oo.mem.len() { break; }
                        let r = k.dim_clut ^ d ^ replacement::clut_hash(&oo.mem[..n]);
                        if r >> 24 == 0xC5 { *resid.entry((r, n, oo.bpp)).or_default() += 1; if r == 0xC500050A { if oo.label.ends_with("#655.6") { println!("   PAL {:02x?}", &oo.pal); } } }
                    }
                }
            }
            if hit == "no-variant" && o.reach >= 1024 && o.bpp == 4 { samples.entry("noreach-miss").or_default().push(format!("{k} {} {}x{} reach {} {}", o.kind, o.w, o.h, o.reach, o.label)); }
            samples.entry("data-only").or_default().push(format!("{k} ours {:08x} {} {}x{} {}bpp reach {}", replacement::dim(o.w,o.h) ^ o.clut ^ 0xC500FF03, o.kind, o.w, o.h, o.bpp, o.reach));
        } else {
            none += 1;
            let target = ini.hashes.iter().find(|l| l.key() == Some(k)).map(|l| l.target.clone()).unwrap_or_default();
            samples.entry("none").or_default().push(format!("{k} {target}"));
        }
    }
    println!("variants {variants:?}");
    println!("own_tally (bpp, own>=1024, exact) {own_tally:?}");
    for ((r, n, b), c) in &resid { println!("  resid {r:08x} n={n} bpp={b} x{c}"); }
    println!("dumped {total}: exact {exact} data-only {data_only} none {none}; in pack {in_pack}; non-clut {nonclut}");
    for (k, v) in &samples { println!("{k}: {}", v.len()); for s in v.iter().take(if k == &"none" { 40 } else { 12 }) { println!("   {s}"); } }
}

#[test]
#[ignore = "scratch sweep"]
fn pngs() {
    for image in ["pulse-psp-eu.chd", "pure-psp-eu.chd"] {
        let path = oag_testdata::image(image).expect("image");
        let mut disc = DiscImage::open(&path).expect("open");
        println!("== {image}");
        walk(&mut disc, |arch, idx, blob| {
            if blob.starts_with(&[0x89, b'P', b'N', b'G']) {
                let w = u32::from_be_bytes(blob[16..20].try_into().unwrap());
                let h = u32::from_be_bytes(blob[20..24].try_into().unwrap());
                println!("  {arch}#{idx} {w}x{h} depth {} colortype {} len {}", blob[24], blob[25], blob.len());
            }
        });
    }
}

fn rgba_id(w: u32, h: u32, rgba: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut s = std::collections::hash_map::DefaultHasher::new();
    (w, h).hash(&mut s);
    rgba.hash(&mut s);
    s.finish()
}

#[test]
#[ignore = "scratch sweep"]
fn decoded_match() {
    let rgba_dir = std::env::var("OAG_RGBA_DIR").expect("rgba dir");
    let path = oag_testdata::image("pulse-psp-eu.chd").expect("image");
    let mut disc = DiscImage::open(&path).expect("open");
    let mut ours: HashMap<u64, String> = HashMap::new();
    walk(&mut disc, |arch, idx, blob| {
        let label = format!("{arch}#{idx}");
        if oag_vex::vex::has_magic(blob) {
            if let Ok(ts) = oag_vex::vex::textures(blob) {
                for (n, t) in ts.iter().enumerate() {
                    if let Some(t) = t { ours.entry(rgba_id(u32::from(t.width), u32::from(t.height), &t.to_rgba())).or_insert(format!("{label}.{n} vex")); }
                }
            }
        } else if let Ok(t) = texture::Texture::parse(blob) {
            ours.entry(rgba_id(u32::from(t.width), u32::from(t.height), &t.to_rgba())).or_insert(format!("{label} mip"));
        } else if fnt::looks_like_font(blob) {
            if let Ok(f) = fnt::Font::parse(blob) { ours.entry(rgba_id(u32::from(f.width), u32::from(f.height), &f.to_rgba())).or_insert(format!("{label} fnt")); }
        } else if blob.len() == oag_texture::ship_skin::FILE_LEN {
            if let Ok(sk) = oag_texture::ship_skin::parse(blob) {
                for (b, bl) in sk.blocks.iter().enumerate() {
                    let mut rgba = Vec::new();
                    for &i in &bl.indices { rgba.extend_from_slice(&bl.palette[i as usize]); }
                    ours.entry(rgba_id(bl.width as u32, bl.height as u32, &rgba)).or_insert(format!("{label}.{b} dat"));
                }
            }
        }
    });
    let obs = observe("pulse-psp-eu.chd");
    let mut full: HashSet<Key> = HashSet::new();
    let mut data: HashSet<u32> = HashSet::new();
    for o in &obs { let h = o.hashes[0].1; full.insert(Key::with_palette(replacement::dim(o.w, o.h), o.clut, 0xC500FF03, h)); data.insert(h); }
    let mut cross: BTreeMap<(bool, bool, bool), usize> = BTreeMap::new();
    let mut samples: Vec<String> = Vec::new();
    for e in std::fs::read_dir(&rgba_dir).unwrap() {
        let p = e.unwrap().path();
        let stem = p.file_stem().unwrap().to_string_lossy().to_string();
        let Some(k) = Key::parse(&stem) else { continue };
        let raw = std::fs::read(&p).unwrap();
        let (w, h) = (u32::from_le_bytes(raw[0..4].try_into().unwrap()), u32::from_le_bytes(raw[4..8].try_into().unwrap()));
        let id = rgba_id(w, h, &raw[8..]);
        let dec = ours.get(&id);
        let c = (full.contains(&k), data.contains(&k.data), dec.is_some());
        *cross.entry(c).or_default() += 1;
        if !c.0 && !c.1 && samples.len() < 40 { samples.push(format!("{k} {w}x{h} decoded-equal: {dec:?}")); }
    }
    println!("(exact, data-hit, decoded-equal) {cross:?}");
    for s in &samples { println!("  {s}"); }
}

#[test]
#[ignore = "scratch sweep"]
fn hull_probe() {
    let path = oag_testdata::image("pulse-psp-eu.chd").expect("image");
    let mut disc = DiscImage::open(&path).expect("open");
    walk(&mut disc, |arch, idx, blob| {
        if !arch.ends_with("Data.wad") || idx != 627 { return; }
        let tree = oag_vex::vex::nodes(blob).unwrap();
        let st = oag_vex::vex::stored_textures(blob, &tree).unwrap();
        for (n, t) in st.iter().enumerate() {
            let Some(t) = t else { continue };
            let (w, h, bpp) = (u32::from(t.width), u32::from(t.height), u32::from(t.bits_per_pixel));
            let stride = (w * bpp / 8) as usize;
            let l0 = &t.texels[..stride * h as usize];
            let sw = swizzle(l0, stride, h as usize);
            let un = oag_formats::swizzle::unswizzle(l0, stride, h as usize);
            let hs: Vec<String> = [("stored", l0), ("swz", &sw[..]), ("unswz", &un[..])].iter().map(|(n, b)| format!("{n}={:08x}", hash_of(b, w, w, h, bpp))).collect();
            println!("  tex {n} {w}x{h} {bpp}bpp flags {:02x} mips {} texels {} :: {}", t.flags, t.mip_count, t.texels.len(), hs.join(" "));
        }
    });
}
