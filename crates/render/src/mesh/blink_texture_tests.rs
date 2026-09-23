use super::{ANIMATED_TEXTURES, animated_v_cycles, is_blink_light_texture};

#[test]
fn matches_the_shared_blink_texture_however_it_is_cased() {
    assert!(is_blink_light_texture("colours_flashing_GLOW.tga"));
    assert!(is_blink_light_texture("COLOURS_FLASHING_GLOW.TGA"));
}

#[test]
fn excludes_unrelated_textures() {
    assert!(!is_blink_light_texture("engine_general.tga"));
    assert!(!is_blink_light_texture("texture1.tga"));
}

#[test]
fn the_blink_palette_animates_through_the_table_too() {
    assert_eq!(animated_v_cycles("colours_flashing_GLOW.tga"), Some(2.0));
}

#[test]
fn track_entries_are_matched_case_insensitively() {
    assert!(animated_v_cycles("col_display7_GLOW.tga").is_some());
    assert!(animated_v_cycles("07_Pulse_light_BLEND_GLOW.TGA").is_some());
    assert!(animated_v_cycles("rf_cyclegrad3_GLOW.tga").is_some());
}

/// The static sponsor art that a `_GLOW`/`_ADD` suffix rule would have
/// swept up. Each of these is a real texture on a real circuit.
#[test]
fn static_art_never_animates() {
    for label in [
        "hub_banner_GLOW.tga",
        "col_banners2_ADD.tga",
        "FEISAR3_GLOW.tga",
        "Harimau_Glow.tga",
        "billboard1.tga",
        "banner2.tga",
        "tunnelanim_sb.tga",
        "flicker1nonalpha_GLOW.tga",
        "Plasma_scroll_ADD_GLOW.tga",
        "SL_stripwindows_shinemap.tga",
    ] {
        assert_eq!(animated_v_cycles(label), None, "{label} must not animate");
    }
}

/// `col_display7_GLOW` is a prefix of `col_display7_BLEND_GLOW` in neither
/// direction, but both contain `col_display7`, so the longest-match rule is
/// what keeps a future shorter entry from swallowing a longer one.
#[test]
fn the_longest_matching_entry_wins() {
    let table_keys: Vec<&str> = ANIMATED_TEXTURES.iter().map(|&(n, _)| n).collect();
    assert!(table_keys.contains(&"col_display7_glow"));
    assert!(table_keys.contains(&"col_display7_blend_glow"));
    assert_eq!(
        animated_v_cycles("col_display7_BLEND_GLOW.tga"),
        Some(2.0),
        "matched by the more specific entry"
    );
}
