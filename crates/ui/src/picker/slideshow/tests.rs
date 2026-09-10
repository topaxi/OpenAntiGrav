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
    Slideshow::read(XML, LOCATION, "Info", &[]).expect("the Info chain reads")
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
    assert!(Slideshow::read(XML, LOCATION, "Zone", &[]).is_none());
    // A file with a start state and no redirects is one terminal state.
    let lone = Slideshow::read(
        r#"<Screen><Image><Values x="1" y="2" Src="a.mip"></Values></Image><Screen name="Info"></Screen></Screen>"#,
        LOCATION,
        "Info",
        &[],
    )
    .unwrap();
    assert_eq!(lone.at(100.0).name, "Info");
    assert_eq!(lone.at(100.0).images.len(), 1);
}
