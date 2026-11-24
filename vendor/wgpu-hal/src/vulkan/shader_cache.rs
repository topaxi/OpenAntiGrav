//! A per-device cache of the SPIR-V that `Device::compile_stage` writes for a
//! [`super::ShaderModule::Intermediate`] module: the Vulkan counterpart of the
//! D3D12 backend's `ShaderCache`.
//!
//! A module with pipeline-overridable constants stays naga IR until a pipeline
//! is created, and every pipeline then runs `process_overrides` and the SPIR-V
//! writer for each of its stages. Applications that build many pipelines from
//! one module (blend, depth or vertex-layout variants of one material) repeat
//! that work for stages whose SPIR-V is identical.
//!
//! On D3D12 the expensive step is the driver-side compiler, so its cache is
//! keyed on naga's output (the HLSL text). On Vulkan the expensive step is naga
//! itself, so this cache is keyed on naga's inputs: see [`ShaderCacheKey`]. The
//! `vk::ShaderModule` is still created and destroyed per pipeline, exactly as
//! without the cache.
//!
//! The bound follows D3D12's (past [`MAX_ENTRIES`], keep the [`RECENT_COMPILES`]
//! most recent), with one change: every lookup, hit or miss, advances the clock.
//! D3D12 advances it only on a compile, so entries refreshed by hits all share
//! one stamp and a prune can evict nothing; here no two entries share a stamp,
//! so a prune always leaves at most [`RECENT_COMPILES`] entries and the cache
//! never holds more than `MAX_ENTRIES + 1`. Because the key names a module by
//! its id, a module's entries are also dropped when it is destroyed.

use alloc::{string::String, sync::Arc, vec::Vec};

use hashbrown::HashMap;

/// Entry count past which the cache prunes (D3D12's value).
pub(super) const MAX_ENTRIES: usize = 200;
/// A pruned cache keeps this many most recently used entries (D3D12's value).
pub(super) const RECENT_COMPILES: u64 = 100;

/// Everything that decides the SPIR-V `compile_stage` writes for an
/// intermediate module.
///
/// The device's own writer options are fixed per device and the cache lives on
/// the device. A module's `runtime_checks` and `debug_source` are fixed per
/// module and covered by `module`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct ShaderCacheKey {
    /// [`ShaderCache::next_module_id`] of the module, never an address: a freed
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

impl ShaderCacheKey {
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

#[derive(Debug)]
pub(super) struct ShaderCacheValue {
    /// [`ShaderCache::clock`] when the entry was last used; unique per entry.
    last_used: u64,
    spv: Arc<[u32]>,
}

#[derive(Debug, Default)]
pub(super) struct ShaderCache {
    /// Advances on every `get` and `insert`.
    clock: u64,
    next_module_id: u64,
    entries: HashMap<ShaderCacheKey, ShaderCacheValue>,
}

impl ShaderCache {
    /// A fresh identity for a new intermediate module.
    pub fn next_module_id(&mut self) -> u64 {
        let id = self.next_module_id;
        self.next_module_id += 1;
        id
    }

    pub fn get(&mut self, key: &ShaderCacheKey) -> Option<Arc<[u32]>> {
        self.clock += 1;
        let now = self.clock;
        let value = self.entries.get_mut(key)?;
        value.last_used = now;
        Some(Arc::clone(&value.spv))
    }

    /// Keeps `spv` for `key`, then prunes past [`MAX_ENTRIES`].
    pub fn insert(&mut self, key: ShaderCacheKey, spv: Arc<[u32]>) {
        self.clock += 1;
        let now = self.clock;
        // A replaced entry is the same stage written by two threads at once.
        self.entries.insert(
            key,
            ShaderCacheValue {
                last_used: now,
                spv,
            },
        );
        if self.entries.len() > MAX_ENTRIES {
            // Stamps are unique, so at most `RECENT_COMPILES` entries are newer
            // than `now - RECENT_COMPILES`.
            let cutoff = now.saturating_sub(RECENT_COMPILES);
            self.entries.retain(|_, v| v.last_used > cutoff);
        }
    }

    /// Drops every entry of the module `module`.
    pub fn forget_module(&mut self, module: u64) {
        self.entries.retain(|key, _| key.module != module);
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(module: u64, binding: u32) -> ShaderCacheKey {
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
        ShaderCacheKey {
            module,
            stage: naga::ShaderStage::Fragment,
            entry_point: "fs_main".into(),
            constants: Vec::new(),
            zero_initialize_workgroup_memory: true,
            binding_map,
        }
    }

    fn words(w: u32) -> Arc<[u32]> {
        Arc::from([w].as_slice())
    }

    #[test]
    fn a_different_binding_map_or_module_is_a_different_entry() {
        let mut cache = ShaderCache::default();
        cache.insert(key(0, 0), words(1));
        assert!(cache.get(&key(0, 1)).is_none());
        assert!(cache.get(&key(1, 0)).is_none());
        assert_eq!(cache.get(&key(0, 0)).as_deref(), Some([1u32].as_slice()));
    }

    #[test]
    fn forgetting_a_module_drops_only_its_entries() {
        let mut cache = ShaderCache::default();
        cache.insert(key(0, 0), words(1));
        cache.insert(key(0, 1), words(2));
        cache.insert(key(1, 0), words(3));
        cache.forget_module(0);
        assert_eq!(cache.len(), 1);
        assert!(cache.get(&key(1, 0)).is_some());
    }

    #[test]
    fn forgetting_a_module_leaves_no_entry_of_it() {
        let mut cache = ShaderCache::default();
        for i in 0..5 {
            cache.insert(key(7, i), words(i));
        }
        cache.insert(key(8, 0), words(9));
        cache.forget_module(7);
        assert!(cache.entries.keys().all(|k| k.module != 7));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn a_prune_keeps_exactly_the_most_recent_entries() {
        let mut cache = ShaderCache::default();
        for i in 0..=MAX_ENTRIES as u32 {
            cache.insert(key(0, i), words(i));
        }
        assert_eq!(cache.len(), RECENT_COMPILES as usize);
        let first_kept = MAX_ENTRIES as u32 + 1 - RECENT_COMPILES as u32;
        assert!(cache.get(&key(0, first_kept - 1)).is_none());
        assert!(cache.get(&key(0, first_kept)).is_some());
    }

    #[test]
    fn entries_refreshed_by_hits_do_not_let_the_cache_grow() {
        let mut cache = ShaderCache::default();
        let mut next = 0u32;
        for _ in 0..10 {
            // Hit every live entry, then miss a hundred times.
            let live: Vec<ShaderCacheKey> = cache.entries.keys().cloned().collect();
            for k in &live {
                assert!(cache.get(k).is_some());
            }
            for _ in 0..RECENT_COMPILES {
                cache.insert(key(1, next), words(next));
                next += 1;
                assert!(cache.len() <= MAX_ENTRIES + 1, "{}", cache.len());
            }
        }
    }

    #[test]
    fn a_recently_hit_entry_survives_a_prune() {
        let mut cache = ShaderCache::default();
        for i in 0..MAX_ENTRIES as u32 {
            cache.insert(key(0, i), words(i));
        }
        assert!(cache.get(&key(0, 0)).is_some());
        cache.insert(key(1, 0), words(0));
        assert_eq!(cache.len(), RECENT_COMPILES as usize);
        assert!(cache.get(&key(0, 0)).is_some());
        assert!(cache.get(&key(0, 1)).is_none());
    }
}
