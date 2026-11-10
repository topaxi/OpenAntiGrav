//! What naga's validator accepts and a mod's shader may still not do. `refuse` runs over the
//! parsed module, the mod's text and the host's as one.
//!
//! **Loops.** A loop in a fragment shader that never ends hangs the GPU until the driver resets
//! it, taking the race with it, and whether a loop ends is not decidable from its text. So every
//! loop is refused: WGSL's `for`, `while` and `loop` all parse to [`Statement::Loop`], and WGSL
//! has no recursion.
//!
//! **Calls.** With no loop a function still runs longer than its text: a helper that calls the
//! next one twice, thirty deep, is a billion calls in two kilobytes. So a function's *work* is
//! counted through its calls ([`work`]: its own expressions and statements, and each callee's as
//! often as it is called), and one past [`MAX_WORK`] is refused. What that leaves open is
//! `docs/architecture/wasm-extensions/mod-shaders.md`, "What is not covered".

use naga::{Block, Function, Handle, Module, Statement, TypeInner};

/// The largest array a shader may declare, bytes: a module's memory is the driver's.
const MAX_ARRAY_BYTES: u64 = 64 * 1024;

/// The bindings the host declares, all of `@group(0)`.
const HOST_BINDINGS: std::ops::RangeInclusive<u32> = 0..=3;

/// The host's entry points: a mod's would be unreachable.
const HOST_ENTRY_POINTS: usize = 5;

/// The most work one call of a function may be ([`work`]). The heaviest shader shipped
/// (`oag.slipstream`'s tunnel) is under a sixteenth of it.
const MAX_WORK: u64 = 32_768;

/// The statements of `block` and the blocks in it, counted, with every call found added to
/// `calls`. Both arms of a branch count: an upper bound.
fn walk(block: &Block, calls: &mut Vec<Handle<Function>>) -> u64 {
    block
        .iter()
        .map(|statement| {
            1 + match statement {
                Statement::Call { function, .. } => {
                    calls.push(*function);
                    0
                }
                Statement::Block(inner) => walk(inner, calls),
                Statement::If { accept, reject, .. } => walk(accept, calls) + walk(reject, calls),
                Statement::Switch { cases, .. } => {
                    cases.iter().map(|case| walk(&case.body, calls)).sum()
                }
                Statement::Loop {
                    body, continuing, ..
                } => walk(body, calls) + walk(continuing, calls),
                _ => 0,
            }
        })
        .sum()
}

/// What one call of `function` costs: its expressions and statements, plus the work of each
/// function it calls, once per call site. `known` holds the functions already counted; one
/// still being counted (a cycle, which a validated module cannot have) is beyond any bound.
fn work(module: &Module, function: Handle<Function>, known: &mut [Option<u64>]) -> u64 {
    if let Some(work) = known[function.index()] {
        return work;
    }
    known[function.index()] = Some(u64::MAX);
    let body = &module.functions[function];
    let mut calls = Vec::new();
    let own = body.expressions.len() as u64 + walk(&body.body, &mut calls);
    let total = calls.into_iter().fold(own, |total, callee| {
        total.saturating_add(work(module, callee, known))
    });
    known[function.index()] = Some(total);
    total
}

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
    let mut known = vec![None; module.functions.len()];
    for (handle, function) in module.functions.iter() {
        let work = work(module, handle, &mut known);
        if work > MAX_WORK {
            let name = function.name.as_deref().unwrap_or("a function");
            return Err(format!(
                "`{name}` is {work} operations through the calls it makes, the most is \
                 {MAX_WORK}: a helper called from helpers multiplies"
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
