use super::*;
use oag_raceplay::Setup;

fn params(seed: u64) -> RaceParams {
    RaceParams {
        mode: "single_race".into(),
        difficulty: "elite".into(),
        scheme: "veteran".into(),
        seed,
        people: 2,
        ..RaceParams::default()
    }
}

fn race(params: &RaceParams) -> Box<Race> {
    let mut race = Box::new(Race::start(Setup::headless(
        oag_race::Mode::SingleRace,
        params.seed,
    )));
    crate::race::prepare(&mut race, params).expect("prepares");
    race
}

fn held(tick: u32, seat: u32) -> Held {
    let cross = oag_gameplay::input::Button::Cross.bit();
    Held {
        buttons: if (tick + seat) % 7 < 4 { cross } else { 0 },
        stick_x: ((tick + 3 * seat) % 11) as f32 / 10.0 - 0.5,
        ..Held::default()
    }
}

/// `ticks` records as an honest host makes them, and the race's tick-0 hash.
fn honest(params: &RaceParams, ticks: u32) -> (u64, Vec<TickRecord>) {
    let mut race = race(params);
    let fingerprint = crate::race::fingerprint(&race);
    let mut edges = [Input::new(); 2];
    let records = (0..ticks)
        .map(|tick| {
            let inputs: Vec<Held> = (0..2).map(|seat| held(tick, seat)).collect();
            let mut snapshots = PlayerInputs::none();
            for (slot, held) in inputs.iter().enumerate() {
                snapshots.set(slot, held.snapshot(&mut edges[slot]));
            }
            crate::race::presence(&mut race, 0);
            race.tick_sim_only(&snapshots);
            TickRecord {
                hash: race.sim.state_hash(),
                inputs,
                retired: 0,
            }
        })
        .collect();
    (fingerprint, records)
}

fn proof(
    key: &PlayerKey,
    params: &RaceParams,
    fingerprint: u64,
    records: Vec<TickRecord>,
) -> Proof {
    let hash = params_hash(params);
    let mut chain = Chain::new(&hash, fingerprint);
    for record in &records {
        chain.push(record);
    }
    Proof {
        claim: Claim::Desync,
        fingerprint,
        checkpoint: Checkpoint::sign(key, &hash, &chain),
        records,
    }
}

fn later() -> Instant {
    Instant::now() + web_time::Duration::from_secs(60)
}

#[test]
fn the_params_hash_names_the_race_down_to_the_seed_and_the_circuit() {
    let base = params(1);
    assert_eq!(params_hash(&base), params_hash(&base.clone()));
    assert_ne!(params_hash(&base), params_hash(&params(2)));
    let other = RaceParams {
        track: "02".into(),
        ..base.clone()
    };
    assert_ne!(params_hash(&base), params_hash(&other));
}

#[test]
fn a_checkpoint_binds_the_host_to_its_records_and_its_race() {
    let key = PlayerKey::generate().expect("a key");
    let p = params(7);
    let (fingerprint, records) = honest(&p, 120);
    let proof = proof(&key, &p, fingerprint, records);
    let hash = params_hash(&p);
    assert_eq!(proof.check_signed(&hash, &key.public()), Ok(()));
    let stranger = PlayerKey::generate().expect("a key").public();
    assert!(proof.check_signed(&hash, &stranger).is_err(), "another key");
    assert!(
        proof
            .check_signed(&params_hash(&params(8)), &key.public())
            .is_err(),
        "another race"
    );
    let mut altered = proof.clone();
    altered.records[50].inputs[1].stick_x = 0.9;
    assert!(
        altered.check_signed(&hash, &key.public()).is_err(),
        "a record the host did not sign"
    );
    let mut short = proof;
    short.records.pop();
    assert!(
        short.check_signed(&hash, &key.public()).is_err(),
        "records short of it"
    );
}

#[test]
fn a_proof_round_trips_and_a_malformed_or_oversized_one_is_refused_without_a_panic() {
    let key = PlayerKey::generate().expect("a key");
    let p = params(3);
    let (fingerprint, records) = honest(&p, 60);
    let mut proof = proof(&key, &p, fingerprint, records);
    proof.claim = Claim::Forged { tick: 12, seat: 1 };
    let bytes = proof.encode();
    assert_eq!(Proof::decode(&bytes, 2, 60), Ok(proof.clone()));
    assert!(Proof::decode(&bytes, 3, 60).is_err(), "another seat count");
    assert!(
        Proof::decode(&bytes, 2, 59).is_err(),
        "more ticks than the race ran"
    );
    assert!(
        Proof::decode(&bytes[..bytes.len() - 1], 2, 60).is_err(),
        "truncated"
    );
    let mut longer = bytes.clone();
    longer.push(0);
    assert!(Proof::decode(&longer, 2, 60).is_err(), "trailing bytes");
    let mut magic = bytes.clone();
    magic[0] = b'X';
    assert!(Proof::decode(&magic, 2, 60).is_err(), "no magic");
    // A header that claims a vast proof is refused on its length, before any allocation.
    let mut vast = bytes[..PROOF_HEADER].to_vec();
    vast[18..22].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Proof::decode(&vast, 2, u32::MAX).is_err());
    for len in 0..PROOF_HEADER + 40 {
        let noise: Vec<u8> = (0..len).map(|i| (i * 37 % 251) as u8).collect();
        let _ = Proof::decode(&noise, 2, 60);
    }
}

#[test]
fn the_lobby_reproduces_an_honest_host_and_proves_an_altered_state() {
    let key = PlayerKey::generate().expect("a key");
    let p = params(11);
    let (fingerprint, records) = honest(&p, 240);
    let fair = proof(&key, &p, fingerprint, records.clone());
    let verdict = judge(&mut race(&p), &fair, later());
    assert!(matches!(verdict, Verdict::Dismissed(_)), "{verdict:?}");
    let mut cheated = records;
    for record in &mut cheated[180..] {
        record.hash ^= 1;
    }
    let caught = proof(&key, &p, fingerprint, cheated);
    match judge(&mut race(&p), &caught, later()) {
        Verdict::Proven(why) => assert!(why.contains("after tick 180"), "{why}"),
        other => panic!("not proven: {other:?}"),
    }
    let elsewhere = judge(&mut race(&params(12)), &caught, later());
    assert!(
        matches!(elsewhere, Verdict::NotCheckable(_)),
        "another start state is never a ban: {elsewhere:?}"
    );
    let late = judge(&mut race(&p), &caught, Instant::now());
    assert!(
        matches!(late, Verdict::NotCheckable(_)),
        "out of time: {late:?}"
    );
}

#[test]
fn a_forgery_claim_needs_an_input_that_is_not_a_repeat_and_a_signed_answer_settles_it() {
    let key = PlayerKey::generate().expect("a key");
    let seat_key = PlayerKey::generate().expect("a key");
    let p = params(5);
    let (fingerprint, mut records) = honest(&p, 60);
    let repeat = records[19].inputs[1];
    records[20].inputs[1] = repeat;
    let proof = proof(&key, &p, fingerprint, records.clone());
    assert!(
        proof.forged_input(1, 20).is_err(),
        "a repeat is the host's right"
    );
    assert!(proof.forged_input(1, 60).is_err(), "past the records");
    let expected = proof.forged_input(1, 30).expect("not a repeat");
    let hash = params_hash(&p);
    let inputs: Vec<Held> = (27..=30).map(|t| records[t].inputs[1]).collect();
    let signature = seat_key.sign(&input_statement(&hash, 1, 27, &inputs));
    let datagram = Datagram::Inputs {
        seat: 1,
        first: 27,
        inputs,
        signature: Some(signature),
    }
    .encode();
    assert!(answer_holds(
        &datagram,
        &hash,
        1,
        30,
        expected,
        &seat_key.public()
    ));
    assert!(
        !answer_holds(&datagram, &hash, 1, 30, expected, &key.public()),
        "another key"
    );
    assert!(
        !answer_holds(&datagram, &hash, 1, 31, expected, &seat_key.public()),
        "another tick"
    );
    assert!(
        !answer_holds(&datagram, &hash, 0, 30, expected, &seat_key.public()),
        "another seat"
    );
    assert!(
        !answer_holds(&[], &hash, 1, 30, expected, &seat_key.public()),
        "nothing shown"
    );
}

#[test]
fn a_game_back_from_its_record_signs_for_the_race_it_is_welcomed_to() {
    let key = Arc::new(PlayerKey::generate().expect("a key"));
    let mut watch = Watch::new([0; 32], [0; 32], Some(key.clone()));
    let p = params(4);
    assert!(
        watch.check_welcome(&p).is_ok(),
        "no race issued: nothing to refuse"
    );
    watch.start(1, 7);
    let inputs = [held(0, 1)];
    let signature = watch.sign(0, &inputs).expect("signed with the key");
    let datagram = Datagram::Inputs {
        seat: 1,
        first: 0,
        inputs: inputs.to_vec(),
        signature: Some(signature),
    };
    assert!(inputs_signed(&datagram, &params_hash(&p), 1, &key.public()));
}

#[test]
fn a_forgery_claim_older_than_half_the_ledger_is_not_put_to_the_host() {
    assert!(!too_old_to_challenge(400, 300), "a fresh claim");
    assert!(
        !too_old_to_challenge(300 + LEDGER_TICKS / 2, 300),
        "at the edge"
    );
    assert!(too_old_to_challenge(301 + LEDGER_TICKS / 2, 300), "past it");
    // Half the ledger plus the challenge's own time is still inside what the host keeps.
    assert!(LEDGER_TICKS / 2 + 600 < LEDGER_TICKS);
}

#[test]
fn a_watch_refuses_another_race_and_reports_an_input_it_never_sent_once_a_checkpoint_covers_it() {
    let host = PlayerKey::generate().expect("a key");
    let p = params(9);
    let hash = params_hash(&p);
    let mut watch = Watch::new(hash, host.public(), None);
    assert!(watch.check_welcome(&p).is_ok());
    assert!(watch.check_welcome(&params(10)).is_err());
    let (fingerprint, mut records) = honest(&p, 120);
    watch.start(1, fingerprint);
    let mut chain = Chain::new(&hash, fingerprint);
    for (tick, record) in records.iter_mut().enumerate() {
        let tick = u32::try_from(tick).expect("tick");
        let _ = watch.sign(tick, &[record.inputs[1]]);
        if tick == 70 {
            record.inputs[1].stick_x = -0.75;
        }
        watch.confirmed(tick, record, true);
        chain.push(record);
        if tick == 59 {
            watch.checkpoint(Checkpoint::sign(&host, &hash, &chain));
            assert!(
                watch.take_report().is_none(),
                "nothing to report before tick 70"
            );
        }
    }
    watch.checkpoint(Checkpoint::sign(&host, &hash, &chain));
    let report = watch.take_report().expect("a report");
    assert_eq!(report.claim, Claim::Forged { tick: 70, seat: 1 });
    assert_eq!(report.check_signed(&hash, &host.public()), Ok(()));
    assert_eq!(watch.stats().checkpoints_verified, 2);
    let stranger = PlayerKey::generate().expect("a key");
    watch.checkpoint(Checkpoint::sign(&stranger, &hash, &chain));
    assert_eq!(watch.stats().checkpoints_unsigned, 1);
}
