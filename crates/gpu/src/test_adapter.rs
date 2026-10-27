//! Which adapter a GPU test runs on.
//!
//! A test that asks wgpu for an adapter with no preference gets whichever one
//! the driver lists first. On a machine with an integrated and a discrete GPU
//! that is usually the integrated one, which is slow and, worse, the one that
//! drives the desktop: a test run then stutters the person's screen. Every GPU
//! test picks its adapter through [`request`] (or [`ordered`] where it walks
//! several) instead.
//!
//! The order is: the discrete GPU, then the integrated one, then anything
//! else, with software adapters last. One exception: a discrete GPU from a
//! vendor whose Linux driver does not spill device memory into system memory
//! (NVIDIA) fails a large test with an out-of-memory error where an integrated
//! GPU, which shares system memory, simply runs. On Linux, with such a discrete
//! GPU and an integrated one present, the integrated one goes first.
//!
//! `OAG_TEST_ADAPTER` overrides the choice for a person who knows better:
//!
//! | Value | Effect |
//! | --- | --- |
//! | `discrete` | the first discrete GPU |
//! | `integrated` | the first integrated GPU |
//! | anything else | the first adapter whose name contains it, case-insensitive |
//!
//! An override that matches nothing is reported on stderr and ignored. The
//! adapter the first caller in a process gets is printed once to stderr
//! (visible with `--no-capture`).

use std::sync::Once;

/// Vendor id of the drivers that do not spill into system memory on Linux.
const NO_SPILL_VENDOR: u32 = 0x10de;

/// Every adapter on the primary backends, best first by the rules above.
#[must_use]
pub fn ordered(instance: &wgpu::Instance) -> Vec<wgpu::Adapter> {
    let mut adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY));
    if adapters.is_empty() {
        adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
    }
    let infos: Vec<Candidate> = adapters
        .iter()
        .map(|adapter| {
            let info = adapter.get_info();
            Candidate {
                name: info.name,
                vendor: info.vendor,
                device_type: info.device_type,
                primary: wgpu::Backends::PRIMARY.contains(info.backend.into()),
            }
        })
        .collect();
    let order = rank(&infos, std::env::var("OAG_TEST_ADAPTER").ok().as_deref());
    let mut slots: Vec<Option<wgpu::Adapter>> = adapters.into_iter().map(Some).collect();
    let ordered: Vec<wgpu::Adapter> = order.into_iter().filter_map(|i| slots[i].take()).collect();
    static ANNOUNCE: Once = Once::new();
    ANNOUNCE.call_once(|| {
        if let Some(first) = ordered.first() {
            let info = first.get_info();
            eprintln!(
                "oag-gpu test adapter: {} ({:?}, {:?})",
                info.name, info.device_type, info.backend
            );
        }
    });
    ordered
}

/// The adapter a test should run on, or `None` when the machine has none (the
/// test then skips, as it always has).
#[must_use]
pub fn request(instance: &wgpu::Instance) -> Option<wgpu::Adapter> {
    ordered(instance).into_iter().next()
}

/// The parts of an adapter's description the choice reads.
struct Candidate {
    name: String,
    vendor: u32,
    device_type: wgpu::DeviceType,
    primary: bool,
}

/// Indices into `infos`, best first. Pure, so the policy is testable without a GPU.
fn rank(infos: &[Candidate], overridden: Option<&str>) -> Vec<usize> {
    let class = |info: &Candidate| match info.device_type {
        wgpu::DeviceType::DiscreteGpu => 0,
        wgpu::DeviceType::IntegratedGpu => 1,
        wgpu::DeviceType::VirtualGpu => 2,
        wgpu::DeviceType::Other => 3,
        wgpu::DeviceType::Cpu => 4,
    };
    let primary = |info: &Candidate| u8::from(!info.primary);
    let mut order: Vec<usize> = (0..infos.len()).collect();
    order.sort_by_key(|&i| (class(&infos[i]), primary(&infos[i])));
    let spills_badly = order.first().is_some_and(|&i| {
        infos[i].vendor == NO_SPILL_VENDOR && infos[i].device_type == wgpu::DeviceType::DiscreteGpu
    });
    let integrated = order
        .iter()
        .position(|&i| infos[i].device_type == wgpu::DeviceType::IntegratedGpu);
    if let (true, true, Some(pos)) = (cfg!(target_os = "linux"), spills_badly, integrated) {
        let integrated = order.remove(pos);
        order.insert(0, integrated);
    }
    if let Some(wanted) = overridden {
        let wanted_lower = wanted.to_lowercase();
        let hit = order.iter().position(|&i| match wanted_lower.as_str() {
            "discrete" => infos[i].device_type == wgpu::DeviceType::DiscreteGpu,
            "integrated" => infos[i].device_type == wgpu::DeviceType::IntegratedGpu,
            name => infos[i].name.to_lowercase().contains(name),
        });
        if let Some(pos) = hit {
            let chosen = order.remove(pos);
            order.insert(0, chosen);
        } else {
            eprintln!("OAG_TEST_ADAPTER={wanted:?} matches no adapter; using the default order");
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(name: &str, vendor: u32, device_type: wgpu::DeviceType) -> Candidate {
        Candidate {
            name: name.into(),
            vendor,
            device_type,
            primary: true,
        }
    }

    #[test]
    fn the_discrete_gpu_beats_the_integrated_one_listed_first() {
        let infos = [
            info("igpu", 0x1002, wgpu::DeviceType::IntegratedGpu),
            info("dgpu", 0x1002, wgpu::DeviceType::DiscreteGpu),
            info("soft", 0, wgpu::DeviceType::Cpu),
        ];
        assert_eq!(rank(&infos, None), [1, 0, 2]);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn a_discrete_gpu_that_cannot_spill_yields_to_the_integrated_one() {
        let infos = [
            info("dgpu", NO_SPILL_VENDOR, wgpu::DeviceType::DiscreteGpu),
            info("igpu", 0x8086, wgpu::DeviceType::IntegratedGpu),
        ];
        assert_eq!(rank(&infos, None), [1, 0]);
        let alone = [info("dgpu", NO_SPILL_VENDOR, wgpu::DeviceType::DiscreteGpu)];
        assert_eq!(rank(&alone, None), [0]);
    }

    #[test]
    fn the_override_picks_by_class_or_by_name() {
        let infos = [
            info("Alpha dGPU", 0x1002, wgpu::DeviceType::DiscreteGpu),
            info("Beta iGPU", 0x1002, wgpu::DeviceType::IntegratedGpu),
        ];
        assert_eq!(rank(&infos, Some("integrated"))[0], 1);
        assert_eq!(rank(&infos, Some("DISCRETE"))[0], 0);
        assert_eq!(rank(&infos, Some("beta"))[0], 1);
        assert_eq!(rank(&infos, Some("nothing like it")), [0, 1]);
    }
}
