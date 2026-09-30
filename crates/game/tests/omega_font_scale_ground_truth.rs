//! Omega's faces against HD's: the measurement behind
//! `oag_display::space::Space::font_texel_scale`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(omega_font_scale_ground_truth)'
//! ```

fn median(mut v: Vec<f32>) -> f32 {
    v.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    v[v.len() / 2]
}

/// Every shared glyph of `helv`, `helvb` and `PS_BUTTONS` is twice HD's size
/// at the median, which is why Omega's text is laid out at half its texels.
#[test]
#[ignore = "needs the decrypted PS4 package pair and the decrypted HD image"]
fn omega_faces_are_hds_at_twice_the_pixel_size() {
    let (Some(omega), Some(hd)) = (
        oag_testdata::exact("data/extracted/ps4"),
        oag_testdata::exact("data/images/hdfury-ps3-eu-dec.iso"),
    ) else {
        return;
    };
    let mut o = oag_omega::open(&omega.display().to_string()).expect("open omega");
    let mut h = oag_hd::open(&hd.display().to_string()).expect("open hd");
    for name in ["helv.fnt", "helvb.fnt", "PS_BUTTONS.fnt"] {
        let path = format!(r"Data\FE\Fonts\{name}");
        let (fo, fh) = (
            o.read_font(&path).expect("omega"),
            h.read_font(&path).expect("hd"),
        );
        let (mut widths, mut heights, mut advances) = (vec![], vec![], vec![]);
        for g in &fo.glyphs {
            let Some(x) = fh.glyphs.iter().find(|x| x.codepoint == g.codepoint) else {
                continue;
            };
            if x.width > 0 && x.height > 0 && x.advance > 0 {
                widths.push(f32::from(g.width) / f32::from(x.width));
                heights.push(f32::from(g.height) / f32::from(x.height));
                advances.push(f32::from(g.advance) / f32::from(x.advance));
            }
        }
        assert!(
            advances.len() > 80,
            "{name}: only {} shared glyphs",
            advances.len()
        );
        for (what, v) in [
            ("width", widths),
            ("height", heights),
            ("advance", advances),
        ] {
            let m = median(v);
            assert!((m - 2.0).abs() < 0.1, "{name} {what} median {m}");
        }
    }
}

/// The three sites the 1920x1080 grid rests on, off Omega's own `skin.xml`.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn skin_xml_authors_a_1920_by_1080_grid() {
    let Some(omega) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut a = oag_omega::open(&omega.display().to_string()).expect("open omega");
    let text = String::from_utf8(
        a.read_name(oag_omega::frontend::names::FRONTEND_ROOT)
            .expect("skin.xml"),
    )
    .expect("utf-8");
    for needle in [
        r#"x="960" y="972""#,
        r#"x="160" y="975" width="1600""#,
        r#"x="160" y="110" width="1600""#,
    ] {
        assert!(text.contains(needle), "{needle} not in skin.xml");
    }
}
