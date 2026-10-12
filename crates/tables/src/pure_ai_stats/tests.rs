use super::*;

fn grid() -> String {
    (1..=8)
        .map(|n| {
            format!(
                r#"<GridPlace{n} NormalRaceSpeedMax="{}" NormalRaceSpeedMin="{n}" StartSpeed="{}"/>"#,
                n + 10,
                n * 100
            )
        })
        .collect()
}

fn zones(stem: &str, base: i32) -> String {
    (1..=6)
        .map(|n| {
            format!(
                r#"<{stem}{n} Position="{}" SpeedPercent="{}"/>"#,
                base + n * 10,
                n * 20
            )
        })
        .collect()
}

const FATIGUE: &str = r#"<FatigueStats><FatigueStartTime val="1"/><FatigueDecayTime val="2"/>
<FatigueMultiplier val="3"/><RestoreRate val="4"/><FatigueZoneFront val="5"/>
<FatigueZoneBack val="-6"/></FatigueStats>"#;

fn race(extra_legacy: &str) -> String {
    format!(
        r#"<?xml version="1.0"?><AIStats><MasterSpeedTweak max="9" min="9"/>
<VenomStats><GridSpread distance="77"/><StartStats>{}</StartStats>
<ZoningStats>{}{}{extra_legacy}</ZoningStats>{FATIGUE}</VenomStats>
<VenomStats><NewStats><GridSpread distance="1"/><PosBalancing/></NewStats></VenomStats>
</AIStats>"#,
        grid(),
        zones("Zone", -100),
        zones("LeadZone", 0)
    )
}

#[test]
fn the_legacy_block_is_read_and_new_stats_is_skipped() {
    let r = parse_race(&race(""), PureClass::Venom).unwrap();
    assert_eq!(
        r.grid_spread, 77.0,
        "the legacy GridSpread, not <NewStats>' 1"
    );
    assert_eq!(r.cruise_min[0], 1.0);
    assert_eq!(r.cruise_max[7], 18.0);
    assert_eq!(r.start_speed[3], 400.0);
    assert_eq!(r.zone.position, [-90.0, -80.0, -70.0, -60.0, -50.0, -40.0]);
    assert_eq!(
        r.zone.speed[0],
        20.0_f32 * 0.01,
        "SpeedPercent times 0.01 in f32, as AiStats_ParseZoning stores it"
    );
    assert_eq!(r.lead_zone.position[5], 60.0);
    assert_eq!(r.fatigue.zone_back, -6.0);
    assert_eq!(r.fatigue.restore_rate, 4.0);
}

#[test]
fn a_tail_zone_in_the_legacy_block_is_refused() {
    let doc = race(r#"<TailZone1 Position="1" SpeedPercent="1"/>"#);
    assert!(matches!(
        parse_race(&doc, PureClass::Venom),
        Err(Error::UnknownElement { .. })
    ));
}

#[test]
fn a_class_the_document_lacks_is_missing() {
    assert!(parse_race(&race(""), PureClass::Vector).is_err());
}

#[test]
fn controller_po_is_skipped_and_controller_read() {
    let doc = r#"<AIStats><FlashStats><ControllerPO SteerMul="9"/>
<Controller SteerMul="1.5" LookAheadSecs="0.25"/></FlashStats></AIStats>"#;
    let c = parse_controller(doc, PureClass::Flash).unwrap();
    assert_eq!(c.steer_mul, Some(1.5));
    assert_eq!(c.look_ahead_secs, Some(0.25));
    assert_eq!(c.steer_damp, None);
    let other = r#"<AIStats><FlashStats><Autopilot/><Controller/></FlashStats></AIStats>"#;
    assert!(parse_controller(other, PureClass::Flash).is_err());
}

#[test]
fn the_tweak_reads_min_and_max_by_class() {
    let doc = r#"<?xml version="1.0"?><Stats><RaceTimes Vector="1"/><AIStats>
<VectorTweak Max="0.5" Min="0.6"/><PhantomTweak Max="0.7" Min="0.7"/></AIStats></Stats>"#;
    assert_eq!(parse_tweak(doc, PureClass::Vector).unwrap(), (0.6, 0.5));
    assert_eq!(parse_tweak(doc, PureClass::Phantom).unwrap(), (0.7, 0.7));
    assert!(parse_tweak(doc, PureClass::Venom).is_err());
}
