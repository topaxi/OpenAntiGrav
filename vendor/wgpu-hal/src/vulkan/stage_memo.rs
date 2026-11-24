//! A per-device memo of the SPIR-V that `Device::compile_stage` writes for a
//! [`super::ShaderModule::Intermediate`] module.
//!
//! A module with pipeline-overridable constants stays naga IR until a pipeline
//! is created, and every pipeline then runs `process_overrides` and the SPIR-V
//! writer for each of its stages. Applications that build many pipelines from
//! one module (blend, depth or vertex-layout variants of one material) repeat
//! that work for stages whose SPIR-V is identical. The memo keeps the words of
//! each distinct stage and hands them back; the Vulkan shader module is still
//! created and destroyed per pipeline, exactly as without the memo.
//!
//! Two stages share an entry only when every input to the writer is equal:
//! see [`StageKey`]. Entries of a module are dropped when the module is
//! destroyed, the whole memo is dropped with the device, and the total size is
//! capped by [`MAX_BYTES`].

use alloc::{string::String, sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};

use hashbrown::HashMap;
use parking_lot::Mutex;

/// Upper bound on the SPIR-V the memo holds for one device, in bytes. An insert
/// that would pass it is skipped, so the memo never grows past it.
pub(super) const MAX_BYTES: usize = 32 << 20;

/// Everything that decides the SPIR-V `compile_stage` writes for an
/// intermediate module.
///
/// The device's own writer options are fixed per device and the memo lives on
/// the device. A module's `runtime_checks` and `debug_source` are fixed per
/// module and covered by `module`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct StageKey {
    /// [`StageMemo::next_module_id`] of the module, never an address: a freed
    /// module's address can be reused by a new one.
    pub module: u64,
    pub stage: naga::ShaderStage,
    pub entry_point: String,
    /// Override constants sorted by name, values as `f64::to_bits` (two values
    /// that compare equal but differ in bits, `0.0` and `-0.0`, only miss).
    pub constants: Vec<(String, u64)>,
    pub zero_initialize_workgroup_memory: bool,
    /// The pipeline layout's renumbering of `(group, binding)` to Vulkan bindings.
    pub binding_map: naga::back::spv::BindingMap,
}

impl StageKey {
    pub fn new(
        module: u64,
        stage: &crate::ProgrammableStage<super::ShaderModule>,
        naga_stage: naga::ShaderStage,
        binding_map: &naga::back::spv::BindingMap,
    ) -> Self {
        // Exhaustive, so a field added to `ProgrammableStage` fails to compile
        // here instead of silently missing from the key.
        let crate::ProgrammableStage {
            module: _,
            entry_point,
            constants,
            zero_initialize_workgroup_memory,
        } = *stage;
        let mut constants: Vec<(String, u64)> = constants
            .iter()
            .map(|(name, value)| (name.clone(), value.to_bits()))
            .collect();
        constants.sort_unstable();
        Self {
            module,
            stage: naga_stage,
            entry_point: String::from(entry_point),
            constants,
            zero_initialize_workgroup_memory,
            binding_map: binding_map.clone(),
        }
    }
}

#[derive(Debug, Default)]
struct Entries {
    map: HashMap<StageKey, Arc<[u32]>>,
    bytes: usize,
}

#[derive(Debug, Default)]
pub(super) struct StageMemo {
    next_module_id: AtomicU64,
    entries: Mutex<Entries>,
}

impl StageMemo {
    /// A fresh identity for a new intermediate module.
    pub fn next_module_id(&self) -> u64 {
        self.next_module_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn get(&self, key: &StageKey) -> Option<Arc<[u32]>> {
        self.entries.lock().map.get(key).cloned()
    }

    /// Keeps `spv` for `key` unless that would pass [`MAX_BYTES`].
    pub fn insert(&self, key: StageKey, spv: Arc<[u32]>) {
        let size = size_of_val::<[u32]>(&spv);
        let mut entries = self.entries.lock();
        if entries.bytes + size > MAX_BYTES {
            return;
        }
        // A replaced entry is the same stage written by two threads at once:
        // equal words, so the byte count already holds it.
        if entries.map.insert(key, spv).is_none() {
            entries.bytes += size;
        }
    }

    /// Drops every entry of the module `module`.
    pub fn forget_module(&self, module: u64) {
        let mut entries = self.entries.lock();
        let mut freed = 0;
        entries.map.retain(|key, spv| {
            let keep = key.module != module;
            if !keep {
                freed += size_of_val::<[u32]>(spv);
            }
            keep
        });
        entries.bytes -= freed;
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.lock().map.len()
    }

    #[cfg(test)]
    fn bytes(&self) -> usize {
        self.entries.lock().bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(module: u64, binding: u32) -> StageKey {
        let mut binding_map = naga::back::spv::BindingMap::new();
        binding_map.insert(
            naga::ResourceBinding {
                group: 0,
                binding: 1,
            },
            naga::back::spv::BindingInfo {
                descriptor_set: 0,
                binding,
                binding_array_size: None,
            },
        );
        StageKey {
            module,
            stage: naga::ShaderStage::Fragment,
            entry_point: "fs_main".into(),
            constants: Vec::new(),
            zero_initialize_workgroup_memory: true,
            binding_map,
        }
    }

    #[test]
    fn a_different_binding_map_or_module_is_a_different_entry() {
        let memo = StageMemo::default();
        memo.insert(key(0, 0), Arc::from([1u32, 2].as_slice()));
        assert!(memo.get(&key(0, 1)).is_none());
        assert!(memo.get(&key(1, 0)).is_none());
        assert_eq!(memo.get(&key(0, 0)).as_deref(), Some([1u32, 2].as_slice()));
    }

    #[test]
    fn forgetting_a_module_frees_its_bytes() {
        let memo = StageMemo::default();
        memo.insert(key(0, 0), Arc::from([1u32, 2].as_slice()));
        memo.insert(key(0, 1), Arc::from([3u32].as_slice()));
        memo.insert(key(1, 0), Arc::from([4u32].as_slice()));
        assert_eq!(memo.bytes(), 16);
        memo.forget_module(0);
        assert_eq!((memo.len(), memo.bytes()), (1, 4));
    }

    #[test]
    fn an_insert_past_the_cap_is_skipped() {
        let memo = StageMemo::default();
        let big: Arc<[u32]> = alloc::vec![0u32; MAX_BYTES / 4].into();
        memo.insert(key(0, 0), big);
        memo.insert(key(0, 1), Arc::from([1u32].as_slice()));
        assert!(memo.get(&key(0, 1)).is_none());
        assert_eq!(memo.bytes(), MAX_BYTES);
    }
}
