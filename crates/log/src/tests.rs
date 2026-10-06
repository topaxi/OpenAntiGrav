use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use log::{Level, Log, Record};

use super::*;
use crate::path::{Env, Os, default_path, resolve};
use crate::prune::prune_text;

const DAY: u64 = 86_400;

fn at(days: u64, secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(days * DAY + secs)
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("oag-log-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone, Default)]
struct Pipe(Arc<Mutex<Vec<u8>>>);

impl Write for Pipe {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn record(tee: &Tee, level: Level, target: &str, message: &str) {
    tee.log(
        &Record::builder()
            .level(level)
            .target(target)
            .module_path(Some(target))
            .args(format_args!("{message}"))
            .build(),
    );
}

#[test]
fn stamp_is_fixed_width_utc_and_sorts() {
    // 2026-10-06T12:34:56.789Z
    let t = UNIX_EPOCH + Duration::new(1_791_290_096, 789_000_000);
    assert_eq!(stamp::format(t), "2026-10-06T12:34:56.789Z");
    assert!(stamp::leads("2026-10-06T12:34:56.789Z INFO  a: b\n"));
    assert!(!stamp::leads("  at frame 3\n"));
    assert!(stamp::format(at(10, 0)) < stamp::format(at(11, 0)));
    assert_eq!(stamp::format(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
    assert!(stamp::format(at(11_016, 0)).starts_with("2000-02-29"));
}

#[test]
fn prune_keeps_six_days_drops_eight_and_keeps_continuations() {
    let now = at(20_000, 0);
    let line = |days_ago: u64, text: &str| {
        format!(
            "{} INFO  oag: {text}\n",
            stamp::format(now - Duration::from_secs(days_ago * DAY))
        )
    };
    let text = format!(
        "orphan from a cut entry\n{}  old continuation\n{}{}  new continuation\n{}",
        line(8, "old"),
        line(6, "six"),
        "",
        line(0, "today"),
    );
    let cutoff = stamp::format(now - prune::RETENTION);
    let kept = prune_text(&text, &cutoff).expect("something was old");
    assert!(!kept.contains("old"), "{kept}");
    assert!(!kept.contains("orphan"));
    assert!(kept.contains("six") && kept.contains("today"));
    assert!(kept.contains("  new continuation\n"));
    assert_eq!(prune_text(&kept, &cutoff), None, "idempotent");
    assert_eq!(prune_text("", &cutoff), None, "empty file");
}

#[test]
fn prune_file_rewrites_atomically_and_tolerates_missing_and_torn_files() {
    let dir = Scratch::new("prune");
    let file = dir.0.join("x.log");
    let now = at(20_000, 0);
    prune::prune_file(&file, now).unwrap();
    assert!(!file.exists(), "a missing file is not created by the prune");
    let old = stamp::format(now - Duration::from_secs(8 * DAY));
    let new = stamp::format(now - Duration::from_secs(DAY));
    fs::write(
        &file,
        format!("{old} INFO  a: old\n{new} INFO  a: new torn"),
    )
    .unwrap();
    prune::prune_file(&file, now).unwrap();
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        format!("{new} INFO  a: new torn\n")
    );
    let leftovers = fs::read_dir(&dir.0).unwrap().count();
    assert_eq!(leftovers, 1, "the temp file is renamed away");
}

#[test]
fn path_default_follows_xdg_then_home_and_the_other_platforms() {
    let env = Env {
        xdg_state_home: Some("/state".into()),
        home: Some("/home/p".into()),
        local_app_data: Some("C:/Local".into()),
    };
    let app = "oag-game";
    assert_eq!(
        default_path(Os::Linux, &env, app),
        Some("/state/oag/logs/oag-game.log".into())
    );
    let no_xdg = Env {
        xdg_state_home: None,
        ..env.clone()
    };
    assert_eq!(
        default_path(Os::Linux, &no_xdg, app),
        Some("/home/p/.local/state/oag/logs/oag-game.log".into())
    );
    let relative = Env {
        xdg_state_home: Some("rel".into()),
        ..env.clone()
    };
    assert_eq!(
        default_path(Os::Linux, &relative, app),
        Some("/home/p/.local/state/oag/logs/oag-game.log".into())
    );
    assert_eq!(
        default_path(Os::MacOs, &env, app),
        Some("/home/p/Library/Logs/oag/oag-game.log".into())
    );
    assert_eq!(
        default_path(Os::Windows, &env, app),
        Some("C:/Local/oag/logs/oag-game.log".into())
    );
    assert_eq!(default_path(Os::Linux, &Env::default(), app), None);
}

#[test]
fn resolve_orders_cli_over_setting_over_default_and_empty_disables() {
    let default = Some(PathBuf::from("/d.log"));
    assert_eq!(resolve(None, None, default.clone()), default);
    assert_eq!(
        resolve(None, Some("/s.log"), default.clone()),
        Some("/s.log".into())
    );
    assert_eq!(
        resolve(Some("/c.log"), Some("/s.log"), default.clone()),
        Some("/c.log".into())
    );
    assert_eq!(
        resolve(None, Some(""), default.clone()),
        None,
        "set but empty disables"
    );
    assert_eq!(
        resolve(Some(""), Some("/s.log"), default.clone()),
        None,
        "--log-file '' wins"
    );
    assert_eq!(
        resolve(Some("/c.log"), Some(""), default),
        Some("/c.log".into())
    );
}

#[test]
fn the_two_sinks_filter_independently() {
    let dir = Scratch::new("filters");
    let file = dir.0.join("a.log");
    let pipe = Pipe::default();
    let terminal = env_logger::Builder::new()
        .parse_filters("warn,oag=info")
        .format_timestamp(None)
        .format_target(false)
        .target(env_logger::Target::Pipe(Box::new(pipe.clone())))
        .build();
    let tee = Tee::new(terminal, "warn,oag=debug");
    record(&tee, Level::Debug, "oag_before", "held until attach");
    tee.attach(&file, None, &["build: test".into()], at(20_000, 5))
        .unwrap();
    record(&tee, Level::Debug, "oag_sound", "debug line");
    record(&tee, Level::Info, "oag_sound", "info line\nsecond line");
    record(&tee, Level::Debug, "wgpu_core", "third party debug");
    tee.flush();

    let terminal_text = String::from_utf8(pipe.0.lock().unwrap().clone()).unwrap();
    assert!(terminal_text.contains("info line"), "{terminal_text}");
    assert!(!terminal_text.contains("debug line"), "{terminal_text}");
    assert!(
        terminal_text.starts_with("[INFO ]"),
        "format unchanged: {terminal_text}"
    );

    let text = fs::read_to_string(&file).unwrap();
    assert!(text.contains("===== oag run start"));
    assert!(text.contains("DEBUG oag_sound: debug line"), "{text}");
    assert!(text.contains("DEBUG oag_before: held until attach"));
    assert!(text.contains("INFO  oag_sound: info line\nsecond line\n"));
    assert!(text.contains("INFO  oag_log: build: test"));
    assert!(!text.contains("third party debug"));
    let first = text.lines().next().unwrap();
    assert!(
        stamp::leads(first),
        "separator is stamped so a prune keeps it: {first}"
    );
}

#[test]
fn a_file_filter_override_replaces_the_default_and_raises_max_level() {
    let dir = Scratch::new("override");
    let terminal = env_logger::Builder::new()
        .parse_filters("warn")
        .target(env_logger::Target::Pipe(Box::new(Pipe::default())))
        .build();
    let tee = Tee::new(terminal, "warn");
    tee.attach(
        &dir.0.join("b.log"),
        Some("warn,oag=trace"),
        &[],
        at(20_000, 0),
    )
    .unwrap();
    assert_eq!(tee.max_level(), log::LevelFilter::Trace);
    record(&tee, Level::Trace, "oag_x", "trace line");
    tee.flush();
    assert!(
        fs::read_to_string(dir.0.join("b.log"))
            .unwrap()
            .contains("trace line")
    );
}

#[test]
fn detached_writes_nothing_and_a_second_run_appends_after_a_separator() {
    let dir = Scratch::new("append");
    let file = dir.0.join("c.log");
    let quiet = || {
        env_logger::Builder::new()
            .parse_filters("error")
            .target(env_logger::Target::Pipe(Box::new(Pipe::default())))
            .build()
    };
    let off = Tee::new(quiet(), "debug");
    record(&off, Level::Error, "oag_x", "lost");
    off.detach();
    record(&off, Level::Error, "oag_x", "lost too");
    off.flush();
    assert!(!file.exists());

    for n in 0..2 {
        let tee = Tee::new(quiet(), "debug");
        tee.attach(&file, None, &[], at(20_000, n)).unwrap();
        record(&tee, Level::Error, "oag_x", &format!("run {n}"));
        tee.flush();
    }
    let text = fs::read_to_string(&file).unwrap();
    assert_eq!(text.matches("===== oag run start").count(), 2);
    assert!(text.find("run 0").unwrap() < text.find("run 1").unwrap());
}
