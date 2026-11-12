use oag_game::boot;
use oag_ui::frontend;
fn main() {
    let src = std::env::args().nth(1).unwrap();
    let out = std::env::args().nth(2).unwrap();
    let options = boot::Options {
        language: Some("English".to_string()),
        source: src,
        dlc: Vec::new(),
        leg: frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-atlas-dump"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
        find_texture_packs: None,
    };
    let (_shell, mut archives, _title) = boot::load_shell(&options).unwrap();
    let blob = archives
        .read_name(r"Data\FE\NewImages\canvasTexture.gxt")
        .or_else(|_| archives.read_name("Data/FE/NewImages/canvasTexture.gxt"))
        .unwrap();
    let gxt = oag_texture::gxt::Gxt::parse(&blob).unwrap();
    let t = gxt.only().unwrap();
    let rgba: Vec<u8> = t.to_rgba(&blob).unwrap().into_iter().flatten().collect();
    std::fs::write(
        &out,
        oag_texture::png::encode_rgba(u32::from(t.width), u32::from(t.height), &rgba),
    )
    .unwrap();
    println!("{}x{}", t.width, t.height);
}
