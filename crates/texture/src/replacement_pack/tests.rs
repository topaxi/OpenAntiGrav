use super::*;

const SAMPLE: &str = "\
[options]
version = 1
hash = xxh64
ignoreAddress = true

[games]
UCES00465 = textures.ini

[hashes]
# a comment
000000007d9d618f1a5be898 = intro/sce/scee_presents.png
0000000000000909 =
0000000000000000cd0aa2c1_1 =
000000000000000023fbcaba_2 = x.png

[hashranges]
0,1,2 = 3,4
";

#[test]
fn reads_the_sections_a_pack_uses() {
    let pack = Pack::parse(SAMPLE);
    assert_eq!(pack.hash_kind().as_deref(), Some("xxh64"));
    assert!(pack.flag("ignoreaddress"));
    assert!(!pack.flag("ignoremipmap"));
    assert_eq!(
        pack.games,
        [("UCES00465".to_string(), "textures.ini".to_string())]
    );
    assert_eq!(pack.hashes.len(), 4);
    assert_eq!(pack.other_sections.get("hashranges"), Some(&1));
    assert!(pack.unparsed.is_empty());
}

#[test]
fn a_key_is_cache_key_hash_and_level() {
    let pack = Pack::parse(SAMPLE);
    let full = &pack.hashes[0];
    assert_eq!(full.cache_key, 0x7d9d_618f);
    assert_eq!(full.data_hash, Some(0x1a5b_e898));
    assert_eq!(
        full.key().map(|k| k.to_string()).as_deref(),
        Some("000000007d9d618f1a5be898")
    );
    assert_eq!(full.target, "intro/sce/scee_presents.png");

    let wildcard = &pack.hashes[1];
    assert_eq!((wildcard.cache_key, wildcard.data_hash), (0x909, None));
    assert!(wildcard.is_blacklist());
    assert!(wildcard.key().is_none());

    let level = &pack.hashes[3];
    assert_eq!((level.data_hash, level.level), (Some(0x23fb_caba), 2));
}

#[test]
fn a_key_that_is_not_hex_is_listed_not_guessed() {
    let pack = Pack::parse("[hashes]\nnot-a-key = a.png\n");
    assert!(pack.hashes.is_empty());
    assert_eq!(pack.unparsed, ["not-a-key = a.png"]);
}
