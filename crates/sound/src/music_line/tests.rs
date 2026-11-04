use super::*;

fn walk(line: &Line, delta: i32, ticks: i32) {
    for _ in 0..ticks {
        line.step(delta, true);
    }
}

#[test]
fn up_then_down_returns_exactly_to_where_it_started() {
    let line = Line::default();
    walk(&line, 1, 2 * SWEEP_TICKS);
    walk(&line, -1, 2 * SWEEP_TICKS);
    assert_eq!(line.ticks(), 0);
    assert_eq!(line.music_trim(4.0), 1.0);
    assert_eq!(line.others_trim(), 1.0);

    walk(&line, -1, 50);
    walk(&line, 1, 50);
    assert_eq!(line.ticks(), 0);
}

#[test]
fn the_first_stage_raises_music_to_the_slider_maximum_and_leaves_the_others() {
    let line = Line::default();
    walk(&line, 1, SWEEP_TICKS);
    assert_eq!(line.music_trim(4.0), 4.0);
    assert_eq!(line.others_trim(), 1.0);
}

#[test]
fn the_second_stage_lowers_the_others_to_a_quarter_with_music_held_at_max() {
    let line = Line::default();
    walk(&line, 1, 2 * SWEEP_TICKS);
    assert_eq!(line.music_trim(4.0), 4.0);
    assert_eq!(line.others_trim(), OTHERS_FLOOR);
    walk(&line, 1, 500);
    assert_eq!(line.ticks(), 2 * SWEEP_TICKS, "the line stops at its end");
    assert_eq!(line.others_trim(), OTHERS_FLOOR);
}

#[test]
fn every_other_bus_gets_the_same_factor_so_their_balance_holds() {
    use crate::{Audio, settings::Settings};
    let audio = Audio::open(
        &Settings::default(),
        None,
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    audio.output().with_mixer(|mixer| {
        mixer.set_bus_gain(Bus::Sfx, 0.8);
        mixer.set_bus_gain(Bus::Speech, 0.4);
    });
    for _ in 0..(SWEEP_TICKS + 60) {
        audio.step_music_line(1, Volume::FULL);
    }
    audio.output().with_mixer(|mixer| {
        let sfx = mixer.bus_trim(Bus::Sfx);
        assert!(sfx < 1.0 && sfx > OTHERS_FLOOR);
        assert_eq!(mixer.bus_trim(Bus::Speech), sfx);
        assert_eq!(mixer.bus_trim(Bus::Group(7)), sfx);
        assert_eq!(
            mixer.bus_trim(Bus::Music),
            1.0,
            "slider at max: nothing above"
        );
        assert_eq!(
            mixer.bus_gain(Bus::Sfx),
            0.8,
            "the settings' gain is untouched"
        );
        let ratio = (mixer.bus_gain(Bus::Sfx) * sfx) / (mixer.bus_gain(Bus::Speech) * sfx);
        assert!((ratio - 2.0).abs() < 1e-6);
    });
}

#[test]
fn going_down_lowers_the_music_to_a_floor_above_silence() {
    let line = Line::default();
    walk(&line, -1, 5 * SWEEP_TICKS);
    assert_eq!(line.ticks(), -SWEEP_TICKS);
    assert_eq!(line.music_trim(4.0), MUSIC_FLOOR);
    assert_eq!(line.others_trim(), 1.0);
}

#[test]
fn a_muted_music_slider_does_not_go_up() {
    let line = Line::default();
    for _ in 0..10 {
        line.step(1, false);
    }
    assert_eq!(line.ticks(), 0);
}

#[test]
fn a_new_race_and_a_settings_change_start_at_the_players_settings() {
    use crate::{Audio, settings::Settings};
    let settings = Settings::default();
    let audio = Audio::open(&settings, None, None, oag_audio::MIN_BUFFER, false);
    audio.step_music_line(SWEEP_TICKS + 40, Volume::FULL);
    assert_ne!(audio.music_line_ticks(), 0);
    audio.reset_music_line();
    assert_eq!(audio.music_line_ticks(), 0);
    audio.output().with_mixer(|mixer| {
        assert_eq!(mixer.bus_trim(Bus::Sfx), 1.0);
        assert_eq!(mixer.bus_trim(Bus::Music), 1.0);
    });

    audio.step_music_line(-60, Volume::FULL);
    audio.apply(&settings);
    assert_eq!(audio.music_line_ticks(), 0);
}

#[test]
fn the_trim_reaches_what_the_mixer_renders() {
    use oag_audio::{Mixer, Play, Sound};
    let sound = std::sync::Arc::new(Sound::new(vec![8000; 480], 1, 48_000).unwrap());
    let level = |trim: f32| {
        let mut mixer = Mixer::new(48_000);
        mixer.set_bus_trim(Bus::Music, trim);
        mixer.play(Play::once(std::sync::Arc::clone(&sound), Bus::Music));
        let mut out = vec![0.0; 960];
        mixer.render(&mut out);
        out.iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()))
    };
    let (full, half) = (level(1.0), level(0.5));
    assert!(
        full > 0.0 && (half / full - 0.5).abs() < 0.01,
        "{full} {half}"
    );
}
