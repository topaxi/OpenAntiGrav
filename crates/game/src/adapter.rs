//! Which GPU - or CPU - the game draws with.
//!
//! wgpu picks an adapter for you if you let it, and until this module existed
//! that was the only behaviour: `request_adapter` with everything defaulted. On
//! a machine with one GPU that is the right answer. On a laptop with an iGPU and
//! a dGPU it is whatever the driver felt like, with no way to say otherwise, and
//! there was no way at all to reach a *software* adapter.
//!
//! # There is no CPU renderer to switch on
//!
//! wgpu has no software rasteriser of its own. `wgpu::Backend::Noop` sounds like
//! one and is not - it creates resources and draws nothing. CPU rendering comes
//! entirely from a driver the *system* has installed, which wgpu then enumerates
//! as an ordinary adapter that happens to report [`wgpu::DeviceType::Cpu`]:
//!
//! | Driver | Backend | Where it comes from |
//! | --- | --- | --- |
//! | Mesa lavapipe | Vulkan | Arch `vulkan-swrast`, Debian `mesa-vulkan-drivers` |
//! | WARP | Dx12 | Built into Windows |
//!
//! So this module enumerates and does not synthesise. There is no `software`
//! value that turns CPU rendering on, because no such switch exists: a machine
//! with lavapipe installed is offered it by name, and a machine without it is
//! not offered anything. `force_fallback_adapter` would have made a row that
//! errors on most machines, which is the same mistake
//! [`oag_display::display::WindowMode::ALL`] declines to make about exclusive
//! fullscreen.
//!
//! Expect a software adapter to be a *diagnostic* path rather than a way to
//! play: llvmpipe rendering a full 3D racer is slow enough that it wants
//! `graphics.render_scale` at 50 to be watchable at all.

use anyhow::{Context, Result};
use log::warn;

use oag_display::display::Renderer;

/// The backends an adapter may be offered from.
///
/// `PRIMARY` - Vulkan, Metal, Dx12 - and deliberately not `GL`, which is wgpu's
/// own second tier and which this renderer has never been run on: the device is
/// requested with `DeviceDescriptor::default()`, so `Limits::default()`, which is
/// exactly what GLES routinely cannot meet, and pipeline creation is a second
/// place it could diverge. Listing a `gl:` row would be offering a player
/// something untested, and the CPU path loses nothing by it - lavapipe is a
/// *Vulkan* ICD.
///
/// Used for the instance as well as for enumeration, so `default` cannot resolve
/// to an adapter the RENDERER row does not offer - and **that is a behaviour
/// change worth saying out loud**: `Instance::default()` is `Backends::all()`, so
/// a machine with no Vulkan driver used to fall through to GL and now gets no
/// adapter at all. That is a clearer failure than a silently untested backend,
/// and on Linux the answer is lavapipe rather than GL - see
/// `docs/tools/packaging.md`.
pub const BACKENDS: wgpu::Backends = wgpu::Backends::PRIMARY;

/// An instance restricted to [`BACKENDS`].
///
/// Every entry point makes its instance through here rather than
/// `Instance::default()`, which is `Backends::all()` - otherwise the `default`
/// setting could land on a GL adapter that the RENDERER row does not offer, and
/// the row would be describing a different pool than the game draws from.
#[must_use]
pub fn instance() -> wgpu::Instance {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = BACKENDS;
    wgpu::Instance::new(descriptor)
}

/// What one adapter is called, on the menu and in the settings file.
///
/// **Backend-qualified**, because a driver's own name is only unique within its
/// backend: one card enumerated under both Vulkan and Dx12 reports the same
/// `name` twice. That matters more here than it looks, because
/// [`oag_ui::menu::Menu::seed`] matches the stored string exactly and silently
/// falls back to the first row when it does not match - two identical entries
/// would put a player on an adapter they did not choose and persist it on their
/// first nudge.
///
/// A [`wgpu::DeviceType::Cpu`] adapter is tagged, because "llvmpipe" is not a
/// word that tells a player they are about to render on the CPU.
#[must_use]
pub fn label(backend: wgpu::Backend, name: &str, device_type: wgpu::DeviceType) -> String {
    let suffix = if device_type == wgpu::DeviceType::Cpu {
        " (cpu)"
    } else {
        ""
    };
    format!("{backend}: {name}{suffix}")
}

/// Makes a list of labels unique, keeping the order they were enumerated in.
///
/// Two identical cards in one machine report the same name on the same backend,
/// and [`label`] cannot tell them apart - nothing in `AdapterInfo` reliably can,
/// since `device_pci_bus_id` is only populated on some drivers. Numbering the
/// repeats is not a good name, it is just a name that picks *one* of them; the
/// alternative is a row where two entries are the same string and one of them
/// is unreachable.
///
/// The number is positional, so it is as fragile as an index would have been -
/// which is why it is the fallback for a collision and not the scheme.
fn unique(labels: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(labels.len());
    for label in labels {
        if !out.contains(&label) {
            out.push(label);
            continue;
        }
        // `#2` is the second one seen, so the first keeps its plain name and an
        // existing settings file does not change meaning when a second card
        // arrives.
        let mut nth = 2;
        let numbered = loop {
            let candidate = format!("{label} #{nth}");
            if !out.contains(&candidate) {
                break candidate;
            }
            nth += 1;
        };
        out.push(numbered);
    }
    out
}

/// What each adapter is called, in enumeration order.
#[must_use]
pub fn names(adapters: &[wgpu::Adapter]) -> Vec<String> {
    unique(
        adapters
            .iter()
            .map(|adapter| {
                let info = adapter.get_info();
                label(info.backend, &info.name, info.device_type)
            })
            .collect(),
    )
}

/// The adapter to draw with, and what this machine had to offer.
#[derive(Debug)]
pub struct Chosen {
    /// The one picked.
    pub adapter: wgpu::Adapter,
    /// Every adapter that could have been, in enumeration order.
    ///
    /// Kept because the adapter itself is not worth keeping around for it: the
    /// RENDERER row needs this list every time the menu is seeded, and
    /// enumerating a second time to ask would be a second answer to the same
    /// question. Same reasoning as `Gpu::offered` and the present modes.
    pub offered: Vec<String>,
    /// Every setting value that truthfully describes this choice.
    ///
    /// What the RENDERER row's restart note is measured against - see
    /// [`oag_ui::menu::Restart`] - so the question it answers is "would moving
    /// the row here change anything", not "does the row match the settings
    /// file".
    ///
    /// Usually one name. **Two on the default path**, and that is the case it
    /// exists for: a game that let wgpu pick is on `default` *and* is drawing
    /// with a particular adapter, so a player selecting that same adapter by
    /// name has changed their settings file and nothing else. One entry would
    /// have told them to restart for a picture that is already on screen.
    ///
    /// Empty only when the default path produced an adapter that is not in
    /// `offered` at all, which is [`wgpu::Instance::request_adapter`] reaching
    /// past what [`BACKENDS`] enumerates - nothing to compare against, and
    /// saying nothing beats guessing.
    pub in_use: Vec<String>,
}

/// The adapter `setting` names, or the one wgpu would have picked.
///
/// `surface` is `Some` for the window and `None` for an offscreen capture; an
/// adapter that cannot present to the surface it was given is dropped, so the
/// RENDERER row never offers one that would fail at `get_default_config`.
///
/// A name this machine does not have is a note and the default, not an error, for
/// the reason `choose_monitor` gives about screens: the ordinary way to get one
/// is to uninstall a driver or unplug an eGPU, and refusing to start the game
/// would be punishing a player for their own desk. The note lists what is there,
/// because the next thing anyone wants is the spelling.
///
/// # Errors
///
/// Propagates the case where there is no usable adapter at all, which is the
/// same failure the default path has always had.
pub fn choose(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
    setting: &Renderer,
) -> Result<Chosen> {
    let adapters: Vec<wgpu::Adapter> = pollster::block_on(instance.enumerate_adapters(BACKENDS))
        .into_iter()
        .filter(|adapter| surface.is_none_or(|surface| adapter.is_surface_supported(surface)))
        .collect();
    let offered = names(&adapters);

    if let Some(index) = setting.choose(&offered) {
        // `offered[index]` and not the setting's own spelling: the match is
        // case-insensitive, and this string is compared against a menu row.
        let in_use = vec![offered[index].clone()];
        let adapter = adapters
            .into_iter()
            .nth(index)
            .expect("`Renderer::choose` returns an index into the list it was given");
        return Ok(Chosen {
            adapter,
            offered,
            in_use,
        });
    }
    if let Some(wanted) = setting.name() {
        warn!(
            "no renderer named {wanted:?}; using the default (this machine has: {})",
            offered.join(", ")
        );
    }
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: surface,
        ..Default::default()
    }))
    .context("no suitable GPU adapter (is a Vulkan driver installed?)")?;
    // Which of the enumerated adapters wgpu landed on, so the row can be told
    // that naming it changes nothing. Matched on `AdapterInfo` because that is
    // all an adapter can be compared by; two identical cards are indistinguishable
    // by it, and the first of them is exactly the one `unique` numbered plainly.
    let mut in_use = vec![Renderer::DEFAULT.to_string()];
    let info = adapter.get_info();
    if let Some(index) = adapters
        .iter()
        .position(|candidate| candidate.get_info() == info)
    {
        in_use.push(offered[index].clone());
    }
    Ok(Chosen {
        adapter,
        offered,
        in_use,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::{Backend, DeviceType};

    /// The label is the settings file's own spelling, so what it produces is a
    /// compatibility promise rather than a display detail.
    #[test]
    fn a_label_names_the_backend_and_marks_the_cpu() {
        assert_eq!(
            label(
                Backend::Vulkan,
                "AMD Radeon Graphics (RADV RENOIR)",
                DeviceType::IntegratedGpu
            ),
            "vulkan: AMD Radeon Graphics (RADV RENOIR)"
        );
        assert_eq!(
            label(
                Backend::Vulkan,
                "llvmpipe (LLVM 22.1.8, 256 bits)",
                DeviceType::Cpu
            ),
            "vulkan: llvmpipe (LLVM 22.1.8, 256 bits) (cpu)"
        );
        // A discrete card and a virtual one are both just GPUs to a player;
        // only the CPU is worth calling out, because only it changes what to
        // expect of the frame rate.
        for kind in [
            DeviceType::DiscreteGpu,
            DeviceType::VirtualGpu,
            DeviceType::Other,
        ] {
            assert_eq!(label(Backend::Dx12, "Card", kind), "dx12: Card");
        }
    }

    /// The same card under two backends is the case that actually happens, and
    /// the one that would silently mis-seed the row if the label were bare.
    #[test]
    fn one_card_on_two_backends_is_two_distinguishable_entries() {
        let labels = vec![
            label(Backend::Vulkan, "Card", DeviceType::DiscreteGpu),
            label(Backend::Dx12, "Card", DeviceType::DiscreteGpu),
        ];
        assert_eq!(unique(labels.clone()), labels, "no numbering was needed");
        assert_eq!(labels[0], "vulkan: Card");
        assert_eq!(labels[1], "dx12: Card");
    }

    #[test]
    fn identical_cards_are_numbered_from_the_second() {
        assert_eq!(
            unique(vec![
                "vulkan: Card".to_string(),
                "vulkan: Card".to_string(),
                "vulkan: Card".to_string(),
                "vulkan: Other".to_string(),
            ]),
            [
                "vulkan: Card",
                "vulkan: Card #2",
                "vulkan: Card #3",
                "vulkan: Other",
            ]
        );
    }

    /// The whole point of the numbering: every entry the row offers has to be
    /// reachable, and two equal strings means one of them never is.
    #[test]
    fn every_name_offered_is_distinct() {
        let names = unique(vec![
            "vulkan: Card".to_string(),
            "vulkan: Card".to_string(),
            // A machine that already has a `#2` in a name is not special-cased,
            // it just pushes the generated one along.
            "vulkan: Card #2".to_string(),
            "vulkan: Card".to_string(),
        ]);
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "{names:?} has a duplicate");
    }

    /// `unique` is called on every list, including the ordinary one-GPU case.
    #[test]
    fn a_list_with_nothing_to_disambiguate_is_left_alone() {
        assert_eq!(unique(Vec::new()), Vec::<String>::new());
        let one = vec!["vulkan: Card".to_string()];
        assert_eq!(unique(one.clone()), one);
    }
}
