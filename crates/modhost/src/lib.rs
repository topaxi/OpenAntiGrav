//! The WebAssembly extension host: one simulation module, called once per race tick.
//!
//! What it is for and what it refuses: `docs/architecture/wasm-extensions/README.md`
//! (sections 2.1 and 4), the engine side in `hooks.md` and `determinism-and-state.md`
//! beside it. This crate knows no game and no title. It compiles a module, checks it,
//! and runs its `tick` export on a fresh pooled instance under a fuel budget, with the
//! observation and the reply crossing in one I/O buffer, laid out by `oag_ext_abi`.
//!
//! - **Lazy**: no wasmtime [`Engine`] exists until the first [`Module::compile`];
//!   [`engines_created`] counts them so a test can prove a vanilla race built none.
//! - **State is the caller's**: the module's blob is passed in and handed back; the
//!   guest's own memory never survives a tick (a fresh instance per call).
//! - **One answer for every failure**: a trap (fuel, `unreachable`, out of bounds) or a
//!   reply outside its bounds is a [`Fault`]; the caller turns the module off at that
//!   tick and the race continues without it. Nothing is clamped.
//!
//! Wall time is not measured here (`docs/architecture/determinism.md`): the caller
//! times the call if it wants to show it.

// Cranelift only: the web build runs modules on the browser's own engine and iOS needs
// Pulley (no JIT), neither built yet (README, section 5). `oag-game` builds both with
// `--no-default-features`.
#[cfg(any(target_arch = "wasm32", target_os = "ios"))]
compile_error!(
    "oag-modhost runs modules on wasmtime's Cranelift, which the web and iOS builds cannot use; build oag-game with --no-default-features there"
);

use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use wasmtime::{
    Config, Engine, ExternType, InstancePre, Linker, Memory, Store, StoreLimits,
    StoreLimitsBuilder, Trap,
};

use oag_ext_abi::track::TRACK_LEN;
use oag_ext_abi::{
    ABI, ABI_WORD, Command, Craft, Event, IO_LEN, MAX_MEMORY_PAGES, MAX_STATE_BYTES, REVISION,
};

mod reply;

/// Fuel for one tick's call, instantiation included. Chosen: Wall Penalty uses a few
/// thousand units on a full grid; fuel is checked at function entries and loop
/// headers, so this bounds loops and calls, not straight-line length
/// (`determinism-and-state.md`, "Fuel, not time").
pub const FUEL_PER_TICK: u64 = 200_000;

/// How many instances may be live at once across the process. One per running module
/// call; a race has at most a handful. Chosen.
const POOL_INSTANCES: u32 = 64;

const PAGE: usize = 64 * 1024;

static ENGINE: OnceLock<Engine> = OnceLock::new();
static ENGINES_CREATED: AtomicUsize = AtomicUsize::new(0);

/// How many wasmtime engines this process has built: 0 until a module is compiled,
/// 1 after. For the proof that a race with no module builds no engine (README 4.5).
#[must_use]
pub fn engines_created() -> usize {
    ENGINES_CREATED.load(Ordering::SeqCst)
}

/// The one engine, built on first use with every setting the README's table 4.1 pins.
fn engine() -> Result<&'static Engine, LoadError> {
    if let Some(engine) = ENGINE.get() {
        return Ok(engine);
    }
    let mut pool = wasmtime::PoolingAllocationConfig::default();
    pool.total_memories(POOL_INSTANCES)
        .total_tables(POOL_INSTANCES)
        .total_core_instances(POOL_INSTANCES)
        .max_memories_per_module(1)
        .max_tables_per_module(1)
        .max_memory_size(MAX_MEMORY_PAGES as usize * PAGE);
    let mut config = Config::new();
    config
        .allocation_strategy(wasmtime::InstanceAllocationStrategy::Pooling(pool))
        .consume_fuel(true)
        .cranelift_nan_canonicalization(true)
        .wasm_simd(false)
        .wasm_relaxed_simd(false)
        .wasm_multi_memory(false)
        .wasm_memory64(false);
    // No `threads` cargo feature: a module declaring shared memory does not compile.
    let built = Engine::new(&config).map_err(|e| LoadError::Engine(e.to_string()))?;
    let engine = ENGINE.get_or_init(|| {
        ENGINES_CREATED.fetch_add(1, Ordering::SeqCst);
        built
    });
    Ok(engine)
}

/// Why a module was left out of a race. Each says what to fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// wasmtime refused the configuration.
    Engine(String),
    /// Not a valid module under the deterministic subset (SIMD, threads, ...).
    Compile(String),
    /// An import outside the `oag` namespace, or one the host does not provide.
    Import(String),
    /// A required export is missing or has the wrong type.
    Export(String),
    /// `oag_abi` returned another major or layout revision; `revision` is `None` for
    /// a module built before revisions existed.
    Abi {
        /// The major the module was built for.
        major: u32,
        /// The layout revision it was built for.
        revision: Option<u32>,
    },
    /// The module's `oag_state_len` export disagrees with the manifest's `state_bytes`.
    State {
        /// What the module exports.
        module: u32,
        /// What the manifest declares.
        manifest: u32,
    },
    /// The declared limits are out of range.
    Limits(String),
    /// Instantiating or the first call trapped.
    Start(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Engine(e) => write!(f, "the wasm engine: {e}"),
            Self::Compile(e) => write!(f, "does not compile: {e}"),
            Self::Import(e) => write!(f, "imports {e}; only the `oag` namespace exists"),
            Self::Export(e) => write!(f, "export {e}"),
            Self::Abi {
                major,
                revision: Some(revision),
            } => write!(
                f,
                "built for ABI {major} rev {revision}, this host is ABI {ABI} rev {REVISION}"
            ),
            Self::Abi {
                major,
                revision: None,
            } => write!(
                f,
                "built for ABI {major} without a layout revision, this host is ABI {ABI} rev {REVISION}"
            ),
            Self::State { module, manifest } => write!(
                f,
                "the module's state is {module} bytes, its manifest declares state_bytes = {manifest}"
            ),
            Self::Limits(e) => write!(f, "{e}"),
            Self::Start(e) => write!(f, "traps on start: {e}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// What a module declares about itself (`[extension]` in `oag-mod.toml`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The guest's maximum linear memory, 64 KiB pages, at most [`MAX_MEMORY_PAGES`].
    pub memory_pages: u32,
    /// The state blob's length, at most [`MAX_STATE_BYTES`].
    pub state_bytes: u32,
    /// Fuel per tick.
    pub fuel: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            memory_pages: 16,
            state_bytes: 64,
            fuel: FUEL_PER_TICK,
        }
    }
}

struct HostData {
    limits: StoreLimits,
}

/// A compiled, checked module, ready to run a tick. Cheap to clone.
#[derive(Clone)]
pub struct Module {
    pre: InstancePre<HostData>,
    limits: Limits,
}

impl std::fmt::Debug for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Module")
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

/// Why a tick's call failed. The module is off from this tick on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// The fuel budget ran out.
    OutOfFuel,
    /// Any other trap: `unreachable`, out-of-bounds memory, stack overflow, a grow limit.
    Trap(String),
    /// The reply broke a rule: a bad command, a NaN, an out-of-range factor, too many.
    Reply(String),
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfFuel => write!(f, "out of fuel"),
            Self::Trap(e) => write!(f, "trapped: {e}"),
            Self::Reply(e) => write!(f, "refused reply: {e}"),
        }
    }
}

/// One tick's call, as the caller hands it in.
#[derive(Debug, Clone, Copy)]
pub struct Observation<'a> {
    /// The world tick.
    pub tick: u64,
    /// One bit per grid slot that holds a craft.
    pub active: u32,
    /// What happened since the last call, in engine order.
    pub events: &'a [Event],
    /// Each grid slot, in slot order.
    pub crafts: &'a [Craft],
    /// The track region (`oag_ext_abi::track`), as many bytes of it as were used: the
    /// same every tick of a race. Written only to a guest exporting `oag_track`.
    pub track: &'a [u8],
    /// The track's speed pads: the bound of a race-wide `scale-pad`'s pad.
    pub speed_pads: usize,
    /// The track's routes: the bound of a `prefer-route`'s route.
    pub routes: usize,
}

/// What a call cost, for the dev overlay. An output, never simulation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cost {
    /// Fuel used by this call, instantiation included.
    pub fuel_used: u64,
    /// The budget it ran under.
    pub fuel_budget: u64,
}

/// Refuses an export that is not `() -> i32`, the shape of every function the ABI names,
/// so a wrong signature is a load refusal by name and never a mid-race fault.
fn returns_a_word(name: &str, func: &wasmtime::FuncType) -> Result<(), LoadError> {
    let word = func.params().len() == 0
        && func.results().len() == 1
        && matches!(func.results().next(), Some(wasmtime::ValType::I32));
    if word {
        Ok(())
    } else {
        Err(LoadError::Export(format!(
            "`{name}` must be `() -> i32`, it has another signature"
        )))
    }
}

impl Module {
    /// Compiles and checks `wasm`. Builds the process's engine on the first call.
    ///
    /// # Errors
    ///
    /// Anything in [`LoadError`].
    pub fn compile(wasm: &[u8], limits: Limits) -> Result<Self, LoadError> {
        if limits.memory_pages == 0 || limits.memory_pages > MAX_MEMORY_PAGES {
            return Err(LoadError::Limits(format!(
                "memory_pages {} is not in 1..={MAX_MEMORY_PAGES}",
                limits.memory_pages
            )));
        }
        if limits.state_bytes as usize > MAX_STATE_BYTES {
            return Err(LoadError::Limits(format!(
                "state_bytes {} is over {MAX_STATE_BYTES}",
                limits.state_bytes
            )));
        }
        let engine = engine()?;
        let module = wasmtime::Module::new(engine, wasm)
            .map_err(|e| LoadError::Compile(format!("{e:#}")))?;
        // The linker holds the `oag` namespace, which is empty in ABI 1: any import at
        // all is refused by name.
        if let Some(import) = module.imports().next() {
            return Err(LoadError::Import(format!(
                "`{}.{}`",
                import.module(),
                import.name()
            )));
        }
        for (name, want_func) in [
            ("memory", false),
            ("oag_abi", true),
            ("oag_io", true),
            ("tick", true),
        ] {
            match (module.get_export(name), want_func) {
                (Some(ExternType::Memory(_)), false) => {}
                (Some(ExternType::Func(func)), true) => returns_a_word(name, &func)?,
                (Some(_), _) => {
                    return Err(LoadError::Export(format!("`{name}` has the wrong kind")));
                }
                (None, _) => return Err(LoadError::Export(format!("`{name}` is missing"))),
            }
        }
        let linker: Linker<HostData> = Linker::new(engine);
        let pre = linker
            .instantiate_pre(&module)
            .map_err(|e| LoadError::Start(format!("{e:#}")))?;
        let loaded = Self { pre, limits };
        // A first instance at load, so a module that cannot start (too much memory, a
        // trapping start function, the wrong ABI) is refused here and never mid-race.
        let (mut store, instance) = loaded
            .instantiate()
            .map_err(|e| LoadError::Start(format!("{e:#}")))?;
        let word = instance
            .get_typed_func::<(), u32>(&mut store, "oag_abi")
            .and_then(|f| f.call(&mut store, ()))
            .map_err(|e| LoadError::Export(format!("`oag_abi`: {e:#}")))?;
        if word != ABI_WORD {
            let (major, revision) = oag_ext_abi::split_abi_word(word);
            return Err(LoadError::Abi { major, revision });
        }
        // Optional: a module that reads the track region exports its buffer.
        match module.get_export("oag_track") {
            None => {}
            Some(ExternType::Func(func)) => returns_a_word("oag_track", &func)?,
            Some(_) => {
                return Err(LoadError::Export(
                    "`oag_track` has the wrong kind".to_string(),
                ));
            }
        }
        // Optional: a module that says how long its state is must agree with its manifest.
        if let Some(export) = module.get_export("oag_state_len") {
            let ExternType::Func(func) = export else {
                return Err(LoadError::Export(
                    "`oag_state_len` has the wrong kind".to_string(),
                ));
            };
            returns_a_word("oag_state_len", &func)?;
            let len = instance
                .get_typed_func::<(), u32>(&mut store, "oag_state_len")
                .and_then(|f| f.call(&mut store, ()))
                .map_err(|e| LoadError::Export(format!("`oag_state_len`: {e:#}")))?;
            if len != limits.state_bytes {
                return Err(LoadError::State {
                    module: len,
                    manifest: limits.state_bytes,
                });
            }
        }
        Ok(loaded)
    }

    /// The limits it was compiled with.
    #[must_use]
    pub fn limits(&self) -> Limits {
        self.limits
    }

    fn instantiate(&self) -> wasmtime::Result<(Store<HostData>, wasmtime::Instance)> {
        let limits = StoreLimitsBuilder::new()
            .memory_size(self.limits.memory_pages as usize * PAGE)
            .instances(1)
            .memories(1)
            .tables(1)
            .build();
        let mut store = Store::new(self.pre.module().engine(), HostData { limits });
        store.limiter(|data| &mut data.limits);
        store.set_fuel(self.limits.fuel)?;
        let instance = self.pre.instantiate(&mut store)?;
        Ok((store, instance))
    }

    /// Runs one tick on a fresh instance. `state` is the module's blob
    /// ([`Limits::state_bytes`] long): read before the call and, on success, replaced by
    /// the guest's. `commands` receives the checked reply. On a fault neither changes.
    ///
    /// # Errors
    ///
    /// A [`Fault`], with what the call cost up to it.
    pub fn tick(
        &self,
        observation: &Observation<'_>,
        state: &mut [u8],
        commands: &mut Vec<Command>,
    ) -> Result<Cost, (Fault, Cost)> {
        let mut store_fuel = None;
        let result = self.run(observation, state, &mut store_fuel);
        let cost = Cost {
            fuel_used: self.limits.fuel - store_fuel.unwrap_or(0),
            fuel_budget: self.limits.fuel,
        };
        match result {
            Ok((reply_state, reply)) => {
                state.copy_from_slice(&reply_state[..state.len()]);
                commands.clear();
                commands.extend(reply);
                Ok(cost)
            }
            Err(fault) => Err((fault, cost)),
        }
    }

    fn run(
        &self,
        observation: &Observation<'_>,
        state: &[u8],
        fuel_left: &mut Option<u64>,
    ) -> Result<(Vec<u8>, Vec<Command>), Fault> {
        let (mut store, instance) = self.instantiate().map_err(trap)?;
        let outcome = (|| {
            let memory: Memory = instance
                .get_memory(&mut store, "memory")
                .ok_or_else(|| Fault::Trap("no memory".to_string()))?;
            let io = instance
                .get_typed_func::<(), u32>(&mut store, "oag_io")
                .and_then(|f| f.call(&mut store, ()))
                .map_err(trap)? as usize;
            let mut buffer = Box::new([0_u8; IO_LEN]);
            oag_ext_abi::write_observation(
                &mut buffer,
                observation.tick,
                observation.active,
                observation.events,
                observation.crafts,
                state,
            );
            memory.write(&mut store, io, &buffer[..]).map_err(|e| {
                Fault::Trap(format!("the I/O buffer at {io:#x} is outside memory: {e}"))
            })?;
            if !observation.track.is_empty()
                && let Ok(export) = instance.get_typed_func::<(), u32>(&mut store, "oag_track")
            {
                let at = export.call(&mut store, ()).map_err(trap)? as usize;
                // The whole region must fit, not only the bytes written.
                let end = at.saturating_add(TRACK_LEN);
                if end > memory.data_size(&store) {
                    return Err(Fault::Trap(format!(
                        "the track region at {at:#x} is outside memory"
                    )));
                }
                memory
                    .write(&mut store, at, observation.track)
                    .map_err(|e| Fault::Trap(e.to_string()))?;
            }
            let status = instance
                .get_typed_func::<(), u32>(&mut store, "tick")
                .and_then(|f| f.call(&mut store, ()))
                .map_err(trap)?;
            if status != 0 {
                return Err(Fault::Reply(format!("tick returned {status}")));
            }
            memory
                .read(&store, io, &mut buffer[..])
                .map_err(|e| Fault::Trap(e.to_string()))?;
            let commands = reply::read(&buffer, observation)?;
            let at = oag_ext_abi::STATE_AT;
            Ok((buffer[at..at + state.len()].to_vec(), commands))
        })();
        *fuel_left = store.get_fuel().ok();
        outcome
    }
}

fn trap(error: wasmtime::Error) -> Fault {
    match error.downcast_ref::<Trap>() {
        Some(Trap::OutOfFuel) => Fault::OutOfFuel,
        Some(other) => Fault::Trap(other.to_string()),
        None => Fault::Trap(format!("{error:#}")),
    }
}

#[cfg(test)]
mod tests;
