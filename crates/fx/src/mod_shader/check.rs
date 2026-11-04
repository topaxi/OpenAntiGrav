//! What naga's validator accepts and a mod's shader may still not do. `refuse` runs over the
//! parsed module, the mod's text and the host's as one.
//!
//! **Loops.** A loop in a fragment shader that never ends hangs the GPU until the driver resets
//! it, taking the race with it, and whether a loop ends is not decidable from its text. So every
//! loop is refused: WGSL's `for`, `while` and `loop` all parse to [`Statement::Loop`], and WGSL
//! has no recursion, so a module with none runs in time bounded by its length. What that leaves
//! open is `docs/architecture/wasm-extensions/mod-shaders.md`, "What is not covered".

use naga::{Block, Module, Statement, TypeInner};

/// The largest array a shader may declare, bytes: a module's memory is the driver's.
const MAX_ARRAY_BYTES: u64 = 64 * 1024;

/// The bindings the host declares, all of `@group(0)`.
const HOST_BINDINGS: std::ops::RangeInclusive<u32> = 0..=3;

/// The host's entry points: a mod's would be unreachable.
const HOST_ENTRY_POINTS: usize = 3;

fn has_loop(block: &Block) -> bool {
    block.iter().any(|statement| match statement {
        Statement::Loop { .. } => true,
        Statement::Block(inner) => has_loop(inner),
        Statement::If { accept, reject, .. } => has_loop(accept) || has_loop(reject),
        Statement::Switch { cases, .. } => cases.iter().any(|case| has_loop(&case.body)),
        _ => false,
    })
}

/// Why `module` may not run, if it may not. The pipeline layout is the host's, so a binding of
/// the mod's own would fail on the device rather than here.
pub(super) fn refuse(module: &Module) -> Result<(), String> {
    for (_, function) in module.functions.iter() {
        if has_loop(&function.body) {
            let name = function.name.as_deref().unwrap_or("a function");
            return Err(format!(
                "`{name}` has a loop: a shader may not loop, it could hang the GPU"
            ));
        }
    }
    if module.entry_points.len() != HOST_ENTRY_POINTS {
        return Err("a mod's file declares no entry points of its own".to_string());
    }
    for (_, global) in module.global_variables.iter() {
        if let Some(binding) = &global.binding
            && (binding.group != 0 || !HOST_BINDINGS.contains(&binding.binding))
        {
            return Err(format!(
                "@group({}) @binding({}): a mod's file declares no bindings of its own",
                binding.group, binding.binding
            ));
        }
    }
    if let Some((_, over)) = module.overrides.iter().find(|(_, o)| o.init.is_none()) {
        let name = over.name.as_deref().unwrap_or("an override");
        return Err(format!(
            "`{name}` has no default, and nothing sets an override"
        ));
    }
    for (_, ty) in module.types.iter() {
        if let TypeInner::Array {
            size: naga::ArraySize::Constant(count),
            stride,
            ..
        } = ty.inner
            && u64::from(count.get()) * u64::from(stride) > MAX_ARRAY_BYTES
        {
            return Err(format!(
                "an array of {count} elements: the most is {MAX_ARRAY_BYTES} bytes"
            ));
        }
    }
    Ok(())
}
