pub mod bindings;
pub mod silo;
mod silo_impl;

pub use silo::SiloCtx;
pub use silo::{SiloImpl, SiloView};

pub fn add_to_linker_sync<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: SiloView,
{
    let closure = type_annotate_silo::<T, _>(|t| SiloImpl(t));
    crate::silo::bindings::process::add_to_linker_get_host(l, closure)?;
    crate::silo::bindings::threads::add_to_linker_get_host(l, closure)?;

    Ok(())
}

// NB: workaround some rustc inference - a future refactoring may make this
// obsolete.
fn type_annotate_silo<T, F>(val: F) -> F
where
    F: Fn(&mut T) -> SiloImpl<&mut T>,
{
    val
}
