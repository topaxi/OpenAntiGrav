use std::collections::HashMap;

use super::*;
use crate::png::encode_rgba;

struct Memory(HashMap<String, Vec<u8>>);

impl Files for Memory {
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.0.get(name).cloned()
    }
}

const PALETTE: [u8; 1024] = [7; 1024];
const TEXELS: [u8; 256] = [3; 256];

fn stored<'a>(palette: &'a [u8], texels: &'a [u8]) -> Stored<'a> {
    Stored {
        width: 16,
        height: 16,
        bits_per_pixel: 8,
        mip_levels: 1,
        swizzled: false,
        palette,
        texels,
        pad_level0: true,
    }
}

fn files(names: &[&str]) -> Box<dyn Files> {
    // A 2x2 picture of one grey per file, so which file was read shows in the pixels.
    Box::new(Memory(
        names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let grey = 40 * (i as u8 + 1);
                let rgba: Vec<u8> = (0..4).flat_map(|_| [grey, grey, grey, 255]).collect();
                ((*name).to_string(), encode_rgba(2, 2, &rgba))
            })
            .collect(),
    ))
}

fn ini(lines: &[String]) -> String {
    format!(
        "[options]\nhash = xxh64\nignoreAddress = true\n[games]\nUCES00465 = textures.ini\n[hashes]\n{}\n",
        lines.join("\n")
    )
}

#[test]
fn a_full_key_finds_its_png_and_the_picture_is_read_once_it_is_asked_for() {
    let texture = stored(&PALETTE, &TEXELS);
    let key = texture.replacement().unwrap().key.unwrap();
    let pack =
        Replacements::new("t", &ini(&[format!("{key} = a.png")]), files(&["a.png"])).unwrap();
    assert_eq!(pack.replaced(), 0);
    let image = pack.find(&texture).expect("the key names a.png");
    assert_eq!((image.width, image.height), (2, 2));
    assert_eq!(&image.rgba[..4], &[40, 40, 40, 255]);
    assert_eq!(pack.replaced(), 1);
}

#[test]
fn lookup_runs_full_key_then_cache_key_then_data_alone_like_ppsspp() {
    let texture = stored(&PALETTE, &TEXELS);
    let found = texture.replacement().unwrap();
    let key = found.key.unwrap();
    let full = format!("{key}");
    let cache_only = format!("{:016x}", u64::from(key.dim_clut));
    let data_only = format!("{:016x}{:08x}", 0, found.data);
    let grey = |lines: &[&String]| {
        let lines: Vec<String> = lines
            .iter()
            .zip(["a.png", "b.png", "c.png"])
            .map(|(k, v)| format!("{k} = {v}"))
            .collect();
        let pack =
            Replacements::new("t", &ini(&lines), files(&["a.png", "b.png", "c.png"])).unwrap();
        pack.find(&texture).map(|image| image.rgba[0])
    };
    assert_eq!(grey(&[&full, &cache_only, &data_only]), Some(40));
    assert_eq!(
        grey(&[&cache_only, &data_only]),
        Some(40),
        "a.png is the first line left"
    );
    assert_eq!(grey(&[&data_only]), Some(40));
    assert_eq!(grey(&[]), None);
}

#[test]
fn a_blank_line_leaves_the_texture_alone_even_when_a_later_line_would_match() {
    let texture = stored(&PALETTE, &TEXELS);
    let found = texture.replacement().unwrap();
    let key = found.key.unwrap();
    let pack = Replacements::new(
        "t",
        &ini(&[
            format!("{key} ="),
            format!("{:016x}{:08x} = a.png", 0, found.data),
        ]),
        files(&["a.png"]),
    )
    .unwrap();
    assert!(pack.find(&texture).is_none());
}

#[test]
fn a_texture_whose_palette_hash_reads_the_heap_is_found_by_data_hash_alone() {
    // 4-bit, 64 palette bytes and 256 texel bytes: under the 1,024-byte palette window.
    let palette = [9u8; 64];
    let texels = [5u8; 256];
    let texture = Stored {
        bits_per_pixel: 4,
        ..stored(&palette, &texels)
    };
    let found = texture.replacement().unwrap();
    assert!(found.key.is_none());
    let by_data = format!("{:016x}{:08x} = a.png", 0, found.data);
    let by_cache = format!("{:016x}{:08x} = a.png", found.dim, found.data);
    let pack =
        |line: &str| Replacements::new("t", &ini(&[line.to_string()]), files(&["a.png"])).unwrap();
    assert!(pack(&by_data).find(&texture).is_some());
    assert!(
        pack(&by_cache).find(&texture).is_none(),
        "no cache key can name it"
    );
}

#[test]
fn a_png_the_pack_does_not_ship_replaces_nothing() {
    let texture = stored(&PALETTE, &TEXELS);
    let key = texture.replacement().unwrap().key.unwrap();
    let pack = Replacements::new("t", &ini(&[format!("{key} = gone.png")]), files(&[])).unwrap();
    assert!(pack.find(&texture).is_none());
    assert_eq!(pack.replaced(), 0);
}

#[test]
fn a_pack_that_hashes_another_way_is_refused_by_name() {
    let none = || files(&[]);
    let text =
        |options: &str, tail: &str| format!("[options]\n{options}\n[games]\n[hashes]\n{tail}");
    assert_eq!(
        Replacements::new("t", &text("hash = quick\nignoreAddress = true", ""), none())
            .unwrap_err(),
        Refused::Hash("quick".to_string())
    );
    assert_eq!(
        Replacements::new("t", &text("ignoreAddress = true", ""), none()).unwrap_err(),
        Refused::Hash("quick".to_string()),
        "no hash line is PPSSPP's quick hash"
    );
    assert_eq!(
        Replacements::new("t", &text("hash = xxh64", ""), none()).unwrap_err(),
        Refused::Address
    );
    assert_eq!(
        Replacements::new(
            "t",
            &text("hash = xxh64\nignoreAddress = true", "[hashranges]\n1 = 2"),
            none()
        )
        .unwrap_err(),
        Refused::Section("hashranges".to_string())
    );
}

#[test]
fn a_pack_serves_the_serials_it_names_in_either_spelling() {
    let pack = Replacements::new("t", &ini(&[]), files(&[])).unwrap();
    assert!(pack.serves("UCES00465") && pack.serves("UCES-00465") && pack.serves("uces-00465"));
    assert!(!pack.serves("UCUS-98712"));
}

#[test]
fn the_active_pack_is_what_find_asks_and_none_asks_nobody() {
    let texture = stored(&PALETTE, &TEXELS);
    let key = texture.replacement().unwrap().key.unwrap();
    let pack = Arc::new(
        Replacements::new("t", &ini(&[format!("{key} = a.png")]), files(&["a.png"])).unwrap(),
    );
    install(Some(pack));
    assert!(find(&texture).is_some());
    install(None);
    assert!(find(&texture).is_none());
    assert!(active().is_none());
}
