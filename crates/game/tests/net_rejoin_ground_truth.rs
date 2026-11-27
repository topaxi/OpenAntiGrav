//! An online race left by a game that is killed, and the game that is started again: real
//! processes, a real `oag-server lobby` and a real circuit on localhost.
//!
//! Two `oag-game --online --net-bot` clients race Pulse's circuit 01 (one lap) through the
//! lobby. One is SIGKILLed mid-race, after it has written `rejoin.toml`, and a new `oag-game`
//! with the same profile directories is driven through the menus (`--drive`) to ONLINE, where
//! the offer is made. The claims: REJOIN RACE takes the seat back and finishes the race with
//! no bad hash on either client; DECLINE deletes the record and leaves the seat to the away
//! driver, which finishes the race; a launch after the race has ended makes no offer.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use oag_testdata::exec::{self, spawn};

const IMAGE: &str = "pulse-psp-eu.chd";
const TRACK: &str = "Data\\Environments\\01_Track\\track.vex";

/// A child process, killed (SIGKILL) and reaped when it goes out of scope.
pub(crate) struct Proc(pub(crate) Child);

impl Proc {
    pub(crate) fn kill(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }

    /// Waits for the process to exit by itself, killing it after `secs`.
    pub(crate) fn finish(&mut self, secs: u64) -> Option<std::process::ExitStatus> {
        let until = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < until {
            if let Ok(Some(status)) = self.0.try_wait() {
                return Some(status);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        self.kill();
        None
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        self.kill();
    }
}

/// A private copy, in `dest`, of the `oag-server` binary built beside this crate's `oag-game`.
///
/// The server is built here, not found, so a test never runs a server older than the source
/// (`cargo nextest` builds only what it tests); the copy is what runs
/// (`oag_testdata::exec`).
fn server_binary(dest: &Path) -> PathBuf {
    let game = Path::new(env!("CARGO_BIN_EXE_oag-game"));
    let dir = game.parent().expect("the game binary's directory");
    let profile = dir.file_name().and_then(|n| n.to_str()).unwrap_or("debug");
    let build = || {
        let mut build = Command::new(env!("CARGO"));
        build.args(["build", "-p", "oag-net", "--bin", "oag-server"]);
        if profile != "debug" {
            build.args(["--profile", profile]);
        }
        let status = build.status().expect("cargo runs");
        assert!(status.success(), "oag-server builds");
    };
    exec::private_copy(
        &oag_testdata::scratch::cache("binary-copies"),
        &dir.join("oag-server"),
        dest,
        build,
    )
}

/// A lobby and the directory its clients keep their profiles in.
pub(crate) struct Lab {
    image: PathBuf,
    addr: String,
    hash: String,
    _lobby: Proc,
    // Declared after the lobby: it is killed before its directory goes.
    pub(crate) dir: oag_testdata::scratch::TempDir,
}

impl Lab {
    /// Starts `oag-server lobby` with races that stop at `max_ticks`.
    pub(crate) fn new(tag: &str, image: PathBuf, max_ticks: u32) -> Self {
        Self::with_lobby_args(tag, image, max_ticks, &[])
    }

    /// [`Self::new`], the lobby given `extra` arguments as well.
    pub(crate) fn with_lobby_args(
        tag: &str,
        image: PathBuf,
        max_ticks: u32,
        extra: &[&str],
    ) -> Self {
        let dir = oag_testdata::scratch::dir(&format!("rejoin-{tag}"));
        std::fs::create_dir_all(dir.join("no-mods")).expect("a scratch directory");
        let log = File::create(dir.join("lobby.log")).expect("a log");
        let lobby = spawn(
            Command::new(server_binary(&dir))
                .arg("lobby")
                .arg("--image")
                .arg(&image)
                .args(["--listen", "127.0.0.1:0", "--race-listen", "127.0.0.1:0"])
                .args([
                    "--countdown-ms",
                    "200",
                    "--max-ticks",
                    &max_ticks.to_string(),
                ])
                .args(extra)
                .stdout(log.try_clone().expect("a handle"))
                .stderr(log),
        )
        .expect("the lobby starts");
        let mut lab = Self {
            dir,
            image,
            addr: String::new(),
            hash: String::new(),
            _lobby: Proc(lobby),
        };
        lab.wait("the lobby to listen", 90, |lab| {
            lab.read("lobby").contains("lobby cert-hash ")
        });
        let text = lab.read("lobby");
        let after = |key: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(key))
                .expect("the lobby prints its address")
                .trim()
                .to_string()
        };
        lab.addr = after("lobby listening on ");
        lab.hash = after("lobby cert-hash ");
        lab
    }

    pub(crate) fn read(&self, log: &str) -> String {
        let mut text = String::new();
        if let Ok(mut file) = File::open(self.dir.join(format!("{log}.log"))) {
            let mut bytes = Vec::new();
            let _ = file.read_to_end(&mut bytes);
            text = String::from_utf8_lossy(&bytes).into_owned();
        }
        text
    }

    pub(crate) fn wait(&self, what: &str, secs: u64, mut done: impl FnMut(&Self) -> bool) {
        let until = Instant::now() + Duration::from_secs(secs);
        while !done(self) {
            assert!(Instant::now() < until, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// `who`'s `rejoin.toml`.
    fn record(&self, who: &str) -> PathBuf {
        self.dir
            .join(who)
            .join("state")
            .join("oag")
            .join("rejoin.toml")
    }

    /// An `oag-game` of `who`'s profile with `args`, its output in `<who><log>.log`.
    fn game(&self, who: &str, log: &str, args: &[&str]) -> Proc {
        self.game_env(who, log, args, &[])
    }

    /// [`Self::game`] with `env` set as well.
    fn game_env(&self, who: &str, log: &str, args: &[&str], env: &[(&str, &str)]) -> Proc {
        let profile = self.dir.join(who);
        let out = File::create(self.dir.join(format!("{log}.log"))).expect("a log");
        let child = spawn(
            Command::new(env!("CARGO_BIN_EXE_oag-game"))
                .envs(env.iter().copied())
                .arg(&self.image)
                .args([
                    "--no-audio",
                    "--no-soundtrack",
                    "--no-effects",
                    "--no-intro",
                    "--net-bot",
                ])
                .args(args)
                .env("XDG_CONFIG_HOME", profile.join("config"))
                .env("XDG_DATA_HOME", profile.join("data"))
                .env("XDG_STATE_HOME", profile.join("state"))
                .env("OAG_MODS", self.dir.join("no-mods"))
                .env("OAG_TRACKS", self.dir.join("no-tracks"))
                .env_remove("WAYLAND_DISPLAY")
                .env_remove("DISPLAY")
                .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
                .stdout(out.try_clone().expect("a handle"))
                .stderr(out),
        )
        .expect("oag-game starts");
        Proc(child)
    }

    /// A headless client of the lobby. `create` makes the room; the other joins it.
    fn client(&self, who: &str, create: bool) -> Proc {
        self.client_with(who, create, &[])
    }

    /// [`Self::client`] with `extra` arguments as well.
    pub(crate) fn client_with(&self, who: &str, create: bool, extra: &[&str]) -> Proc {
        self.client_env(who, create, extra, &[])
    }

    /// [`Self::client_with`] with `env` set as well.
    pub(crate) fn client_env(
        &self,
        who: &str,
        create: bool,
        extra: &[&str],
        env: &[(&str, &str)],
    ) -> Proc {
        let shot = self.dir.join(format!("{who}.png"));
        let mut args = vec![
            "--online",
            &self.addr,
            "--lobby-hash",
            &self.hash,
            "--online-name",
            who,
            "--online-team",
            "Feisar",
            "--screenshot",
        ];
        let shot = shot.display().to_string();
        args.push(&shot);
        args.extend(["--ticks", "20000"]);
        if create {
            args.extend([
                "--online-create",
                "--online-track",
                TRACK,
                "--online-laps",
                "1",
                "--online-start-at",
                "2",
            ]);
        }
        args.extend(extra);
        self.game_env(who, who, &args, env)
    }

    /// The relaunch of `who`: the menus, then ONLINE, then `rest` of the script.
    fn relaunch(&self, who: &str, rest: &str) -> Proc {
        let script = format!(
            "boot; until stage=menu within 3000; select multiplayer; press cross; \
             select multiplayer_online; press cross; {rest}"
        );
        let state = self.dir.join(format!("{who}-state.jsonl"));
        let state = state.display().to_string();
        self.game(
            who,
            &format!("{who}-again"),
            &["--drive", &script, "--state-log", &state],
        )
    }

    /// Two clients race; `a` is on the road and has written its seat, `b` is in the race too.
    fn race(&self) -> (Proc, Proc) {
        let a = self.client("a", true);
        self.wait("the room to open", 90, |lab| {
            lab.read("a").contains("online: room 1: 1 in it")
        });
        let b = self.client("b", false);
        self.wait("both to be racing", 180, |lab| {
            lab.record("a").exists() && lab.progress("a").is_some_and(|(tick, _, _)| tick >= 300)
        });
        (a, b)
    }

    /// A client's per-second lines: the newest tick, and the hashes checked (ok, bad) in all of
    /// them (each line counts the second since the one before).
    pub(crate) fn progress(&self, log: &str) -> Option<(u32, u32, u32)> {
        let text = self.read(log);
        let number = |line: &str, after: &str| -> Option<u32> {
            line.split(after)
                .nth(1)?
                .split_whitespace()
                .next()?
                .parse()
                .ok()
        };
        let mut seen = None;
        let (mut ok, mut bad) = (0, 0);
        for line in text.lines().filter(|line| line.starts_with("net: tick ")) {
            seen = number(line, "net: tick ");
            ok += number(line, "hash ok ")?;
            bad += number(line, "bad ")?;
        }
        Some((seen?, ok, bad))
    }
}

pub(crate) fn image() -> Option<PathBuf> {
    oag_testdata::image(IMAGE)
}

/// The state lines a driven run wrote, as JSON.
fn states(lab: &Lab, who: &str) -> Vec<serde_json::Value> {
    std::fs::read_to_string(lab.dir.join(format!("{who}-state.jsonl")))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn screen_id(state: &serde_json::Value) -> &str {
    state["screen"]["id"].as_str().unwrap_or_default()
}

/// The game killed mid-race and started again takes REJOIN RACE: it is offered, the seat is
/// taken back, the race is finished by both clients with no bad hash, and the record is gone.
#[test]
#[ignore = "needs a disc image"]
fn a_killed_game_offered_its_race_takes_the_seat_back_and_finishes() {
    let Some(image) = image() else {
        return;
    };
    let lab = Lab::new("accept", image, 20_000);
    let (mut a, mut b) = lab.race();
    a.kill();
    assert!(lab.record("a").exists(), "the killed game left its seat");
    let shot = lab.dir.join("offer.png");
    let mut again = lab.relaunch(
        "a",
        &format!(
            "until screen.id=rejoinrace within 3600; screenshot {}; press cross; \
             until stage=race within 3600; until endrace=photo within 20000",
            shot.display()
        ),
    );
    let status = again.finish(280);
    let log = lab.read("a-again");
    assert!(
        status.is_some_and(|s| s.success()),
        "the relaunch ran to the end:\n{}",
        &log[log.len().saturating_sub(2000)..]
    );
    assert!(shot.exists(), "the offer was drawn");
    let (_, ok, bad) = lab
        .progress("a-again")
        .expect("the rejoined client logged its hashes");
    assert_eq!(bad, 0, "the rejoined client's hashes");
    assert!(ok > 600, "{ok} confirmed ticks checked after the rejoin");
    let _ = b.finish(60);
    let (_, ok_b, bad_b) = lab
        .progress("b")
        .expect("the other client logged its hashes");
    assert_eq!(
        (bad_b, ok_b > 1500),
        (0, true),
        "the other client: ok {ok_b} bad {bad_b}"
    );
    lab.wait("the race to be over", 30, |lab| {
        lab.read("lobby").contains("every person has finished")
    });
    assert!(
        !lab.record("a").exists(),
        "the record is deleted at the end of the race"
    );
}

/// DECLINE deletes the record and goes on to the lobby questions; the seat stays with the away
/// driver, which finishes the race with the other client.
#[test]
#[ignore = "needs a disc image"]
fn a_declined_offer_deletes_the_record_and_leaves_the_seat_to_the_away_driver() {
    let Some(image) = image() else {
        return;
    };
    let lab = Lab::new("decline", image, 20_000);
    let (mut a, mut b) = lab.race();
    a.kill();
    let mut again = lab.relaunch(
        "a",
        "until screen.id=rejoinrace within 3600; press left; press cross; until screen.id=online within 600; ticks 30",
    );
    let status = again.finish(200);
    assert!(
        status.is_some_and(|s| s.success()),
        "the relaunch ran to the end"
    );
    assert!(!lab.record("a").exists(), "DECLINE deleted the record");
    let seen = states(&lab, "a");
    assert!(
        seen.iter().any(|s| screen_id(s) == "rejoinrace"),
        "the offer was made"
    );
    assert_eq!(
        seen.last().map(screen_id),
        Some("online"),
        "the lobby questions follow"
    );
    lab.wait("the away seat to finish the race", 200, |lab| {
        lab.read("lobby").contains("every person has finished")
    });
    let _ = b.finish(60);
    let (_, ok_b, bad_b) = lab
        .progress("b")
        .expect("the other client logged its hashes");
    assert_eq!(
        (bad_b, ok_b > 1500),
        (0, true),
        "the other client: ok {ok_b} bad {bad_b}"
    );
}

/// A launch after the race has ended makes no offer: the record is deleted, the lobby
/// questions open with the reason.
#[test]
#[ignore = "needs a disc image"]
fn a_launch_after_the_race_has_ended_is_not_offered_it() {
    let Some(image) = image() else {
        return;
    };
    let lab = Lab::new("ended", image, 900);
    let (mut a, mut b) = lab.race();
    a.kill();
    lab.wait("the race to stop at its tick limit", 120, |lab| {
        lab.read("lobby").contains("is over")
    });
    let _ = b.finish(30);
    assert!(lab.record("a").exists(), "the killed game left its seat");
    let mut again = lab.relaunch("a", "until screen.id=online within 3600; ticks 30");
    let status = again.finish(200);
    assert!(
        status.is_some_and(|s| s.success()),
        "the relaunch reached the lobby questions"
    );
    let seen = states(&lab, "a");
    assert!(
        seen.iter().all(|s| screen_id(s) != "rejoinrace"),
        "an ended race was offered"
    );
    assert_eq!(seen.last().map(screen_id), Some("online"));
    assert!(!lab.record("a").exists(), "the stale record is deleted");
}
