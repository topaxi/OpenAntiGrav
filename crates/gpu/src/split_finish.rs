//! A frame's command encoder closed part-way and finished on another thread.
//!
//! `CommandEncoder::finish` is where wgpu validates and tracks everything a frame recorded:
//! every pass's usage scope, every bound group's elements, every draw. On the Steam Deck that
//! is about half the frame thread on HD's Anulpha Pass (`docs/rendering/frame-thread-cpu.md`).
//! It runs on one encoder at a time, but two encoders finish independently. So the frame loop
//! opens a [`Frame`], the scene calls [`split`] where a run of passes is complete, and the
//! closed encoder is finished by another thread while this one records the rest. The buffers
//! go to the queue in one submission, in the order they were recorded, so what the GPU runs
//! and every queue write it sees are what one encoder gave it.
//!
//! Opt-in per frame: a [`split`] on a thread with no open [`Frame`] does nothing, so a
//! capture, a test or a split screen's view thread records into its one encoder as before.
//! In a browser there are no threads to hand an encoder to, and a split encoder is finished
//! where it is split.
//!
//! **Only on uniform cores, by default.** On a CPU whose cores differ in capacity (a phone's
//! big.LITTLE, a hybrid desktop part) the split made the frame thread slower: on a Galaxy S24
//! the HD flyby's recording rose by about 1 ms and `submit` did not fall
//! (`docs/rendering/frame-thread-cpu.md`, "The reported phone regression"). So the split is on
//! where every core reports the same `cpu_capacity` (the Steam Deck, a desktop Zen) or where
//! the capacities cannot be read, and off otherwise. `OAG_SPLIT_FINISH=0` or `=1` overrides
//! the probe (an A/B switch for benches, nothing a player sets).

use std::cell::RefCell;

thread_local! {
    /// The open frame's closed encoders, oldest first.
    static OPEN: RefCell<Option<Vec<Closed>>> = const { RefCell::new(None) };
}

/// A closed encoder: finished, or being finished on the worker.
enum Closed {
    Finished(wgpu::CommandBuffer),
    #[cfg(not(target_arch = "wasm32"))]
    Finishing(std::sync::mpsc::Receiver<wgpu::CommandBuffer>),
}

impl Closed {
    fn buffer(self) -> wgpu::CommandBuffer {
        match self {
            Self::Finished(buffer) => buffer,
            #[cfg(not(target_arch = "wasm32"))]
            Self::Finishing(finished) => finished
                .recv()
                .expect("the finishing thread answers every encoder it is sent"),
        }
    }
}

/// A frame whose encoder [`split`] may close part-way; [`Self::submit`] submits it. Dropped
/// unsubmitted, it drops what was split off, as dropping the encoder would.
#[derive(Debug)]
#[must_use = "a frame's split-off work reaches the GPU only through `Frame::submit`"]
pub struct Frame {
    /// Not `Send`: the open frame belongs to the thread that opened it.
    _thread: std::marker::PhantomData<*const ()>,
}

impl Frame {
    /// Opens a frame on this thread when [`wanted`] says this machine gains from the split.
    pub fn begin() -> Self {
        static WANTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *WANTED.get_or_init(wanted) {
            Self::open()
        } else {
            Self {
                _thread: std::marker::PhantomData,
            }
        }
    }

    /// Opens a frame on this thread whatever [`wanted`] says: for a test of what a split
    /// frame draws on a machine that would not split.
    pub fn open() -> Self {
        OPEN.with(|open| *open.borrow_mut() = Some(Vec::new()));
        Self {
            _thread: std::marker::PhantomData,
        }
    }

    /// Finishes `encoder` and submits every buffer split off this frame before it, in order.
    pub fn submit(
        self,
        queue: &wgpu::Queue,
        encoder: wgpu::CommandEncoder,
    ) -> wgpu::SubmissionIndex {
        let closed = OPEN
            .with(|open| open.borrow_mut().take())
            .unwrap_or_default();
        let last = encoder.finish();
        let mut buffers: Vec<wgpu::CommandBuffer> =
            closed.into_iter().map(Closed::buffer).collect();
        buffers.push(last);
        queue.submit(buffers)
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        OPEN.with(|open| open.borrow_mut().take());
    }
}

/// Submits `last`, behind everything this thread's open frame has split off so far, in one
/// submission; with no frame open it is `queue.submit` of `last` alone.
///
/// For a caller that submits part of a frame ahead of [`Frame::submit`] (a split screen's view,
/// whose post work reads the picture its own scene pass drew): the split-off passes go out
/// first, or `last` would run ahead of the passes it follows. The frame stays open.
pub fn submit_with_closed(queue: &wgpu::Queue, last: wgpu::CommandBuffer) -> wgpu::SubmissionIndex {
    let closed = OPEN
        .with(|open| open.borrow_mut().as_mut().map(std::mem::take))
        .unwrap_or_default();
    let mut buffers: Vec<wgpu::CommandBuffer> = closed.into_iter().map(Closed::buffer).collect();
    buffers.push(last);
    queue.submit(buffers)
}

/// Whether frames split their encoder: `OAG_SPLIT_FINISH` when set (`0` off, anything else
/// on), otherwise [`uniform_cores`] over this machine's core capacities.
#[must_use]
pub fn wanted() -> bool {
    match std::env::var("OAG_SPLIT_FINISH") {
        Ok(value) => value != "0",
        Err(_) => uniform_cores(&core_capacities()),
    }
}

/// True when every core has the same capacity, or none could be read.
#[must_use]
pub fn uniform_cores(capacities: &[u32]) -> bool {
    capacities.windows(2).all(|pair| pair[0] == pair[1])
}

/// Each core's scheduler capacity (`/sys/devices/system/cpu/cpu<N>/cpu_capacity`, 1024 for the
/// fastest), in core order; empty where the kernel does not report it or on another OS.
///
/// Empty means unreadable, not uniform: [`uniform_cores`] counts it as uniform for the split,
/// and a caller that needs positive evidence of equal cores checks the list is not empty.
#[must_use]
pub fn core_capacities() -> Vec<u32> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        (0..)
            .map(|core| {
                std::fs::read_to_string(format!("/sys/devices/system/cpu/cpu{core}/cpu_capacity"))
            })
            .take_while(Result::is_ok)
            .filter_map(|text| text.ok()?.trim().parse().ok())
            .collect()
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        Vec::new()
    }
}

/// True on native Linux with at least two cores that all report the same capacity: positive
/// evidence of equal cores, which what has been measured only on such a machine asks for.
#[must_use]
pub fn known_equal_cores() -> bool {
    #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
    {
        equal_cores(&core_capacities())
    }
    #[cfg(not(all(target_os = "linux", not(target_arch = "wasm32"))))]
    {
        false
    }
}

/// A list of at least two capacities, all the same.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn equal_cores(capacities: &[u32]) -> bool {
    capacities.len() > 1 && uniform_cores(capacities)
}

/// Closes `encoder` here and goes on in a fresh one, when this thread has an open [`Frame`];
/// does nothing otherwise. Call it between passes, with none open on `encoder`.
pub fn split(device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder) {
    OPEN.with(|open| {
        let mut open = open.borrow_mut();
        let Some(closed) = open.as_mut() else {
            return;
        };
        let done = std::mem::replace(
            encoder,
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame, continued"),
            }),
        );
        closed.push(finish_elsewhere(done));
    });
}

#[cfg(target_arch = "wasm32")]
fn finish_elsewhere(encoder: wgpu::CommandEncoder) -> Closed {
    Closed::Finished(encoder.finish())
}

/// How many threads finish a frame's closed encoders where [`known_equal_cores`], each encoder
/// on the next: encoders finish independently, and a frame closes its second while the first is
/// still finishing, so one thread would queue it and the frame thread would wait for the pair
/// (`docs/rendering/frame-thread-cpu.md`, "A second finishing thread"). Elsewhere one thread.
/// `OAG_FINISH_THREADS` overrides it (an A/B switch for benches, nothing a player sets).
#[cfg(not(target_arch = "wasm32"))]
const FINISH_THREADS: usize = 2;

/// Sends `encoder` to a finishing thread, started on first use; finishes it here if no thread
/// can be started.
#[cfg(not(target_arch = "wasm32"))]
fn finish_elsewhere(encoder: wgpu::CommandEncoder) -> Closed {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    type Job = (wgpu::CommandEncoder, mpsc::Sender<wgpu::CommandBuffer>);
    static WORKERS: std::sync::OnceLock<Vec<std::sync::Mutex<mpsc::Sender<Job>>>> =
        std::sync::OnceLock::new();
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let workers = WORKERS.get_or_init(|| {
        let threads = std::env::var("OAG_FINISH_THREADS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(if known_equal_cores() {
                FINISH_THREADS
            } else {
                1
            });
        (0..threads)
            .filter_map(|_| {
                let (jobs, inbox) = mpsc::channel::<Job>();
                std::thread::Builder::new()
                    .name("oag-finish".into())
                    .spawn(move || {
                        for (encoder, answer) in inbox {
                            let _ = answer.send(encoder.finish());
                        }
                    })
                    .ok()
                    .map(|_| std::sync::Mutex::new(jobs))
            })
            .collect()
    });
    if workers.is_empty() {
        return Closed::Finished(encoder.finish());
    }
    let jobs = &workers[NEXT.fetch_add(1, Ordering::Relaxed) % workers.len()];
    let (answer, finished) = mpsc::channel();
    let sent = jobs
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .send((encoder, answer));
    match sent {
        Ok(()) => Closed::Finishing(finished),
        Err(mpsc::SendError((encoder, _))) => Closed::Finished(encoder.finish()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_cores_takes_two_or_more_the_same() {
        assert!(equal_cores(&[1024; 8]));
        assert!(!equal_cores(&[]));
        assert!(!equal_cores(&[1024]));
        assert!(!equal_cores(&[253, 1024]));
    }

    fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::default();
        let adapter = crate::test_adapter::request(&instance)?;
        pollster::block_on(adapter.request_device(&Default::default())).ok()
    }

    fn buffer(device: &wgpu::Device, usage: wgpu::BufferUsages, bytes: &[u8]) -> wgpu::Buffer {
        crate::init_buffer::init(device, "split test", usage, bytes)
    }

    /// Three steps recorded into one encoder with two splits between them reach the GPU in
    /// the order they were recorded: a clear, a copy over half of it, a copy out.
    #[test]
    fn the_split_parts_run_in_the_order_they_were_recorded() {
        let Some((device, queue)) = device() else {
            return;
        };
        let target = buffer(
            &device,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            &[9; 8],
        );
        let ones = buffer(&device, wgpu::BufferUsages::COPY_SRC, &[1; 4]);
        let out = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("split test readback"),
            size: 8,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame = Frame::begin();
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.clear_buffer(&target, 0, None);
        split(&device, &mut encoder);
        encoder.copy_buffer_to_buffer(&ones, 0, &target, 4, 4);
        split(&device, &mut encoder);
        encoder.copy_buffer_to_buffer(&target, 0, &out, 0, 8);
        frame.submit(&queue, encoder);
        out.slice(..)
            .map_async(wgpu::MapMode::Read, |r| r.expect("maps"));
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the readback lands");
        let bytes = out
            .slice(..)
            .get_mapped_range()
            .expect("a mapped buffer reads")
            .to_vec();
        assert_eq!(bytes, [0, 0, 0, 0, 1, 1, 1, 1]);
    }

    /// A part submitted ahead of the frame's own submit still runs behind what was split off
    /// before it: a clear, then a copy over half of it submitted by `submit_with_closed`.
    #[test]
    fn a_part_submitted_early_runs_behind_what_was_split_off_before_it() {
        let Some((device, queue)) = device() else {
            return;
        };
        let target = buffer(
            &device,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            &[9; 8],
        );
        let ones = buffer(&device, wgpu::BufferUsages::COPY_SRC, &[1; 4]);
        let out = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("split test readback"),
            size: 8,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame = Frame::open();
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.clear_buffer(&target, 0, None);
        split(&device, &mut encoder);
        encoder.copy_buffer_to_buffer(&ones, 0, &target, 4, 4);
        submit_with_closed(&queue, encoder.finish());
        let mut rest = device.create_command_encoder(&Default::default());
        rest.copy_buffer_to_buffer(&target, 0, &out, 0, 8);
        frame.submit(&queue, rest);
        out.slice(..)
            .map_async(wgpu::MapMode::Read, |r| r.expect("maps"));
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the readback lands");
        let bytes = out
            .slice(..)
            .get_mapped_range()
            .expect("a mapped buffer reads")
            .to_vec();
        assert_eq!(bytes, [0, 0, 0, 0, 1, 1, 1, 1]);
    }

    /// With no frame open `submit_with_closed` is a plain submit.
    #[test]
    fn with_no_open_frame_it_submits_the_one_buffer() {
        let Some((device, queue)) = device() else {
            return;
        };
        let encoder = device.create_command_encoder(&Default::default());
        submit_with_closed(&queue, encoder.finish());
        assert!(OPEN.with(|open| open.borrow().is_none()));
    }

    /// The S24's capacities split nothing; the Deck's and unreadable ones split.
    #[test]
    fn only_uniform_cores_split() {
        let s24 = [253, 253, 253, 253, 653, 653, 653, 730, 730, 1024];
        assert!(!uniform_cores(&s24));
        assert!(uniform_cores(&[1024; 8]));
        assert!(uniform_cores(&[]));
        assert!(uniform_cores(&[512]));
    }

    /// With no frame open on this thread a split leaves the encoder alone.
    #[test]
    fn a_split_with_no_open_frame_does_nothing() {
        let Some((device, _queue)) = device() else {
            return;
        };
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("kept"),
        });
        let before = format!("{encoder:?}");
        split(&device, &mut encoder);
        assert_eq!(format!("{encoder:?}"), before);
        assert!(OPEN.with(|open| open.borrow().is_none()));
    }
}
