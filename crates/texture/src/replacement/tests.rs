use super::*;

#[test]
fn key_round_trips_and_ignores_the_address() {
    let key = Key::parse("000000007d9d618f1a5be898").expect("24 hex");
    assert_eq!(key.dim_clut, 0x7d9d_618f);
    assert_eq!(key.data, 0x1a5b_e898);
    assert_eq!(key.to_string(), "000000007d9d618f1a5be898");
    assert_eq!(
        Key::parse("12345678625c9ceb6ac651f1").map(|k| k.dim_clut),
        Some(0x625c_9ceb)
    );
}

#[test]
fn key_refuses_what_is_not_24_hex() {
    assert!(Key::parse("0000000000000909").is_none());
    assert!(Key::parse("0000000000000000cd0aa2c1_1").is_none());
    assert!(Key::parse("0000000000000000cd0aa2cg").is_none());
}

#[test]
fn dim_is_the_two_exponents() {
    assert_eq!(dim(256, 128), 0x708);
    assert_eq!(dim(512, 512), 0x909);
}

#[test]
fn register_word_for_a_full_8888_palette() {
    assert_eq!(clut_register(CLUT_FORMAT_ABGR8888, 0, 0xff, 0), 0xC500_FF03);
}

#[test]
fn contiguous_hash_covers_stride_times_height() {
    let memory: Vec<u8> = (0..64u8).collect();
    let want = xxh64(&memory[..32], DATA_SEED) as u32;
    assert_eq!(data_hash(&memory, 8, 8, 4, 8, 0), Some(want));
    assert_eq!(data_hash(&memory, 8, 8, 16, 8, 0), None);
}

#[test]
fn gapped_hash_folds_rows_with_times_eleven() {
    let memory: Vec<u8> = (0..64u8).collect();
    let r0 = xxh64(&memory[0..4], DATA_SEED) as u32;
    let r1 = xxh64(&memory[8..12], DATA_SEED) as u32;
    let want = 0u32.wrapping_mul(11) ^ r0;
    let want = want.wrapping_mul(11) ^ r1;
    assert_eq!(data_hash(&memory, 8, 4, 2, 8, 0), Some(want));
}

#[test]
fn a_tall_texture_is_clamped_to_the_highest_sampled_v() {
    let memory = vec![7u8; 512 * 4];
    let clamped = data_hash(&memory, 4, 4, 512, 8, 272);
    assert_eq!(clamped, data_hash(&memory[..272 * 4], 4, 4, 272, 8, 0));
    assert_ne!(clamped, data_hash(&memory, 4, 4, 512, 8, 0));
}
