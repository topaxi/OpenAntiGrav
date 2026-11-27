use super::*;

const DAY: &str = "2026-11-18T09:15:00.000Z";
const HOUR: Duration = Duration::from_secs(3600);
const MINUTE: Duration = Duration::from_secs(60);

fn day() -> SystemTime {
    oag_log::stamp::parse(DAY).expect("a stamp")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = oag_testdata::scratch::dir(&format!("moderation-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir.join("bans.txt")
}

/// A strike on host `[1; 32]` in race `race`, reported by `[by; 32]`, at `at`.
fn strike(race: u64, by: u8, at: SystemTime) -> Strike {
    Strike {
        key: [1; 32],
        at,
        race,
        by: [by; 32],
        reason: "altered the race".into(),
    }
}

#[test]
fn a_moderation_file_names_keys_addresses_and_strikes_and_skips_the_rest_by_name() {
    let key = "ab".repeat(32);
    let by = "cd".repeat(32);
    let text = format!(
        "# the list\n\nkey {key} {DAY} altered the race\naddr 192.0.2.7 {DAY}\n\
         addr ::ffff:192.0.2.9 {DAY} v4 in v6\nkey short {DAY}\nnonsense here\n\
         addr 192.0.2.8 undated reason\n\
         key {key} {DAY} until 2026-11-18T10:15:00.000Z timed\n\
         key {key} {DAY} until tomorrow\n\
         strike {key} {DAY} race 00112233445566ff by {by} forged an input\n\
         strike {key} {DAY} race 0011 by {by}\n\
         strike {key} {DAY} race 00112233445566ff from {by}\n\
         strike {key} {DAY} race 00112233445566ff by nobody\n"
    );
    let (bans, skipped) = Bans::parse(&text);
    assert_eq!(bans.len(), 5, "{bans:?}");
    let whys = [
        "line 6: the key is not a 64-digit key",
        "line 7: no date after the nonsense",
        "line 8: no date after the addr",
        "line 10: `until` is not followed by a UTC stamp",
        "line 12: a strike needs `race <16 hex digits>`",
        "line 13: a strike needs `by <reporter's key>`",
        "line 14: the reporter's key is not a 64-digit key",
    ];
    assert_eq!(skipped.len(), whys.len(), "{skipped:?}");
    for (line, why) in skipped.iter().zip(whys) {
        assert!(line.starts_with(why), "{line:?} is not {why:?}");
    }
    let now = day() + MINUTE;
    let free: IpAddr = "198.51.100.1".parse().expect("addr");
    let refused = bans.banned(&[0xAB; 32], free, now).expect("banned");
    assert_eq!(
        (refused.reason, refused.left),
        ("altered the race", None),
        "a line without `until` is permanent and wins over a timed one"
    );
    assert_eq!(refused.to_string(), "altered the race");
    let timed = bans.key_bans(&[0xAB; 32]).nth(1).expect("the timed line");
    assert_eq!(timed.until, Some(day() + HOUR));
    let strikes: Vec<&Strike> = bans.strikes(&[0xAB; 32]).collect();
    assert_eq!(strikes.len(), 1);
    assert_eq!(
        (strikes[0].race, strikes[0].by, strikes[0].reason.as_str()),
        (0x0011_2233_4455_66ff, [0xCD; 32], "forged an input")
    );
    let addr = |text: &str| text.parse::<IpAddr>().expect("addr");
    let reason = |addr| bans.banned(&[1; 32], addr, now).map(|r| r.reason);
    assert_eq!(reason(addr("192.0.2.7")), Some(NO_REASON));
    assert_eq!(reason(addr("192.0.2.9")), Some("v4 in v6"));
    assert_eq!(
        reason(addr("192.0.2.8")),
        None,
        "an undated line bans nobody"
    );
    assert_eq!(reason(free), None);
}

#[test]
fn a_timed_ban_refuses_with_the_time_left_and_not_after_it_ends() {
    let key = "ab".repeat(32);
    let text = format!("key {key} {DAY} until 2026-11-18T10:15:00.000Z caught cheating\n");
    let (bans, skipped) = Bans::parse(&text);
    assert!(skipped.is_empty(), "{skipped:?}");
    let ip: IpAddr = "198.51.100.1".parse().expect("addr");
    let at = |after: Duration| bans.banned(&[0xAB; 32], ip, day() + after);
    assert_eq!(
        at(Duration::ZERO).map(|r| r.to_string()).as_deref(),
        Some("caught cheating (1 h left)")
    );
    assert_eq!(
        at(MINUTE + Duration::from_secs(1))
            .map(|r| r.to_string())
            .as_deref(),
        Some("caught cheating (59 min left)")
    );
    assert_eq!(at(HOUR), None, "expired");
    assert_eq!(span(Duration::from_secs(90 * 60)), "1 h 30 min");
    assert_eq!(span(Duration::from_secs(26 * 3600)), "1 d 2 h");
    assert_eq!(span(LONGEST_BAN), "30 d");
}

#[test]
fn strikes_count_over_distinct_races_and_distinct_reporters_whatever_their_order() {
    let (x, y) = ([7; 32], [8; 32]);
    // Greedy would pair race 1 with X and find nothing left for race 2.
    assert_eq!(distinct_pairs(&[(1, x), (1, y), (2, x)]), 2);
    assert_eq!(distinct_pairs(&[(1, x), (2, x), (3, x)]), 1, "one reporter");
    assert_eq!(distinct_pairs(&[(1, x), (1, y)]), 1, "one race");
    assert_eq!(distinct_pairs(&[(1, x), (2, y), (3, [9; 32])]), 3);
    assert_eq!(
        distinct_pairs(&[(1, [0; 32]), (2, [0; 32])]),
        1,
        "unkeyed is one"
    );
    assert_eq!(distinct_pairs(&[]), 0);
}

#[test]
fn one_reporter_or_one_race_is_one_strikes_worth_and_bans_nobody() {
    let file = scratch("one-strike");
    let mut bans = BanFile::new(Some(file.clone()));
    let t = day();
    assert_eq!(bans.strike(&strike(1, 7, t), t).ban, None);
    assert_eq!(
        bans.strike(&strike(2, 7, t + MINUTE), t + MINUTE).ban,
        None,
        "one reporter"
    );
    let standing = bans.strike(&strike(2, 8, t + 2 * MINUTE), t + 2 * MINUTE);
    assert_eq!(standing.counted, 2, "races 1 and 2 by two reporters");
    let file2 = scratch("one-race");
    let mut one_race = BanFile::new(Some(file2.clone()));
    one_race.strike(&strike(1, 7, t), t);
    let standing = one_race.strike(&strike(1, 8, t), t);
    assert_eq!((standing.counted, standing.ban), (1, None), "one race");
    let ip: IpAddr = "198.51.100.1".parse().expect("addr");
    assert!(one_race.bans().banned(&[1; 32], ip, t).is_none());
    let _ = std::fs::remove_dir_all(file.parent().expect("dir"));
    let _ = std::fs::remove_dir_all(file2.parent().expect("dir"));
}

#[test]
fn strikes_outside_the_window_do_not_add_up() {
    let mut bans = BanFile::new(None);
    let t = day();
    bans.strike(&strike(1, 7, t), t);
    let later = t + STRIKE_WINDOW + MINUTE;
    let standing = bans.strike(&strike(2, 8, later), later);
    assert_eq!((standing.counted, standing.ban), (1, None));
}

#[test]
fn each_later_ban_doubles_up_to_the_cap_and_a_ban_uses_up_its_strikes() {
    let file = scratch("doubling");
    let ip: IpAddr = "198.51.100.1".parse().expect("addr");
    let mut t = day();
    let mut lengths = Vec::new();
    for round in 0..3_u64 {
        let mut bans = BanFile::new(Some(file.clone()));
        assert!(
            bans.bans().banned(&[1; 32], ip, t).is_none(),
            "round {round}: free"
        );
        bans.strike(&strike(round * 2, 7, t), t);
        assert!(bans.bans().banned(&[1; 32], ip, t).is_none(), "one strike");
        t += MINUTE;
        let standing = bans.strike(&strike(round * 2 + 1, 8, t), t);
        let length = standing.ban.expect("two strikes ban");
        lengths.push(length);
        let refused = bans.bans().banned(&[1; 32], ip, t).expect("banned");
        assert_eq!(refused.left, Some(length));
        // A third strike while banned makes no second ban.
        assert_eq!(bans.strike(&strike(99 + round, 9, t), t).ban, None);
        t += length;
        assert!(
            BanFile::new(Some(file.clone()))
                .bans()
                .banned(&[1; 32], ip, t)
                .is_none(),
            "expired, read after a restart"
        );
        // The strikes the ban was made on are used up: one more is not a ban.
        let mut again = BanFile::new(Some(file.clone()));
        assert_eq!(again.bans().standing(&[1; 32], t).counted, 0);
        t += MINUTE;
        let _ = again.strike(&strike(1_000 + round, 10 + round as u8, t), t);
        assert_eq!(again.bans().standing(&[1; 32], t).ban, None);
        t += STRIKE_WINDOW + MINUTE;
    }
    assert_eq!(lengths, [HOUR, 2 * HOUR, 4 * HOUR]);
    let text = std::fs::read_to_string(&file).expect("written");
    let (read, skipped) = Bans::parse(&text);
    assert!(skipped.is_empty(), "{text}");
    assert_eq!(read.key_bans(&[1; 32]).count(), 3, "{text}");
    assert_eq!(ban_length(10), 512 * HOUR);
    assert_eq!(ban_length(11), LONGEST_BAN, "capped");
    assert_eq!(ban_length(usize::MAX), LONGEST_BAN, "no overflow");
    let _ = std::fs::remove_dir_all(file.parent().expect("dir"));
}

#[test]
fn a_ban_the_lobby_makes_is_written_with_its_dates_and_reads_back() {
    let file = scratch("dates");
    let mut bans = BanFile::new(Some(file.clone()));
    let t = day();
    let mut first = strike(1, 7, t);
    first.reason = "altered\nthe race".into();
    bans.strike(&first, t);
    bans.strike(&strike(2, 8, t), t);
    let written = std::fs::read_to_string(&file).expect("written");
    assert_eq!(written.lines().count(), 3, "{written}");
    let (again, skipped) = Bans::parse(&written);
    assert!(skipped.is_empty(), "{written}");
    assert_eq!(
        again.strikes(&[1; 32]).next().map(|s| s.reason.as_str()),
        Some("altered the race")
    );
    let ban = again.key_ban(&[1; 32], t).expect("read back");
    assert_eq!((ban.since, ban.until), (t, Some(t + HOUR)));
    assert!(
        ban.reason.starts_with("caught cheating in 2 races"),
        "{written}"
    );
    assert_eq!(again, *bans.bans(), "the file is what the lobby holds");
    let _ = std::fs::remove_dir_all(file.parent().expect("dir"));
}

#[test]
fn a_late_report_on_a_race_a_ban_was_made_on_does_not_count_toward_the_next_ban() {
    let mut bans = BanFile::new(None);
    let t = day();
    bans.strike(&strike(1, 7, t), t);
    assert_eq!(bans.strike(&strike(2, 8, t), t).ban, Some(HOUR));
    // Another guest of race 2 reports within the grace window, after the ban was made.
    let late = t + MINUTE;
    assert_eq!(
        bans.strike(&strike(2, 9, late), late).ban,
        None,
        "banned already"
    );
    let after = t + HOUR + MINUTE;
    let standing = bans.strike(&strike(3, 10, after), after);
    assert_eq!(
        (standing.counted, standing.ban),
        (1, None),
        "race 2 was used up by the ban: one new case is one strike"
    );
    let next = after + MINUTE;
    assert_eq!(bans.strike(&strike(4, 11, next), next).ban, Some(2 * HOUR));
}
