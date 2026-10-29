use super::*;

/// `Data\Environments\16_Track\screen.xml` in miniature: the same nesting,
/// three cards deep instead of four, with the same two-second chain.
const XML: &str = r#"
<Screen>
<Viewport><Values Reset="true"></Values></Viewport>
<Screen name="Top">
<Mode3D>
<Values OriginX="-145.0" OriginY="13.0" nearZ="1000.0" farZ="5000.0"></Values>
<Model name="Ship" DisableTransition="0">
<Values Src="%s\FE\forward.vex" x="0.0" y="-300.0" z="-3700.0" RotY="0.1" RotX="1.5"></Values>
</Model>
</Mode3D>
<Screen>
<Image><Values x="50" y="70" Src="%s\FE\image_01.mip"></Values></Image>
<Screen name="Info">
<Redirect delay="2"><Values forward="none" backward="none"></Values><Default goto="Info2"></Default></Redirect>
</Screen>
<Screen>
<Image><Values x="47" y="69" width="226" height="128" color="0xCF000000" txtrwidth="113" txtrheight="64" Src="Data\FE\Images\track_sel_shadow.mip"></Values></Image>
<Image><Values x="43" y="65" Src="%s\FE\image_02.mip"></Values></Image>
<Screen name="Info2">
<Redirect delay="2"><Values forward="none" backward="none"></Values><Default goto="Info3"></Default></Redirect>
</Screen>
<Screen name="Info2 Close">
<Redirect delay="2"><Values forward="none" backward="none"></Values><Default goto="Info"></Default></Redirect>
</Screen>
<Screen>
<Image><Values x="40" y="64" width="226" height="128" color="0xCF000000" txtrwidth="113" txtrheight="64" Src="Data\FE\Images\track_sel_shadow.mip"></Values></Image>
<Image><Values x="36" y="60" Src="%s\FE\image_03.mip"></Values></Image>
<Screen name="Info3">
<Redirect delay="2"><Values forward="none" backward="none"></Values><Default goto="Info2 Close"></Default></Redirect>
</Screen>
</Screen>
</Screen>
</Screen>
</Screen>
</Screen>
"#;

const LOCATION: &str = r"Data\Environments\16_Track";

fn show() -> Slideshow {
    Slideshow::read(XML, LOCATION, "Info", &[], &StringTable::default())
        .expect("the Info chain reads")
}

#[test]
fn the_chain_stacks_the_cards_and_walks_back_down() {
    let show = show();
    let stacked = |seconds: f32| show.at(seconds).images.len();
    // Info, Info2, Info3, Info2 Close, then round again: 1, 2, 3+shadow,
    // 2, 1 cards - the shadow under each card but the first counts too.
    assert_eq!(show.at(0.0).name, "Info");
    assert_eq!(stacked(0.0), 1);
    assert_eq!(show.at(2.5).name, "Info2");
    assert_eq!(stacked(2.5), 3, "image_01, its shadow, image_02");
    assert_eq!(show.at(4.5).name, "Info3");
    assert_eq!(stacked(4.5), 5);
    assert_eq!(show.at(6.5).name, "Info2 Close");
    assert_eq!(stacked(6.5), 3);
    assert_eq!(show.at(8.5).name, "Info", "eight seconds round");
    assert_eq!(show.at(10.5).name, "Info2");
    assert_eq!(show.at(800.5).name, "Info", "the cycle holds at any hour");
}

#[test]
fn the_stills_are_named_with_the_circuits_own_location() {
    let show = show();
    assert_eq!(
        show.sources(),
        vec![
            r"Data\Environments\16_Track\FE\image_01.mip".to_string(),
            r"Data\FE\Images\track_sel_shadow.mip".to_string(),
            r"Data\Environments\16_Track\FE\image_02.mip".to_string(),
            r"Data\Environments\16_Track\FE\image_03.mip".to_string(),
        ],
        "each once, in first-use order"
    );
    let model = show.model.as_ref().expect("the Mode3D model reads");
    assert_eq!(model.src, r"Data\Environments\16_Track\FE\forward.vex");
    assert_eq!(model.position, [0.0, -300.0, -3700.0]);
    assert_eq!(model.origin, [-145.0, 13.0]);
}

#[test]
fn a_card_draws_at_its_authored_place_and_the_shadow_scales_its_sub_rect() {
    let show = show();
    let placed = |src: &str| -> Option<Placed> {
        let (width, height) = if src.ends_with("track_sel_shadow.mip") {
            (128, 64)
        } else {
            (256, 128)
        };
        Some(Placed {
            x: 0,
            y: 0,
            width,
            height,
            quad_extent: None,
            quad_centre: None,
            blend: None,
        })
    };
    let draws = show.draws(3.0, &placed);
    assert_eq!(draws.len(), 3);
    assert_eq!(
        draws[0],
        Draw::Sprite {
            rect: [50.0, 70.0, 256.0, 128.0],
            uv: [0.0, 0.0, 256.0, 128.0],
            color: [1.0, 1.0, 1.0, 1.0],
        },
        "the first card at its native size"
    );
    match &draws[1] {
        Draw::Sprite { rect, uv, color } => {
            assert_eq!(*rect, [47.0, 69.0, 226.0, 128.0]);
            assert_eq!(
                *uv,
                [0.0, 0.0, 113.0, 64.0],
                "113x64 of the texture, doubled"
            );
            assert!((color[3] - 0xCF as f32 / 255.0).abs() < 1e-3);
        }
        other => panic!("{other:?}"),
    }
    // A still the sheet does not hold is left out, not boxed.
    let none = show.draws(3.0, &|_| None);
    assert!(none.is_empty());
}

#[test]
fn a_chain_the_file_does_not_author_is_none() {
    assert!(Slideshow::read(XML, LOCATION, "Zone", &[], &StringTable::default()).is_none());
    // A file with a start state and no redirects is one terminal state.
    let lone = Slideshow::read(
        r#"<Screen><Image><Values x="1" y="2" Src="a.mip"></Values></Image><Screen name="Info"></Screen></Screen>"#,
        LOCATION,
        "Info",
        &[],
        &StringTable::default(),
    )
    .unwrap();
    assert_eq!(lone.at(100.0).name, "Info");
    assert_eq!(lone.at(100.0).images.len(), 1);
}

/// `Team Selection`'s own stat panel, authored above any named `Screen` in
/// the entry's `screen.xml` - the same shape `Data\Ships\Feisar\screen.xml`
/// carries: a `TabBackColor`/`TabFrontColor` bar pair, a resolved `idstring`
/// label and a literal-string value beside it. Both are inherited into every
/// state the same way a still is.
#[test]
fn the_panel_bars_and_labels_are_read_above_any_named_screen() {
    let xml = r#"
    <Screen>
    <Viewport>
    <Values Reset="true"></Values>
    <Image><Values x="247" y="180" Width="100" Height="4" color="0xFF2081A1"></Values></Image>
    <Text><Values idstring="SPEED" font="Stats" x="249" y="164" color="0xFF11ACD0"></Values></Text>
    <Text><Values string="2/5" font="Stats" x="329" y="164" color="0xFF11ACD0"></Values></Text>
    <Screen name="Info">
    <Image><Values x="50" y="70" Src="%s\FE\image_01.mip"></Values></Image>
    </Screen>
    </Viewport>
    </Screen>
    "#;
    let mut strings = StringTable::default();
    strings.merge(std::collections::HashMap::from([(
        "SPEED".to_string(),
        "GESCHWINDIGKEIT".to_string(),
    )]));
    let show = Slideshow::read(xml, LOCATION, "Info", &[], &strings)
        .expect("the panel and its Info state both read");
    let state = show.at(0.0);
    assert_eq!(state.fills.len(), 1, "the bar, with no src");
    assert_eq!(state.fills[0].color, 0xFF20_81A1);
    assert_eq!(
        state.texts.len(),
        2,
        "the idstring label and the literal value"
    );
    assert_eq!(
        state.texts[0].string.as_deref(),
        Some("GESCHWINDIGKEIT"),
        "idstring resolved through the string table"
    );
    assert_eq!(state.texts[1].string.as_deref(), Some("2/5"));

    let draws = show.draws(0.0, &|_| None);
    assert_eq!(
        draws,
        vec![
            Draw::Fill {
                rect: [247.0, 180.0, 100.0, 4.0],
                color: argb_to_rgba(0xFF20_81A1),
            },
            Draw::Text {
                x: 249.0,
                y: 164.0,
                scale: 1.0,
                color: argb_to_rgba(0xFF11_ACD0),
                border: None,
                align: Align::Left,
                text: "GESCHWINDIGKEIT".to_string(),
                wrap_width: None,
            },
            Draw::Text {
                x: 329.0,
                y: 164.0,
                scale: 1.0,
                color: argb_to_rgba(0xFF11_ACD0),
                border: None,
                align: Align::Left,
                text: "2/5".to_string(),
                wrap_width: None,
            },
        ],
        "the panel draws before the still, since it sits before it in the XML"
    );
}

/// `Data\Ships\Assegai\screen.xml` as the disc authors it (compact form expanded): two
/// screens, the split-screen lobby's first, each with its own `Mode3D` and `Model`.
const CRAFT_XML: &str = r#"
<Screen>
<Screen name="MPLobby Info">
<Mode3D><Values OriginX="-100.0" OriginY="20.0" nearZ="1.0"></Values>
<Model name="Ship" DisableTransition="0">
<Values Src="Data\Ships\Assegai\ship_FE.vex" x="0.0" y="0.0" z="-15.0" RotX="0.4" RotY="0.4"></Values>
</Model></Mode3D>
</Screen>
<Screen name="Info">
<Mode3D><Values OriginX="120.0" OriginY="20.0" nearZ="1.0"></Values>
<Model name="Ship" DisableTransition="0">
<Values Src="Data\Ships\Assegai\ship_FE.vex" x="0.0" y="0.0" z="-15.0" RotX="0.4" RotY="0.4"></Values>
</Model></Mode3D>
</Screen>
</Screen>
"#;

/// Pulse's Ship Select has no turntable in code: the hull's pose is the craft `screen.xml`'s own
/// `Mode3D` (`Info`'s, not the lobby's), with `farZ` left to `Mode3D_ReadValues`' default, and
/// the sway is the hull's own `Anim Transform`.
#[test]
fn a_craft_places_its_hull_from_the_info_states_own_mode3d() {
    let show = Slideshow::read(
        CRAFT_XML,
        r"Data\Ships\Assegai",
        "Info",
        &[],
        &StringTable::default(),
    )
    .expect("the Info state reads");
    let model = show.model.as_ref().expect("Info authors a Mode3D");
    assert_eq!(model.src, r"Data\Ships\Assegai\ship_FE.vex");
    assert_eq!(model.origin, [120.0, 20.0], "Info's, not MPLobby Info's");
    assert_eq!(model.position, [0.0, 0.0, -15.0]);
    assert_eq!(model.rotation, [0.4, 0.4], "authored, no turn added");
    assert_eq!(model.depth, [1.0, oag_ui::screen::DEFAULT_FAR_Z]);
}

fn flat(_: &str) -> Option<Placed> {
    Some(Placed {
        x: 0,
        y: 0,
        width: 256,
        height: 128,
        quad_extent: None,
        quad_centre: None,
        blend: None,
    })
}

fn alphas(draws: &[Draw]) -> Vec<f32> {
    draws
        .iter()
        .map(|draw| match draw {
            Draw::Sprite { color, .. } => color[3],
            other => panic!("{other:?}"),
        })
        .collect()
}

/// Measured on Pulse PSP: each step's new cards ramp in linearly over
/// [`STEP_FADE_SECONDS`], and the cards a step drops ramp out on top.
#[test]
fn a_step_cross_fades_the_cards_it_adds_and_drops() {
    let show = show();
    let half = STEP_FADE_SECONDS / 2.0;
    let shadow = 0xCF as f32 / 255.0;
    // Info -> Info2 at 2 s: the first card holds, its shadow and the second ramp in.
    let a = alphas(&show.draws_fading(2.0 + half, STEP_FADE_SECONDS, &flat));
    assert_eq!(a.len(), 3);
    assert!((a[0] - 1.0).abs() < 1e-6, "{a:?}");
    assert!((a[1] - 0.5 * shadow).abs() < 1e-3, "{a:?}");
    assert!((a[2] - 0.5).abs() < 1e-3, "{a:?}");
    // Settled: nothing extra, everything opaque.
    let settled = alphas(&show.draws_fading(3.5, STEP_FADE_SECONDS, &flat));
    assert_eq!(settled.len(), 3);
    assert!((settled[2] - 1.0).abs() < 1e-6);
    // Info3 -> Info2 Close at 6 s: the third card and its shadow ramp out, drawn on top.
    let closing = show.draws_fading(6.0 + half, STEP_FADE_SECONDS, &flat);
    let a = alphas(&closing);
    assert_eq!(a.len(), 5, "three held plus the two leaving: {a:?}");
    assert!(
        (a[3] - 0.5 * shadow).abs() < 1e-3 && (a[4] - 0.5).abs() < 1e-3,
        "{a:?}"
    );
    // The first pass through the start state has nothing to cross-fade from, and
    // `0.0` steps at once like `draws`.
    assert_eq!(
        alphas(&show.draws_fading(0.1, STEP_FADE_SECONDS, &flat)),
        [1.0]
    );
    assert_eq!(
        show.draws_fading(2.0 + half, 0.0, &flat),
        show.draws(2.0 + half, &flat)
    );
    // The cycle's wrap fades out the last state's cards too: Info2 Close -> Info at 8 s.
    let a = alphas(&show.draws_fading(8.0 + half, STEP_FADE_SECONDS, &flat));
    assert_eq!(a.len(), 3, "the first card and the two leaving: {a:?}");
}
