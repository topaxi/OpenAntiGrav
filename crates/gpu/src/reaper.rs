//! Finished submissions reclaimed on a thread of their own.
//!
//! `Queue::submit` ends by asking wgpu to clean up every submission the GPU has finished, and
//! that is a real part of the frame: on the Steam Deck dropping a frame's encoders (the command
//! pool reset, a reference count for each resource they used) is about 1 ms of the frame thread
//! (`docs/rendering/frame-thread-cpu.md`, "Reaping off the frame thread"). A submission that a
//! poll has already reclaimed costs the next submit nothing, so this thread polls after a
//! frame's last submit until the queue is empty.
//!
//! It polls and never waits. `Device::poll` holds wgpu's snatch lock for read through a wait on
//! the fence, and presenting takes that lock for write, so a thread parked in a wait would hold
//! the frame thread's present back until the GPU finished.
//!
//! A `map_async` callback fires on the thread that polls, so here. They have to be `Send`.
//!
//! **Only on cores known to be equal** ([`crate::split_finish::known_equal_cores`], the rule
//! threaded recording takes): the work moves to another core, which pays on the Deck and is not
//! measured on a phone's slower ones.
//! `OAG_REAP_THREAD=0` or `=1` overrides that (an A/B switch for benches, nothing a player sets).
//!
//! **Not for a frame of one view** ([`Reaper::frame_submitted`]). A split has no idle moment on
//! the frame thread and gains 3 to 6 %. One view waits on its finishing threads anyway, gains
//! 0.1 ms of `submit`, and pays more than that in `scene` and the next acquire on the Deck
//! (`docs/rendering/frame-thread-cpu.md`, "Reaping off the frame thread").

/// How long the thread leaves the GPU between polls. Long enough to miss the present that
/// follows a submit, which would otherwise wait for this thread's hold on the snatch lock.
#[cfg(not(target_arch = "wasm32"))]
const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(1);

/// The reaping thread; dropping this stops it and joins it. Holds a clone of the device until
/// then.
#[derive(Debug)]
pub struct Reaper {
    #[cfg(not(target_arch = "wasm32"))]
    thread: Option<std::thread::JoinHandle<()>>,
    #[cfg(not(target_arch = "wasm32"))]
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    #[cfg(not(target_arch = "wasm32"))]
    every_frame: bool,
}

impl Reaper {
    /// A reaper for `device` when [`wanted`] says this machine gains from one.
    #[must_use]
    pub fn begin(device: &wgpu::Device) -> Option<Self> {
        wanted().then(|| Self::start(device)).flatten()
    }

    /// A reaper for `device`, whatever [`wanted`] says; `None` if no thread could be started.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub fn start(device: &wgpu::Device) -> Option<Self> {
        use std::sync::atomic::{AtomicBool, Ordering};
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let stopped = std::sync::Arc::clone(&stop);
        let device = device.clone();
        let thread = std::thread::Builder::new()
            .name("oag-reap".into())
            .spawn(move || {
                while !stopped.load(Ordering::Acquire) {
                    std::thread::park();
                    while !stopped.load(Ordering::Acquire) {
                        std::thread::sleep(POLL_INTERVAL);
                        if !matches!(
                            device.poll(wgpu::PollType::Poll),
                            Ok(wgpu::PollStatus::Poll)
                        ) {
                            break;
                        }
                    }
                }
            })
            .ok()?;
        Some(Self {
            thread: Some(thread),
            stop,
            every_frame: std::env::var("OAG_REAP_THREAD").is_ok_and(|value| value != "0"),
        })
    }

    /// No threads in a browser.
    #[cfg(target_arch = "wasm32")]
    #[must_use]
    pub fn start(_device: &wgpu::Device) -> Option<Self> {
        None
    }

    /// A frame's last submission has gone to the queue: reclaim until the queue is empty,
    /// unless the frame drew `one_view` and left it to the next submit, which does it on the
    /// frame thread. `OAG_REAP_THREAD=1` reclaims here after every frame.
    pub fn frame_submitted(&self, one_view: bool) {
        #[cfg(not(target_arch = "wasm32"))]
        if !one_view || self.every_frame {
            self.reap();
        }
        #[cfg(target_arch = "wasm32")]
        let _ = one_view;
    }

    /// Reclaims until the queue is empty.
    pub fn reap(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(thread) = &self.thread {
            thread.thread().unpark();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for Reaper {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
        if let Some(thread) = self.thread.take() {
            thread.thread().unpark();
            let _ = thread.join();
        }
    }
}

/// Whether to reap on a thread: `OAG_REAP_THREAD` when set (`0` off, anything else on),
/// otherwise [`crate::split_finish::known_equal_cores`].
#[must_use]
pub fn wanted() -> bool {
    match std::env::var("OAG_REAP_THREAD") {
        Ok(value) => value != "0",
        Err(_) => crate::split_finish::known_equal_cores(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The map's callback fires with no poll from this thread: the reaper's did it. Dropping
    /// the reaper then returns, so its thread stopped.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_finished_submission_is_reclaimed_with_no_poll_from_the_caller() {
        let instance = wgpu::Instance::default();
        let Some(adapter) = crate::test_adapter::request(&instance) else {
            return;
        };
        let Ok((device, queue)) = pollster::block_on(adapter.request_device(&Default::default()))
        else {
            return;
        };
        let reaper = Reaper::start(&device).expect("a thread starts");
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("reaper test"),
            size: 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&buffer, 0, &[1; 4]);
        queue.submit([]);
        let (mapped, landed) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                mapped.send(result.is_ok()).expect("the test is waiting");
            });
        reaper.reap();
        let waited = landed.recv_timeout(std::time::Duration::from_secs(20));
        assert_eq!(waited, Ok(true));
        drop(reaper);
    }
}
