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

/// The permutation word decodes to the name the executable ships as a literal.
///
/// `HalfBrightAmbientSunSpot0SVC0` sits at `EBOOT.elf 0x7b17d0` and is variant
/// #5 of `track_surface.rcsmaterial`, whose `[1]` is `0xfb61d927`. The word is
/// `1` - `Sun` set, and every other pass-decided token the *clear* side of its
/// bit or the zero of its field.
#[test]
fn the_lit_race_pass_word_is_the_name_the_executable_ships() {
    let set = Features::from_pass_word(LIT_RACE_PASS);
    assert_eq!(set.name(), "HalfBrightAmbientSunSpot0SVC0");
    assert_eq!(set.hash(), 0xfb61_d927);

    // The chunk half rides in bits 1-2, which is the same field `for_chunk`
    // answers by name.
    assert_eq!(
        Features::from_pass_word(LIT_RACE_PASS | 0b100).name(),
        "HalfBrightIleLightmapSunSpot0SVC0"
    );
    assert_eq!(
        Features::from_pass_word(LIT_RACE_PASS | 0b010).name(),
        "HalfBrightIleVertexSunSpot0SVC0"
    );
}

/// Every field of the word, exercised one at a time.
#[test]
fn each_field_of_the_permutation_word_names_what_it_should() {
    assert!(Features::from_pass_word(1 << 5).has("ShadowToAlpha"));
    assert!(!Features::from_pass_word(1 << 5).has("HalfBright"));
    assert!(Features::from_pass_word(0).has("HalfBright"), "bit 5 clear");
    assert!(Features::from_pass_word(0).has("SVC0"), "bit 11 clear");
    assert!(Features::from_pass_word(1 << 11).has("SVC1"));
    for (bit, name) in [
        (6, "ShadowMap"),
        (7, "FalseLight"),
        (8, "ZoneMode"),
        (9, "ZoneTrans"),
        (10, "NoAlbedo"),
    ] {
        assert!(Features::from_pass_word(1 << bit).has(name), "bit {bit}");
    }
    for (n, name) in ["Spot0", "Spot1", "Spot2", "Spot3"].into_iter().enumerate() {
        assert!(Features::from_pass_word((n as u32) << 3).has(name));
    }
    for (n, name) in ["Ambient", "IleVertex", "IleLightmap", "IBL"]
        .into_iter()
        .enumerate()
    {
        assert!(Features::from_pass_word((n as u32) << 1).has(name));
    }
    // Above the word's width is a different quantity, and is ignored.
    assert_eq!(
        Features::from_pass_word(1 << 12).name(),
        Features::from_pass_word(0).name()
    );
}

/// **Sweeping all 4096 words produces 4096 distinct hashes.**
///
/// No collisions is what makes `table[word]` a lookup rather than a lottery,
/// and it is the check that would catch a mis-read field width - overlapping
/// fields would collapse two words onto one name.
#[test]
fn the_whole_permutation_word_space_hashes_without_collision() {
    let mut seen = std::collections::BTreeSet::new();
    for word in 0..(1u32 << PASS_WORD_BITS) {
        seen.insert(Features::from_pass_word(word).hash());
    }
    assert_eq!(seen.len(), 1 << PASS_WORD_BITS, "two words share a hash");
}

/// The chunk half is a **field**, so composing it as a set is wrong.
///
/// The trap this exists for, caught in the renderer wiring: `LIT_RACE_PASS`
/// already carries `Ambient` (bits 1-2 at zero), so `with`-ing a lightmapped
/// chunk's `IleLightmap` onto it leaves **both** set and names a permutation no
/// material ships. On Talon's Junction that dropped variant resolution from 913
/// chunks to 289. `chunk_word` replaces the field instead.
#[test]
fn the_chunk_half_replaces_a_field_rather_than_joining_a_set() {
    // The union that looks right and is not.
    let wrong = Features::from_pass_word(LIT_RACE_PASS).with(Features::token("IleLightmap"));
    assert_eq!(wrong.name(), "HalfBrightAmbientIleLightmapSunSpot0SVC0");

    // The word that is right: bits 1-2 replaced, everything else kept.
    let right = Features::from_pass_word(Features::chunk_word(LIT_RACE_PASS, None) | 0b100);
    assert_eq!(right.name(), "HalfBrightIleLightmapSunSpot0SVC0");
    assert!(!right.has("Ambient"), "the field was replaced, not joined");

    // A base with other pass bits set keeps them.
    let zoned = Features::chunk_word(LIT_RACE_PASS | (1 << 8), None);
    assert!(Features::from_pass_word(zoned).has("ZoneMode"));
    assert!(Features::from_pass_word(zoned).has("Sun"));
}

/// A `SHO` block's declaration tables read, and refuse to read a non-block.
#[test]
fn a_declaration_table_reads_and_a_non_block_is_refused() {
    // A minimal fragment block: one parameter, one sampler, tables abutting.
    let mut b = Vec::new();
    b.extend_from_slice(b"SHO\x08");
    b.extend_from_slice(&1u32.to_be_bytes()); // fragment
    for v in [
        2u16, 0, 1, 1, // version, 0 attributes, 1 parameter, 1 sampler
        0x18, 0x18, 0x24, 0x2c, // offsets: attrs, params, samplers, program
    ] {
        b.extend_from_slice(&v.to_be_bytes());
    }
    b.extend_from_slice(&CONSTANT_AMBIENT.to_be_bytes());
    b.extend_from_slice(&[0u8; 8]); // ty/count/vreg/fslot
    b.extend_from_slice(&LIGHTMAP_SAMPLER.to_be_bytes());
    b.extend_from_slice(&2u32.to_be_bytes()); // unit 2
    b.extend_from_slice(&[0u8; 16]);

    let d = Declared::parse(&b, 0).expect("the block reads");
    assert_eq!(d.parameters, vec![CONSTANT_AMBIENT]);
    assert_eq!(d.samplers, vec![(LIGHTMAP_SAMPLER, 2)]);
    assert!(d.samples_lightmap());
    assert!(d.takes_constant_ambient());

    assert!(Declared::parse(b"not a block at all", 0).is_none());
    assert!(Declared::parse(&b, 4).is_none(), "no magic at that offset");

    // The framing check: tables that do not abut are refused rather than read
    // as something else.
    let mut bad = b.clone();
    bad[0x12] = 0x00;
    bad[0x13] = 0x40; // params offset no longer follows the attributes
    assert!(Declared::parse(&bad, 0).is_none());
}

/// **The declaration does not name a lighting family**, and this records why.
///
/// Splitting on `samples_lightmap` / `takes_constant_ambient` / neither puts
/// 447 of Talon's Junction's 978 drawn chunks in the third bucket, which holds
/// `track_wall`, `glasstest` and `simplefogdiffuse` beside the billboards. A
/// surface lit only by the interpolated per-vertex term declares neither,
/// because both of its light sources are interpolators rather than uniforms.
#[test]
fn declaring_neither_light_input_does_not_mean_unlit() {
    let empty = Declared::default();
    assert!(!empty.samples_lightmap());
    assert!(!empty.takes_constant_ambient());
    // Deliberately no `lighting()` to call: the header cannot answer it, and a
    // method here would invite the caller to believe it can.
}
