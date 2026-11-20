use super::*;

fn window(s: f32) -> (f32, f32) {
    let window = Vwipe::window(s).expect("a window below s = 1");
    (window.top, window.height)
}

fn close(got: (f32, f32), want: (f32, f32)) {
    assert!(
        (got.0 - want.0).abs() < 1e-5 && (got.1 - want.1).abs() < 1e-5,
        "{got:?} vs {want:?}"
    );
}

/// "What is drawn: `Image_Render`": a 0.1 H slab at `2.25 s H` for `s` below 0.4.
#[test]
fn the_slab_sweeps_down_for_the_first_two_fifths() {
    close(window(0.0), (0.0, 0.1));
    close(window(0.2), (0.45, 0.1));
    close(window(0.39), (0.8775, 0.1));
}

/// The same slab back up at `(0.9 - 2.25 (s - 0.4)) H` for `s` from 0.4 to 0.8, joined to the
/// first sweep at the apex.
#[test]
fn the_slab_comes_back_up_to_the_top() {
    close(window(0.4), (0.9, 0.1));
    close(window(0.6), (0.45, 0.1));
    close(window(0.79), (0.0225, 0.1));
}

/// Then the quad grows from its top edge: `(0.1 + 4.5 (s - 0.8)) H`, whole at `s = 1`.
#[test]
fn the_quad_grows_down_from_its_top_and_is_plain_at_one() {
    close(window(0.8), (0.0, 0.1));
    close(window(0.9), (0.0, 0.55));
    close(window(0.99), (0.0, 0.1 + 0.19 * 4.5));
    assert_eq!(Vwipe::window(1.0), None);
    assert_eq!(Vwipe::window(3.0), None);
}

#[test]
fn the_alpha_is_a_linear_ramp_over_the_in_time() {
    let wipe = Vwipe { seconds: 0.7 };
    assert_eq!(wipe.alpha(-1.0), 0.0);
    assert!((wipe.alpha(0.35) - 0.5).abs() < 1e-6);
    assert_eq!(wipe.alpha(0.7), 1.0);
    assert_eq!(wipe.alpha(9.0), 1.0);
    assert_eq!(Vwipe { seconds: 0.0 }.alpha(0.0), 1.0);
}

fn parse_one(values: &str) -> Option<Vwipe> {
    let xml = format!(r#"<Image name="Logo"><Values {values}></Values></Image>"#);
    let root = crate::screen::parse(&xml);
    let node = root.children.first().expect("the image");
    Screens::from_xml_with_fallback_globals("", &[]).vwipe_from_node(node)
}

/// The Team Selection `Logo` authors `enebletransition="0.5"`, which matches no key, so the class
/// default stands ("How the XML selects the transition"); a correctly spelled key is read.
#[test]
fn a_misspelt_enable_transition_leaves_the_class_default() {
    let class = Some(Vwipe {
        seconds: IMAGE_RAMP_SECONDS,
    });
    assert_eq!(
        parse_one(r#"transitiontype="vwipe" delay="0.5" enebletransition="0.5""#),
        class
    );
    assert_eq!(
        parse_one(r#"TransitionType="VWipe" EnableTransition="0.5""#),
        Some(Vwipe { seconds: 0.5 })
    );
    assert_eq!(
        parse_one(r#"transitiontype="vwipe" transition="0""#),
        Some(Vwipe { seconds: 0.0 })
    );
}

#[test]
fn only_vwipe_is_a_vwipe() {
    assert_eq!(parse_one(r#"transitiontype="hscale""#), None);
    assert_eq!(parse_one(r#"x="0""#), None);
}
