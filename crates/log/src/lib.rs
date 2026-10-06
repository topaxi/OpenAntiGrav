//! The binaries' log sink: two outputs behind one `log::Log`, each with its own filter.
//!
//! - **The terminal** is `env_logger` exactly as the binaries always built it:
//!   the same default expression, `RUST_LOG` replacing it, level and message
//!   only. Nothing here changes what it prints.
//! - **The file** is appended to, every line carries a sortable UTC timestamp,
//!   its level and its module path, and its filter is its own (more verbose by
//!   default), so a debug line can reach it without reaching the terminal. It
//!   exists because under a launcher that swallows stderr (Steam) the terminal
//!   is not readable at all.
//!
//! **Why a small `Log` rather than `tracing`:** the workspace logs through the
//! `log` facade everywhere, and the two sinks differ only in filter and format.
//! `env_filter` (the parser `env_logger` is built on) gives the file the same
//! `RUST_LOG` syntax for free; `tracing` would add a second facade and a bridge
//! for what this file already does.
//!
//! **Order of use.** [`install`] runs first thing in `main`, before the command
//! line and the settings file exist, because the file's path depends on both.
//! Lines logged before [`attach`] are held in memory and written then; that is
//! how a settings-load warning reaches the file. [`attach`] picks the path,
//! prunes, opens for append and starts the writer.
//!
//! **The frame loop never waits on the disk.** A record is formatted and
//! handed to a writer thread over a bounded channel; when the channel is full
//! the line is dropped and counted, never waited for. The thread does one
//! `write_all` per line on an `O_APPEND` handle, so two processes appending to
//! the same file interleave whole lines, and a crash loses at most what was
//! still in the channel. [`flush`] waits for the channel to drain; the panic
//! hook [`install_panic_hook`] and `main`'s exit call it.

pub mod path;
pub mod prune;
pub mod stamp;
pub mod tool;

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use log::{Level, LevelFilter, Log, Metadata, Record};

/// Lines held while the file's path is not known yet.
const PENDING_CAP: usize = 4096;
/// Lines in flight to the writer before one is dropped.
const QUEUE: usize = 8192;

enum Msg {
    Line(String),
    Flush(SyncSender<()>),
}

enum Sink {
    Pending(Vec<String>),
    Active(SyncSender<Msg>),
    Off,
}

struct State {
    filter: env_filter::Filter,
    sink: Sink,
}

/// The two sinks. Public so a test can drive one directly through [`Log::log`]
/// without touching the process-global logger.
pub struct Tee {
    terminal: env_logger::Logger,
    state: Mutex<State>,
    dropped: Arc<AtomicU64>,
}

impl std::fmt::Debug for Tee {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tee").finish_non_exhaustive()
    }
}

impl Tee {
    /// A tee whose terminal side is `terminal` and whose file side starts on
    /// `file_default`, holding lines until [`Tee::attach`] or [`Tee::detach`].
    #[must_use]
    pub fn new(terminal: env_logger::Logger, file_default: &str) -> Self {
        Self {
            terminal,
            state: Mutex::new(State {
                filter: file_filter(file_default),
                sink: Sink::Pending(Vec::new()),
            }),
            dropped: Arc::new(AtomicU64::new(0)),
        }
    }

    /// The terminal sink as the binaries have always built it.
    #[must_use]
    pub fn terminal(default_filter: &str) -> env_logger::Logger {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_filter))
            .format_timestamp(None)
            .format_target(false)
            .build()
    }

    fn max_level(&self) -> LevelFilter {
        let file = self
            .state
            .lock()
            .map_or(LevelFilter::Trace, |s| s.filter.filter());
        self.terminal.filter().max(file)
    }

    /// Stops holding lines and drops them: no file this run.
    pub fn detach(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.sink = Sink::Off;
        }
    }

    /// Opens `path` for appending (after pruning it as of `now`), switches the
    /// file's filter to `filter` when given, and writes a run separator, what
    /// was held, then `header`.
    ///
    /// # Errors
    /// The directory or file could not be made. The tee is left detached, so a
    /// run with an unwritable log directory logs to the terminal and nothing
    /// else.
    pub fn attach(
        &self,
        file: &Path,
        filter: Option<&str>,
        header: &[String],
        now: SystemTime,
    ) -> std::io::Result<()> {
        let opened = open(file, now);
        let file_handle = match opened {
            Ok(handle) => handle,
            Err(e) => {
                self.detach();
                return Err(e);
            }
        };
        let (tx, rx) = mpsc::sync_channel::<Msg>(QUEUE);
        let dropped = Arc::clone(&self.dropped);
        std::thread::Builder::new()
            .name("oag-log".into())
            .spawn(move || write_loop(file_handle, &rx, &dropped))?;

        let mut state = self.state.lock().expect("log state");
        if let Some(spec) = filter {
            state.filter = file_filter(spec);
        }
        let separator = format!(
            "{} ===== oag run start (pid {}) =====\n",
            stamp::format(now),
            std::process::id()
        );
        let _ = tx.send(Msg::Line(separator));
        if let Sink::Pending(held) = std::mem::replace(&mut state.sink, Sink::Off) {
            for line in held {
                let _ = tx.send(Msg::Line(line));
            }
        }
        for line in header {
            let _ = tx.send(Msg::Line(entry(now, Level::Info, module_path!(), line)));
        }
        state.sink = Sink::Active(tx);
        drop(state);
        log::set_max_level(self.max_level());
        Ok(())
    }

    /// Waits (up to a second) for every line handed over so far to be written.
    pub fn flush_file(&self) {
        let tx = match self.state.lock() {
            Ok(state) => match &state.sink {
                Sink::Active(tx) => tx.clone(),
                _ => return,
            },
            Err(_) => return,
        };
        let (ack, done) = mpsc::sync_channel(1);
        if tx.try_send(Msg::Flush(ack)).is_ok() {
            let _ = done.recv_timeout(Duration::from_secs(1));
        }
    }
}

impl Log for Tee {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.terminal.enabled(metadata)
            || self.state.lock().is_ok_and(|s| s.filter.enabled(metadata))
    }

    fn log(&self, record: &Record) {
        self.terminal.log(record);
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if matches!(state.sink, Sink::Off) || !state.filter.matches(record) {
            return;
        }
        let line = entry(
            SystemTime::now(),
            record.level(),
            record.module_path().unwrap_or(record.target()),
            &record.args().to_string(),
        );
        match &mut state.sink {
            Sink::Pending(held) => {
                if held.len() < PENDING_CAP {
                    held.push(line);
                }
            }
            Sink::Active(tx) => {
                if let Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) =
                    tx.try_send(Msg::Line(line))
                {
                    self.dropped.fetch_add(1, Ordering::Relaxed);
                }
            }
            Sink::Off => {}
        }
    }

    fn flush(&self) {
        self.terminal.flush();
        self.flush_file();
    }
}

fn file_filter(spec: &str) -> env_filter::Filter {
    env_filter::Builder::new().parse(spec).build()
}

/// One whole line: stamp, level, module path, message, newline. A message
/// that spans lines keeps its later lines unstamped, which is what the prune
/// reads as a continuation.
fn entry(at: SystemTime, level: Level, target: &str, message: &str) -> String {
    format!(
        "{} {level:<5} {target}: {}\n",
        stamp::format(at),
        message.trim_end_matches('\n')
    )
}

fn open(path: &Path, now: SystemTime) -> std::io::Result<File> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    // A prune that fails (read-only file, say) must not cost the log.
    let _ = prune::prune_file(path, now);
    OpenOptions::new().create(true).append(true).open(path)
}

fn write_loop(mut file: File, rx: &mpsc::Receiver<Msg>, dropped: &AtomicU64) {
    let mut reported = 0;
    while let Ok(msg) = rx.recv() {
        match msg {
            Msg::Line(line) => {
                let _ = file.write_all(line.as_bytes());
                let lost = dropped.load(Ordering::Relaxed);
                if lost != reported {
                    reported = lost;
                    let note = entry(
                        SystemTime::now(),
                        Level::Warn,
                        module_path!(),
                        &format!("{lost} line(s) dropped so far: the writer fell behind"),
                    );
                    let _ = file.write_all(note.as_bytes());
                }
            }
            Msg::Flush(ack) => {
                let _ = file.flush();
                let _ = ack.send(());
            }
        }
    }
}

static TEE: OnceLock<&'static Tee> = OnceLock::new();

/// Installs the process-global logger: the terminal on `terminal_default`
/// (replaced by `RUST_LOG`), the file side holding lines until [`attach`].
/// Does nothing if a logger is already set.
pub fn install(terminal_default: &str, file_default: &str) {
    let tee: &'static Tee = Box::leak(Box::new(Tee::new(
        Tee::terminal(terminal_default),
        file_default,
    )));
    if log::set_logger(tee).is_ok() {
        let _ = TEE.set(tee);
        log::set_max_level(tee.max_level());
    }
}

/// [`Tee::attach`] on the installed logger, stamped now.
///
/// # Errors
/// See [`Tee::attach`].
pub fn attach(file: &Path, filter: Option<&str>, header: &[String]) -> std::io::Result<()> {
    TEE.get().map_or(Ok(()), |tee| {
        tee.attach(file, filter, header, SystemTime::now())
    })
}

/// [`Tee::detach`] on the installed logger.
pub fn detach() {
    if let Some(tee) = TEE.get() {
        tee.detach();
    }
}

/// Waits for the file to catch up; call before exiting.
pub fn flush() {
    log::logger().flush();
}

/// Logs a panic at `error` and flushes before the previous hook runs, so a
/// crash under a launcher that swallows stderr still leaves its message.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("panic: {info}");
        flush();
        previous(info);
    }));
}

#[cfg(test)]
mod tests;
