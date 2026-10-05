use super::*;

const SAMPLE: &str = r#"
<ParameterMaps>
  <ParameterMapFrontEnd transition_speed="0.5">
    <Stereo>
      <GroupVolumes music="0.30" user1="1.0f" user2="1.0f" user3="1.0f" user4="1.0f" user5="1.0f" user6="1.0f" user7="0.5f" user8="0.3f" user9="0.5f" user10="0.5f" user11="0.5f" user12="0.5f"/>
    </Stereo>
    <Surround>
      <GroupVolumes music="0.99" user1="9" user2="9" user3="9" user4="9" user5="9" user6="9" user7="9" user8="9" user9="9" user10="9" user11="9" user12="9"/>
    </Surround>
  </ParameterMapFrontEnd>
  <ParameterMapRaceNormal transition_speed="0.05">
    <Stereo>
      <GroupVolumes music="0.65" user1="1.1" user2="0.9" user3="1.15" user4="0.9" user5="0.8" user6="0.8" user7="0.68" user8="0.8" user9="0.5" user10="0.0" user11="0.8" user12="0.5"/>
    </Stereo>
  </ParameterMapRaceNormal>
</ParameterMaps>
"#;

#[test]
fn a_state_reads_its_stereo_row_and_not_the_surround_one() {
    let maps = Maps::parse(SAMPLE);
    let front = maps.row(State::FrontEnd).expect("front end row");
    assert!((front[0] - 0.30).abs() < 1e-6);
    assert!((front[7] - 0.5).abs() < 1e-6);
    let race = maps.row(State::RaceNormal).expect("race row");
    assert!((race[7] - 0.68).abs() < 1e-6, "user7 is the engine group");
    assert!(maps.row(State::Countdown).is_none());
}

#[test]
fn a_trailing_f_on_a_value_is_read() {
    let maps = Maps::parse(SAMPLE);
    assert!((maps.row(State::FrontEnd).expect("row")[1] - 1.0).abs() < 1e-6);
}

#[test]
fn the_sfx_law_is_the_square_of_slider_times_group() {
    // The player's engine voices on RPCS3: K = 0.295, 0.074, 0.018 at groups
    // 0.68, 0.34, 0.17 with the slider at 0.8, in this port's own unity.
    let full = sfx_gain(0.8, 0.68) * PSP_UNITY;
    assert!((full - 0.296).abs() < 0.002, "{full}");
    assert!((sfx_gain(0.8, 0.34) * PSP_UNITY - 0.074).abs() < 0.002);
    assert!((sfx_gain(0.8, 0.17) * PSP_UNITY - 0.0185).abs() < 0.002);
}

#[test]
fn the_music_law_is_linear_in_both_terms() {
    assert!((music_gain(0.8, 0.65) - 0.52).abs() < 1e-6);
    assert!((music_gain(0.4, 0.65) * 2.0 - music_gain(0.8, 0.65)).abs() < 1e-6);
    assert!((music_gain(0.8, 0.325) * 2.0 - music_gain(0.8, 0.65)).abs() < 1e-6);
}

#[test]
fn the_live_array_snaps_once_then_glides() {
    let mut live = Live::new(Maps::parse(SAMPLE));
    live.set_state(State::FrontEnd);
    live.tick();
    assert!((live.group(0) - 0.30).abs() < 1e-6);
    live.set_state(State::RaceNormal);
    live.tick();
    let one = live.group(0);
    assert!(one > 0.30 && one < 0.65, "{one}");
    for _ in 0..600 {
        live.tick();
    }
    assert!((live.group(0) - 0.65).abs() < 1e-3);
}

#[test]
fn a_state_with_no_row_keeps_the_last_one() {
    let mut live = Live::new(Maps::parse(SAMPLE));
    live.set_state(State::RaceNormal);
    live.set_state(State::Countdown);
    live.tick();
    assert!((live.group(7) - 0.68).abs() < 1e-6);
}
