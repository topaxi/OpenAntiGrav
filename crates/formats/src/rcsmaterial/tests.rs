use super::*;

/// The hash is the one the rest of the format uses, checked against names whose
/// preimages are recorded elsewhere in the tree.
#[test]
fn the_name_hash_reproduces_names_recovered_by_other_routes() {
    // From docs/ghidra/functions/ps3-hdfury-eu/renderer.md.
    assert_eq!(name_hash("fogColour"), 0x3dc3_1258);
    assert_eq!(name_hash("diffuse"), 0x515e_298e);
    assert_eq!(name_hash("viewProj"), 0x2e7d_5f33);
    assert_eq!(name_hash("constantAmbientColour"), 0x81db_67ea);
    // The attribute no `.rcsmodel` declares, because the SPU writes it.
    assert_eq!(name_hash("SpuVertexColours"), 0x868f_8229);
}

/// The four class names, against the words the records actually carry.
///
/// `0x7893d2ec` and `0xd29c9ee2` are `track_surface.rcsmaterial` records #0 and
/// #35 - read off the file before the preimage was known, which is what makes
/// this a check rather than a restatement.
#[test]
fn every_vertex_class_hashes_to_the_word_the_records_carry() {
    assert_eq!(Class::Static.hash(), 0x7893_d2ec);
    assert_eq!(Class::StaticQuake.hash(), 0xd29c_9ee2);
    assert_eq!(Class::RigidBody.hash(), 0xdd70_bfd5);
    assert_eq!(Class::StaticUncompressed.hash(), 0xa9ed_fe7e);
    for class in Class::ALL {
        assert_eq!(Class::from_hash(class.hash()), Some(class));
    }
    assert_eq!(Class::from_hash(0), None, "an unknown class is reported");
}

/// The canonical order, against permutation names the executable itself ships.
///
/// `HalfBrightAmbientSunSpot0SVC0` is a literal at `EBOOT.elf 0x7a17d0` and is
/// variant #5 of `track_surface.rcsmaterial`, whose `[1]` is `0xfb61d927`. If
/// [`TOKENS`] were reordered, the concatenation would change and this would
/// fail - which is the point, because the order is derived rather than stated
/// anywhere in the data.
#[test]
fn the_token_order_rebuilds_the_permutation_names_the_disc_ships() {
    let set = Features::NONE
        .with(Features::token("HalfBright"))
        .with(Features::token("Ambient"))
        .with(Features::token("Sun"))
        .with(Features::token("Spot0"))
        .with(Features::token("SVC0"));
    assert_eq!(set.name(), "HalfBrightAmbientSunSpot0SVC0");
    assert_eq!(set.hash(), 0xfb61_d927);

    // Two more from the same file's table, exercising a different group each.
    let zone = Features::NONE
        .with(Features::token("HalfBright"))
        .with(Features::token("ZoneMode"))
        .with(Features::token("IleLightmap"))
        .with(Features::token("Sun"))
        .with(Features::token("Spot0"))
        .with(Features::token("SVC0"));
    assert_eq!(zone.name(), "HalfBrightZoneModeIleLightmapSunSpot0SVC0");

    let shadow = Features::NONE
        .with(Features::token("ShadowToAlpha"))
        .with(Features::token("Ambient"))
        .with(Features::token("Spot0"))
        .with(Features::token("SVC1"));
    assert_eq!(shadow.name(), "ShadowToAlphaAmbientSpot0SVC1");

    // The four standalone permutations are a single token each.
    for name in [
        "Ambient",
        "ZAlphaOnly",
        "AmbientShadow",
        "SunOcclusionLightmap",
        "SunOcclusionVertex",
    ] {
        assert_eq!(Features::token(name).name(), name);
    }
}

/// A name that is not a token is not silently accepted as one.
#[test]
fn an_unknown_token_name_is_the_empty_set_rather_than_a_bit() {
    assert_eq!(Features::token("NotAToken"), Features::NONE);
    assert_eq!(
        Features::token("halfbright"),
        Features::NONE,
        "case matters"
    );
    assert!(!Features::NONE.has("HalfBright"));
    assert!(Features::token("HalfBright").has("HalfBright"));
}

/// The three tokens the executable declares and no shipped permutation uses.
///
/// Carried so that [`TOKENS`] is the executable's table rather than a subset of
/// what happens to ship - if a later reading finds a permutation using one, it
/// will hash correctly instead of silently dropping the token.
#[test]
fn the_unused_tokens_are_still_addressable() {
    for name in ["Spot3", "FalseLight", "NoAlbedo"] {
        assert!(
            Features::token(name).has(name),
            "{name} is in the executable's table and must stay addressable"
        );
    }
}

/// A synthetic table, framed the way the disc frames one.
fn synthetic() -> Vec<u8> {
    let mut out = vec![0u8; 0x10];
    out[0..4].copy_from_slice(&2u32.to_be_bytes()); // two variants
    out[4..8].copy_from_slice(&0x10u32.to_be_bytes()); // table at +0x10
    let features = Features::token("Ambient");
    for (class, vp, fp) in [
        (Class::Static, 0x1000u32, 0x2000u32),
        (Class::StaticQuake, 0x3000, 0x2000),
    ] {
        let mut r = [0u32; 16];
        r[0] = class.hash();
        r[1] = features.hash();
        r[4] = vp;
        r[5] = fp;
        r[6] = 0x200;
        r[7] = 0x100;
        r[8] = 0xaaaa_0000;
        r[9] = 0xbbbb_0000;
        for w in r {
            out.extend_from_slice(&w.to_be_bytes());
        }
    }
    out
}

#[test]
fn a_variant_table_parses_and_the_key_selects_a_row() {
    let data = synthetic();
    let mat = RcsMaterial::parse(&data).expect("the table parses");
    assert_eq!(mat.variants.len(), 2);

    let features = Features::token("Ambient");
    let found = mat
        .variant(Class::StaticQuake, features)
        .expect("the key selects its row");
    assert_eq!(found.vertex.offset, 0x3000);
    assert_eq!(found.fragment.offset, 0x2000);
    assert_eq!(found.vertex.len, 0x200);
    assert_eq!(found.fragment.program_hash, 0xbbbb_0000);
    assert_eq!(found.class, Some(Class::StaticQuake));

    // The two variants share a fragment program, which is why the content hash
    // exists at all.
    assert_eq!(
        mat.variants[0].fragment.program_hash,
        mat.variants[1].fragment.program_hash
    );
}

/// A permutation the file does not ship is a miss, not a wrong row.
#[test]
fn a_key_the_file_does_not_carry_answers_none() {
    let data = synthetic();
    let mat = RcsMaterial::parse(&data).expect("the table parses");
    assert!(
        mat.variant(Class::RigidBody, Features::token("Ambient"))
            .is_none(),
        "a class the file does not carry"
    );
    assert!(
        mat.variant(Class::Static, Features::token("ZAlphaOnly"))
            .is_none(),
        "a permutation the file does not carry"
    );
}

#[test]
fn a_truncated_file_is_an_error_rather_than_a_panic() {
    assert_eq!(
        RcsMaterial::parse(&[0u8; 4]),
        Err(Error::TooShort { got: 4 })
    );
    let mut data = synthetic();
    data.truncate(0x40);
    assert!(
        matches!(RcsMaterial::parse(&data), Err(Error::OutOfBounds { .. })),
        "a table that runs past the file is reported"
    );
}

/// The last record's four trailing zero words are elided on every file on the
/// disc, so the bounds check has to allow the table to end `0x10` short.
#[test]
fn the_elided_trailing_words_of_the_last_record_are_not_an_error() {
    let mut data = synthetic();
    data.truncate(data.len() - 0x10);
    let mat = RcsMaterial::parse(&data).expect("the elision is not out of bounds");
    assert_eq!(mat.variants.len(), 2);
    assert_eq!(mat.variants[1].class, Some(Class::StaticQuake));
    assert_eq!(
        mat.variants[1].fragment.program_hash, 0xbbbb_0000,
        "the fields before the elision still read"
    );
}

/// The flag-bit order is the executable's table, and it is *not* the order the
/// names concatenate in.
///
/// The second half is the load-bearing part. If a builder walked
/// [`FLAG_BITS`] in order, variant #5 of `track_surface.rcsmaterial` would be
/// spelled `HalfBrightSunSVC0AmbientSpot0` - and the executable ships the
/// literal `HalfBrightAmbientSunSpot0SVC0`. That mismatch is why [`TOKENS`]
/// has its own order, derived from the 143 shipped names rather than copied
/// from this table.
#[test]
fn the_flag_bit_order_is_not_the_concatenation_order() {
    // Bit 3 is an empty string in the table: a flag naming no token.
    assert_eq!(FLAG_BITS[3], None);
    assert_eq!(FLAG_BITS[0], Some("ShadowToAlpha"));
    assert_eq!(FLAG_BITS[20], Some("AmbientShadow"));

    let bits = (1 << 1) | (1 << 2) | (1 << 12) | (1 << 15) | (1 << 10);
    let set = Features::from_flags(bits);
    assert_eq!(set.name(), "HalfBrightAmbientSunSpot0SVC0");
    assert_eq!(set.hash(), 0xfb61_d927);

    // What walking the table in order would have produced instead.
    let table_order: String = FLAG_BITS
        .iter()
        .enumerate()
        .filter_map(|(bit, t)| (bits & (1 << bit) != 0).then_some(*t).flatten())
        .collect();
    assert_eq!(table_order, "HalfBrightSunSVC0AmbientSpot0");
    assert_ne!(table_order, set.name(), "the two orders must differ");
}

/// A flags word carrying a bit this reading cannot name is ignored, not an
/// error - the layout is read and the per-pass assignment is not.
#[test]
fn an_unnamed_flag_bit_is_ignored_rather_than_rejected() {
    assert_eq!(Features::from_flags(1 << 3), Features::NONE);
    assert_eq!(Features::from_flags(1 << 31), Features::NONE);
    assert_eq!(
        Features::from_flags((1 << 12) | (1 << 3)),
        Features::token("Ambient"),
        "an unnamed bit does not disturb the named ones"
    );
}
