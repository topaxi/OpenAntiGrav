//! A frame's command encoder closed part-way and finished on another thread.
//!
//! `CommandEncoder::finish` is where wgpu validates and tracks everything a frame recorded:
//! every pass's usage scope, every bound group's elements, every draw. On the Steam Deck that
//! is about half the frame thread on HD's Anulpha Pass (`docs/rendering/frame-thread-cpu.md`).
//! It runs on one encoder at a time, but two encoders finish independently. So the frame loop
//! opens a [`Frame`], the scene calls [`split`] where a run of passes is complete, and the
//! closed encoder is finished by a worker thread while this one records the rest. The buffers
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
        let wanted = *WANTED.get_or_init(wanted);
        if wanted {
            OPEN.with(|open| *open.borrow_mut() = Some(Vec::new()));
        }
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
fn core_capacities() -> Vec<u32> {
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

/// Sends `encoder` to the finishing thread, started on first use; finishes it here if the
/// thread cannot be started.
#[cfg(not(target_arch = "wasm32"))]
fn finish_elsewhere(encoder: wgpu::CommandEncoder) -> Closed {
    use std::sync::mpsc;
    type Job = (wgpu::CommandEncoder, mpsc::Sender<wgpu::CommandBuffer>);
    static WORKER: std::sync::OnceLock<Option<std::sync::Mutex<mpsc::Sender<Job>>>> =
        std::sync::OnceLock::new();
    let worker = WORKER.get_or_init(|| {
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
    });
    let Some(jobs) = worker else {
        return Closed::Finished(encoder.finish());
    };
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
