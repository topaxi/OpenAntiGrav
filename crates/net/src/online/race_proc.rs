//! The one game slot's race: brought up behind a seam ([`Runner`]), as a child process
//! (`oag-server race`, [`ChildRunner`]) or on a thread ([`ThreadRunner`], what tests use).
//!
//! The race itself is [`run_race`], the same loop in both: seat the people the lobby named
//! (by the token each was given), wait for every one to load, then [`crate::serve::run`].
//! A person who never arrives is a timeout that ends the race with a reason, not a slot
//! held for ever.

use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use web_time::{Duration, Instant};

use crate::host::Host;
use crate::serve::Pace;
use crate::wire::online::RaceJob;
use crate::wt::{Identity, Listener};

/// Where a brought-up race is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Loading; not accepting yet.
    Starting,
    /// Accepting its people on `port`, under the certificate with SHA-256 `cert_hash`.
    Up {
        /// The UDP port.
        port: u16,
        /// The certificate's SHA-256.
        cert_hash: [u8; 32],
    },
    /// Over, and why: finished, crashed, timed out.
    Over(String),
}

/// A race that was brought up. Dropping it ends the race.
pub trait Handle: Send {
    /// Where the race is now.
    fn poll(&mut self) -> Status;
}

/// Brings up a race for a [`RaceJob`].
pub trait Runner: Send {
    /// Starts the race loading.
    ///
    /// # Errors
    ///
    /// The race could not even be started (no image for the title, the program missing).
    fn launch(&mut self, job: &RaceJob) -> Result<Box<dyn Handle>, String>;
}

/// How a race process is bounded.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// How long the people have to arrive and load once the race is up. Chosen.
    pub join_timeout: Duration,
    /// Stop after this many ticks even if nobody finished: 30 minutes. Chosen.
    pub max_ticks: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            join_timeout: Duration::from_secs(60),
            max_ticks: 60 * 60 * 30,
        }
    }
}

/// Seats the people in `seats` by token, waits for all of them, runs the race to its end,
/// and says why it ended.
///
/// # Errors
///
/// Somebody did not arrive and load within `limits.join_timeout`, or `stop` came first.
pub fn run_race(
    host: &mut Host,
    listener: &mut Listener,
    seats: &[(String, String)],
    limits: Limits,
    stop: &dyn Fn() -> bool,
    say: &mut dyn FnMut(&str),
) -> Result<String, String> {
    host.expect_seats(seats.to_vec());
    let asked = Instant::now();
    while !host.all_ready() {
        if stop() {
            return Err("the lobby stopped the race".into());
        }
        if asked.elapsed() > limits.join_timeout {
            let missing: Vec<&str> = host
                .seat_names()
                .iter()
                .zip(seats)
                .filter(|(joined, _)| joined.is_none())
                .map(|(_, (name, _))| name.as_str())
                .collect();
            return Err(if missing.is_empty() {
                format!(
                    "everyone connected but not everyone loaded within {}s",
                    limits.join_timeout.as_secs()
                )
            } else {
                format!(
                    "timed out after {}s waiting for {}",
                    limits.join_timeout.as_secs(),
                    missing.join(", ")
                )
            });
        }
        while let Some(link) = listener.accept() {
            say(&format!("connection from {}", link.peer()));
            host.join(Box::new(link));
        }
        host.poll();
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut ended = String::from("the race ended");
    let pace = Pace {
        max_ticks: Some(limits.max_ticks),
        ..Pace::default()
    };
    crate::serve::run(
        host,
        pace,
        &mut || {
            listener
                .accept()
                .map(|link| Box::new(link) as Box<dyn crate::link::Link>)
        },
        stop,
        &mut |line| {
            // The once-a-second progress line also starts with `tick N`; it has a hash in it.
            if line.starts_with("tick ")
                && !line.contains(" hash ")
                && let Some((_, why)) = line.split_once(": ")
            {
                ended = why.to_string();
            }
            say(line);
        },
    );
    Ok(ended)
}

/// Builds the race for a job's params.
pub type Loader =
    Arc<dyn Fn(&crate::wire::RaceParams) -> anyhow::Result<Box<oag_raceplay::Race>> + Send + Sync>;

/// A race on a thread of this process, loading with `load`: what the tests run, and a
/// runner for a server that does not want a child.
pub struct ThreadRunner {
    /// Builds the race for a job's params.
    pub load: Loader,
    /// Where the race listens (port 0: any).
    pub listen: SocketAddr,
    /// The bounds.
    pub limits: Limits,
}

impl std::fmt::Debug for ThreadRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThreadRunner")
            .field("listen", &self.listen)
            .finish_non_exhaustive()
    }
}

struct ThreadHandle {
    status: mpsc::Receiver<Status>,
    last: Status,
    stop: Arc<AtomicBool>,
}

impl Handle for ThreadHandle {
    fn poll(&mut self) -> Status {
        while let Ok(status) = self.status.try_recv() {
            self.last = status;
        }
        self.last.clone()
    }
}

impl Drop for ThreadHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Runner for ThreadRunner {
    fn launch(&mut self, job: &RaceJob) -> Result<Box<dyn Handle>, String> {
        let (tx, status) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let (load, listen, limits, job) =
            (self.load.clone(), self.listen, self.limits, job.clone());
        let flag = stop.clone();
        std::thread::Builder::new()
            .name("oag-race".into())
            .spawn(move || {
                let outcome = (|| -> Result<String, String> {
                    let race = load(&job.params).map_err(|e| format!("{e:#}"))?;
                    let mut host = Host::new(race, job.params.clone());
                    let identity = Identity::self_signed(&[]).map_err(|e| format!("{e:#}"))?;
                    let mut listener =
                        Listener::bind(listen, &identity).map_err(|e| format!("{e:#}"))?;
                    let _ = tx.send(Status::Up {
                        port: listener.local_addr().port(),
                        cert_hash: identity.hash(),
                    });
                    run_race(
                        &mut host,
                        &mut listener,
                        &job.seats,
                        limits,
                        &|| flag.load(Ordering::Relaxed),
                        &mut |line| log::debug!("race: {line}"),
                    )
                })();
                let _ = tx.send(Status::Over(match outcome {
                    Ok(why) | Err(why) => why,
                }));
            })
            .map_err(|e| format!("starting the race thread: {e}"))?;
        Ok(Box::new(ThreadHandle {
            status,
            last: Status::Starting,
            stop,
        }))
    }
}

/// A race as a child process: `oag-server race`, which reads its job off standard input,
/// prints `listening on ADDR` and `cert-hash HEX` once it accepts, and exits when the race
/// is over (status 0) or fails (non-zero, with the reason on stderr).
#[derive(Debug)]
pub struct ChildRunner {
    /// The program to run: this binary, `std::env::current_exe`.
    pub program: PathBuf,
    /// The disc image or folder for each title, by `Title::name`.
    pub images: Vec<(String, String)>,
    /// Where the child listens (the race port; 0 for any).
    pub listen: SocketAddr,
    /// The bounds, passed down.
    pub limits: Limits,
    /// How long the child may take to start accepting (loading a race). Chosen.
    pub load_timeout: Duration,
}

struct ChildHandle {
    child: Child,
    /// The child's standard input, held open for the whole race: the child treats its end
    /// of file as "the lobby is gone" and stops, so a lobby that dies for any reason
    /// (SIGKILL included, when the kernel closes the pipe) does not orphan its race.
    _stdin: Option<std::process::ChildStdin>,
    /// The threads draining the child's two pipes; each ends at the pipe's end of file.
    readers: Vec<std::thread::JoinHandle<()>>,
    lines: mpsc::Receiver<String>,
    port: Option<u16>,
    hash: Option<[u8; 32]>,
    started: Instant,
    load_timeout: Duration,
    /// The child's last lines of output, for a failure's reason.
    tail: Arc<Mutex<Vec<String>>>,
    over: Option<String>,
}

impl ChildHandle {
    fn reason(&self, status: &std::process::ExitStatus) -> String {
        let tail = self.tail.lock().map(|t| t.clone()).unwrap_or_default();
        let last = |prefix: &str| {
            tail.iter()
                .rev()
                .find_map(|l| l.strip_prefix(prefix).map(str::to_string))
        };
        if status.success() {
            last("race-over ").unwrap_or_else(|| "the race ended".into())
        } else {
            let detail = last("error: ").unwrap_or_default();
            format!("the race process exited with {status}: {detail}")
        }
    }
}

impl Handle for ChildHandle {
    fn poll(&mut self) -> Status {
        if let Some(why) = &self.over {
            return Status::Over(why.clone());
        }
        while let Ok(line) = self.lines.try_recv() {
            if let Some(addr) = line.strip_prefix("listening on ") {
                self.port = addr.trim().parse::<SocketAddr>().ok().map(|a| a.port());
            } else if let Some(hex) = line.strip_prefix("cert-hash ") {
                self.hash = crate::wt::parse_hash(hex.trim()).ok();
            }
        }
        match self.child.try_wait() {
            Ok(Some(status)) => {
                // Lines still in the pipe when it exited are in the readers' hands: wait for
                // their end of file (bounded, a grandchild may hold a pipe) so the reason is
                // the child's own, whatever the machine's load.
                let waited = Instant::now();
                while self.readers.iter().any(|r| !r.is_finished())
                    && waited.elapsed() < Duration::from_secs(2)
                {
                    std::thread::sleep(Duration::from_millis(5));
                }
                let why = self.reason(&status);
                self.over = Some(why.clone());
                return Status::Over(why);
            }
            Ok(None) => {}
            Err(e) => {
                let why = format!("lost the race process: {e}");
                self.over = Some(why.clone());
                return Status::Over(why);
            }
        }
        if let (Some(port), Some(cert_hash)) = (self.port, self.hash) {
            return Status::Up { port, cert_hash };
        }
        if self.started.elapsed() > self.load_timeout {
            let _ = self.child.kill();
            let why = format!(
                "the race did not come up within {}s",
                self.load_timeout.as_secs()
            );
            self.over = Some(why.clone());
            return Status::Over(why);
        }
        Status::Starting
    }
}

impl Drop for ChildHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Runner for ChildRunner {
    fn launch(&mut self, job: &RaceJob) -> Result<Box<dyn Handle>, String> {
        let image = self
            .images
            .iter()
            .find(|(title, _)| *title == job.params.title)
            .map(|(_, path)| path.clone())
            .ok_or_else(|| format!("no image for {}", job.params.title))?;
        let mut child = Command::new(&self.program)
            .arg("race")
            .args(["--image", &image])
            .args(["--listen", &self.listen.to_string()])
            .args([
                "--join-timeout",
                &self.limits.join_timeout.as_secs().to_string(),
            ])
            .args(["--max-ticks", &self.limits.max_ticks.to_string()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("starting {}: {e}", self.program.display()))?;
        let mut stdin = child.stdin.take();
        if let Some(stdin) = stdin.as_mut() {
            let _ = writeln!(stdin, "{}", job.to_line());
            let _ = stdin.flush();
        }
        // Both pipes are drained for the whole race: a full pipe would block the child
        // and look like a frozen race.
        let (tx, lines) = mpsc::channel();
        let tail = Arc::new(Mutex::new(Vec::new()));
        let mut readers = Vec::new();
        for pipe in [
            child
                .stdout
                .take()
                .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
            child
                .stderr
                .take()
                .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
        ]
        .into_iter()
        .flatten()
        {
            let (tx, tail) = (tx.clone(), tail.clone());
            readers.push(std::thread::spawn(move || {
                for line in std::io::BufReader::new(pipe).lines().map_while(Result::ok) {
                    log::debug!("race process: {line}");
                    if let Ok(mut tail) = tail.lock() {
                        tail.push(line.clone());
                        let excess = tail.len().saturating_sub(16);
                        tail.drain(..excess);
                    }
                    if tx.send(line).is_err() {
                        break;
                    }
                }
            }));
        }
        Ok(Box::new(ChildHandle {
            child,
            _stdin: stdin,
            readers,
            lines,
            port: None,
            hash: None,
            started: Instant::now(),
            load_timeout: self.load_timeout,
            tail,
            over: None,
        }))
    }
}
